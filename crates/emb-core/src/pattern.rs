//! The pivot model: every conversion goes through [`StitchPattern`].

use std::collections::BTreeMap;

use thiserror::Error;

use crate::stitch::{Stitch, StitchKind};
use crate::thread::Thread;
use crate::time::Timestamp;
use crate::units::{Bounds, Mm};

/// Invariant violations detected when building a [`StitchPattern`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PatternError {
    /// The number of threads differs from the number of colour blocks.
    #[error("{threads} thread(s) for {blocks} colour block(s)")]
    ThreadCountMismatch {
        /// Number of colour blocks in the stitch list.
        blocks: usize,
        /// Number of threads supplied.
        threads: usize,
    },
    /// A stitch position is infinite or NaN.
    #[error("stitch {index} has a non-finite position")]
    NonFinitePosition {
        /// Index of the offending stitch.
        index: usize,
    },
    /// An `End` stitch appears before the last position.
    #[error("End stitch at index {index} is not the last stitch")]
    EndNotLast {
        /// Index of the offending stitch.
        index: usize,
    },
}

/// Size of an embroidery hoop (sewable area).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoopSize {
    /// Horizontal extent.
    pub width: Mm,
    /// Vertical extent.
    pub height: Mm,
}

/// Descriptive information about a design.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    /// Design name.
    pub name: Option<String>,
    /// Author.
    pub author: Option<String>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Free-form comments.
    pub comments: Option<String>,
    /// Hoop the design was originally made for.
    pub hoop: Option<HoopSize>,
    /// Creation or last modification date.
    pub date: Option<Timestamp>,
}

/// Counts of each kind of stitch in a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PatternStats {
    /// Visible stitches.
    pub normal: usize,
    /// Jumps.
    pub jumps: usize,
    /// Trims.
    pub trims: usize,
    /// Colour changes.
    pub color_changes: usize,
    /// Stops.
    pub stops: usize,
    /// Sequin ejections.
    pub sequins: usize,
}

/// A colour block: the stitches sewn with one thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorBlock<'a> {
    /// Thread of the block.
    pub thread: &'a Thread,
    /// Stitches of the block, excluding the `ColorChange` that ends it.
    pub stitches: &'a [Stitch],
}

/// A complete embroidery design in absolute millimetre coordinates.
///
/// The stitch list is split into colour blocks by `ColorChange` stitches; a
/// non-empty pattern with `n` colour changes has `n + 1` blocks, and
/// `threads()[i]` is the thread of block `i`. An empty pattern has no block.
///
/// ```
/// use emb_core::{Mm, Point, Rgb, Stitch, StitchKind, StitchPattern, Thread};
///
/// let p = |x, y| Point::new(Mm(x), Mm(y));
/// let pattern = StitchPattern::new(
///     vec![
///         Stitch::new(StitchKind::Normal, p(0.0, 0.0)),
///         Stitch::new(StitchKind::ColorChange, p(0.0, 0.0)),
///         Stitch::new(StitchKind::Normal, p(1.0, 0.0)),
///     ],
///     vec![
///         Thread::from_color(Rgb::new(0, 0, 0)),
///         Thread::from_color(Rgb::new(255, 0, 0)),
///     ],
/// )?;
/// assert_eq!(pattern.color_blocks().count(), 2);
/// # Ok::<(), emb_core::PatternError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StitchPattern {
    stitches: Vec<Stitch>,
    threads: Vec<Thread>,
    /// Descriptive information.
    pub metadata: Metadata,
    /// Format-specific raw data (`format → bytes`) kept for lossless
    /// round trips within the same format.
    pub extensions: BTreeMap<String, Vec<u8>>,
}

impl StitchPattern {
    /// Builds a pattern after checking its invariants.
    ///
    /// # Errors
    ///
    /// Returns a [`PatternError`] if a position is not finite, if `End` is
    /// not the last stitch, or if `threads` does not hold exactly one thread
    /// per colour block.
    pub fn new(stitches: Vec<Stitch>, threads: Vec<Thread>) -> Result<Self, PatternError> {
        if let Some(index) = stitches.iter().position(|s| !s.position.is_finite()) {
            return Err(PatternError::NonFinitePosition { index });
        }
        if let Some(index) = stitches
            .iter()
            .position(|s| s.kind == StitchKind::End)
            .filter(|&i| i + 1 != stitches.len())
        {
            return Err(PatternError::EndNotLast { index });
        }
        let blocks = block_count(&stitches);
        if threads.len() != blocks {
            return Err(PatternError::ThreadCountMismatch {
                blocks,
                threads: threads.len(),
            });
        }
        Ok(Self {
            stitches,
            threads,
            metadata: Metadata::default(),
            extensions: BTreeMap::new(),
        })
    }

    /// All stitches, in sewing order.
    #[must_use]
    pub fn stitches(&self) -> &[Stitch] {
        &self.stitches
    }

    /// One thread per colour block, in block order.
    #[must_use]
    pub fn threads(&self) -> &[Thread] {
        &self.threads
    }

    /// Splits the pattern into its stitches and threads.
    #[must_use]
    pub fn into_parts(self) -> (Vec<Stitch>, Vec<Thread>) {
        (self.stitches, self.threads)
    }

    /// Iterates over the colour blocks, in sewing order.
    pub fn color_blocks(&self) -> impl Iterator<Item = ColorBlock<'_>> {
        let runs = if self.stitches.is_empty() {
            None
        } else {
            Some(self.stitches.split(|s| s.kind == StitchKind::ColorChange))
        };
        runs.into_iter()
            .flatten()
            .zip(&self.threads)
            .map(|(stitches, thread)| ColorBlock { thread, stitches })
    }

    /// Bounding box of every stitch position (including jumps and commands),
    /// or `None` for an empty pattern.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::of(self.stitches.iter().map(|s| s.position))
    }

    /// Number of stitches of each kind.
    #[must_use]
    pub fn stats(&self) -> PatternStats {
        let mut stats = PatternStats::default();
        for stitch in &self.stitches {
            match stitch.kind {
                StitchKind::Normal => stats.normal += 1,
                StitchKind::Jump => stats.jumps += 1,
                StitchKind::Trim => stats.trims += 1,
                StitchKind::ColorChange => stats.color_changes += 1,
                StitchKind::Stop => stats.stops += 1,
                StitchKind::SequinEject => stats.sequins += 1,
                StitchKind::End => {}
            }
        }
        stats
    }
}

/// Number of colour blocks implied by a stitch list.
///
/// ```
/// use emb_core::{block_count, Point, Stitch, StitchKind};
///
/// assert_eq!(block_count(&[]), 0);
/// let s = |kind| Stitch::new(kind, Point::ORIGIN);
/// assert_eq!(block_count(&[s(StitchKind::Normal)]), 1);
/// assert_eq!(block_count(&[s(StitchKind::Normal), s(StitchKind::ColorChange)]), 2);
/// ```
#[must_use]
pub fn block_count(stitches: &[Stitch]) -> usize {
    if stitches.is_empty() {
        0
    } else {
        1 + stitches
            .iter()
            .filter(|s| s.kind == StitchKind::ColorChange)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::units::{EPSILON_MM, Point};

    fn at(kind: StitchKind, x: f64, y: f64) -> Stitch {
        Stitch::new(kind, Point::new(Mm(x), Mm(y)))
    }

    fn thread(v: u8) -> Thread {
        Thread::from_color(Rgb::new(v, v, v))
    }

    #[test]
    fn empty_pattern_has_no_block_and_no_bounds() {
        let pattern = StitchPattern::new(vec![], vec![]).unwrap();
        assert_eq!(pattern.color_blocks().count(), 0);
        assert!(pattern.bounds().is_none());
    }

    #[test]
    fn empty_pattern_rejects_threads() {
        assert_eq!(
            StitchPattern::new(vec![], vec![thread(1)]),
            Err(PatternError::ThreadCountMismatch {
                blocks: 0,
                threads: 1
            })
        );
    }

    #[test]
    fn thread_count_must_match_block_count() {
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::ColorChange, 0.0, 0.0),
            at(StitchKind::Normal, 1.0, 1.0),
        ];
        assert_eq!(
            StitchPattern::new(stitches, vec![thread(1)]),
            Err(PatternError::ThreadCountMismatch {
                blocks: 2,
                threads: 1
            })
        );
    }

    #[test]
    fn non_finite_positions_are_rejected() {
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::Normal, f64::NAN, 0.0),
        ];
        assert_eq!(
            StitchPattern::new(stitches, vec![thread(1)]),
            Err(PatternError::NonFinitePosition { index: 1 })
        );
    }

    #[test]
    fn end_must_be_last() {
        let stitches = vec![
            at(StitchKind::End, 0.0, 0.0),
            at(StitchKind::Normal, 0.0, 0.0),
        ];
        assert_eq!(
            StitchPattern::new(stitches, vec![thread(1)]),
            Err(PatternError::EndNotLast { index: 0 })
        );
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::End, 0.0, 0.0),
        ];
        assert!(StitchPattern::new(stitches, vec![thread(1)]).is_ok());
    }

    #[test]
    fn color_blocks_follow_color_changes_in_order() {
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::Normal, 1.0, 0.0),
            at(StitchKind::ColorChange, 1.0, 0.0),
            at(StitchKind::ColorChange, 1.0, 0.0),
            at(StitchKind::Normal, 2.0, 0.0),
        ];
        let pattern = StitchPattern::new(stitches, vec![thread(1), thread(2), thread(3)]).unwrap();
        let blocks: Vec<_> = pattern
            .color_blocks()
            .map(|b| (b.thread.color.r, b.stitches.len()))
            .collect();
        assert_eq!(blocks, vec![(1, 2), (2, 0), (3, 1)]);
    }

    #[test]
    fn bounds_include_jumps() {
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::Jump, -5.0, 7.0),
            at(StitchKind::Normal, 2.0, 1.0),
        ];
        let pattern = StitchPattern::new(stitches, vec![thread(1)]).unwrap();
        let bounds = pattern.bounds().unwrap();
        assert!(bounds.width().approx_eq(Mm(7.0), EPSILON_MM));
        assert!(bounds.height().approx_eq(Mm(7.0), EPSILON_MM));
    }

    #[test]
    fn stats_count_each_kind() {
        let stitches = vec![
            at(StitchKind::Normal, 0.0, 0.0),
            at(StitchKind::Jump, 1.0, 0.0),
            at(StitchKind::Trim, 1.0, 0.0),
            at(StitchKind::Stop, 1.0, 0.0),
            at(StitchKind::SequinEject, 2.0, 0.0),
            at(StitchKind::ColorChange, 2.0, 0.0),
            at(StitchKind::End, 2.0, 0.0),
        ];
        let pattern = StitchPattern::new(stitches, vec![thread(1), thread(2)]).unwrap();
        assert_eq!(
            pattern.stats(),
            PatternStats {
                normal: 1,
                jumps: 1,
                trims: 1,
                color_changes: 1,
                stops: 1,
                sequins: 1,
            }
        );
    }
}
