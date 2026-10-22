//! Integration tests of the format readers and writers: round-trip
//! properties, the format conversion matrix, golden files and the private
//! corpus of real files.

// Test helpers fail loudly on purpose; `clippy.toml` only covers `#[test]` functions.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod fixtures;
mod golden;
mod matrix;
mod private_corpus;
mod roundtrip;

use emb_core::StitchPattern;
use emb_formats::{Format, ReadOutput, WriteOutput};

fn write(format: Format, pattern: &StitchPattern) -> (Vec<u8>, WriteOutput) {
    let mut bytes = Vec::new();
    let output = format
        .writer()
        .unwrap_or_else(|| panic!("{format} is not writable"))
        .write(pattern, &mut bytes)
        .unwrap_or_else(|e| panic!("{format} write failed: {e}"));
    (bytes, output)
}

fn read(format: Format, bytes: &[u8]) -> ReadOutput {
    format
        .reader()
        .read(&mut &bytes[..])
        .unwrap_or_else(|e| panic!("{format} read failed: {e}"))
}
