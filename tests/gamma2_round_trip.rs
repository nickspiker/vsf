//! Gamma 2 decode → encode is exact at every code, in f32 and in S44 (colour.md, "Quantization"). S44 at 16 bits used to drop every odd code from 257 up by one.

use vsf::colour::convert::*;

#[test]
fn f32_every_code_round_trips() {
    for c in 0..=255u8 {
        assert_eq!(delinearize_gamma2_u8_f32(linearize_gamma2_u8_f32(c)), c, "8-bit code {c}");
    }
    for c in 0..=65535u16 {
        assert_eq!(delinearize_gamma2_u16_f32(linearize_gamma2_u16_f32(c)), c, "16-bit code {c}");
    }
}

#[cfg(feature = "spirix")]
#[test]
fn s44_every_code_round_trips() {
    use vsf::colour::convert::ColourValue;
    for c in 0..=255u8 {
        assert_eq!(delinearize_gamma2_u8_s44(linearize_gamma2_u8_s44(c)), c, "8-bit code {c}");
        let l: spirix::ScalarF4E4 = c.to_linear_gamma2();
        assert_eq!(<u8 as ColourValue<spirix::ScalarF4E4>>::from_linear_gamma2(l), c, "8-bit trait code {c}");
    }
    for c in 0..=65535u16 {
        assert_eq!(delinearize_gamma2_u16_s44(linearize_gamma2_u16_s44(c)), c, "16-bit code {c}");
        let l: spirix::ScalarF4E4 = c.to_linear_gamma2();
        assert_eq!(<u16 as ColourValue<spirix::ScalarF4E4>>::from_linear_gamma2(l), c, "16-bit trait code {c}");
    }
}
