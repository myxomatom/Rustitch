//! Generates the JEF files of the Janome machine acceptance checklist
//! (`docs/acceptance/janome.md`), for a machine with the `SQ20b` (200 x 200),
//! `RE20b` (170 x 200) and `RE28b` (200 x 280) hoops.
//!
//! ```text
//! cargo run -p emb-formats --example janome_acceptance -- <usb directory> [real files...]
//! ```
//!
//! File names are two characters long (`01.jef`, `H4.jef`, `R1.jef`…), as
//! some machines require; `<usb directory>.txt` lists what each file is.
//! The synthetic designs are deterministic (fixed date) and drawn here.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread, Timestamp};
use emb_formats::convert::load;
use emb_formats::jef::palette::{JANOME_PALETTE, palette_thread};
use emb_formats::jef::{JefWriter, Placement};
use emb_formats::{FormatError, FormatWriter, Hoop};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn at(kind: StitchKind, x: f64, y: f64) -> Stitch {
    Stitch::new(kind, Point::new(Mm(x), Mm(y)))
}

/// Running stitches from `from` (excluded) to `to`, at most `step` mm long.
fn line(from: (f64, f64), to: (f64, f64), step: f64) -> Vec<Stitch> {
    let length = (to.0 - from.0).hypot(to.1 - from.1);
    let mut segments = 1_u32;
    while f64::from(segments) * step < length {
        segments += 1;
    }
    (1..=segments)
        .map(|i| {
            let t = f64::from(i) / f64::from(segments);
            at(
                StitchKind::Normal,
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
            )
        })
        .collect()
}

/// A polyline sewn with running stitches, starting with a jump to its start.
fn path(points: &[(f64, f64)], step: f64) -> Vec<Stitch> {
    let Some(&(x, y)) = points.first() else {
        return Vec::new();
    };
    let mut stitches = vec![at(StitchKind::Jump, x, y), at(StitchKind::Normal, x, y)];
    for pair in points.windows(2) {
        if let [from, to] = pair {
            stitches.extend(line(*from, *to, step));
        }
    }
    stitches
}

/// Closed rectangle outline centred on (`cx`, `cy`).
fn rectangle(cx: f64, cy: f64, width: f64, height: f64, step: f64) -> Vec<Stitch> {
    let (l, r) = (cx - width / 2.0, cx + width / 2.0);
    let (b, t) = (cy - height / 2.0, cy + height / 2.0);
    path(&[(l, b), (r, b), (r, t), (l, t), (l, b)], step)
}

/// Joins colour blocks with colour changes at the needle position.
fn blocks(parts: Vec<Vec<Stitch>>) -> Vec<Stitch> {
    let mut stitches: Vec<Stitch> = Vec::new();
    for part in parts {
        if let Some(last) = stitches.last().map(|s| s.position) {
            stitches.push(Stitch::new(StitchKind::ColorChange, last));
        }
        stitches.extend(part);
    }
    stitches
}

fn janome(code: &str) -> Result<Thread> {
    JANOME_PALETTE
        .iter()
        .find(|e| e.code == code)
        .map(palette_thread)
        .ok_or_else(|| format!("unknown Janome code {code}").into())
}

fn design(stitches: Vec<Stitch>, threads: Vec<Thread>, name: &str) -> Result<StitchPattern> {
    let mut pattern = StitchPattern::new(stitches, threads)?;
    pattern.metadata.name = Some(name.to_owned());
    pattern.metadata.date = Timestamp::parse_compact("20261023120000");
    Ok(pattern)
}

/// An "F" and an arrow pointing to the top of the design.
fn orientation() -> Result<StitchPattern> {
    let mut stitches = path(&[(-10.0, -15.0), (-10.0, 15.0), (6.0, 15.0)], 2.5);
    stitches.extend(path(&[(-10.0, 0.0), (2.0, 0.0)], 2.5));
    stitches.extend(path(&[(15.0, -15.0), (15.0, 15.0)], 2.5));
    stitches.extend(path(&[(11.0, 10.0), (15.0, 15.0), (19.0, 10.0)], 2.0));
    design(stitches, vec![janome("002")?], "orientation")
}

/// A rectangle with a cross at its centre.
fn frame(width: f64, height: f64, name: &str) -> Result<StitchPattern> {
    let mut stitches = rectangle(0.0, 0.0, width, height, 3.0);
    stitches.extend(path(&[(-5.0, 0.0), (5.0, 0.0)], 2.5));
    stitches.extend(path(&[(0.0, -5.0), (0.0, 5.0)], 2.5));
    design(stitches, vec![janome("002")?], name)
}

/// Threads of the colour test: Madeira-like threads that are not Janome
/// colours, so that the ENHANCE block carries information the palette lacks.
fn madeira(rgb: u32, code: &str, name: &str) -> Thread {
    Thread {
        name: Some(name.to_owned()),
        brand: Some("Madeira Classic 40".to_owned()),
        catalog_number: Some(code.to_owned()),
        ..Thread::from_color(Rgb::from_hex(rgb))
    }
}

/// Six squares in six colours.
fn colours() -> Result<StitchPattern> {
    let threads = vec![
        madeira(0xFC_C68D, "1026", "Orange Sorbet"),
        madeira(0xC6_240A, "1146", "Tomato Red"),
        madeira(0xA8_C3BC, "1219", "Hint of Mint"),
        madeira(0x1C_7E82, "1090", "Deep Sky Blue"),
        madeira(0x0E_4D4E, "1290", "Mallard Teal"),
        madeira(0x00_0000, "1000", "Black"),
    ];
    let parts = (0..6_u32)
        .map(|i| {
            let cx = f64::from(i % 3) * 14.0 - 14.0;
            let cy = if i < 3 { 7.0 } else { -7.0 };
            rectangle(cx, cy, 10.0, 10.0, 2.0)
        })
        .collect();
    design(blocks(parts), threads, "colours")
}

/// Three pairs of crosses joined by jumps of 3, 15 and 60 mm; the two
/// longer jumps are preceded by a trim in the source design.
fn jumps_and_trims() -> Result<StitchPattern> {
    let cross = |cx: f64, cy: f64| {
        let mut s = path(&[(cx - 3.0, cy), (cx + 3.0, cy)], 2.0);
        s.extend(path(&[(cx, cy - 3.0), (cx, cy + 3.0)], 2.0));
        s
    };
    let mut stitches = Vec::new();
    for (row, gap, trim) in [(20.0, 3.0, false), (0.0, 15.0, true), (-20.0, 60.0, true)] {
        let left = -gap / 2.0 - 3.0;
        stitches.extend(cross(left, row));
        if trim {
            let last = stitches
                .last()
                .map_or(Point::ORIGIN, |s: &Stitch| s.position);
            stitches.push(Stitch::new(StitchKind::Trim, last));
        }
        stitches.extend(cross(-left, row));
    }
    design(stitches, vec![janome("002")?], "jumps")
}

/// Zigzags with stitches of 5, 10 and 12.7 mm, then one 20 mm stitch
/// (written as jumps followed by a stitch: the JEF limit is 12.7 mm).
fn long_stitches() -> Result<StitchPattern> {
    let mut stitches = vec![
        at(StitchKind::Jump, -30.0, 20.0),
        at(StitchKind::Normal, -30.0, 20.0),
    ];
    for (row, length) in [(20.0, 5.0), (5.0, 10.0), (-10.0, 12.7)] {
        stitches.push(at(StitchKind::Jump, -30.0, row));
        for i in 0..8_u32 {
            let y = if i % 2 == 0 { row } else { row - length };
            stitches.push(at(StitchKind::Normal, -30.0 + f64::from(i) * 2.0, y));
        }
    }
    stitches.push(at(StitchKind::Jump, -10.0, -25.0));
    stitches.push(at(StitchKind::Normal, -10.0, -25.0));
    stitches.push(at(StitchKind::Normal, 10.0, -25.0));
    design(stitches, vec![janome("002")?], "long stitches")
}

/// A small square away from the origin, to compare placements.
fn offset_square() -> Result<StitchPattern> {
    design(
        rectangle(30.0, 20.0, 20.0, 20.0, 2.5),
        vec![janome("002")?],
        "offset",
    )
}

/// Hoop with a known size and a given JEF code.
const fn hoop(name: &'static str, code: i32, width: f64, height: f64) -> Hoop {
    Hoop {
        name,
        code,
        width: Mm(width),
        height: Mm(height),
    }
}

/// The codes of the `SQ20b`, `RE20b` and `RE28b` hoops are unknown (the H files are
/// there to find them): until then, every file declares code 4, the
/// 200 x 200 hoop of pyembroidery's table.
const DEFAULT_CODE: i32 = 4;
const SQ20B: Hoop = hoop("SQ20b 200 x 200 mm", DEFAULT_CODE, 200.0, 200.0);
const RE20B: Hoop = hoop("RE20b 170 x 200 mm", DEFAULT_CODE, 170.0, 200.0);
const RE28B: Hoop = hoop("RE28b 200 x 280 mm", DEFAULT_CODE, 200.0, 280.0);

/// Codes written by the hoop probes: those of pyembroidery's table and
/// their neighbours (0 to 8), and those seen in real files (14, 40, 65535).
const PROBES: [(&str, i32); 12] = [
    ("H0", 0),
    ("H1", 1),
    ("H2", 2),
    ("H3", 3),
    ("H4", 4),
    ("H5", 5),
    ("H6", 6),
    ("H7", 7),
    ("H8", 8),
    ("HA", 14),
    ("HB", 40),
    ("HC", 65535),
];

/// Writes `<name>.jef` and records it in `index`.
fn write(
    directory: &Path,
    index: &mut String,
    (name, description): (&str, &str),
    writer: JefWriter,
    pattern: &StitchPattern,
) -> Result<()> {
    let mut bytes = Vec::new();
    let output = writer.write(pattern, &mut bytes)?;
    fs::write(directory.join(format!("{name}.jef")), bytes)?;
    writeln!(index, "{name}.jef  {description}")?;
    for warning in output.warnings {
        writeln!(index, "            note: {warning}")?;
    }
    Ok(())
}

/// Two-character name of the `n`-th real file: R1 to R9, then RA to RZ.
fn real_name(n: usize) -> Result<String> {
    let digit = char::from_digit(u32::try_from(n + 1)?, 36).ok_or("too many real files")?;
    Ok(format!("R{}", digit.to_ascii_uppercase()))
}

fn with_hoop(hoop: Hoop) -> JefWriter {
    JefWriter {
        hoop: Some(hoop),
        ..JefWriter::default()
    }
}

fn synthetic(directory: &Path, index: &mut String) -> Result<()> {
    let sq20b = with_hoop(SQ20B);
    let orientation = orientation()?;
    write(
        directory,
        index,
        ("01", "orientation: F and arrow up"),
        sq20b,
        &orientation,
    )?;
    let colours = colours()?;
    write(
        directory,
        index,
        ("02", "6 colours, with ENHANCE"),
        sq20b,
        &colours,
    )?;
    let plain = JefWriter {
        enhance: false,
        ..sq20b
    };
    write(
        directory,
        index,
        ("03", "6 colours, without ENHANCE"),
        plain,
        &colours,
    )?;
    write(
        directory,
        index,
        ("04", "jumps of 3, 15, 60 mm"),
        sq20b,
        &jumps_and_trims()?,
    )?;
    write(
        directory,
        index,
        ("05", "long stitches"),
        sq20b,
        &long_stitches()?,
    )?;
    let square = offset_square()?;
    write(
        directory,
        index,
        ("06", "offset square, kept"),
        sq20b,
        &square,
    )?;
    let centred = JefWriter {
        placement: Placement::Center,
        ..sq20b
    };
    write(
        directory,
        index,
        ("07", "offset square, centred"),
        centred,
        &square,
    )?;
    let mut undated = orientation;
    undated.metadata.date = None;
    write(
        directory,
        index,
        ("08", "orientation, no date"),
        sq20b,
        &undated,
    )?;
    Ok(())
}

/// Hoop size tests and hoop code probes.
fn hoops(directory: &Path, index: &mut String) -> Result<()> {
    let sizes = [
        ("S1", "196 x 196 mm (SQ20b), code 4", 196.0, 196.0, SQ20B),
        ("S2", "166 x 196 mm (RE20b), code 4", 166.0, 196.0, RE20B),
        ("S3", "196 x 276 mm (RE28b), code 4", 196.0, 276.0, RE28B),
        (
            "S4",
            "196 x 276 mm (RE28b), code 65535",
            196.0,
            276.0,
            Hoop {
                code: 65535,
                ..RE28B
            },
        ),
    ];
    for (name, description, width, height, hoop) in sizes {
        let pattern = frame(width, height, name)?;
        write(
            directory,
            index,
            (name, description),
            with_hoop(hoop),
            &pattern,
        )?;
    }
    let probe = frame(30.0, 30.0, "probe")?;
    for (name, code) in PROBES {
        let description = format!("hoop code {code}");
        let writer = with_hoop(Hoop { code, ..SQ20B });
        write(directory, index, (name, &description), writer, &probe)?;
    }
    Ok(())
}

/// Converts the real files with the `SQ20b` hoop declared (code 4), or the
/// default hoop choice if the design does not fit in it.
fn real(directory: &Path, index: &mut String, files: &[String]) -> Result<()> {
    for (n, file) in files.iter().enumerate() {
        let path = Path::new(file);
        let extension = path.extension().and_then(|e| e.to_str());
        let mut pattern = load(&fs::read(path)?, extension)?.output.pattern;
        if pattern.metadata.date.is_none() {
            pattern.metadata.date = Timestamp::parse_compact("20261023120000");
        }
        let sq20b = with_hoop(SQ20B);
        let writer = match sq20b.write(&pattern, &mut Vec::new()) {
            Err(FormatError::DesignTooLarge { .. }) => JefWriter::default(),
            _ => sq20b,
        };
        write(directory, index, (&real_name(n)?, file), writer, &pattern)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let directory = args
        .next()
        .ok_or("usage: janome_acceptance <usb directory> [real files...]")?;
    let real_files: Vec<String> = args.collect();
    let directory = Path::new(&directory);
    fs::create_dir_all(directory)?;
    let mut index = String::new();
    synthetic(directory, &mut index)?;
    hoops(directory, &mut index)?;
    real(directory, &mut index, &real_files)?;
    fs::write(directory.with_extension("txt"), &index)?;
    print!("{index}");
    Ok(())
}
