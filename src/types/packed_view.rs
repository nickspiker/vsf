//! Partial access to MSB-first bit-packed samples — any sub-range, any rows, any tile — without unpacking the rest, plus a streaming packer that builds the same bytes row by row.
//!
//! [`BitPackedTensor`]'s own `unpack_*` decode the whole tensor into one allocation, which is the right call for a 2-megapixel plane and the wrong one for a 50-megapixel one on a phone, or for a viewer that only needs the rows under the window. A [`BitPackedView`] is the same wire layout (`p`: samples at bit offset `i × bit_depth`, MSB-first, final byte top-aligned and zero-padded) read through a window: the bytes covering the requested samples, nothing else. It borrows — over an in-memory document it is zero-copy, and [`crate::container::Document`] builds one over exactly the byte range it fetched from a file.
//!
//! Every unpack here is asserted byte-for-byte against the whole-tensor unpackers (`tests/bitpacked_slicing.rs`), and [`BitPacker`]'s output against `pack_*`, so the two paths can never drift from the format.

use crate::prelude::*;
use crate::types::BitPackedTensor;

/// A window onto bit-packed samples: `shape` row-major, sample `i` at bit `bit_base + i × bit_depth` of `data`. `bit_base` lets a sub-view (one plane of a `[k, h, w]` tensor) start mid-byte, which the wire layout allows.
#[derive(Debug, Clone, Copy)]
pub struct BitPackedView<'a> {
    data: &'a [u8],
    bit_base: usize,
    bit_depth: u8,
    shape: &'a [usize],
}

/// The product of a shape, refusing to wrap.
pub fn shape_elements(shape: &[usize]) -> Result<usize, String> {
    shape.iter().try_fold(1usize, |acc, &d| acc.checked_mul(d)).ok_or_else(|| format!("shape {:?} overflows the address space", shape))
}

/// Bytes needed for `elements` samples of `bit_depth` bits, refusing to wrap.
pub fn packed_bytes(elements: usize, bit_depth: u8) -> Result<usize, String> {
    let bits = elements.checked_mul(bit_depth as usize).ok_or_else(|| format!("{elements} samples × {bit_depth} bits overflows the address space"))?;
    Ok(bits.div_ceil(8))
}

impl<'a> BitPackedView<'a> {
    /// A view over `data` holding a tensor of `shape` at `bit_depth` bits (1–128), starting at bit 0. Errors instead of panicking when the depth is out of range or `data` is too short.
    pub fn new(data: &'a [u8], bit_depth: u8, shape: &'a [usize]) -> Result<Self, String> {
        Self::with_bit_base(data, 0, bit_depth, shape)
    }

    /// As [`Self::new`], with the first sample at bit `bit_base`.
    pub fn with_bit_base(data: &'a [u8], bit_base: usize, bit_depth: u8, shape: &'a [usize]) -> Result<Self, String> {
        if bit_depth == 0 || bit_depth > 128 {
            return Err(format!("bit depth {bit_depth} outside 1..=128"));
        }
        let n = shape_elements(shape)?;
        let bits = n.checked_mul(bit_depth as usize).and_then(|b| b.checked_add(bit_base)).ok_or("tensor bit length overflows the address space")?;
        let need = bits.div_ceil(8);
        if need > data.len() {
            return Err(format!("{} bytes hold {} bits, but {n} samples × {bit_depth} bits from bit {bit_base} need {need} bytes", data.len(), data.len() * 8));
        }
        Ok(Self { data, bit_base, bit_depth, shape })
    }

    /// A view over a whole tensor.
    pub fn of(t: &'a BitPackedTensor) -> Result<Self, String> {
        Self::new(&t.data, t.bit_depth, &t.shape)
    }

    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }
    pub fn shape(&self) -> &[usize] {
        self.shape
    }
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sub-view of one slab along the first dimension: plane `k` of a `[k, h, w]` tensor, or row `k` of a `[h, w]` one. The sub-view may start mid-byte; that is what `bit_base` is for.
    pub fn slab(&self, k: usize) -> Result<BitPackedView<'a>, String> {
        if self.shape.is_empty() {
            return Err("a scalar has no slabs".into());
        }
        if k >= self.shape[0] {
            return Err(format!("slab {k} outside 0..{}", self.shape[0]));
        }
        let per = shape_elements(&self.shape[1..])?;
        let bit_base = self.bit_base + k * per * self.bit_depth as usize;
        Self::with_bit_base(self.data, bit_base, self.bit_depth, &self.shape[1..])
    }

    /// The byte range of `data` that samples `first..first+count` touch, so a caller fetching from a file knows what to read; `(byte_start, byte_len, bit_base_within)`.
    pub fn byte_window(&self, first: usize, count: usize) -> Result<(usize, usize, usize), String> {
        let n = self.len();
        let end = first.checked_add(count).filter(|&e| e <= n).ok_or_else(|| format!("samples {first}..{first}+{count} outside 0..{n}"))?;
        let bits = self.bit_depth as usize;
        let b0 = self.bit_base + first * bits;
        let b1 = self.bit_base + end * bits;
        let byte0 = b0 >> 3;
        let byte1 = b1.div_ceil(8);
        Ok((byte0, byte1 - byte0, b0 & 7))
    }

    fn check_range(&self, first: usize, count: usize, width: u32) -> Result<(), String> {
        if self.bit_depth as u32 > width {
            return Err(format!("{}-bit samples do not fit u{width}", self.bit_depth));
        }
        self.byte_window(first, count).map(|_| ())
    }

    /// The rows `y0..y1` along the first dimension (whole slabs), flattened — for a `[h, w]` plane these are image rows.
    pub fn rows_u16(&self, y0: usize, y1: usize) -> Result<Vec<u16>, String> {
        let per = shape_elements(self.shape.get(1..).unwrap_or(&[]))?;
        if y1 < y0 {
            return Err(format!("rows {y0}..{y1} reversed"));
        }
        self.unpack_range_u16(y0 * per, (y1 - y0) * per)
    }

    /// A `h × w` tile at `(y, x)` of a 2-D view, row-major.
    pub fn tile_u16(&self, y: usize, x: usize, h: usize, w: usize) -> Result<Vec<u16>, String> {
        if self.shape.len() != 2 {
            return Err(format!("tile needs a 2-D view, this one is {}-D (take a slab first)", self.shape.len()));
        }
        let (rows, cols) = (self.shape[0], self.shape[1]);
        if y + h > rows || x + w > cols {
            return Err(format!("tile ({y},{x}) {h}×{w} outside {rows}×{cols}"));
        }
        let mut out = Vec::with_capacity(h * w);
        for r in y..y + h {
            out.extend(self.unpack_range_u16(r * cols + x, w)?);
        }
        Ok(out)
    }
}

/// Window extraction: the sample's `bits` sit at `intra` (0..=7) bits into a big-endian window of `W` bits, so `intra + bits ≤ W` guarantees one shift and one mask per sample.
macro_rules! unpack_range {
    ($fn_name:ident, $t:ty, $w:ty, $nbytes:expr) => {
        impl<'a> BitPackedView<'a> {
            #[doc = concat!("Samples `first..first+count` as `", stringify!($t), "`, reading only the bytes they occupy.")]
            pub fn $fn_name(&self, first: usize, count: usize) -> Result<Vec<$t>, String> {
                self.check_range(first, count, <$t>::BITS)?;
                let bits = self.bit_depth as usize;
                let mask: $w = if bits >= <$w>::BITS as usize { <$w>::MAX } else { ((1 as $w) << bits) - 1 };
                let data = self.data;
                let mut out = Vec::with_capacity(count);
                let mut bit = self.bit_base + first * bits;
                for _ in 0..count {
                    let byte = bit >> 3;
                    let intra = bit & 7;
                    let window: $w = if byte + $nbytes <= data.len() {
                        let mut b = [0u8; $nbytes];
                        b.copy_from_slice(&data[byte..byte + $nbytes]);
                        <$w>::from_be_bytes(b)
                    } else {
                        let mut b = [0u8; $nbytes];
                        for (i, slot) in b.iter_mut().enumerate() {
                            if byte + i < data.len() {
                                *slot = data[byte + i];
                            }
                        }
                        <$w>::from_be_bytes(b)
                    };
                    out.push(((window >> (<$w>::BITS as usize - bits - intra)) & mask) as $t);
                    bit += bits;
                }
                Ok(out)
            }
        }
    };
}

unpack_range!(unpack_range_u8, u8, u32, 4);
unpack_range!(unpack_range_u16, u16, u32, 4);
unpack_range!(unpack_range_u32, u32, u64, 8);
unpack_range!(unpack_range_u64, u64, u128, 16);

impl<'a> BitPackedView<'a> {
    /// Samples `first..first+count` as `u128` — the per-bit path, for depths past 64 (and correct for every depth).
    pub fn unpack_range_u128(&self, first: usize, count: usize) -> Result<Vec<u128>, String> {
        self.check_range(first, count, 128)?;
        let bits = self.bit_depth as usize;
        let mut out = Vec::with_capacity(count);
        let mut bit = self.bit_base + first * bits;
        for _ in 0..count {
            let mut v = 0u128;
            for _ in 0..bits {
                let b = (self.data[bit >> 3] >> (7 - (bit & 7))) & 1;
                v = (v << 1) | b as u128;
                bit += 1;
            }
            out.push(v);
        }
        Ok(out)
    }
}

/// A streaming MSB-first packer: push samples in any batches, get bytes identical to `BitPackedTensor::pack_*` of the whole. A writer packs a plane row by row as it leaves the sensor instead of holding a second copy.
#[derive(Debug, Clone)]
pub struct BitPacker {
    bit_depth: u8,
    acc: u128,
    nbits: u32,
    out: Vec<u8>,
    count: usize,
}

impl BitPacker {
    pub fn new(bit_depth: u8) -> Result<Self, String> {
        if bit_depth == 0 || bit_depth > 128 {
            return Err(format!("bit depth {bit_depth} outside 1..=128"));
        }
        Ok(Self { bit_depth, acc: 0, nbits: 0, out: Vec::new(), count: 0 })
    }

    pub fn with_capacity(bit_depth: u8, samples: usize) -> Result<Self, String> {
        let mut p = Self::new(bit_depth)?;
        p.out.reserve(packed_bytes(samples, bit_depth)?);
        Ok(p)
    }

    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }
    /// Samples pushed so far.
    pub fn samples(&self) -> usize {
        self.count
    }

    #[inline]
    fn push_one(&mut self, v: u128) {
        let bits = self.bit_depth as u32;
        if bits > 121 {
            // The accumulator cannot hold a partial byte plus one sample: per-bit, as the tensor packer does at these depths.
            for i in (0..bits).rev() {
                self.acc = (self.acc << 1) | ((v >> i) & 1);
                self.nbits += 1;
                if self.nbits == 8 {
                    self.out.push(self.acc as u8);
                    self.acc = 0;
                    self.nbits = 0;
                }
            }
            self.count += 1;
            return;
        }
        let mask = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
        self.acc = (self.acc << bits) | (v & mask);
        self.nbits += bits;
        while self.nbits >= 8 {
            self.nbits -= 8;
            self.out.push((self.acc >> self.nbits) as u8);
        }
        // Keep the accumulator small: only the pending bits matter.
        if self.nbits == 0 {
            self.acc = 0;
        } else {
            self.acc &= (1u128 << self.nbits) - 1;
        }
        self.count += 1;
    }

    pub fn push_u8(&mut self, samples: &[u8]) {
        for &s in samples {
            self.push_one(s as u128);
        }
    }
    pub fn push_u16(&mut self, samples: &[u16]) {
        for &s in samples {
            self.push_one(s as u128);
        }
    }
    pub fn push_u32(&mut self, samples: &[u32]) {
        for &s in samples {
            self.push_one(s as u128);
        }
    }
    pub fn push_u64(&mut self, samples: &[u64]) {
        for &s in samples {
            self.push_one(s as u128);
        }
    }
    pub fn push_u128(&mut self, samples: &[u128]) {
        for &s in samples {
            self.push_one(s);
        }
    }

    /// The packed bytes, final partial byte top-aligned and zero-padded.
    pub fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.out.push((self.acc << (8 - self.nbits)) as u8);
        }
        self.out
    }

    /// The packed bytes as a tensor of `shape`, which must account for every sample pushed.
    pub fn into_tensor(self, shape: Vec<usize>) -> Result<BitPackedTensor, String> {
        let n = shape_elements(&shape)?;
        if n != self.count {
            return Err(format!("{} samples pushed but shape {:?} holds {n}", self.count, shape));
        }
        let bit_depth = self.bit_depth;
        Ok(BitPackedTensor { bit_depth, shape, data: self.finish() })
    }
}
