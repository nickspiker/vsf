//! AV1 planes (`vsf::visual::av1`): a plane's samples wrapped under codec `a`, written with rav1e and read with rav1d; the plane's own fields say everything else. Without `av1-decode` a reader names av1 in a clean error.
#![cfg_attr(not(feature = "av1"), allow(dead_code))]

use vsf::types::BitPackedTensor;
use vsf::visual::{Encoding, Plane, PlaneRole, Primaries, Samples, Visual, CODEC_AV1};

/// A deterministic generator for texture and sensor-like noise (xorshift).
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A real-looking still as 8-bit `g2t` codes, planar `[3, h, w]`: a sky gradient up to a bright sun past diffuse white (code 194), a textured hillside, a dark doorway with hard edges, and sensor noise everywhere.
fn photo(w: usize, h: usize) -> Vec<u8> {
    let n = w * h;
    let mut out = vec![0u8; 3 * n];
    let mut noise = Noise(G_SEED);
    // A coarse value-noise lattice for the hillside's texture.
    let lattice: Vec<f32> = (0..(1 << 12)).map(|_| noise.next()).collect();
    let tex = |x: usize, y: usize| lattice[((x / 3) * 61 + (y / 3) * 17) % lattice.len()];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let horizon = 0.55 + 0.08 * (fx * 7.).sin();
            let mut rgb = if fy < horizon {
                let sun = (-((fx - 0.75).powi(2) + (fy - 0.2).powi(2)) * 90.).exp();
                [90. + 60. * fy + 140. * sun, 130. + 50. * fy + 120. * sun, 200. - 20. * fy + 60. * sun]
            } else {
                let t = tex(x, y);
                [70. + 60. * t, 100. + 70. * t, 40. + 30. * t]
            };
            if (0.2..0.3).contains(&fx) && (0.45..0.8).contains(&fy) {
                rgb = [18., 14., 12.];
            }
            let i = y * w + x;
            for ch in 0..3 {
                let v = rgb[ch] + (noise.next() - 0.5) * 6.;
                out[ch * n + i] = v as u8;
            }
        }
    }
    out
}

const G_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// A smooth synthetic: three ramps crossing, odd on both sides so the 4:2:0 edge blocks are one pixel wide and tall.
fn ramps(w: usize, h: usize) -> Vec<u8> {
    let n = w * h;
    let mut out = vec![0u8; 3 * n];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            out[i] = (255 * x / (w - 1)) as u8;
            out[n + i] = (255 * y / (h - 1)) as u8;
            out[2 * n + i] = (255 * (x + y) / (w + h - 2)) as u8;
        }
    }
    out
}

/// Mean and largest absolute error, and PSNR in dB, over every sample.
fn errors(a: &[u8], b: &[u16]) -> (f64, u16, f64) {
    assert_eq!(a.len(), b.len());
    let (mut sum, mut sq, mut max) = (0f64, 0f64, 0u16);
    for (&x, &y) in a.iter().zip(b) {
        let d = (x as i32 - y as i32).unsigned_abs() as u16;
        sum += d as f64;
        sq += (d as f64) * (d as f64);
        max = max.max(d);
    }
    let mse = sq / a.len() as f64;
    (sum / a.len() as f64, max, 10. * (255. * 255. / mse.max(1e-9)).log10())
}

#[cfg(feature = "av1")]
mod with_av1 {
    use super::*;
    use vsf::visual::av1;
    use vsf::visual::Av1Settings;

    /// The ramps survive near-losslessly at a fine quantizer: the transform's own rounding plus the codec's.
    #[test]
    fn ramps_round_trip_odd_sides() {
        let (w, h) = (67, 45);
        let src = ramps(w, h);
        let fine = Av1Settings { quantizer: 0, speed: 10, tiles: 0, threads: 0 };
        let stream = av1::encode_u8(&src, 3, w, h, &fine).unwrap();
        let back = av1::decode(&stream, 3, w, h, 8).unwrap();
        let (mean, max, psnr) = errors(&src, &back);
        assert!(mean < 1.5 && max <= 8 && psnr > 40., "ramps at q0: mean {mean:.3}, max {max}, {psnr:.1} dB");
    }

    /// A real-looking still at the still settings: far smaller than its codes, close to them on average, every colour within reach.
    #[test]
    fn photo_round_trip_size_and_error() {
        let (w, h) = (512, 384);
        let src = photo(w, h);
        for (name, s, max_ratio, min_psnr) in [("still", Av1Settings::STILL, 1. / 16., 28.), ("avatar", Av1Settings::AVATAR, 1. / 4., 33.)] {
            let stream = av1::encode_u8(&src, 3, w, h, &s).unwrap();
            let back = av1::decode(&stream, 3, w, h, 8).unwrap();
            let (mean, max, psnr) = errors(&src, &back);
            let ratio = stream.len() as f64 / src.len() as f64;
            eprintln!("{name}: {} bytes of {} ({:.4}), mean {mean:.2}, max {max}, {psnr:.1} dB", stream.len(), src.len(), ratio);
            assert!(ratio < max_ratio, "{name}: {} bytes for {} codes", stream.len(), src.len());
            assert!(psnr > min_psnr && mean < 4., "{name}: mean {mean:.2}, max {max}, {psnr:.1} dB");
        }
    }

    /// A one-channel 10-bit plane travels as AV1 monochrome and keeps its depth.
    #[test]
    fn mono_ten_bit_round_trip() {
        let (w, h) = (96, 64);
        let src: Vec<u16> = (0..w * h).map(|i| ((i % w) * 1023 / (w - 1)) as u16).collect();
        let mut p = Plane::encoded("b0", "s0", w, h, Primaries::Vsf, Encoding::Gamma2Tanh, BitPackedTensor::pack_u16(10, vec![1, h, w], &src));
        p.channels.truncate(1);
        p.black.truncate(1);
        p.white.truncate(1);
        p.role = PlaneRole::Depth;
        p.wrap_av1(&Av1Settings { quantizer: 0, speed: 10, tiles: 0, threads: 0 }).unwrap();
        assert!(matches!(p.samples, Samples::Wrapped(CODEC_AV1, _)));
        let back = p.packed_samples().unwrap();
        assert_eq!((back.bit_depth, back.shape.clone()), (10, vec![h, w]));
        let codes = back.unpack_u16();
        let max = src.iter().zip(&codes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
        assert!(max <= 4, "10-bit ramp off by {max}");
        assert!(av1::encode(&[1024], 1, 1, 1, 10, &Av1Settings::STILL).is_err(), "a code past 10 bits is refused");
    }

    /// The VSF 8B still: an 8-bit g2t plane in VSF RGB, wrapped as AV1, written and read back as a visual; the reader needs nothing but the file.
    #[test]
    fn g2t8_still_thru_a_visual() {
        let (w, h) = (256, 192);
        let src = photo(w, h);
        let plane = Plane::encoded_av1("b0", "s0", w, h, Primaries::Vsf, Encoding::Gamma2Tanh, &src, &Av1Settings::STILL).unwrap();
        let bytes = Visual::still(plane, None, None, &[]).write().unwrap();
        assert!(bytes.len() < src.len() / 8, "{} bytes", bytes.len());
        let v = Visual::read(&bytes).unwrap();
        let p = v.primary_plane().unwrap();
        assert_eq!((p.width, p.height, p.depth, p.encoding, p.primaries, p.channels.len()), (w, h, 8, Encoding::Gamma2Tanh, Primaries::Vsf, 3));
        assert!(matches!(p.samples, Samples::Wrapped(CODEC_AV1, _)));
        let codes = p.packed_samples().unwrap().unpack_u16();
        let (mean, _, psnr) = errors(&src, &codes);
        assert!(mean < 4. && psnr > 28., "mean {mean:.2}, {psnr:.1} dB");
        // The linear readers decode thru it: g2t keeps its highlights above white.
        let (lin, _, white) = p.linear_samples().unwrap();
        assert_eq!(lin.bit_depth, 16);
        assert!(white[0] < 65535., "the sun is past diffuse white");
        v.to_spectral_image().unwrap();
    }

    /// A stream whose picture is not the plane's geometry is refused, not stretched.
    #[test]
    fn geometry_must_match() {
        let src = ramps(64, 64);
        let stream = av1::encode_u8(&src, 3, 64, 64, &Av1Settings::STILL).unwrap();
        assert!(av1::decode(&stream, 3, 64, 32, 8).is_err(), "a picture larger than the plane");
        let e = av1::decode(&stream, 3, 128, 64, 8).unwrap_err().to_string();
        assert!(e.contains("64 x 64"), "a picture smaller than the plane: {e}");
        assert!(av1::decode(&stream, 1, 64, 64, 8).is_err(), "4:2:0 read as monochrome");
        assert!(av1::decode(&stream, 3, 64, 64, 10).is_err(), "8 bits read as 10");
        assert!(av1::decode(&[0x12, 0x00], 3, 64, 64, 8).is_err(), "two bytes are no picture");
    }

    /// A capture plane and its phase-detect samples are lossless only: av1 on either is refused when written.
    #[test]
    fn sensor_planes_stay_lossless() {
        let src = ramps(16, 16);
        for role in [PlaneRole::Sensor, PlaneRole::Phase] {
            let mut plane = Plane::encoded_av1("b0", "s0", 16, 16, Primaries::Vsf, Encoding::Gamma2Tanh, &src, &Av1Settings::STILL).unwrap();
            plane.role = role;
            let e = Visual::still(plane, None, None, &[]).write().unwrap_err();
            assert!(e.contains("lossless only"), "{e}");
        }
    }
}

/// Without the feature, a wrapped plane still reads as a visual, and asking for its samples names av1.
#[cfg(not(feature = "av1-decode"))]
#[test]
fn without_the_feature_av1_is_named() {
    let mut plane = Plane::encoded("b0", "s0", 4, 4, Primaries::Vsf, Encoding::Gamma2Tanh, BitPackedTensor::pack_u8(8, vec![3, 4, 4], &[0u8; 48]));
    plane.samples = Samples::Wrapped(CODEC_AV1, vec![0x12, 0x00, 0x0a, 0x0b]);
    let bytes = Visual::still(plane, None, None, &[]).write().unwrap();
    let v = Visual::read(&bytes).unwrap();
    let p = v.primary_plane().unwrap();
    assert!(matches!(p.samples, Samples::Wrapped(CODEC_AV1, _)));
    let e = p.linear_samples().unwrap_err().to_string();
    assert!(e.contains("wrapped under av1") && e.contains("av1-decode"), "{e}");
}
