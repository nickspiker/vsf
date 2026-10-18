//! Display mapping: scene light → display light.
//!
//! Storage and display are separate stages. An encoding (`gamma2`, `g2t`, `linear`) says what scene light a code holds, with diffuse white at 1.0; it never changes with the screen. This module says how one particular display shows that light, given its headroom: how many times brighter than diffuse white its peak is (1 for an SDR monitor, the HDR/SDR ratio on an HDR panel). Every app calls it as the last step before its screen, so no two viewers disagree, and nothing here is ever written back into a file (colour.md, "Display Mapping").
//!
//! The mapping is linear up to a knee and rolls off above it, reaching the display's peak exactly at the content's brightest value. Content that fits the display (`content_peak ≤ headroom`) is shown linearly, untouched: SDR content on any display, and HDR content on a display with room for it.

use crate::colour::convert::{linearize_gamma2_u8_f32, linearize_gamma2tanh_bits_f32};
use crate::visual::Encoding;

/// Where the roll-off starts, as a fraction of the display's peak, when the content is brighter than the display can show. 0.75 keeps the lower three quarters of the display's range linear; on an SDR monitor (headroom 1) diffuse white then lands near 0.88 of peak, leaving the top eighth for the highlights.
pub const DISPLAY_KNEE: f32 = 0.75;

/// The brightest scene light a `g2t` code of `bits` depth can hold: its top code, decoded (9.6× diffuse white at 8 bits, 34.6× at 16).
pub fn g2t_peak(bits: u32) -> f32 {
    linearize_gamma2tanh_bits_f32((1u32 << bits) - 1, bits)
}

/// Scene light `l` (diffuse white = 1.0) shown on a display whose peak is `headroom` × diffuse white, for content whose brightest value is `content_peak`. Returns display light as a fraction of the display's peak, 0.0 to 1.0, linear; the caller applies the display's own transfer (sRGB, PQ, a panel's measured power law).
///
/// When the content fits (`content_peak ≤ headroom`) this is `l / headroom`, exactly linear. Otherwise it is linear up to the knee at `DISPLAY_KNEE × headroom` and an extended-Reinhard shoulder above it: slope-continuous at the knee, monotone, and reaching exactly 1.0 at `content_peak`, so nothing in the content clips and nothing brighter than its peak is assumed. Headroom below 1 is taken as 1; NaN and negative light show as 0.
pub fn display_light(l: f32, headroom: f32, content_peak: f32) -> f32 {
    if !(l > 0.) {
        return 0.;
    }
    let h = if headroom > 1. { headroom } else { 1. };
    if !(content_peak > h) {
        return (l / h).min(1.);
    }
    let k = DISPLAY_KNEE * h;
    if l <= k {
        return l / h;
    }
    let r = h - k;
    let u = (l - k) / r;
    let top = (content_peak - k) / r;
    let s = u * (1. + u / (top * top)) / (1. + u);
    ((k + r * s) / h).min(1.)
}

/// Scene light held by each 8-bit code of a plane or image with `encoding` (×256, bucket bottom, the VSF quantization rule).
pub fn scene_light_u8(code: u8, encoding: Encoding) -> f32 {
    match encoding {
        Encoding::Linear => code as f32 / 256.,
        Encoding::Gamma2 => linearize_gamma2_u8_f32(code),
        Encoding::Gamma2Tanh => linearize_gamma2tanh_bits_f32(code as u32, 8),
    }
}

/// The display light (fraction of peak, linear) for every 8-bit code of `encoding`, on a display with `headroom`, for content whose brightest code is `peak_code`. One table per image: the per-pixel work is a lookup.
pub fn display_lut_u8(encoding: Encoding, headroom: f32, peak_code: u8) -> [f32; 256] {
    let content_peak = scene_light_u8(peak_code, encoding);
    let mut lut = [0f32; 256];
    for (c, out) in lut.iter_mut().enumerate() {
        *out = display_light(scene_light_u8(c as u8, encoding), headroom, content_peak);
    }
    lut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_that_fits_is_linear() {
        for &l in &[0.0f32, 0.01, 0.5, 1.0] {
            assert_eq!(display_light(l, 1., 1.), l);
            assert_eq!(display_light(l, 4., 1.), l / 4.);
        }
        assert_eq!(display_light(3., 4., 3.5), 0.75);
    }

    #[test]
    fn the_shoulder_is_continuous_monotone_and_reaches_peak_at_the_content_peak() {
        for &(h, peak) in &[(1.0f32, 9.6f32), (1.0, 1.5), (2.5, 9.6), (4.0, 34.6)] {
            let k = DISPLAY_KNEE * h;
            let below = display_light(k * 0.9999, h, peak);
            let above = display_light(k * 1.0001, h, peak);
            assert!((above - below).abs() < 1e-3, "knee jump at h {h} peak {peak}");
            let mut last = 0.;
            for i in 0..=2000 {
                let l = peak * i as f32 / 2000.;
                let y = display_light(l, h, peak);
                assert!(y >= last, "not monotone at l {l} (h {h} peak {peak})");
                last = y;
            }
            assert!((display_light(peak, h, peak) - 1.).abs() < 1e-6);
        }
    }

    #[test]
    fn gamma2_on_an_sdr_display_maps_every_code_to_itself() {
        let lut = display_lut_u8(Encoding::Gamma2, 1., 255);
        for c in 0..=255u32 {
            assert_eq!((lut[c as usize].sqrt() * 256.) as u32, c);
        }
    }

    #[test]
    fn g2t_white_on_an_sdr_display_sits_below_peak_and_highlights_fit_above_it() {
        let lut = display_lut_u8(Encoding::Gamma2Tanh, 1., 255);
        let white = lut[194];
        assert!(white > 0.8 && white < 0.95, "white at {white}");
        assert_eq!(lut[255], 1.);
        assert_eq!(g2t_peak(8), scene_light_u8(255, Encoding::Gamma2Tanh));
    }

    #[test]
    fn bad_input_shows_black() {
        assert_eq!(display_light(f32::NAN, 1., 2.), 0.);
        assert_eq!(display_light(-1., 1., 2.), 0.);
    }
}
