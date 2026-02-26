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
//! # Why is CIE 1931 XYZ Legacy?
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
//! VSF RGB is based on the **Stockman & Sharpe 2000 cone fundamentals** (10° observer), which represent current understanding of human colour vision. All transformations go thru **LMS cone response space**, not XYZ.
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
//!   - These use 1931-derived xy coordinates
//!
//! - **Rec.2020 / BT.2020**: Uses their WAVELENGTH specification (630nm, 532nm, 467nm)
//!   - We IGNORE their xy coordinates (which contradict their wavelength spec!)
//!   - Spectral primaries likely chosen for convenience, not deep understanding (532nm green is suspiciously DPSS)
//!   - Direct spectral-to-spectral conversion: 703/523/462nm → 630/532/467nm
//!   - Uses proper D65 spectral power distribution, not xy-derived "D65"
//!   - Transfer function: Rec.709 OETF for encoding

#[allow(deprecated)] // Need legacy functions for backward compatibility conversions
use crate::colour::legacy::{
    delinearize_srgb, encode_bt709, linearize_bt709, linearize_srgb_u16, linearize_srgb_u8,
};
use crate::colour::rec2020::REC2020_2VSF_RGB;
use crate::colour::{LMS2PHOTOPIC, VSF_RGB2LMS};
use crate::types::VsfType;
#[cfg(feature = "spirix")]
use spirix::ScalarF4E4 as S44;
#[cfg(feature = "spirix")]
use spirix::sf;
/// Trait for colour value types that can be converted to/from linear
///
/// **Convention**:
/// - Floats (f32, S44) are ALWAYS linear (no gamma encoding)
/// - Integers (u8, u16) are ALWAYS gamma-encoded (various EOTF applied)
///
/// The `Linear` type parameter determines the target float type (f32 or S44).
/// Integers convert directly to either float type without roundtripping.
pub trait ColourValue<Linear: Copy>: Copy {
    /// Convert from gamma-encoded value to linear (0-1 range)
    /// For integers: apply EOTF and normalize to 0-1
    fn to_linear_srgb(self) -> Linear;

    /// Convert from linear (0-1 range) to gamma-encoded value
    /// For floats: pass thru (stay linear)
    /// For integers: apply OETF and quantize
    fn from_linear_srgb(linear: Linear) -> Self;

    /// Convert from gamma-encoded value to linear using Rec.709 EOTF
    fn to_linear_rec709(self) -> Linear;

    /// Convert from linear to gamma-encoded using Rec.709 OETF
    fn from_linear_rec709(linear: Linear) -> Self;

    /// Convert from gamma-encoded value to linear using Rec.2020 EOTF
    /// (Rec.2020 uses Rec.709 transfer function)
    fn to_linear_rec2020(self) -> Linear {
        self.to_linear_rec709()
    }

    /// Convert from linear to gamma-encoded using Rec.2020 OETF
    /// (Rec.2020 uses Rec.709 transfer function)
    fn from_linear_rec2020(linear: Linear) -> Self {
        Self::from_linear_rec709(linear)
    }

    /// Convert from gamma 2.2 encoded (Adobe RGB) to linear
    fn to_linear_adobe_rgb(self) -> Linear;

    /// Convert from linear to gamma 2.2 encoded (Adobe RGB)
    fn from_linear_adobe_rgb(linear: Linear) -> Self;

    /// Convert from gamma 2 encoded (VSF RGB native) to linear
    fn to_linear_gamma2(self) -> Linear;

    /// Convert from linear to gamma 2 encoded (VSF RGB native)
    fn from_linear_gamma2(linear: Linear) -> Self;
}

impl ColourValue<f32> for f32 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        self
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        linear
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        self
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        linear
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> f32 {
        self
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: f32) -> Self {
        linear
    }

    #[inline]
    fn to_linear_gamma2(self) -> f32 {
        self
    }

    #[inline]
    fn from_linear_gamma2(linear: f32) -> Self {
        linear
    }
}

#[cfg(feature = "spirix")]
impl ColourValue<S44> for S44 {
    #[inline]
    fn to_linear_srgb(self) -> S44 {
        self
    }

    #[inline]
    fn from_linear_srgb(linear: S44) -> Self {
        linear
    }

    #[inline]
    fn to_linear_rec709(self) -> S44 {
        self
    }

    #[inline]
    fn from_linear_rec709(linear: S44) -> Self {
        linear
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> S44 {
        self
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: S44) -> Self {
        linear
    }

    #[inline]
    fn to_linear_gamma2(self) -> S44 {
        self
    }

    #[inline]
    fn from_linear_gamma2(linear: S44) -> Self {
        linear
    }
}

impl ColourValue<f32> for u8 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        linearize_srgb_u8(self)
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        (delinearize_srgb(linear) * 255.).round() as u8
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        // Rec.709 integers use studio range: [16, 235]
        // Denormalize to [0, 1] then apply EOTF
        let normalized = (self as f32 - 16.) / 219.; // 235-16=219
        linearize_bt709(normalized.clamp(0., 1.))
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        // Apply OETF then map to studio range [16, 235]
        ((encode_bt709(linear) * 219.).round() as u8).min(219) + 16
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> f32 {
        use crate::colour::legacy::transfer::adobe_rgb_eotf;
        let normalized = self as f32 / 255.;
        adobe_rgb_eotf(normalized)
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: f32) -> Self {
        use crate::colour::legacy::transfer::adobe_rgb_oetf;
        (adobe_rgb_oetf(linear) * 255.).round() as u8
    }

    #[inline]
    fn to_linear_gamma2(self) -> f32 {
        linearize_gamma2_u8_f32(self)
    }

    #[inline]
    fn from_linear_gamma2(linear: f32) -> Self {
        delinearize_gamma2_u8_f32(linear)
    }
}

#[cfg(feature = "spirix")]
// Shared colour transfer constants (compile-time, no IEEE runtime ops)
mod colour_tf_consts {
    use spirix::ScalarF4E4 as S44;
    use spirix::sf;
    pub const SRGB_EOTF_THRESH: S44 = sf!(0.04045);
    pub const SRGB_OETF_THRESH: S44 = sf!(0.0031308);
    pub const SRGB_LINEAR_SLOPE: S44 = sf!(12.92);
    pub const SRGB_GAMMA: S44 = sf!(2.4);
    pub const SRGB_INV_GAMMA: S44 = sf!(1.0 / 2.4);
    pub const SRGB_A: S44 = sf!(1.055);
    pub const SRGB_B: S44 = sf!(0.055);
    pub const BT709_EOTF_THRESH: S44 = sf!(0.081);
    pub const BT709_OETF_THRESH: S44 = sf!(0.018);
    pub const BT709_LINEAR_SLOPE: S44 = sf!(4.5);
    pub const BT709_GAMMA: S44 = sf!(0.45);
    pub const BT709_INV_GAMMA: S44 = sf!(1.0 / 0.45);
    pub const BT709_A: S44 = sf!(1.099);
    pub const BT709_B: S44 = sf!(0.099);
    pub const ADOBE_GAMMA: S44 = sf!(2.2);
    pub const ADOBE_INV_GAMMA: S44 = sf!(1.0 / 2.2);
}

#[cfg(feature = "spirix")]
impl ColourValue<S44> for u8 {
    #[inline]
    fn to_linear_srgb(self) -> S44 {
        use colour_tf_consts::*;
        let normalized = S44::from(self) / 255i16;
        // sRGB EOTF
        if normalized <= SRGB_EOTF_THRESH {
            normalized / SRGB_LINEAR_SLOPE
        } else {
            ((normalized + SRGB_B) / SRGB_A).pow(SRGB_GAMMA)
        }
    }

    #[inline]
    fn from_linear_srgb(linear: S44) -> Self {
        use colour_tf_consts::*;
        // sRGB OETF
        let encoded: S44 = if linear <= SRGB_OETF_THRESH {
            linear * SRGB_LINEAR_SLOPE
        } else {
            linear.pow(SRGB_INV_GAMMA) * SRGB_A - SRGB_B
        };
        (encoded * 255i16).round().to_u8()
    }

    #[inline]
    fn to_linear_rec709(self) -> S44 {
        use colour_tf_consts::*;
        // Rec.709 integers use studio range: [16, 235]
        let normalized = (S44::from(self) - 16i16) / 219i16; // 235-16=219
        let normalized = normalized.clamp(0i16, 1i16);
        // Rec.709 EOTF
        if normalized < BT709_EOTF_THRESH {
            normalized / BT709_LINEAR_SLOPE
        } else {
            ((normalized + BT709_B) / BT709_A).pow(BT709_INV_GAMMA)
        }
    }

    #[inline]
    fn from_linear_rec709(linear: S44) -> Self {
        use colour_tf_consts::*;
        // Rec.709 OETF
        let encoded: S44 = if linear < BT709_OETF_THRESH {
            linear * BT709_LINEAR_SLOPE
        } else {
            linear.pow(BT709_GAMMA) * BT709_A - BT709_B
        };
        (encoded * 219i16).round().to_u8() + 16
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> S44 {
        use colour_tf_consts::*;
        let normalized = S44::from(self) / 255i16;
        // Adobe RGB gamma 2.2
        normalized.pow(ADOBE_GAMMA)
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: S44) -> Self {
        use colour_tf_consts::*;
        let encoded = linear.pow(ADOBE_INV_GAMMA);
        (encoded).round().to_u8()
    }

    #[inline]
    fn to_linear_gamma2(self) -> S44 {
        let normalized = S44::from(self) >> 8isize;
        normalized.square()
    }

    #[inline]
    fn from_linear_gamma2(linear: S44) -> Self {
        let encoded = linear.sqrt();
        (encoded << 8isize).to_u8()
    }
}

impl ColourValue<f32> for u16 {
    #[inline]
    fn to_linear_srgb(self) -> f32 {
        linearize_srgb_u16(self)
    }

    #[inline]
    fn from_linear_srgb(linear: f32) -> Self {
        (delinearize_srgb(linear) * 65535.).round() as u16
    }

    #[inline]
    fn to_linear_rec709(self) -> f32 {
        // Rec.709 integers use studio range: [4096, 60160]
        // Denormalize to [0, 1] then apply EOTF
        let normalized = (self as f32 - 4096.) / 56064.; // 60160-4096=56064
        linearize_bt709(normalized.clamp(0., 1.))
    }

    #[inline]
    fn from_linear_rec709(linear: f32) -> Self {
        // Apply OETF then map to studio range [4096, 60160]
        ((encode_bt709(linear) * 56064.).round() as u16).min(56064) + 4096
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> f32 {
        use crate::colour::legacy::transfer::adobe_rgb_eotf;
        let normalized = self as f32 / 65535.;
        adobe_rgb_eotf(normalized)
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: f32) -> Self {
        use crate::colour::legacy::transfer::adobe_rgb_oetf;
        (adobe_rgb_oetf(linear) * 65535.).round() as u16
    }

    #[inline]
    fn to_linear_gamma2(self) -> f32 {
        linearize_gamma2_u16_f32(self)
    }

    #[inline]
    fn from_linear_gamma2(linear: f32) -> Self {
        delinearize_gamma2_u16_f32(linear)
    }
}

#[cfg(feature = "spirix")]
impl ColourValue<S44> for u16 {
    #[inline]
    fn to_linear_srgb(self) -> S44 {
        use colour_tf_consts::*;
        let normalized = S44::from(self) / 65535i32;
        // sRGB EOTF
        if normalized <= SRGB_EOTF_THRESH {
            normalized / SRGB_LINEAR_SLOPE
        } else {
            ((normalized + SRGB_B) / SRGB_A).pow(SRGB_GAMMA)
        }
    }

    #[inline]
    fn from_linear_srgb(linear: S44) -> Self {
        use colour_tf_consts::*;
        // sRGB OETF
        let encoded: S44 = if linear <= SRGB_OETF_THRESH {
            linear * SRGB_LINEAR_SLOPE
        } else {
            linear.pow(SRGB_INV_GAMMA) * SRGB_A - SRGB_B
        };
        (encoded * 65535i32).round().to_u16()
    }

    #[inline]
    fn to_linear_rec709(self) -> S44 {
        use colour_tf_consts::*;
        // Rec.709 integers use studio range: [4096, 60160]
        let normalized = (S44::from(self) - 4096i32) / 56064i32; // 60160-4096=56064
        let normalized = normalized.clamp(0i16, 1i16);
        // Rec.709 EOTF
        if normalized < BT709_EOTF_THRESH {
            normalized / BT709_LINEAR_SLOPE
        } else {
            ((normalized + BT709_B) / BT709_A).pow(BT709_INV_GAMMA)
        }
    }

    #[inline]
    fn from_linear_rec709(linear: S44) -> Self {
        use colour_tf_consts::*;
        // Rec.709 OETF
        let encoded: S44 = if linear < BT709_OETF_THRESH {
            linear * BT709_LINEAR_SLOPE
        } else {
            linear.pow(BT709_GAMMA) * BT709_A - BT709_B
        };
        (encoded * 56064i32).round().to_u16() + 4096
    }

    #[inline]
    fn to_linear_adobe_rgb(self) -> S44 {
        use colour_tf_consts::*;
        let normalized = S44::from(self) / 65535i32;
        // Adobe RGB gamma 2.2
        normalized.pow(ADOBE_GAMMA)
    }

    #[inline]
    fn from_linear_adobe_rgb(linear: S44) -> Self {
        use colour_tf_consts::*;
        let encoded = linear.pow(ADOBE_INV_GAMMA);
        (encoded * 65535i32).round().to_u16()
    }

    #[inline]
    fn to_linear_gamma2(self) -> S44 {
        let normalized = S44::from(self) >> 16isize;
        normalized.square()
    }

    #[inline]
    fn from_linear_gamma2(linear: S44) -> Self {
        let encoded = linear.sqrt();
        (encoded << 16isize).to_u16()
    }
}

/// Invert a 3x3 matrix stored in column-major format (f32 version)
///
/// Matrix format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
pub fn invert_matrix_3x3_f32(m: &[f32; 9]) -> [f32; 9] {
    let d = 1.
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

/// Invert a 3x3 matrix stored in column-major format (S44 version)
///
/// Matrix format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
#[cfg(feature = "spirix")]
pub fn invert_matrix_3x3_s44(m: &[S44; 9]) -> [S44; 9] {
    let d = (m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]))
        .reciprocal();
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

/// Apply a 3x3 transformation matrix to a colour (f32 version)
///
/// Matrix is in column-major format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
///
/// Computes: result = matrix * colour
pub fn apply_matrix_3x3_f32(cmx: &[f32], colour: &[f32; 3]) -> [f32; 3] {
    [
        cmx[0] * colour[0] + cmx[3] * colour[1] + cmx[6] * colour[2], // Row 0
        cmx[1] * colour[0] + cmx[4] * colour[1] + cmx[7] * colour[2], // Row 1
        cmx[2] * colour[0] + cmx[5] * colour[1] + cmx[8] * colour[2], // Row 2
    ]
}

/// Apply a 3x3 transformation matrix to a colour (S44 version)
///
/// Matrix is in column-major format: [col0_r, col0_g, col0_b, col1_r, col1_g, col1_b, col2_r, col2_g, col2_b]
///
/// Computes: result = matrix * colour
#[cfg(feature = "spirix")]
pub fn apply_matrix_3x3_s44(cmx: &[S44], colour: &[S44; 3]) -> [S44; 3] {
    [
        cmx[0] * colour[0] + cmx[3] * colour[1] + cmx[6] * colour[2], // Row 0
        cmx[1] * colour[0] + cmx[4] * colour[1] + cmx[7] * colour[2], // Row 1
        cmx[2] * colour[0] + cmx[5] * colour[1] + cmx[8] * colour[2], // Row 2
    ]
}

/// Multiply two 3×3 matrices: C = A * B (column-major)
///
/// Column j of C = A * column j of B
pub fn convert_matrix_3x3_f32(a: &[f32], b: &[f32]) -> [f32; 9] {
    [
        // Column 0 of result = A * column 0 of B
        a[0] * b[0] + a[3] * b[1] + a[6] * b[2], // C[0]
        a[1] * b[0] + a[4] * b[1] + a[7] * b[2], // C[1]
        a[2] * b[0] + a[5] * b[1] + a[8] * b[2], // C[2]
        // Column 1 of result = A * column 1 of B
        a[0] * b[3] + a[3] * b[4] + a[6] * b[5], // C[3]
        a[1] * b[3] + a[4] * b[4] + a[7] * b[5], // C[4]
        a[2] * b[3] + a[5] * b[4] + a[8] * b[5], // C[5]
        // Column 2 of result = A * column 2 of B
        a[0] * b[6] + a[3] * b[7] + a[6] * b[8], // C[6]
        a[1] * b[6] + a[4] * b[7] + a[7] * b[8], // C[7]
        a[2] * b[6] + a[5] * b[7] + a[8] * b[8], // C[8]
    ]
}

/// Multiply two 3×3 matrices: C = A * B (column-major, S44 version)
///
/// Column j of C = A * column j of B
#[cfg(feature = "spirix")]
pub fn convert_matrix_3x3_s44(a: &[S44], b: &[S44]) -> [S44; 9] {
    [
        // Column 0 of result = A * column 0 of B
        a[0] * b[0] + a[3] * b[1] + a[6] * b[2], // C[0]
        a[1] * b[0] + a[4] * b[1] + a[7] * b[2], // C[1]
        a[2] * b[0] + a[5] * b[1] + a[8] * b[2], // C[2]
        // Column 1 of result = A * column 1 of B
        a[0] * b[3] + a[3] * b[4] + a[6] * b[5], // C[3]
        a[1] * b[3] + a[4] * b[4] + a[7] * b[5], // C[4]
        a[2] * b[3] + a[5] * b[4] + a[8] * b[5], // C[5]
        // Column 2 of result = A * column 2 of B
        a[0] * b[6] + a[3] * b[7] + a[6] * b[8], // C[6]
        a[1] * b[6] + a[4] * b[7] + a[7] * b[8], // C[7]
        a[2] * b[6] + a[5] * b[7] + a[8] * b[8], // C[8]
    ]
}
/// Scale RGB to fit [0,1] gamut while preserving hue/saturation
#[inline]
/// Scale RGB values to fit within [?,1] gamut while preserving hue
///
/// - If min < 0: Assumes you will handle the -'s with the cast (Rust does this when casting to unsigned int)
/// - If max > 1: Scales all values down proportionally
///
/// This preserves the bright hues, whites especially while ensuring displayable colours.
pub fn scale_to_gamut_f32(mut r: f32, mut g: f32, mut b: f32) -> (f32, f32, f32) {
    // let min = r.min(g).min(b);
    // if min < 0. {
    //     r = 1. - r;
    //     g = 1. - g;
    //     b = 1. - b;
    //     let scale = 1. / (1. - min);
    //     r *= scale;
    //     g *= scale;
    //     b *= scale;
    //     r = 1. - r;
    //     g = 1. - g;
    //     b = 1. - b;
    // }

    // r = r.max(0.);
    // g = g.max(0.);
    // b = b.max(0.);

    let max = r.max(g).max(b);
    if max > 1. {
        let scale = 1. / max;
        r *= scale;
        g *= scale;
        b *= scale;
    }
    (r, g, b)
}

/// Scale RGB values to fit within [?,1] gamut while preserving hue (S44 version)
///
/// - If min < 0: Assumes you will handle the -'s with the cast (Rust does this when casting to unsigned int)
/// - If max > 1: Scales all values down proportionally
///
/// This preserves the bright hues, whites especially while ensuring displayable colours.
#[cfg(feature = "spirix")]
#[inline]
pub fn scale_to_gamut_s44(mut r: S44, mut g: S44, mut b: S44) -> (S44, S44, S44) {
    let max = r.max(g).max(b);
    if max > 1 {
        let scale = max.reciprocal();
        r *= scale;
        g *= scale;
        b *= scale;
    }
    (r, g, b)
}

/// Linear VSF RGB colour (f32 per channel, 0-1 range)
/// Primaries are VSF RGB (703nm, 523nm, 462nm)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbLinearF32 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Linear RGBA colour (f32 per channel, 0-1 range)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaLinearF32 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Linear VSF RGB colour (S44 per channel, 0-1 range)
/// Primaries are VSF RGB (703nm, 523nm, 462nm)
#[cfg(feature = "spirix")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbLinearS44 {
    pub r: S44,
    pub g: S44,
    pub b: S44,
}

/// Linear RGBA colour (S44 per channel, 0-1 range)
#[cfg(feature = "spirix")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaLinearS44 {
    pub r: S44,
    pub g: S44,
    pub b: S44,
    pub a: S44,
}

impl VsfType {
    /// Convert any colour type to linear RGB (f32, 0-1 range)
    /// All VSF integers are gamma 2 encoded wheras floats are linear
    pub fn to_rgb_linear_f32(&self) -> Option<RgbLinearF32> {
        match self {
            // Named shortcuts (gamma 2 encoded)
            VsfType::rck => Some(RgbLinearF32 {
                r: 0.,
                g: 0.,
                b: 0.,
            }), // Black
            VsfType::rcw => Some(RgbLinearF32 {
                r: 1.,
                g: 1.,
                b: 1.,
            }), // White
            VsfType::rcg => Some(RgbLinearF32 {
                r: 0.25,
                g: 0.25,
                b: 0.25,
            }), // Middle grey
            VsfType::rcr => Some(RgbLinearF32 {
                r: 1.,
                g: 0.,
                b: 0.,
            }), // Red
            VsfType::rcn => Some(RgbLinearF32 {
                r: 0.,
                g: 1.,
                b: 0.,
            }), // Green
            VsfType::rcb => Some(RgbLinearF32 {
                r: 0.,
                g: 0.,
                b: 1.,
            }), // Blue
            VsfType::rcc => Some(RgbLinearF32 {
                r: 0.,
                g: 1.,
                b: 1.,
            }), // Cyan
            VsfType::rcj => Some(RgbLinearF32 {
                r: 1.,
                g: 0.,
                b: 1.,
            }), // Magenta
            VsfType::rcy => Some(RgbLinearF32 {
                r: 1.,
                g: 1.,
                b: 0.,
            }), // Yellow
            VsfType::rco => Some(RgbLinearF32 {
                r: 1.,
                g: 0.5,
                b: 0.,
            }), // Orange
            VsfType::rcl => Some(RgbLinearF32 {
                r: 0.5,
                g: 1.,
                b: 0.,
            }), // Lime
            VsfType::rcq => Some(RgbLinearF32 {
                r: 0.,
                g: 1.,
                b: 0.5,
            }), // Aqua
            VsfType::rcv => Some(RgbLinearF32 {
                r: 0.25,
                g: 0.,
                b: 1.,
            }), // Purple

            // Greyscale → RGB (replicate value, linearize)
            VsfType::re(grey) => {
                let lin = linearize_gamma2_u8_f32(*grey);
                Some(RgbLinearF32 {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rx(grey) => {
                let lin = linearize_gamma2_u16_f32(*grey);
                Some(RgbLinearF32 {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rz(grey) => {
                // rz stores linear f32 directly
                Some(RgbLinearF32 {
                    r: *grey,
                    g: *grey,
                    b: *grey,
                })
            }

            // Packed RGB (gamma-encoded, lossy - linearize)
            VsfType::ri(packed) => {
                let (r, g, b) = unpack_rgb_676_linear_f32(*packed);
                Some(RgbLinearF32 { r, g, b })
            }
            VsfType::rp(packed) => {
                let (r, g, b) = unpack_rgb_565_linear_f32(*packed);
                Some(RgbLinearF32 { r, g, b })
            }

            // Standard RGB (gamma-encoded - linearize)
            VsfType::ru([r, g, b]) => Some(RgbLinearF32 {
                r: linearize_gamma2_u8_f32(*r),
                g: linearize_gamma2_u8_f32(*g),
                b: linearize_gamma2_u8_f32(*b),
            }),
            VsfType::rs([r, g, b]) => Some(RgbLinearF32 {
                r: linearize_gamma2_u16_f32(*r),
                g: linearize_gamma2_u16_f32(*g),
                b: linearize_gamma2_u16_f32(*b),
            }),
            VsfType::rf([r, g, b]) => Some(RgbLinearF32 {
                r: *r,
                g: *g,
                b: *b,
            }), // Already linear

            // RGBA → RGB (drop alpha, linearize)
            VsfType::ra([r, g, b, _]) => Some(RgbLinearF32 {
                r: linearize_gamma2_u8_f32(*r),
                g: linearize_gamma2_u8_f32(*g),
                b: linearize_gamma2_u8_f32(*b),
            }),
            VsfType::rt([r, g, b, _]) => Some(RgbLinearF32 {
                r: linearize_gamma2_u16_f32(*r),
                g: linearize_gamma2_u16_f32(*g),
                b: linearize_gamma2_u16_f32(*b),
            }),
            VsfType::rh([r, g, b, _]) => Some(RgbLinearF32 {
                r: *r,
                g: *g,
                b: *b,
            }), // Already linear

            // General format and magic matrix not supported for simple conversion
            _ => None,
        }
    }

    /// Convert any colour type to linear RGBA (f32, 0-1 range)
    pub fn to_rgba_linear_f32(&self) -> Option<RgbaLinearF32> {
        match self {
            // RGBA formats (gamma-encoded - linearize)
            VsfType::ra([r, g, b, a]) => Some(RgbaLinearF32 {
                r: linearize_gamma2_u8_f32(*r),
                g: linearize_gamma2_u8_f32(*g),
                b: linearize_gamma2_u8_f32(*b),
                a: *a as f32 / 255.0, // Alpha is already linear!
            }),
            VsfType::rt([r, g, b, a]) => Some(RgbaLinearF32 {
                r: linearize_gamma2_u16_f32(*r),
                g: linearize_gamma2_u16_f32(*g),
                b: linearize_gamma2_u16_f32(*b),
                a: *a as f32 / 65535.0, // Alpha is already linear!
            }),
            VsfType::rh([r, g, b, a]) => Some(RgbaLinearF32 {
                r: *r,
                g: *g,
                b: *b,
                a: *a,
            }), // Already linear

            // RGB formats → add opaque alpha
            _ => self.to_rgb_linear_f32().map(|rgb| RgbaLinearF32 {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
                a: 1., // Opaque
            }),
        }
    }

    /// Convert any colour type to linear RGB (S44, 0-1 range)
    /// All VSF integers are gamma 2 encoded whereas floats are linear
    #[cfg(feature = "spirix")]
    pub fn to_rgb_linear_s44(&self) -> Option<RgbLinearS44> {
        const QUARTER: S44 = sf!(0.25);
        const HALF: S44 = sf!(0.5);
        match self {
            // Named shortcuts (gamma 2 encoded)
            VsfType::rck => Some(RgbLinearS44 {
                r: S44::ZERO,
                g: S44::ZERO,
                b: S44::ZERO,
            }),
            VsfType::rcw => Some(RgbLinearS44 {
                r: S44::ONE,
                g: S44::ONE,
                b: S44::ONE,
            }),
            VsfType::rcg => Some(RgbLinearS44 {
                r: QUARTER,
                g: QUARTER,
                b: QUARTER,
            }),
            VsfType::rcr => Some(RgbLinearS44 {
                r: S44::ONE,
                g: S44::ZERO,
                b: S44::ZERO,
            }),
            VsfType::rcn => Some(RgbLinearS44 {
                r: S44::ZERO,
                g: S44::ONE,
                b: S44::ZERO,
            }),
            VsfType::rcb => Some(RgbLinearS44 {
                r: S44::ZERO,
                g: S44::ZERO,
                b: S44::ONE,
            }),
            VsfType::rcc => Some(RgbLinearS44 {
                r: S44::ZERO,
                g: S44::ONE,
                b: S44::ONE,
            }),
            VsfType::rcj => Some(RgbLinearS44 {
                r: S44::ONE,
                g: S44::ZERO,
                b: S44::ONE,
            }),
            VsfType::rcy => Some(RgbLinearS44 {
                r: S44::ONE,
                g: S44::ONE,
                b: S44::ZERO,
            }),
            VsfType::rco => Some(RgbLinearS44 {
                r: S44::ONE,
                g: HALF,
                b: S44::ZERO,
            }),
            VsfType::rcl => Some(RgbLinearS44 {
                r: HALF,
                g: S44::ONE,
                b: S44::ZERO,
            }),
            VsfType::rcq => Some(RgbLinearS44 {
                r: S44::ZERO,
                g: S44::ONE,
                b: HALF,
            }),
            VsfType::rcv => Some(RgbLinearS44 {
                r: QUARTER,
                g: S44::ZERO,
                b: S44::ONE,
            }),

            // Greyscale → RGB (replicate value, linearize)
            VsfType::re(grey) => {
                let lin = linearize_gamma2_u8_s44(*grey);
                Some(RgbLinearS44 {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rx(grey) => {
                let lin = linearize_gamma2_u16_s44(*grey);
                Some(RgbLinearS44 {
                    r: lin,
                    g: lin,
                    b: lin,
                })
            }
            VsfType::rz(grey) => Some(RgbLinearS44 {
                r: S44::from_f32(*grey),
                g: S44::from_f32(*grey),
                b: S44::from_f32(*grey),
            }),

            // Packed RGB (gamma-encoded, lossy - linearize)
            VsfType::ri(packed) => {
                let (r, g, b) = unpack_rgb_676_linear_s44(*packed);
                Some(RgbLinearS44 { r, g, b })
            }
            VsfType::rp(packed) => {
                let (r, g, b) = unpack_rgb_565_linear_s44(*packed);
                Some(RgbLinearS44 { r, g, b })
            }

            // Standard RGB (gamma-encoded - linearize)
            VsfType::ru([r, g, b]) => Some(RgbLinearS44 {
                r: linearize_gamma2_u8_s44(*r),
                g: linearize_gamma2_u8_s44(*g),
                b: linearize_gamma2_u8_s44(*b),
            }),
            VsfType::rs([r, g, b]) => Some(RgbLinearS44 {
                r: linearize_gamma2_u16_s44(*r),
                g: linearize_gamma2_u16_s44(*g),
                b: linearize_gamma2_u16_s44(*b),
            }),
            VsfType::rf([r, g, b]) => Some(RgbLinearS44 {
                r: S44::from_f32(*r),
                g: S44::from_f32(*g),
                b: S44::from_f32(*b),
            }),

            // RGBA → RGB (drop alpha, linearize)
            VsfType::ra([r, g, b, _]) => Some(RgbLinearS44 {
                r: linearize_gamma2_u8_s44(*r),
                g: linearize_gamma2_u8_s44(*g),
                b: linearize_gamma2_u8_s44(*b),
            }),
            VsfType::rt([r, g, b, _]) => Some(RgbLinearS44 {
                r: linearize_gamma2_u16_s44(*r),
                g: linearize_gamma2_u16_s44(*g),
                b: linearize_gamma2_u16_s44(*b),
            }),
            VsfType::rh([r, g, b, _]) => Some(RgbLinearS44 {
                r: S44::from_f32(*r),
                g: S44::from_f32(*g),
                b: S44::from_f32(*b),
            }),

            _ => None,
        }
    }

    /// Convert any colour type to linear RGBA (S44, 0-1 range)
    #[cfg(feature = "spirix")]
    pub fn to_rgba_linear_s44(&self) -> Option<RgbaLinearS44> {
        match self {
            // RGBA formats (gamma-encoded - linearize)
            VsfType::ra([r, g, b, a]) => Some(RgbaLinearS44 {
                r: linearize_gamma2_u8_s44(*r),
                g: linearize_gamma2_u8_s44(*g),
                b: linearize_gamma2_u8_s44(*b),
                a: S44::from(a) >> 8, // Alpha is already linear!
            }),
            VsfType::rt([r, g, b, a]) => Some(RgbaLinearS44 {
                r: linearize_gamma2_u16_s44(*r),
                g: linearize_gamma2_u16_s44(*g),
                b: linearize_gamma2_u16_s44(*b),
                a: S44::from(a) >> 16, // Alpha is already linear!
            }),
            VsfType::rh([r, g, b, a]) => Some(RgbaLinearS44 {
                r: S44::from_f32(*r),
                g: S44::from_f32(*g),
                b: S44::from_f32(*b),
                a: S44::from_f32(*a),
            }),

            // RGB formats → add opaque alpha
            _ => self.to_rgb_linear_s44().map(|rgb| RgbaLinearS44 {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
                a: S44::ONE,
            }),
        }
    }

    /// Convert any colour type to 8-bit greyscale (f32 pipeline)
    ///
    /// Uses VSF RGB photopic luminance matrix for RGB → Grey conversions
    pub fn to_grey8_f32(&self) -> Option<u8> {
        match self {
            // Greyscale formats (direct)
            VsfType::re(grey) => Some(*grey),
            VsfType::rx(grey) => Some((*grey >> 8) as u8),
            VsfType::rz(grey) => Some(delinearize_gamma2_u8_f32(*grey)),

            // RGB → Grey: Use VSF RGB photopic luminance (in linear space)
            _ => self.to_rgb_linear_f32().map(|rgb| {
                let lum = vsf_rgb_to_photopic_f32(rgb.r, rgb.g, rgb.b);
                delinearize_gamma2_u8_f32(lum)
            }),
        }
    }

    /// Convert any colour type to 8-bit greyscale (S44 pipeline)
    ///
    /// Uses VSF RGB photopic luminance matrix for RGB → Grey conversions
    #[cfg(feature = "spirix")]
    pub fn to_grey8_s44(&self) -> Option<u8> {
        match self {
            // Greyscale formats (direct)
            VsfType::re(grey) => Some(*grey),
            VsfType::rx(grey) => Some((*grey >> 8) as u8),
            VsfType::rz(grey) => Some(delinearize_gamma2_u8_s44(S44::from_f32(*grey))),

            // RGB → Grey: Use VSF RGB photopic luminance (in linear space)
            _ => self.to_rgb_linear_s44().map(|rgb| {
                let lum = vsf_rgb_to_photopic_s44(rgb.r, rgb.g, rgb.b);
                delinearize_gamma2_u8_s44(lum)
            }),
        }
    }

    /// Convert VSF RGB to native 8-bit gamma-encoded values (0-255 range, f32 pipeline)
    ///
    /// Uses VSF RGB gamma 2 and ×256 truncation quantization (fast).
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    pub fn to_rgb_u8_f32(&self) -> Option<(u8, u8, u8)> {
        let rgb = self.to_rgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(rgb.r, rgb.g, rgb.b);
        Some((
            delinearize_gamma2_u8_f32(r),
            delinearize_gamma2_u8_f32(g),
            delinearize_gamma2_u8_f32(b),
        ))
    }

    /// Convert VSF RGB to native 8-bit gamma-encoded values (0-255 range, S44 pipeline)
    ///
    /// Uses VSF RGB gamma 2 and ×256 truncation quantization (fast).
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    #[cfg(feature = "spirix")]
    pub fn to_rgb_u8_s44(&self) -> Option<(u8, u8, u8)> {
        let rgb = self.to_rgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(rgb.r, rgb.g, rgb.b);
        Some((
            delinearize_gamma2_u8_s44(r),
            delinearize_gamma2_u8_s44(g),
            delinearize_gamma2_u8_s44(b),
        ))
    }

    /// Convert VSF RGB to LMS cone space (linear, f32)
    ///
    /// LMS represents Long, Medium, and Short wavelength cone responses.
    /// Used as a perceptually-uniform intermediate space for chromatic adaptation.
    ///
    /// Based on CIE 2006 2° Standard Observer.
    pub fn to_lms_linear_f32(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear_f32()?;
        use crate::colour::VSF_RGB2LMS;
        let result = apply_matrix_3x3_f32(&VSF_RGB2LMS, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to CIE 1931 XYZ tristimulus values (linear, f32)
    ///
    /// **DEPRECATED**: XYZ is based on CIE 1931 Standard Observer (legacy).
    ///
    /// XYZ tristimulus values are the foundation of most xy-coordinate-based
    /// colour standards. Useful for colourimetric calculations and converting
    /// between legacy colourspaces.
    pub fn to_xyz_linear_f32(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear_f32()?;
        use crate::colour::VSF_RGB2XYZ;
        let result = apply_matrix_3x3_f32(&VSF_RGB2XYZ, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to LMS cone space (linear, S44)
    #[cfg(feature = "spirix")]
    pub fn to_lms_linear_s44(&self) -> Option<(S44, S44, S44)> {
        let rgb = self.to_rgb_linear_s44()?;
        use crate::colour::VSF_RGB2LMS_S44;
        let result = apply_matrix_3x3_s44(&VSF_RGB2LMS_S44, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to CIE 1931 XYZ tristimulus values (linear, S44)
    #[cfg(feature = "spirix")]
    pub fn to_xyz_linear_s44(&self) -> Option<(S44, S44, S44)> {
        let rgb = self.to_rgb_linear_s44()?;
        use crate::colour::VSF_RGB2XYZ_S44;
        let result = apply_matrix_3x3_s44(&VSF_RGB2XYZ_S44, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Create colour from sRGB with piecewise transfer function
    ///
    /// Converts from sRGB/Rec.709 colourspace (D65 white) to VSF RGB (E white)
    /// Uses the sRGB piecewise transfer function (linear + gamma 2.4)
    /// Conversion path: sRGB → LMS → VSF RGB (precomputed)
    /// Convert VSF RGB colour to sRGB with piecewise transfer function
    ///
    /// Converts from VSF RGB (E white) to sRGB/Rec.709 (D65 white)
    /// Uses the sRGB piecewise transfer function (linear + gamma 2.4)
    /// Conversion path: VSF RGB → Rec.709 → sRGB encoding (precomputed)
    ///
    /// Returns (r, g, b) as 8-bit sRGB values
    /// Convert VSF RGB to sRGB/Rec.709 linear (f32, 0-1 nominal range)
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    /// Use this for HDR or when you need the raw linear values.
    /// Convert VSF RGB to sRGB linear (f32, 0-1 nominal range)
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    /// Use `scale_to_gamut_f32()` to bring into displayable range, prioritizing hue
    pub fn to_srgb_linear_f32(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear_f32()?;
        use crate::colour::VSF_RGB2SRGB;
        let result = apply_matrix_3x3_f32(&VSF_RGB2SRGB, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to BT.2020/Rec.2020 linear (f32, 0-1 nominal range)
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    /// Use this for HDR or when you need the raw linear values.
    pub fn to_rec2020_linear_f32(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear_f32()?;
        use crate::colour::VSF_RGB2REC2020;
        let result = apply_matrix_3x3_f32(&VSF_RGB2REC2020, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to sRGB linear (S44, 0-1 nominal range)
    #[cfg(feature = "spirix")]
    pub fn to_srgb_linear_s44(&self) -> Option<(S44, S44, S44)> {
        let rgb = self.to_rgb_linear_s44()?;
        use crate::colour::VSF_RGB2SRGB_S44;
        let result = apply_matrix_3x3_s44(&VSF_RGB2SRGB_S44, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to BT.2020/Rec.2020 linear (S44, 0-1 nominal range)
    #[cfg(feature = "spirix")]
    pub fn to_rec2020_linear_s44(&self) -> Option<(S44, S44, S44)> {
        let rgb = self.to_rgb_linear_s44()?;
        use crate::colour::VSF_RGB2REC2020_S44;
        let result = apply_matrix_3x3_s44(&VSF_RGB2REC2020_S44, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to Adobe RGB linear (S44, 0-1 nominal range)
    #[cfg(feature = "spirix")]
    pub fn to_adobe_rgb_linear_s44(&self) -> Option<(S44, S44, S44)> {
        let rgb = self.to_rgb_linear_s44()?;
        use crate::colour::VSF_RGB2ADOBE_RGB_S44;
        let result = apply_matrix_3x3_s44(&VSF_RGB2ADOBE_RGB_S44, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to sRGB (u8, gamma-encoded, 0-255 range, f32 pipeline)
    ///
    /// Uses sRGB OETF and ×255 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    pub fn to_srgb_u8_f32(&self) -> Option<(u8, u8, u8)> {
        use crate::colour::legacy::transfer::srgb_oetf;
        let (r, g, b) = self.to_srgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (srgb_oetf(r) * 255.).round() as u8,
            (srgb_oetf(g) * 255.).round() as u8,
            (srgb_oetf(b) * 255.).round() as u8,
        ))
    }

    /// Convert VSF RGB to sRGB (u8, gamma-encoded, 0-255 range, S44 pipeline)
    ///
    /// Uses sRGB OETF and ×255 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    #[cfg(feature = "spirix")]
    pub fn to_srgb_u8_s44(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_srgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (srgb_oetf_s44(r) * 255i16).round().to_u8(),
            (srgb_oetf_s44(g) * 255i16).round().to_u8(),
            (srgb_oetf_s44(b) * 255i16).round().to_u8(),
        ))
    }

    /// Convert VSF RGB to sRGB (u16, gamma-encoded, 0-65535 range, f32 pipeline)
    ///
    /// Uses sRGB OETF and ×65535 + round quantization (sRGB spec).
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    pub fn to_srgb_u16_f32(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (delinearize_srgb(r) * 65535.).round() as u16,
            (delinearize_srgb(g) * 65535.).round() as u16,
            (delinearize_srgb(b) * 65535.).round() as u16,
        ))
    }

    /// Convert VSF RGB to sRGB (u16, gamma-encoded, 0-65535 range, S44 pipeline)
    ///
    /// Uses sRGB OETF and ×65535 + round quantization (sRGB spec).
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    #[cfg(feature = "spirix")]
    pub fn to_srgb_u16_s44(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (srgb_oetf_s44(r) * 65535i32).round().to_u16(),
            (srgb_oetf_s44(g) * 65535i32).round().to_u16(),
            (srgb_oetf_s44(b) * 65535i32).round().to_u16(),
        ))
    }

    /// Convert VSF RGB to Rec.709 (u8, gamma-encoded, studio range 16-235, f32 pipeline)
    pub fn to_rec709_u8_f32(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_srgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (encode_bt709(r) * 219.) as u8 + 16,
            (encode_bt709(g) * 219.) as u8 + 16,
            (encode_bt709(b) * 219.) as u8 + 16,
        ))
    }

    /// Convert VSF RGB to Rec.709 (u8, gamma-encoded, studio range 16-235, S44 pipeline)
    #[cfg(feature = "spirix")]
    pub fn to_rec709_u8_s44(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_srgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (encode_bt709_s44(r) * 219i16).round().to_u8() + 16,
            (encode_bt709_s44(g) * 219i16).round().to_u8() + 16,
            (encode_bt709_s44(b) * 219i16).round().to_u8() + 16,
        ))
    }

    /// Convert VSF RGB to Rec.709 (u16, gamma-encoded, studio range 4096-60160, f32 pipeline)
    pub fn to_rec709_u16_f32(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (encode_bt709(r) * 56064.) as u16 + 4096,
            (encode_bt709(g) * 56064.) as u16 + 4096,
            (encode_bt709(b) * 56064.) as u16 + 4096,
        ))
    }

    /// Convert VSF RGB to Rec.709 (u16, gamma-encoded, studio range 4096-60160, S44 pipeline)
    #[cfg(feature = "spirix")]
    pub fn to_rec709_u16_s44(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_srgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (encode_bt709_s44(r) * 56064i32).round().to_u16() + 4096,
            (encode_bt709_s44(g) * 56064i32).round().to_u16() + 4096,
            (encode_bt709_s44(b) * 56064i32).round().to_u16() + 4096,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u8, gamma-encoded, studio range 16-235, f32 pipeline)
    pub fn to_rec2020_u8_f32(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_rec2020_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (encode_bt709(r) * 219.) as u8 + 16,
            (encode_bt709(g) * 219.) as u8 + 16,
            (encode_bt709(b) * 219.) as u8 + 16,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u8, gamma-encoded, studio range 16-235, S44 pipeline)
    #[cfg(feature = "spirix")]
    pub fn to_rec2020_u8_s44(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_rec2020_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (encode_bt709_s44(r) * 219i16).round().to_u8() + 16,
            (encode_bt709_s44(g) * 219i16).round().to_u8() + 16,
            (encode_bt709_s44(b) * 219i16).round().to_u8() + 16,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u16, gamma-encoded, studio range 4096-60160)
    pub fn to_rec2020_u16_f32(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_rec2020_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (encode_bt709(r) * 56064.) as u16 + 4096,
            (encode_bt709(g) * 56064.) as u16 + 4096,
            (encode_bt709(b) * 56064.) as u16 + 4096,
        ))
    }

    /// Convert VSF RGB to Rec.2020 (u16, gamma-encoded, studio range 4096-60160, S44 pipeline)
    #[cfg(feature = "spirix")]
    pub fn to_rec2020_u16_s44(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_rec2020_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (encode_bt709_s44(r) * 56064i32).round().to_u16() + 4096,
            (encode_bt709_s44(g) * 56064i32).round().to_u16() + 4096,
            (encode_bt709_s44(b) * 56064i32).round().to_u16() + 4096,
        ))
    }

    /// Convert VSF RGB to Adobe RGB linear (f32, 0-1 nominal range)
    ///
    /// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
    ///
    /// Returns linear light values. May be out of gamut (negative or >1).
    pub fn to_adobe_rgb_linear_f32(&self) -> Option<(f32, f32, f32)> {
        let rgb = self.to_rgb_linear_f32()?;
        use crate::colour::VSF_RGB2ADOBE_RGB;
        let result = apply_matrix_3x3_f32(&VSF_RGB2ADOBE_RGB, &[rgb.r, rgb.g, rgb.b]);
        Some((result[0], result[1], result[2]))
    }

    /// Convert VSF RGB to Adobe RGB (u8, gamma-encoded, 0-255 range, f32 pipeline)
    ///
    /// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
    ///
    /// Uses Adobe RGB gamma 2.2 and ×255 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    pub fn to_adobe_rgb_u8_f32(&self) -> Option<(u8, u8, u8)> {
        use crate::colour::legacy::transfer::adobe_rgb_oetf;
        let (r, g, b) = self.to_adobe_rgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (adobe_rgb_oetf(r) * 255.).round() as u8,
            (adobe_rgb_oetf(g) * 255.).round() as u8,
            (adobe_rgb_oetf(b) * 255.).round() as u8,
        ))
    }

    /// Convert VSF RGB to Adobe RGB (u8, gamma-encoded, 0-255 range, S44 pipeline)
    ///
    /// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
    ///
    /// Uses Adobe RGB gamma 2.2 and ×255 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    #[cfg(feature = "spirix")]
    pub fn to_adobe_rgb_u8_s44(&self) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.to_adobe_rgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (adobe_rgb_oetf_s44(r) * 255i16).round().to_u8(),
            (adobe_rgb_oetf_s44(g) * 255i16).round().to_u8(),
            (adobe_rgb_oetf_s44(b) * 255i16).round().to_u8(),
        ))
    }

    /// Convert VSF RGB to Adobe RGB (u16, gamma-encoded, 0-65535 range, f32 pipeline)
    ///
    /// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
    ///
    /// Uses Adobe RGB gamma 2.2 and ×65535 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    pub fn to_adobe_rgb_u16_f32(&self) -> Option<(u16, u16, u16)> {
        use crate::colour::legacy::transfer::adobe_rgb_oetf;
        let (r, g, b) = self.to_adobe_rgb_linear_f32()?;
        let (r, g, b) = scale_to_gamut_f32(r, g, b);
        Some((
            (adobe_rgb_oetf(r) * 65535.).round() as u16,
            (adobe_rgb_oetf(g) * 65535.).round() as u16,
            (adobe_rgb_oetf(b) * 65535.).round() as u16,
        ))
    }

    /// Convert VSF RGB to Adobe RGB (u16, gamma-encoded, 0-65535 range, S44 pipeline)
    ///
    /// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
    ///
    /// Uses Adobe RGB gamma 2.2 and ×65535 + round quantization.
    /// Automatically scales out-of-gamut colours to fit [0,1] range.
    #[cfg(feature = "spirix")]
    pub fn to_adobe_rgb_u16_s44(&self) -> Option<(u16, u16, u16)> {
        let (r, g, b) = self.to_adobe_rgb_linear_s44()?;
        let (r, g, b) = scale_to_gamut_s44(r, g, b);
        Some((
            (adobe_rgb_oetf_s44(r) * 65535i32).round().to_u16(),
            (adobe_rgb_oetf_s44(g) * 65535i32).round().to_u16(),
            (adobe_rgb_oetf_s44(b) * 65535i32).round().to_u16(),
        ))
    }

    /// Convert from sRGB to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded full range [0,255]), or u16 (gamma-encoded full range [0,65535]).
    /// **Convention**: Floats are always linear, integers are always gamma-encoded.
    ///
    /// Uses sRGB piecewise transfer function (EOTF) for integers, direct pass-thru for floats.
    ///
    /// # Examples
    /// ```ignore
    /// // From 8-bit gamma-encoded sRGB (full range)
    /// let colour = VsfType::from_srgb(255u8, 128u8, 64u8, ColourFormat::Rf);
    ///
    /// // From linear float sRGB
    /// let colour = VsfType::from_srgb(1.0f32, 0.5f32, 0.25f32, ColourFormat::Rf);
    /// ```
    pub fn from_srgb_f32<T: ColourValue<f32>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        // Apply sRGB EOTF (for integers) or pass-thru (floats already linear)
        let r_lin = r.to_linear_srgb();
        let g_lin = g.to_linear_srgb();
        let b_lin = b.to_linear_srgb();

        // sRGB → VSF RGB matrix (sRGB and Rec.709 share the same primaries)
        use crate::colour::SRGB2VSF_RGB;
        let result = apply_matrix_3x3_f32(&SRGB2VSF_RGB, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_f32(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from sRGB to VSF RGB (S44 version)
    #[cfg(feature = "spirix")]
    pub fn from_srgb_s44<T: ColourValue<S44>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        let r_lin = r.to_linear_srgb();
        let g_lin = g.to_linear_srgb();
        let b_lin = b.to_linear_srgb();

        use crate::colour::SRGB2VSF_RGB_S44;
        let result = apply_matrix_3x3_s44(&SRGB2VSF_RGB_S44, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_s44(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from Rec.709 to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded studio range [16,235]), or u16 (gamma-encoded studio range [4096,60160]).
    /// **Convention**: Floats are always linear, integers use studio range.
    ///
    /// Uses Rec.709 OETF/EOTF transfer function. Note: Rec.709 and sRGB have the same primaries,
    /// only the transfer function and quantization range differ.
    ///
    /// # Examples
    /// ```ignore
    /// // From 8-bit gamma-encoded Rec.709 (studio range [16,235])
    /// let colour = VsfType::from_rec709(235u8, 128u8, 64u8, ColourFormat::Rf);
    ///
    /// // From linear float Rec.709
    /// let colour = VsfType::from_rec709(1f32, 0.5f32, 0.25f32, ColourFormat::Rf);
    /// ```
    pub fn from_rec709_f32<T: ColourValue<f32>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        // Apply Rec.709 EOTF (for integers in studio range) or pass-thru (floats are linear)
        let r_lin = r.to_linear_rec709();
        let g_lin = g.to_linear_rec709();
        let b_lin = b.to_linear_rec709();

        // Rec.709 → VSF RGB matrix (same as sRGB - same primaries, different transfer)
        use crate::colour::SRGB2VSF_RGB;
        let result = apply_matrix_3x3_f32(&SRGB2VSF_RGB, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_f32(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from Rec.709 to VSF RGB (S44 version)
    #[cfg(feature = "spirix")]
    pub fn from_rec709_s44<T: ColourValue<S44>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        let r_lin = r.to_linear_rec709();
        let g_lin = g.to_linear_rec709();
        let b_lin = b.to_linear_rec709();

        use crate::colour::SRGB2VSF_RGB_S44;
        let result = apply_matrix_3x3_s44(&SRGB2VSF_RGB_S44, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_s44(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from BT.2020/Rec.2020 to VSF RGB
    ///
    /// Accepts f32 (linear), u8 (gamma-encoded studio range [16,235]), or u16 (gamma-encoded studio range [4096,60160]).
    /// **Convention**: Floats are always linear, integers use studio range.
    ///
    /// Uses Rec.709 OETF/EOTF transfer function (BT.2020 uses Rec.709 transfer, but BT.1886 for display).
    ///
    /// # Examples
    /// ```ignore
    /// // From 8-bit gamma-encoded Rec.2020 (studio range [16,235])
    /// let colour = VsfType::from_rec2020(235u8, 128u8, 64u8, ColourFormat::Rf);
    ///
    /// // From linear float Rec.2020
    /// let colour = VsfType::from_rec2020(1f32, 0.5f32, 0.25f32, ColourFormat::Rf);
    /// ```
    pub fn from_rec2020_f32<T: ColourValue<f32>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        let r_lin = r.to_linear_rec709(); // BT.2020 uses Rec.709 OETF
        let g_lin = g.to_linear_rec709();
        let b_lin = b.to_linear_rec709();

        // BT.2020 → VSF RGB matrix
        let result = apply_matrix_3x3_f32(&REC2020_2VSF_RGB, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_f32(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from BT.2020/Rec.2020 to VSF RGB (S44 version)
    #[cfg(feature = "spirix")]
    pub fn from_rec2020_s44<T: ColourValue<S44>>(r: T, g: T, b: T, format: ColourFormat) -> Self {
        let r_lin = r.to_linear_rec709();
        let g_lin = g.to_linear_rec709();
        let b_lin = b.to_linear_rec709();

        use crate::colour::rec2020::REC2020_2VSF_RGB_S44;
        let result = apply_matrix_3x3_s44(&REC2020_2VSF_RGB_S44, &[r_lin, g_lin, b_lin]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);

        Self::from_rgb_linear_s44(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from LMS cone space to VSF RGB
    ///
    /// LMS represents Long, Medium, and Short wavelength cone responses.
    /// Based on CIE 2006 2° Standard Observer.
    ///
    /// # Arguments
    /// * `l` - Long wavelength cone response (linear, 0-1 nominal range)
    /// * `m` - Medium wavelength cone response (linear, 0-1 nominal range)
    /// * `s` - Short wavelength cone response (linear, 0-1 nominal range)
    /// * `format` - Target VSF colour format
    ///
    /// # Examples
    /// ```ignore
    /// let colour = VsfType::from_lms(0.5, 0.4, 0.3, ColourFormat::Rf);
    /// ```
    pub fn from_lms_f32(l: f32, m: f32, s: f32, format: ColourFormat) -> Self {
        use crate::colour::LMS2VSF_RGB;
        let result = apply_matrix_3x3_f32(&LMS2VSF_RGB, &[l, m, s]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);
        Self::from_rgb_linear_f32(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from CIE 1931 XYZ tristimulus values to VSF RGB
    ///
    /// **DEPRECATED**: XYZ is based on CIE 1931 Standard Observer (legacy).
    ///
    /// XYZ tristimulus values are the foundation of most xy-coordinate-based
    /// colour standards.
    ///
    /// # Arguments
    /// * `x` - X tristimulus value (linear, 0-1 nominal range)
    /// * `y` - Y tristimulus value (linear, 0-1 nominal range)
    /// * `z` - Z tristimulus value (linear, 0-1 nominal range)
    /// * `format` - Target VSF colour format
    ///
    /// # Examples
    /// ```ignore
    /// let colour = VsfType::from_xyz(0.4, 0.5, 0.3, ColourFormat::Rf);
    /// ```
    pub fn from_xyz_f32(x: f32, y: f32, z: f32, format: ColourFormat) -> Self {
        use crate::colour::XYZ2VSF_RGB;
        let result = apply_matrix_3x3_f32(&XYZ2VSF_RGB, &[x, y, z]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);
        Self::from_rgb_linear_f32(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from LMS cone space to VSF RGB (S44 version)
    #[cfg(feature = "spirix")]
    pub fn from_lms_s44(l: S44, m: S44, s: S44, format: ColourFormat) -> Self {
        use crate::colour::LMS2VSF_RGB_S44;
        let result = apply_matrix_3x3_s44(&LMS2VSF_RGB_S44, &[l, m, s]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);
        Self::from_rgb_linear_s44(vsf_r, vsf_g, vsf_b, format)
    }

    /// Convert from CIE 1931 XYZ tristimulus values to VSF RGB (S44 version)
    #[cfg(feature = "spirix")]
    pub fn from_xyz_s44(x: S44, y: S44, z: S44, format: ColourFormat) -> Self {
        use crate::colour::XYZ2VSF_RGB_S44;
        let result = apply_matrix_3x3_s44(&XYZ2VSF_RGB_S44, &[x, y, z]);
        let (vsf_r, vsf_g, vsf_b) = (result[0], result[1], result[2]);
        Self::from_rgb_linear_s44(vsf_r, vsf_g, vsf_b, format)
    }

    /// Helper: Create VsfType from linear RGB floats
    /// For integer formats: scale to gamut (preserves hue/saturation, prevents white clipping)
    /// For float formats: preserve full range (no clamping)
    fn from_rgb_linear_f32(r: f32, g: f32, b: f32, format: ColourFormat) -> Self {
        match format {
            ColourFormat::Rf => VsfType::rf([r, g, b]),
            ColourFormat::Ru => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::ru([
                    delinearize_gamma2_u8_f32(r),
                    delinearize_gamma2_u8_f32(g),
                    delinearize_gamma2_u8_f32(b),
                ])
            }
            ColourFormat::Rs => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::rs([
                    delinearize_gamma2_u16_f32(r),
                    delinearize_gamma2_u16_f32(g),
                    delinearize_gamma2_u16_f32(b),
                ])
            }
            ColourFormat::Ri => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::ri(pack_rgb_676_linear_f32(r, g, b))
            }
            ColourFormat::Rp => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::rp(pack_rgb_565_linear_f32(r, g, b))
            }
            // Greyscale formats
            ColourFormat::Re => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                let lum = vsf_rgb_to_photopic_f32(r, g, b);
                VsfType::re(delinearize_gamma2_u8_f32(lum))
            }
            ColourFormat::Rx => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                let lum = vsf_rgb_to_photopic_f32(r, g, b);
                VsfType::rx(delinearize_gamma2_u16_f32(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic_f32(r, g, b);
                VsfType::rz(lum)
            }
            // RGBA formats → add opaque alpha
            ColourFormat::Ra => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::ra([
                    delinearize_gamma2_u8_f32(r),
                    delinearize_gamma2_u8_f32(g),
                    delinearize_gamma2_u8_f32(b),
                    255,
                ])
            }
            ColourFormat::Rt => {
                let (r, g, b) = scale_to_gamut_f32(r, g, b);
                VsfType::rt([
                    delinearize_gamma2_u16_f32(r),
                    delinearize_gamma2_u16_f32(g),
                    delinearize_gamma2_u16_f32(b),
                    0xFFFF,
                ])
            }
            ColourFormat::Rh => VsfType::rh([r, g, b, 1.0]),
            #[cfg(feature = "spirix")]
            ColourFormat::Rd => {
                let lum = vsf_rgb_to_photopic_f32(r, g, b);
                VsfType::rd(S44::from(lum))
            }
            #[cfg(feature = "spirix")]
            ColourFormat::Rb => VsfType::rb([S44::from(r), S44::from(g), S44::from(b)]),
            #[cfg(feature = "spirix")]
            ColourFormat::Rw => VsfType::rw([S44::from(r), S44::from(g), S44::from(b), S44::ONE]),
        }
    }

    /// Helper: Create VsfType from linear RGB S44
    /// For integer formats: scale to gamut (preserves hue/saturation, prevents white clipping)
    /// For float formats: preserve full range (no clamping)
    #[cfg(feature = "spirix")]
    fn from_rgb_linear_s44(r: S44, g: S44, b: S44, format: ColourFormat) -> Self {
        match format {
            ColourFormat::Rf => VsfType::rf([r.into(), g.into(), b.into()]),
            ColourFormat::Ru => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::ru([
                    delinearize_gamma2_u8_s44(r),
                    delinearize_gamma2_u8_s44(g),
                    delinearize_gamma2_u8_s44(b),
                ])
            }
            ColourFormat::Rs => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::rs([
                    delinearize_gamma2_u16_s44(r),
                    delinearize_gamma2_u16_s44(g),
                    delinearize_gamma2_u16_s44(b),
                ])
            }
            ColourFormat::Ri => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::ri(pack_rgb_676_linear_s44(r, g, b))
            }
            ColourFormat::Rp => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::rp(pack_rgb_565_linear_s44(r, g, b))
            }
            // Greyscale formats
            ColourFormat::Re => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                let lum = vsf_rgb_to_photopic_s44(r, g, b);
                VsfType::re(delinearize_gamma2_u8_s44(lum))
            }
            ColourFormat::Rx => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                let lum = vsf_rgb_to_photopic_s44(r, g, b);
                VsfType::rx(delinearize_gamma2_u16_s44(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic_s44(r, g, b);
                VsfType::rz(lum.into())
            }
            #[cfg(feature = "spirix")]
            ColourFormat::Rd => {
                let lum = vsf_rgb_to_photopic_s44(r, g, b);
                VsfType::rd(lum)
            }
            // RGB S44 format
            #[cfg(feature = "spirix")]
            ColourFormat::Rb => VsfType::rb([r, g, b]),
            // RGBA formats → add opaque alpha
            ColourFormat::Ra => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::ra([
                    delinearize_gamma2_u8_s44(r),
                    delinearize_gamma2_u8_s44(g),
                    delinearize_gamma2_u8_s44(b),
                    255,
                ])
            }
            ColourFormat::Rt => {
                let (r, g, b) = scale_to_gamut_s44(r, g, b);
                VsfType::rt([
                    delinearize_gamma2_u16_s44(r),
                    delinearize_gamma2_u16_s44(g),
                    delinearize_gamma2_u16_s44(b),
                    0xFFFF,
                ])
            }
            ColourFormat::Rh => VsfType::rh([r.into(), g.into(), b.into(), 1.0]),
            #[cfg(feature = "spirix")]
            ColourFormat::Rw => VsfType::rw([r, g, b, S44::ONE]),
        }
    }

    /// Create colour from gamma-encoded RGB (8-bit per channel, f32 version)
    ///
    /// Input RGB is assumed to be gamma-encoded VSF RGB colourspace
    pub fn from_rgb8_f32(r: u8, g: u8, b: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8_f32(r);
        let g_lin = linearize_gamma2_u8_f32(g);
        let b_lin = linearize_gamma2_u8_f32(b);

        match format {
            ColourFormat::Ru => VsfType::ru([r, g, b]),
            ColourFormat::Rs => VsfType::rs([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
            ]),
            ColourFormat::Rf => VsfType::rf([r_lin, g_lin, b_lin]),
            ColourFormat::Ri => VsfType::ri(pack_rgb_676_linear_f32(r_lin, g_lin, b_lin)),
            ColourFormat::Rp => VsfType::rp(pack_rgb_565_linear_f32(r_lin, g_lin, b_lin)),

            // RGB → Greyscale: Use VSF RGB photopic luminance
            ColourFormat::Re => {
                let lum = vsf_rgb_to_photopic_f32(r_lin, g_lin, b_lin);
                VsfType::re(delinearize_gamma2_u8_f32(lum))
            }
            ColourFormat::Rx => {
                let lum = vsf_rgb_to_photopic_f32(r_lin, g_lin, b_lin);
                VsfType::rx(delinearize_gamma2_u16_f32(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic_f32(r_lin, g_lin, b_lin);
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
            #[cfg(feature = "spirix")]
            ColourFormat::Rd => {
                let lum = vsf_rgb_to_photopic_f32(r_lin, g_lin, b_lin);
                VsfType::rd(S44::from(lum))
            }
            #[cfg(feature = "spirix")]
            ColourFormat::Rb => VsfType::rb([S44::from(r_lin), S44::from(g_lin), S44::from(b_lin)]),
            #[cfg(feature = "spirix")]
            ColourFormat::Rw => VsfType::rw([
                S44::from(r_lin),
                S44::from(g_lin),
                S44::from(b_lin),
                S44::ONE,
            ]),
        }
    }

    /// Create colour from gamma-encoded RGB (8-bit per channel, S44 version)
    ///
    /// Input RGB is assumed to be gamma-encoded VSF RGB colourspace
    #[cfg(feature = "spirix")]
    pub fn from_rgb8_s44(r: u8, g: u8, b: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8_s44(r);
        let g_lin = linearize_gamma2_u8_s44(g);
        let b_lin = linearize_gamma2_u8_s44(b);

        match format {
            ColourFormat::Ru => VsfType::ru([r, g, b]),
            ColourFormat::Rs => VsfType::rs([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
            ]),
            ColourFormat::Rf => VsfType::rf([r_lin.into(), g_lin.into(), b_lin.into()]),
            ColourFormat::Ri => VsfType::ri(pack_rgb_676_linear_s44(r_lin, g_lin, b_lin)),
            ColourFormat::Rp => VsfType::rp(pack_rgb_565_linear_s44(r_lin, g_lin, b_lin)),

            // RGB → Greyscale: Use VSF RGB photopic luminance
            ColourFormat::Re => {
                let lum = vsf_rgb_to_photopic_s44(r_lin, g_lin, b_lin);
                VsfType::re(delinearize_gamma2_u8_s44(lum))
            }
            ColourFormat::Rx => {
                let lum = vsf_rgb_to_photopic_s44(r_lin, g_lin, b_lin);
                VsfType::rx(delinearize_gamma2_u16_s44(lum))
            }
            ColourFormat::Rz => {
                let lum = vsf_rgb_to_photopic_s44(r_lin, g_lin, b_lin);
                VsfType::rz(lum.into())
            }
            ColourFormat::Rd => {
                let lum = vsf_rgb_to_photopic_s44(r_lin, g_lin, b_lin);
                VsfType::rd(lum)
            }
            ColourFormat::Rb => VsfType::rb([r_lin, g_lin, b_lin]),

            // RGBA formats → add opaque alpha
            ColourFormat::Ra => VsfType::ra([r, g, b, 255]),
            ColourFormat::Rt => VsfType::rt([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
                0xFFFF,
            ]),
            ColourFormat::Rh => VsfType::rh([r_lin.into(), g_lin.into(), b_lin.into(), 1.0]),
            ColourFormat::Rw => VsfType::rw([r_lin, g_lin, b_lin, S44::ONE]),
        }
    }

    /// Create colour from gamma-encoded RGBA (8-bit per channel, f32 version)
    ///
    /// Input RGBA is assumed to be gamma-encoded VSF RGB colourspace
    pub fn from_rgba8_f32(r: u8, g: u8, b: u8, a: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8_f32(r);
        let g_lin = linearize_gamma2_u8_f32(g);
        let b_lin = linearize_gamma2_u8_f32(b);
        let a_lin = a as f32 / 256.; // Alpha is linear, ×256 truncation

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
            _ => Self::from_rgb8_f32(r, g, b, format),
        }
    }

    /// Create colour from gamma-encoded RGBA (8-bit per channel, S44 version)
    ///
    /// Input RGBA is assumed to be gamma-encoded VSF RGB colourspace
    #[cfg(feature = "spirix")]
    pub fn from_rgba8_s44(r: u8, g: u8, b: u8, a: u8, format: ColourFormat) -> Self {
        // Linearize input
        let r_lin = linearize_gamma2_u8_s44(r);
        let g_lin = linearize_gamma2_u8_s44(g);
        let b_lin = linearize_gamma2_u8_s44(b);
        let a_lin = S44::from(a) >> 8isize; // Alpha is linear, ×256 truncation

        match format {
            ColourFormat::Ra => VsfType::ra([r, g, b, a]),
            ColourFormat::Rt => VsfType::rt([
                (r as u16) << 8 | r as u16,
                (g as u16) << 8 | g as u16,
                (b as u16) << 8 | b as u16,
                (a as u16) << 8 | a as u16,
            ]),
            ColourFormat::Rh => {
                VsfType::rh([r_lin.into(), g_lin.into(), b_lin.into(), a_lin.into()])
            }
            ColourFormat::Rw => VsfType::rw([r_lin, g_lin, b_lin, a_lin]),
            // For RGB-only formats, ignore alpha
            _ => Self::from_rgb8_s44(r, g, b, format),
        }
    }

    /// Convert this colour to any other format (f32 pipeline)
    pub fn convert_colour_f32(&self, target: ColourFormat) -> Option<Self> {
        // Get as linear RGBA (most general representation)
        let rgba = self.to_rgba_linear_f32()?;

        // Convert linear back to gamma u8 for from_rgba8_f32 (which expects gamma input)
        let r_gamma = delinearize_gamma2_u8_f32(rgba.r);
        let g_gamma = delinearize_gamma2_u8_f32(rgba.g);
        let b_gamma = delinearize_gamma2_u8_f32(rgba.b);
        let a_u8 = (rgba.a * 256.) as u8; // Alpha is linear, ×256 truncation

        // Convert to target format
        Some(Self::from_rgba8_f32(
            r_gamma, g_gamma, b_gamma, a_u8, target,
        ))
    }

    /// Convert this colour to any other format (S44 pipeline)
    #[cfg(feature = "spirix")]
    pub fn convert_colour_s44(&self, target: ColourFormat) -> Option<Self> {
        // Get as linear RGBA (most general representation)
        let rgba = self.to_rgba_linear_s44()?;

        // Convert linear back to gamma u8 for from_rgba8_s44 (which expects gamma input)
        let r_gamma = delinearize_gamma2_u8_s44(rgba.r);
        let g_gamma = delinearize_gamma2_u8_s44(rgba.g);
        let b_gamma = delinearize_gamma2_u8_s44(rgba.b);
        let a_u8 = (rgba.a << 8isize).to_u8();

        // Convert to target format
        Some(Self::from_rgba8_s44(
            r_gamma, g_gamma, b_gamma, a_u8, target,
        ))
    }
}

/// Target colour format for conversions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColourFormat {
    // Greyscale
    Re, // 8-bit
    Rx, // 16-bit
    Rz, // f32
    #[cfg(feature = "spirix")]
    Rd, // S44

    // Packed RGB
    Ri, // 6×7×6
    Rp, // 5-6-5

    // Standard RGB
    Ru, // 8-bit
    Rs, // 16-bit
    Rf, // f32
    #[cfg(feature = "spirix")]
    Rb, // S44

    // Standard RGBA
    Ra, // 8-bit
    Rt, // 16-bit
    Rh, // f32
    #[cfg(feature = "spirix")]
    Rw, // S44
}

/// Unpack 6×7×6 RGB from single byte to linear f32
fn unpack_rgb_676_linear_f32(packed: u8) -> (f32, f32, f32) {
    let b = packed % 6;
    let temp = packed / 6;
    let g = temp % 7;
    let r = temp / 7;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = r as f32 / 5.;
    let g_gamma = g as f32 / 6.;
    let b_gamma = b as f32 / 5.;

    (
        linearize_gamma2_f32(r_gamma),
        linearize_gamma2_f32(g_gamma),
        linearize_gamma2_f32(b_gamma),
    )
}

/// Pack linear RGB into 6×7×6 format (single byte)
fn pack_rgb_676_linear_f32(r: f32, g: f32, b: f32) -> u8 {
    // Delinearize to gamma, then quantize
    let r_gamma = delinearize_gamma2_f32(r);
    let g_gamma = delinearize_gamma2_f32(g);
    let b_gamma = delinearize_gamma2_f32(b);

    let r6 = (r_gamma * 5.).min(5.) as u8;
    let g7 = (g_gamma * 6.).min(6.) as u8;
    let b6 = (b_gamma * 5.).min(5.) as u8;

    ((r6 * 7) + g7) * 6 + b6
}

/// Unpack 5-6-5 RGB from u16 to linear f32
fn unpack_rgb_565_linear_f32(packed: u16) -> (f32, f32, f32) {
    let r5 = (packed >> 11) & 0x1F;
    let g6 = (packed >> 5) & 0x3F;
    let b5 = packed & 0x1F;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = r5 as f32 / 32.;
    let g_gamma = g6 as f32 / 64.;
    let b_gamma = b5 as f32 / 32.;

    (
        linearize_gamma2_f32(r_gamma),
        linearize_gamma2_f32(g_gamma),
        linearize_gamma2_f32(b_gamma),
    )
}

/// Pack linear RGB into 5-6-5 format (u16)
fn pack_rgb_565_linear_f32(r: f32, g: f32, b: f32) -> u16 {
    // Delinearize to gamma, then quantize
    let r_gamma = delinearize_gamma2_f32(r);
    let g_gamma = delinearize_gamma2_f32(g);
    let b_gamma = delinearize_gamma2_f32(b);

    let r5 = (r_gamma.min(1.) * 32.) as u16;
    let g6 = (g_gamma.min(1.) * 64.) as u16;
    let b5 = (b_gamma.min(1.) * 32.) as u16;

    (r5 << 11) | (g6 << 5) | b5
}

// ==================== PACKED RGB FUNCTIONS (S44) ====================

/// Unpack 6×7×6 RGB from single byte to linear S44
#[cfg(feature = "spirix")]
fn unpack_rgb_676_linear_s44(packed: u8) -> (S44, S44, S44) {
    let b = packed % 6;
    let temp = packed / 6;
    let g = temp % 7;
    let r = temp / 7;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = S44::from(r) / 5;
    let g_gamma = S44::from(g) / 6;
    let b_gamma = S44::from(b) / 5;

    (r_gamma * r_gamma, g_gamma * g_gamma, b_gamma * b_gamma)
}

/// Pack linear RGB into 6×7×6 format (single byte, S44 version)
#[cfg(feature = "spirix")]
fn pack_rgb_676_linear_s44(r: S44, g: S44, b: S44) -> u8 {
    // Delinearize to gamma, then quantize
    let r_gamma = r.sqrt();
    let g_gamma = g.sqrt();
    let b_gamma = b.sqrt();

    let r_scaled: S44 = r_gamma * 5;
    let g_scaled: S44 = g_gamma * 6;
    let b_scaled: S44 = b_gamma * 5;

    let r6: u8 = r_scaled.to_u8();
    let g7: u8 = g_scaled.to_u8();
    let b6: u8 = b_scaled.to_u8();

    ((r6 * 7) + g7) * 6 + b6
}

/// Unpack 5-6-5 RGB from u16 to linear S44
#[cfg(feature = "spirix")]
fn unpack_rgb_565_linear_s44(packed: u16) -> (S44, S44, S44) {
    let r5 = (packed >> 11) & 0x1F;
    let g6 = (packed >> 5) & 0x3F;
    let b5 = packed & 0x1F;

    // Normalize to 0-1 gamma-encoded, then linearize
    let r_gamma = S44::from(r5) >> 5;
    let g_gamma = S44::from(g6) >> 6;
    let b_gamma = S44::from(b5) >> 5;

    (r_gamma * r_gamma, g_gamma * g_gamma, b_gamma * b_gamma)
}

/// Pack linear RGB into 5-6-5 format (u16, S44 version)
#[cfg(feature = "spirix")]
fn pack_rgb_565_linear_s44(r: S44, g: S44, b: S44) -> u16 {
    // Delinearize to gamma, then quantize
    let r_gamma = r.sqrt();
    let g_gamma = g.sqrt();
    let b_gamma = b.sqrt();

    let r_clamped: S44 = r_gamma.min(1);
    let g_clamped: S44 = g_gamma.min(1);
    let b_clamped: S44 = b_gamma.min(1);

    let r_scaled: S44 = r_clamped * 32;
    let g_scaled: S44 = g_clamped * 64;
    let b_scaled: S44 = b_clamped * 32;

    let r5: u16 = r_scaled.to_u16();
    let g6: u16 = g_scaled.to_u16();
    let b5: u16 = b_scaled.to_u16();

    (r5 << 11) | (g6 << 5) | b5
}

// ==================== VSF RGB PHOTOPIC LUMINANCE ====================
// Convert VSF RGB to photopic luminance (perceptual brightness)
// Uses VSF RGB → LMS → Photopic transformation

/// Precomputed photopic white point normalization constant
///
/// Calculated from VSF_RGB2LMS and LMS2PHOTOPIC for white point [1,1,1]
const PHOTOPIC_WHITE_NORM: f32 = {
    // L white = sum of first row
    let l_white = VSF_RGB2LMS[0] + VSF_RGB2LMS[1] + VSF_RGB2LMS[2];
    // M white = sum of second row
    let m_white = VSF_RGB2LMS[3] + VSF_RGB2LMS[4] + VSF_RGB2LMS[5];
    // S white = sum of third row
    let s_white = VSF_RGB2LMS[6] + VSF_RGB2LMS[7] + VSF_RGB2LMS[8];

    // Photopic response for white point
    LMS2PHOTOPIC[0] * l_white + LMS2PHOTOPIC[1] * m_white + LMS2PHOTOPIC[2] * s_white
};

/// Convert linear VSF RGB to photopic luminance (0-1 range)
///
/// This performs colourimetric conversion:
/// 1. VSF RGB → LMS (cone responses using column-major matrix)
/// 2. LMS → Photopic luminance (weighted sum: 1.05L + 0.62M)
/// 3. Normalize so Illuminant E white [1,1,1] → 1.0
pub fn vsf_rgb_to_photopic_f32(r: f32, g: f32, b: f32) -> f32 {
    // VSF RGB → LMS (matrix in column-major order)
    let l = VSF_RGB2LMS[0] * r + VSF_RGB2LMS[1] * g + VSF_RGB2LMS[2] * b;
    let m = VSF_RGB2LMS[3] * r + VSF_RGB2LMS[4] * g + VSF_RGB2LMS[5] * b;
    let s = VSF_RGB2LMS[6] * r + VSF_RGB2LMS[7] * g + VSF_RGB2LMS[8] * b;

    // LMS → Photopic luminance (raw)
    let photopic_raw = LMS2PHOTOPIC[0] * l + LMS2PHOTOPIC[1] * m + LMS2PHOTOPIC[2] * s;

    // Normalize by precomputed white point so [1,1,1] → 1.0
    photopic_raw / PHOTOPIC_WHITE_NORM
}

/// Convert linear VSF RGB to photopic luminance (0-1 range, S44 version)
///
/// This performs colourimetric conversion:
/// 1. VSF RGB → LMS (cone responses using column-major matrix)
/// 2. LMS → Photopic luminance (weighted sum: 1.05L + 0.62M)
/// 3. Normalize so Illuminant E white [1,1,1] → 1.0
#[cfg(feature = "spirix")]
pub fn vsf_rgb_to_photopic_s44(r: S44, g: S44, b: S44) -> S44 {
    use crate::colour::{LMS2PHOTOPIC_S44, VSF_RGB2LMS_S44};
    const PHOTOPIC_WHITE_NORM_S44: S44 = sf!(PHOTOPIC_WHITE_NORM);

    let l = VSF_RGB2LMS_S44[0] * r + VSF_RGB2LMS_S44[1] * g + VSF_RGB2LMS_S44[2] * b;
    let m = VSF_RGB2LMS_S44[3] * r + VSF_RGB2LMS_S44[4] * g + VSF_RGB2LMS_S44[5] * b;
    let s = VSF_RGB2LMS_S44[6] * r + VSF_RGB2LMS_S44[7] * g + VSF_RGB2LMS_S44[8] * b;

    // LMS → Photopic luminance (raw)
    let photopic_raw = LMS2PHOTOPIC_S44[0] * l + LMS2PHOTOPIC_S44[1] * m + LMS2PHOTOPIC_S44[2] * s;

    // Normalize by precomputed white point so [1,1,1] → 1.0
    photopic_raw / PHOTOPIC_WHITE_NORM_S44
}

// ==================== GAMMA 2 FUNCTIONS ====================
// VSF RGB uses gamma 2 by default (simple sqrt/square operations)

/// Linearize a gamma 2 encoded value (0-1 range)
///
/// Converts from gamma-encoded to linear light. For VSF RGB gamma 2,
/// this is simply squaring the value.
pub fn linearize_gamma2_f32(encoded: f32) -> f32 {
    encoded * encoded
}

/// Delinearize a linear value to gamma 2 (0-1 range)
///
/// Converts from linear light to gamma-encoded. For VSF RGB gamma 2,
/// this is simply the square root operation.
pub fn delinearize_gamma2_f32(linear: f32) -> f32 {
    linear.sqrt()
}

/// Linearize an 8-bit gamma 2 encoded value
///
/// **VSF uses ×256 truncation for fast, symmetric quantization:**
/// - Division by 256 is a free bitshift operation (÷256 = >>8)
/// - Truncation is free (cast drops fractional bits)
/// - Creates uniform 1/256 buckets across entire [0,1] range
/// - Symmetric: encode and decode both use bucket bottoms
///
/// **Performance: 10-50× faster than sRGB's ×255 + rounding approach**
///
/// Converts 0-255 range to linear 0-1
pub fn linearize_gamma2_u8_f32(encoded: u8) -> f32 {
    let normalized = encoded as f32 / 256.;
    linearize_gamma2_f32(normalized)
}

/// Delinearize a linear value to 8-bit gamma 2
///
/// **VSF uses ×256 truncation for fast, symmetric quantization:**
/// - Multiplication by 256 allows free division via bitshift
/// - Truncation is free (cast drops fractional bits)
/// - Creates uniform 1/256 buckets across entire [0,1] range
/// - Symmetric: encode and decode both use bucket bottoms
///
/// **Performance: 10-50× faster than sRGB's ×255 + rounding approach**
///
/// Converts linear 0-1 to 0-255 range
pub fn delinearize_gamma2_u8_f32(linear: f32) -> u8 {
    let encoded = delinearize_gamma2_f32(linear);
    (encoded * 256.) as u8
}

/// Linearize a 16-bit gamma 2 encoded value
///
/// **VSF uses ×65536 truncation for fast, symmetric quantization:**
/// - Division by 65536 is a free bitshift operation (÷65536 = >>16)
/// - Truncation is free (cast drops fractional bits)
/// - Creates uniform 1/65536 buckets across entire [0,1] range
/// - Symmetric: encode and decode both use bucket bottoms
///
/// Converts 0-65535 range to linear 0-1
pub fn linearize_gamma2_u16_f32(encoded: u16) -> f32 {
    let normalized = encoded as f32 / 65536.;
    linearize_gamma2_f32(normalized)
}

/// Delinearize a linear value to 16-bit gamma 2
///
/// **VSF uses ×65536 truncation for fast, symmetric quantization:**
/// - Multiplication by 65536 allows free division via bitshift
/// - Truncation is free (cast drops fractional bits)
/// - Creates uniform 1/65536 buckets across entire [0,1] range
/// - Symmetric: encode and decode both use bucket bottoms
///
/// Converts linear 0.0-1.0 to 0-65535 range
pub fn delinearize_gamma2_u16_f32(linear: f32) -> u16 {
    let encoded = delinearize_gamma2_f32(linear);
    (encoded * 65536.) as u16
}

/// Linearize an RGB triple (8-bit per channel)
pub fn linearize_gamma2_rgb_f32(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    (
        linearize_gamma2_u8_f32(r),
        linearize_gamma2_u8_f32(g),
        linearize_gamma2_u8_f32(b),
    )
}

/// Delinearize a linear RGB triple to 8-bit
pub fn delinearize_gamma2_rgb_f32(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    (
        delinearize_gamma2_u8_f32(r),
        delinearize_gamma2_u8_f32(g),
        delinearize_gamma2_u8_f32(b),
    )
}

// ==================== GAMMA 2 FUNCTIONS (S44) ====================

/// Linearize a gamma 2 encoded value (0-1 range, S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn linearize_gamma2_s44(encoded: S44) -> S44 {
    encoded.square()
}

/// Delinearize a linear value to gamma 2 (0-1 range, S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn delinearize_gamma2_s44(linear: S44) -> S44 {
    linear.sqrt()
}

/// Linearize an 8-bit gamma 2 encoded value (S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn linearize_gamma2_u8_s44(encoded: u8) -> S44 {
    let normalized = S44::from(encoded) >> 8isize;
    normalized.square()
}

/// Delinearize a linear value to 8-bit gamma 2 (S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn delinearize_gamma2_u8_s44(linear: S44) -> u8 {
    let encoded = linear.sqrt();
    let scaled = encoded << 8isize;
    scaled.to_u8()
}

/// Linearize a 16-bit gamma 2 encoded value (S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn linearize_gamma2_u16_s44(encoded: u16) -> S44 {
    let normalized = S44::from(encoded) >> 16isize;
    normalized.square()
}

/// Delinearize a linear value to 16-bit gamma 2 (S44 version)
#[cfg(feature = "spirix")]
#[inline]
pub fn delinearize_gamma2_u16_s44(linear: S44) -> u16 {
    let encoded = linear.sqrt();
    let scaled: S44 = encoded << 16;
    scaled.to_u16()
}

/// Linearize an RGB triple (8-bit per channel, S44 version)
#[cfg(feature = "spirix")]
pub fn linearize_gamma2_rgb_s44(r: u8, g: u8, b: u8) -> (S44, S44, S44) {
    (
        linearize_gamma2_u8_s44(r),
        linearize_gamma2_u8_s44(g),
        linearize_gamma2_u8_s44(b),
    )
}

/// Delinearize a linear RGB triple to 8-bit (S44 version)
#[cfg(feature = "spirix")]
pub fn delinearize_gamma2_rgb_s44(r: S44, g: S44, b: S44) -> (u8, u8, u8) {
    (
        delinearize_gamma2_u8_s44(r),
        delinearize_gamma2_u8_s44(g),
        delinearize_gamma2_u8_s44(b),
    )
}

/// Apply sRGB OETF (gamma encoding) using S44 arithmetic
///
/// Converts linear sRGB values to gamma-encoded sRGB using the piecewise sRGB transfer function.
/// This is the S44 version for use in pure S44 pipelines without IEEE-754 floats.
///
/// # Arguments
/// * `linear` - Linear sRGB value in range [0.0, 1.0]
///
/// # Returns
/// Gamma-encoded sRGB value in range [0.0, 1.0]
#[cfg(feature = "spirix")]
#[inline]
pub fn srgb_oetf_s44(linear: S44) -> S44 {
    const THRESH: S44 = sf!(0.0031308);
    const A: S44 = sf!(12.92);
    const B: S44 = sf!(1.055);
    const GAMMA: S44 = sf!(1.0 / 2.4);
    const C: S44 = sf!(0.055);
    if linear <= THRESH {
        A * linear
    } else {
        B * linear.pow(GAMMA) - C
    }
}

/// Apply BT.709/Rec.709 OETF (gamma encoding) using S44 arithmetic
///
/// Converts linear BT.709 values to gamma-encoded using the piecewise BT.709 transfer function.
/// This is the S44 version for use in pure S44 pipelines without IEEE-754 floats.
///
/// # Arguments
/// * `linear` - Linear BT.709 value in range [0.0, 1.0]
///
/// # Returns
/// Gamma-encoded BT.709 value in range [0.0, 1.0]
#[cfg(feature = "spirix")]
#[inline]
pub fn encode_bt709_s44(linear: S44) -> S44 {
    const THRESH: S44 = sf!(0.018);
    const A: S44 = sf!(4.5);
    const B: S44 = sf!(1.099);
    const GAMMA: S44 = sf!(0.45);
    const C: S44 = sf!(0.099);
    if linear < THRESH {
        linear * A
    } else {
        B * linear.pow(GAMMA) - C
    }
}

/// Apply Adobe RGB OETF (gamma encoding) using S44 arithmetic
///
/// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts linear Adobe RGB values to gamma-encoded using simple gamma 2.2.
/// This is the S44 version for use in pure S44 pipelines without IEEE-754 floats.
///
/// # Arguments
/// * `linear` - Linear Adobe RGB value in range [0.0, 1.0]
///
/// # Returns
/// Gamma-encoded Adobe RGB value in range [0.0, 1.0]
#[cfg(feature = "spirix")]
#[inline]
pub fn adobe_rgb_oetf_s44(linear: S44) -> S44 {
    const GAMMA: S44 = sf!(1.0 / 2.2);
    linear.pow(GAMMA)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(deprecated)]
    use crate::colour::legacy::{delinearize_srgb_u8, linearize_srgb};

    #[test]
    fn test_named_shortcuts_to_rgb() {
        assert_eq!(
            VsfType::rck.to_rgb_linear_f32(),
            Some(RgbLinearF32 {
                r: 0.,
                g: 0.,
                b: 0.
            })
        );
        assert_eq!(
            VsfType::rcw.to_rgb_linear_f32(),
            Some(RgbLinearF32 {
                r: 1.,
                g: 1.,
                b: 1.
            })
        );
        assert_eq!(
            VsfType::rcr.to_rgb_linear_f32(),
            Some(RgbLinearF32 {
                r: 1.,
                g: 0.,
                b: 0.
            })
        );
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
            let vsf = VsfType::from_srgb_f32(r_in, g_in, b_in, ColourFormat::Rf);

            // VSF RGB → sRGB
            if let Some((r_out, g_out, b_out)) = vsf.to_srgb_u8_f32() {
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
        let packed = VsfType::from_rgb8_f32(
            original_u8.0,
            original_u8.1,
            original_u8.2,
            ColourFormat::Ri,
        );
        let unpacked = packed.to_rgb_linear_f32().unwrap();

        // Convert back to u8 for comparison
        let unpacked_u8 = (
            delinearize_gamma2_u8_f32(unpacked.r),
            delinearize_gamma2_u8_f32(unpacked.g),
            delinearize_gamma2_u8_f32(unpacked.b),
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
        let red_rgba = red_rgb.convert_colour_f32(ColourFormat::Ra).unwrap();

        let result = red_rgba.to_rgba_linear_f32().unwrap();
        // VSF uses ×256 quantization, so 255 → (255/256)² ≈ 0.998
        assert!((result.r - 1.0).abs() < 0.01, "r={}", result.r);
        assert!(result.g < 0.01, "g={}", result.g);
        assert!(result.b < 0.01, "b={}", result.b);
        assert!((result.a - 1.0).abs() < 0.01, "a={}", result.a);
    }

    #[test]
    fn test_gamma2_roundtrip() {
        let values = [0., 0.25, 0.5, 0.75, 1.];
        for &v in &values {
            let delinearized = delinearize_gamma2_f32(v);
            let linearized = linearize_gamma2_f32(delinearized);
            // Use epsilon comparison for floating point
            assert!(
                (linearized - v).abs() < 1e-7,
                "v={} → {} → {} (diff={})",
                v,
                delinearized,
                linearized,
                (linearized - v).abs()
            );
        }
    }

    #[test]
    fn test_gamma2_u8_roundtrip() {
        let values = [0u8, 64, 128, 192, 255];
        for &v in &values {
            let linearized = linearize_gamma2_u8_f32(v);
            let delinearized = delinearize_gamma2_u8_f32(linearized);
            // ×256 truncation has roundtrip asymmetry - allow ±1 error
            assert!(
                (delinearized as i16 - v as i16).abs() <= 1,
                "Value {} → {} → {} (diff={})",
                v,
                linearized,
                delinearized,
                (delinearized as i16 - v as i16)
            );
        }
    }

    #[test]
    fn test_bt2020_conversion() {
        // Test BT.2020 white (studio range [16,235]) → VSF RGB
        let bt2020_white = VsfType::from_rec2020_f32(235u8, 235u8, 235u8, ColourFormat::Ru);
        let vsf_white = bt2020_white.to_rgb_linear_f32().unwrap();

        // D65 white → E white adaptation results in slightly shifted values
        // This is expected - D65 is bluer than E, so we expect non-uniform RGB
        // After chromatic adaptation and studio range normalization
        assert!(vsf_white.r > 0.8 && vsf_white.r < 0.9, "r={}", vsf_white.r);
        assert!(vsf_white.g > 0.9 && vsf_white.g <= 1.0, "g={}", vsf_white.g);
        assert!(vsf_white.b > 0.9 && vsf_white.b <= 1.0, "b={}", vsf_white.b);

        // Test BT.2020 black (studio range [16,235]) → VSF RGB black
        let bt2020_black = VsfType::from_rec2020_f32(16u8, 16u8, 16u8, ColourFormat::Ru);
        let vsf_black = bt2020_black.to_rgb_linear_f32().unwrap();

        assert!(vsf_black.r < 0.01);
        assert!(vsf_black.g < 0.01);
        assert!(vsf_black.b < 0.01);

        // Test BT.2020 primary red (studio range)
        let bt2020_red = VsfType::from_rec2020_f32(235u8, 16u8, 16u8, ColourFormat::Ru);
        let vsf_red = bt2020_red.to_rgb_linear_f32().unwrap();
        // Red should stay mostly red
        assert!(vsf_red.r > vsf_red.g && vsf_red.r > vsf_red.b);
    }

    #[test]
    #[cfg(feature = "spirix")]
    fn test_scalar_f4e4_conversions() {
        use super::*;
        use spirix::S44;

        // Test u8 → S44 conversion (direct, no f32 roundtrip)
        let u8_val = 128u8;
        let s44_linear: S44 = <u8 as ColourValue<S44>>::to_linear_srgb(u8_val);

        // Verify it's approximately in the right range (128/255 with sRGB curve ≈ 0.2)
        let s44_f32 = s44_linear.to_f32();
        assert!(s44_f32 > 0.15 && s44_f32 < 0.25, "s44_linear = {}", s44_f32);

        // Test roundtrip: u8 → S44 → u8
        let roundtrip = <u8 as ColourValue<S44>>::from_linear_srgb(s44_linear);
        assert!(
            (roundtrip as i16 - u8_val as i16).abs() <= 2,
            "Roundtrip failed: {} → {} → {}",
            u8_val,
            s44_f32,
            roundtrip
        );

        // Test that S44 passthru works
        let s44_val = S44::from(0.5);
        let s44_out: S44 = <S44 as ColourValue<S44>>::to_linear_srgb(s44_val);
        assert!((s44_out.to_f32() - 0.5).abs() < 0.001, "Passthru failed");

        // Test gamma2 conversions with S44
        let u8_gamma = 180u8;
        let s44_gamma2: S44 = <u8 as ColourValue<S44>>::to_linear_gamma2(u8_gamma);
        let u8_back = <u8 as ColourValue<S44>>::from_linear_gamma2(s44_gamma2);
        assert!(
            (u8_back as i16 - u8_gamma as i16).abs() <= 1,
            "Gamma2 roundtrip failed: {} → {} → {}",
            u8_gamma,
            s44_gamma2.to_f32(),
            u8_back
        );
    }
}
