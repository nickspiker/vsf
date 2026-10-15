//! Context-coded tensors (`vc`): every model gives back exactly the tensor it was given, inside a document too, and a short or forged payload is an error, never a panic or a wrong tensor.

use vsf::codec::{self, Model};
use vsf::types::{BitPackedTensor, Tensor};
use vsf::VsfType;

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
}

/// A raster's worth of values with structure (runs, patches, edges) and some noise, as a map plane has.
fn plane(rng: &mut Rng, n: usize, max: u64) -> Vec<u64> {
    let mut v = Vec::with_capacity(n);
    let mut cur = 0;
    for _ in 0..n {
        if rng.below(17) == 0 {
            cur = rng.below(max + 1);
        }
        v.push(if rng.below(50) == 0 { rng.below(max + 1) } else { cur });
    }
    v
}

fn through(value: &VsfType, model: Model, reference: Option<&[i32]>) -> VsfType {
    let coded = codec::encode(value, model, reference).expect("encodes");
    // As it sits in a document: flattened, parsed back, decoded.
    let flat = coded.flatten();
    let mut p = 0;
    let parsed = vsf::parse(&flat, &mut p).expect("the wrapped value parses");
    assert_eq!(p, flat.len());
    assert_eq!(parsed, coded);
    assert_eq!(codec::needs_reference(&parsed), reference.is_some());
    codec::decode(&parsed, reference).expect("decodes")
}

#[test]
fn rasters_of_every_element_type_come_back() {
    let mut rng = Rng(7);
    for _ in 0..60 {
        let (h, w) = (1 + rng.below(40) as usize, 1 + rng.below(70) as usize);
        let shape = if rng.below(3) == 0 { vec![h * w] } else { vec![h, w] };
        let n = h * w;
        let cases = [
            VsfType::t_u0(Tensor::new(shape.clone(), plane(&mut rng, n, 1).iter().map(|&x| x == 1).collect())),
            VsfType::t_u3(Tensor::new(shape.clone(), plane(&mut rng, n, 255).iter().map(|&x| x as u8).collect())),
            VsfType::t_u4(Tensor::new(shape.clone(), plane(&mut rng, n, 65535).iter().map(|&x| x as u16).collect())),
            VsfType::t_u5(Tensor::new(shape.clone(), plane(&mut rng, n, u32::MAX as u64).iter().map(|&x| x as u32).collect())),
            VsfType::t_i3(Tensor::new(shape.clone(), plane(&mut rng, n, 255).iter().map(|&x| x as u8 as i8).collect())),
            VsfType::t_i4(Tensor::new(shape.clone(), plane(&mut rng, n, 65535).iter().map(|&x| x as u16 as i16).collect())),
            VsfType::t_i5(Tensor::new(shape.clone(), plane(&mut rng, n, u32::MAX as u64).iter().map(|&x| x as u32 as i32).collect())),
        ];
        for v in cases {
            assert_eq!(through(&v, Model::Raster, None), v);
            assert_eq!(through(&v, Model::Series, None), v);
        }
        for w in [1u8, 3, 12, 32] {
            let vals = plane(&mut rng, n, (1u64 << w) - 1);
            let mut data = vec![0u8; (n * w as usize).div_ceil(8)];
            for (i, &x) in vals.iter().enumerate() {
                for b in 0..w as usize {
                    if (x >> (w as usize - 1 - b)) & 1 == 1 {
                        let bit = i * w as usize + b;
                        data[bit >> 3] |= 0x80 >> (bit & 7);
                    }
                }
            }
            let v = VsfType::p(BitPackedTensor { bit_depth: w, shape: shape.clone(), data });
            assert_eq!(through(&v, Model::Raster, None), v);
        }
    }
}

#[test]
fn a_constant_plane_is_a_few_bytes() {
    let v = VsfType::t_u3(Tensor::new(vec![512, 512], vec![7u8; 512 * 512]));
    let coded = codec::encode(&v, Model::Raster, None).unwrap();
    assert!(coded.flatten().len() < 128, "{} bytes", coded.flatten().len());
    assert_eq!(codec::decode(&coded, None).unwrap(), v);
    let empty = VsfType::t_u3(Tensor::new(vec![0], vec![]));
    assert_eq!(through(&empty, Model::Raster, None), empty);
}

#[test]
fn pyramids_come_back_with_and_without_a_reference() {
    let mut rng = Rng(11);
    for roots in [1usize, 2, 6] {
        for depth in 0..7 {
            let n: usize = (0..=depth).map(|l| roots << (2 * l)).sum();
            let coefs: Vec<i32> = (0..n).map(|i| if rng.below(3) == 0 { (rng.below(41) as i32 - 20) * (1 + (n - i) as i32 / 300) } else { 0 }).collect();
            let other: Vec<i32> = coefs.iter().map(|&c| if rng.below(4) == 0 { c / 2 } else { c }).collect();
            let v = VsfType::t_i5(Tensor::new(vec![n], coefs));
            assert_eq!(through(&v, Model::Pyramid { roots }, None), v);
            assert_eq!(through(&v, Model::Pyramid { roots }, Some(&other)), v);
            // Without the reference it was coded against, the reader says so.
            let coded = codec::encode(&v, Model::Pyramid { roots }, Some(&other)).unwrap();
            assert!(codec::decode(&coded, None).is_err());
        }
    }
    // Not a quadtree stream: refused when written.
    let v = VsfType::t_i5(Tensor::new(vec![5], vec![1, 2, 3, 4, 5]));
    assert!(codec::encode(&v, Model::Pyramid { roots: 2 }, None).is_err());
    // Unsigned coefficients: refused.
    assert!(codec::encode(&VsfType::t_u3(Tensor::new(vec![5], vec![0; 5])), Model::Pyramid { roots: 1 }, None).is_err());
}

#[test]
fn short_or_forged_payloads_are_errors() {
    let mut rng = Rng(3);
    let vals: Vec<u8> = plane(&mut rng, 64 * 64, 9).iter().map(|&x| x as u8).collect();
    let v = VsfType::t_u3(Tensor::new(vec![64, 64], vals));
    let VsfType::v(enc, payload) = codec::encode(&v, Model::Raster, None).unwrap() else { panic!() };
    // Every cut of the payload: an error or (where the cut only lost padding) the same tensor, never a panic.
    for cut in 0..payload.len() {
        if let Ok(t) = codec::decode(&VsfType::v(enc, payload[..cut].to_vec()), None) {
            assert_eq!(t, v, "a cut at {cut} gave a different tensor");
        }
    }
    // Random bytes after a valid header, and random headers.
    for _ in 0..500 {
        let mut p = payload.clone();
        for _ in 0..1 + rng.below(6) {
            let i = rng.below(p.len() as u64) as usize;
            p[i] = rng.next() as u8;
        }
        let _ = codec::decode(&VsfType::v(enc, p), None);
        let junk: Vec<u8> = (0..rng.below(40)).map(|_| rng.next() as u8).collect();
        let _ = codec::decode(&VsfType::v(enc, junk), None);
    }
    // Something else entirely.
    assert!(codec::decode(&VsfType::u3(1), None).is_err());
    assert!(codec::encode(&VsfType::f5(1.0), Model::Raster, None).is_err());
}
