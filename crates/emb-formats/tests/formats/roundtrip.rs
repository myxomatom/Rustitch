//! Properties: `read(write(p)) == normalize(quantize(p))`, determinism,
//! bounded quantisation error and panic-free reading.

use emb_core::{EPSILON_MM, Mm, Rgb, Stitch, StitchKind, StitchPattern, Thread, block_count};
use emb_formats::{Format, MachinePattern, normalize};
use proptest::prelude::*;

use crate::fixtures::at;
use crate::{read, write};

fn any_format() -> impl Strategy<Value = Format> {
    prop::sample::select(Format::WRITABLE.to_vec())
}

fn any_kind() -> impl Strategy<Value = StitchKind> {
    prop_oneof![
        6 => Just(StitchKind::Normal),
        2 => Just(StitchKind::Jump),
        1 => Just(StitchKind::Trim),
        1 => Just(StitchKind::ColorChange),
        1 => Just(StitchKind::Stop),
        1 => Just(StitchKind::SequinEject),
    ]
}

fn any_thread() -> impl Strategy<Value = Thread> {
    (any::<[u8; 3]>(), proptest::option::of(0..90_u8)).prop_map(|([r, g, b], janome)| {
        let mut thread = Thread::from_color(Rgb::new(r, g, b));
        if let Some(code) = janome {
            thread.brand = Some("Janome".into());
            thread.catalog_number = Some(format!("{:03}", 200 + u16::from(code)));
        }
        thread
    })
}

/// Designs within ±90 mm, so that they fit the largest JEF hoop.
fn any_pattern() -> impl Strategy<Value = StitchPattern> {
    let stitch =
        (any_kind(), -90.0..90.0_f64, -90.0..90.0_f64).prop_map(|(kind, x, y)| at(kind, x, y));
    (prop::collection::vec(stitch, 0..120), any::<bool>())
        .prop_flat_map(|(mut stitches, end)| {
            let last = stitches.last().copied().filter(|_| end);
            stitches.extend(last.map(|s| Stitch::new(StitchKind::End, s.position)));
            let blocks = block_count(&stitches);
            (Just(stitches), prop::collection::vec(any_thread(), blocks))
        })
        .prop_map(|(stitches, threads)| StitchPattern::new(stitches, threads).unwrap())
}

proptest! {
    #[test]
    fn read_after_write_equals_normalized_quantized_design(
        pattern in any_pattern(),
        format in any_format(),
    ) {
        let caps = *format.writer().unwrap().capabilities();
        let quantized = MachinePattern::from_pattern(&pattern, caps.units_per_mm).unwrap();
        let expected = normalize(quantized, &caps).unwrap().pattern;
        let (bytes, _) = write(format, &pattern);
        let back = read(format, &bytes);
        let got = MachinePattern::from_pattern(&back.pattern, caps.units_per_mm).unwrap();
        prop_assert_eq!(got.stitches(), expected.stitches());
        prop_assert_eq!(got.threads().len(), expected.threads().len());
    }

    #[test]
    fn writing_is_deterministic(pattern in any_pattern(), format in any_format()) {
        prop_assert_eq!(write(format, &pattern).0, write(format, &pattern).0);
    }

    #[test]
    fn rewriting_a_read_file_is_byte_identical(pattern in any_pattern(), format in any_format()) {
        let (first, _) = write(format, &pattern);
        let (second, _) = write(format, &read(format, &first).pattern);
        prop_assert_eq!(first, second);
    }

    #[test]
    fn readers_never_panic_on_arbitrary_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..2048),
        format in prop::sample::select(Format::ALL.to_vec()),
    ) {
        let _ = format.reader().read(&mut bytes.as_slice());
    }

    /// Garbage behind a valid PES signature and PEC offset reaches the
    /// header, thread list and PEC decoders.
    #[test]
    fn pes_reader_never_panics_behind_a_valid_signature(
        version in prop::sample::select(vec!["0001", "0040", "0070", "0100", "0999"]),
        pec_offset in 12..700_u32,
        body in prop::collection::vec(any::<u8>(), 0..2048),
    ) {
        let mut data = format!("#PES{version}").into_bytes();
        data.extend(pec_offset.to_le_bytes());
        data.extend(body);
        let _ = Format::Pes.reader().read(&mut data.as_slice());
    }

    #[test]
    fn readers_never_panic_on_corrupted_files(
        pattern in any_pattern(),
        format in any_format(),
        corruption in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..16),
        keep in any::<prop::sample::Index>(),
    ) {
        let (mut bytes, _) = write(format, &pattern);
        for (index, value) in corruption {
            let i = index.index(bytes.len());
            bytes[i] = value;
        }
        bytes.truncate(keep.index(bytes.len() + 1));
        let _ = format.reader().read(&mut bytes.as_slice());
    }
}

/// Random walk of `steps` (≤ 6 mm per axis) kept within ±90 mm.
fn random_walk(steps: &[(f64, f64)]) -> StitchPattern {
    let mut position = (0.0_f64, 0.0_f64);
    let stitches = steps
        .iter()
        .map(|&(dx, dy)| {
            position = (
                (position.0 + dx).clamp(-90.0, 90.0),
                (position.1 + dy).clamp(-90.0, 90.0),
            );
            at(StitchKind::Normal, position.0, position.1)
        })
        .collect();
    StitchPattern::new(stitches, vec![Thread::from_color(Rgb::new(0, 0, 0))]).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4))]

    /// CONVERSION §1.2: on 100 000 random points, no exported position is more
    /// than 0.05 mm away from its source.
    #[test]
    fn quantization_error_stays_within_half_a_unit_on_100k_points(
        steps in prop::collection::vec((-6.0..6.0_f64, -6.0..6.0_f64), 100_000),
        format in any_format(),
    ) {
        let pattern = random_walk(&steps);
        let (bytes, _) = write(format, &pattern);
        let back = read(format, &bytes).pattern;
        prop_assert_eq!(back.stitches().len(), pattern.stitches().len());
        let worst = pattern
            .stitches()
            .iter()
            .zip(back.stitches())
            .map(|(a, b)| (a.position.x - b.position.x).abs().max((a.position.y - b.position.y).abs()))
            .fold(Mm::ZERO, Mm::max);
        prop_assert!(worst.get() <= 0.05 + EPSILON_MM.get(), "worst deviation {} mm", worst.get());
    }
}
