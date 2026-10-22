//! Brother PEC block: the machine part of PES files (and the content of
//! `.pec` files). Structure, sources and open questions: `docs/formats/pes.md`.

pub mod palette;

use emb_core::StitchKind;

use crate::error::FormatError;
use crate::machine::MachineStitch;
use crate::warning::Warning;

/// Offset of the colour-change count from the start of the PEC block.
const COLOR_COUNT_OFFSET: usize = 48;
/// Offset of the stitch data from the start of the PEC block.
const STITCH_DATA_OFFSET: usize = 528;
const LABEL_RANGE: std::ops::Range<usize> = 3..19;
const END: u8 = 0xFF;
const COLOR_CHANGE: [u8; 2] = [0xFE, 0xB0];
const LONG_FORM: u8 = 0x80;
const TRIM_FLAG: u8 = 0x20;
const JUMP_FLAG: u8 = 0x10;

/// The decoded content of a PEC block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PecBlock {
    /// Design label (`LA:` field), trimmed.
    pub label: Option<String>,
    /// One PEC palette index per colour block.
    pub color_indices: Vec<u8>,
    /// Stitches in pivot axes (Y up), relative to the PEC origin.
    pub stitches: Vec<MachineStitch>,
}

/// Decodes the PEC block starting at `start` in `data`.
pub(crate) fn decode_pec(
    data: &[u8],
    start: usize,
    format: &'static str,
    warnings: &mut Vec<Warning>,
) -> Result<PecBlock, FormatError> {
    let block = data
        .get(start..)
        .filter(|b| b.len() >= STITCH_DATA_OFFSET)
        .ok_or_else(|| FormatError::InvalidHeader {
            format,
            reason: format!(
                "PEC block at {start} is shorter than its {STITCH_DATA_OFFSET}-byte header"
            ),
        })?;
    let label = block
        .get(LABEL_RANGE)
        .map(|bytes| String::from_utf8_lossy(bytes).trim().to_owned())
        .filter(|s| !s.is_empty());
    let color_count = match block.get(COLOR_COUNT_OFFSET).copied() {
        Some(0xFF) | None => 0,
        Some(changes) => usize::from(changes) + 1,
    };
    let color_indices = block
        .get(COLOR_COUNT_OFFSET + 1..COLOR_COUNT_OFFSET + 1 + color_count)
        .unwrap_or_default()
        .to_vec();
    let stitch_data = block.get(STITCH_DATA_OFFSET..).unwrap_or_default();
    let stitches = decode_stitches(stitch_data, start + STITCH_DATA_OFFSET, warnings);
    Ok(PecBlock {
        label,
        color_indices,
        stitches,
    })
}

/// One coordinate: short form (7-bit signed, one byte) or long form
/// (12-bit signed, two bytes, with trim/jump flags in the first byte).
struct Coordinate {
    value: i32,
    flags: u8,
    len: usize,
}

fn decode_coordinate(data: &[u8]) -> Option<Coordinate> {
    let first = *data.first()?;
    if first & LONG_FORM == 0 {
        let value = i32::from(first) - if first >= 0x40 { 0x80 } else { 0 };
        return Some(Coordinate {
            value,
            flags: 0,
            len: 1,
        });
    }
    let raw = (i32::from(first & 0x0F) << 8) | i32::from(*data.get(1)?);
    let value = if raw & 0x800 == 0 { raw } else { raw - 0x1000 };
    Some(Coordinate {
        value,
        flags: first & (TRIM_FLAG | JUMP_FLAG),
        len: 2,
    })
}

/// Decodes stitch records up to the end marker; `base` is the file offset of
/// `data`. PEC's Y axis points down: it is flipped here.
fn decode_stitches(data: &[u8], base: usize, warnings: &mut Vec<Warning>) -> Vec<MachineStitch> {
    let mut out = Vec::with_capacity(data.len() / 2);
    let (mut x, mut y) = (0_i32, 0_i32);
    let mut pos = 0;
    loop {
        let rest = data.get(pos..).unwrap_or_default();
        match rest {
            [] => {
                warnings.push(Warning::MissingEndMarker);
                return out;
            }
            [END, ..] => return out,
            [a, b, ..] if [*a, *b] == COLOR_CHANGE => {
                out.push(MachineStitch::new(StitchKind::ColorChange, x, y));
                pos += 3;
                continue;
            }
            _ => {}
        }
        let decoded = decode_coordinate(rest).and_then(|dx| {
            let dy = decode_coordinate(rest.get(dx.len..)?)?;
            Some((dx, dy))
        });
        let Some((dx, dy)) = decoded else {
            warnings.push(Warning::TruncatedData { offset: base + pos });
            return out;
        };
        let flags = dx.flags | dy.flags;
        if flags & TRIM_FLAG != 0 && flags & JUMP_FLAG == 0 {
            out.push(MachineStitch::new(StitchKind::Trim, x, y));
        }
        let kind = if flags == 0 {
            StitchKind::Normal
        } else {
            StitchKind::Jump
        };
        (x, y) = (x.saturating_add(dx.value), y.saturating_sub(dy.value));
        out.push(MachineStitch::new(kind, x, y));
        pos += dx.len + dy.len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(data: &[u8]) -> (Vec<MachineStitch>, Vec<Warning>) {
        let mut warnings = Vec::new();
        let stitches = decode_stitches(data, 0, &mut warnings);
        (stitches, warnings)
    }

    fn s(kind: StitchKind, x: i32, y: i32) -> MachineStitch {
        MachineStitch::new(kind, x, y)
    }

    #[test]
    fn short_form_is_seven_bit_signed_and_y_points_down() {
        let (stitches, warnings) = decode(&[0x05, 0x7F, 0x40, 0x3F, 0xFF, 0x00]);
        assert_eq!(
            stitches,
            vec![s(StitchKind::Normal, 5, 1), s(StitchKind::Normal, -59, -62)]
        );
        assert_eq!(warnings, []);
    }

    #[test]
    fn long_form_is_twelve_bit_signed_with_flags() {
        // x = +303 jump, y = +753 (down) jump.
        let jump = [0x91, 0x2F, 0x92, 0xF1];
        // x = -1 trim, y = -2048 trim.
        let trim = [0xAF, 0xFF, 0xA8, 0x00];
        // x = 100 long stitch (no flag), y short 0.
        let long_stitch = [0x80, 0x64, 0x00];
        let data = [&jump[..], &trim, &long_stitch, &[0xFF]].concat();
        let (stitches, _) = decode(&data);
        assert_eq!(
            stitches,
            vec![
                s(StitchKind::Jump, 303, -753),
                s(StitchKind::Trim, 303, -753),
                s(StitchKind::Jump, 302, 1295),
                s(StitchKind::Normal, 402, 1295),
            ]
        );
    }

    #[test]
    fn color_change_takes_three_bytes() {
        let (stitches, _) = decode(&[0x01, 0x01, 0xFE, 0xB0, 0x02, 0x01, 0x00, 0xFF]);
        assert_eq!(
            stitches,
            vec![
                s(StitchKind::Normal, 1, -1),
                s(StitchKind::ColorChange, 1, -1),
                s(StitchKind::Normal, 2, -1),
            ]
        );
    }

    #[test]
    fn truncated_and_unterminated_data_are_reported() {
        let (stitches, warnings) = decode(&[0x01, 0x01, 0x91]);
        assert_eq!(stitches.len(), 1);
        assert_eq!(warnings, vec![Warning::TruncatedData { offset: 2 }]);
        let (_, warnings) = decode(&[0x01, 0x01]);
        assert_eq!(warnings, vec![Warning::MissingEndMarker]);
    }

    #[test]
    fn short_block_is_an_error() {
        let mut warnings = Vec::new();
        assert!(decode_pec(&[0; 100], 0, "PES", &mut warnings).is_err());
    }

    #[test]
    fn header_fields_are_read() {
        let mut block = vec![0_u8; STITCH_DATA_OFFSET];
        block[..19].copy_from_slice(b"LA:Flower          ");
        block[COLOR_COUNT_OFFSET] = 1;
        block[COLOR_COUNT_OFFSET + 1] = 20;
        block[COLOR_COUNT_OFFSET + 2] = 5;
        block.extend([0x01, 0x01, 0xFF]);
        let mut warnings = Vec::new();
        let pec = decode_pec(&block, 0, "PES", &mut warnings).unwrap();
        assert_eq!(pec.label.as_deref(), Some("Flower"));
        assert_eq!(pec.color_indices, vec![20, 5]);
        assert_eq!(pec.stitches.len(), 1);
    }
}
