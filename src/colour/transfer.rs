//! Transfer functions for modern spectral colourspaces
//!
//! **Legacy transfer functions are in `src/colour/legacy/transfer.rs`**
//!
//! This module provides transfer functions for spectral colourspaces:
//! - **VSF RGB**: Simple gamma 2 (see `src/colour/convert.rs`)
//! - **Rec.2020**: Uses Rec.709 transfer function (aliases provided below)
//!
//! For legacy xy-based colourspaces (sRGB, Rec.709, Adobe RGB), use:
//! ```rust
//! use vsf::colour::legacy::transfer::*;
//! ```

// ==================== Rec.2020 TRANSFER FUNCTIONS ====================
//
// Rec.2020 is a spectral colourspace (primaries: 630nm, 532nm, 467nm) but it uses the Rec.709 transfer function for backwards compatibility.

/// Rec.2020 EOTF: Convert gamma-corrected Rec.2020 to linear RGB
///
/// Note: Rec.2020 uses the Rec.709 transfer function. Rec.2020 is a spectral colourspace, not a legacy xy-based colourspace.
pub use crate::colour::legacy::transfer::rec709_eotf as rec2020_eotf;

/// Apply Rec.2020 EOTF to RGB triplet
pub use crate::colour::legacy::transfer::rec709_eotf_rgb as rec2020_eotf_rgb;

/// Rec.2020 OETF: Convert linear RGB to gamma-corrected Rec.2020
///
/// Note: Rec.2020 uses the Rec.709 transfer function. Rec.2020 is a spectral colourspace, not a legacy xy-based colourspace.
pub use crate::colour::legacy::transfer::rec709_oetf as rec2020_oetf;

/// Apply Rec.2020 OETF to RGB triplet
pub use crate::colour::legacy::transfer::rec709_oetf_rgb as rec2020_oetf_rgb;

// ==================== BT.2100 HDR SIGNALS ====================
//
// PQ and HLG signals decoded to light relative to diffuse white, the unit VSF works in (colour.md: linear 1.0 is diffuse white). BT.2408 places diffuse white at 203 cd/m² for PQ and at 75 % of the HLG signal. Light above white is kept, so a `g2t` encode or a linear plane with white below its top count can hold it.

/// PQ (SMPTE ST 2084) signal, 0 to 1, → light relative to diffuse white (203 cd/m², BT.2408). The signal's top, 10 000 cd/m², is 49.3× diffuse white.
pub fn pq_light(signal: f32) -> f32 {
    const M1: f32 = 2610. / 16384.;
    const M2: f32 = 2523. / 4096. * 128.;
    const C1: f32 = 3424. / 4096.;
    const C2: f32 = 2413. / 4096. * 32.;
    const C3: f32 = 2392. / 4096. * 32.;
    let p = signal.clamp(0., 1.).powf(1. / M2);
    let nits = ((p - C1).max(0.) / (C2 - C3 * p)).powf(1. / M1) * 10_000.;
    nits / 203.
}

/// HLG (BT.2100) signal, 0 to 1, → scene light relative to diffuse white (75 % of the signal, BT.2408; scene light 0.264 97). The signal's top is 3.77× diffuse white.
pub fn hlg_light(signal: f32) -> f32 {
    const A: f32 = 0.178_832_77;
    const B: f32 = 0.284_668_92;
    const C: f32 = 0.559_910_73;
    const WHITE: f32 = 0.264_972;
    let e = signal.clamp(0., 1.);
    let scene = if e <= 0.5 { e * e / 3. } else { (((e - C) / A).exp() + B) / 12. };
    scene / WHITE
}

#[cfg(test)]
mod hdr_signal_tests {
    use super::*;

    #[test]
    fn bt2408_diffuse_white_is_one() {
        assert!((hlg_light(0.75) - 1.).abs() < 1e-3);
        assert!((pq_light(0.580_69) - 1.).abs() < 2e-3, "203 cd/m² is PQ 0.58069");
        assert!((pq_light(1.) - 10_000. / 203.).abs() < 0.1);
        assert!((hlg_light(1.) - 1. / 0.264_972).abs() < 0.01);
        assert_eq!(pq_light(0.), 0.);
        assert_eq!(hlg_light(0.), 0.);
    }
}
