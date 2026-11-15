//! End-to-end tests of the `emb` binary.

// Test helpers fail loudly on purpose; `clippy.toml` only covers `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use emb_core::{Mm, Point, Stitch, StitchKind, StitchPattern};
use emb_formats::Format;
use emb_formats::jef::palette::{JANOME_PALETTE, palette_thread};
use predicates::prelude::*;
use tempfile::TempDir;

const EXIT_WARNINGS: i32 = 3;

fn janome_design() -> StitchPattern {
    let at = |kind, x, y| Stitch::new(kind, Point::new(Mm(x), Mm(y)));
    let mut pattern = StitchPattern::new(
        vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::Normal, 5.0, 2.0),
            at(StitchKind::ColorChange, 5.0, 2.0),
            at(StitchKind::Normal, 5.0, 7.0),
        ],
        vec![
            palette_thread(&JANOME_PALETTE[0]),
            palette_thread(&JANOME_PALETTE[9]),
        ],
    )
    .unwrap();
    pattern.metadata.name = Some("cli".into());
    pattern
}

fn write_fixture(dir: &TempDir, name: &str, format: Format) -> PathBuf {
    let path = dir.path().join(name);
    let mut bytes = Vec::new();
    format
        .writer()
        .unwrap()
        .write(&janome_design(), &mut bytes)
        .unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

fn emb() -> Command {
    Command::cargo_bin("emb").unwrap()
}

fn detected(path: &Path) -> Option<Format> {
    Format::detect(&fs::read(path).unwrap())
}

#[test]
fn lossless_conversion_exits_with_success() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.jef", Format::Jef);
    let output = dir.path().join("out.jef");
    emb()
        .arg("convert")
        .arg(&input)
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("(JEF) ->"))
        .stderr(predicate::str::is_empty());
    assert_eq!(detected(&output), Some(Format::Jef));
}

#[test]
fn lossy_conversion_exits_with_the_warning_status() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.jef", Format::Jef);
    let output = dir.path().join("out.dst");
    emb()
        .arg("convert")
        .arg(&input)
        .arg(&output)
        .assert()
        .code(EXIT_WARNINGS)
        .stderr(predicate::str::contains(
            "warning (write): the format stores no colour",
        ));
    assert_eq!(detected(&output), Some(Format::Dst));
}

#[test]
fn content_signature_wins_over_extension() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "mislabelled.dst", Format::Jef);
    emb()
        .arg("info")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Format:   JEF"))
        .stderr(predicate::str::contains("contains JEF data"));
}

#[test]
fn output_format_can_be_forced() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.jef", Format::Jef);
    let output = dir.path().join("out.bin");
    emb()
        .args(["convert", "--to", "jef", "--center"])
        .arg(&input)
        .arg(&output)
        .assert()
        .success();
    assert_eq!(detected(&output), Some(Format::Jef));
}

/// A DST whose extended header declares two near-black threads.
fn near_blacks_dst(dir: &TempDir) -> PathBuf {
    let mut bytes = b"LA:blacks\rTC:#000000,Black,\rTC:#050505,Anthracite,\r\x1a".to_vec();
    bytes.resize(512, b' ');
    bytes.extend([
        0x00, 0x00, 0x03, 0x00, 0x00, 0xC3, 0x01, 0x00, 0x03, 0x00, 0x00, 0xF3,
    ]);
    let path = dir.path().join("blacks.dst");
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn colour_strategy_can_be_chosen_for_jef_output() {
    let dir = TempDir::new().unwrap();
    let input = near_blacks_dst(&dir);
    let output = dir.path().join("out.jef");
    let shared = "all use palette colour 1";
    emb()
        .args(["convert", "--colors", "nearest"])
        .arg(&input)
        .arg(&output)
        .assert()
        .code(EXIT_WARNINGS)
        .stderr(predicate::str::contains(shared));
    emb()
        .arg("convert")
        .arg(&input)
        .arg(&output)
        .assert()
        .stderr(predicate::str::contains(shared).not());
    emb()
        .args(["convert", "--colors", "injective"])
        .arg(&input)
        .arg(dir.path().join("out.dst"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--colors only applies to JEF"));
}

#[test]
fn enhance_block_is_written_by_default_and_can_be_disabled() {
    let dir = TempDir::new().unwrap();
    let input = near_blacks_dst(&dir);
    let contains_enhance =
        |path: &Path| fs::read(path).unwrap().windows(7).any(|w| w == b"ENHANCE");
    let with = dir.path().join("with.jef");
    emb()
        .arg("convert")
        .arg(&input)
        .arg(&with)
        .assert()
        .stdout(predicate::str::contains("(JEF)"));
    assert!(contains_enhance(&with));
    let without = dir.path().join("without.jef");
    emb()
        .args(["convert", "--no-enhance"])
        .arg(&input)
        .arg(&without)
        .assert()
        .stdout(predicate::str::contains("(JEF)"));
    assert!(!contains_enhance(&without));
    emb()
        .args(["convert", "--no-enhance"])
        .arg(&input)
        .arg(dir.path().join("out.dst"))
        .assert()
        .code(1);
}

#[test]
fn info_lists_threads_and_statistics() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.jef", Format::Jef);
    emb()
        .arg("info")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("3 stitches, 2 colour block(s)"))
        .stdout(predicate::str::contains("#FF0000  Red Janome 225"))
        .stdout(predicate::str::contains("Hoop:     50 x 50 mm"));
}

#[test]
fn colourless_input_reports_default_colours() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.dst", Format::Dst);
    emb()
        .arg("info")
        .arg(&input)
        .assert()
        .code(EXIT_WARNINGS)
        .stdout(predicate::str::contains("Name:     cli"))
        .stderr(predicate::str::contains("default colours assigned"));
}

#[test]
fn errors_exit_with_failure() {
    let dir = TempDir::new().unwrap();
    let input = write_fixture(&dir, "in.jef", Format::Jef);
    emb()
        .arg("convert")
        .arg(&input)
        .arg(dir.path().join("out.xyz"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("use --to"));
    emb()
        .args(["convert", "--center"])
        .arg(&input)
        .arg(dir.path().join("out.dst"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("only applies to JEF"));
    emb()
        .arg("info")
        .arg(dir.path().join("missing.jef"))
        .assert()
        .code(1);
    let garbage = dir.path().join("garbage.bin");
    fs::write(&garbage, b"not an embroidery file").unwrap();
    emb()
        .arg("info")
        .arg(&garbage)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("cannot recognise"));
}

#[test]
fn invalid_usage_exits_with_status_2() {
    emb().arg("convert").assert().code(2);
    emb()
        .args(["convert", "--to", "svg", "a", "b"])
        .assert()
        .code(2);
}
