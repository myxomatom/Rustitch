//! Brother thread palette referenced by PEC colour indices.
//!
//! Entry `i` of [`PEC_PALETTE`] is PEC index `i + 1` (index 0 means
//! "unknown"). Colours and names come from pyembroidery (`EmbThreadPec.py`,
//! MIT licence); the RGB values are approximations of physical threads and
//! remain to be checked against Brother's chart (see `docs/formats/pes.md`).

use emb_core::{Rgb, Thread};

/// One palette entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PecPaletteEntry {
    /// Display colour.
    pub color: Rgb,
    /// Brother colour name.
    pub name: &'static str,
}

const fn entry(r: u8, g: u8, b: u8, name: &'static str) -> PecPaletteEntry {
    PecPaletteEntry {
        color: Rgb::new(r, g, b),
        name,
    }
}

/// The 64 PEC palette colours, in index order starting at 1.
pub const PEC_PALETTE: [PecPaletteEntry; 64] = [
    entry(14, 31, 124, "Prussian Blue"),
    entry(10, 85, 163, "Blue"),
    entry(0, 135, 119, "Teal Green"),
    entry(75, 107, 175, "Cornflower Blue"),
    entry(237, 23, 31, "Red"),
    entry(209, 92, 0, "Reddish Brown"),
    entry(145, 54, 151, "Magenta"),
    entry(228, 154, 203, "Light Lilac"),
    entry(145, 95, 172, "Lilac"),
    entry(158, 214, 125, "Mint Green"),
    entry(232, 169, 0, "Deep Gold"),
    entry(254, 186, 53, "Orange"),
    entry(255, 255, 0, "Yellow"),
    entry(112, 188, 31, "Lime Green"),
    entry(186, 152, 0, "Brass"),
    entry(168, 168, 168, "Silver"),
    entry(125, 111, 0, "Russet Brown"),
    entry(255, 255, 179, "Cream Brown"),
    entry(79, 85, 86, "Pewter"),
    entry(0, 0, 0, "Black"),
    entry(11, 61, 145, "Ultramarine"),
    entry(119, 1, 118, "Royal Purple"),
    entry(41, 49, 51, "Dark Gray"),
    entry(42, 19, 1, "Dark Brown"),
    entry(246, 74, 138, "Deep Rose"),
    entry(178, 118, 36, "Light Brown"),
    entry(252, 187, 197, "Salmon Pink"),
    entry(254, 55, 15, "Vermilion"),
    entry(240, 240, 240, "White"),
    entry(106, 28, 138, "Violet"),
    entry(168, 221, 196, "Seacrest"),
    entry(37, 132, 187, "Sky Blue"),
    entry(254, 179, 67, "Pumpkin"),
    entry(255, 243, 107, "Cream Yellow"),
    entry(208, 166, 96, "Khaki"),
    entry(209, 84, 0, "Clay Brown"),
    entry(102, 186, 73, "Leaf Green"),
    entry(19, 74, 70, "Peacock Blue"),
    entry(135, 135, 135, "Gray"),
    entry(216, 204, 198, "Warm Gray"),
    entry(67, 86, 7, "Dark Olive"),
    entry(253, 217, 222, "Flesh Pink"),
    entry(249, 147, 188, "Pink"),
    entry(0, 56, 34, "Deep Green"),
    entry(178, 175, 212, "Lavender"),
    entry(104, 106, 176, "Wisteria Violet"),
    entry(239, 227, 185, "Beige"),
    entry(247, 56, 102, "Carmine"),
    entry(181, 75, 100, "Amber Red"),
    entry(19, 43, 26, "Olive Green"),
    entry(199, 1, 86, "Dark Fuchsia"),
    entry(254, 158, 50, "Tangerine"),
    entry(168, 222, 235, "Light Blue"),
    entry(0, 103, 62, "Emerald Green"),
    entry(78, 41, 144, "Purple"),
    entry(47, 126, 32, "Moss Green"),
    entry(255, 204, 204, "Flesh Pink"),
    entry(255, 217, 17, "Harvest Gold"),
    entry(9, 91, 166, "Electric Blue"),
    entry(240, 249, 112, "Lemon Yellow"),
    entry(227, 243, 91, "Fresh Green"),
    entry(255, 153, 0, "Orange"),
    entry(255, 240, 141, "Cream Yellow"),
    entry(255, 200, 200, "Applique"),
];

/// The thread of a PEC palette index, if valid (1 to 64).
///
/// The palette index is not a Brother catalogue number, so only the colour
/// and the name are set.
///
/// ```
/// use emb_formats::pec::palette::pec_palette_thread;
///
/// assert_eq!(pec_palette_thread(20).unwrap().name.as_deref(), Some("Black"));
/// assert!(pec_palette_thread(0).is_none());
/// ```
#[must_use]
pub fn pec_palette_thread(index: u8) -> Option<Thread> {
    let entry = PEC_PALETTE.get(usize::from(index).checked_sub(1)?)?;
    Some(Thread {
        name: Some(entry.name.to_owned()),
        ..Thread::from_color(entry.color)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_bounds_are_one_to_sixty_four() {
        assert!(pec_palette_thread(0).is_none());
        assert_eq!(
            pec_palette_thread(1).unwrap().name.as_deref(),
            Some("Prussian Blue")
        );
        assert_eq!(
            pec_palette_thread(64).unwrap().name.as_deref(),
            Some("Applique")
        );
        assert!(pec_palette_thread(65).is_none());
    }
}
