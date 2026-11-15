//! Readers and writers for embroidery machine formats.
//!
//! Every conversion goes through the pivot model [`emb_core::StitchPattern`]:
//! one reader and one writer per format, never a direct format-to-format
//! converter. Writers quantise absolute positions ([`quantize`]), then
//! [`normalize`] the design to the format's [`FormatCapabilities`].
//!
//! ```
//! use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread};
//! use emb_formats::Format;
//!
//! let pattern = StitchPattern::new(
//!     vec![Stitch::new(StitchKind::Normal, Point::new(Mm(1.0), Mm(2.0)))],
//!     vec![Thread::from_color(Rgb::new(0, 0, 0))],
//! )?;
//! let mut jef = Vec::new();
//! Format::Jef.writer().ok_or("JEF is writable")?.write(&pattern, &mut jef)?;
//! assert_eq!(Format::detect(&jef), Some(Format::Jef));
//! let back = Format::Jef.reader().read(&mut jef.as_slice())?;
//! assert_eq!(back.pattern.stitches().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]

pub mod capabilities;
pub mod codec;
pub mod convert;
pub mod dst;
pub mod error;
pub mod format;
pub mod jef;
pub mod machine;
pub mod normalize;
pub mod pec;
pub mod pes;
mod threads;
pub mod warning;

pub use capabilities::{ColorModel, FormatCapabilities, Hoop, TrimSupport};
pub use codec::{FormatReader, FormatWriter, ReadOutput, WriteOutput};
pub use error::FormatError;
pub use format::Format;
pub use machine::{MachinePattern, MachineStitch, dequantize, quantize};
pub use normalize::{Normalized, normalize};
pub use warning::Warning;
