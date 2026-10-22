//! Physical units and planar geometry.
//!
//! # Axis convention
//!
//! The pivot model uses a right-handed Cartesian frame, as seen from the
//! front of the fabric:
//!
//! - **X grows to the right**;
//! - **Y grows upwards** (towards the top of the design).
//!
//! Every boundary converts explicitly from or to this convention (machine
//! formats, SVG import, rendering). Positions are absolute, in millimetres.

use std::ops::{Add, Neg, Sub};

/// Tolerance used to compare two lengths that should be equal.
pub const EPSILON_MM: Mm = Mm(1e-9);

/// A length in millimetres.
///
/// This is the only length unit of the public API: machine units only exist
/// at the boundaries of the file formats.
///
/// ```
/// use emb_core::Mm;
///
/// let a = Mm(1.5);
/// let b = Mm(0.25);
/// assert!((a - b).approx_eq(Mm(1.25), emb_core::EPSILON_MM));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Mm(pub f64);

impl Mm {
    /// Zero length.
    pub const ZERO: Mm = Mm(0.0);

    /// Returns the raw value in millimetres.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(self) -> Mm {
        Mm(self.0.abs())
    }

    /// Whether the value is neither infinite nor NaN.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }

    /// Whether `self` and `other` differ by at most `tolerance`.
    #[must_use]
    pub fn approx_eq(self, other: Mm, tolerance: Mm) -> bool {
        (self.0 - other.0).abs() <= tolerance.0
    }

    /// The smaller of two lengths.
    #[must_use]
    pub fn min(self, other: Mm) -> Mm {
        Mm(self.0.min(other.0))
    }

    /// The larger of two lengths.
    #[must_use]
    pub fn max(self, other: Mm) -> Mm {
        Mm(self.0.max(other.0))
    }
}

impl Add for Mm {
    type Output = Mm;

    fn add(self, rhs: Mm) -> Mm {
        Mm(self.0 + rhs.0)
    }
}

impl Sub for Mm {
    type Output = Mm;

    fn sub(self, rhs: Mm) -> Mm {
        Mm(self.0 - rhs.0)
    }
}

impl Neg for Mm {
    type Output = Mm;

    fn neg(self) -> Mm {
        Mm(-self.0)
    }
}

/// An absolute position in the design plane (see the [module](self) axis convention).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Horizontal coordinate, growing to the right.
    pub x: Mm,
    /// Vertical coordinate, growing upwards.
    pub y: Mm,
}

impl Point {
    /// The origin of the design plane.
    pub const ORIGIN: Point = Point {
        x: Mm::ZERO,
        y: Mm::ZERO,
    };

    /// Creates a point from its coordinates.
    #[must_use]
    pub const fn new(x: Mm, y: Mm) -> Self {
        Self { x, y }
    }

    /// Whether both coordinates are finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Euclidean distance between two points.
    ///
    /// ```
    /// use emb_core::{Mm, Point, EPSILON_MM};
    ///
    /// let d = Point::new(Mm(0.0), Mm(0.0)).distance_to(Point::new(Mm(3.0), Mm(4.0)));
    /// assert!(d.approx_eq(Mm(5.0), EPSILON_MM));
    /// ```
    #[must_use]
    pub fn distance_to(self, other: Point) -> Mm {
        Mm((other.x.0 - self.x.0).hypot(other.y.0 - self.y.0))
    }
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Bottom-left corner (smallest coordinates).
    pub min: Point,
    /// Top-right corner (largest coordinates).
    pub max: Point,
}

impl Bounds {
    /// Smallest box containing every point, or `None` for an empty iterator.
    ///
    /// ```
    /// use emb_core::{Bounds, Mm, Point, EPSILON_MM};
    ///
    /// let b = Bounds::of([Point::new(Mm(1.0), Mm(-2.0)), Point::new(Mm(-3.0), Mm(4.0))]).unwrap();
    /// assert!(b.width().approx_eq(Mm(4.0), EPSILON_MM));
    /// assert!(b.height().approx_eq(Mm(6.0), EPSILON_MM));
    /// ```
    pub fn of(points: impl IntoIterator<Item = Point>) -> Option<Bounds> {
        points.into_iter().fold(None, |acc, p| {
            Some(match acc {
                None => Bounds { min: p, max: p },
                Some(b) => Bounds {
                    min: Point::new(b.min.x.min(p.x), b.min.y.min(p.y)),
                    max: Point::new(b.max.x.max(p.x), b.max.y.max(p.y)),
                },
            })
        })
    }

    /// Horizontal extent.
    #[must_use]
    pub fn width(&self) -> Mm {
        self.max.x - self.min.x
    }

    /// Vertical extent.
    #[must_use]
    pub fn height(&self) -> Mm {
        self.max.y - self.min.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_eq_accepts_values_within_tolerance() {
        assert!(Mm(1.0).approx_eq(Mm(1.0 + 1e-12), EPSILON_MM));
        assert!(!Mm(1.0).approx_eq(Mm(1.001), EPSILON_MM));
    }

    #[test]
    fn non_finite_lengths_are_detected() {
        assert!(!Mm(f64::NAN).is_finite());
        assert!(!Point::new(Mm(0.0), Mm(f64::INFINITY)).is_finite());
        assert!(Point::new(Mm(-1.0), Mm(2.0)).is_finite());
    }

    #[test]
    fn bounds_of_empty_iterator_is_none() {
        assert!(Bounds::of(std::iter::empty()).is_none());
    }

    #[test]
    fn bounds_of_single_point_is_degenerate() {
        let p = Point::new(Mm(2.0), Mm(3.0));
        let b = Bounds::of([p]).unwrap();
        assert!(b.width().approx_eq(Mm::ZERO, EPSILON_MM));
        assert!(b.height().approx_eq(Mm::ZERO, EPSILON_MM));
    }
}
