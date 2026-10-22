//! Conversion service shared by every front end (CLI, future MCP server and
//! GUI): format detection, target resolution, writer options and the
//! conversion itself. Front ends only handle files and presentation
//! (ADR 0002).
//!
//! ```
//! use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread};
//! use emb_formats::convert::{ConvertRequest, WriteOptions, convert};
//! use emb_formats::Format;
//!
//! let pattern = StitchPattern::new(
//!     vec![Stitch::new(StitchKind::Normal, Point::new(Mm(1.0), Mm(2.0)))],
//!     vec![Thread::from_color(Rgb::new(0, 0, 0))],
//! )?;
//! let mut dst = Vec::new();
//! Format::Dst.writer().ok_or("DST is writable")?.write(&pattern, &mut dst)?;
//!
//! let request = ConvertRequest {
//!     target: Format::Jef,
//!     options: WriteOptions::default(),
//!     default_date: None,
//! };
//! let conversion = convert(&dst, Some("dst"), &request)?;
//! assert_eq!(conversion.source.format, Format::Dst);
//! assert_eq!(Format::detect(&conversion.bytes), Some(Format::Jef));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use emb_core::{MatchStrategy, StitchPattern, Timestamp};
use thiserror::Error;

use crate::codec::{FormatWriter, ReadOutput};
use crate::error::FormatError;
use crate::format::Format;
use crate::jef::{JefWriter, Placement};
use crate::warning::Warning;

/// Failure of a conversion request.
#[derive(Debug, Error)]
pub enum ConvertError {
    /// Neither the content nor the extension identifies a known format.
    #[error("cannot recognise the file format")]
    UnknownFormat,
    /// No target format was given and none can be inferred.
    #[error("cannot infer the output format")]
    UnknownTarget,
    /// The target format has no writer yet.
    #[error("writing {0} files is not supported yet")]
    NotWritable(Format),
    /// An option was given for a format it does not apply to.
    #[error("option `{option}` only applies to {applies_to} output, not {target}")]
    OptionNotApplicable {
        /// Option name (field of [`WriteOptions`]).
        option: &'static str,
        /// Format the option applies to.
        applies_to: Format,
        /// Requested target format.
        target: Format,
    },
    /// The input could not be read.
    #[error("cannot read the design as {format}")]
    Read {
        /// Detected input format.
        format: Format,
        /// Reader failure.
        #[source]
        source: FormatError,
    },
    /// The design could not be written.
    #[error("cannot write the design as {format}")]
    Write {
        /// Target format.
        format: Format,
        /// Writer failure.
        #[source]
        source: FormatError,
    },
}

/// A design read from a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// Format of the content (from its signature, else from the extension).
    pub format: Format,
    /// Format the extension suggested, when it disagrees with the content.
    pub extension_mismatch: Option<Format>,
    /// The design and the reader's warnings.
    pub output: ReadOutput,
}

/// Format of `data`: the content signature wins, the extension (without the
/// dot, any case) is only a fallback. Also returns the format the extension
/// suggested when it disagrees with the content.
///
/// # Errors
///
/// [`ConvertError::UnknownFormat`] if neither identifies a known format.
pub fn detect(
    data: &[u8],
    extension: Option<&str>,
) -> Result<(Format, Option<Format>), ConvertError> {
    let hinted = extension.and_then(Format::from_extension);
    match (Format::detect(data), hinted) {
        (Some(detected), Some(hinted)) if detected != hinted => Ok((detected, Some(hinted))),
        (Some(format), _) | (None, Some(format)) => Ok((format, None)),
        (None, None) => Err(ConvertError::UnknownFormat),
    }
}

/// Detects the format of `data` and reads it.
///
/// # Errors
///
/// [`ConvertError::UnknownFormat`] or [`ConvertError::Read`].
pub fn load(data: &[u8], extension: Option<&str>) -> Result<Loaded, ConvertError> {
    let (format, extension_mismatch) = detect(data, extension)?;
    let mut input = data;
    let output = format
        .reader()
        .read(&mut input)
        .map_err(|source| ConvertError::Read { format, source })?;
    Ok(Loaded {
        format,
        extension_mismatch,
        output,
    })
}

/// The target format: the explicit one, else the one of the output extension.
///
/// # Errors
///
/// [`ConvertError::UnknownTarget`] if neither is known.
pub fn resolve_target(
    explicit: Option<Format>,
    output_extension: Option<&str>,
) -> Result<Format, ConvertError> {
    explicit
        .or_else(|| output_extension.and_then(Format::from_extension))
        .ok_or(ConvertError::UnknownTarget)
}

/// Writer options; `None` keeps the writer's default. Each option applies to
/// one format only, and setting it for another target is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteOptions {
    /// JEF: placement of the design in the hoop.
    pub placement: Option<Placement>,
    /// JEF: thread to palette matching.
    pub colors: Option<MatchStrategy>,
    /// JEF: whether to append the `ENHANCE` block with the real threads.
    pub enhance: Option<bool>,
}

impl WriteOptions {
    /// Names of the options that are set.
    fn set_options(&self) -> impl Iterator<Item = &'static str> {
        [
            ("placement", self.placement.is_some()),
            ("colors", self.colors.is_some()),
            ("enhance", self.enhance.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
    }
}

/// The writer of `target` configured with `options`.
///
/// # Errors
///
/// [`ConvertError::NotWritable`] or [`ConvertError::OptionNotApplicable`].
pub fn writer(
    target: Format,
    options: &WriteOptions,
) -> Result<Box<dyn FormatWriter>, ConvertError> {
    if target == Format::Jef {
        let defaults = JefWriter::default();
        return Ok(Box::new(JefWriter {
            placement: options.placement.unwrap_or(defaults.placement),
            colors: options.colors.unwrap_or(defaults.colors),
            enhance: options.enhance.unwrap_or(defaults.enhance),
        }));
    }
    if let Some(option) = options.set_options().next() {
        return Err(ConvertError::OptionNotApplicable {
            option,
            applies_to: Format::Jef,
            target,
        });
    }
    target.writer().ok_or(ConvertError::NotWritable(target))
}

/// What to convert to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertRequest {
    /// Target format.
    pub target: Format,
    /// Writer options.
    pub options: WriteOptions,
    /// Date given to designs that have none (the library never reads the
    /// clock: the caller supplies "now" if it wants one).
    pub default_date: Option<Timestamp>,
}

/// A completed conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// The input as read.
    pub source: Loaded,
    /// The design as written (with the default date applied).
    pub pattern: StitchPattern,
    /// Target format.
    pub target: Format,
    /// The written file.
    pub bytes: Vec<u8>,
    /// Losses and approximations of the writer.
    pub write_warnings: Vec<Warning>,
}

/// Converts the file `data` to `request.target`. The writer and its options
/// are validated before the input is read.
///
/// # Errors
///
/// Any [`ConvertError`].
pub fn convert(
    data: &[u8],
    input_extension: Option<&str>,
    request: &ConvertRequest,
) -> Result<Conversion, ConvertError> {
    let writer = writer(request.target, &request.options)?;
    let source = load(data, input_extension)?;
    let mut pattern = source.output.pattern.clone();
    if pattern.metadata.date.is_none() {
        pattern.metadata.date = request.default_date;
    }
    let mut bytes = Vec::new();
    let output = writer
        .write(&pattern, &mut bytes)
        .map_err(|source| ConvertError::Write {
            format: request.target,
            source,
        })?;
    Ok(Conversion {
        source,
        pattern,
        target: request.target,
        bytes,
        write_warnings: output.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, Thread};

    fn design() -> StitchPattern {
        StitchPattern::new(
            vec![Stitch::new(
                StitchKind::Normal,
                Point::new(Mm(1.0), Mm(1.0)),
            )],
            vec![Thread::from_color(Rgb::new(10, 20, 30))],
        )
        .unwrap()
    }

    fn file(format: Format) -> Vec<u8> {
        let mut bytes = Vec::new();
        format
            .writer()
            .unwrap()
            .write(&design(), &mut bytes)
            .unwrap();
        bytes
    }

    fn request(target: Format) -> ConvertRequest {
        ConvertRequest {
            target,
            options: WriteOptions::default(),
            default_date: None,
        }
    }

    #[test]
    fn content_wins_over_extension_and_mismatch_is_reported() {
        let jef = file(Format::Jef);
        assert_eq!(
            detect(&jef, Some("dst")).unwrap(),
            (Format::Jef, Some(Format::Dst))
        );
        assert_eq!(detect(&jef, Some("JEF")).unwrap(), (Format::Jef, None));
        assert_eq!(detect(&jef, None).unwrap(), (Format::Jef, None));
    }

    #[test]
    fn extension_is_a_fallback_only() {
        assert_eq!(
            detect(b"garbage", Some("dst")).unwrap(),
            (Format::Dst, None)
        );
        assert!(matches!(
            detect(b"garbage", None),
            Err(ConvertError::UnknownFormat)
        ));
        assert!(matches!(
            load(b"garbage", Some("dst")),
            Err(ConvertError::Read {
                format: Format::Dst,
                ..
            })
        ));
    }

    #[test]
    fn target_is_explicit_or_from_extension() {
        assert_eq!(
            resolve_target(Some(Format::Dst), Some("jef")).unwrap(),
            Format::Dst
        );
        assert_eq!(resolve_target(None, Some("jef")).unwrap(), Format::Jef);
        assert!(matches!(
            resolve_target(None, Some("xyz")),
            Err(ConvertError::UnknownTarget)
        ));
        assert!(matches!(
            resolve_target(None, None),
            Err(ConvertError::UnknownTarget)
        ));
    }

    #[test]
    fn jef_options_are_rejected_for_other_targets() {
        let options = WriteOptions {
            colors: Some(MatchStrategy::Nearest),
            ..WriteOptions::default()
        };
        assert!(matches!(
            writer(Format::Dst, &options),
            Err(ConvertError::OptionNotApplicable {
                option: "colors",
                ..
            })
        ));
        assert!(writer(Format::Jef, &options).is_ok());
        assert!(matches!(
            writer(Format::Pes, &WriteOptions::default()),
            Err(ConvertError::NotWritable(Format::Pes))
        ));
    }

    #[test]
    fn options_are_checked_before_reading_the_input() {
        let request = ConvertRequest {
            options: WriteOptions {
                enhance: Some(false),
                ..WriteOptions::default()
            },
            ..request(Format::Dst)
        };
        assert!(matches!(
            convert(b"garbage", None, &request),
            Err(ConvertError::OptionNotApplicable {
                option: "enhance",
                ..
            })
        ));
    }

    #[test]
    fn conversion_matches_the_direct_writer_and_applies_the_default_date() {
        let date = Timestamp::parse_compact("20261022120000");
        let request = ConvertRequest {
            default_date: date,
            ..request(Format::Jef)
        };
        let conversion = convert(&file(Format::Dst), Some("dst"), &request).unwrap();
        assert_eq!(conversion.pattern.metadata.date, date);
        let mut expected = Vec::new();
        JefWriter::default()
            .write(&conversion.pattern, &mut expected)
            .unwrap();
        assert_eq!(conversion.bytes, expected);
        assert!(
            conversion
                .source
                .output
                .warnings
                .contains(&Warning::DefaultColorsAssigned { blocks: 1 })
        );
    }

    #[test]
    fn an_existing_date_is_kept() {
        let mut dated = design();
        dated.metadata.date = Timestamp::parse_compact("20200101000000");
        let mut jef = Vec::new();
        JefWriter::default().write(&dated, &mut jef).unwrap();
        let request = ConvertRequest {
            default_date: Timestamp::parse_compact("20261022120000"),
            ..request(Format::Jef)
        };
        let conversion = convert(&jef, None, &request).unwrap();
        assert_eq!(conversion.pattern.metadata.date, dated.metadata.date);
    }
}
