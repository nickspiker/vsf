//! Legacy transfer functions for CIE 1931 xy-based colourspaces
//!
//! **DEPRECATED**: These colourspaces are based on CIE 1931 xy chromaticity coordinates and are permanently tied to the 1931 Standard Observer. Use spectral colourspaces (VSF RGB, Rec.2020) instead for future-proof colour workflows.
//!
//! Transfer functions (OETF/EOTF) convert between linear light and gamma-corrected values.
//! - **OETF** (Opto-Electronic Transfer Function): linear → gamma-corrected (encoding)
//! - **EOTF** (Electro-Optical Transfer Function): gamma-corrected → linear (decoding)

use num_traits::Float;

// ==================== sRGB TRANSFER FUNCTIONS ====================

/// sRGB OETF: Convert linear RGB to gamma-corrected sRGB
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Piecewise function:
/// - Linear segment for dark values (≤ 0.0031308)
/// - 2.4 gamma for bright values (> 0.0031308)
///
/// # Arguments
/// * `linear` - Linear RGB value in range [0.0, 1.0]
///
/// # Returns
/// Gamma-corrected sRGB value in range [0.0, 1.0]
pub fn srgb_oetf(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB EOTF: Convert gamma-corrected sRGB to linear RGB
///
/// **DEPRECATED**: sRGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Inverse of sRGB OETF.
///
/// # Arguments
/// * `encoded` - Gamma-corrected sRGB value in range [0.0, 1.0]
///
/// # Returns
/// Linear RGB value in range [0.0, 1.0]
pub fn srgb_eotf(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// Apply sRGB OETF to RGB triplet
#[inline]
pub fn srgb_oetf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [srgb_oetf(rgb[0]), srgb_oetf(rgb[1]), srgb_oetf(rgb[2])]
}

/// Apply sRGB EOTF to RGB triplet
#[inline]
pub fn srgb_eotf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [srgb_eotf(rgb[0]), srgb_eotf(rgb[1]), srgb_eotf(rgb[2])]
}

// ==================== Rec.709/Rec.2020 TRANSFER FUNCTIONS ====================

/// Rec.709 OETF: Convert linear RGB to gamma-corrected Rec.709/Rec.2020
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy). Note: Rec.2020 uses the same transfer function as Rec.709.
///
/// Piecewise function with parameters:
/// - α = 1.099
/// - β = 0.018
///
/// # Arguments
/// * `linear` - Linear RGB value in range [0.0, 1.0]
///
/// # Returns
/// Gamma-corrected value in range [0.0, 1.0]
pub fn rec709_oetf(linear: f32) -> f32 {
    const ALPHA: f32 = 1.099;
    const BETA: f32 = 0.018;

    if linear < BETA {
        4.5 * linear
    } else {
        ALPHA * linear.powf(0.45) - (ALPHA - 1.0)
    }
}

/// Rec.709 EOTF: Convert gamma-corrected Rec.709/Rec.2020 to linear RGB
///
/// **DEPRECATED**: Rec.709 primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Inverse of Rec.709 OETF.
///
/// # Arguments
/// * `encoded` - Gamma-corrected value in range [0.0, 1.0]
///
/// # Returns
/// Linear RGB value in range [0.0, 1.0]
pub fn rec709_eotf(encoded: f32) -> f32 {
    const ALPHA: f32 = 1.099;
    const BETA: f32 = 0.018;
    const THRESHOLD: f32 = 4.5 * BETA; // 0.081

    if encoded < THRESHOLD {
        encoded / 4.5
    } else {
        ((encoded + (ALPHA - 1.0)) / ALPHA).powf(1.0 / 0.45)
    }
}

/// Apply Rec.709 OETF to RGB triplet
#[inline]
pub fn rec709_oetf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [
        rec709_oetf(rgb[0]),
        rec709_oetf(rgb[1]),
        rec709_oetf(rgb[2]),
    ]
}

/// Apply Rec.709 EOTF to RGB triplet
#[inline]
pub fn rec709_eotf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [
        rec709_eotf(rgb[0]),
        rec709_eotf(rgb[1]),
        rec709_eotf(rgb[2]),
    ]
}

// ==================== Adobe RGB TRANSFER FUNCTIONS ====================

/// Adobe RGB (1998) OETF: Convert linear RGB to gamma mapped Adobe RGB
///
/// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// **Spec**: Adobe RGB (1998) Colour Image Encoding, Version 2005-05
/// - Gamma: **2.2 exactly** (simple power function, no linear segment)
/// - Some implementations use 563/256 ≈ 2.19921875 however spec says 2.2
///
/// See `src/colour/legacy/constants.rs` for transformation matrices and deprecation notes.
///
/// # Arguments
/// * `linear` - Linear RGB value in range [0, 1]
///
/// # Returns
/// Gamma mapped value in range [0, 1]
#[inline]
pub fn adobe_rgb_oetf(linear: f32) -> f32 {
    linear.powf(1. / 2.2)
}

/// Adobe RGB (1998) EOTF: Convert gamma mapped Adobe RGB to linear RGB
///
/// **DEPRECATED**: Adobe RGB primaries are based on CIE 1931 xy chromaticity (legacy).
///
/// Inverse of Adobe RGB OETF.
///
/// # Arguments
/// * `encoded` - Gamma mapped value in range [0, 1]
///
/// # Returns
/// Linear RGB value in range [0, 1]
#[inline]
pub fn adobe_rgb_eotf(encoded: f32) -> f32 {
    encoded.powf(2.2)
}

/// Apply Adobe RGB OETF to RGB triplet
#[inline]
pub fn adobe_rgb_oetf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [
        adobe_rgb_oetf(rgb[0]),
        adobe_rgb_oetf(rgb[1]),
        adobe_rgb_oetf(rgb[2]),
    ]
}

/// Apply Adobe RGB EOTF to RGB triplet
#[inline]
pub fn adobe_rgb_eotf_rgb(rgb: &[f32; 3]) -> [f32; 3] {
    [
        adobe_rgb_eotf(rgb[0]),
        adobe_rgb_eotf(rgb[1]),
        adobe_rgb_eotf(rgb[2]),
    ]
}

// Note: Rec.2020 transfer function aliases have been moved to src/colour/transfer.rs Rec.2020 is a spectral colourspace (NOT legacy), but it reuses Rec.709 transfer functions

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_roundtrip() {
        let linear_values = [0.0, 0.001, 0.01, 0.1, 0.5, 0.9, 1.0];

        for &linear in &linear_values {
            let encoded = srgb_oetf(linear);
            let decoded = srgb_eotf(encoded);
            assert!(
                (linear - decoded).abs() < 1e-6,
                "sRGB roundtrip failed for {}: got {}",
                linear,
                decoded
            );
        }
    }

    #[test]
    fn test_rec709_roundtrip() {
        let linear_values = [0.0, 0.001, 0.01, 0.1, 0.5, 0.9, 1.0];

        for &linear in &linear_values {
            let encoded = rec709_oetf(linear);
            let decoded = rec709_eotf(encoded);
            assert!(
                (linear - decoded).abs() < 1e-6,
                "Rec.709 roundtrip failed for {}: got {}",
                linear,
                decoded
            );
        }
    }

    #[test]
    fn test_srgb_known_values() {
        // Black
        assert_eq!(srgb_oetf(0.0), 0.0);
        assert_eq!(srgb_eotf(0.0), 0.0);

        // White - use tolerance due to floating point precision
        assert!((srgb_oetf(1.0) - 1.0).abs() < 1e-6);
        assert!((srgb_eotf(1.0) - 1.0).abs() < 1e-6);

        // Mid-gray (linear 0.5 → ~0.735 sRGB)
        let encoded = srgb_oetf(0.5);
        assert!((encoded - 0.735357).abs() < 1e-5);
    }

    #[test]
    fn test_rec709_known_values() {
        // Black
        assert_eq!(rec709_oetf(0.0), 0.0);
        assert_eq!(rec709_eotf(0.0), 0.0);

        // White
        assert_eq!(rec709_oetf(1.0), 1.0);
        assert_eq!(rec709_eotf(1.0), 1.0);
    }

    #[test]
    fn test_srgb_vs_rec709_difference() {
        // The transfer functions should differ for mid-range values
        let linear = 0.5;
        let srgb_encoded = srgb_oetf(linear);
        let rec709_encoded = rec709_oetf(linear);

        // They should be different but close
        assert!((srgb_encoded - rec709_encoded).abs() > 0.0001);
        assert!((srgb_encoded - rec709_encoded).abs() < 0.1);
    }
}
