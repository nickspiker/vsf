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

#[cfg(not(feature = "std"))]
use num_traits::Float;

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

// ==================== SDR SIGNALS OF OTHER ENCODINGS ====================
//
// What a plane kept exactly as its source encoded it decodes thru (`visual::Encoding`, `idt/fields.md`). sRGB and Adobe RGB use the legacy module's curves; these two have no copy there.

/// BT.1886 reference display, black level 0: a BT.709 or BT.2020 SDR signal, 0 to 1, → display light, white = 1. A 2.4 power. This is the display's curve, not the camera's (BT.709's OETF): a plane in this encoding holds a picture already rendered for that display.
pub fn bt1886_light(signal: f32) -> f32 {
    signal.clamp(0., 1.).powf(2.4)
}

/// ProPhoto RGB (ROMM RGB, ISO 22028-2) signal, 0 to 1, → light, white = 1: a 1.8 power with a linear toe of slope 16 below 1/512 of light (signal 1/32), continuous where they meet.
pub fn prophoto_light(signal: f32) -> f32 {
    let e = signal.clamp(0., 1.);
    if e < 1. / 32. {
        e / 16.
    } else {
        e.powf(1.8)
    }
}

// ==================== CAMERA, CINEMA AND BROADCAST SIGNALS ====================
//
// Each decodes a source's normalised code value (0 to 1 over the full code range, the domain each specification states, colour-science's `in_normalised_code_value`) to light with VSF's convention: 1.0 is diffuse white. For the camera logs that is a perfect diffuse reflector (their "reflection" output, 0.18 is grey); for display curves it is the reference white. Formulas and constants as colour-science 0.4.7 implements them; `tests/encoding_references.rs` holds every one to its values. Signals are not clamped: logs carry sub-black and super-white codes.

fn pow10(x: f32) -> f32 {
    (x * core::f32::consts::LN_10).exp()
}

/// A pure power: `signal^g` (gamma 1.8, 2.2, 2.6 displays).
pub fn gamma_light(signal: f32, g: f32) -> f32 {
    signal.max(0.).powf(g)
}

/// BT.709 / BT.601 camera curve, inverted: scene light (the OETF's input, white = 1).
pub fn bt709_scene_light(e: f32) -> f32 {
    if e < 0.081 { e / 4.5 } else { ((e + 0.099) / 1.099).powf(1. / 0.45) }
}

/// SMPTE 240M reference display.
pub fn smpte240m_light(v: f32) -> f32 {
    if v < 0.0912 { v / 4. } else { ((v + 0.1115) / 1.1115).max(0.).powf(1. / 0.45) }
}

/// DCDM (SMPTE ST 428-1) X'Y'Z' → light relative to the 48 cd/m² cinema white: 52.37/48 · V^2.6.
pub fn dcdm_light(v: f32) -> f32 {
    52.37 / 48. * v.max(0.).powf(2.6)
}

/// Sony S-Log (10-bit legal-range code value, as Sony defines it).
pub fn slog_light(y: f32) -> f32 {
    let x = (y * 1023. - 64.) / 876.;
    let r = if y >= 0.088_251_291_513_445_77 { pow10((x - 0.616_596 - 0.03) / 0.432_699) - 0.037_584 } else { (x - 0.030_001_222_851_889_303) / 5. };
    r * 0.9
}

/// Sony S-Log2: S-Log scaled by 219/155.
pub fn slog2_light(y: f32) -> f32 {
    219. * slog_light(y) / 155.
}

/// Sony S-Log3.
pub fn slog3_light(y: f32) -> f32 {
    if y >= 171.210_294_692_9 / 1023. { pow10((y * 1023. - 420.) / 261.5) * (0.18 + 0.01) - 0.01 } else { (y * 1023. - 95.) * 0.011_25 / (171.210_294_692_9 - 95.) }
}

/// ARRI LogC3, SUP 3.x, EI 800 (the curve's other exposure indices differ; EI 800 is the one ARRI publishes as the reference).
pub fn logc3_light(t: f32) -> f32 {
    let (cut, a, b, c, d, e, f) = (0.010_591, 5.555_556, 0.052_272, 0.247_19, 0.385_537, 5.367_655, 0.092_809);
    if t > e * cut + f { (pow10((t - d) / c) - b) / a } else { (t - f) / e }
}

/// ARRI LogC4.
pub fn logc4_light(e: f32) -> f32 {
    let (a, b, c, s, t) = (2231.826_309_067_688_3, 0.907_135_874_877_810_3, 0.092_864_125_122_189_64, 0.113_597_208_610_589_1, -0.018_056_996_119_911_31);
    if e >= 0. { ((14. * (e - c) / b + 6.) * core::f32::consts::LN_2).exp() / a - 64. / a } else { e * s + t }
}

/// Panasonic V-Log.
pub fn vlog_light(v: f32) -> f32 {
    if v < 0.181 { (v - 0.125) / 5.6 } else { pow10((v - 0.598_206) / 0.241_514) - 0.008_73 }
}

/// Canon Log (v1.2).
pub fn clog_light(c: f32) -> f32 {
    let k = 0.125_122_48;
    let x = if c < k { -(pow10((k - c) / 0.453_101_79) - 1.) / 10.1596 } else { (pow10((c - k) / 0.453_101_79) - 1.) / 10.1596 };
    x * 0.9
}

/// Canon Log 2 (v1.2).
pub fn clog2_light(c: f32) -> f32 {
    let k = 0.092_864_125;
    let x = if c < k { -(pow10((k - c) / 0.241_360_77) - 1.) / 87.099_375_46 } else { (pow10((c - k) / 0.241_360_77) - 1.) / 87.099_375_46 };
    x * 0.9
}

/// Canon Log 3 (v1.2).
pub fn clog3_light(c: f32) -> f32 {
    let x = if c < 0.097_465_473 {
        -(pow10((0.127_839_01 - c) / 0.367_268_45) - 1.) / 14.983_25
    } else if c <= 0.152_778_91 {
        (c - 0.125_122_19) / 1.975_479_8
    } else {
        (pow10((c - 0.122_405_37) / 0.367_268_45) - 1.) / 14.983_25
    };
    x * 0.9
}

fn flog_with(o: f32, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, cut2: f32) -> f32 {
    if o < cut2 { (o - f) / e } else { pow10((o - d) / c) / a - b / a }
}

/// Fujifilm F-Log.
pub fn flog_light(o: f32) -> f32 {
    flog_with(o, 0.555_556, 0.009_468, 0.344_676, 0.790_453, 8.735_631, 0.092_864, 0.100_537_775_223_865)
}

/// Fujifilm F-Log2.
pub fn flog2_light(o: f32) -> f32 {
    flog_with(o, 5.555_556, 0.064_829, 0.245_281, 0.384_316, 8.799_461, 0.092_864, 0.100_686_685_370_811)
}

/// Nikon N-Log.
pub fn nlog_light(x: f32) -> f32 {
    let (a, b, c, d, cut2) = (0.635_386_119_257_087, 0.0075, 0.146_627_565_982_404_7, 0.605_083_088_954_056_7, 0.441_837_732_160_312_8);
    if x < cut2 {
        let r = x / a;
        r * r * r - b
    } else {
        ((x - d) / c).exp()
    }
}

/// DJI D-Log.
pub fn dlog_light(y: f32) -> f32 {
    if y <= 0.14 { (y - 0.0929) / 6.025 } else { (pow10(3.896_16 * y - 2.277_52) - 0.0108) / 0.9892 }
}

/// Leica L-Log.
pub fn llog_light(l: f32) -> f32 {
    let (a, b, c, d, e, f) = (8., 0.09, 0.27, 1.3, 0.0115, 0.6);
    if l <= 0.138 { (l - b) / a } else { (pow10((l - f) / c) - e) / d }
}

fn apple_log_with(p: f32, r0: f32, rt: f32, sigma: f32, beta: f32, gamma: f32, delta: f32) -> f32 {
    let pt = sigma * (rt - r0) * (rt - r0);
    if p >= pt {
        (((p - delta) / gamma) * core::f32::consts::LN_2).exp() - beta
    } else if p >= 0. {
        (p / sigma).sqrt() + r0
    } else {
        r0
    }
}

/// Apple Log (iPhone ProRes Log).
pub fn apple_log_light(p: f32) -> f32 {
    apple_log_with(p, -0.056_410_88, 0.01, 47.287_112_36, 0.009_640_52, 0.085_504_79, 0.693_369_45)
}

/// Xiaomi Mi-Log: Apple Log's form with Xiaomi's constants.
pub fn mi_log_light(p: f32) -> f32 {
    apple_log_with(p, -0.090_237_29, 0.019_741_85, 18.105_319_98, 0.013_845_78, 0.092_715_29, 0.672_918_5)
}

/// RED Log3G10 (v3).
pub fn log3g10_light(y: f32) -> f32 {
    let (a, b, c, g) = (0.224_282, 155.975_327, 0.01, 15.1927);
    if y < 0. { y / g - c } else { (pow10(y / a) - 1.) / b - c }
}

/// RED Log3G12.
pub fn log3g12_light(y: f32) -> f32 {
    let m = (pow10(y.abs() / 0.184_904) - 1.) / 347.189_667;
    if y < 0. { -m } else { m }
}

/// ACEScc (ACES AP1 linear).
pub fn acescc_light(y: f32) -> f32 {
    let p = ((y * 17.52 - 9.72) * core::f32::consts::LN_2).exp();
    if y >= (65504f32.log2() + 9.72) / 17.52 {
        65504.
    } else if y < (9.72 - 15.) / 17.52 {
        (p - (-16f32 * core::f32::consts::LN_2).exp()) * 2.
    } else {
        p
    }
}

/// ACEScct (ACES AP1 linear).
pub fn acescct_light(y: f32) -> f32 {
    if y > 0.155_251_141_552_511 { ((y * 17.52 - 9.72) * core::f32::consts::LN_2).exp() } else { (y - 0.072_905_534_195_835_5) / 10.540_237_741_654_5 }
}

/// Kodak Cineon (film scans): 10-bit printing density, white at code 685, black at 95.
pub fn cineon_light(y: f32) -> f32 {
    let bo = pow10((95. - 685.) / 300.);
    (pow10((1023. * y - 685.) / 300.) - bo) / (1. - bo)
}

/// GoPro Protune.
pub fn protune_light(y: f32) -> f32 {
    ((y * 113f32.ln()).exp() - 1.) / 112.
}

/// FilmLight T-Log (E-Gamut's companion curve), at its published w = 128, g = 16, o = 0.075.
pub fn tlog_light(t: f32) -> f32 {
    let (w, g, o) = (128f32, 16f32, 0.075f32);
    let b = 1. / (0.7107 + 1.2359 * (w * g).ln());
    let gs = g / (1. - o);
    let c = b / gs;
    let a = 1. - b * (w + c).ln();
    let y0 = a + b * c.ln();
    let s = (1. - o) / (1. - y0);
    let (aa, bb, gg) = (1. + (a - 1.) * s, b * s, gs * s);
    if t < o { (t - o) / gg } else { ((t - aa) / bb).exp() - c }
}

/// DaVinci Intermediate.
pub fn davinci_intermediate_light(v: f32) -> f32 {
    if v <= 0.027_406_68 { v / 10.444_268_55 } else { ((v / 0.073_292_48 - 7.) * core::f32::consts::LN_2).exp() - 0.0075 }
}

/// Blackmagic Film Generation 5.
pub fn bmd_film_gen5_light(y: f32) -> f32 {
    let (a, b, c, d, e) = (0.086_928_760_654_912_24, 0.005_494_072_432_257_808, 0.530_013_339_229_193_9, 8.283_605_932_402_494, 0.092_465_753_424_657_53);
    if y < d * 0.005 + e { (y - e) / d } else { ((y - c) / a).exp() - b }
}

#[cfg(test)]
mod sdr_signal_tests {
    use super::*;

    #[test]
    fn white_is_one_and_prophoto_toe_meets_its_power() {
        assert_eq!(bt1886_light(1.), 1.);
        assert_eq!(prophoto_light(1.), 1.);
        assert_eq!(bt1886_light(0.), 0.);
        let toe = prophoto_light(1. / 32. - 1e-7);
        let power = prophoto_light(1. / 32.);
        assert!((toe - 1. / 512.).abs() < 1e-6 && (power - 1. / 512.).abs() < 1e-6, "toe {toe} power {power}");
    }
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
