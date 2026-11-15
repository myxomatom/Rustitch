//! The `ENHANCE` block that some JEF files append after the end marker: the
//! real threads (RGB, catalogue number, name) behind the Janome palette
//! indices of the header. Layout observed on real files, see
//! `docs/formats/jef.md`.

use emb_core::{Rgb, Thread};

use crate::warning::Warning;

const PREFIX: [u8; 2] = [0, 0];
const MAGIC: &[u8] = b"ENHANCE";
const THREADS_SECTION: u32 = 0;
/// Section of one `0xFFFFFFFF` per block; its meaning is unknown.
const UNKNOWN_SECTION: u32 = 3;
const NAME_LENGTH: usize = 32;
const RECORD_LENGTH: usize = 12 + NAME_LENGTH;

/// What follows the end marker of a JEF file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trailer {
    /// Nothing.
    Empty,
    /// An `ENHANCE` block with one thread per record, in block order.
    Threads(Vec<Thread>),
    /// An `ENHANCE` block whose thread section cannot be read.
    Unreadable,
    /// Bytes that are not an `ENHANCE` block.
    Unknown,
}

/// Bounds-checked little-endian reader.
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

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.bytes(4)?.try_into().ok()?))
    }
}

/// Decodes the bytes found after the end marker.
pub(crate) fn decode(trailer: &[u8]) -> Trailer {
    if trailer.is_empty() {
        return Trailer::Empty;
    }
    let body = trailer.strip_prefix(&PREFIX).unwrap_or(trailer);
    let Some(sections) = body.strip_prefix(MAGIC) else {
        return Trailer::Unknown;
    };
    decode_threads(sections).map_or(Trailer::Unreadable, Trailer::Threads)
}

fn decode_threads(data: &[u8]) -> Option<Vec<Thread>> {
    let mut cursor = Cursor { data, pos: 0 };
    let section_count = *cursor.bytes(1)?.first()?;
    let mut descriptors = Vec::new();
    for _ in 0..section_count {
        descriptors.push((cursor.u32()?, usize::try_from(cursor.u32()?).ok()?));
    }
    let mut threads = None;
    for (kind, size) in descriptors {
        let payload = cursor.bytes(size)?;
        if kind == THREADS_SECTION {
            threads = Some(decode_records(payload)?);
        }
    }
    threads
}

fn decode_records(payload: &[u8]) -> Option<Vec<Thread>> {
    let mut cursor = Cursor {
        data: payload,
        pos: 0,
    };
    let count = usize::try_from(cursor.u32()?).ok()?;
    (0..count)
        .map(|_| {
            let record = cursor.bytes(RECORD_LENGTH)?;
            let [r, g, b, _] = record.get(4..8)?.try_into().ok()?;
            let number = u32::from_le_bytes(record.get(8..12)?.try_into().ok()?);
            let name_bytes = record.get(12..)?;
            let name_end = name_bytes
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(name_bytes.len());
            let name = String::from_utf8_lossy(name_bytes.get(..name_end)?)
                .trim()
                .to_owned();
            Some(Thread {
                name: (!name.is_empty()).then_some(name),
                catalog_number: (number != 0).then(|| number.to_string()),
                ..Thread::from_color(Rgb::new(r, g, b))
            })
        })
        .collect()
}

/// Encodes one record per thread (one thread per colour block), reporting
/// what the block cannot hold: brands, non-numeric references and names that
/// are not printable ASCII or longer than 31 characters.
pub(crate) fn encode(threads: &[Thread], warnings: &mut Vec<Warning>) -> Vec<u8> {
    let count = u32::try_from(threads.len()).unwrap_or(u32::MAX);
    let mut records = count.to_le_bytes().to_vec();
    let mut details_lost = Vec::new();
    for (block, thread) in threads.iter().enumerate() {
        records.extend(u32::try_from(block).unwrap_or(u32::MAX).to_le_bytes());
        records.extend([thread.color.r, thread.color.g, thread.color.b, 0]);
        let (number, reference_lost) = catalog_number(thread);
        records.extend(number.to_le_bytes());
        records.extend(name_field(thread, block, warnings));
        if reference_lost || has_foreign_brand(thread) {
            details_lost.push(block);
        }
    }
    if !details_lost.is_empty() {
        warnings.push(Warning::ThreadDetailsNotStored {
            blocks: details_lost,
        });
    }
    let mut unknown = count.to_le_bytes().to_vec();
    unknown.extend(threads.iter().flat_map(|_| [0xFF; 4]));

    let mut block = PREFIX.to_vec();
    block.extend(MAGIC);
    block.push(2);
    for (kind, payload) in [(THREADS_SECTION, &records), (UNKNOWN_SECTION, &unknown)] {
        block.extend(kind.to_le_bytes());
        block.extend(
            u32::try_from(payload.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
    }
    block.extend(records);
    block.extend(unknown);
    block
}

/// The numeric catalogue reference, and whether a reference was lost.
fn catalog_number(thread: &Thread) -> (u32, bool) {
    match thread.catalog_number.as_deref() {
        None | Some("") => (0, false),
        Some(text) => text.trim().parse().map_or((0, true), |n| (n, false)),
    }
}

fn has_foreign_brand(thread: &Thread) -> bool {
    thread
        .brand
        .as_deref()
        .is_some_and(|b| !b.eq_ignore_ascii_case(super::palette::JANOME_BRAND))
}

/// The NUL-padded name field. Unnamed threads are named after their colour
/// (`R184 G63 B3`), as observed in real files.
fn name_field(thread: &Thread, block: usize, warnings: &mut Vec<Warning>) -> [u8; NAME_LENGTH] {
    let Rgb { r, g, b } = thread.color;
    let original = thread
        .name
        .clone()
        .unwrap_or_else(|| format!("R{r} G{g} B{b}"));
    let written: String = original
        .chars()
        .map(|c| {
            if c == ' ' || c.is_ascii_graphic() {
                c
            } else {
                '_'
            }
        })
        .take(NAME_LENGTH - 1)
        .collect();
    if written != original {
        warnings.push(Warning::ThreadNameAltered {
            block,
            original,
            written: written.clone(),
        });
    }
    let mut field = [0; NAME_LENGTH];
    for (slot, byte) in field.iter_mut().zip(written.bytes()) {
        *slot = byte;
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    fn madeira(rgb: u32, code: &str, name: &str) -> Thread {
        Thread {
            name: Some(name.into()),
            brand: Some("Madeira Classic 40".into()),
            catalog_number: Some(code.into()),
            ..Thread::from_color(Rgb::from_hex(rgb))
        }
    }

    /// Layout of `Love1.jef` (first record, sizes for one thread).
    #[test]
    fn layout_matches_real_files() {
        let mut warnings = Vec::new();
        let block = encode(
            &[madeira(0xFC_C68D, "1026", "Orange Sorbet")],
            &mut warnings,
        );
        let mut expected = vec![0, 0];
        expected.extend(b"ENHANCE");
        expected.push(2);
        expected.extend([0, 0, 0, 0, 48, 0, 0, 0, 3, 0, 0, 0, 8, 0, 0, 0]);
        expected.extend([
            1, 0, 0, 0, 0, 0, 0, 0, 0xFC, 0xC6, 0x8D, 0, 0x02, 0x04, 0, 0,
        ]);
        let mut name = b"Orange Sorbet".to_vec();
        name.resize(NAME_LENGTH, 0);
        expected.extend(name);
        expected.extend([1, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(block, expected);
        assert_eq!(
            warnings,
            vec![Warning::ThreadDetailsNotStored { blocks: vec![0] }]
        );
    }

    #[test]
    fn threads_round_trip_without_brand() {
        let threads = [
            madeira(0xFC_C68D, "1026", "Orange Sorbet"),
            Thread::from_color(Rgb::new(184, 63, 3)),
        ];
        let block = encode(&threads, &mut Vec::new());
        let Trailer::Threads(read) = decode(&block) else {
            panic!("not decoded");
        };
        assert_eq!(read[0].name.as_deref(), Some("Orange Sorbet"));
        assert_eq!(read[0].catalog_number.as_deref(), Some("1026"));
        assert_eq!(read[0].brand, None);
        assert_eq!(read[1].name.as_deref(), Some("R184 G63 B3"));
        assert_eq!(read[1].catalog_number, None);
        assert_eq!(read[1].color, Rgb::new(184, 63, 3));
    }

    #[test]
    fn long_or_non_ascii_names_are_reported() {
        let mut warnings = Vec::new();
        let thread = Thread {
            name: Some("Rouge éclatant très très très long".into()),
            ..Thread::from_color(Rgb::new(200, 0, 0))
        };
        encode(&[thread], &mut warnings);
        assert!(matches!(
            &warnings[..],
            [Warning::ThreadNameAltered { block: 0, written, .. }] if written.len() == 31
        ));
    }

    #[test]
    fn non_numeric_references_are_reported() {
        let mut warnings = Vec::new();
        let thread = Thread {
            catalog_number: Some("A-12".into()),
            name: Some("x".into()),
            ..Thread::from_color(Rgb::new(1, 2, 3))
        };
        encode(&[thread], &mut warnings);
        assert_eq!(
            warnings,
            vec![Warning::ThreadDetailsNotStored { blocks: vec![0] }]
        );
    }

    #[test]
    fn other_trailers_are_classified() {
        assert_eq!(decode(&[]), Trailer::Empty);
        assert_eq!(decode(b"\0\0garbage"), Trailer::Unknown);
        assert_eq!(
            decode(b"\0\0ENHANCE\x01\0\0\0\0\xFF\0\0\0"),
            Trailer::Unreadable
        );
        // A block without thread section holds no thread.
        assert_eq!(decode(b"ENHANCE\x00"), Trailer::Unreadable);
    }
}
