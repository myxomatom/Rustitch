//! Machine-unit representation of a design, shared by every format.
//!
//! Quantisation always rounds **absolute** positions; displacements are then
//! differences between rounded positions, so rounding errors never accumulate.

use emb_core::{Mm, PatternError, Point, Stitch, StitchKind, StitchPattern, Thread};

use crate::error::FormatError;

/// Rounds a length to the nearest machine unit.
///
/// This is the single rounding function of the export path. Halfway values
/// round away from zero.
///
/// # Errors
///
/// [`FormatError::CoordinateOutOfRange`] if the result does not fit in an `i32`.
///
/// ```
/// use emb_core::Mm;
/// use emb_formats::quantize;
///
/// assert_eq!(quantize(Mm(1.26), 10).unwrap(), 13);
/// assert_eq!(quantize(Mm(-0.05), 10).unwrap(), -1);
/// ```
pub fn quantize(value: Mm, units_per_mm: u32) -> Result<i32, FormatError> {
    let scaled = (value.get() * f64::from(units_per_mm)).round();
    let in_range = scaled >= f64::from(i32::MIN) && scaled <= f64::from(i32::MAX);
    if !in_range {
        return Err(FormatError::CoordinateOutOfRange { value });
    }
    // In range and integral: the cast is exact.
    #[allow(clippy::cast_possible_truncation)]
    Ok(scaled as i32)
}

/// Converts machine units back to millimetres.
///
/// ```
/// use emb_core::{Mm, EPSILON_MM};
/// use emb_formats::dequantize;
///
/// assert!(dequantize(-13, 10).approx_eq(Mm(-1.3), EPSILON_MM));
/// ```
#[must_use]
pub fn dequantize(units: i32, units_per_mm: u32) -> Mm {
    Mm(f64::from(units) / f64::from(units_per_mm))
}

/// A stitch at an absolute position in machine units (axes as in the pivot model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MachineStitch {
    /// Machine action.
    pub kind: StitchKind,
    /// Horizontal position, growing to the right.
    pub x: i32,
    /// Vertical position, growing upwards.
    pub y: i32,
}

impl MachineStitch {
    /// Creates a stitch.
    #[must_use]
    pub const fn new(kind: StitchKind, x: i32, y: i32) -> Self {
        Self { kind, x, y }
    }

    /// Whether the stitch is at the given position.
    #[must_use]
    pub const fn is_at(&self, x: i32, y: i32) -> bool {
        self.x == x && self.y == y
    }
}

/// Bounding box in machine units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineBounds {
    /// Smallest horizontal position.
    pub min_x: i32,
    /// Smallest vertical position.
    pub min_y: i32,
    /// Largest horizontal position.
    pub max_x: i32,
    /// Largest vertical position.
    pub max_y: i32,
}

/// A design quantised to a machine grid, with one thread per colour block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachinePattern {
    stitches: Vec<MachineStitch>,
    threads: Vec<Thread>,
}

impl MachinePattern {
    /// Builds a machine pattern after checking the block/thread invariant and
    /// the position of `End`.
    ///
    /// # Errors
    ///
    /// [`PatternError`] if an invariant is violated.
    pub fn new(stitches: Vec<MachineStitch>, threads: Vec<Thread>) -> Result<Self, PatternError> {
        if let Some(index) = stitches
            .iter()
            .position(|s| s.kind == StitchKind::End)
            .filter(|&i| i + 1 != stitches.len())
        {
            return Err(PatternError::EndNotLast { index });
        }
        let blocks = machine_block_count(&stitches);
        if blocks != threads.len() {
            return Err(PatternError::ThreadCountMismatch {
                blocks,
                threads: threads.len(),
            });
        }
        Ok(Self { stitches, threads })
    }

    /// Quantises a pivot pattern (absolute rounding of every position).
    ///
    /// # Errors
    ///
    /// [`FormatError::CoordinateOutOfRange`] if a position does not fit.
    pub fn from_pattern(pattern: &StitchPattern, units_per_mm: u32) -> Result<Self, FormatError> {
        let stitches = pattern
            .stitches()
            .iter()
            .map(|s| {
                Ok(MachineStitch::new(
                    s.kind,
                    quantize(s.position.x, units_per_mm)?,
                    quantize(s.position.y, units_per_mm)?,
                ))
            })
            .collect::<Result<Vec<_>, FormatError>>()?;
        Ok(Self {
            stitches,
            threads: pattern.threads().to_vec(),
        })
    }

    /// Converts back to a pivot pattern (metadata left empty).
    ///
    /// # Errors
    ///
    /// Never fails for a pattern built through [`MachinePattern::new`]; the
    /// pivot invariants are checked again nonetheless.
    pub fn to_pattern(&self, units_per_mm: u32) -> Result<StitchPattern, PatternError> {
        let stitches = self
            .stitches
            .iter()
            .map(|s| {
                Stitch::new(
                    s.kind,
                    Point::new(dequantize(s.x, units_per_mm), dequantize(s.y, units_per_mm)),
                )
            })
            .collect();
        StitchPattern::new(stitches, self.threads.clone())
    }

    /// All stitches, in sewing order.
    #[must_use]
    pub fn stitches(&self) -> &[MachineStitch] {
        &self.stitches
    }

    /// One thread per colour block.
    #[must_use]
    pub fn threads(&self) -> &[Thread] {
        &self.threads
    }

    /// Splits the pattern into its stitches and threads.
    #[must_use]
    pub fn into_parts(self) -> (Vec<MachineStitch>, Vec<Thread>) {
        (self.stitches, self.threads)
    }

    /// Translates the design by whole machine units so that its bounding box
    /// is centred on the origin (no rounding of relative positions).
    ///
    /// # Errors
    ///
    /// [`FormatError::CoordinateOutOfRange`] if a translated position does
    /// not fit in an `i32` (only possible for designs spanning most of the
    /// `i32` range).
    pub fn centered(self, units_per_mm: u32) -> Result<Self, FormatError> {
        let Some(bounds) = self.bounds() else {
            return Ok(self);
        };
        let offset_x = -(i64::from(bounds.min_x) + i64::from(bounds.max_x)).div_euclid(2);
        let offset_y = -(i64::from(bounds.min_y) + i64::from(bounds.max_y)).div_euclid(2);
        let shift = |value: i32, offset: i64| {
            i32::try_from(i64::from(value) + offset).map_err(|_| {
                FormatError::CoordinateOutOfRange {
                    value: dequantize(value, units_per_mm),
                }
            })
        };
        let stitches = self
            .stitches
            .into_iter()
            .map(|s| {
                Ok(MachineStitch::new(
                    s.kind,
                    shift(s.x, offset_x)?,
                    shift(s.y, offset_y)?,
                ))
            })
            .collect::<Result<Vec<_>, FormatError>>()?;
        Ok(Self {
            stitches,
            threads: self.threads,
        })
    }

    /// Bounding box of every stitch position, or `None` if empty.
    #[must_use]
    pub fn bounds(&self) -> Option<MachineBounds> {
        self.stitches.iter().fold(None, |acc, s| {
            Some(match acc {
                None => MachineBounds {
                    min_x: s.x,
                    min_y: s.y,
                    max_x: s.x,
                    max_y: s.y,
                },
                Some(b) => MachineBounds {
                    min_x: b.min_x.min(s.x),
                    min_y: b.min_y.min(s.y),
                    max_x: b.max_x.max(s.x),
                    max_y: b.max_y.max(s.y),
                },
            })
        })
    }
}

/// Number of colour blocks implied by a machine stitch list.
#[must_use]
pub fn machine_block_count(stitches: &[MachineStitch]) -> usize {
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
    use emb_core::{EPSILON_MM, Rgb};

    #[test]
    fn quantize_rounds_halfway_away_from_zero() {
        assert_eq!(quantize(Mm(0.25), 10).unwrap(), 3);
        assert_eq!(quantize(Mm(-0.25), 10).unwrap(), -3);
        assert_eq!(quantize(Mm(0.24), 10).unwrap(), 2);
    }

    #[test]
    fn quantize_rejects_unrepresentable_values() {
        assert!(quantize(Mm(1e12), 10).is_err());
        assert!(quantize(Mm(f64::NAN), 10).is_err());
        assert!(quantize(Mm(f64::NEG_INFINITY), 10).is_err());
    }

    #[test]
    fn dequantize_then_quantize_is_identity() {
        for units in [-2_000_000, -121, -1, 0, 1, 127, 9_999_999] {
            assert_eq!(quantize(dequantize(units, 10), 10).unwrap(), units);
        }
    }

    #[test]
    fn absolute_rounding_does_not_accumulate_error() {
        // 1000 steps of 0.04 mm: rounding each step would never move.
        let stitches = (0..=1000)
            .map(|i| {
                Stitch::new(
                    StitchKind::Normal,
                    Point::new(Mm(f64::from(i) * 0.04), Mm::ZERO),
                )
            })
            .collect();
        let thread = Thread::from_color(Rgb::new(0, 0, 0));
        let pattern = StitchPattern::new(stitches, vec![thread]).unwrap();
        let machine = MachinePattern::from_pattern(&pattern, 10).unwrap();
        let last = machine.stitches().last().unwrap();
        assert_eq!(last.x, 400);
        let back = machine.to_pattern(10).unwrap();
        for (a, b) in pattern.stitches().iter().zip(back.stitches()) {
            assert!(a.position.x.approx_eq(b.position.x, Mm(0.05) + EPSILON_MM));
        }
    }

    #[test]
    fn centering_moves_the_bounding_box_centre_to_the_origin() {
        let stitches = vec![
            MachineStitch::new(StitchKind::Normal, 10, -3),
            MachineStitch::new(StitchKind::Jump, 31, 7),
        ];
        let thread = Thread::from_color(Rgb::new(0, 0, 0));
        let centered = MachinePattern::new(stitches, vec![thread])
            .unwrap()
            .centered(10)
            .unwrap();
        let b = centered.bounds().unwrap();
        assert_eq!((b.min_x, b.max_x, b.min_y, b.max_y), (-10, 11, -5, 5));
    }

    #[test]
    fn new_checks_thread_count() {
        let stitches = vec![MachineStitch::new(StitchKind::Normal, 0, 0)];
        assert!(MachinePattern::new(stitches, vec![]).is_err());
    }
}
