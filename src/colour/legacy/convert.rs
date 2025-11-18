//! Legacy colour conversion helper functions for CIE 1931 xy-based colourspaces
//!
//! **DEPRECATED**: These colourspaces are based on CIE 1931 xy chromaticity coordinates and are permanently tied to the 1931 Standard Observer. Use spectral colourspaces (VSF RGB, Rec.2020) instead for future-proof colour workflows.

// ==================== BT.709/Rec.709 CONSTANTS AND FUNCTIONS ====================

const BT709_LINEAR_THRESHOLD: f32 = 0.018;
const BT709_LINEAR_COEFF: f32 = 4.5;
const BT709_GAMMA_A: f32 = 0.099;
const BT709_GAMMA_DIVISOR: f32 = 1.099;
const BT709_GAMMA_EXPONENT: f32 = 0.45; // Inverse is ~2.222

/// Linearize a BT.709-encoded value (0-1 range)
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
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
        ((encoded + BT709_GAMMA_A) / BT709_GAMMA_DIVISOR).powf(1. / BT709_GAMMA_EXPONENT)
    }
}

/// Encode a linear value to BT.709
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
#[inline]
pub fn encode_bt709(linear: f32) -> f32 {
    if linear < BT709_LINEAR_THRESHOLD {
        linear * BT709_LINEAR_COEFF
    } else {
        BT709_GAMMA_DIVISOR * linear.powf(BT709_GAMMA_EXPONENT) - BT709_GAMMA_A
    }
}

// ==================== sRGB CONSTANTS AND FUNCTIONS ====================

const SRGB_LINEAR_THRESHOLD: f32 = 0.0031308;
const SRGB_LINEAR_COEFF: f32 = 12.92;
const SRGB_GAMMA_A: f32 = 0.055;
const SRGB_GAMMA_DIVISOR: f32 = 1.055;
const SRGB_GAMMA_EXPONENT: f32 = 2.4;

/// Linearize an sRGB-encoded value (0-1 range) using the specification piecewise function
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
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

/// Delinearize a linear value to sRGB-encoded (0-1 range) using the specification piecewise function
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Inverse sRGB transfer function:
/// - Linear: `C_linear * 12.92` for `C_linear <= 0.0031308`
/// - Gamma: `1.055 * C_linear^(1/2.4) - 0.055` for `C_linear > 0.0031308`
#[inline]
pub fn delinearize_srgb(linear: f32) -> f32 {
    if linear <= SRGB_LINEAR_THRESHOLD {
        linear * SRGB_LINEAR_COEFF
    } else {
        SRGB_GAMMA_DIVISOR * linear.powf(1. / SRGB_GAMMA_EXPONENT) - SRGB_GAMMA_A
    }
}

// ==================== sRGB u8/u16 QUANTIZATION FUNCTIONS ====================

/// Linearize an 8-bit sRGB-encoded value (spec-compliant, asymmetric)
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts 0-255 sRGB to linear 0-1 using the sRGB specification.
///
/// **WARNING: This follows the sRGB spec which is asymmetric:**
/// - Encoding uses ×255 + round (bucket centers)
/// - Decoding uses ÷255 (bucket bottoms)
///
/// This asymmetry means encode→decode roundtrip has inherent error.
#[inline]
pub fn linearize_srgb_u8(encoded: u8) -> f32 {
    let normalized = encoded as f32 / 255.;
    linearize_srgb(normalized)
}

/// Delinearize a linear value to 8-bit sRGB (spec-compliant, asymmetric)
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts linear 0-1 to 0-255 sRGB using the sRGB specification.
///
/// **WARNING: This follows the sRGB spec which is asymmetric:**
/// - Encoding uses ×255 + round (bucket centers) ← YOU ARE HERE
/// - Decoding uses ÷255 (bucket bottoms)
///
/// Note the expensive `.round()` call which costs 5-30 cycles.
#[inline]
pub fn delinearize_srgb_u8(linear: f32) -> u8 {
    let encoded = delinearize_srgb(linear);
    (encoded * 255.).round() as u8
}

/// Linearize a 16-bit sRGB-encoded value (spec-compliant, asymmetric)
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts 0-65535 sRGB to linear 0-1 using the sRGB specification.
///
/// **WARNING: This follows the sRGB spec which is slow and asymmetric:**
/// - Encoding uses ×65535 + round (bucket centers)
/// - Decoding uses ÷65535 (bucket bottoms, actual division!) ← YOU ARE HERE
#[inline]
pub fn linearize_srgb_u16(encoded: u16) -> f32 {
    let normalized = encoded as f32 / 65535.;
    linearize_srgb(normalized)
}

/// Delinearize a linear value to 16-bit sRGB (spec-compliant, asymmetric)
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts linear 0-1 to 0-65535 sRGB using the sRGB specification.
///
/// **WARNING: This follows the sRGB spec which is asymmetric:**
/// - Encoding uses ×65535 + round (bucket centers) ← YOU ARE HERE
/// - Decoding uses ÷65535 (bucket bottoms)
///
/// Note the expensive `.round()` call which costs 5-30 cycles.
#[inline]
pub fn delinearize_srgb_u16(linear: f32) -> u16 {
    let encoded = delinearize_srgb(linear);
    (encoded * 65535.).round() as u16
}

// ==================== BT.709/BT.2020 STUDIO RANGE FUNCTIONS ====================

/// Linearize an 8-bit BT.709-encoded value (studio range: 16-235)
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts studio range 16-235 to linear 0-1 using BT.709 transfer function.
///
/// **Studio range mapping:**
/// - 16 (video black) → 0.0 linear
/// - 235 (video white) → 1.0 linear
/// - Values outside [16, 235] are clamped
#[inline]
pub fn linearize_bt709_u8(encoded: u8) -> f32 {
    let normalized = (encoded as f32 - 16.) / 219.;
    linearize_bt709(normalized.clamp(0., 1.))
}

/// Encode a linear value to 8-bit BT.709 (studio range: 16-235)
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts linear 0-1 to studio range 16-235 using BT.709 transfer function.
///
/// **Studio range mapping:**
/// - 0.0 linear → 16 (video black)
/// - 1.0 linear → 235 (video white)
/// - Output clamped to [16, 235]
#[inline]
pub fn encode_bt709_u8(linear: f32) -> u8 {
    (encode_bt709(linear) * 219. + 16.).round().clamp(16., 235.) as u8
}

/// Linearize a 16-bit BT.709-encoded value (studio range: 4096-60160)
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts studio range 4096-60160 to linear 0-1 using BT.709 transfer function.
///
/// **Studio range mapping:**
/// - 4096 (video black) → 0.0 linear
/// - 60160 (video white) → 1.0 linear
/// - Values outside [4096, 60160] are clamped
#[inline]
pub fn linearize_bt709_u16(encoded: u16) -> f32 {
    let normalized = (encoded as f32 - 4096.) / 56064.;
    linearize_bt709(normalized.clamp(0., 1.))
}

/// Encode a linear value to 16-bit BT.709 (studio range: 4096-60160)
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Converts linear 0-1 to studio range 4096-60160 using BT.709 transfer function.
///
/// **Studio range mapping:**
/// - 0.0 linear → 4096 (video black)
/// - 1.0 linear → 60160 (video white)
/// - Output clamped to [4096, 60160]
#[inline]
pub fn encode_bt709_u16(linear: f32) -> u16 {
    (encode_bt709(linear) * 56064. + 4096.)
        .round()
        .clamp(4096., 60160.) as u16
}

// ==================== sRGB RGB TRIPLE HELPERS ====================

/// Linearize an sRGB RGB triple (8-bit per channel)
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
#[inline]
pub fn linearize_srgb_rgb(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    (
        linearize_srgb_u8(r),
        linearize_srgb_u8(g),
        linearize_srgb_u8(b),
    )
}

/// Delinearize a linear RGB triple to 8-bit sRGB
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
#[inline]
pub fn delinearize_srgb_rgb(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
    (
        delinearize_srgb_u8(r),
        delinearize_srgb_u8(g),
        delinearize_srgb_u8(b),
    )
}
