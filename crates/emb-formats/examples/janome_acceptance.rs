//! Generates the synthetic JEF files of the Janome machine acceptance
//! checklist (`docs/acceptance/janome.md`).
//!
//! ```text
//! cargo run -p emb-formats --example janome_acceptance -- <output directory>
//! ```
//!
//! Every design is deterministic (fixed date) and drawn here: the files can
//! be shared freely.

use std::error::Error;
use std::fs;
use std::path::Path;

use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread, Timestamp};
use emb_formats::FormatWriter;
use emb_formats::jef::palette::{JANOME_PALETTE, palette_thread};
use emb_formats::jef::{JefWriter, Placement};

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
    let (l, r, b, t) = (
        cx - width / 2.0,
        cx + width / 2.0,
        cy - height / 2.0,
        cy + height / 2.0,
    );
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
    pattern.metadata.date = Timestamp::parse_compact("20261022120000");
    Ok(pattern)
}

/// 01: an "F" and an arrow pointing to the top of the design.
fn orientation() -> Result<StitchPattern> {
    let mut stitches = path(&[(-10.0, -15.0), (-10.0, 15.0), (6.0, 15.0)], 2.5);
    stitches.extend(path(&[(-10.0, 0.0), (2.0, 0.0)], 2.5));
    stitches.extend(path(&[(15.0, -15.0), (15.0, 15.0)], 2.5));
    stitches.extend(path(&[(11.0, 10.0), (15.0, 15.0), (19.0, 10.0)], 2.0));
    design(stitches, vec![janome("002")?], "01 orientation")
}

/// 02: a rectangle just inside a hoop, to check the hoop the machine picks.
fn hoop(width: f64, height: f64, name: &str) -> Result<StitchPattern> {
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

/// 03: six numbered squares (1 to 6 stitches across) in six colours.
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
    design(blocks(parts), threads, "03 couleurs")
}

/// 04: three pairs of crosses joined by jumps of 3, 15 and 60 mm; the two
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
    design(stitches, vec![janome("002")?], "04 sauts coupes")
}

/// 05: zigzags with stitches of 5, 10 and 12.7 mm, then one 20 mm stitch
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
    design(stitches, vec![janome("002")?], "05 points longs")
}

/// 06: a small square away from the origin, to compare placements.
fn offset_square() -> Result<StitchPattern> {
    design(
        rectangle(30.0, 20.0, 20.0, 20.0, 2.5),
        vec![janome("002")?],
        "06 centrage",
    )
}

fn write(directory: &Path, file: &str, writer: JefWriter, pattern: &StitchPattern) -> Result<()> {
    let mut bytes = Vec::new();
    let output = writer.write(pattern, &mut bytes)?;
    fs::write(directory.join(file), bytes)?;
    println!("{file}");
    for warning in output.warnings {
        println!("    note: {warning}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: janome_acceptance <output directory>")?;
    let directory = Path::new(&directory);
    fs::create_dir_all(directory)?;
    let default = JefWriter::default();
    write(directory, "01-orientation.jef", default, &orientation()?)?;
    let hoops = [
        ("02a-cadre-50x50.jef", 46.0, 46.0),
        ("02b-cadre-110x110.jef", 104.0, 104.0),
        ("02c-cadre-126x110.jef", 120.0, 104.0),
        ("02d-cadre-140x200.jef", 134.0, 194.0),
        ("02e-cadre-200x200.jef", 194.0, 194.0),
    ];
    for (file, width, height) in hoops {
        write(directory, file, default, &hoop(width, height, file)?)?;
    }
    let colours = colours()?;
    write(
        directory,
        "03a-couleurs-avec-enhance.jef",
        default,
        &colours,
    )?;
    let plain = JefWriter {
        enhance: false,
        ..default
    };
    write(directory, "03b-couleurs-sans-enhance.jef", plain, &colours)?;
    write(
        directory,
        "04-sauts-coupes.jef",
        default,
        &jumps_and_trims()?,
    )?;
    write(directory, "05-points-longs.jef", default, &long_stitches()?)?;
    let square = offset_square()?;
    write(directory, "06a-centrage-conserve.jef", default, &square)?;
    let centred = JefWriter {
        placement: Placement::Center,
        ..default
    };
    write(directory, "06b-centrage-centre.jef", centred, &square)?;
    let mut undated = orientation()?;
    undated.metadata.date = None;
    write(directory, "07-sans-date.jef", default, &undated)?;
    Ok(())
}
