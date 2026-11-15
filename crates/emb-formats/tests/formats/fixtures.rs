//! Synthetic designs exercising the conversion invariants (CONVERSION §4.5).

use emb_core::{
    Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread, Timestamp, delta_e_2000,
};

pub fn at(kind: StitchKind, x: f64, y: f64) -> Stitch {
    Stitch::new(kind, Point::new(Mm(x), Mm(y)))
}

fn pattern(stitches: Vec<Stitch>, threads: Vec<Thread>, name: &str) -> StitchPattern {
    let mut pattern = StitchPattern::new(stitches, threads).unwrap();
    pattern.metadata.name = Some(name.to_owned());
    pattern.metadata.date = Timestamp::parse_compact("20261022120000");
    pattern
}

/// `segments` evenly spaced running stitches from `from` (excluded) to `to`.
fn run(from: (f64, f64), to: (f64, f64), segments: u32) -> Vec<Stitch> {
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

/// The letter "F", 20 × 30 mm: any mirroring or axis swap changes it.
pub fn f_glyph() -> StitchPattern {
    let mut stitches = vec![at(StitchKind::Normal, 0.0, 0.0)];
    stitches.extend(run((0.0, 0.0), (0.0, 30.0), 12));
    stitches.extend(run((0.0, 30.0), (20.0, 30.0), 8));
    stitches.push(at(StitchKind::Trim, 20.0, 30.0));
    stitches.push(at(StitchKind::Jump, 0.0, 15.0));
    stitches.extend(run((0.0, 15.0), (12.0, 15.0), 5));
    pattern(
        stitches,
        vec![Thread::from_color(Rgb::new(0x0B, 0x2F, 0x84))],
        "F",
    )
}

/// One small square per thread, separated by trims and colour changes.
fn blocks_of(threads: Vec<Thread>, name: &str) -> StitchPattern {
    let mut stitches = Vec::new();
    for i in 0..threads.len() {
        let x = f64::from(u32::try_from(i % 10).unwrap()) * 4.0 - 20.0;
        let y = f64::from(u32::try_from(i / 10).unwrap()) * 4.0 - 16.0;
        if let Some(last) = stitches.last().map(|s: &Stitch| s.position) {
            stitches.push(Stitch::new(StitchKind::ColorChange, last));
        }
        stitches.push(at(StitchKind::Jump, x, y));
        for (dx, dy) in [(0.0, 0.0), (3.0, 0.0), (3.0, 3.0), (0.0, 3.0), (0.0, 0.0)] {
            stitches.push(at(StitchKind::Normal, x + dx, y + dy));
        }
        stitches.push(at(StitchKind::Trim, x, y));
    }
    pattern(stitches, threads, name)
}

/// Eight blues, each 1.5 to 3 ΔE00 away from the previous one.
pub fn near_blues_colors() -> Vec<Rgb> {
    let mut colors = vec![Rgb::new(20, 40, 140)];
    while colors.len() < 8 {
        let last = *colors.last().unwrap();
        let next = (1..=30)
            .map(|k| Rgb::new(last.r + k, last.g + 2 * k, last.b + 3 * k))
            .find(|c| delta_e_2000(last.to_lab(), c.to_lab()).get() >= 2.0)
            .unwrap();
        colors.push(next);
    }
    colors
}

pub fn near_blues() -> StitchPattern {
    let threads = near_blues_colors()
        .into_iter()
        .map(Thread::from_color)
        .collect();
    blocks_of(threads, "near blues")
}

/// Two off-whites and two blacks (black and very dark anthracite).
pub fn near_neutrals() -> StitchPattern {
    let threads = [
        Rgb::new(250, 249, 242),
        Rgb::new(244, 242, 232),
        Rgb::new(0, 0, 0),
        Rgb::new(28, 28, 30),
    ]
    .into_iter()
    .map(Thread::from_color)
    .collect();
    blocks_of(threads, "neutrals")
}

/// More distinct threads (80) than the Janome palette (78).
pub fn more_threads_than_palette() -> StitchPattern {
    let threads = (0..80_u8)
        .map(|i| {
            let blue = u8::try_from(u16::from(i) * 7 % 255).unwrap();
            let mut thread = Thread::from_color(Rgb::new(i * 3, 255 - i * 3, blue));
            thread.catalog_number = Some(format!("T{i}"));
            thread
        })
        .collect();
    blocks_of(threads, "80 threads")
}

/// Long moves (split into several records), stops and sequins.
pub fn long_moves_and_commands() -> StitchPattern {
    let stitches = vec![
        at(StitchKind::Normal, 0.0, 0.0),
        at(StitchKind::Normal, 30.0, -25.0),
        at(StitchKind::Jump, -60.0, 40.0),
        at(StitchKind::Normal, -60.5, 40.25),
        at(StitchKind::Stop, -60.5, 40.25),
        at(StitchKind::Normal, -59.0, 41.0),
        at(StitchKind::SequinEject, -58.0, 41.0),
        at(StitchKind::Trim, -58.0, 41.0),
        at(StitchKind::ColorChange, 10.0, 10.0),
        at(StitchKind::Normal, 10.04, 10.06),
        at(StitchKind::End, 10.04, 10.06),
    ];
    let threads = vec![
        Thread::from_color(Rgb::new(200, 30, 30)),
        Thread::from_color(Rgb::new(30, 200, 30)),
    ];
    pattern(stitches, threads, "long moves")
}

/// Every fixture, with a name for assertion messages.
pub fn all() -> Vec<(&'static str, StitchPattern)> {
    vec![
        ("f_glyph", f_glyph()),
        ("near_blues", near_blues()),
        ("near_neutrals", near_neutrals()),
        ("more_threads_than_palette", more_threads_than_palette()),
        ("long_moves_and_commands", long_moves_and_commands()),
    ]
}

#[test]
fn near_blues_are_close_but_distinct() {
    for pair in near_blues_colors().windows(2) {
        let delta = delta_e_2000(pair[0].to_lab(), pair[1].to_lab()).get();
        assert!(
            (1.5..=3.0).contains(&delta),
            "{} -> {}: {delta}",
            pair[0],
            pair[1]
        );
    }
}
