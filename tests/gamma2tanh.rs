//! `gamma2tanh` (colour.md, "HDR: Gamma 2, Then tanh"): every code of every depth survives decode → encode, black is black, diffuse white and the top code sit where the spec says, and the shadows are plain gamma 2.

use vsf::colour::convert::*;

#[test]
fn every_8_bit_code_round_trips() {
    for c in 0..=255u8 {
        assert_eq!(delinearize_gamma2tanh_u8_f32(linearize_gamma2tanh_u8_f32(c)), c, "code {c}");
    }
}

#[test]
fn every_16_bit_code_round_trips() {
    for c in 0..=65535u16 {
        assert_eq!(delinearize_gamma2tanh_u16_f32(linearize_gamma2tanh_u16_f32(c)), c, "code {c}");
    }
}

#[test]
fn every_10_and_12_bit_code_round_trips() {
    for bits in [10u32, 12] {
        for c in 0..(1u32 << bits) {
            assert_eq!(delinearize_gamma2tanh_bits_f32(linearize_gamma2tanh_bits_f32(c, bits), bits), c, "{bits}-bit code {c}");
        }
    }
}

#[test]
fn black_white_and_headroom() {
    assert_eq!(linearize_gamma2tanh_u8_f32(0), 0.);
    assert_eq!(delinearize_gamma2tanh_u8_f32(0.), 0);
    // Diffuse white (linear 1.0) encodes to tanh(1) = 0.7616, code 194 of 256.
    assert_eq!(delinearize_gamma2tanh_u8_f32(1.), 194);
    // The top code is finite, and its brightness above diffuse white in stops is the spec's table.
    for (bits, stops) in [(8u32, 3.28f32), (10, 3.86), (12, 4.34), (16, 5.12)] {
        let top = linearize_gamma2tanh_bits_f32((1 << bits) - 1, bits);
        assert!(top.is_finite(), "{bits}-bit top code is finite");
        assert!((top.log2() - stops).abs() < 0.01, "{bits}-bit top code is {} stops, spec says {stops}", top.log2());
    }
    // Anything brighter than the top code still encodes, to the top code.
    assert_eq!(delinearize_gamma2tanh_u8_f32(1e6), 255);
    assert_eq!(delinearize_gamma2tanh_u16_f32(1e6), 65535);
    assert_eq!(delinearize_gamma2tanh_bits_f32(1e6, 12), 4095);
}

#[test]
fn decode_is_strictly_increasing() {
    let mut last = -1.;
    for c in 0..=65535u16 {
        let l = linearize_gamma2tanh_u16_f32(c);
        assert!(l > last, "code {c}");
        last = l;
    }
}

#[test]
fn shadows_are_gamma_2() {
    // tanh(x) ≈ x near zero, so the darkest codes decode within a few percent of plain gamma 2.
    for c in 1..=16u8 {
        let t = linearize_gamma2tanh_u8_f32(c);
        let g = linearize_gamma2_u8_f32(c);
        assert!((t / g - 1.).abs() < 0.01, "code {c}: gamma2tanh {t} vs gamma2 {g}");
    }
}

#[test]
fn trait_matches_free_functions() {
    for c in 0..=255u8 {
        let l: f32 = c.to_linear_gamma2tanh();
        assert_eq!(l, linearize_gamma2tanh_u8_f32(c));
        assert_eq!(<u8 as ColourValue<f32>>::from_linear_gamma2tanh(l), c);
    }
    // Floats are linear: the f32 conversion is the identity, as for every other transfer.
    assert_eq!(2.5f32.to_linear_gamma2tanh(), 2.5);
}

#[cfg(feature = "spirix")]
#[test]
fn s44_matches_f32() {
    use spirix::ScalarF4E4 as S44;
    for c in 0..=255u8 {
        let s = linearize_gamma2tanh_u8_s44(c);
        let f = linearize_gamma2tanh_u8_f32(c);
        let sf: f32 = s.into();
        assert!((sf - f).abs() <= f * 1e-3, "code {c}: S44 {sf} vs f32 {f}");
        assert_eq!(delinearize_gamma2tanh_u8_s44(s), c, "S44 code {c} round trip");
    }
    // S44's 16-bit fraction cannot separate every pair of adjacent 16-bit codes, so 16 bits round-trips within one code (convert.rs, gamma2tanh_code_s44).
    for c in 0..=65535u16 {
        let r = delinearize_gamma2tanh_u16_s44(linearize_gamma2tanh_u16_s44(c));
        assert!((r as i32 - c as i32).abs() <= 1, "S44 16-bit code {c} came back {r}");
    }
    let black: f32 = linearize_gamma2tanh_u8_s44(0).into();
    assert_eq!(black, 0.);
    assert_eq!(delinearize_gamma2tanh_u8_s44(S44::ONE), 194, "diffuse white");
}
