//! Reader and writer abstractions implemented by every format.

use std::io::{Read, Write};

use emb_core::StitchPattern;

use crate::capabilities::FormatCapabilities;
use crate::error::FormatError;
use crate::warning::Warning;

/// A design read from a file, with everything the reader had to guess or skip.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadOutput {
    /// The decoded design.
    pub pattern: StitchPattern,
    /// Anomalies found in the file.
    pub warnings: Vec<Warning>,
}

/// What a writer had to approximate or drop.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WriteOutput {
    /// Losses and approximations.
    pub warnings: Vec<Warning>,
}

/// Decodes a machine file into the pivot model.
///
/// Readers are tolerant: a truncated or slightly corrupted file is read up to
/// its last valid record and the problem is reported as a [`Warning`]. They
/// never panic, whatever the input.
pub trait FormatReader {
    /// Reads a whole file.
    ///
    /// # Errors
    ///
    /// [`FormatError`] if the data cannot be interpreted at all.
    fn read(&self, input: &mut dyn Read) -> Result<ReadOutput, FormatError>;
}

/// Encodes the pivot model into a machine file.
///
/// Writers quantise (absolute rounding) and normalise the design to their
/// [`FormatCapabilities`] before encoding, so any valid pattern can be written.
pub trait FormatWriter {
    /// Constraints of the format.
    fn capabilities(&self) -> &FormatCapabilities;

    /// Writes a whole file.
    ///
    /// # Errors
    ///
    /// [`FormatError`] if the design cannot be represented in the format or
    /// if the output fails.
    fn write(
        &self,
        pattern: &StitchPattern,
        out: &mut dyn Write,
    ) -> Result<WriteOutput, FormatError>;
}
