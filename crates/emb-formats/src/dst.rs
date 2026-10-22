//! Tajima DST. Structure, sources and open questions: `docs/formats/dst.md`.

use std::io::{Read, Write};

use emb_core::{Metadata, Rgb, StitchKind, Thread};

use crate::capabilities::{ColorModel, FormatCapabilities, TrimSupport};
use crate::codec::{FormatReader, FormatWriter, ReadOutput, WriteOutput};
use crate::error::FormatError;
use crate::machine::{MachinePattern, MachineStitch, machine_block_count};
use crate::normalize::{Normalized, normalize};
use crate::threads::reconcile_threads;
use crate::warning::Warning;

const FORMAT: &str = "DST";
const UNITS_PER_MM: u32 = 10;
const HEADER_SIZE: usize = 512;
const NAME_LENGTH: usize = 16;
const MACHINE_TRIM_JUMPS: usize = 3;
const JUMP_FLAGS: u8 = 0x83;
const STITCH_FLAGS: u8 = 0x03;
const COLOR_CHANGE_FLAGS: u8 = 0xC3;
const END_RECORD: [u8; 3] = [0x00, 0x00, 0xF3];
/// Jump displacements (file axes) encoding one trim, as written by pyembroidery.
const TRIM_SIGNATURE: [(i32, i32); 3] = [(2, -2), (-4, 4), (2, -2)];

/// Capabilities of the DST writer.
pub const DST_CAPABILITIES: FormatCapabilities = FormatCapabilities {
    name: FORMAT,
    units_per_mm: UNITS_PER_MM,
    max_delta: 121,
    trims: TrimSupport::JumpSequence {
        machine_trims_after_jumps: MACHINE_TRIM_JUMPS,
    },
    stops: false,
    sequins: false,
    color_model: ColorModel::None,
    max_colors: None,
    max_stitches: None,
    hoops: &[],
};

/// Reads Tajima DST files.
#[derive(Debug, Clone, Copy, Default)]
pub struct DstReader;

/// Writes Tajima DST files.
#[derive(Debug, Clone, Copy, Default)]
pub struct DstWriter;

/// Whether `data` starts like a DST file.
#[must_use]
pub fn looks_like_dst(data: &[u8]) -> bool {
    data.len() >= HEADER_SIZE && data.starts_with(b"LA:")
}

/// One balanced-ternary digit of a displacement: `weight` is added when bit
/// `plus` of byte `byte` is set and subtracted when bit `minus` is set.
struct Trit {
    weight: i32,
    byte: usize,
    plus: u8,
    minus: u8,
}

const fn trit(weight: i32, byte: usize, plus: u8, minus: u8) -> Trit {
    Trit {
        weight,
        byte,
        plus,
        minus,
    }
}

/// X digits, largest weight first. File axes: X to the right.
const X_TRITS: [Trit; 5] = [
    trit(81, 2, 2, 3),
    trit(27, 1, 2, 3),
    trit(9, 0, 2, 3),
    trit(3, 1, 0, 1),
    trit(1, 0, 0, 1),
];

/// Y digits, largest weight first. File axes: Y upwards, as in the pivot model.
const Y_TRITS: [Trit; 5] = [
    trit(81, 2, 5, 4),
    trit(27, 1, 5, 4),
    trit(9, 0, 5, 4),
    trit(3, 1, 7, 6),
    trit(1, 0, 7, 6),
];

fn encode_axis(mut value: i32, trits: &[Trit], bytes: &mut [u8; 3]) -> bool {
    for t in trits {
        let lower_weights = (t.weight - 1) / 2;
        let bit = if value > lower_weights {
            value -= t.weight;
            Some(t.plus)
        } else if value < -lower_weights {
            value += t.weight;
            Some(t.minus)
        } else {
            None
        };
        if let (Some(bit), Some(byte)) = (bit, bytes.get_mut(t.byte)) {
            *byte |= 1 << bit;
        }
    }
    value == 0
}

fn decode_axis(bytes: [u8; 3], trits: &[Trit]) -> i32 {
    trits
        .iter()
        .map(|t| {
            let byte = bytes.get(t.byte).copied().unwrap_or(0);
            let bit = |n: u8| i32::from((byte >> n) & 1);
            t.weight * (bit(t.plus) - bit(t.minus))
        })
        .sum()
}

/// Encodes a displacement with the given control flags (in byte 2).
fn encode_record(dx: i64, dy: i64, flags: u8) -> Result<[u8; 3], FormatError> {
    let too_long = || FormatError::DisplacementTooLong {
        format: FORMAT,
        dx,
        dy,
    };
    let x = i32::try_from(dx).map_err(|_| too_long())?;
    let y = i32::try_from(dy).map_err(|_| too_long())?;
    let mut bytes = [0, 0, flags];
    if encode_axis(x, &X_TRITS, &mut bytes) && encode_axis(y, &Y_TRITS, &mut bytes) {
        Ok(bytes)
    } else {
        Err(too_long())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Record {
    Stitch(i32, i32),
    Jump(i32, i32),
    ColorChange(i32, i32),
    SequinMode(i32, i32),
    End,
}

fn decode_record(bytes: [u8; 3]) -> Record {
    let [_, _, control] = bytes;
    let dx = decode_axis(bytes, &X_TRITS);
    let dy = decode_axis(bytes, &Y_TRITS);
    if control & 0xF3 == 0xF3 {
        Record::End
    } else if control & COLOR_CHANGE_FLAGS == COLOR_CHANGE_FLAGS {
        Record::ColorChange(dx, dy)
    } else if control & 0x43 == 0x43 {
        Record::SequinMode(dx, dy)
    } else if control & JUMP_FLAGS == JUMP_FLAGS {
        Record::Jump(dx, dy)
    } else {
        Record::Stitch(dx, dy)
    }
}

impl FormatWriter for DstWriter {
    fn capabilities(&self) -> &FormatCapabilities {
        &DST_CAPABILITIES
    }

    fn write(
        &self,
        pattern: &emb_core::StitchPattern,
        out: &mut dyn Write,
    ) -> Result<WriteOutput, FormatError> {
        let machine = MachinePattern::from_pattern(pattern, UNITS_PER_MM)?;
        let Normalized {
            pattern: machine,
            mut warnings,
        } = normalize(machine, &DST_CAPABILITIES)?;
        if !machine.threads().is_empty() {
            warnings.push(Warning::ColorsNotStored {
                blocks: machine.threads().len(),
            });
        }
        let body = encode_stitches(&machine)?;
        let name = header_name(
            pattern.metadata.name.as_deref().unwrap_or(""),
            &mut warnings,
        );
        let header = encode_header(&name, &machine, body.len() / 3)?;
        out.write_all(&header)?;
        out.write_all(&body)?;
        Ok(WriteOutput { warnings })
    }
}

fn encode_stitches(machine: &MachinePattern) -> Result<Vec<u8>, FormatError> {
    let mut data = Vec::with_capacity(3 * (machine.stitches().len() + 1));
    let (mut x, mut y) = (0_i32, 0_i32);
    for stitch in machine.stitches() {
        let dx = i64::from(stitch.x) - i64::from(x);
        let dy = i64::from(stitch.y) - i64::from(y);
        match stitch.kind {
            StitchKind::Normal => data.extend(encode_record(dx, dy, STITCH_FLAGS)?),
            StitchKind::Jump | StitchKind::SequinEject => {
                data.extend(encode_record(dx, dy, JUMP_FLAGS)?);
            }
            StitchKind::ColorChange | StitchKind::Stop => {
                data.extend(encode_record(dx, dy, COLOR_CHANGE_FLAGS)?);
            }
            StitchKind::Trim => {
                for (tx, ty) in TRIM_SIGNATURE {
                    data.extend(encode_record(tx.into(), ty.into(), JUMP_FLAGS)?);
                }
            }
            StitchKind::End => {}
        }
        (x, y) = (stitch.x, stitch.y);
    }
    data.extend(END_RECORD);
    Ok(data)
}

/// Restricts the name to printable ASCII and to the 16 characters of the `LA` field.
fn header_name(name: &str, warnings: &mut Vec<Warning>) -> String {
    let written: String = name
        .chars()
        .map(|c| {
            if c == ' ' || c.is_ascii_graphic() {
                c
            } else {
                '_'
            }
        })
        .take(NAME_LENGTH)
        .collect();
    if written != name {
        warnings.push(Warning::NameAltered {
            original: name.to_owned(),
            written: written.clone(),
        });
    }
    written
}

fn encode_header(
    name: &str,
    machine: &MachinePattern,
    records: usize,
) -> Result<Vec<u8>, FormatError> {
    let (min_x, min_y, max_x, max_y) = machine.bounds().map_or((0, 0, 0, 0), |b| {
        (
            b.min_x.min(0),
            b.min_y.min(0),
            b.max_x.max(0),
            b.max_y.max(0),
        )
    });
    let (end_x, end_y) = machine.stitches().last().map_or((0, 0), |s| (s.x, s.y));
    let color_changes = machine
        .stitches()
        .iter()
        .filter(|s| s.kind == StitchKind::ColorChange)
        .count();
    let mut text = format!("LA:{name:<NAME_LENGTH$}\r");
    text.push_str(&unsigned_field("ST", count_to_i64(records), 7)?);
    text.push_str(&unsigned_field("CO", count_to_i64(color_changes), 3)?);
    text.push_str(&unsigned_field("+X", max_x.into(), 5)?);
    text.push_str(&unsigned_field("-X", -i64::from(min_x), 5)?);
    text.push_str(&unsigned_field("+Y", max_y.into(), 5)?);
    text.push_str(&unsigned_field("-Y", -i64::from(min_y), 5)?);
    text.push_str(&signed_field("AX", end_x.into())?);
    text.push_str(&signed_field("AY", end_y.into())?);
    text.push_str(&signed_field("MX", 0)?);
    text.push_str(&signed_field("MY", 0)?);
    text.push_str("PD:******\r");
    let mut header = text.into_bytes();
    header.push(0x1A);
    header.resize(HEADER_SIZE, b' ');
    Ok(header)
}

fn count_to_i64(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

fn unsigned_field(field: &'static str, value: i64, width: u32) -> Result<String, FormatError> {
    if !(0..10_i64.pow(width)).contains(&value) {
        return Err(FormatError::HeaderFieldOverflow {
            format: FORMAT,
            field,
            value,
        });
    }
    let width = width as usize;
    Ok(format!("{field}:{value:>width$}\r"))
}

fn signed_field(field: &'static str, value: i64) -> Result<String, FormatError> {
    let sign = if value < 0 { '-' } else { '+' };
    let digits = unsigned_field(field, value.abs(), 5)?;
    Ok(digits.replacen(':', &format!(":{sign}"), 1))
}

impl FormatReader for DstReader {
    fn read(&self, input: &mut dyn Read) -> Result<ReadOutput, FormatError> {
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;
        let Some((header, body)) = data.split_at_checked(HEADER_SIZE) else {
            return Err(FormatError::InvalidHeader {
                format: FORMAT,
                reason: format!("{} bytes, shorter than the 512-byte header", data.len()),
            });
        };
        let mut warnings = Vec::new();
        let (metadata, declared_threads) = parse_header(header);
        let records = decode_records(body, &mut warnings);
        let stitches = build_stitches(&records);
        let blocks = machine_block_count(&stitches);
        let threads = reconcile_threads(declared_threads, blocks, &mut warnings);
        let mut pattern = MachinePattern::new(stitches, threads)?.to_pattern(UNITS_PER_MM)?;
        pattern.metadata = metadata;
        Ok(ReadOutput { pattern, warnings })
    }
}

fn parse_header(header: &[u8]) -> (Metadata, Vec<Thread>) {
    let text_end = header
        .iter()
        .position(|&b| b == 0x1A)
        .unwrap_or(header.len());
    let text = header.get(..text_end).unwrap_or(header);
    let mut metadata = Metadata::default();
    let mut threads = Vec::new();
    for line in text.split(|&b| b == b'\r' || b == b'\n') {
        let line = String::from_utf8_lossy(line);
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let text = (!value.is_empty()).then(|| value.to_owned());
        match key.trim() {
            "LA" => metadata.name = text,
            "AU" => metadata.author = text,
            "CP" => metadata.copyright = text,
            "TC" => threads.extend(parse_thread(value)),
            _ => {}
        }
    }
    (metadata, threads)
}

/// Parses an extended-header thread: `#RRGGBB,description,catalog`.
fn parse_thread(value: &str) -> Option<Thread> {
    let mut fields = value.split(',').map(str::trim);
    let mut thread = Thread::from_color(Rgb::parse_hex(fields.next()?)?);
    let non_empty = |s: Option<&str>| s.filter(|s| !s.is_empty()).map(str::to_owned);
    thread.name = non_empty(fields.next());
    thread.catalog_number = non_empty(fields.next());
    Some(thread)
}

/// Decodes records up to the end marker, reporting truncation.
fn decode_records(body: &[u8], warnings: &mut Vec<Warning>) -> Vec<Record> {
    let mut chunks = body.chunks_exact(3);
    let mut records = Vec::with_capacity(body.len() / 3);
    for chunk in chunks.by_ref() {
        let Ok(bytes) = <[u8; 3]>::try_from(chunk) else {
            continue;
        };
        match decode_record(bytes) {
            Record::End => return records,
            record => records.push(record),
        }
    }
    let remainder = chunks.remainder().len();
    if remainder == 0 {
        warnings.push(Warning::MissingEndMarker);
    } else {
        warnings.push(Warning::TruncatedData {
            offset: HEADER_SIZE + body.len() - remainder,
        });
    }
    records
}

fn is_jump(record: &Record) -> bool {
    matches!(record, Record::Jump(..))
}

fn build_stitches(records: &[Record]) -> Vec<MachineStitch> {
    let mut out = Vec::with_capacity(records.len());
    let mut position = (0_i32, 0_i32);
    let mut sequin_mode = false;
    for group in records.chunk_by(|a, b| is_jump(a) && is_jump(b)) {
        if !sequin_mode && group.first().is_some_and(is_jump) {
            push_jump_run(group, &mut position, &mut out);
            continue;
        }
        for record in group {
            let (kind, dx, dy) = match *record {
                Record::Stitch(dx, dy) => (StitchKind::Normal, dx, dy),
                Record::Jump(dx, dy) => (StitchKind::SequinEject, dx, dy),
                Record::ColorChange(dx, dy) => (StitchKind::ColorChange, dx, dy),
                Record::SequinMode(dx, dy) => {
                    sequin_mode = !sequin_mode;
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    (StitchKind::Jump, dx, dy)
                }
                Record::End => continue,
            };
            out.push(advance(&mut position, kind, dx, dy));
        }
    }
    out
}

fn advance(position: &mut (i32, i32), kind: StitchKind, dx: i32, dy: i32) -> MachineStitch {
    *position = (position.0.saturating_add(dx), position.1.saturating_add(dy));
    MachineStitch::new(kind, position.0, position.1)
}

/// Decodes consecutive jump records: trim signatures become trims, and a run
/// of jumps long enough to make the machine trim gets an explicit trim.
fn push_jump_run(run: &[Record], position: &mut (i32, i32), out: &mut Vec<MachineStitch>) {
    let deltas: Vec<(i32, i32)> = run
        .iter()
        .filter_map(|r| match *r {
            Record::Jump(dx, dy) => Some((dx, dy)),
            _ => None,
        })
        .collect();
    let (start, start_position) = (out.len(), *position);
    let mut found_trim = false;
    let mut rest = deltas.as_slice();
    while let Some((&(dx, dy), tail)) = rest.split_first() {
        if rest.starts_with(&TRIM_SIGNATURE) {
            out.push(MachineStitch::new(StitchKind::Trim, position.0, position.1));
            rest = rest.get(TRIM_SIGNATURE.len()..).unwrap_or_default();
            found_trim = true;
        } else {
            out.push(advance(position, StitchKind::Jump, dx, dy));
            rest = tail;
        }
    }
    if deltas.len() >= MACHINE_TRIM_JUMPS && !found_trim {
        let trim = MachineStitch::new(StitchKind::Trim, start_position.0, start_position.1);
        out.insert(start, trim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_core::{Mm, Point, Stitch, StitchPattern, Thread};

    #[test]
    fn every_encodable_displacement_round_trips() {
        for dx in -121..=121 {
            for dy in -121..=121 {
                let bytes = encode_record(dx, dy, STITCH_FLAGS).unwrap();
                assert_eq!(
                    decode_record(bytes),
                    Record::Stitch(i32::try_from(dx).unwrap(), i32::try_from(dy).unwrap())
                );
                let bytes = encode_record(dx, dy, JUMP_FLAGS).unwrap();
                assert!(matches!(decode_record(bytes), Record::Jump(..)));
            }
        }
    }

    #[test]
    fn displacements_beyond_121_are_rejected() {
        assert!(encode_record(122, 0, STITCH_FLAGS).is_err());
        assert!(encode_record(0, -122, STITCH_FLAGS).is_err());
    }

    /// Tajima layout: byte 0 bit 7 is y+1, bit 0 is x+1 (Y up, X right).
    #[test]
    fn axes_follow_the_tajima_bit_layout() {
        assert_eq!(
            encode_record(0, 1, STITCH_FLAGS).unwrap(),
            [0x80, 0x00, 0x03]
        );
        assert_eq!(
            encode_record(1, 0, STITCH_FLAGS).unwrap(),
            [0x01, 0x00, 0x03]
        );
        assert_eq!(
            encode_record(0, 81, STITCH_FLAGS).unwrap(),
            [0x00, 0x00, 0x23]
        );
        assert_eq!(
            encode_record(-81, 0, STITCH_FLAGS).unwrap(),
            [0x00, 0x00, 0x0B]
        );
    }

    #[test]
    fn control_records_decode_to_their_kind() {
        assert_eq!(decode_record(END_RECORD), Record::End);
        assert_eq!(decode_record([0, 0, 0xC3]), Record::ColorChange(0, 0));
        assert_eq!(decode_record([0, 0, 0x43]), Record::SequinMode(0, 0));
    }

    fn single_block(stitches: Vec<Stitch>) -> StitchPattern {
        StitchPattern::new(stitches, vec![Thread::from_color(Rgb::new(0, 0, 0))]).unwrap()
    }

    fn at(kind: StitchKind, x: f64, y: f64) -> Stitch {
        Stitch::new(kind, Point::new(Mm(x), Mm(y)))
    }

    fn write(pattern: &StitchPattern) -> (Vec<u8>, WriteOutput) {
        let mut bytes = Vec::new();
        let output = DstWriter.write(pattern, &mut bytes).unwrap();
        (bytes, output)
    }

    #[test]
    fn header_is_512_bytes_with_expected_fields() {
        let mut pattern = single_block(vec![
            at(StitchKind::Normal, 1.0, 2.0),
            at(StitchKind::Normal, -3.0, -4.5),
        ]);
        pattern.metadata.name = Some("Test".into());
        let (bytes, _) = write(&pattern);
        let header = String::from_utf8_lossy(&bytes[..HEADER_SIZE]);
        assert!(header.starts_with(
            "LA:Test            \rST:      3\rCO:  0\r+X:   10\r-X:   30\r+Y:   20\r-Y:   45\r\
             AX:-   30\rAY:-   45\rMX:+    0\rMY:+    0\rPD:******\r\u{1a}"
        ));
        assert_eq!(bytes.len(), HEADER_SIZE + 3 * 3);
        assert_eq!(bytes[bytes.len() - 3..], END_RECORD);
    }

    #[test]
    fn long_names_are_truncated_with_a_warning() {
        let mut pattern = single_block(vec![at(StitchKind::Normal, 0.0, 0.0)]);
        pattern.metadata.name = Some("A very long design name".into());
        let (bytes, output) = write(&pattern);
        assert!(bytes.starts_with(b"LA:A very long desi\r"));
        assert!(
            output
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::NameAltered { .. }))
        );
    }

    #[test]
    fn trims_use_the_jump_signature_and_read_back() {
        let pattern = single_block(vec![
            at(StitchKind::Normal, 1.0, 0.0),
            at(StitchKind::Trim, 1.0, 0.0),
            at(StitchKind::Jump, 2.0, 0.0),
            at(StitchKind::Normal, 2.0, 0.0),
        ]);
        let (bytes, _) = write(&pattern);
        let body = &bytes[HEADER_SIZE..];
        assert_eq!(&body[3..6], &encode_record(2, -2, JUMP_FLAGS).unwrap());
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        let kinds: Vec<_> = read.pattern.stitches().iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                StitchKind::Normal,
                StitchKind::Trim,
                StitchKind::Jump,
                StitchKind::Normal
            ]
        );
    }

    #[test]
    fn three_plain_jumps_read_as_an_automatic_trim() {
        let mut bytes = vec![b' '; HEADER_SIZE];
        for _ in 0..3 {
            bytes.extend(encode_record(10, 0, JUMP_FLAGS).unwrap());
        }
        bytes.extend(END_RECORD);
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        let kinds: Vec<_> = read.pattern.stitches().iter().map(|s| s.kind).collect();
        assert_eq!(kinds[0], StitchKind::Trim);
        assert_eq!(kinds.len(), 4);
    }

    #[test]
    fn colors_are_reported_as_lost_and_defaulted_on_read() {
        let pattern = single_block(vec![at(StitchKind::Normal, 0.0, 0.0)]);
        let (bytes, output) = write(&pattern);
        assert!(
            output
                .warnings
                .contains(&Warning::ColorsNotStored { blocks: 1 })
        );
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        assert!(
            read.warnings
                .contains(&Warning::DefaultColorsAssigned { blocks: 1 })
        );
    }

    #[test]
    fn extended_header_threads_are_read() {
        let mut header = b"LA:x\rTC:#FF0000,Red,1147\rTC:#0000FF,,\r\x1a".to_vec();
        header.resize(HEADER_SIZE, b' ');
        header.extend(encode_record(1, 1, STITCH_FLAGS).unwrap());
        header.extend([0, 0, COLOR_CHANGE_FLAGS]);
        header.extend(END_RECORD);
        let read = DstReader.read(&mut header.as_slice()).unwrap();
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        let threads = read.pattern.threads();
        assert_eq!(threads[0].color, Rgb::new(255, 0, 0));
        assert_eq!(threads[0].name.as_deref(), Some("Red"));
        assert_eq!(threads[0].catalog_number.as_deref(), Some("1147"));
        assert_eq!(threads[1].name, None);
    }

    #[test]
    fn truncated_data_keeps_valid_records_and_warns() {
        let mut bytes = vec![b' '; HEADER_SIZE];
        bytes.extend(encode_record(5, 5, STITCH_FLAGS).unwrap());
        bytes.extend([0x01, 0x02]);
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        assert_eq!(read.pattern.stitches().len(), 1);
        assert!(read.warnings.contains(&Warning::TruncatedData {
            offset: HEADER_SIZE + 3
        }));
    }

    #[test]
    fn missing_end_marker_is_reported() {
        let mut bytes = vec![b' '; HEADER_SIZE];
        bytes.extend(encode_record(5, 5, STITCH_FLAGS).unwrap());
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        assert!(read.warnings.contains(&Warning::MissingEndMarker));
    }

    #[test]
    fn short_file_is_an_error() {
        let bytes = b"LA:short".to_vec();
        assert!(matches!(
            DstReader.read(&mut bytes.as_slice()),
            Err(FormatError::InvalidHeader { .. })
        ));
    }

    #[test]
    fn sequin_mode_turns_jumps_into_sequins() {
        let mut bytes = vec![b' '; HEADER_SIZE];
        bytes.extend([0, 0, 0x43]);
        bytes.extend(encode_record(3, 0, JUMP_FLAGS).unwrap());
        bytes.extend([0, 0, 0x43]);
        bytes.extend(encode_record(3, 0, JUMP_FLAGS).unwrap());
        bytes.extend(END_RECORD);
        let read = DstReader.read(&mut bytes.as_slice()).unwrap();
        let kinds: Vec<_> = read.pattern.stitches().iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![StitchKind::SequinEject, StitchKind::Jump]);
    }
}
