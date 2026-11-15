//! Core types of the embroidery toolkit: units, colours, threads and the
//! pivot [`StitchPattern`] model that every file conversion goes through.
//!
//! All lengths are millimetres ([`Mm`]); the axis convention is documented in
//! [`units`].

#![forbid(unsafe_code)]

pub mod color;
pub mod matching;
pub mod pattern;
pub mod stitch;
pub mod thread;
pub mod time;
pub mod units;

pub use color::{DeltaE, Lab, Rgb, delta_e_2000};
pub use matching::{MatchStrategy, PaletteMatch, match_to_palette};
pub use pattern::{
    ColorBlock, HoopSize, Metadata, PatternError, PatternStats, StitchPattern, block_count,
};
pub use stitch::{Stitch, StitchKind};
pub use thread::Thread;
pub use time::Timestamp;
pub use units::{Bounds, EPSILON_MM, Mm, Point};
