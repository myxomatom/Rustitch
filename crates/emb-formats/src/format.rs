//! Format registry and detection.

use std::fmt;

use crate::codec::{FormatReader, FormatWriter};
use crate::dst::{DstReader, DstWriter, looks_like_dst};
use crate::jef::{JefReader, JefWriter, looks_like_jef};
use crate::pes::{PesReader, looks_like_pes};

/// A supported file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    /// Tajima `.dst`.
    Dst,
    /// Janome `.jef`.
    Jef,
    /// Brother `.pes` (reading only for now).
    Pes,
}

impl Format {
    /// Every supported format (all can be read).
    pub const ALL: [Format; 3] = [Format::Dst, Format::Jef, Format::Pes];

    /// Formats that can also be written.
    pub const WRITABLE: [Format; 2] = [Format::Dst, Format::Jef];

    /// Short upper-case name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dst => "DST",
            Self::Jef => "JEF",
            Self::Pes => "PES",
        }
    }

    /// Usual file extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Dst => "dst",
            Self::Jef => "jef",
            Self::Pes => "pes",
        }
    }

    /// Format usually associated with a file extension (case-insensitive,
    /// with or without the leading dot).
    ///
    /// ```
    /// use emb_formats::Format;
    ///
    /// assert_eq!(Format::from_extension(".JEF"), Some(Format::Jef));
    /// assert_eq!(Format::from_extension("svg"), None);
    /// ```
    #[must_use]
    pub fn from_extension(extension: &str) -> Option<Format> {
        let extension = extension.trim_start_matches('.');
        Self::ALL
            .into_iter()
            .find(|f| f.extension().eq_ignore_ascii_case(extension))
    }

    /// Detects the format from the file content (binary signature).
    #[must_use]
    pub fn detect(data: &[u8]) -> Option<Format> {
        if looks_like_pes(data) {
            Some(Self::Pes)
        } else if looks_like_dst(data) {
            Some(Self::Dst)
        } else if looks_like_jef(data) {
            Some(Self::Jef)
        } else {
            None
        }
    }

    /// The reader of this format.
    #[must_use]
    pub fn reader(self) -> Box<dyn FormatReader> {
        match self {
            Self::Dst => Box::new(DstReader),
            Self::Jef => Box::new(JefReader),
            Self::Pes => Box::new(PesReader),
        }
    }

    /// The writer of this format, with default options, or `None` if the
    /// format cannot be written yet.
    ///
    /// ```
    /// use emb_formats::Format;
    ///
    /// assert!(Format::Jef.writer().is_some());
    /// assert!(Format::Pes.writer().is_none());
    /// ```
    #[must_use]
    pub fn writer(self) -> Option<Box<dyn FormatWriter>> {
        match self {
            Self::Dst => Some(Box::new(DstWriter)),
            Self::Jef => Some(Box::new(JefWriter::default())),
            Self::Pes => None,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread};

    #[test]
    fn written_files_are_detected_as_their_format() {
        let pattern = StitchPattern::new(
            vec![Stitch::new(
                StitchKind::Normal,
                Point::new(Mm(1.0), Mm(1.0)),
            )],
            vec![Thread::from_color(Rgb::new(1, 2, 3))],
        )
        .unwrap();
        for format in Format::WRITABLE {
            let mut bytes = Vec::new();
            format
                .writer()
                .unwrap()
                .write(&pattern, &mut bytes)
                .unwrap();
            assert_eq!(Format::detect(&bytes), Some(format));
        }
    }

    #[test]
    fn writable_formats_are_exactly_those_with_a_writer() {
        for format in Format::ALL {
            assert_eq!(
                format.writer().is_some(),
                Format::WRITABLE.contains(&format),
                "{format}"
            );
        }
    }

    #[test]
    fn garbage_is_not_detected() {
        assert_eq!(Format::detect(b""), None);
        assert_eq!(Format::detect(&[0xFF; 600]), None);
        assert_eq!(Format::detect(b"#PESabcd\0\0\0\0"), None);
    }
}
