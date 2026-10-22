//! Janome thread palette referenced by JEF colour indices.
//!
//! Entry `i` of [`JANOME_PALETTE`] is JEF index `i + 1` (index 0 is not a
//! colour). Colours, names and catalogue codes come from pyembroidery
//! (`EmbThreadJef.py`, MIT licence); the RGB values are approximations of
//! physical threads and remain to be checked against the official Janome
//! chart (see `docs/formats/jef.md`).

use emb_core::{DeltaE, MatchStrategy, Rgb, Thread, match_to_palette};

/// Brand recorded on threads read from JEF files.
pub const JANOME_BRAND: &str = "Janome";

/// One palette entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteEntry {
    /// Display colour.
    pub color: Rgb,
    /// Janome colour name.
    pub name: &'static str,
    /// Janome catalogue code.
    pub code: &'static str,
}

const fn entry(hex: u32, name: &'static str, code: &'static str) -> PaletteEntry {
    PaletteEntry {
        color: Rgb::from_hex(hex),
        name,
        code,
    }
}

/// The 78 Janome palette colours, in JEF index order starting at 1.
pub const JANOME_PALETTE: [PaletteEntry; 78] = [
    entry(0x00_0000, "Black", "002"),
    entry(0xFF_FFFF, "White", "001"),
    entry(0xFF_FF17, "Yellow", "204"),
    entry(0xFF_6600, "Orange", "203"),
    entry(0x2F_5933, "Olive Green", "219"),
    entry(0x23_7336, "Green", "226"),
    entry(0x65_C2C8, "Sky", "217"),
    entry(0xAB_5A96, "Purple", "208"),
    entry(0xF6_69A0, "Pink", "201"),
    entry(0xFF_0000, "Red", "225"),
    entry(0xB1_704E, "Brown", "214"),
    entry(0x0B_2F84, "Blue", "207"),
    entry(0xE4_C35D, "Gold", "003"),
    entry(0x48_1A05, "Dark Brown", "205"),
    entry(0xAC_9CC7, "Pale Violet", "209"),
    entry(0xFC_F294, "Pale Yellow", "210"),
    entry(0xF9_99B7, "Pale Pink", "211"),
    entry(0xFA_B381, "Peach", "212"),
    entry(0xC9_A480, "Beige", "213"),
    entry(0x97_0533, "Wine Red", "215"),
    entry(0xA0_B8CC, "Pale Sky", "216"),
    entry(0x7F_C21C, "Yellow Green", "218"),
    entry(0xE5_E5E5, "Silver Gray", "220"),
    entry(0x88_9B9B, "Gray", "221"),
    entry(0x98_D6BD, "Pale Aqua", "227"),
    entry(0xB2_E1E3, "Baby Blue", "228"),
    entry(0x36_8BA0, "Powder Blue", "229"),
    entry(0x4F_83AB, "Bright Blue", "230"),
    entry(0x38_6A91, "Slate Blue", "231"),
    entry(0x07_1650, "Navy Blue", "232"),
    entry(0xF9_99A2, "Salmon Pink", "233"),
    entry(0xF9_676B, "Coral", "234"),
    entry(0xE3_311F, "Burnt Orange", "235"),
    entry(0xE2_A188, "Cinnamon", "236"),
    entry(0xB5_9474, "Umber", "237"),
    entry(0xE4_CF99, "Blond", "238"),
    entry(0xFF_CB00, "Sunflower", "239"),
    entry(0xE1_ADD4, "Orchid Pink", "240"),
    entry(0xC3_007E, "Peony Purple", "241"),
    entry(0x80_004B, "Burgundy", "242"),
    entry(0x54_0571, "Royal Purple", "243"),
    entry(0xB1_0525, "Cardinal Red", "244"),
    entry(0xCA_E0C0, "Opal Green", "245"),
    entry(0x89_9856, "Moss Green", "246"),
    entry(0x5C_941A, "Meadow Green", "247"),
    entry(0x00_3114, "Dark Green", "248"),
    entry(0x5D_AE94, "Aquamarine", "249"),
    entry(0x4C_BF8F, "Emerald Green", "250"),
    entry(0x00_7772, "Peacock Green", "251"),
    entry(0x59_5B61, "Dark Gray", "252"),
    entry(0xFF_FFF2, "Ivory White", "253"),
    entry(0xB1_5818, "Hazel", "254"),
    entry(0xCB_8A07, "Toast", "255"),
    entry(0x98_6C80, "Salmon", "256"),
    entry(0x98_692D, "Cocoa Brown", "257"),
    entry(0x4D_3419, "Sienna", "258"),
    entry(0x4C_330B, "Sepia", "259"),
    entry(0x33_200A, "Dark Sepia", "260"),
    entry(0x52_3A97, "Violet Blue", "261"),
    entry(0x0D_217E, "Blue Ink", "262"),
    entry(0x1E_77AC, "Sola Blue", "263"),
    entry(0xB2_DD53, "Green Dust", "264"),
    entry(0xF3_3689, "Crimson", "265"),
    entry(0xDE_649E, "Floral Pink", "266"),
    entry(0x98_4161, "Wine", "267"),
    entry(0x4C_5612, "Olive Drab", "268"),
    entry(0x4C_881F, "Meadow", "269"),
    entry(0xE4_DE79, "Mustard", "270"),
    entry(0xCB_8A1A, "Yellow Ocher", "271"),
    entry(0xCB_A21C, "Old Gold", "272"),
    entry(0xFF_9805, "Honey Dew", "273"),
    entry(0xFC_B257, "Tangerine", "274"),
    entry(0xFF_E505, "Canary Yellow", "275"),
    entry(0xF0_331F, "Vermilion", "202"),
    entry(0x1A_842D, "Bright Green", "206"),
    entry(0x38_6CAE, "Ocean Blue", "222"),
    entry(0xE3_C4B4, "Beige Gray", "223"),
    entry(0xE3_AC81, "Bamboo", "224"),
];

/// The palette entry for a JEF index, if valid (1 to 78).
#[must_use]
pub fn palette_entry(index: i64) -> Option<&'static PaletteEntry> {
    let position = usize::try_from(index.checked_sub(1)?).ok()?;
    JANOME_PALETTE.get(position)
}

/// The thread corresponding to a palette entry.
#[must_use]
pub fn palette_thread(entry: &PaletteEntry) -> Thread {
    Thread {
        name: Some(entry.name.to_owned()),
        brand: Some(JANOME_BRAND.to_owned()),
        catalog_number: Some(entry.code.to_owned()),
        ..Thread::from_color(entry.color)
    }
}

/// Janome palette colour chosen for a thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JefColor {
    /// JEF index (1 to 78).
    pub index: u8,
    /// `None` for an exact match on the Janome catalogue reference, otherwise
    /// the CIEDE2000 difference to the chosen colour.
    pub delta_e: Option<DeltaE>,
}

/// Palette position designated by a Janome thread's own catalogue reference.
fn janome_reference(thread: &Thread) -> Option<usize> {
    let is_janome = thread
        .brand
        .as_deref()
        .is_some_and(|b| b.eq_ignore_ascii_case(JANOME_BRAND));
    thread
        .catalog_number
        .as_deref()
        .filter(|_| is_janome)
        .and_then(|code| JANOME_PALETTE.iter().position(|e| e.code == code))
}

/// Chooses one Janome palette colour per thread (one thread per colour
/// block). Janome threads keep their own reference; the others are matched
/// by ΔE00 with the given strategy (see [`emb_core::matching`]).
///
/// ```
/// use emb_core::{MatchStrategy, Rgb, Thread};
/// use emb_formats::jef::palette::match_threads;
///
/// let colors = match_threads(&[Thread::from_color(Rgb::new(250, 0, 5))], MatchStrategy::Injective);
/// assert_eq!(colors[0].index, 10); // Red
/// ```
#[must_use]
pub fn match_threads(threads: &[Thread], strategy: MatchStrategy) -> Vec<JefColor> {
    let palette: Vec<Rgb> = JANOME_PALETTE.iter().map(|e| e.color).collect();
    match_to_palette(threads, &palette, janome_reference, strategy)
        .unwrap_or_default()
        .into_iter()
        .map(|m| JefColor {
            index: u8::try_from(m.index + 1).unwrap_or(u8::MAX),
            delta_e: m.delta_e,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_codes_are_unique() {
        let mut codes: Vec<_> = JANOME_PALETTE.iter().map(|e| e.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), JANOME_PALETTE.len());
    }

    #[test]
    fn palette_indices_start_at_one() {
        assert!(palette_entry(0).is_none());
        assert_eq!(palette_entry(1).unwrap().name, "Black");
        assert_eq!(palette_entry(78).unwrap().name, "Bamboo");
        assert!(palette_entry(79).is_none());
        assert!(palette_entry(-1).is_none());
    }

    const STRATEGIES: [MatchStrategy; 2] = [MatchStrategy::Nearest, MatchStrategy::Injective];

    #[test]
    fn janome_reference_wins_over_colour() {
        let mut thread = Thread::from_color(Rgb::new(255, 0, 0));
        thread.brand = Some("janome".into());
        thread.catalog_number = Some("232".into()); // Navy Blue
        for strategy in STRATEGIES {
            let m = match_threads(std::slice::from_ref(&thread), strategy)[0];
            assert_eq!(m.index, 30);
            assert_eq!(m.delta_e, None);
        }
    }

    #[test]
    fn foreign_reference_falls_back_to_colour() {
        let mut thread = Thread::from_color(Rgb::new(0, 0, 0));
        thread.brand = Some("Madeira".into());
        thread.catalog_number = Some("232".into());
        for strategy in STRATEGIES {
            let m = match_threads(std::slice::from_ref(&thread), strategy)[0];
            assert_eq!(m.index, 1);
            assert!(m.delta_e.unwrap().get() < 1e-9);
        }
    }

    #[test]
    fn every_palette_thread_maps_back_to_its_index() {
        let threads: Vec<_> = JANOME_PALETTE.iter().map(palette_thread).collect();
        for strategy in STRATEGIES {
            for (position, m) in match_threads(&threads, strategy).iter().enumerate() {
                assert_eq!(usize::from(m.index), position + 1);
                assert_eq!(m.delta_e, None);
            }
        }
    }

    /// `Love1.pes`: Madeira Deep Sky Blue (1090) and Mallard Teal (1290) both
    /// have Janome Peacock Green (49) as nearest colour.
    #[test]
    fn close_madeira_threads_get_distinct_janome_colours_when_injective() {
        let madeira = |rgb, code: &str| Thread {
            brand: Some("Madeira Classic 40".into()),
            catalog_number: Some(code.into()),
            ..Thread::from_color(Rgb::from_hex(rgb))
        };
        let threads = [madeira(0x1C_7E82, "1090"), madeira(0x0E_4D4E, "1290")];
        let nearest = match_threads(&threads, MatchStrategy::Nearest);
        assert_eq!((nearest[0].index, nearest[1].index), (49, 49));
        let injective = match_threads(&threads, MatchStrategy::Injective);
        assert_ne!(injective[0].index, injective[1].index);
    }
}
