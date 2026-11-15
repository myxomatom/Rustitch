//! What each format can store, used to normalise designs before writing.

use emb_core::Mm;

/// How a format represents thread cuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimSupport {
    /// Trims are encoded as a dedicated sequence of jump records, and the
    /// machine also trims on its own after `machine_trims_after_jumps`
    /// consecutive jump records.
    JumpSequence {
        /// Number of consecutive jump records that triggers a machine trim.
        machine_trims_after_jumps: usize,
    },
    /// The format has no trim command: trims are dropped (and reported).
    NotStored,
}

/// How a format records thread colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorModel {
    /// No colour information at all.
    None,
    /// An index into a fixed manufacturer palette.
    PaletteIndex {
        /// Number of entries in the palette.
        palette_size: usize,
    },
}

/// A hoop supported by a format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hoop {
    /// Human-readable name.
    pub name: &'static str,
    /// Format-specific identifier written to files.
    pub code: i32,
    /// Sewable width.
    pub width: Mm,
    /// Sewable height.
    pub height: Mm,
}

/// Constraints of a file format, declared by its writer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormatCapabilities {
    /// Format name, for messages.
    pub name: &'static str,
    /// Coordinate resolution: machine units per millimetre.
    pub units_per_mm: u32,
    /// Largest displacement per axis of one record, in machine units.
    pub max_delta: i32,
    /// Trim encoding.
    pub trims: TrimSupport,
    /// Whether stops (without colour change) can be stored.
    pub stops: bool,
    /// Whether sequins can be stored.
    pub sequins: bool,
    /// Colour representation.
    pub color_model: ColorModel,
    /// Largest number of colour blocks, if limited.
    pub max_colors: Option<usize>,
    /// Largest number of stitch records, if limited.
    pub max_stitches: Option<usize>,
    /// Supported hoops, smallest first (empty if the format has none).
    pub hoops: &'static [Hoop],
}
