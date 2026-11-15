//! Golden files: exact bytes written for reference designs. Any change to
//! these snapshots must be explained and validated by a human.

use std::fmt::Write as _;

use emb_core::StitchPattern;
use emb_formats::Format;

use crate::fixtures;
use crate::write;

fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (line, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<_> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let text: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    char::from(b)
                } else {
                    '.'
                }
            })
            .collect();
        writeln!(out, "{:06x}  {:<47}  {text}", line * 16, hex.join(" ")).unwrap();
    }
    out
}

fn golden(format: Format, pattern: &StitchPattern) -> String {
    let (bytes, output) = write(format, pattern);
    let mut text = String::from("warnings:\n");
    for warning in &output.warnings {
        writeln!(text, "  {warning}").unwrap();
    }
    text.push_str(&hex_dump(&bytes));
    text
}

#[test]
fn f_glyph_dst() {
    insta::assert_snapshot!(golden(Format::Dst, &fixtures::f_glyph()));
}

#[test]
fn f_glyph_jef() {
    insta::assert_snapshot!(golden(Format::Jef, &fixtures::f_glyph()));
}

#[test]
fn long_moves_and_commands_dst() {
    insta::assert_snapshot!(golden(Format::Dst, &fixtures::long_moves_and_commands()));
}

#[test]
fn long_moves_and_commands_jef() {
    insta::assert_snapshot!(golden(Format::Jef, &fixtures::long_moves_and_commands()));
}

#[test]
fn near_neutrals_jef() {
    insta::assert_snapshot!(golden(Format::Jef, &fixtures::near_neutrals()));
}
