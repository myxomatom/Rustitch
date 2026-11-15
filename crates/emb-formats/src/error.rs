//! Errors of the format readers and writers.

use emb_core::{Mm, PatternError};
use thiserror::Error;

/// Failure to read or write a machine file.
#[derive(Debug, Error)]
pub enum FormatError {
    /// Underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The data cannot be interpreted as the expected format at all.
    #[error("invalid {format} file: {reason}")]
    InvalidHeader {
        /// Format name.
        format: &'static str,
        /// What is wrong.
        reason: String,
    },
    /// A position cannot be represented in machine units.
    #[error("coordinate {} mm cannot be represented in machine units", .value.get())]
    CoordinateOutOfRange {
        /// The offending coordinate.
        value: Mm,
    },
    /// A header field is too small for the value to store.
    #[error("{format} header field {field} cannot hold the value {value}")]
    HeaderFieldOverflow {
        /// Format name.
        format: &'static str,
        /// Field name.
        field: &'static str,
        /// Value that does not fit.
        value: i64,
    },
    /// The design does not fit in any hoop of the target format.
    #[error(
        "design extends {} mm horizontally and {} mm vertically from the hoop centre, \
         which exceeds every hoop known for {format} so far",
        .half_width.get(), .half_height.get()
    )]
    DesignTooLarge {
        /// Format name.
        format: &'static str,
        /// Largest horizontal distance from the hoop centre.
        half_width: Mm,
        /// Largest vertical distance from the hoop centre.
        half_height: Mm,
    },
    /// The design uses more colour blocks than the format allows.
    #[error("{format} supports at most {max} colour blocks, the design has {actual}")]
    TooManyColors {
        /// Format name.
        format: &'static str,
        /// Format limit.
        max: usize,
        /// Blocks in the design.
        actual: usize,
    },
    /// The design has more stitch records than the format allows.
    #[error("{format} supports at most {max} stitches, the design needs {actual}")]
    TooManyStitches {
        /// Format name.
        format: &'static str,
        /// Format limit.
        max: usize,
        /// Stitches in the design.
        actual: usize,
    },
    /// A displacement exceeds what one record can encode (normalisation bug).
    #[error("displacement ({dx}, {dy}) cannot be encoded in a single {format} record")]
    DisplacementTooLong {
        /// Format name.
        format: &'static str,
        /// Horizontal displacement, in machine units.
        dx: i64,
        /// Vertical displacement, in machine units.
        dy: i64,
    },
    /// The decoded data violates a pattern invariant.
    #[error(transparent)]
    Pattern(#[from] PatternError),
}
