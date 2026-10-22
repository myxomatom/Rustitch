//! Embroidery threads.

use crate::color::Rgb;

/// A physical embroidery thread.
///
/// Only the colour is mandatory: most file formats store little else.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Thread {
    /// Display colour (sRGB).
    pub color: Rgb,
    /// Commercial or descriptive name ("Navy Blue").
    pub name: Option<String>,
    /// Manufacturer or colour chart ("Madeira", "Isacord", "Janome"…).
    pub brand: Option<String>,
    /// Reference in the brand's catalogue ("1843", "002"…).
    pub catalog_number: Option<String>,
    /// Weight and/or material ("40 wt rayon"…).
    pub weight: Option<String>,
}

impl Thread {
    /// A thread known only by its colour.
    #[must_use]
    pub const fn from_color(color: Rgb) -> Self {
        Self {
            color,
            name: None,
            brand: None,
            catalog_number: None,
            weight: None,
        }
    }

    /// Whether two threads are the same physical thread: same colour **and**
    /// same catalogue reference (brand and number).
    ///
    /// Two threads that differ only by their reference are distinct, even
    /// when their colours are perceptually indistinguishable.
    ///
    /// ```
    /// use emb_core::{Rgb, Thread};
    ///
    /// let mut a = Thread::from_color(Rgb::new(10, 20, 30));
    /// let mut b = a.clone();
    /// b.name = Some("Renamed".into());
    /// assert!(a.is_same_thread(&b));
    /// a.catalog_number = Some("1843".into());
    /// assert!(!a.is_same_thread(&b));
    /// ```
    #[must_use]
    pub fn is_same_thread(&self, other: &Thread) -> bool {
        self.color == other.color
            && self.brand == other.brand
            && self.catalog_number == other.catalog_number
    }
}
