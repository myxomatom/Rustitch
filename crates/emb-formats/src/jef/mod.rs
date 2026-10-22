//! Janome JEF. Structure, sources and open questions: `docs/formats/jef.md`.

mod enhance;
pub mod palette;

use std::collections::BTreeMap;
use std::io::{Read, Write};

use emb_core::{
    HoopSize, MatchStrategy, Mm, StitchKind, StitchPattern, Thread, Timestamp, delta_e_2000,
};

use crate::capabilities::{ColorModel, FormatCapabilities, Hoop, TrimSupport};
use crate::codec::{FormatReader, FormatWriter, ReadOutput, WriteOutput};
use crate::error::FormatError;
use crate::machine::{MachinePattern, MachineStitch, dequantize, machine_block_count, quantize};
use crate::normalize::{Normalized, normalize};
use crate::threads::{default_thread, reconcile_threads};
use crate::warning::Warning;

use palette::{JefColor, match_threads, palette_entry, palette_thread};

const FORMAT: &str = "JEF";
const UNITS_PER_MM: u32 = 10;
/// Size of the fixed part of the header; the colour tables follow.
const HEADER_SIZE: usize = 0x74;
const FORMAT_FLAGS: i64 = 0x14;
const THREAD_TYPE: i64 = 0x0D;
const DATE_OFFSET: usize = 0x08;
const DATE_LENGTH: usize = 14;
const COLOR_COUNT_OFFSET: usize = 0x18;
const HOOP_OFFSET: usize = 0x20;
/// Zero-length jump written before every run of jumps, as observed in real
/// files (see `docs/formats/jef.md`).
const JUMP_RUN_PREFIX: [u8; 4] = [ESCAPE, JUMP, 0, 0];
const ESCAPE: u8 = 0x80;
const COLOR_CHANGE: u8 = 0x01;
const JUMP: u8 = 0x02;
const END: u8 = 0x10;
/// ΔE00 above which a palette substitution is reported (FEATURES §3.4).
const APPROXIMATION_WARNING_DELTA_E: f64 = 3.0;
/// Half sizes, in machine units, of the hoops whose edge distances the
/// header records (110 × 110, 50 × 50, 140 × 200 twice).
const EDGE_HOOPS: [(i64, i64); 4] = [(550, 550), (250, 250), (700, 1000), (700, 1000)];

const fn hoop(name: &'static str, code: i32, width: f64, height: f64) -> Hoop {
    Hoop {
        name,
        code,
        width: Mm(width),
        height: Mm(height),
    }
}

/// Janome hoops known to the JEF header, smallest first. Codes are to be
/// confirmed on real files (sources disagree, see `docs/formats/jef.md`).
pub const JEF_HOOPS: [Hoop; 5] = [
    hoop("50 x 50 mm", 1, 50.0, 50.0),
    hoop("110 x 110 mm", 0, 110.0, 110.0),
    hoop("126 x 110 mm", 3, 126.0, 110.0),
    hoop("140 x 200 mm", 2, 140.0, 200.0),
    hoop("200 x 200 mm", 4, 200.0, 200.0),
];

/// Capabilities of the JEF writer.
pub const JEF_CAPABILITIES: FormatCapabilities = FormatCapabilities {
    name: FORMAT,
    units_per_mm: UNITS_PER_MM,
    max_delta: 127,
    trims: TrimSupport::NotStored,
    stops: false,
    sequins: false,
    color_model: ColorModel::PaletteIndex {
        palette_size: palette::JANOME_PALETTE.len(),
    },
    max_colors: None,
    max_stitches: None,
    hoops: &JEF_HOOPS,
};

/// Where the design goes in the hoop. The hoop centre is the origin of JEF
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Keep the pivot coordinates: the pivot origin is the hoop centre.
    #[default]
    Keep,
    /// Translate the design (by whole machine units) so that its bounding
    /// box is centred in the hoop.
    Center,
}

/// Reads Janome JEF files.
#[derive(Debug, Clone, Copy, Default)]
pub struct JefReader;

/// Writes Janome JEF files.
#[derive(Debug, Clone, Copy)]
pub struct JefWriter {
    /// Placement of the design in the hoop.
    pub placement: Placement,
    /// How threads are matched to the Janome palette (injective by default).
    pub colors: MatchStrategy,
    /// Append the `ENHANCE` block with the real threads (colour, catalogue
    /// number, name) after the end marker (on by default).
    pub enhance: bool,
}

impl Default for JefWriter {
    fn default() -> Self {
        Self {
            placement: Placement::default(),
            colors: MatchStrategy::default(),
            enhance: true,
        }
    }
}

/// Key of [`StitchPattern::extensions`] holding the Janome palette index of
/// each colour block (one byte per block), as read from a JEF header. The
/// writer reuses them when they still match the design's blocks, so that a
/// JEF → JEF conversion keeps the original colour choices.
pub const PALETTE_EXTENSION: &str = "jef.palette";

/// Whether `data` starts like a JEF file: the stitch offset must point right
/// after the two colour tables.
#[must_use]
pub fn looks_like_jef(data: &[u8]) -> bool {
    let (Some(offset), Some(colors)) = (read_i32(data, 0), read_i32(data, COLOR_COUNT_OFFSET))
    else {
        return false;
    };
    let expected = i64::from(colors) * 8 + i64::try_from(HEADER_SIZE).unwrap_or(i64::MAX);
    colors >= 0
        && i64::from(offset) == expected
        && usize::try_from(offset).is_ok_and(|o| o <= data.len())
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

fn byte(value: i8) -> u8 {
    u8::from_le_bytes(value.to_le_bytes())
}

/// Distances from the hoop centre (the origin) to the design edges, in
/// machine units; never negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Extents {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl Extents {
    fn of(machine: &MachinePattern) -> Self {
        machine.bounds().map_or_else(Self::default, |b| Self {
            left: (-i64::from(b.min_x)).max(0),
            top: i64::from(b.max_y).max(0),
            right: i64::from(b.max_x).max(0),
            bottom: (-i64::from(b.min_y)).max(0),
        })
    }

    fn half_width(&self) -> i64 {
        self.left.max(self.right)
    }

    fn half_height(&self) -> i64 {
        self.top.max(self.bottom)
    }
}

impl FormatWriter for JefWriter {
    fn capabilities(&self) -> &FormatCapabilities {
        &JEF_CAPABILITIES
    }

    fn write(
        &self,
        pattern: &StitchPattern,
        out: &mut dyn Write,
    ) -> Result<WriteOutput, FormatError> {
        let mut machine = MachinePattern::from_pattern(pattern, UNITS_PER_MM)?;
        if self.placement == Placement::Center {
            machine = machine.centered(UNITS_PER_MM)?;
        }
        let Normalized {
            pattern: machine,
            mut warnings,
        } = normalize(machine, &JEF_CAPABILITIES)?;
        let extents = Extents::of(&machine);
        let hoop = choose_hoop(&extents)?;
        let preserved = pattern.extensions.get(PALETTE_EXTENSION).map(Vec::as_slice);
        let colors = choose_colors(machine.threads(), preserved, self.colors, &mut warnings);
        let body = encode_stitches(&machine)?;
        let header = encode_header(&HeaderFields {
            colors: &colors,
            point_count: body.len() / 2,
            hoop_code: hoop.code,
            extents,
            date: pattern.metadata.date,
        })?;
        out.write_all(&header)?;
        out.write_all(&body)?;
        if self.enhance && !machine.threads().is_empty() {
            out.write_all(&enhance::encode(machine.threads(), &mut warnings))?;
        }
        Ok(WriteOutput { warnings })
    }
}

/// Smallest hoop that contains the design around its centre.
fn choose_hoop(extents: &Extents) -> Result<&'static Hoop, FormatError> {
    for hoop in &JEF_HOOPS {
        let width = i64::from(quantize(hoop.width, UNITS_PER_MM)?);
        let height = i64::from(quantize(hoop.height, UNITS_PER_MM)?);
        if 2 * extents.half_width() <= width && 2 * extents.half_height() <= height {
            return Ok(hoop);
        }
    }
    let to_mm = |units: i64| dequantize(i32::try_from(units).unwrap_or(i32::MAX), UNITS_PER_MM);
    Err(FormatError::DesignTooLarge {
        format: FORMAT,
        half_width: to_mm(extents.half_width()),
        half_height: to_mm(extents.half_height()),
    })
}

/// One palette index per block, reporting approximations and distinct
/// threads that end up with the same index.
fn choose_colors(
    threads: &[Thread],
    preserved: Option<&[u8]>,
    strategy: MatchStrategy,
    warnings: &mut Vec<Warning>,
) -> Vec<u8> {
    let matches =
        preserved_colors(threads, preserved).unwrap_or_else(|| match_threads(threads, strategy));
    let mut blocks_by_index: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (block, m) in matches.iter().enumerate() {
        blocks_by_index.entry(m.index).or_default().push(block);
        if let Some(delta_e) = m
            .delta_e
            .filter(|d| d.get() > APPROXIMATION_WARNING_DELTA_E)
        {
            warnings.push(Warning::ThreadApproximated {
                block,
                palette_index: m.index,
                delta_e,
            });
        }
    }
    for (palette_index, blocks) in blocks_by_index {
        let mut block_threads = blocks.iter().filter_map(|&b| threads.get(b));
        let first = block_threads.next();
        let distinct = first.is_some_and(|f| block_threads.any(|t| !t.is_same_thread(f)));
        if distinct {
            warnings.push(Warning::ThreadsShareColor {
                palette_index,
                blocks,
            });
        }
    }
    matches.into_iter().map(|m| m.index).collect()
}

/// The palette indices read from a JEF header, if they still describe one
/// valid entry per block (see [`PALETTE_EXTENSION`]).
fn preserved_colors(threads: &[Thread], preserved: Option<&[u8]>) -> Option<Vec<JefColor>> {
    let indices = preserved.filter(|p| p.len() == threads.len())?;
    threads
        .iter()
        .zip(indices)
        .map(|(thread, &index)| {
            let entry = palette_entry(index.into())?;
            Some(JefColor {
                index,
                delta_e: Some(delta_e_2000(thread.color.to_lab(), entry.color.to_lab())),
            })
        })
        .collect()
}

fn displacement(stitch: &MachineStitch, x: i32, y: i32) -> Result<(u8, u8), FormatError> {
    let dx = i64::from(stitch.x) - i64::from(x);
    let dy = i64::from(stitch.y) - i64::from(y);
    let max = i64::from(JEF_CAPABILITIES.max_delta);
    let encode = |d: i64| {
        i8::try_from(d)
            .ok()
            .filter(|_| d.abs() <= max)
            .map(byte)
            .ok_or(FormatError::DisplacementTooLong {
                format: FORMAT,
                dx,
                dy,
            })
    };
    Ok((encode(dx)?, encode(dy)?))
}

fn encode_stitches(machine: &MachinePattern) -> Result<Vec<u8>, FormatError> {
    let mut data = Vec::with_capacity(2 * machine.stitches().len() + 2);
    let (mut x, mut y) = (0, 0);
    let mut in_jump_run = false;
    for stitch in machine.stitches() {
        let (dx, dy) = displacement(stitch, x, y)?;
        let is_jump = matches!(stitch.kind, StitchKind::Jump | StitchKind::SequinEject);
        if is_jump && !in_jump_run {
            data.extend(JUMP_RUN_PREFIX);
        }
        in_jump_run = is_jump;
        match stitch.kind {
            StitchKind::Normal => data.extend([dx, dy]),
            StitchKind::Jump | StitchKind::SequinEject => data.extend([ESCAPE, JUMP, dx, dy]),
            StitchKind::ColorChange | StitchKind::Stop => {
                data.extend([ESCAPE, COLOR_CHANGE, dx, dy]);
            }
            // Removed by normalisation: the format has no trim command.
            StitchKind::Trim | StitchKind::End => {}
        }
        (x, y) = (stitch.x, stitch.y);
    }
    data.extend([ESCAPE, END]);
    Ok(data)
}

struct HeaderFields<'a> {
    colors: &'a [u8],
    point_count: usize,
    hoop_code: i32,
    extents: Extents,
    date: Option<Timestamp>,
}

fn put_i32(header: &mut Vec<u8>, field: &'static str, value: i64) -> Result<(), FormatError> {
    let value = i32::try_from(value).map_err(|_| FormatError::HeaderFieldOverflow {
        format: FORMAT,
        field,
        value,
    })?;
    header.extend(value.to_le_bytes());
    Ok(())
}

fn count(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn encode_header(fields: &HeaderFields<'_>) -> Result<Vec<u8>, FormatError> {
    let color_count = fields.colors.len();
    let stitch_offset = HEADER_SIZE + 8 * color_count;
    let mut header = Vec::with_capacity(stitch_offset);
    put_i32(&mut header, "stitch offset", count(stitch_offset))?;
    put_i32(&mut header, "flags", FORMAT_FLAGS)?;
    let date = fields
        .date
        .map_or_else(|| "0".repeat(DATE_LENGTH), |d| d.to_compact_string());
    header.extend(date.as_bytes());
    header.extend([0, 0]);
    put_i32(&mut header, "colour count", count(color_count))?;
    put_i32(&mut header, "point count", count(fields.point_count))?;
    put_i32(&mut header, "hoop", fields.hoop_code.into())?;
    let Extents {
        left,
        top,
        right,
        bottom,
    } = fields.extents;
    for value in [left, top, right, bottom] {
        put_i32(&mut header, "extents", value)?;
    }
    for (half_width, half_height) in EDGE_HOOPS {
        let edges = [
            half_width - left,
            half_height - top,
            half_width - right,
            half_height - bottom,
        ];
        let fits = edges.iter().all(|&e| e >= 0);
        for edge in edges {
            put_i32(&mut header, "hoop edges", if fits { edge } else { -1 })?;
        }
    }
    for &color in fields.colors {
        put_i32(&mut header, "colour", color.into())?;
    }
    for _ in fields.colors {
        put_i32(&mut header, "thread type", THREAD_TYPE)?;
    }
    Ok(header)
}

impl FormatReader for JefReader {
    fn read(&self, input: &mut dyn Read) -> Result<ReadOutput, FormatError> {
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;
        let header = parse_header(&data)?;
        let mut warnings = Vec::new();
        let stitch_data = data.get(header.stitch_offset..).unwrap_or_default();
        let (records, end) = decode_records(stitch_data, header.stitch_offset, &mut warnings);
        let stitches = build_stitches(&records);
        let palette_threads: Vec<Thread> = header
            .color_indices
            .iter()
            .enumerate()
            .map(|(block, &index)| {
                palette_entry(index).map_or_else(
                    || {
                        warnings.push(Warning::UnknownPaletteIndex { block, index });
                        default_thread(block)
                    },
                    palette_thread,
                )
            })
            .collect();
        let trailer_offset = end.map(|e| header.stitch_offset + e);
        let trailer = trailer_offset
            .and_then(|offset| data.get(offset..))
            .unwrap_or_default();
        let threads = real_threads(
            palette_threads,
            &header.color_indices,
            trailer,
            trailer_offset.unwrap_or_default(),
            &mut warnings,
        );
        let threads = reconcile_threads(threads, machine_block_count(&stitches), &mut warnings);
        let mut pattern = MachinePattern::new(stitches, threads)?.to_pattern(UNITS_PER_MM)?;
        let indices: Option<Vec<u8>> = header
            .color_indices
            .iter()
            .map(|&i| palette_entry(i).and_then(|_| u8::try_from(i).ok()))
            .collect();
        if let Some(indices) = indices.filter(|i| !i.is_empty()) {
            pattern
                .extensions
                .insert(PALETTE_EXTENSION.to_owned(), indices);
        }
        pattern.metadata.date = header.date;
        pattern.metadata.hoop = JEF_HOOPS
            .iter()
            .find(|h| h.code == header.hoop_code)
            .map(|h| HoopSize {
                width: h.width,
                height: h.height,
            });
        Ok(ReadOutput { pattern, warnings })
    }
}

/// The threads of the colour blocks: those of the `ENHANCE` block when it
/// holds one per block, the palette threads otherwise. A record identical to
/// its palette entry (colour and Janome code) is the palette thread itself.
fn real_threads(
    palette_threads: Vec<Thread>,
    indices: &[i64],
    trailer: &[u8],
    trailer_offset: usize,
    warnings: &mut Vec<Warning>,
) -> Vec<Thread> {
    let enhanced = match enhance::decode(trailer) {
        enhance::Trailer::Empty => return palette_threads,
        enhance::Trailer::Unknown => {
            warnings.push(Warning::UnreadTrailingData {
                offset: trailer_offset,
                bytes: trailer.len(),
            });
            return palette_threads;
        }
        enhance::Trailer::Unreadable => {
            warnings.push(Warning::ThreadListUnreadable);
            return palette_threads;
        }
        enhance::Trailer::Threads(threads) => threads,
    };
    if enhanced.len() != palette_threads.len() {
        warnings.push(Warning::ThreadCountMismatch {
            declared: enhanced.len(),
            blocks: palette_threads.len(),
        });
        return palette_threads;
    }
    enhanced
        .into_iter()
        .zip(indices)
        .zip(palette_threads)
        .map(|((thread, &index), palette_thread)| {
            let same_as_palette = palette_entry(index).is_some_and(|entry| {
                thread.color == entry.color
                    && thread
                        .catalog_number
                        .as_deref()
                        .and_then(|n| n.parse::<u32>().ok())
                        == entry.code.parse::<u32>().ok()
            });
            if same_as_palette {
                palette_thread
            } else {
                thread
            }
        })
        .collect()
}

struct ParsedHeader {
    stitch_offset: usize,
    color_indices: Vec<i64>,
    date: Option<Timestamp>,
    hoop_code: i32,
}

fn invalid(reason: &str) -> FormatError {
    FormatError::InvalidHeader {
        format: FORMAT,
        reason: reason.to_owned(),
    }
}

fn parse_header(data: &[u8]) -> Result<ParsedHeader, FormatError> {
    if data.len() < HEADER_SIZE {
        return Err(invalid("file shorter than the 116-byte header"));
    }
    let field = |offset| read_i32(data, offset).ok_or_else(|| invalid("truncated header"));
    let stitch_offset =
        usize::try_from(field(0)?).map_err(|_| invalid("negative stitch offset"))?;
    let color_count = usize::try_from(field(COLOR_COUNT_OFFSET)?)
        .map_err(|_| invalid("negative colour count"))?;
    let color_indices = (0..color_count)
        .map(|i| {
            let offset = i.checked_mul(4).and_then(|o| o.checked_add(HEADER_SIZE));
            offset
                .and_then(|o| read_i32(data, o))
                .map(i64::from)
                .ok_or_else(|| invalid("colour table extends beyond the end of the file"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if stitch_offset > data.len() {
        return Err(invalid("stitch data offset beyond the end of the file"));
    }
    let timestamp = data
        .get(DATE_OFFSET..DATE_OFFSET + DATE_LENGTH)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(Timestamp::parse_compact);
    Ok(ParsedHeader {
        stitch_offset,
        color_indices,
        date: timestamp,
        hoop_code: field(HOOP_OFFSET)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Record {
    Stitch(i32, i32),
    Jump(i32, i32),
    ColorChange(i32, i32),
}

fn signed(value: u8) -> i32 {
    i32::from(i8::from_le_bytes([value]))
}

/// Decodes records up to the end marker; `base` is the file offset of `data`.
/// Also returns the position right after the end marker, if one was found.
fn decode_records(
    data: &[u8],
    base: usize,
    warnings: &mut Vec<Warning>,
) -> (Vec<Record>, Option<usize>) {
    let mut records = Vec::with_capacity(data.len() / 2);
    let mut pos = 0;
    loop {
        let Some(&[first, second]) = data.get(pos..pos + 2) else {
            warnings.push(if pos < data.len() {
                Warning::TruncatedData { offset: base + pos }
            } else {
                Warning::MissingEndMarker
            });
            return (records, None);
        };
        if first != ESCAPE {
            records.push(Record::Stitch(signed(first), signed(second)));
            pos += 2;
            continue;
        }
        if second == END {
            return (records, Some(pos + 2));
        }
        if second != COLOR_CHANGE && second != JUMP {
            warnings.push(Warning::UnknownCommand {
                offset: base + pos,
                code: second,
            });
            return (records, None);
        }
        let Some(&[dx, dy]) = data.get(pos + 2..pos + 4) else {
            warnings.push(Warning::TruncatedData { offset: base + pos });
            return (records, None);
        };
        let (dx, dy) = (signed(dx), signed(dy));
        records.push(if second == JUMP {
            Record::Jump(dx, dy)
        } else {
            Record::ColorChange(dx, dy)
        });
        pos += 4;
    }
}

fn is_jump(record: &Record) -> bool {
    matches!(record, Record::Jump(..))
}

/// Converts records to absolute stitches, dropping the zero-length jump that
/// prefixes a run of jumps (an encoding artefact, see [`JUMP_RUN_PREFIX`]).
fn build_stitches(records: &[Record]) -> Vec<MachineStitch> {
    let mut out = Vec::with_capacity(records.len());
    let (mut x, mut y) = (0_i32, 0_i32);
    for group in records.chunk_by(|a, b| is_jump(a) && is_jump(b)) {
        let group = match group {
            [Record::Jump(0, 0), rest @ ..] => rest,
            _ => group,
        };
        for record in group {
            let (kind, dx, dy) = match *record {
                Record::Stitch(dx, dy) => (StitchKind::Normal, dx, dy),
                Record::Jump(dx, dy) => (StitchKind::Jump, dx, dy),
                Record::ColorChange(dx, dy) => (StitchKind::ColorChange, dx, dy),
            };
            (x, y) = (x.saturating_add(dx), y.saturating_add(dy));
            out.push(MachineStitch::new(kind, x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_core::{Point, Rgb, Stitch};

    fn at(kind: StitchKind, x: f64, y: f64) -> Stitch {
        Stitch::new(kind, Point::new(Mm(x), Mm(y)))
    }

    fn write_with(writer: JefWriter, pattern: &StitchPattern) -> (Vec<u8>, WriteOutput) {
        let mut bytes = Vec::new();
        let output = writer.write(pattern, &mut bytes).unwrap();
        (bytes, output)
    }

    fn black() -> Thread {
        Thread::from_color(Rgb::new(0, 0, 0))
    }

    fn header_i32(bytes: &[u8], offset: usize) -> i32 {
        read_i32(bytes, offset).unwrap()
    }

    #[test]
    fn header_layout_matches_reference_structure() {
        let mut pattern = StitchPattern::new(
            vec![
                at(StitchKind::Normal, -10.0, 5.0),
                at(StitchKind::Normal, 20.0, -15.0),
                at(StitchKind::ColorChange, 20.0, -15.0),
                at(StitchKind::Normal, 20.0, -14.0),
            ],
            vec![black(), Thread::from_color(Rgb::new(255, 255, 255))],
        )
        .unwrap();
        pattern.metadata.date = Timestamp::parse_compact("20261022120000");
        let (bytes, _) = write_with(JefWriter::default(), &pattern);
        assert!(looks_like_jef(&bytes));
        assert_eq!(header_i32(&bytes, 0), 0x74 + 16);
        assert_eq!(header_i32(&bytes, 4), 0x14);
        assert_eq!(&bytes[8..24], b"20261022120000\0\0");
        assert_eq!(header_i32(&bytes, 0x18), 2);
        // 3 stitches (2 bytes each) + jump run prefix (4) + 2 jumps (4 each)
        // + colour change (4) + end (2) = 24 bytes = 12 points.
        assert_eq!(header_i32(&bytes, 0x1C), 12);
        let end = 0x74 + 16 + 24;
        assert_eq!(&bytes[end - 2..end], &[0x80, 0x10]);
        assert_eq!(&bytes[end..end + 9], b"\0\0ENHANCE", "real threads follow");
        assert_eq!(header_i32(&bytes, 0x20), 1, "50 x 50 hoop");
        let extents: Vec<_> = (0..4).map(|i| header_i32(&bytes, 0x24 + 4 * i)).collect();
        assert_eq!(extents, vec![100, 50, 200, 150]);
        assert_eq!(header_i32(&bytes, 0x34), 450, "110 x 110 left edge");
        assert_eq!(header_i32(&bytes, 0x44), 150, "50 x 50 left edge");
        assert_eq!(header_i32(&bytes, 0x74), 1, "black");
        assert_eq!(header_i32(&bytes, 0x78), 2, "white");
        assert_eq!(header_i32(&bytes, 0x7C), 0x0D);
    }

    /// The "F" fixture: an asymmetric glyph whose mirror image would differ.
    #[test]
    fn axes_are_not_mirrored() {
        let pattern = StitchPattern::new(
            vec![
                at(StitchKind::Normal, 0.0, 0.0),
                at(StitchKind::Normal, 0.0, 1.0),
                at(StitchKind::Normal, 0.5, 1.0),
            ],
            vec![black()],
        )
        .unwrap();
        let (bytes, _) = write_with(JefWriter::default(), &pattern);
        let body = &bytes[0x74 + 8..];
        // Y up is +10, X right is +5.
        assert_eq!(&body[..6], &[0, 0, 0, 10, 5, 0]);
    }

    #[test]
    fn smallest_containing_hoop_is_chosen() {
        let extents = |half_w, half_h| Extents {
            left: half_w,
            top: half_h,
            right: 0,
            bottom: 0,
        };
        assert_eq!(choose_hoop(&extents(250, 250)).unwrap().code, 1);
        assert_eq!(choose_hoop(&extents(251, 250)).unwrap().code, 0);
        assert_eq!(choose_hoop(&extents(600, 500)).unwrap().code, 3);
        assert_eq!(choose_hoop(&extents(700, 1000)).unwrap().code, 2);
        assert_eq!(choose_hoop(&extents(1000, 1000)).unwrap().code, 4);
        assert!(matches!(
            choose_hoop(&extents(1001, 10)),
            Err(FormatError::DesignTooLarge { .. })
        ));
    }

    #[test]
    fn centering_translates_by_whole_units() {
        let pattern = StitchPattern::new(
            vec![
                at(StitchKind::Normal, 10.0, 10.0),
                at(StitchKind::Normal, 30.0, 20.0),
            ],
            vec![black()],
        )
        .unwrap();
        let writer = JefWriter {
            placement: Placement::Center,
            ..JefWriter::default()
        };
        let (bytes, _) = write_with(writer, &pattern);
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        let bounds = read.pattern.bounds().unwrap();
        assert!(bounds.min.x.approx_eq(Mm(-10.0), emb_core::EPSILON_MM));
        assert!(bounds.max.y.approx_eq(Mm(5.0), emb_core::EPSILON_MM));
    }

    #[test]
    fn distinct_threads_sharing_an_index_are_reported() {
        let near_black = Thread::from_color(Rgb::new(5, 5, 5));
        let pattern = StitchPattern::new(
            vec![
                at(StitchKind::Normal, 0.0, 0.0),
                at(StitchKind::ColorChange, 0.0, 0.0),
                at(StitchKind::Normal, 1.0, 0.0),
            ],
            vec![black(), near_black],
        )
        .unwrap();
        let nearest = JefWriter {
            colors: MatchStrategy::Nearest,
            ..JefWriter::default()
        };
        let (_, output) = write_with(nearest, &pattern);
        assert!(output.warnings.contains(&Warning::ThreadsShareColor {
            palette_index: 1,
            blocks: vec![0, 1]
        }));
        // The default injective matching keeps them apart.
        let (bytes, output) = write_with(JefWriter::default(), &pattern);
        assert!(
            !output
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::ThreadsShareColor { .. }))
        );
        assert_ne!(header_i32(&bytes, 0x74), header_i32(&bytes, 0x78));
    }

    #[test]
    fn far_colours_are_reported_as_approximated() {
        let pattern = StitchPattern::new(
            vec![at(StitchKind::Normal, 0.0, 0.0)],
            vec![Thread::from_color(Rgb::new(0x80, 0x80, 0x00))],
        )
        .unwrap();
        let (_, output) = write_with(JefWriter::default(), &pattern);
        assert!(
            output
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::ThreadApproximated { block: 0, .. }))
        );
    }

    #[test]
    fn jump_runs_get_a_zero_prefix_and_trims_are_reported() {
        let pattern = StitchPattern::new(
            vec![
                at(StitchKind::Normal, 1.0, 0.0),
                at(StitchKind::Trim, 1.0, 0.0),
                at(StitchKind::Jump, 2.0, 0.0),
                at(StitchKind::Jump, 3.0, 0.0),
                at(StitchKind::Normal, 3.0, 0.5),
                at(StitchKind::Jump, 3.0, 0.5),
            ],
            vec![black()],
        )
        .unwrap();
        let plain = JefWriter {
            enhance: false,
            ..JefWriter::default()
        };
        let (bytes, output) = write_with(plain, &pattern);
        assert_eq!(output.warnings, vec![Warning::TrimsNotStored { count: 1 }]);
        let body = &bytes[0x74 + 8..];
        // The default writer emits the same data followed by the ENHANCE block.
        let (enhanced, _) = write_with(JefWriter::default(), &pattern);
        assert!(enhanced.starts_with(&bytes));
        assert_eq!(&enhanced[bytes.len()..bytes.len() + 9], b"\0\0ENHANCE");
        assert_eq!(
            body,
            [
                10, 0, // stitch
                0x80, 0x02, 0, 0, // run prefix
                0x80, 0x02, 10, 0, 0x80, 0x02, 10, 0, // jumps
                0, 5, // stitch
                0x80, 0x02, 0, 0, // run prefix
                0x80, 0x02, 0, 0, // genuine zero-length jump
                0x80, 0x10,
            ]
        );
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        let kinds: Vec<_> = read.pattern.stitches().iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                StitchKind::Normal,
                StitchKind::Jump,
                StitchKind::Jump,
                StitchKind::Normal,
                StitchKind::Jump
            ]
        );
    }

    #[test]
    fn janome_threads_round_trip_exactly() {
        let navy = palette_thread(&palette::JANOME_PALETTE[29]);
        let pattern =
            StitchPattern::new(vec![at(StitchKind::Normal, 0.0, 0.0)], vec![navy.clone()]).unwrap();
        let (bytes, output) = write_with(JefWriter::default(), &pattern);
        assert!(output.warnings.is_empty(), "{:?}", output.warnings);
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        assert_eq!(read.pattern.threads(), &[navy]);
        assert_eq!(read.pattern.metadata.hoop.unwrap().width, Mm(50.0));
    }

    fn file_with(colors: &[i32], stitch_data: &[u8]) -> Vec<u8> {
        let offset = 0x74 + 8 * colors.len();
        let mut bytes = Vec::new();
        bytes.extend(i32::try_from(offset).unwrap().to_le_bytes());
        bytes.resize(0x18, 0);
        bytes.extend(i32::try_from(colors.len()).unwrap().to_le_bytes());
        bytes.resize(0x74, 0);
        for c in colors {
            bytes.extend(c.to_le_bytes());
        }
        bytes.resize(offset, 0);
        bytes.extend(stitch_data);
        bytes
    }

    #[test]
    fn unknown_palette_index_gets_a_default_colour() {
        let bytes = file_with(&[99], &[1, 1, 0x80, 0x10]);
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        assert_eq!(
            read.warnings,
            vec![Warning::UnknownPaletteIndex {
                block: 0,
                index: 99
            }]
        );
    }

    #[test]
    fn truncated_command_keeps_previous_stitches() {
        let bytes = file_with(&[1], &[1, 1, 0x80, 0x02, 5]);
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        assert_eq!(read.pattern.stitches().len(), 1);
        assert_eq!(
            read.warnings,
            vec![Warning::TruncatedData {
                offset: 0x74 + 8 + 2
            }]
        );
    }

    #[test]
    fn unknown_command_stops_reading_with_a_warning() {
        let bytes = file_with(&[1], &[1, 1, 0x80, 0x07, 0, 0, 2, 2]);
        let read = JefReader.read(&mut bytes.as_slice()).unwrap();
        assert_eq!(read.pattern.stitches().len(), 1);
        assert!(matches!(
            read.warnings[..],
            [Warning::UnknownCommand { code: 0x07, .. }]
        ));
    }

    #[test]
    fn colour_table_beyond_file_end_is_an_error() {
        let mut bytes = file_with(&[1], &[]);
        bytes[0x18] = 50;
        assert!(matches!(
            JefReader.read(&mut bytes.as_slice()),
            Err(FormatError::InvalidHeader { .. })
        ));
    }
}
