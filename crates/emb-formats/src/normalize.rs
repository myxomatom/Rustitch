//! Adapts a quantised design to the constraints of a target format.
//!
//! Normalisation works on machine units so that every decision is exact:
//! no floating-point rounding can push a split displacement over the limit.

use emb_core::{StitchKind, Thread};

use crate::capabilities::{FormatCapabilities, TrimSupport};
use crate::error::FormatError;
use crate::machine::{MachinePattern, MachineStitch};
use crate::warning::Warning;

/// A normalised design and what normalisation changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Normalized {
    /// Design ready to be encoded.
    pub pattern: MachinePattern,
    /// Changes that the report must mention.
    pub warnings: Vec<Warning>,
}

/// Adapts `pattern` to `caps`. After normalisation:
///
/// - there is no `End` stitch (writers always terminate the data);
/// - stops and sequins unsupported by the format are converted;
/// - every command (trim, colour change, stop) sits at the current needle
///   position, a jump being inserted before it if needed;
/// - no displacement exceeds `caps.max_delta` on either axis: longer moves
///   are preceded by evenly spaced intermediate jumps;
/// - trims follow the format's trim encoding rules.
///
/// # Errors
///
/// [`FormatError::TooManyColors`] or [`FormatError::TooManyStitches`] if the
/// format limits are exceeded.
pub fn normalize(
    pattern: MachinePattern,
    caps: &FormatCapabilities,
) -> Result<Normalized, FormatError> {
    let mut warnings = Vec::new();
    let (stitches, threads) = pattern.into_parts();
    let (stitches, threads) = convert_unsupported_kinds(stitches, threads, caps, &mut warnings);
    let stitches = anchor_commands(stitches);
    let stitches = split_long_moves(stitches, caps.max_delta);
    let stitches = match caps.trims {
        TrimSupport::JumpSequence {
            machine_trims_after_jumps,
        } => insert_implicit_trims(&stitches, machine_trims_after_jumps, &mut warnings),
        TrimSupport::NotStored => drop_trims(stitches, &mut warnings),
    };
    check_limits(&stitches, &threads, caps)?;
    Ok(Normalized {
        pattern: MachinePattern::new(stitches, threads)?,
        warnings,
    })
}

fn convert_unsupported_kinds(
    stitches: Vec<MachineStitch>,
    mut threads: Vec<Thread>,
    caps: &FormatCapabilities,
    warnings: &mut Vec<Warning>,
) -> (Vec<MachineStitch>, Vec<Thread>) {
    let mut out = Vec::with_capacity(stitches.len());
    let mut block = 0;
    let (mut stops, mut sequins) = (0, 0);
    for mut stitch in stitches {
        match stitch.kind {
            StitchKind::End => continue,
            StitchKind::Stop if !caps.stops => {
                if let Some(thread) = threads.get(block).cloned() {
                    threads.insert(block + 1, thread);
                }
                stitch.kind = StitchKind::ColorChange;
                stops += 1;
            }
            StitchKind::SequinEject if !caps.sequins => {
                stitch.kind = StitchKind::Jump;
                sequins += 1;
            }
            _ => {}
        }
        if stitch.kind == StitchKind::ColorChange {
            block += 1;
        }
        out.push(stitch);
    }
    if out.is_empty() {
        // Only an `End` was dropped: an empty design has no colour block.
        threads.clear();
    }
    if stops > 0 {
        warnings.push(Warning::StopsConvertedToColorChanges { count: stops });
    }
    if sequins > 0 {
        warnings.push(Warning::SequinsConvertedToJumps { count: sequins });
    }
    (out, threads)
}

/// Inserts a jump before every command that is not at the needle position.
fn anchor_commands(stitches: Vec<MachineStitch>) -> Vec<MachineStitch> {
    let mut out = Vec::with_capacity(stitches.len());
    let (mut x, mut y) = (0, 0);
    for stitch in stitches {
        if !stitch.kind.is_move() && !stitch.is_at(x, y) {
            out.push(MachineStitch::new(StitchKind::Jump, stitch.x, stitch.y));
        }
        (x, y) = (stitch.x, stitch.y);
        out.push(stitch);
    }
    out
}

/// Splits moves longer than `max_delta` (on either axis) into evenly spaced
/// jumps followed by the original stitch.
fn split_long_moves(stitches: Vec<MachineStitch>, max_delta: i32) -> Vec<MachineStitch> {
    let max = i64::from(max_delta.max(1));
    let mut out = Vec::with_capacity(stitches.len());
    let (mut x, mut y) = (0_i32, 0_i32);
    for stitch in stitches {
        let dx = i64::from(stitch.x) - i64::from(x);
        let dy = i64::from(stitch.y) - i64::from(y);
        let longest = dx.abs().max(dy.abs());
        if stitch.kind.is_move() && longest > max {
            let steps = (longest + max - 1) / max;
            for k in 1..steps {
                out.push(MachineStitch::new(
                    StitchKind::Jump,
                    step_towards(x, dx, k, steps),
                    step_towards(y, dy, k, steps),
                ));
            }
        }
        (x, y) = (stitch.x, stitch.y);
        out.push(stitch);
    }
    out
}

/// `from + floor(delta * k / steps)`: consecutive values differ by
/// `floor(delta / steps)` or `ceil(delta / steps)`, never more.
fn step_towards(from: i32, delta: i64, k: i64, steps: i64) -> i32 {
    let value = i64::from(from) + (delta * k).div_euclid(steps);
    // The value lies between two `i32` positions, so it always fits.
    i32::try_from(value).unwrap_or(from)
}

fn is_jump_run_member(stitch: &MachineStitch) -> bool {
    matches!(stitch.kind, StitchKind::Jump | StitchKind::Trim)
}

/// Makes explicit the trims that the machine performs on its own after
/// `threshold` consecutive jumps.
fn insert_implicit_trims(
    stitches: &[MachineStitch],
    threshold: usize,
    warnings: &mut Vec<Warning>,
) -> Vec<MachineStitch> {
    let mut out = Vec::with_capacity(stitches.len());
    let mut inserted = 0;
    let (mut x, mut y) = (0, 0);
    for group in stitches.chunk_by(|a, b| is_jump_run_member(a) && is_jump_run_member(b)) {
        let is_run = group.first().is_some_and(is_jump_run_member);
        let jumps = group.iter().filter(|s| s.kind == StitchKind::Jump).count();
        let has_trim = group.iter().any(|s| s.kind == StitchKind::Trim);
        if is_run && jumps >= threshold && !has_trim {
            out.push(MachineStitch::new(StitchKind::Trim, x, y));
            inserted += 1;
        }
        out.extend_from_slice(group);
        if let Some(last) = group.last() {
            (x, y) = (last.x, last.y);
        }
    }
    if inserted > 0 {
        warnings.push(Warning::ImplicitTrimsInserted { count: inserted });
    }
    out
}

/// Removes trims, which the format cannot store. Trims are anchored, so
/// removing them changes no position.
fn drop_trims(stitches: Vec<MachineStitch>, warnings: &mut Vec<Warning>) -> Vec<MachineStitch> {
    let before = stitches.len();
    let out: Vec<_> = stitches
        .into_iter()
        .filter(|s| s.kind != StitchKind::Trim)
        .collect();
    let dropped = before - out.len();
    if dropped > 0 {
        warnings.push(Warning::TrimsNotStored { count: dropped });
    }
    out
}

fn check_limits(
    stitches: &[MachineStitch],
    threads: &[Thread],
    caps: &FormatCapabilities,
) -> Result<(), FormatError> {
    if let Some(max) = caps.max_colors.filter(|&max| threads.len() > max) {
        return Err(FormatError::TooManyColors {
            format: caps.name,
            max,
            actual: threads.len(),
        });
    }
    if let Some(max) = caps.max_stitches.filter(|&max| stitches.len() > max) {
        return Err(FormatError::TooManyStitches {
            format: caps.name,
            max,
            actual: stitches.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::ColorModel;
    use emb_core::Rgb;
    use proptest::prelude::*;

    const CAPS: FormatCapabilities = FormatCapabilities {
        name: "TEST",
        units_per_mm: 10,
        max_delta: 10,
        trims: TrimSupport::JumpSequence {
            machine_trims_after_jumps: 3,
        },
        stops: false,
        sequins: false,
        color_model: ColorModel::None,
        max_colors: None,
        max_stitches: None,
        hoops: &[],
    };

    const NO_TRIM_CAPS: FormatCapabilities = FormatCapabilities {
        trims: TrimSupport::NotStored,
        ..CAPS
    };

    fn s(kind: StitchKind, x: i32, y: i32) -> MachineStitch {
        MachineStitch::new(kind, x, y)
    }

    fn thread(v: u8) -> Thread {
        Thread::from_color(Rgb::new(v, v, v))
    }

    fn run(
        stitches: Vec<MachineStitch>,
        threads: Vec<Thread>,
        caps: &FormatCapabilities,
    ) -> Normalized {
        normalize(MachinePattern::new(stitches, threads).unwrap(), caps).unwrap()
    }

    fn kinds(n: &Normalized) -> Vec<StitchKind> {
        n.pattern.stitches().iter().map(|s| s.kind).collect()
    }

    #[test]
    fn long_stitch_is_split_into_even_jumps_without_tiny_remainder() {
        let n = run(vec![s(StitchKind::Normal, 25, 0)], vec![thread(1)], &CAPS);
        let xs: Vec<_> = n.pattern.stitches().iter().map(|s| s.x).collect();
        assert_eq!(xs, vec![8, 16, 25]);
        assert_eq!(
            kinds(&n),
            vec![StitchKind::Jump, StitchKind::Jump, StitchKind::Normal]
        );
    }

    #[test]
    fn negative_long_moves_are_split_within_limit() {
        let n = run(vec![s(StitchKind::Normal, -21, 7)], vec![thread(1)], &CAPS);
        let positions: Vec<_> = n.pattern.stitches().iter().map(|s| (s.x, s.y)).collect();
        assert_eq!(positions, vec![(-7, 2), (-14, 4), (-21, 7)]);
    }

    #[test]
    fn commands_are_anchored_to_the_needle_position() {
        let n = run(
            vec![
                s(StitchKind::Normal, 1, 1),
                s(StitchKind::ColorChange, 5, 5),
                s(StitchKind::Normal, 6, 6),
            ],
            vec![thread(1), thread(2)],
            &CAPS,
        );
        assert_eq!(
            n.pattern.stitches(),
            &[
                s(StitchKind::Normal, 1, 1),
                s(StitchKind::Jump, 5, 5),
                s(StitchKind::ColorChange, 5, 5),
                s(StitchKind::Normal, 6, 6),
            ]
        );
    }

    #[test]
    fn stop_becomes_color_change_with_duplicated_thread() {
        let n = run(
            vec![
                s(StitchKind::Normal, 0, 0),
                s(StitchKind::Stop, 0, 0),
                s(StitchKind::Normal, 1, 0),
                s(StitchKind::ColorChange, 1, 0),
                s(StitchKind::Normal, 2, 0),
            ],
            vec![thread(1), thread(2)],
            &CAPS,
        );
        let colors: Vec<_> = n.pattern.threads().iter().map(|t| t.color.r).collect();
        assert_eq!(colors, vec![1, 1, 2]);
        assert_eq!(
            n.warnings,
            vec![Warning::StopsConvertedToColorChanges { count: 1 }]
        );
    }

    #[test]
    fn end_is_dropped_and_lone_end_leaves_an_empty_design() {
        let n = run(vec![s(StitchKind::End, 0, 0)], vec![thread(1)], &CAPS);
        assert_eq!(n.pattern.stitches(), []);
        assert_eq!(n.pattern.threads(), []);
    }

    #[test]
    fn sequins_become_jumps_when_unsupported() {
        let n = run(
            vec![s(StitchKind::SequinEject, 3, 0)],
            vec![thread(1)],
            &CAPS,
        );
        assert_eq!(kinds(&n), vec![StitchKind::Jump]);
        assert_eq!(
            n.warnings,
            vec![Warning::SequinsConvertedToJumps { count: 1 }]
        );
    }

    #[test]
    fn long_untrimmed_jump_runs_get_an_explicit_trim() {
        let n = run(
            vec![s(StitchKind::Normal, 1, 0), s(StitchKind::Jump, 31, 0)],
            vec![thread(1)],
            &CAPS,
        );
        assert_eq!(
            n.pattern.stitches(),
            &[
                s(StitchKind::Normal, 1, 0),
                s(StitchKind::Trim, 1, 0),
                s(StitchKind::Jump, 11, 0),
                s(StitchKind::Jump, 21, 0),
                s(StitchKind::Jump, 31, 0),
            ]
        );
        assert_eq!(
            n.warnings,
            vec![Warning::ImplicitTrimsInserted { count: 1 }]
        );
    }

    #[test]
    fn short_or_already_trimmed_jump_runs_are_untouched() {
        let stitches = vec![
            s(StitchKind::Normal, 0, 0),
            s(StitchKind::Jump, 10, 0),
            s(StitchKind::Jump, 20, 0),
            s(StitchKind::Normal, 20, 1),
            s(StitchKind::Jump, 21, 1),
            s(StitchKind::Trim, 21, 1),
            s(StitchKind::Jump, 22, 1),
            s(StitchKind::Jump, 23, 1),
        ];
        let n = run(stitches.clone(), vec![thread(1)], &CAPS);
        assert_eq!(n.pattern.stitches(), stitches.as_slice());
        assert_eq!(n.warnings, []);
    }

    #[test]
    fn trims_are_dropped_and_reported_when_not_stored() {
        let n = run(
            vec![
                s(StitchKind::Normal, 1, 1),
                s(StitchKind::Jump, 1, 1),
                s(StitchKind::Trim, 1, 1),
                s(StitchKind::Jump, 2, 1),
            ],
            vec![thread(1)],
            &NO_TRIM_CAPS,
        );
        assert_eq!(
            kinds(&n),
            vec![StitchKind::Normal, StitchKind::Jump, StitchKind::Jump]
        );
        assert_eq!(n.warnings, vec![Warning::TrimsNotStored { count: 1 }]);
    }

    #[test]
    fn color_limit_is_enforced() {
        let caps = FormatCapabilities {
            max_colors: Some(1),
            ..CAPS
        };
        let pattern = MachinePattern::new(
            vec![
                s(StitchKind::Normal, 0, 0),
                s(StitchKind::ColorChange, 0, 0),
            ],
            vec![thread(1), thread(2)],
        )
        .unwrap();
        assert!(matches!(
            normalize(pattern, &caps),
            Err(FormatError::TooManyColors {
                max: 1,
                actual: 2,
                ..
            })
        ));
    }

    fn arbitrary_stitch() -> impl Strategy<Value = MachineStitch> {
        let kind = prop_oneof![
            Just(StitchKind::Normal),
            Just(StitchKind::Jump),
            Just(StitchKind::Trim),
            Just(StitchKind::ColorChange),
            Just(StitchKind::Stop),
            Just(StitchKind::SequinEject),
        ];
        (kind, -500..500_i32, -500..500_i32).prop_map(|(k, x, y)| MachineStitch::new(k, x, y))
    }

    fn arbitrary_pattern() -> impl Strategy<Value = MachinePattern> {
        prop::collection::vec(arbitrary_stitch(), 0..60).prop_map(|stitches| {
            let blocks = crate::machine::machine_block_count(&stitches);
            let threads = (0..blocks)
                .map(|i| thread(u8::try_from(i % 256).unwrap()))
                .collect();
            MachinePattern::new(stitches, threads).unwrap()
        })
    }

    proptest! {
        #[test]
        fn normalized_moves_never_exceed_max_delta(pattern in arbitrary_pattern()) {
            let n = normalize(pattern, &CAPS).unwrap();
            let (mut x, mut y) = (0, 0);
            for st in n.pattern.stitches() {
                prop_assert!((st.x - x).abs() <= CAPS.max_delta);
                prop_assert!((st.y - y).abs() <= CAPS.max_delta);
                if !st.kind.is_move() {
                    prop_assert!(st.is_at(x, y), "command not anchored: {st:?}");
                }
                (x, y) = (st.x, st.y);
            }
        }

        #[test]
        fn normalization_preserves_visible_stitches_and_blocks(pattern in arbitrary_pattern()) {
            let normal = |p: &MachinePattern| -> Vec<(i32, i32)> {
                p.stitches().iter().filter(|s| s.kind == StitchKind::Normal).map(|s| (s.x, s.y)).collect()
            };
            let stops = pattern.stitches().iter().filter(|s| s.kind == StitchKind::Stop).count();
            let n = normalize(pattern.clone(), &CAPS).unwrap();
            prop_assert_eq!(normal(&n.pattern), normal(&pattern));
            prop_assert_eq!(n.pattern.threads().len(), pattern.threads().len() + stops);
        }

        #[test]
        fn normalization_is_idempotent(pattern in arbitrary_pattern()) {
            let once = normalize(pattern, &CAPS).unwrap().pattern;
            let twice = normalize(once.clone(), &CAPS).unwrap();
            prop_assert_eq!(&twice.pattern, &once);
            prop_assert_eq!(twice.warnings, vec![]);
        }
    }
}
