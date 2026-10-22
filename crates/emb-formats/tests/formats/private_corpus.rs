//! Real files from `tests/fixtures/private/` (not versioned: redistribution
//! rights unknown). Each test checks every file present; on a machine without
//! the corpus they check nothing and say so.

use std::fs;
use std::path::{Path, PathBuf};

use emb_core::{EPSILON_MM, Mm, StitchKind, StitchPattern};
use emb_formats::jef::PALETTE_EXTENSION;
use emb_formats::jef::palette::JANOME_PALETTE;
use emb_formats::{Format, MachinePattern, Warning};

use crate::{read, write};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/private")
}

/// Every file of the given format, sorted by name for determinism.
fn files(format: Format) -> Vec<PathBuf> {
    let dir = corpus_dir().join(format.extension());
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(format.extension()))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        eprintln!(
            "note: no {format} file in {}, nothing checked",
            dir.display()
        );
    }
    files
}

fn read_file(format: Format, path: &Path) -> (Vec<u8>, StitchPattern, Vec<Warning>) {
    let bytes = fs::read(path).unwrap();
    let output = read(format, &bytes);
    (bytes, output.pattern, output.warnings)
}

fn is_structural(warning: &Warning) -> bool {
    matches!(
        warning,
        Warning::TruncatedData { .. }
            | Warning::MissingEndMarker
            | Warning::UnknownCommand { .. }
            | Warning::ThreadCountMismatch { .. }
            | Warning::UnknownPaletteIndex { .. }
    )
}

#[test]
fn real_files_are_detected_and_read_without_structural_warnings() {
    for format in Format::ALL {
        for path in files(format) {
            let (bytes, _, warnings) = read_file(format, &path);
            assert_eq!(Format::detect(&bytes), Some(format), "{}", path.display());
            let structural: Vec<_> = warnings.iter().filter(|w| is_structural(w)).collect();
            assert!(structural.is_empty(), "{}: {structural:?}", path.display());
        }
    }
}

/// The same design in another format, if the corpus has it.
fn twin(path: &Path, format: Format) -> Option<PathBuf> {
    let name = path.with_extension(format.extension());
    let twin = corpus_dir()
        .join(format.extension())
        .join(name.file_name()?);
    twin.exists().then_some(twin)
}

#[test]
fn twin_files_have_the_same_blocks_and_size() {
    for format in Format::ALL {
        for path in files(format) {
            let (_, a, _) = read_file(format, &path);
            for other in Format::ALL.into_iter().filter(|&f| f > format) {
                let Some(twin_path) = twin(&path, other) else {
                    continue;
                };
                let context = format!("{} vs {other}", path.display());
                let (_, b, _) = read_file(other, &twin_path);
                assert_eq!(a.threads().len(), b.threads().len(), "{context}");
                let (ba, bb) = (a.bounds().unwrap(), b.bounds().unwrap());
                // Twins come from separate exports: allow one machine unit per side.
                let tolerance = Mm(0.2) + EPSILON_MM;
                assert!(ba.width().approx_eq(bb.width(), tolerance), "{context}");
                assert!(ba.height().approx_eq(bb.height(), tolerance), "{context}");
            }
        }
    }
}

/// Visible stitches of each colour block, in machine units.
fn normal_stitches_per_block(pattern: &StitchPattern) -> Vec<Vec<(i32, i32)>> {
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

#[test]
fn real_files_convert_to_every_writable_format_without_moving_a_stitch() {
    for source in Format::ALL {
        for path in files(source) {
            let (_, pattern, _) = read_file(source, &path);
            for target in Format::WRITABLE {
                let (bytes, _) = write(target, &pattern);
                let back = read(target, &bytes).pattern;
                assert_eq!(
                    normal_stitches_per_block(&back),
                    normal_stitches_per_block(&pattern),
                    "{} -> {target}",
                    path.display()
                );
            }
        }
    }
}

/// PES threads reach the JEF writer, which must report every palette
/// approximation it makes, and are kept in the ENHANCE block (all but the
/// brand, which the block cannot hold).
#[test]
fn pes_threads_survive_in_jef_and_approximations_are_reported() {
    for path in files(Format::Pes) {
        let (_, pattern, warnings) = read_file(Format::Pes, &path);
        if warnings.contains(&Warning::ThreadListUnreadable) {
            continue;
        }
        let (bytes, output) = write(Format::Jef, &pattern);
        let jef = read(Format::Jef, &bytes).pattern;
        let indices = &jef.extensions[PALETTE_EXTENSION];
        for (block, (source, read_back)) in pattern.threads().iter().zip(jef.threads()).enumerate()
        {
            let context = format!("{} block {block}", path.display());
            assert_eq!(read_back.color, source.color, "{context}");
            assert_eq!(read_back.catalog_number, source.catalog_number, "{context}");
            if source.name.is_some() {
                assert_eq!(read_back.name, source.name, "{context}");
            }
            let entry = JANOME_PALETTE[usize::from(indices[block]) - 1];
            let delta = emb_core::delta_e_2000(source.color.to_lab(), entry.color.to_lab());
            let reported = output
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::ThreadApproximated { block: b, .. } if *b == block));
            assert!(
                delta.get() <= 3.0 || reported,
                "{} block {block}: ΔE {delta} not reported",
                path.display()
            );
        }
    }
}

/// With the default injective matching, distinct threads of a real design
/// never share a Janome colour (the palette has more entries than any
/// corpus design has threads).
#[test]
fn real_designs_keep_distinct_threads_distinct_in_jef() {
    for source in Format::ALL {
        for path in files(source) {
            let (_, pattern, _) = read_file(source, &path);
            let (_, output) = write(Format::Jef, &pattern);
            let shared: Vec<_> = output
                .warnings
                .iter()
                .filter(|w| matches!(w, Warning::ThreadsShareColor { .. }))
                .collect();
            assert!(shared.is_empty(), "{}: {shared:?}", path.display());
        }
    }
}

/// Deterministic corruption of real files: overwritten bytes and truncations
/// must never make a reader panic.
#[test]
fn readers_survive_corrupted_real_files() {
    for format in Format::ALL {
        for path in files(format) {
            let original = fs::read(&path).unwrap();
            for step in [7, 31, 97, 251] {
                for value in [0x00, 0x80, 0xFE, 0xFF] {
                    let mut bytes = original.clone();
                    bytes.iter_mut().step_by(step).for_each(|b| *b = value);
                    let _ = format.reader().read(&mut bytes.as_slice());
                }
            }
            for keep in (0..original.len()).step_by(original.len() / 50 + 1) {
                let _ = format.reader().read(&mut &original[..keep]);
            }
        }
    }
}

#[test]
fn rewriting_a_real_file_keeps_every_stitch() {
    for format in Format::WRITABLE {
        for path in files(format) {
            let (_, pattern, _) = read_file(format, &path);
            let (bytes, _) = write(format, &pattern);
            let back = read(format, &bytes).pattern;
            let units = |p: &StitchPattern| MachinePattern::from_pattern(p, 10).unwrap();
            assert_eq!(
                units(&back).stitches(),
                units(&pattern).stitches(),
                "{}",
                path.display()
            );
        }
    }
}

/// Stitch data up to and including the end marker.
fn jef_stitch_data(bytes: &[u8]) -> &[u8] {
    let offset = usize::try_from(u32::from_le_bytes(bytes[0..4].try_into().unwrap())).unwrap();
    let data = &bytes[offset..];
    let mut pos = 0;
    while !(data[pos] == 0x80 && data[pos + 1] == 0x10) {
        pos += if data[pos] == 0x80 { 4 } else { 2 };
    }
    &data[..pos + 2]
}

/// Bytes after the end marker.
fn jef_trailer(bytes: &[u8]) -> &[u8] {
    let offset = usize::try_from(u32::from_le_bytes(bytes[0..4].try_into().unwrap())).unwrap();
    &bytes[offset + jef_stitch_data(bytes).len()..]
}

/// Palette indices of the header.
fn jef_palette(bytes: &[u8]) -> Vec<u32> {
    let count = usize::try_from(u32::from_le_bytes(bytes[0x18..0x1C].try_into().unwrap())).unwrap();
    (0..count)
        .map(|i| u32::from_le_bytes(bytes[0x74 + 4 * i..0x78 + 4 * i].try_into().unwrap()))
        .collect()
}

/// Real JEF files are rewritten with the same palette indices and, when
/// they have one, the very same ENHANCE block.
#[test]
fn jef_colours_and_enhance_block_are_rewritten_identically() {
    for path in files(Format::Jef) {
        let (original, pattern, _) = read_file(Format::Jef, &path);
        let (rewritten, _) = write(Format::Jef, &pattern);
        assert_eq!(
            jef_palette(&rewritten),
            jef_palette(&original),
            "{}",
            path.display()
        );
        if !jef_trailer(&original).is_empty() {
            assert!(
                jef_trailer(&rewritten) == jef_trailer(&original),
                "{}: ENHANCE block differs",
                path.display()
            );
        }
    }
}

/// Files whose flags field is `0x0A` prefix every run of jumps with one
/// zero-length jump, the encoding the writer reproduces.
#[test]
fn jump_run_prefix_family_stitch_data_is_rewritten_byte_for_byte() {
    for path in files(Format::Jef) {
        let (original, pattern, _) = read_file(Format::Jef, &path);
        if original[4] != 0x0A {
            continue;
        }
        let (rewritten, _) = write(Format::Jef, &pattern);
        assert!(
            jef_stitch_data(&rewritten) == jef_stitch_data(&original),
            "{}",
            path.display()
        );
    }
}
