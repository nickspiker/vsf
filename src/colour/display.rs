//! Display mapping: scene light → display light.
//!
//! Storage and display are separate stages. An encoding (`gamma2`, `g2t`, `linear`) says what scene light a code holds, with diffuse white at 1.0; it never changes with the screen. This module says how one particular display shows that light, given its headroom: how many times brighter than diffuse white its peak is (1 for an SDR monitor, the HDR/SDR ratio on an HDR panel). Every app calls it as the last step before its screen, so no two viewers disagree, and nothing here is ever written back into a file (colour.md, "Display Mapping").
//!
//! The mapping is linear up to a knee and rolls off above it, reaching the display's peak exactly at the content's brightest value. Content that fits the display (`content_peak ≤ headroom`) is shown linearly, untouched: SDR content on any display, and HDR content on a display with room for it.

use crate::colour::convert::linearize_gamma2tanh_bits_f32;
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

/// Light held by each 8-bit code of a plane or image with `encoding`: ×256, bucket bottom for VSF's own encodings; white at code 255 for a source's ([`Encoding::light`]).
pub fn scene_light_u8(code: u8, encoding: Encoding) -> f32 {
    encoding.light(code as u32, 8)
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

/// The brightest an emissive panel can show, in cd/m², for a picture whose mean displayed light is `mean` cd/m², from the panel's window measurements: `area[i]` is a white window's fraction of the screen (ascending, 0 to 1) on black, and `peak[i]` the luminance it reached (`idt/output.md`, "Peak brightness depends on how much is lit"). A panel's brightness limiter answers to the power it draws, which tracks the picture's mean light; a window of area `a` at peak `p` draws a mean of `a·p`. So the picture gets the peak of the window that draws the same mean, interpolated between the two measurements that bracket it: below the smallest window's mean, that window's peak; above full screen's, full screen's. This takes white's power for every colour (a saturated colour draws differently) and one time scale (a sustained curve is a second set of measurements, passed the same way). Empty or mismatched measurements give 0.
pub fn abl_peak(area: &[f32], peak: &[f32], mean: f32) -> f32 {
    if area.is_empty() || area.len() != peak.len() {
        return 0.;
    }
    let drawn = |i: usize| area[i] * peak[i];
    if !(mean > drawn(0)) {
        return peak[0];
    }
    for i in 1..area.len() {
        let (lo, hi) = (drawn(i - 1), drawn(i));
        if mean <= hi && hi > lo {
            let t = (mean - lo) / (hi - lo);
            return peak[i - 1] + t * (peak[i] - peak[i - 1]);
        }
    }
    peak[peak.len() - 1]
}

/// The headroom (peak ÷ diffuse white) to pass [`display_light`] for this picture on a panel whose brightness limiter is measured as in [`abl_peak`], with diffuse white shown at `white` cd/m². `light` is the picture's scene light, diffuse white = 1 (a subsample is enough). The answer is the largest headroom the panel can sustain for the picture as it would be shown at that headroom: showing it brighter raises its mean, which lowers what the panel allows, so the two meet at one value, found by bisection. A small highlight gets the small-window peak and a bright sky the full-screen one. Never below 1.
pub fn abl_headroom(light: &[f32], white: f32, area: &[f32], peak: &[f32]) -> f32 {
    let top = peak.iter().copied().fold(0f32, f32::max) / white;
    if light.is_empty() || !(top > 1.) {
        return 1.;
    }
    let content_peak = light.iter().copied().fold(0f32, f32::max);
    // Positive while the panel can sustain headroom `h` for this picture shown at `h`.
    let room = |h: f32| {
        let mean = light.iter().map(|&l| display_light(l, h, content_peak)).sum::<f32>() / light.len() as f32 * h * white;
        abl_peak(area, peak, mean) - h * white
    };
    if room(top) >= 0. {
        return top;
    }
    let (mut lo, mut hi) = (1f32, top);
    if room(lo) < 0. {
        return 1.;
    }
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if room(mid) >= 0. {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An OLED-like limiter: 1000 cd/m² up to a 10 % window, 250 at full screen.
    const AREA: [f32; 5] = [0.02, 0.1, 0.25, 0.5, 1.0];
    const PEAK: [f32; 5] = [1000., 1000., 600., 400., 250.];

    #[test]
    fn abl_peak_follows_the_window_that_draws_the_same_power() {
        assert_eq!(abl_peak(&AREA, &PEAK, 5.), 1000., "a tiny highlight gets the small-window peak");
        assert_eq!(abl_peak(&AREA, &PEAK, 100.), 1000., "a 10 % window's mean is 100");
        assert_eq!(abl_peak(&AREA, &PEAK, 250.), 250., "full screen white");
        let mid = abl_peak(&AREA, &PEAK, 175.);
        assert!(mid < 600. && mid > 400., "between the 25 % and 50 % windows: {mid}");
        assert_eq!(abl_peak(&[], &[], 10.), 0.);
    }

    #[test]
    fn a_small_highlight_gets_the_full_peak_and_a_bright_frame_does_not() {
        let white = 200.;
        // 1 % of the picture at 5× white, the rest at a fifth of white.
        let mut speck = vec![0.2f32; 99];
        speck.push(5.);
        assert!((abl_headroom(&speck, white, &AREA, &PEAK) - 5.).abs() < 1e-3, "the panel's whole 1000 cd/m² is available");
        // Everything at 2× white: the panel cannot hold 400 cd/m² full screen.
        let sky = vec![2f32; 100];
        let h = abl_headroom(&sky, white, &AREA, &PEAK);
        assert!(h < 2. && h >= 1., "a bright frame is limited: {h}");
        assert!(abl_headroom(&sky, white, &[], &[]) == 1.);
    }

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
