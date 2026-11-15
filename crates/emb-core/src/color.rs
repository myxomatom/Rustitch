//! Colours: sRGB storage, CIELAB conversion and the CIEDE2000 perceptual distance.
//!
//! Colour comparisons must always go through [`delta_e_2000`]: a Euclidean
//! distance in RGB poorly separates close colours.

use std::fmt;

/// An 8-bit-per-channel sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Creates a colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a colour from a `0xRRGGBB` value (higher bits are ignored).
    ///
    /// ```
    /// use emb_core::Rgb;
    ///
    /// assert_eq!(Rgb::from_hex(0x12_34_56), Rgb::new(0x12, 0x34, 0x56));
    /// ```
    #[must_use]
    pub const fn from_hex(value: u32) -> Self {
        let [_, r, g, b] = value.to_be_bytes();
        Self { r, g, b }
    }

    /// Parses `#RRGGBB` or `RRGGBB` (case-insensitive).
    ///
    /// ```
    /// use emb_core::Rgb;
    ///
    /// assert_eq!(Rgb::parse_hex("#FF8000"), Some(Rgb::new(255, 128, 0)));
    /// assert_eq!(Rgb::parse_hex("nope"), None);
    /// ```
    #[must_use]
    pub fn parse_hex(text: &str) -> Option<Self> {
        let digits = text.strip_prefix('#').unwrap_or(text);
        if digits.len() != 6 || !digits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(digits, 16).ok().map(Self::from_hex)
    }

    /// Converts to CIELAB under illuminant D65 (2° observer).
    ///
    /// ```
    /// use emb_core::Rgb;
    ///
    /// let white = Rgb::new(255, 255, 255).to_lab();
    /// assert!((white.l - 100.0).abs() < 1e-3);
    /// ```
    #[must_use]
    pub fn to_lab(self) -> Lab {
        let red = srgb_to_linear(self.r);
        let green = srgb_to_linear(self.g);
        let blue = srgb_to_linear(self.b);
        // IEC 61966-2-1 primaries, D65 white point.
        let x = 0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue;
        let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175_0 * blue;
        let z = 0.019_333_9 * red + 0.119_192_0 * green + 0.950_304_1 * blue;
        let fx = lab_f(x / D65_WHITE[0]);
        let fy = lab_f(y / D65_WHITE[1]);
        let fz = lab_f(z / D65_WHITE[2]);
        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// D65 reference white in XYZ (Y normalised to 1).
const D65_WHITE: [f64; 3] = [0.950_47, 1.0, 1.088_83];

fn srgb_to_linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn lab_f(t: f64) -> f64 {
    const EPSILON: f64 = 216.0 / 24_389.0;
    const KAPPA: f64 = 24_389.0 / 27.0;
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

/// A colour in the CIELAB space (D65).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    /// Lightness L*, from 0 (black) to 100 (white).
    pub l: f64,
    /// Green (negative) to red (positive) axis a*.
    pub a: f64,
    /// Blue (negative) to yellow (positive) axis b*.
    pub b: f64,
}

/// A CIEDE2000 colour difference. About 1 is the threshold of perception.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DeltaE(pub f64);

impl DeltaE {
    /// Returns the raw value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl fmt::Display for DeltaE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

/// CIEDE2000 colour difference (`kL = kC = kH = 1`), after Sharma, Wu and
/// Dalal, *Color Research & Application* 30(1), 2005.
///
/// ```
/// use emb_core::{delta_e_2000, Lab};
///
/// let a = Lab { l: 50.0, a: 2.6772, b: -79.7751 };
/// let b = Lab { l: 50.0, a: 0.0, b: -82.7485 };
/// assert!((delta_e_2000(a, b).get() - 2.0425).abs() < 1e-4);
/// ```
#[must_use]
pub fn delta_e_2000(first: Lab, second: Lab) -> DeltaE {
    let pow25_7 = 25.0_f64.powi(7);
    let c_bar = f64::midpoint(first.a.hypot(first.b), second.a.hypot(second.b));
    let g = 0.5 * (1.0 - (c_bar.powi(7) / (c_bar.powi(7) + pow25_7)).sqrt());
    let a1 = (1.0 + g) * first.a;
    let a2 = (1.0 + g) * second.a;
    let c1 = a1.hypot(first.b);
    let c2 = a2.hypot(second.b);
    let h1 = hue_degrees(first.b, a1);
    let h2 = hue_degrees(second.b, a2);
    let chroma_product_is_zero = c1 * c2 == 0.0;

    let delta_l = second.l - first.l;
    let delta_c = c2 - c1;
    let delta_h_angle = if chroma_product_is_zero {
        0.0
    } else if (h2 - h1).abs() <= 180.0 {
        h2 - h1
    } else if h2 - h1 > 180.0 {
        h2 - h1 - 360.0
    } else {
        h2 - h1 + 360.0
    };
    let delta_h = 2.0 * (c1 * c2).sqrt() * (delta_h_angle.to_radians() / 2.0).sin();

    let l_mean = f64::midpoint(first.l, second.l);
    let c_mean = f64::midpoint(c1, c2);
    let h_mean = if chroma_product_is_zero {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        f64::midpoint(h1, h2)
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_mean - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_mean).to_radians().cos()
        + 0.32 * (3.0 * h_mean + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_mean - 63.0).to_radians().cos();
    let delta_theta = 30.0 * (-((h_mean - 275.0) / 25.0).powi(2)).exp();
    let r_c = 2.0 * (c_mean.powi(7) / (c_mean.powi(7) + pow25_7)).sqrt();
    let l_offset = (l_mean - 50.0).powi(2);
    let s_l = 1.0 + 0.015 * l_offset / (20.0 + l_offset).sqrt();
    let s_c = 1.0 + 0.045 * c_mean;
    let s_h = 1.0 + 0.015 * c_mean * t;
    let r_t = -(2.0 * delta_theta).to_radians().sin() * r_c;

    let l_term = delta_l / s_l;
    let c_term = delta_c / s_c;
    let h_term = delta_h / s_h;
    DeltaE((l_term.powi(2) + c_term.powi(2) + h_term.powi(2) + r_t * c_term * h_term).sqrt())
}

/// Hue angle in degrees, in `[0, 360)`.
fn hue_degrees(b: f64, a_prime: f64) -> f64 {
    let h = b.atan2(a_prime).to_degrees();
    if h < 0.0 { h + 360.0 } else { h }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sharma, Wu & Dalal test data: `L1 a1 b1 L2 a2 b2 ΔE00`.
    /// Source: <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/>.
    const SHARMA: [[f64; 7]; 34] = [
        [50.0000, 2.6772, -79.7751, 50.0000, 0.0000, -82.7485, 2.0425],
        [50.0000, 3.1571, -77.2803, 50.0000, 0.0000, -82.7485, 2.8615],
        [50.0000, 2.8361, -74.0200, 50.0000, 0.0000, -82.7485, 3.4412],
        [
            50.0000, -1.3802, -84.2814, 50.0000, 0.0000, -82.7485, 1.0000,
        ],
        [
            50.0000, -1.1848, -84.8006, 50.0000, 0.0000, -82.7485, 1.0000,
        ],
        [
            50.0000, -0.9009, -85.5211, 50.0000, 0.0000, -82.7485, 1.0000,
        ],
        [50.0000, 0.0000, 0.0000, 50.0000, -1.0000, 2.0000, 2.3669],
        [50.0000, -1.0000, 2.0000, 50.0000, 0.0000, 0.0000, 2.3669],
        [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0009, 7.1792],
        [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0010, 7.1792],
        [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0011, 7.2195],
        [50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0012, 7.2195],
        [50.0000, -0.0010, 2.4900, 50.0000, 0.0009, -2.4900, 4.8045],
        [50.0000, -0.0010, 2.4900, 50.0000, 0.0010, -2.4900, 4.8045],
        [50.0000, -0.0010, 2.4900, 50.0000, 0.0011, -2.4900, 4.7461],
        [50.0000, 2.5000, 0.0000, 50.0000, 0.0000, -2.5000, 4.3065],
        [50.0000, 2.5000, 0.0000, 73.0000, 25.0000, -18.0000, 27.1492],
        [50.0000, 2.5000, 0.0000, 61.0000, -5.0000, 29.0000, 22.8977],
        [50.0000, 2.5000, 0.0000, 56.0000, -27.0000, -3.0000, 31.9030],
        [50.0000, 2.5000, 0.0000, 58.0000, 24.0000, 15.0000, 19.4535],
        [50.0000, 2.5000, 0.0000, 50.0000, 3.1736, 0.5854, 1.0000],
        [50.0000, 2.5000, 0.0000, 50.0000, 3.2972, 0.0000, 1.0000],
        [50.0000, 2.5000, 0.0000, 50.0000, 1.8634, 0.5757, 1.0000],
        [50.0000, 2.5000, 0.0000, 50.0000, 3.2592, 0.3350, 1.0000],
        [
            60.2574, -34.0099, 36.2677, 60.4626, -34.1751, 39.4387, 1.2644,
        ],
        [
            63.0109, -31.0961, -5.8663, 62.8187, -29.7946, -4.0864, 1.2630,
        ],
        [61.2901, 3.7196, -5.3901, 61.4292, 2.2480, -4.9620, 1.8731],
        [35.0831, -44.1164, 3.7933, 35.0232, -40.0716, 1.5901, 1.8645],
        [
            22.7233, 20.0904, -46.6940, 23.0331, 14.9730, -42.5619, 2.0373,
        ],
        [36.4612, 47.8580, 18.3852, 36.2715, 50.5065, 21.2231, 1.4146],
        [90.8027, -2.0831, 1.4410, 91.1528, -1.6435, 0.0447, 1.4441],
        [90.9257, -0.5406, -0.9208, 88.6381, -0.8985, -0.7239, 1.5381],
        [6.7747, -0.2908, -2.4247, 5.8714, -0.0985, -2.2286, 0.6377],
        [2.0776, 0.0795, -1.1350, 0.9033, -0.0636, -0.5514, 0.9082],
    ];

    #[test]
    fn delta_e_2000_matches_sharma_reference_data() {
        for (i, row) in SHARMA.iter().enumerate() {
            let first = Lab {
                l: row[0],
                a: row[1],
                b: row[2],
            };
            let second = Lab {
                l: row[3],
                a: row[4],
                b: row[5],
            };
            let computed = delta_e_2000(first, second).get();
            assert!(
                (computed - row[6]).abs() < 1e-4,
                "pair {}: computed {computed}, expected {}",
                i + 1,
                row[6]
            );
        }
    }

    #[test]
    fn delta_e_2000_is_symmetric_and_zero_on_identity() {
        let a = Lab {
            l: 40.0,
            a: 12.0,
            b: -30.0,
        };
        let b = Lab {
            l: 42.0,
            a: 10.0,
            b: -28.0,
        };
        assert!((delta_e_2000(a, b).get() - delta_e_2000(b, a).get()).abs() < 1e-12);
        assert!(delta_e_2000(a, a).get().abs() < 1e-12);
    }

    /// Reference values: Bruce Lindbloom's colour calculator (sRGB, D65).
    #[test]
    fn srgb_primaries_convert_to_reference_lab() {
        let cases = [
            (Rgb::new(255, 0, 0), [53.2408, 80.0925, 67.2032]),
            (Rgb::new(0, 255, 0), [87.7347, -86.1827, 83.1793]),
            (Rgb::new(0, 0, 255), [32.2970, 79.1875, -107.8602]),
            (Rgb::new(255, 255, 255), [100.0, 0.0, 0.0]),
            (Rgb::new(0, 0, 0), [0.0, 0.0, 0.0]),
        ];
        for (rgb, [l, a, b]) in cases {
            let lab = rgb.to_lab();
            assert!(
                (lab.l - l).abs() < 0.01 && (lab.a - a).abs() < 0.01 && (lab.b - b).abs() < 0.01,
                "{rgb}: got {lab:?}"
            );
        }
    }

    #[test]
    fn mid_grey_has_neutral_chroma() {
        let lab = Rgb::new(128, 128, 128).to_lab();
        assert!((lab.l - 53.585).abs() < 0.01, "{lab:?}");
        assert!(lab.a.abs() < 0.01 && lab.b.abs() < 0.01, "{lab:?}");
    }

    #[test]
    fn hex_round_trips_through_display() {
        let c = Rgb::new(0x0B, 0x2F, 0x84);
        assert_eq!(c.to_string(), "#0B2F84");
        assert_eq!(Rgb::parse_hex(&c.to_string()), Some(c));
    }
}
