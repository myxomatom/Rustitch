//! Conversion matrix: every readable format to every writable format, on
//! every fixture, checking the global invariants of FEATURES §0.

use emb_core::{StitchKind, StitchPattern};
use emb_formats::jef::PALETTE_EXTENSION;
use emb_formats::{Format, MachinePattern, Warning, normalize};

use crate::fixtures;
use crate::{read, write};

/// Visible stitch positions of each colour block, in 0.1 mm units.
fn normal_positions_per_block(pattern: &StitchPattern) -> Vec<Vec<(i32, i32)>> {
    let machine = MachinePattern::from_pattern(pattern, 10).unwrap();
    machine
        .stitches()
        .split(|s| s.kind == StitchKind::ColorChange)
        .map(|block| {
            block
                .iter()
                .filter(|s| s.kind == StitchKind::Normal)
                .map(|s| (s.x, s.y))
                .collect()
        })
        .collect()
}

fn convert(pattern: &StitchPattern, target: Format) -> (StitchPattern, Vec<Warning>) {
    let (bytes, output) = write(target, pattern);
    (read(target, &bytes).pattern, output.warnings)
}

#[test]
fn first_write_keeps_every_visible_stitch_within_a_tenth_of_a_millimetre() {
    for (name, fixture) in fixtures::all() {
        let expected: Vec<_> = normal_positions_per_block(&fixture).concat();
        for format in Format::WRITABLE {
            let (back, _) = convert(&fixture, format);
            let got: Vec<_> = normal_positions_per_block(&back).concat();
            assert_eq!(got, expected, "{name} via {format}");
        }
    }
}

#[test]
fn every_format_pair_preserves_geometry_blocks_and_trims() {
    for (name, fixture) in fixtures::all() {
        for source in Format::WRITABLE {
            let (a, _) = convert(&fixture, source);
            for target in Format::WRITABLE {
                let context = format!("{name}: {source} -> {target}");
                let (b, warnings) = convert(&a, target);
                assert_eq!(b.threads().len(), a.threads().len(), "{context}: blocks");
                assert_eq!(
                    normal_positions_per_block(&b),
                    normal_positions_per_block(&a),
                    "{context}: geometry"
                );
                // Trims are kept, or every lost trim is reported.
                let source_trims = a.stats().trims;
                let reported_lost = warnings.contains(&Warning::TrimsNotStored {
                    count: source_trims,
                });
                assert!(
                    b.stats().trims >= source_trims || reported_lost,
                    "{context}: trims"
                );
                let (c, _) = convert(&b, source);
                assert_eq!(
                    normal_positions_per_block(&c),
                    normal_positions_per_block(&a),
                    "{context} -> {source}: drift"
                );
                if source == target {
                    assert_eq!(b.threads(), a.threads(), "{context}: threads");
                }
            }
        }
    }
}

#[test]
fn jef_never_merges_distinct_threads_silently() {
    for (name, fixture) in fixtures::all() {
        let quantized = MachinePattern::from_pattern(&fixture, 10).unwrap();
        let caps = *Format::Jef.writer().unwrap().capabilities();
        let source_threads = normalize(quantized, &caps)
            .unwrap()
            .pattern
            .threads()
            .to_vec();
        let (back, warnings) = convert(&fixture, Format::Jef);
        let reported = |i: usize, j: usize| {
            warnings.iter().any(|w| {
                matches!(w, Warning::ThreadsShareColor { blocks, .. }
                    if blocks.contains(&i) && blocks.contains(&j))
            })
        };
        // Janome indices actually written (the threads read back are the
        // real ones, kept by the ENHANCE block).
        let indices = &back.extensions[PALETTE_EXTENSION];
        assert_eq!(indices.len(), source_threads.len(), "{name}");
        for (i, a) in source_threads.iter().enumerate() {
            for (j, b) in source_threads.iter().enumerate().skip(i + 1) {
                let merged = indices[i] == indices[j];
                if merged && !a.is_same_thread(b) {
                    assert!(reported(i, j), "{name}: blocks {i} and {j} merged silently");
                }
            }
        }
    }
}

#[test]
fn colourless_target_reports_lost_threads() {
    for (name, fixture) in fixtures::all() {
        let (_, warnings) = convert(&fixture, Format::Dst);
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, Warning::ColorsNotStored { .. })),
            "{name}"
        );
    }
}

#[test]
fn more_threads_than_palette_reports_shared_colours() {
    let (back, warnings) = convert(&fixtures::more_threads_than_palette(), Format::Jef);
    assert_eq!(back.threads().len(), 80);
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, Warning::ThreadsShareColor { .. }))
    );
}
