//! A `BitPackedView` must agree with the whole-tensor unpackers on every sub-range, row band, tile and slab, at every depth and at awkward widths — and `BitPacker` must emit the bytes `pack_*` emits. Both are asserted here so the partial path can never drift from the format.

use vsf::types::{BitPackedTensor, BitPackedView, BitPacker};

/// A deterministic sample generator — no dependency, no RNG: a 64-bit LCG masked to the depth.
fn samples(n: usize, bits: u32, seed: u64) -> Vec<u128> {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let v = ((x >> 11) as u128) | (((x >> 7) as u128) << 53);
            if bits >= 128 { v } else { v & ((1u128 << bits) - 1) }
        })
        .collect()
}

#[test]
fn ranges_rows_tiles_and_slabs_match_the_whole_unpack_u16() {
    for bits in 1..=16u8 {
        for &w in &[1usize, 3, 5, 7, 8, 13, 64] {
            for &h in &[1usize, 2, 9] {
                for &k in &[1usize, 3] {
                    let n = k * h * w;
                    let s: Vec<u16> = samples(n, bits as u32, (bits as u64) << 20 | (w as u64) << 8 | h as u64 | (k as u64) << 40).into_iter().map(|v| v as u16).collect();
                    let t = BitPackedTensor::pack_u16(bits, vec![k, h, w], &s);
                    let full = t.unpack_u16();
                    assert_eq!(full, s);
                    let v = BitPackedView::of(&t).unwrap();
                    // Every start, several lengths.
                    for first in 0..n {
                        for count in [0usize, 1, 2, w, n - first] {
                            if first + count > n {
                                continue;
                            }
                            assert_eq!(v.unpack_range_u16(first, count).unwrap(), full[first..first + count], "bits {bits} shape [{k},{h},{w}] range {first}+{count}");
                        }
                    }
                    // Slabs (planes), then rows and tiles within one.
                    for plane in 0..k {
                        let p = v.slab(plane).unwrap();
                        assert_eq!(p.shape(), &[h, w]);
                        let base = plane * h * w;
                        for y0 in 0..h {
                            for y1 in y0..=h {
                                assert_eq!(p.rows_u16(y0, y1).unwrap(), full[base + y0 * w..base + y1 * w], "slab {plane} rows {y0}..{y1}");
                            }
                        }
                        // A tile that is not byte aligned on either axis.
                        let (ty, tx) = (h / 3, w / 3);
                        let (th, tw) = (h - ty, w - tx);
                        let tile = p.tile_u16(ty, tx, th, tw).unwrap();
                        let mut want = Vec::new();
                        for r in ty..ty + th {
                            want.extend_from_slice(&full[base + r * w + tx..base + r * w + tx + tw]);
                        }
                        assert_eq!(tile, want, "slab {plane} tile ({ty},{tx}) {th}×{tw}");
                    }
                    // Out of range is an error, not a panic or a zero.
                    assert!(v.unpack_range_u16(n, 1).is_err());
                    assert!(v.slab(k).is_err());
                }
            }
        }
    }
}

#[test]
fn wider_depths_match_their_unpackers() {
    for &bits in &[17u8, 24, 25, 31, 32] {
        let s: Vec<u32> = samples(97, bits as u32, bits as u64).into_iter().map(|v| v as u32).collect();
        let t = BitPackedTensor::pack_u32(bits, vec![97], &s);
        let v = BitPackedView::of(&t).unwrap();
        assert_eq!(v.unpack_range_u32(0, 97).unwrap(), t.unpack_u32());
        assert_eq!(v.unpack_range_u32(40, 13).unwrap(), t.unpack_u32()[40..53]);
        assert!(v.unpack_range_u16(0, 1).is_err(), "{bits}-bit samples must refuse a u16 output");
    }
    for &bits in &[33u8, 48, 63, 64] {
        let s: Vec<u64> = samples(61, bits as u32, bits as u64).into_iter().map(|v| v as u64).collect();
        let t = BitPackedTensor::pack_u64(bits, vec![61], &s);
        let v = BitPackedView::of(&t).unwrap();
        assert_eq!(v.unpack_range_u64(0, 61).unwrap(), t.unpack_u64());
        assert_eq!(v.unpack_range_u64(7, 50).unwrap(), t.unpack_u64()[7..57]);
    }
    for &bits in &[65u8, 100, 127, 128] {
        let s = samples(23, bits as u32, bits as u64);
        let t = BitPackedTensor::pack_u128(bits, vec![23], &s);
        let v = BitPackedView::of(&t).unwrap();
        assert_eq!(v.unpack_range_u128(0, 23).unwrap(), t.unpack_u128());
        assert_eq!(v.unpack_range_u128(5, 11).unwrap(), t.unpack_u128()[5..16]);
    }
    // u8 at every depth it can hold.
    for bits in 1..=8u8 {
        let s: Vec<u8> = samples(50, bits as u32, bits as u64 + 1000).into_iter().map(|v| v as u8).collect();
        let t = BitPackedTensor::pack_u8(bits, vec![5, 10], &s);
        let v = BitPackedView::of(&t).unwrap();
        assert_eq!(v.unpack_range_u8(0, 50).unwrap(), t.unpack_u8());
        assert_eq!(v.unpack_range_u8(13, 20).unwrap(), t.unpack_u8()[13..33]);
    }
}

#[test]
fn bit_packer_emits_the_same_bytes_as_pack_in_any_batching() {
    for bits in [1u8, 3, 7, 8, 9, 12, 14, 16] {
        let s: Vec<u16> = samples(1001, bits as u32, bits as u64 + 77).into_iter().map(|v| v as u16).collect();
        let want = BitPackedTensor::pack_u16(bits, vec![1001], &s);
        for chunk in [1usize, 7, 64, 1001] {
            let mut p = BitPacker::new(bits).unwrap();
            for c in s.chunks(chunk) {
                p.push_u16(c);
            }
            assert_eq!(p.samples(), 1001);
            let t = p.into_tensor(vec![1001]).unwrap();
            assert_eq!(t, want, "bits {bits} chunk {chunk}");
        }
    }
    // The pinned layouts from tensor.rs's own tests, through the packer.
    let mut p = BitPacker::new(12).unwrap();
    p.push_u16(&[0xABC]);
    p.push_u16(&[0x123]);
    assert_eq!(p.finish(), vec![0xAB, 0xC1, 0x23]);
    let mut p = BitPacker::new(14).unwrap();
    p.push_u16(&[0x3FFF, 0x0001]);
    assert_eq!(p.finish(), vec![0xFF, 0xFC, 0x00, 0x10]);
    // Wide depths, including the per-bit path past 121 bits.
    for &bits in &[33u8, 64, 100, 121, 122, 128] {
        let s = samples(37, bits as u32, bits as u64);
        let want = BitPackedTensor::pack_u128(bits, vec![37], &s);
        let mut p = BitPacker::new(bits).unwrap();
        p.push_u128(&s[..10]);
        p.push_u128(&s[10..]);
        assert_eq!(p.into_tensor(vec![37]).unwrap(), want, "bits {bits}");
    }
    // Shape must account for every sample.
    let mut p = BitPacker::new(8).unwrap();
    p.push_u8(&[1, 2, 3]);
    assert!(p.into_tensor(vec![2, 2]).is_err());
    assert!(BitPacker::new(0).is_err());
    assert!(BitPacker::new(129).is_err());
}

#[test]
fn byte_window_names_exactly_the_bytes_a_range_touches() {
    let t = BitPackedTensor::pack_u16(12, vec![4, 8], &(0..32).map(|i| i as u16 * 100).collect::<Vec<_>>());
    let v = BitPackedView::of(&t).unwrap();
    // Samples 3..3+5 at 12 bits: bits 36..96 → bytes 4..12, 4 bits into the first.
    assert_eq!(v.byte_window(3, 5).unwrap(), (4, 8, 4));
    // A window re-based on those bytes reads the same samples.
    let (b0, blen, intra) = v.byte_window(3, 5).unwrap();
    let shape = [5];
    let sub = BitPackedView::with_bit_base(&t.data[b0..b0 + blen], intra, 12, &shape).unwrap();
    assert_eq!(sub.unpack_range_u16(0, 5).unwrap(), t.unpack_u16()[3..8]);
}
