//! Colour conversion utilities for VSF colour types
//!
//! **All colours in VSF default to VSF RGB colourspace.**
//!
//! # VSF RGB Colourspace
//!
//! VSF RGB is a spectral-based colourspace defined by wavelengths, not chromaticity coordinates:
//! - **Primaries**: R=703nm, G=523nm, B=462nm (monochromatic spectral lines)
//! - **White point**: Illuminant E (equal energy), NOT D65
//! - **Gamma**: 2 (simple square/sqrt operations)
//!
//! Named shortcuts (rr, rn, rb, rc, rj, ry, etc.) are convenient colours for human use.
//!
//! # Why No CIE 1931 XYZ Support?
//!
//! **VSF does NOT provide conversions to/from CIE 1931 XYZ or xy chromaticity coordinates.**
//!
//! This is intentional. Here's why:
//!
//! ## The Problem with CIE 1931
//!
//! CIE 1931 XYZ tristimulus values and xy chromaticity coordinates are based on colour matching experiments from the 1920s performed on approximately 17 observers. These empirical measurements have known flaws and introduce accumulated errors thru multiple transformation steps:
//!
//! 0. Start with wavelengths (usually not - typically xy coordinates or existing RGB definitions)
//! 1. Calculate XYZ using 1931 observer functions
//! 2. Derive xy chromaticity coordinates
//! 3. Define primaries in xy space
//! 4. Build transformation matrices
//!
//! Each step accumulates error and makes assumptions about human perception.
//!
//! ## VSF Uses Modern Colourimetry
//!
//! VSF RGB is based on the **Stockman & Sharpe 2000 cone fundamentals** (10° observer),
//! which represent current understanding of human colour vision. All transformations go thru
//! **LMS cone response space**, not XYZ.
//!
//! Conversion path: `Other colourspace → LMS (2000 10°) → VSF RGB`
//!
//! This approach:
//! - Uses physiologically accurate cone fundamentals
//! - Avoids accumulated transformation errors
//! - Provides exact reproducibility (wavelengths don't lie, xy coordinates do)
//! - Eliminates dependency on flawed 1931 data
//!
//! ## Supported Colourspaces
//!
//! We support specific legacy standards only where necessary for compatibility:
//!
//! - **sRGB / Rec.709**: Same primaries, different transfer functions (piecewise sRGB vs. Rec.709 OETF)
//!   - These use 1931-derived xy coordinates (we're stuck with them for compatibility)
//!   - Conversion goes thru LMS, never directly thru XYZ
//!
//! - **Rec.2020 / BT.2020**: Uses their WAVELENGTH specification (630nm, 532nm, 467nm)
//!   - We IGNORE their xy coordinates (which contradict their wavelength spec!)
//!   - Spectral primaries likely chosen for convenience, not deep understanding (532nm green is suspiciously round)
//!   - Direct spectral-to-spectral conversion: 703/523/462nm → 630/532/467nm
//!   - Uses proper D65 spectral power distribution, not xy-derived "D65"
//!   - Transfer function: Rec.709 OETF for encoding
//!
//! ## If You Need XYZ...
//!
//! If you absolutely must have XYZ values:
//! 0. Convert VSF RGB → sRGB using `to_srgb_linear()`
//! 1. Use your own sRGB→XYZ matrix (whichever vintage you prefer)
//! 2. Accept that different 1931 implementations will give different results
//!
//! We won't provide XYZ conversion because:
//! - Our LMS is Stockman & Sharpe 2000-based, so our derived XYZ ≠ CIE 1931 XYZ
//! - Users would assume it's "standard" XYZ and get confused
//! - Wavelengths are the specification; everything else is derived
//!
//! # Provided Conversions
//!
//! Between VSF colour formats:
//! - Named shortcuts ↔ RGB/RGBA
//! - Packed formats ↔ RGB/RGBA
//! - RGB ↔ Greyscale (using VSF RGB photopic luminance)
//! - RGB ↔ RGBA (add/remove alpha)
//! - Bit depth conversions (8-bit ↔ 16-bit ↔ float)
//!
//! To/from other colourspaces:
//! - sRGB ↔ VSF RGB (via LMS, with proper sRGB piecewise transfer function)
//! - Rec.709 ↔ VSF RGB (via LMS, with proper Rec.709 OETF/EOTF)
//! - Rec.2020 ↔ VSF RGB (spectral wavelength conversion, ignoring xy coordinates)

use crate::colour::{
    LMS2PHOTOPIC,
    REC20202VSF_RGB,
    VSF_RGB2LMS,
    VSF_RGB2REC2020,
    // TODO: Add Rec.709 matrices
    // REC7092VSF_RGB, VSF_RGB2REC709,
};
use crate::types::VsfType;

/// Trait for colour value types that can be converted to/from linear
///
/// **Convention**:
/// - Floats (f32) are ALWAYS linear (no gamma encoding)
/// - Integers (u8, u16) are ALWAYS gamma-encoded (sRGB/Rec.709 EOTF applied)
pub trait ColourValue: Copy {
    /// Convert from gamma-encoded value to linear (0-1 range)
    /// For f32: pass thru (already linear)
    /// For integers: apply EOTF and normalize to 0-1
    fn to_linear_srgb(self) -> f32;

    /// Convert from linear (0-1 range) to gamma-encoded value
    /// For f32: pass thru (stay linear)
    /// For integers: apply OETF and quantize
    fn from_linear_srgb(linear: f32) -> Self;

    /// Convert from gamma-encoded value to linear using Rec.709 EOTF
    fn to_linear_rec709(self) -> f32;

    /// Convert from linear to gamma-encoded using Rec.709 OETF
    fn from_linear_rec709(linear: f32) -> Self;
}

impl ColourValue for f32 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        self // Floats are always linear
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        linear // Stay linear
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        self // Floats are always linear
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        linear // Stay linear
    }
}

impl ColourValue for u8 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        linearize_srgb_u8(self)
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        (delinearize_srgb(linear) * 256.0) as u8
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        let normalized = self as f32 / 256.0;
        linearize_bt709(normalized)
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        (encode_bt709(linear) * 256.0) as u8
    }
}

impl ColourValue for u16 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        linearize_srgb_u16(self)
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        (delinearize_srgb(linear) * 65536.0) as u16
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        let normalized = self as f32 / 65536.0;
        linearize_bt709(normalized)
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        (encode_bt709(linear) * 65536.0) as u16
    }
}

/// Invert a 3x3 matrix stored in column-major format
///
/// Matrix format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
pub fn invert_matrix_3x3(m: &[f32; 9]) -> [f32; 9] {
    let d = 1.0
        / (m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]));
    [
        (m[4] * m[8] - m[5] * m[7]) * d,
        (m[2] * m[7] - m[1] * m[8]) * d,
        (m[1] * m[5] - m[2] * m[4]) * d,
        (m[5] * m[6] - m[3] * m[8]) * d,
        (m[0] * m[8] - m[2] * m[6]) * d,
        (m[2] * m[3] - m[0] * m[5]) * d,
        (m[3] * m[7] - m[4] * m[6]) * d,
        (m[1] * m[6] - m[0] * m[7]) * d,
        (m[0] * m[4] - m[1] * m[3]) * d,
    ]
}

/// Apply a 3x3 transformation matrix to a colour
///
/// Matrix is in column-major format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
///
/// This means:
/// - out[0] = colour[0] * cmx[0] + colour[1] * cmx[1] + colour[2] * cmx[2]
/// - out[1] = colour[0] * cmx[3] + colour[1] * cmx[4] + colour[2] * cmx[5]
/// - out[2] = colour[0] * cmx[6] + colour[1] * cmx[7] + colour[2] * cmx[8]
pub fn apply_matrix_3x3(cmx: &[f32], colour: &[f32; 3]) -> [f32; 3] {
    [
        colour[0] * cmx[0] + colour[1] * cmx[1] + colour[2] * cmx[2],
        colour[0] * cmx[3] + colour[1] * cmx[4] + colour[2] * cmx[5],
        colour[0] * cmx[6] + colour[1] * cmx[7] + colour[2] * cmx[8],
    ]
}

/// Multiply two 3x3 matrices (matrix multiplication: result = a * b)
///
/// Matrices are in column-major format
pub fn convert_matrix_3x3(b: &[f32], a: &[f32]) -> [f32; 9] {
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        //
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        //
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}
/// Scale RGB to fit [0,1] gamut while preserving hue/saturation
#[inline]
fn scale_to_gamut(mut r: f32, mut g: f32, mut b: f32) -> (f32, f32, f32) {
    let min = r.min(g).min(b);
    if min < 0.0 {
        r = 1. - r;
        g = 1. - g;
        b = 1. - b;
        let scale = 1. / (1. - min);
        r *= scale;
        g *= scale;
        b *= scale;
        r = 1. - r;
        g = 1. - g;
        b = 1. - b;
    }
    let max = r.max(g).max(b);
    if max > 1. {
        let scale = 1. / max;
        r *= scale;
        g *= scale;
        b *= scale;
    }
    (r, g, b)
}

/// Linear VSF RGB colour (f32 per channel, 0-1 range)
/// Primaries are VSF RGB (703nm, 523nm, 462nm)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbLinear {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Linear RGBA colour (f32 per channel, 0-1 range)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaLinear {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl VsfType {
    /// Convert any colour type to linear RGB (f32, 0-1 range)
    /// All VSF integers are gamma 2 encoded wheras floats are linear
    pub fn to_rgb_linear(&self) -> Option<RgbLinear> {
        match self {
            // Named shortcuts (gamma 2 encoded)
            VsfType::rk => Some(RgbLinear {
                r: 0.,
                g: 0.,
                b: 0.,
            }), // Black
            VsfType::rw => Some(RgbLinear {
                r: 1.,
                g: 1.,
                b: 1.,
            }), // White
            VsfType::rg => Some(RgbLinear {
                r: 0.25,
                g: 0.25,
                b: 0.25,
            }), // Middle grey
            VsfType::rr => Some(RgbLinear {
                r: 1.,
                g: 0.,
                b: 0.,
            }), // Red
            VsfType::rn => Some(RgbLinear {
                r: 0.,
                g: 1.,
                b: 0.,
            }), // Green
            VsfType::rb => Some(RgbLinear {
                r: 0.,
                g: 0.,
                b: 1.,
            }), // Blue
            VsfType::rc => Some(RgbLinear {
                r: 0.,
                g: 1.,
                b: 1.,
            }), // Cyan
            VsfType::rj => Some(RgbLinear {
                r: 1.,
                g: 0.,
                b: 1.,
            }), // Magenta
            VsfType::ry => Some(RgbLinear {
                r: 1.,
                g: 1.,
                b: 0.,
            }), // Yellow
            VsfType::ro => Some(RgbLinear {
                r: 1.,
                g: 0.25,
                b: 0.,
            }), // Orange
            VsfType::rl => Some(RgbLinear {
                r: 0.25,
                g: 1.,
                b: 0.,
            }), // Lime
            VsfType::rq => Some(RgbLinear {
                r: 0.,
                g: 1.,
                b: 0.25,
            }), // Aqua
            VsfType::rv => Some(RgbLinear {
                r: 0.25,
                g: 0.,
                b: 1.,
            }), // Purple

            // Greyscale → RGB (replicate value, linearize)
            VsfType::re(grey) => {
                let lin = linearize_gamma2_u8(*grey);
                Some(RgbLinear {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rx(grey) => {
                let lin = linearize_gamma2_u16(*grey);
                Some(RgbLinear {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rz(grey) => {
                // rz stores linear f32 directly
                Some(RgbLinear {
                    r: *grey,
                    g: *grey,
                    b: *grey,
                })
            }

            // Packed RGB (gamma-encoded, lossy - linearize)
            VsfType::ri(packed) => {
                let (r, g, b) = unpack_rgb_676_linear(*packed);
                Some(RgbLinear { r, g, b })
            }
            VsfType::rp(packed) => {
                let (r, g, b) = unpack_rgb_565_linear(*packed);
                Some(RgbLinear { r, g, b })
            }

            // Standard RGB (gamma-encoded - linearize)
            VsfType::ru([r, g, b]) => Some(RgbLinear {
                r: linearize_gamma2_u8(*r),
                g: linearize_gamma2_u8(*g),
                b: linearize_gamma2_u8(*b),
            }),
            VsfType::rs([r, g, b]) => Some(RgbLinear {
                r: linearize_gamma2_u16(*r),
                g: linearize_gamma2_u16(*g),
                b: linearize_gamma2_u16(*b),
            }),
            VsfType::rf([r, g, b]) => Some(RgbLinear {
                r: *r,
                g: *g,
                b: *b,
            }), // Already linear

            // RGBA → RGB (drop alpha, linearize)
            VsfType::ra([r, g, b, _]) => Some(RgbLinear {
                r: linearize_gamma2_u8(*r),
                g: linearize_gamma2_u8(*g),
                b: linearize_gamma2_u8(*b),
            }),
            VsfType::rt([r, g, b, _]) => Some(RgbLinear {
                r: linearize_gamma2_u16(*r),
                g: linearize_gamma2_u16(*g),
                b: linearize_gamma2_u16(*b),
            }),
            VsfType::rh([r, g, b, _]) => Some(RgbLinear {
                r: *r,
                g: *g,
                b: *b,
            }), // Already linear

            // General format and magic matrix not supported for simple conversion
            _ => None,
        }
    }

    /// Convert any colour type to linear RGBA (f32, 0-1 range)
    pub fn to_rgba_linear(&self) -> Option<RgbaLinear> {
        match self {
            // RGBA formats (gamma-encoded - linearize)
            VsfType::ra([r, g, b, a]) => Some(RgbaLinear {
                r: linearize_gamma2_u8(*r),
                g: linearize_gamma2_u8(*g),
                b: linearize_gamma2_u8(*b),
                a: linearize_gamma2_u8(*a),
            }),
            VsfType::rt([r, g, b, a]) => Some(RgbaLinear {
                r: linearize_gamma2_u16(*r),
                g: linearize_gamma2_u16(*g),
                b: linearize_gamma2_u16(*b),
                a: linearize_gamma2_u16(*a),
            }),
            VsfType::rh([r, g, b, a]) => Some(RgbaLinear {
                r: *r,
                g: *g,
                b: *b,
                a: *a,
            }), // Already linear

            // RGB formats → add opaque alpha
            _ => self.to_rgb_linear().map(|rgb| RgbaLinear {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
                a: 1., // Opaque
            }),
        }
    }

    /// Convert any colour type to 8-bit greyscale
    ///
    /// Uses VSF RGB photopic luminance matrix for RGB → Grey conversions
    pub fn to_grey8(&self) -> Option<u8> {
        match self {
            // Greyscale formats (direct)
            VsfType::re(grey) => Some(*grey),
            VsfType::rx(grey) => Some((*grey >> 8) as u8),
            VsfType::rz(grey) => Some(delinearize_gamma2_u8(*grey)),

            // RGB → Grey: Use VSF RGB photopic luminance (in linear space)
            _ => self.to_rgb_linear().map(|rgb| {
                let lum = vsf_rgb_to_photopic(rgb.r, rgb.g, rgb.b);
                delinearize_gamma2_u8(lum)
            }),
        }
    }

    /// Create colour from sRGB with piecewise transfer function
    ///
    /// Converts from sRGB/Rec.709 colourspace (D65 white) to VSF RGB (E white)
    /// Uses the sRGB piecewise transfer function (linear + gamma 2.4)
    /// Conversion path: sRGB → LMS → VSF RGB
    /// Convert VSF RGB colour to sRGB with piecewise transfer function
    ///
    /// Converts from VSF RGB (E white) to sRGB/Rec.709 (D65 white)
    /// Uses the sRGB piecewise transfer function (linear + gamma 2.4)
    /// Conversion path: VSF RGB → Rec.709 → sRGB encoding
    ///
    /// Returns (r, g, b) as 8-bit sRGB values
    /// Convert VSF RGB to sRGB/Rec.709 linear (f32, 0-1 nominal range)
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    /// Use this for HDR or when you need the raw linear values.
    pub fn to_srgb_linear(&self) -> Option<(f32, f32, f32)> {
        // TODO: Implement Rec.709 matrix
        todo!("Rec.709 conversion not yet implemented")
    }

    /// Convert VSF RGB to BT.2020/Rec.2020 linear (f32, 0-1 nominal range)
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    /// Use this for HDR or when you need the raw linear values.
    pub fn to_rec2020_linear(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear()?;
        use crate::colour::VSF_RGB2REC2020;
        let result = apply_matrix_3x3(&VSF_RGB2REC2020, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to sRGB (u8, gamma-encoded, 0-255 range)
    pub fn to_srgb_u8(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_srgb_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            (delinearize_srgb(r) * 255.) as u8,
            (delinearize_srgb(g) * 255.) as u8,
            (delinearize_srgb(b) * 255.) as u8,
        ))
    }

    /// Convert VSF RGB to sRGB (u16, gamma-encoded, 0-65535 range)
    pub fn to_srgb_u16(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            (delinearize_srgb(r) * 65535.) as u16,
            (delinearize_srgb(g) * 65535.) as u16,
            (delinearize_srgb(b) * 65535.) as u16,
        ))
    }

    /// Convert VSF RGB to Rec.709 (u8, gamma-encoded, studio range 16-235)
    pub fn to_rec709_u8(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_srgb_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            ((encode_bt709(r) * 219.) + 16.) as u8,
            ((encode_bt709(g) * 219.) + 16.) as u8,
            ((encode_bt709(b) * 219.) + 16.) as u8,
        ))
    }

    /// Convert VSF RGB to Rec.709 (u16, gamma-encoded, studio range 4096-60160)
    pub fn to_rec709_u16(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            ((encode_bt709(r) * 56064.) + 4096.) as u16,
            ((encode_bt709(g) * 56064.) + 4096.) as u16,
            ((encode_bt709(b) * 56064.) + 4096.) as u16,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u8, gamma-encoded, studio range 16-235)
    pub fn to_rec2020_u8(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_rec2020_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            ((encode_bt709(r) * 219.) + 16.) as u8,
            ((encode_bt709(g) * 219.) + 16.) as u8,
            ((encode_bt709(b) * 219.) + 16.) as u8,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u16, gamma-encoded, studio range 4096-60160)
    pub fn to_rec2020_u16(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_rec2020_linear()?;
        let (r, g, b) = scale_to_gamut(r, g, b);
        Some((
            ((encode_bt709(r) * 56064.) + 4096.) as u16,
            ((encode_bt709(g) * 56064.) + 4096.) as u16,
            ((encode_bt709(b) * 56064.) + 4096.) as u16,
        ))
    }

    /// Convert from sRGB/Rec.709 to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded), or u16 (gamma-encoded).
    /// **Convention**: Floats are always linear, integers are always gamma-encoded.
    ///
    /// # Examples
    /// ```ignore
    /// // From 8-bit gamma-encoded sRGB
    /// let colour = VsfType::from_srgb(255u8, 128u8, 64u8, ColourFormat::Rf);
    ///
    /// // From linear float sRGB
    /// let colour = VsfType::from_srgb(1.0f32, 0.5f32, 0.25f32, ColourFormat::Rf);
    /// ```
    pub fn from_srgb<T: ColourValue>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        // TODO: Implement Rec.709 matrix
        let _ = (r, g, b, format);
        todo!("Rec.709 conversion not yet implemented")
    }

    /// Convert from Rec.709 to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded), or u16 (gamma-encoded).
    /// **Convention**: Floats are always linear, integers are always gamma-encoded.
    ///
    /// Note: Rec.709 and sRGB have the same primaries, only the transfer function differs.
    pub fn from_rec709<T: ColourValue>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        // TODO: Implement Rec.709 matrix
        let _ = (r, g, b, format);
        todo!("Rec.709 conversion not yet implemented")
    }

    /// Convert from BT.2020/Rec.2020 to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded), or u16 (gamma-encoded).
    /// **Convention**: Floats are always linear, integers are always gamma-encoded with Rec.709 EOTF.
    ///
    /// Note: BT.2020 uses Rec.709 OETF for encoding, but BT.1886 for display.
    /// This function assumes Rec.709 OETF for integer inputs.
    pub fn from_rec2020<T: ColourValue>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        let r_lin = r.to_linear_rec709(); // BT.2020 uses Rec.709 OETF
        let g_lin = g.to_linear_rec709();
        let b_lin = b.to_linear_rec709();

        // BT.2020 → VSF RGB matrix
        let result = apply_matrix_3x3(&REC20202VSF_RGB, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear(vsf_r, vsf_g, vsf_b, format)
    }

    /// Helper: Create VsfType from linear RGB floats
    /// For integer formats: scale to gamut (preserves hue/saturation, prevents white clipping)
    /// For float formats: preserve full range (no clamping)
    fn from_rgb_linear(r: f32, g: f32, b: f32, format: ColourFormat) -> Self {
        match format {
            ColourFormat::Rf => VsfType::rf([r, g, b]),
            ColourFormat::Ru => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::ru([
                    delinearize_gamma2_u8(r),
                    delinearize_gamma2_u8(g),
                    delinearize_gamma2_u8(b),
                ])
            }
            ColourFormat::Rs => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::rs([
                    delinearize_gamma2_u16(r),
                    delinearize_gamma2_u16(g),
                    delinearize_gamma2_u16(b),
                ])
            }
            ColourFormat::Ri => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::ri(pack_rgb_676_linear(r, g, b))
            }
            ColourFormat::Rp => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::rp(pack_rgb_565_linear(r, g, b))
            }
            // Greyscale formats
            ColourFormat::Re => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                let lum = vsf_rgb_to_photopic(r, g, b);
                VsfType::re(delinearize_gamma2_u8(lum))
            }
            ColourFormat::Rx => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                let lum = vsf_rgb_to_photopic(r, g, b);
                VsfType::rx(delinearize_gamma2_u16(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic(r, g, b);
                VsfType::rz(lum)
            }
            // RGBA formats → add opaque alpha
            ColourFormat::Ra => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::ra([
                    delinearize_gamma2_u8(r),
                    delinearize_gamma2_u8(g),
                    delinearize_gamma2_u8(b),
                    255,
                ])
            }
            ColourFormat::Rt => {
                let (r, g, b) = scale_to_gamut(r, g, b);
                VsfType::rt([
                    delinearize_gamma2_u16(r),
                    delinearize_gamma2_u16(g),
                    delinearize_gamma2_u16(b),
                    0xFFFF,
                ])
            }
            ColourFormat::Rh => VsfType::rh([r, g, b, 1.0]),
        }
    }

    /// Create colour from gamma-encoded RGB (8-bit per channel)
    ///
    /// Input RGB is assumed to be gamma-encoded VSF RGB colourspace
    pub fn from_rgb8(r: u8, g: u8, b: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8(r);
        let g_lin = linearize_gamma2_u8(g);
        let b_lin = linearize_gamma2_u8(b);

        match format {
            ColourFormat::Ru => VsfType::ru([r, g, b]),
            ColourFormat::Rs => VsfType::rs([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
            ]),
            ColourFormat::Rf => VsfType::rf([r_lin, g_lin, b_lin]),
            ColourFormat::Ri => VsfType::ri(pack_rgb_676_linear(r_lin, g_lin, b_lin)),
            ColourFormat::Rp => VsfType::rp(pack_rgb_565_linear(r_lin, g_lin, b_lin)),

            // RGB → Greyscale: Use VSF RGB photopic luminance
            ColourFormat::Re => {
                let lum = vsf_rgb_to_photopic(r_lin, g_lin, b_lin);
                VsfType::re(delinearize_gamma2_u8(lum))
            }
            ColourFormat::Rx => {
                let lum = vsf_rgb_to_photopic(r_lin, g_lin, b_lin);
                VsfType::rx(delinearize_gamma2_u16(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic(r_lin, g_lin, b_lin);
                VsfType::rz(lum)
            }

            // RGBA formats → add opaque alpha
            ColourFormat::Ra => VsfType::ra([r, g, b, 255]),
            ColourFormat::Rt => VsfType::rt([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
                0xFFFF,
            ]),
            ColourFormat::Rh => VsfType::rh([r_lin, g_lin, b_lin, 1.0]),
        }
    }

    /// Create colour from gamma-encoded RGBA (8-bit per channel)
    ///
    /// Input RGBA is assumed to be gamma-encoded VSF RGB colourspace
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8(r);
        let g_lin = linearize_gamma2_u8(g);
        let b_lin = linearize_gamma2_u8(b);
        let a_lin = linearize_gamma2_u8(a);

        match format {
            ColourFormat::Ra => VsfType::ra([r, g, b, a]),
            ColourFormat::Rt => VsfType::rt([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
                (a as u16) << 8 | a as u16,
            ]),
            ColourFormat::Rh => VsfType::rh([r_lin, g_lin, b_lin, a_lin]),
            // For RGB-only formats, ignore alpha
            _ => Self::from_rgb8(r, g, b, format),
        }
    }

    /// Convert this colour to any other format
    pub fn convert_colour(&self, target: ColourFormat) -> Option<Self> {
        // Get as linear RGBA (most general representation)
        let rgba = self.to_rgba_linear()?;

        // Convert linear back to gamma u8 for from_rgba8 (which expects gamma input)
        let r_gamma = delinearize_gamma2_u8(rgba.r);
        let g_gamma = delinearize_gamma2_u8(rgba.g);
        let b_gamma = delinearize_gamma2_u8(rgba.b);
        let a_gamma = delinearize_gamma2_u8(rgba.a);

        // Convert to target format
        Some(Self::from_rgba8(r_gamma, g_gamma, b_gamma, a_gamma, target))
    }
}

/// Target colour format for conversions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColourFormat {
    // Greyscale
    Re, // 8-bit
    Rx, // 16-bit
    Rz, // float

    // Packed RGB
    Ri, // 6×7×6
    Rp, // 5-6-5

    // Standard RGB
    Ru, // 8-bit
    Rs, // 16-bit
    Rf, // float

    // Standard RGBA
    Ra, // 8-bit
    Rt, // 16-bit
    Rh, // float
}

/// Unpack 6×7×6 RGB from single byte to linear f32
fn unpack_rgb_676_linear(packed: u8) -> (f32, f32, f32) {
    let b = packed % 6;
    let temp = packed / 6;
    let g = temp % 7;
    let r = temp / 7;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = r as f32 / 5.;
    let g_gamma = g as f32 / 6.;
    let b_gamma = b as f32 / 5.;

    (
        linearize_gamma2(r_gamma),
        linearize_gamma2(g_gamma),
        linearize_gamma2(b_gamma),
    )
}

/// Pack linear RGB into 6×7×6 format (single byte)
fn pack_rgb_676_linear(r: f32, g: f32, b: f32) -> u8 {
    // Delinearize to gamma, then quantize
    let r_gamma = delinearize_gamma2(r);
    let g_gamma = delinearize_gamma2(g);
    let b_gamma = delinearize_gamma2(b);

    let r6 = (r_gamma * 5.).min(5.) as u8;
    let g7 = (g_gamma * 6.).min(6.) as u8;
    let b6 = (b_gamma * 5.).min(5.) as u8;

    ((r6 * 7) + g7) * 6 + b6
}

/// Unpack 5-6-5 RGB from u16 to linear f32
fn unpack_rgb_565_linear(packed: u16) -> (f32, f32, f32) {
    let r5 = (packed >> 11) & 0x1F;
    let g6 = (packed >> 5) & 0x3F;
    let b5 = packed & 0x1F;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = r5 as f32 / 32.;
    let g_gamma = g6 as f32 / 64.;
    let b_gamma = b5 as f32 / 32.;

    (
        linearize_gamma2(r_gamma),
        linearize_gamma2(g_gamma),
        linearize_gamma2(b_gamma),
    )
}

/// Pack linear RGB into 5-6-5 format (u16)
fn pack_rgb_565_linear(r: f32, g: f32, b: f32) -> u16 {
    // Delinearize to gamma, then quantize
    let r_gamma = delinearize_gamma2(r);
    let g_gamma = delinearize_gamma2(g);
    let b_gamma = delinearize_gamma2(b);

    let r5 = (r_gamma.min(1.) * 32.) as u16;
    let g6 = (g_gamma.min(1.) * 64.) as u16;
    let b5 = (b_gamma.min(1.) * 32.) as u16;

    (r5 << 11) | (g6 << 5) | b5
}

// ==================== VSF RGB PHOTOPIC LUMINANCE ====================
// Convert VSF RGB to photopic luminance (perceptual brightness)
// Uses VSF RGB → LMS → Photopic transformation

/// Convert linear VSF RGB to photopic luminance (0-1 range)
///
/// This performs colourimetric conversion:
/// 1. VSF RGB → LMS (cone responses)
/// 2. LMS → Photopic luminance (L&M weighted sum)
/// 3. Normalize so E white [1,1,1] → 1
pub fn vsf_rgb_to_photopic(r: f32, g: f32, b: f32) -> f32 {
    // VSF RGB → LMS (matrix in row-major order)
    let l = VSF_RGB2LMS[0] * r + VSF_RGB2LMS[1] * g + VSF_RGB2LMS[2] * b;
    let m = VSF_RGB2LMS[3] * r + VSF_RGB2LMS[4] * g + VSF_RGB2LMS[5] * b;
    let s = VSF_RGB2LMS[6] * r + VSF_RGB2LMS[7] * g + VSF_RGB2LMS[8] * b;

    // LMS → Photopic (raw)
    let photopic_raw = LMS2PHOTOPIC[0] * l + LMS2PHOTOPIC[1] * m + LMS2PHOTOPIC[2] * s;

    // Normalize by white point so [1,1,1] → 1.0
    // White point value (precomputed would be better, but const fn limits)
    let l_white = VSF_RGB2LMS[0] + VSF_RGB2LMS[1] + VSF_RGB2LMS[2];
    let m_white = VSF_RGB2LMS[3] + VSF_RGB2LMS[4] + VSF_RGB2LMS[5];
    let s_white = VSF_RGB2LMS[6] + VSF_RGB2LMS[7] + VSF_RGB2LMS[8];
    let white_photopic =
        LMS2PHOTOPIC[0] * l_white + LMS2PHOTOPIC[1] * m_white + LMS2PHOTOPIC[2] * s_white;

    photopic_raw / white_photopic
}

// ==================== GAMMA 2 FUNCTIONS ====================
// VSF RGB uses gamma 2 by default (simple sqrt/square operations)

/// Linearize a gamma 2 encoded value (0-1 range)
///
/// Converts from gamma-encoded to linear light. For VSF RGB gamma 2,
/// this is simply the square root operation.
pub fn linearize_gamma2(encoded: f32) -> f32 {
    encoded.sqrt()
}

/// Delinearize a linear value to gamma 2 (0-1 range)
///
/// Converts from linear light to gamma-encoded. For VSF RGB gamma 2,
/// this is simply squaring the value.
pub fn delinearize_gamma2(linear: f32) -> f32 {
    linear * linear
}

/// Linearize an 8-bit gamma 2 encoded value
///
/// Converts 0-255 range to linear 0-1
pub fn linearize_gamma2_u8(encoded: u8) -> f32 {
    let normalized = encoded as f32 / 256.;
    linearize_gamma2(normalized)
}

/// Delinearize a linear value to 8-bit gamma 2
///
/// Converts linear 0-1 to 0-255 range
pub fn delinearize_gamma2_u8(linear: f32) -> u8 {
    let encoded = delinearize_gamma2(linear);
    (encoded * 256.) as u8
}

/// Linearize a 16-bit gamma 2 encoded value
///
/// Converts 0-65535 range to linear 0-1
pub fn linearize_gamma2_u16(encoded: u16) -> f32 {
    let normalized = encoded as f32 / 65536.;
    linearize_gamma2(normalized)
}

/// Delinearize a linear value to 16-bit gamma 2
///
/// Converts linear 0.0-1.0 to 0-65535 range
pub fn delinearize_gamma2_u16(linear: f32) -> u16 {
    let encoded = delinearize_gamma2(linear);
    (encoded * 65536.) as u16
}

/// Linearize an RGB triple (8-bit per channel)
pub fn linearize_gamma2_rgb(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    (
        linearize_gamma2_u8(r),
        linearize_gamma2_u8(g),
        linearize_gamma2_u8(b),
    )
}

/// Delinearize a linear RGB triple to 8-bit
pub fn delinearize_gamma2_rgb(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    (
        delinearize_gamma2_u8(r),
        delinearize_gamma2_u8(g),
        delinearize_gamma2_u8(b),
    )
}

// ==================== GAMMA 2.4 FUNCTIONS ====================
// Used by BT.2020, HDR content, etc.

/// Linearize a gamma 2.4 encoded value (0-1 range)
pub fn linearize_gamma24(encoded: f32) -> f32 {
    encoded.powf(2.4)
}

/// Delinearize a linear value to gamma 2.4 (0-1 range)
pub fn delinearize_gamma24(linear: f32) -> f32 {
    linear.powf(1. / 2.4)
}

/// Linearize an 8-bit gamma 2.4 encoded value
pub fn linearize_gamma24_u8(encoded: u8) -> f32 {
    let normalized = encoded as f32 / 256.;
    linearize_gamma24(normalized)
}

/// Delinearize a linear value to 8-bit gamma 2.4
pub fn delinearize_gamma24_u8(linear: f32) -> u8 {
    let encoded = delinearize_gamma24(linear);
    (encoded * 256.) as u8
}

// ================= Rec.709 & sRGB PIECEWISE TRANSFER FUNCTIONS =================
// The sRGB/Rec.709 transfer function is piecewise:
// - Linear segment near black to avoid infinite slope at 0
// - Gamma 2.4 for the rest (NOT gamma 2.2 - common misconception!)
//
// This is the "correct" sRGB, but slow due to the branch and powf().
// These functions exist for accurate sRGB/Rec.709 conversions.

const BT709_LINEAR_THRESHOLD: f32 = 0.018;
const BT709_LINEAR_COEFF: f32 = 4.5;
const BT709_GAMMA_A: f32 = 0.099;
const BT709_GAMMA_DIVISOR: f32 = 1.099;
const BT709_GAMMA_EXPONENT: f32 = 0.45; // Inverse is ~2.222

/// Linearize a BT.709-encoded value (0-1 range)
///
/// BT.709 OETF (encoding, camera):
/// - Linear: V = 4.5 * L for L < 0.018
/// - Gamma: V = 1.099 * L^0.45 - 0.099 for L >= 0.018
#[inline]
pub fn linearize_bt709(encoded: f32) -> f32 {
    // This is the inverse (decoding)
    if encoded < 0.081 {
        // 4.5 * 0.018 = 0.081
        encoded / BT709_LINEAR_COEFF
    } else {
        ((encoded + BT709_GAMMA_A) / BT709_GAMMA_DIVISOR).powf(1.0 / BT709_GAMMA_EXPONENT)
    }
}

/// Encode a linear value to BT.709
#[inline]
pub fn encode_bt709(linear: f32) -> f32 {
    if linear < BT709_LINEAR_THRESHOLD {
        linear * BT709_LINEAR_COEFF
    } else {
        BT709_GAMMA_DIVISOR * linear.powf(BT709_GAMMA_EXPONENT) - BT709_GAMMA_A
    }
}

const SRGB_LINEAR_THRESHOLD: f32 = 0.0031308;
const SRGB_LINEAR_COEFF: f32 = 12.92;
const SRGB_GAMMA_A: f32 = 0.055;
const SRGB_GAMMA_DIVISOR: f32 = 1.055;
const SRGB_GAMMA_EXPONENT: f32 = 2.4;

/// Linearize an sRGB-encoded value (0-1 range) using the proper piecewise function
///
/// sRGB transfer function:
/// - Linear: `C_srgb / 12.92` for `C_srgb <= 0.04045`
/// - Gamma: `((C_srgb + 0.055) / 1.055)^2.4` for `C_srgb > 0.04045`
///
/// Note: The threshold 0.04045 is the encoded value; the linear threshold is 0.0031308
#[inline]
pub fn linearize_srgb(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / SRGB_LINEAR_COEFF
    } else {
        ((encoded + SRGB_GAMMA_A) / SRGB_GAMMA_DIVISOR).powf(SRGB_GAMMA_EXPONENT)
    }
}

/// Delinearize a linear value to sRGB-encoded (0-1 range) using the proper piecewise function
///
/// Inverse sRGB transfer function:
/// - Linear: `C_linear * 12.92` for `C_linear <= 0.0031308`
/// - Gamma: `1.055 * C_linear^(1/2.4) - 0.055` for `C_linear > 0.0031308`
#[inline]
pub fn delinearize_srgb(linear: f32) -> f32 {
    if linear <= SRGB_LINEAR_THRESHOLD {
        linear * SRGB_LINEAR_COEFF
    } else {
        SRGB_GAMMA_DIVISOR * linear.powf(1.0 / SRGB_GAMMA_EXPONENT) - SRGB_GAMMA_A
    }
}

/// Linearize an 8-bit sRGB-encoded value
///
/// Converts 0-255 sRGB to linear 0-1 using proper piecewise function
#[inline]
pub fn linearize_srgb_u8(encoded: u8) -> f32 {
    let normalized = encoded as f32 / 256.0;
    linearize_srgb(normalized)
}

/// Delinearize a linear value to 8-bit sRGB
///
/// Converts linear 0-1 to 0-255 sRGB using proper piecewise function
/// Rust's as u8 handles out-of-bounds naturally: >255 → 255, <0 → 0, NaN → 0
#[inline]
pub fn delinearize_srgb_u8(linear: f32) -> u8 {
    let encoded = delinearize_srgb(linear);
    (encoded * 256.0) as u8
}

/// Linearize a 16-bit sRGB-encoded value
///
/// Converts 0-65535 sRGB to linear 0-1 using proper piecewise function
#[inline]
pub fn linearize_srgb_u16(encoded: u16) -> f32 {
    let normalized = encoded as f32 / 65536.0;
    linearize_srgb(normalized)
}

/// Delinearize a linear value to 16-bit sRGB
///
/// Converts linear 0-1 to 0-65535 sRGB using proper piecewise function
/// Rust's as u16 handles out-of-bounds naturally: >65535 → 65535, <0 → 0, NaN → 0
#[inline]
pub fn delinearize_srgb_u16(linear: f32) -> u16 {
    let encoded = delinearize_srgb(linear);
    (encoded * 65536.0) as u16
}

/// Linearize an sRGB RGB triple (8-bit per channel)
#[inline]
pub fn linearize_srgb_rgb(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    (
        linearize_srgb_u8(r),
        linearize_srgb_u8(g),
        linearize_srgb_u8(b),
    )
}

/// Delinearize a linear RGB triple to 8-bit sRGB
#[inline]
pub fn delinearize_srgb_rgb(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    (
        delinearize_srgb_u8(r),
        delinearize_srgb_u8(g),
        delinearize_srgb_u8(b),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_shortcuts_to_rgb() {
        assert_eq!(
            VsfType::rk.to_rgb_linear(),
            Some(RgbLinear {
                r: 0.,
                g: 0.,
                b: 0.
            })
        );
        assert_eq!(
            VsfType::rw.to_rgb_linear(),
            Some(RgbLinear {
                r: 1.,
                g: 1.,
                b: 1.
            })
        );
        assert_eq!(
            VsfType::rr.to_rgb_linear(),
            Some(RgbLinear {
                r: 1.,
                g: 0.,
                b: 0.
            })
        );
    }

    #[test]
    fn test_rgb_to_greyscale() {
        // Test basic greyscale conversion
        let white = VsfType::ru([255, 255, 255]);
        let black = VsfType::ru([0, 0, 0]);
        let grey = VsfType::ru([128, 128, 128]);

        assert_eq!(white.to_grey8(), Some(255));
        assert_eq!(black.to_grey8(), Some(0));
        assert!(grey.to_grey8().unwrap() > 100 && grey.to_grey8().unwrap() < 150);
    }

    #[test]
    fn test_srgb_piecewise_correctness() {
        // Test the piecewise function at the threshold
        // Linear segment: C_srgb <= 0.04045 → C_linear = C_srgb / 12.92
        // Gamma segment: C_srgb > 0.04045 → C_linear = ((C_srgb + 0.055) / 1.055)^2.4

        // Test black (always linear segment)
        assert_eq!(linearize_srgb(0.0), 0.0);
        assert_eq!(delinearize_srgb(0.0), 0.0);

        // Test white
        let white_linear = linearize_srgb(1.0);
        assert!((white_linear - 1.0).abs() < 0.001);

        // Test roundtrip at threshold boundary
        let threshold_encoded = 0.04045;
        let linear = linearize_srgb(threshold_encoded);
        let roundtrip = delinearize_srgb(linear);
        assert!((roundtrip - threshold_encoded).abs() < 0.001);

        // Test u8 roundtrip for common values
        let test_values = [0u8, 10, 50, 128, 200, 255];
        for &val in &test_values {
            let linear = linearize_srgb_u8(val);
            let roundtrip = delinearize_srgb_u8(linear);
            // Allow ±1 error due to rounding
            assert!(
                (roundtrip as i16 - val as i16).abs() <= 1,
                "Failed roundtrip for {}: got {}",
                val,
                roundtrip
            );
        }
    }

    #[test]
    fn test_srgb_vsf_rgb_conversion() {
        // Test sRGB → VSF RGB → sRGB roundtrip
        // Start with sRGB values to avoid gamma mismatch
        let test_colours = [
            (0u8, 0u8, 0u8),       // Black
            (255u8, 255u8, 255u8), // White
            (128u8, 128u8, 128u8), // Middle grey
            (255u8, 0u8, 0u8),     // Red
            (0u8, 255u8, 0u8),     // Green
            (0u8, 0u8, 255u8),     // Blue
            (200u8, 100u8, 50u8),  // Random colour
        ];

        for &(r_in, g_in, b_in) in &test_colours {
            // sRGB → VSF RGB (linear storage)
            let vsf = VsfType::from_srgb(r_in, g_in, b_in, ColourFormat::Rf);

            // VSF RGB → sRGB
            if let Some((r_out, g_out, b_out)) = vsf.to_srgb_u8() {
                // Roundtrip should be very close (within ±2 due to rounding and f32 precision)
                let r_diff = (r_out as i16 - r_in as i16).abs();
                let g_diff = (g_out as i16 - g_in as i16).abs();
                let b_diff = (b_out as i16 - b_in as i16).abs();

                assert!(
                    r_diff <= 2 && g_diff <= 2 && b_diff <= 2,
                    "sRGB roundtrip failed for ({}, {}, {}): got ({}, {}, {}), diffs: r={}, g={}, b={}",
                    r_in, g_in, b_in, r_out, g_out, b_out, r_diff, g_diff, b_diff
                );
            } else {
                panic!("to_srgb() returned None for valid colour");
            }
        }
    }

    #[test]
    fn test_packed_rgb_roundtrip() {
        let original_u8 = (130u8, 60u8, 200u8);
        let packed = VsfType::from_rgb8(
            original_u8.0,
            original_u8.1,
            original_u8.2,
            ColourFormat::Ri,
        );
        let unpacked = packed.to_rgb_linear().unwrap();

        // Convert back to u8 for comparison
        let unpacked_u8 = (
            delinearize_gamma2_u8(unpacked.r),
            delinearize_gamma2_u8(unpacked.g),
            delinearize_gamma2_u8(unpacked.b),
        );

        // Should be close (lossy compression)
        assert!((unpacked_u8.0 as i16 - original_u8.0 as i16).abs() < 50);
        assert!((unpacked_u8.1 as i16 - original_u8.1 as i16).abs() < 40);
        assert!((unpacked_u8.2 as i16 - original_u8.2 as i16).abs() < 50);
    }

    #[test]
    fn test_colour_conversion() {
        // Red as ru, convert to ra
        let red_rgb = VsfType::ru([255, 0, 0]);
        let red_rgba = red_rgb.convert_colour(ColourFormat::Ra).unwrap();

        assert_eq!(
            red_rgba.to_rgba_linear(),
            Some(RgbaLinear {
                r: 1.,
                g: 0.,
                b: 0.,
                a: 1.
            })
        );
    }

    #[test]
    fn test_gamma2_roundtrip() {
        let values = [0., 0.25, 0.5, 0.75, 1.];
        for &v in &values {
            let delinearized = delinearize_gamma2(v);
            let linearized = linearize_gamma2(delinearized);
            assert!(linearized == v);
        }
    }

    #[test]
    fn test_gamma2_u8_roundtrip() {
        let values = [0u8, 64, 128, 192, 255];
        for &v in &values {
            let linearized = linearize_gamma2_u8(v);
            let delinearized = delinearize_gamma2_u8(linearized);
            assert!(delinearized as i16 == v as i16);
        }
    }

    #[test]
    fn test_bt2020_conversion() {
        // Test BT.2020 white (gamma 2.4) → VSF RGB
        let bt2020_white = VsfType::from_rec2020(255u8, 255u8, 255u8, ColourFormat::Ru);
        let vsf_white = bt2020_white.to_rgb_linear().unwrap();

        // D65 white → E white adaptation results in slightly shifted values
        // This is expected - D65 is bluer than E, so red gets reduced
        assert!(vsf_white.r > 0.6 && vsf_white.r < 0.7, "r={}", vsf_white.r);
        assert!(vsf_white.g > 0.9 && vsf_white.g < 1.0, "g={}", vsf_white.g);
        assert!(vsf_white.b > 0.9 && vsf_white.b < 1.0, "b={}", vsf_white.b);

        // Test BT.2020 black → VSF RGB black
        let bt2020_black = VsfType::from_rec2020(0u8, 0u8, 0u8, ColourFormat::Ru);
        let vsf_black = bt2020_black.to_rgb_linear().unwrap();

        assert!(vsf_black.r < 0.1);
        assert!(vsf_black.g < 0.1);
        assert!(vsf_black.b < 0.1);

        // Test BT.2020 primary red
        let bt2020_red = VsfType::from_rec2020(255u8, 0u8, 0u8, ColourFormat::Ru);
        let vsf_red = bt2020_red.to_rgb_linear().unwrap();
        // Red should stay mostly red
        assert!(vsf_red.r > vsf_red.g && vsf_red.r > vsf_red.b);
    }

    #[test]
    fn test_gamma24_roundtrip() {
        let values = [0u8, 64, 128, 192, 255];
        for &v in &values {
            let linearized = linearize_gamma24_u8(v);
            let delinearized = delinearize_gamma24_u8(linearized);
            // Gamma 2.4 may have slightly more rounding error than gamma 2.0
            assert!((delinearized as i16 - v as i16).abs() <= 1);
        }
    }
}
