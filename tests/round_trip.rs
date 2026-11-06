//! What is built is what is read (v10): every value, flattened and parsed back, comes back as the same variant with the same shape and data. Random shapes (empty, 1-D, up to four dimensions) and random data under a fixed seed, many times over; before v10 a 1-D tensor read back as a separate vector type, and a reader matching only the tensor silently found nothing.

use num_complex::Complex;
use vsf::types::{BitPackedTensor, Tensor};
use vsf::VsfType;

/// A small deterministic generator (SplitMix64), so a failure reproduces.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    /// A shape: sometimes empty, often 1-D, up to four dimensions, at most a few thousand elements.
    fn shape(&mut self) -> Vec<usize> {
        let ndim = [1, 1, 1, 2, 3, 4][self.below(6) as usize];
        let mut s: Vec<usize> = (0..ndim).map(|_| self.below(9) as usize).collect();
        if self.below(8) == 0 {
            s[0] = 0;
        }
        s
    }
}

fn round_trips(v: VsfType) {
    let flat = v.flatten();
    let mut ptr = 0;
    let back = vsf::parse(&flat, &mut ptr).unwrap_or_else(|e| panic!("{v:?} did not parse back: {e:?}"));
    assert_eq!(ptr, flat.len(), "{v:?} left bytes unread");
    assert_eq!(back, v, "read back as something else");
}

macro_rules! tensor_of {
    ($rng:expr, $variant:ident, $gen:expr) => {{
        let shape = $rng.shape();
        let n: usize = shape.iter().product();
        let data = (0..n).map(|_| $gen(&mut *$rng)).collect();
        VsfType::$variant(Tensor::new(shape, data))
    }};
}

#[test]
fn every_tensor_reads_back_as_built() {
    let mut rng = Rng(0x5EED_0F_0A_11);
    for _ in 0..400 {
        let r = &mut rng;
        round_trips(tensor_of!(r, t_u0, |r: &mut Rng| r.next() & 1 == 1));
        round_trips(tensor_of!(r, t_u3, |r: &mut Rng| r.next() as u8));
        round_trips(tensor_of!(r, t_u4, |r: &mut Rng| r.next() as u16));
        round_trips(tensor_of!(r, t_u5, |r: &mut Rng| r.next() as u32));
        round_trips(tensor_of!(r, t_u6, |r: &mut Rng| r.next()));
        round_trips(tensor_of!(r, t_u7, |r: &mut Rng| ((r.next() as u128) << 64) | r.next() as u128));
        round_trips(tensor_of!(r, t_i3, |r: &mut Rng| r.next() as i8));
        round_trips(tensor_of!(r, t_i4, |r: &mut Rng| r.next() as i16));
        round_trips(tensor_of!(r, t_i5, |r: &mut Rng| r.next() as i32));
        round_trips(tensor_of!(r, t_i6, |r: &mut Rng| r.next() as i64));
        round_trips(tensor_of!(r, t_i7, |r: &mut Rng| (((r.next() as u128) << 64) | r.next() as u128) as i128));
        // Floats from bit patterns that are numbers, so equality means equal bits.
        round_trips(tensor_of!(r, t_f5, |r: &mut Rng| (r.next() as i32 as f32) / 7.0));
        round_trips(tensor_of!(r, t_f6, |r: &mut Rng| (r.next() as i64 as f64) / 7.0));
        round_trips(tensor_of!(r, t_j5, |r: &mut Rng| Complex::new(r.next() as i16 as f32, r.next() as i16 as f32)));
        round_trips(tensor_of!(r, t_j6, |r: &mut Rng| Complex::new(r.next() as i32 as f64, r.next() as i32 as f64)));
    }
}

#[test]
fn bitpacked_tensors_read_back_as_built() {
    let mut rng = Rng(0xB17_5);
    for _ in 0..400 {
        let shape = rng.shape();
        let n: usize = shape.iter().product();
        let bit_depth = 1 + rng.below(32) as u8;
        let bytes = (n * bit_depth as usize).div_ceil(8);
        let mut data: Vec<u8> = (0..bytes).map(|_| rng.next() as u8).collect();
        // The padding past the last sample is zero, as an encoder writes it.
        let spare = bytes * 8 - n * bit_depth as usize;
        if let Some(last) = data.last_mut() {
            *last &= 0xFFu8.checked_shl(spare as u32).unwrap_or(0);
        }
        round_trips(VsfType::p(BitPackedTensor { bit_depth, shape, data }));
    }
}

#[test]
fn scalars_read_back_as_built() {
    let mut rng = Rng(0x5CA1A);
    for _ in 0..400 {
        let x = rng.next();
        for v in [
            VsfType::u0(x & 1 == 1),
            VsfType::u3(x as u8),
            VsfType::u4(x as u16),
            VsfType::u5(x as u32),
            VsfType::u6(x),
            VsfType::u7(((x as u128) << 64) | rng.next() as u128),
            VsfType::i3(x as i8),
            VsfType::i4(x as i16),
            VsfType::i5(x as i32),
            VsfType::i6(x as i64),
            VsfType::f5(x as i32 as f32 / 3.0),
            VsfType::f6(x as i64 as f64 / 3.0),
        ] {
            round_trips(v);
        }
    }
}

/// Spirix tensors, every shape the generator makes. Until v11 none of them read back: the reader took a Spirix tensor's E marker from after the shape, a circle F6E4 tensor went to the complex reader, and a strided circle tensor wrote its element type as a scalar.
#[cfg(feature = "spirix")]
#[test]
fn spirix_tensors_read_back_as_built() {
    use spirix::{CircleF3E3, CircleF4E4, CircleF7E7, ScalarF3E3, ScalarF4E4, ScalarF7E7};
    use vsf::types::StridedTensor;
    let mut rng = Rng(0x5EED_5_7A_11);
    for _ in 0..100 {
        let r = &mut rng;
        let x = |r: &mut Rng| (r.next() as i16 as f64) / 7.;
        round_trips(tensor_of!(r, t_s33, |r: &mut Rng| ScalarF3E3::from(x(r))));
        round_trips(tensor_of!(r, t_s44, |r: &mut Rng| ScalarF4E4::from(x(r))));
        round_trips(tensor_of!(r, t_s77, |r: &mut Rng| ScalarF7E7::from(x(r))));
        round_trips(tensor_of!(r, t_c33, |r: &mut Rng| CircleF3E3::from((x(r), x(r)))));
        round_trips(tensor_of!(r, t_c44, |r: &mut Rng| CircleF4E4::from((x(r), x(r)))));
        round_trips(tensor_of!(r, t_c64, |r: &mut Rng| spirix::CircleF6E4::from((x(r), x(r)))));
        round_trips(tensor_of!(r, t_c77, |r: &mut Rng| CircleF7E7::from((x(r), x(r)))));
    }
    let s: Vec<ScalarF4E4> = [0.5, -1.5, 3., 0.].map(ScalarF4E4::from).to_vec();
    round_trips(VsfType::q_s44(StridedTensor::new(vec![2, 2], vec![2, 1], s)));
    let c: Vec<CircleF4E4> = [(0.5, 1.), (-1.5, 0.), (3., 3.), (0., 0.)].map(CircleF4E4::from).to_vec();
    round_trips(VsfType::q_c44(StridedTensor::new(vec![2, 2], vec![2, 1], c)));
}
