//! Brother PES (reading). Structure, sources and open questions:
//! `docs/formats/pes.md`.
//!
//! Stitches come from the embedded PEC block; PES versions 5 and later add a
//! thread list (RGB, catalogue reference, name, brand) that takes precedence
//! over the PEC palette indices.

use std::collections::BTreeMap;
use std::io::Read;

use emb_core::{Metadata, Rgb, Thread};

use crate::codec::{FormatReader, ReadOutput};
use crate::error::FormatError;
use crate::machine::{MachinePattern, machine_block_count};
use crate::pec::decode_pec;
use crate::pec::palette::pec_palette_thread;
use crate::threads::{default_thread, reconcile_threads};
use crate::warning::Warning;

const FORMAT: &str = "PES";
const UNITS_PER_MM: u32 = 10;
const SIGNATURE: &[u8] = b"#PES";
const PEC_OFFSET_FIELD: usize = 8;
const METADATA_OFFSET: usize = 16;

/// Reads Brother PES files.
#[derive(Debug, Clone, Copy, Default)]
pub struct PesReader;

/// Whether `data` starts like a PES file: `#PES` followed by four digits and
/// a PEC offset inside the file.
#[must_use]
pub fn looks_like_pes(data: &[u8]) -> bool {
    let digits = data
        .get(4..8)
        .is_some_and(|d| d.iter().all(u8::is_ascii_digit));
    data.starts_with(SIGNATURE)
        && digits
        && pec_offset(data).is_some_and(|offset| offset < data.len())
}

fn pec_offset(data: &[u8]) -> Option<usize> {
    let bytes = data.get(PEC_OFFSET_FIELD..PEC_OFFSET_FIELD + 4)?;
    usize::try_from(u32::from_le_bytes(bytes.try_into().ok()?)).ok()
}

/// Header layout of the versions that store metadata and threads: bytes to
/// skip before the hoop name (versions 9 and 10 only), before the image file
/// name and after it. Values from pyembroidery and libembroidery.
struct Layout {
    before_hoop_name: Option<usize>,
    before_image: usize,
    after_image: usize,
}

fn layout(version: &str) -> Option<Layout> {
    let (before_hoop_name, before_image, after_image) = match version {
        "0050" | "0055" | "0056" => (None, 24, 24),
        "0060" | "0070" => (None, 36, 24),
        "0080" => (None, 38, 26),
        "0090" => (Some(14), 30, 34),
        "0100" => (Some(14), 38, 34),
        _ => return None,
    };
    Some(Layout {
        before_hoop_name,
        before_image,
        after_image,
    })
}

/// Versions known to carry no metadata (or only metadata, for version 4).
const KNOWN_VERSIONS: [&str; 5] = ["0001", "0020", "0022", "0030", "0040"];

/// Bounds-checked little-endian reader over the PES header.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn bytes(&mut self, len: usize) -> Option<&[u8]> {
        let bytes = self.data.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        self.bytes(1)?.first().copied()
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.bytes(2)?.try_into().ok()?))
    }

    /// A string prefixed by its length on one byte, trimmed (possibly empty).
    fn string(&mut self) -> Option<String> {
        let len = usize::from(self.u8()?);
        Some(String::from_utf8_lossy(self.bytes(len)?).trim().to_owned())
    }
}

/// `None` for an empty string.
fn non_empty(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

/// Reads name, category, author, keywords and comments (version 4 and later).
fn read_metadata(cursor: &mut Cursor<'_>) -> Option<Metadata> {
    let name = non_empty(cursor.string()?);
    let _category = cursor.string()?;
    let author = non_empty(cursor.string()?);
    let _keywords = cursor.string()?;
    let comments = non_empty(cursor.string()?);
    Some(Metadata {
        name,
        author,
        comments,
        ..Metadata::default()
    })
}

fn read_thread(cursor: &mut Cursor<'_>) -> Option<Thread> {
    let catalog_number = non_empty(cursor.string()?);
    let rgb = cursor.bytes(3)?;
    let color = Rgb::new(*rgb.first()?, *rgb.get(1)?, *rgb.get(2)?);
    cursor.bytes(5)?;
    let name = non_empty(cursor.string()?);
    let brand = non_empty(cursor.string()?);
    let _chart = cursor.string()?;
    Some(Thread {
        name,
        brand,
        catalog_number,
        ..Thread::from_color(color)
    })
}

/// Reads the thread list after the metadata, if the header holds no
/// programmable fill, motif or feather pattern (whose layouts are unknown).
fn read_threads(cursor: &mut Cursor<'_>, layout: &Layout) -> Option<Vec<Thread>> {
    if let Some(skip) = layout.before_hoop_name {
        cursor.bytes(skip)?;
        cursor.string()?;
    }
    cursor.bytes(layout.before_image)?;
    cursor.string()?;
    cursor.bytes(layout.after_image)?;
    for _ in 0..3 {
        if cursor.u16()? != 0 {
            return None;
        }
    }
    let count = cursor.u16()?;
    (0..count).map(|_| read_thread(cursor)).collect()
}

/// What the PES part of the header provides.
#[derive(Default)]
struct PesHeader {
    metadata: Option<Metadata>,
    threads: Vec<Thread>,
}

fn read_header(data: &[u8], version: &str, warnings: &mut Vec<Warning>) -> PesHeader {
    let mut cursor = Cursor {
        data,
        pos: METADATA_OFFSET,
    };
    if let Some(layout) = layout(version) {
        let metadata = read_metadata(&mut cursor);
        let threads = metadata
            .as_ref()
            .and_then(|_| read_threads(&mut cursor, &layout));
        if threads.is_none() {
            warnings.push(Warning::ThreadListUnreadable);
        }
        return PesHeader {
            metadata,
            threads: threads.unwrap_or_default(),
        };
    }
    if version == "0040" {
        return PesHeader {
            metadata: read_metadata(&mut cursor),
            threads: Vec::new(),
        };
    }
    if !KNOWN_VERSIONS.contains(&version) {
        warnings.push(Warning::UnknownVersion {
            version: version.to_owned(),
        });
    }
    PesHeader::default()
}

/// Assigns one thread per PEC colour block. A PES thread list either holds
/// one thread per block, or one thread per distinct PEC index, in order of
/// first use (pyembroidery's "tabled" mode — to be confirmed on real files).
fn assign_threads(
    pes_threads: Vec<Thread>,
    color_indices: &[u8],
    warnings: &mut Vec<Warning>,
) -> Vec<Thread> {
    if pes_threads.len() == color_indices.len() {
        return pes_threads;
    }
    let mut first_use: BTreeMap<u8, usize> = BTreeMap::new();
    for &index in color_indices {
        let next = first_use.len();
        first_use.entry(index).or_insert(next);
    }
    if !pes_threads.is_empty() && pes_threads.len() == first_use.len() {
        return color_indices
            .iter()
            .filter_map(|index| first_use.get(index).and_then(|&i| pes_threads.get(i)))
            .cloned()
            .collect();
    }
    if !pes_threads.is_empty() {
        warnings.push(Warning::ThreadCountMismatch {
            declared: pes_threads.len(),
            blocks: color_indices.len(),
        });
    }
    color_indices
        .iter()
        .enumerate()
        .map(|(block, &index)| {
            pec_palette_thread(index).unwrap_or_else(|| {
                warnings.push(Warning::UnknownPaletteIndex {
                    block,
                    index: index.into(),
                });
                default_thread(block)
            })
        })
        .collect()
}

impl FormatReader for PesReader {
    fn read(&self, input: &mut dyn Read) -> Result<ReadOutput, FormatError> {
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;
        if !looks_like_pes(&data) {
            return Err(FormatError::InvalidHeader {
                format: FORMAT,
                reason: "missing #PES signature or PEC offset beyond the end of the file".into(),
            });
        }
        let version = String::from_utf8_lossy(data.get(4..8).unwrap_or_default()).into_owned();
        let mut warnings = Vec::new();
        let header = read_header(&data, &version, &mut warnings);
        let start = pec_offset(&data).unwrap_or_default();
        let pec = decode_pec(&data, start, FORMAT, &mut warnings)?;
        let threads = assign_threads(header.threads, &pec.color_indices, &mut warnings);
        let blocks = machine_block_count(&pec.stitches);
        let threads = reconcile_threads(threads, blocks, &mut warnings);
        // PEC coordinates start at the design's top-left corner: centre the
        // design on the origin, as PES places it in the hoop.
        let machine = MachinePattern::new(pec.stitches, threads)?.centered(UNITS_PER_MM)?;
        let mut pattern = machine.to_pattern(UNITS_PER_MM)?;
        pattern.metadata = header.metadata.unwrap_or_default();
        if pattern.metadata.name.is_none() {
            pattern.metadata.name = pec.label;
        }
        Ok(ReadOutput { pattern, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_core::StitchKind;

    fn pes_v1(stitch_data: &[u8], colors: &[u8]) -> Vec<u8> {
        let pec_start = 0x30;
        let mut data = b"#PES0001".to_vec();
        data.extend(u32::try_from(pec_start).unwrap().to_le_bytes());
        data.resize(pec_start, 0);
        let mut pec = vec![0x20_u8; 528];
        pec[..3].copy_from_slice(b"LA:");
        pec[3..8].copy_from_slice(b"Test ");
        pec[48] = u8::try_from(colors.len()).unwrap() - 1;
        pec[49..49 + colors.len()].copy_from_slice(colors);
        data.extend(pec);
        data.extend(stitch_data);
        data
    }

    #[test]
    fn version_one_uses_the_pec_palette_and_label() {
        let data = pes_v1(&[0x0A, 0x00, 0xFE, 0xB0, 0x02, 0x00, 0x0A, 0xFF], &[20, 5]);
        assert!(looks_like_pes(&data));
        let read = PesReader.read(&mut data.as_slice()).unwrap();
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        let names: Vec<_> = read
            .pattern
            .threads()
            .iter()
            .map(|t| t.name.as_deref())
            .collect();
        assert_eq!(names, vec![Some("Black"), Some("Red")]);
        assert_eq!(read.pattern.metadata.name.as_deref(), Some("Test"));
    }

    #[test]
    fn design_is_centred_and_y_flipped() {
        // Right 10 then down 10 (PEC axes).
        let data = pes_v1(&[0x0A, 0x00, 0x00, 0x0A, 0xFF], &[20]);
        let read = PesReader.read(&mut data.as_slice()).unwrap();
        let machine = MachinePattern::from_pattern(&read.pattern, UNITS_PER_MM).unwrap();
        let positions: Vec<_> = machine.stitches().iter().map(|s| (s.x, s.y)).collect();
        assert_eq!(positions, vec![(0, 5), (0, -5)]);
        assert_eq!(read.pattern.stitches()[0].kind, StitchKind::Normal);
    }

    fn thread(r: u8) -> Thread {
        Thread::from_color(Rgb::new(r, 0, 0))
    }

    #[test]
    fn thread_list_maps_one_to_one_or_by_first_use() {
        let mut warnings = Vec::new();
        let one_to_one = assign_threads(vec![thread(1), thread(2)], &[5, 5], &mut warnings);
        assert_eq!(one_to_one, vec![thread(1), thread(2)]);
        let tabled = assign_threads(vec![thread(1), thread(2)], &[7, 3, 7], &mut warnings);
        assert_eq!(tabled, vec![thread(1), thread(2), thread(1)]);
        assert_eq!(warnings, []);
    }

    #[test]
    fn inconsistent_thread_list_falls_back_to_the_palette() {
        let mut warnings = Vec::new();
        let threads = assign_threads(vec![thread(1)], &[20, 5, 7], &mut warnings);
        assert_eq!(threads[0].name.as_deref(), Some("Black"));
        assert_eq!(
            warnings,
            vec![Warning::ThreadCountMismatch {
                declared: 1,
                blocks: 3
            }]
        );
    }

    #[test]
    fn unknown_palette_index_gets_a_default_colour() {
        let mut warnings = Vec::new();
        assign_threads(vec![], &[0], &mut warnings);
        assert_eq!(
            warnings,
            vec![Warning::UnknownPaletteIndex { block: 0, index: 0 }]
        );
    }

    #[test]
    fn unknown_versions_are_read_with_a_warning() {
        let mut data = pes_v1(&[0x01, 0x01, 0xFF], &[20]);
        data[4..8].copy_from_slice(b"0999");
        let read = PesReader.read(&mut data.as_slice()).unwrap();
        assert_eq!(
            read.warnings,
            vec![Warning::UnknownVersion {
                version: "0999".into()
            }]
        );
    }

    #[test]
    fn missing_signature_is_an_error() {
        assert!(PesReader.read(&mut &b"#PEC0001"[..]).is_err());
    }
}
