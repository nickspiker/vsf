//! Context-coded tensors, `vc` (v10): an integer tensor range-coded under adaptive context models, wrapped as `v` data so a reader that lacks the codec skips it by length and one that has it gets back the tensor it was, element type and shape included.
//!
//! The payload: `[model][element type: two bytes, as a tensor writes them][ndim][shape...][model parameter][reference flag][coded bytes...]`, numbers in VSF's own encoding.
//! The coder is LZMA's binary range coder (32-bit range, carry through a cached byte) with 15-bit probabilities adapting by a 32nd per bit: a bit the context makes certain costs about a thousandth of a bit, so a constant plane is a few dozen bytes whatever its size.
//!
//! Three models:
//! - **Raster**: the values in row-major order, rows the tensor's last dimension, each against the value to its left, two to its left and above it ("same as" bits under contexts of how those three agree, else the value itself under an adaptive Exp-Golomb). Classes, coverages, masks, index planes.
//! - **Pyramid**: a quadtree coefficient stream (`roots` values, then four children per value per level, each level in its parent's order), each coefficient's "is zero" under its level, its parent's size, its place among its siblings and the size of the coefficient before; then sign and size. Optionally against a reference stream of the same length (another band of an image), whose size and sign become contexts too: a reader must hold the reference first.
//! - **Series**: each value as its change from the one before. Short tables, edges, profiles.
//!
//! Element types: u0 (bools), u3, u4, u5, i3, i4, i5, and bit-packed tensors up to 32 bits; a pyramid takes signed ones.
//! Written for mahere's map cells (Nick Spiker 2026-10-14), where it beat whole-file zstd by a fifth.

use crate::decoding::helpers::decode_usize;
use crate::decoding::traits::DecodeError;
use crate::encoding::traits::EncodeNumber;
use crate::prelude::*;
use crate::types::{BitPackedTensor, Tensor, VsfType};

/// The `v` encoding byte of a context-coded tensor.
pub const ENCODING: u8 = b'c';

/// How the values are modelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Raster,
    Pyramid { roots: usize },
    Series,
}

impl Model {
    fn code(self) -> u8 {
        match self {
            Model::Raster => 1,
            Model::Pyramid { .. } => 2,
            Model::Series => 3,
        }
    }
}

// ==================== THE RANGE CODER ====================

const PROB_BITS: u32 = 15;
const PROB_ONE: u16 = 1 << PROB_BITS;
const HALF: u16 = PROB_ONE / 2;
const ADAPT: u32 = 5;
const TOP: u32 = 1 << 24;

struct Encoder {
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
    out: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Encoder { low: 0, range: u32::MAX, cache: 0, cache_size: 1, out: Vec::new() }
    }

    fn shift_low(&mut self) {
        if self.low < 0xFF00_0000 || self.low > 0xFFFF_FFFF {
            let carry = (self.low >> 32) as u8;
            let mut temp = self.cache;
            loop {
                self.out.push(temp.wrapping_add(carry));
                temp = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = (self.low >> 24) as u8;
        }
        self.cache_size += 1;
        self.low = (self.low & 0x00FF_FFFF) << 8;
    }

    fn bit(&mut self, p: &mut u16, bit: bool) {
        let bound = (self.range >> PROB_BITS) * *p as u32;
        if bit {
            self.low += bound as u64;
            self.range -= bound;
            *p -= *p >> ADAPT;
        } else {
            self.range = bound;
            *p += (PROB_ONE - *p) >> ADAPT;
        }
        while self.range < TOP {
            self.range <<= 8;
            self.shift_low();
        }
    }

    fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        // The first byte the scheme writes is always the empty cache.
        self.out.remove(0);
        self.out
    }
}

struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
    code: u32,
    range: u32,
}

impl<'a> Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut d = Decoder { data, pos: 0, code: 0, range: u32::MAX };
        for _ in 0..4 {
            d.code = (d.code << 8) | d.byte() as u32;
        }
        d
    }

    fn byte(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    fn bit(&mut self, p: &mut u16) -> bool {
        let bound = (self.range >> PROB_BITS) * *p as u32;
        let bit = if self.code < bound {
            self.range = bound;
            *p += (PROB_ONE - *p) >> ADAPT;
            false
        } else {
            self.code -= bound;
            self.range -= bound;
            *p -= *p >> ADAPT;
            true
        };
        while self.range < TOP {
            self.range <<= 8;
            self.code = (self.code << 8) | self.byte() as u32;
        }
        bit
    }

    /// Whether the decoder read past its bytes: a short or corrupt stream (a whole stream ends exactly where its bytes do; the encoder's flush writes every byte the decoder will read).
    fn overrun(&self) -> bool {
        self.pos > self.data.len()
    }
}

/// An adaptive Exp-Golomb code for a whole number: its length in unary, then the bits under it, every bit under a context of its own.
struct Golomb {
    prefix: [u16; 33],
    suffix: Vec<u16>,
}

impl Golomb {
    fn new() -> Self {
        Golomb { prefix: [HALF; 33], suffix: vec![HALF; 33 * 32] }
    }

    fn put(&mut self, e: &mut Encoder, v: u32) {
        let w = v as u64 + 1;
        let n = 63 - w.leading_zeros();
        for i in 0..n {
            e.bit(&mut self.prefix[i as usize], true);
        }
        if n < 32 {
            e.bit(&mut self.prefix[n as usize], false);
        }
        for j in (0..n).rev() {
            e.bit(&mut self.suffix[(n as usize) * 32 + (n - 1 - j).min(31) as usize], (w >> j) & 1 == 1);
        }
    }

    fn get(&mut self, d: &mut Decoder) -> Option<u32> {
        let mut n = 0u32;
        while n < 32 && d.bit(&mut self.prefix[n as usize]) {
            n += 1;
        }
        let mut w: u64 = 1;
        for j in 0..n {
            w = (w << 1) | d.bit(&mut self.suffix[(n as usize) * 32 + j.min(31) as usize]) as u64;
        }
        u32::try_from(w - 1).ok()
    }
}

fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

fn unzigzag(u: u32) -> i32 {
    ((u >> 1) as i32) ^ -((u & 1) as i32)
}

// ==================== RASTER ====================

struct RasterModel {
    same_a: [u16; 8],
    same_u: [u16; 8],
    same_l: [u16; 8],
    literal: Golomb,
}

impl RasterModel {
    fn new() -> Self {
        RasterModel { same_a: [HALF; 8], same_u: [HALF; 8], same_l: [HALF; 8], literal: Golomb::new() }
    }
}

/// The value to the left (above at a row's start), two to the left, and above; 0 off the tensor. And their agreement as a context.
#[inline(always)]
fn neighbours(v: &[u32], i: usize, width: usize) -> (u32, u32, u32, usize) {
    let x = i % width;
    let u = if i >= width { v[i - width] } else { 0 };
    let a = if x >= 1 { v[i - 1] } else { u };
    let l = if x >= 2 { v[i - 2] } else { 0 };
    (a, l, u, (a == l) as usize | ((a == u) as usize) << 1 | ((l == u) as usize) << 2)
}

fn raster_encode(v: &[u32], width: usize) -> Vec<u8> {
    let mut m = RasterModel::new();
    let mut e = Encoder::new();
    for i in 0..v.len() {
        let (a, l, u, c) = neighbours(v, i, width);
        let x = v[i];
        e.bit(&mut m.same_a[c], x == a);
        if x == a {
            continue;
        }
        if u != a {
            e.bit(&mut m.same_u[c], x == u);
            if x == u {
                continue;
            }
        }
        if l != a && l != u {
            e.bit(&mut m.same_l[c], x == l);
            if x == l {
                continue;
            }
        }
        m.literal.put(&mut e, x);
    }
    e.finish()
}

fn raster_decode(bytes: &[u8], n: usize, width: usize) -> Option<Vec<u32>> {
    let mut v = vec![0u32; n];
    let mut m = RasterModel::new();
    let mut d = Decoder::new(bytes);
    for i in 0..n {
        let (a, l, u, c) = neighbours(&v, i, width);
        v[i] = if d.bit(&mut m.same_a[c]) {
            a
        } else if u != a && d.bit(&mut m.same_u[c]) {
            u
        } else if l != a && l != u && d.bit(&mut m.same_l[c]) {
            l
        } else {
            m.literal.get(&mut d)?
        };
    }
    (!d.overrun()).then_some(v)
}

// ==================== PYRAMID ====================

/// The levels of a quadtree stream of `n` coefficients over `roots` roots, and where each level starts; None when `n` is not such a stream.
fn levels(n: usize, roots: usize) -> Option<Vec<usize>> {
    if roots == 0 {
        return None;
    }
    let mut offsets = vec![0usize];
    let (mut total, mut size) = (0usize, roots);
    while total < n {
        total = total.checked_add(size)?;
        offsets.push(total);
        size = size.checked_mul(4)?;
    }
    (total == n).then_some(offsets)
}

/// The size classes a coefficient falls in, as a context.
fn size_class(c: i32) -> usize {
    match c.unsigned_abs() {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        _ => 3,
    }
}

struct PyramidModel {
    zero: Vec<u16>,
    sign: Vec<u16>,
    size: Vec<Golomb>,
    roots: Golomb,
}

impl PyramidModel {
    fn new(levels: usize) -> Self {
        PyramidModel { zero: vec![HALF; levels * 4 * 4 * 4 * 5], sign: vec![HALF; levels * 3], size: (0..levels * 4 * 4).map(|_| Golomb::new()).collect(), roots: Golomb::new() }
    }
}

fn pyramid_encode(c: &[i32], roots: usize, offsets: &[usize], reference: Option<&[i32]>) -> Vec<u8> {
    let nl = offsets.len() - 1;
    let mut m = PyramidModel::new(nl);
    let mut e = Encoder::new();
    for &v in &c[..roots.min(c.len())] {
        m.roots.put(&mut e, zigzag(v));
    }
    for l in 1..nl {
        let (off, poff) = (offsets[l], offsets[l - 1]);
        let mut prev = 0usize;
        for d in 0..offsets[l + 1] - off {
            let parent = if l == 1 { 0 } else { size_class(c[poff + d / 4]) };
            let v = c[off + d];
            let r = reference.map_or(4, |r| size_class(r[off + d]));
            e.bit(&mut m.zero[(((l * 4 + parent) * 4 + d % 4) * 4 + prev) * 5 + r], v == 0);
            if v != 0 {
                e.bit(&mut m.sign[l * 3 + reference.map_or(1, |r| (r[off + d].signum() + 1) as usize)], v < 0);
                m.size[(l * 4 + parent) * 4 + prev].put(&mut e, v.unsigned_abs() - 1);
            }
            prev = size_class(v);
        }
    }
    e.finish()
}

fn pyramid_decode(bytes: &[u8], roots: usize, offsets: &[usize], reference: Option<&[i32]>) -> Option<Vec<i32>> {
    let nl = offsets.len() - 1;
    let n = offsets[nl];
    let mut m = PyramidModel::new(nl);
    let mut d = Decoder::new(bytes);
    let mut c = vec![0i32; n];
    for v in c.iter_mut().take(roots) {
        *v = unzigzag(m.roots.get(&mut d)?);
    }
    for l in 1..nl {
        let (off, poff) = (offsets[l], offsets[l - 1]);
        let mut prev = 0usize;
        for i in 0..offsets[l + 1] - off {
            let parent = if l == 1 { 0 } else { size_class(c[poff + i / 4]) };
            let r = reference.map_or(4, |r| size_class(r[off + i]));
            if !d.bit(&mut m.zero[(((l * 4 + parent) * 4 + i % 4) * 4 + prev) * 5 + r]) {
                let neg = d.bit(&mut m.sign[l * 3 + reference.map_or(1, |r| (r[off + i].signum() + 1) as usize)]);
                let size = m.size[(l * 4 + parent) * 4 + prev].get(&mut d)? as i64 + 1;
                c[off + i] = (if neg { -size } else { size }) as i32;
            }
            prev = size_class(c[off + i]);
        }
    }
    (!d.overrun()).then_some(c)
}

// ==================== SERIES ====================

fn series_encode(v: &[u32]) -> Vec<u8> {
    let mut g = Golomb::new();
    let mut e = Encoder::new();
    let mut last = 0u32;
    for &x in v {
        g.put(&mut e, zigzag(x.wrapping_sub(last) as i32));
        last = x;
    }
    e.finish()
}

fn series_decode(bytes: &[u8], n: usize) -> Option<Vec<u32>> {
    let mut g = Golomb::new();
    let mut d = Decoder::new(bytes);
    let mut last = 0u32;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        last = last.wrapping_add(unzigzag(g.get(&mut d)?) as u32);
        out.push(last);
    }
    (!d.overrun()).then_some(out)
}

// ==================== TENSORS IN, TENSORS OUT ====================

/// A tensor's integers as the coder sees them: its element type (two marker bytes), shape, and values as u32 (signed ones as their two's complement bits).
fn values_of(value: &VsfType) -> Result<([u8; 2], Vec<usize>, Vec<u32>), DecodeError> {
    let unsupported = || DecodeError::Unsupported(format!("vc codes integer tensors (u0, u3-u5, i3-i5, bit-packed to 32 bits), not {}", short(value)));
    Ok(match value {
        VsfType::t_u0(t) => ([b'u', b'0'], t.shape.clone(), t.data.iter().map(|&b| b as u32).collect()),
        VsfType::t_u3(t) => ([b'u', b'3'], t.shape.clone(), t.data.iter().map(|&x| x as u32).collect()),
        VsfType::t_u4(t) => ([b'u', b'4'], t.shape.clone(), t.data.iter().map(|&x| x as u32).collect()),
        VsfType::t_u5(t) => ([b'u', b'5'], t.shape.clone(), t.data.clone()),
        VsfType::t_i3(t) => ([b'i', b'3'], t.shape.clone(), t.data.iter().map(|&x| x as i32 as u32).collect()),
        VsfType::t_i4(t) => ([b'i', b'4'], t.shape.clone(), t.data.iter().map(|&x| x as i32 as u32).collect()),
        VsfType::t_i5(t) => ([b'i', b'5'], t.shape.clone(), t.data.iter().map(|&x| x as u32).collect()),
        VsfType::p(t) if (1..=32).contains(&t.bit_depth) => {
            let n: usize = t.shape.iter().product();
            let w = t.bit_depth as usize;
            if (n * w).div_ceil(8) > t.data.len() {
                return Err(DecodeError::InvalidDataMsg("bit-packed tensor shorter than its shape".into()));
            }
            let vals = (0..n)
                .map(|i| {
                    let mut v = 0u32;
                    for b in 0..w {
                        let bit = i * w + b;
                        v = (v << 1) | ((t.data[bit >> 3] >> (7 - (bit & 7))) & 1) as u32;
                    }
                    v
                })
                .collect();
            ([b'p', t.bit_depth], t.shape.clone(), vals)
        }
        _ => return Err(unsupported()),
    })
}

/// The tensor back from its element type, shape and values.
fn tensor_of(elem: [u8; 2], shape: Vec<usize>, v: Vec<u32>) -> Result<VsfType, DecodeError> {
    Ok(match elem {
        [b'u', b'0'] => VsfType::t_u0(Tensor { shape, data: v.iter().map(|&x| x != 0).collect() }),
        [b'u', b'3'] => VsfType::t_u3(Tensor { shape, data: v.iter().map(|&x| x as u8).collect() }),
        [b'u', b'4'] => VsfType::t_u4(Tensor { shape, data: v.iter().map(|&x| x as u16).collect() }),
        [b'u', b'5'] => VsfType::t_u5(Tensor { shape, data: v }),
        [b'i', b'3'] => VsfType::t_i3(Tensor { shape, data: v.iter().map(|&x| x as i8).collect() }),
        [b'i', b'4'] => VsfType::t_i4(Tensor { shape, data: v.iter().map(|&x| x as i16).collect() }),
        [b'i', b'5'] => VsfType::t_i5(Tensor { shape, data: v.iter().map(|&x| x as i32).collect() }),
        [b'p', w] if (1..=32).contains(&w) => {
            let w = w as usize;
            let mut data = vec![0u8; (v.len() * w).div_ceil(8)];
            for (i, &x) in v.iter().enumerate() {
                for b in 0..w {
                    if (x >> (w - 1 - b)) & 1 == 1 {
                        let bit = i * w + b;
                        data[bit >> 3] |= 0x80 >> (bit & 7);
                    }
                }
            }
            VsfType::p(BitPackedTensor { bit_depth: w as u8, shape, data })
        }
        _ => return Err(DecodeError::InvalidDataMsg(format!("vc: unknown element type {:?}", elem))),
    })
}

fn short(v: &VsfType) -> String {
    let s = format!("{v:?}");
    s.chars().take(24).collect()
}

/// The integers of a tensor as i32, for a reference: what a pyramid coded against it needs from the reader.
pub fn ints(value: &VsfType) -> Option<Vec<i32>> {
    values_of(value).ok().map(|(_, _, v)| v.into_iter().map(|x| x as i32).collect())
}

/// A tensor context-coded as `vc`. A pyramid takes a signed tensor whose length is a quadtree stream over `roots`, and optionally a reference stream of the same length the reader will hold first.
pub fn encode(value: &VsfType, model: Model, reference: Option<&[i32]>) -> Result<VsfType, DecodeError> {
    let (elem, shape, v) = values_of(value)?;
    let mut out = vec![model.code(), elem[0], elem[1]];
    out.extend_from_slice(&shape.len().encode_number());
    for &d in &shape {
        out.extend_from_slice(&d.encode_number());
    }
    let coded = match model {
        Model::Raster => {
            out.extend_from_slice(&0usize.encode_number());
            out.push(0);
            raster_encode(&v, shape.last().copied().unwrap_or(1).max(1))
        }
        Model::Series => {
            out.extend_from_slice(&0usize.encode_number());
            out.push(0);
            series_encode(&v)
        }
        Model::Pyramid { roots } => {
            if elem[0] != b'i' {
                return Err(DecodeError::Unsupported("vc: a pyramid is signed coefficients (i3, i4, i5)".into()));
            }
            let offsets = levels(v.len(), roots).ok_or_else(|| DecodeError::InvalidDataMsg(format!("vc: {} coefficients are no quadtree stream over {roots} roots", v.len())))?;
            if reference.is_some_and(|r| r.len() != v.len()) {
                return Err(DecodeError::InvalidDataMsg("vc: the reference is not the stream's length".into()));
            }
            out.extend_from_slice(&roots.encode_number());
            out.push(reference.is_some() as u8);
            let c: Vec<i32> = v.iter().map(|&x| x as i32).collect();
            pyramid_encode(&c, roots, &offsets, reference)
        }
    };
    out.extend_from_slice(&coded);
    Ok(VsfType::v(ENCODING, out))
}

/// The header of a `vc` payload: model, element type, shape, model parameter, whether it needs a reference, and where the coded bytes start.
struct Head {
    model: u8,
    elem: [u8; 2],
    shape: Vec<usize>,
    param: usize,
    reference: bool,
    start: usize,
}

fn head(payload: &[u8]) -> Result<Head, DecodeError> {
    let bad = |m: &str| DecodeError::InvalidDataMsg(format!("vc: {m}"));
    if payload.len() < 3 {
        return Err(bad("payload shorter than its header"));
    }
    let (model, elem) = (payload[0], [payload[1], payload[2]]);
    let mut p = 3;
    let ndim = decode_usize(payload, &mut p)?;
    if ndim > 16 {
        return Err(bad("more than sixteen dimensions"));
    }
    let shape = (0..ndim).map(|_| decode_usize(payload, &mut p)).collect::<Result<Vec<usize>, _>>()?;
    let param = decode_usize(payload, &mut p)?;
    let reference = *payload.get(p).ok_or_else(|| bad("payload ends in its header"))? != 0;
    Ok(Head { model, elem, shape, param, reference, start: p + 1 })
}

/// Whether a `vc` value was coded against a reference the reader must hold first.
pub fn needs_reference(value: &VsfType) -> bool {
    matches!(value, VsfType::v(ENCODING, p) if head(p).is_ok_and(|h| h.reference))
}

/// The tensor a `vc` value was coded from, exactly (element type and shape included); `reference` for one coded against another stream.
pub fn decode(value: &VsfType, reference: Option<&[i32]>) -> Result<VsfType, DecodeError> {
    let VsfType::v(ENCODING, payload) = value else {
        return Err(DecodeError::InvalidDataMsg(format!("not a vc value: {}", short(value))));
    };
    let h = head(payload)?;
    let n = h.shape.iter().try_fold(1usize, |a, &d| a.checked_mul(d)).ok_or_else(|| DecodeError::InvalidDataMsg("vc: shape overflows".into()))?;
    // A plane of a billion values from a few bytes is a forged header, not a picture.
    if n > (1 << 30) {
        return Err(DecodeError::InvalidDataMsg("vc: more than 2^30 values".into()));
    }
    let bytes = &payload[h.start..];
    let short_stream = || DecodeError::UnexpectedEofMsg("vc: coded bytes end early".into());
    let v: Vec<u32> = match h.model {
        1 => raster_decode(bytes, n, h.shape.last().copied().unwrap_or(1).max(1)).ok_or_else(short_stream)?,
        3 => series_decode(bytes, n).ok_or_else(short_stream)?,
        2 => {
            let offsets = levels(n, h.param).ok_or_else(|| DecodeError::InvalidDataMsg("vc: not a quadtree stream".into()))?;
            let r = match (h.reference, reference) {
                (true, Some(r)) if r.len() == n => Some(r),
                (true, _) => return Err(DecodeError::InvalidDataMsg("vc: coded against a reference the reader does not hold".into())),
                (false, _) => None,
            };
            pyramid_decode(bytes, h.param, &offsets, r).ok_or_else(short_stream)?.into_iter().map(|x| x as u32).collect()
        }
        m => return Err(DecodeError::InvalidDataMsg(format!("vc: unknown model {m}"))),
    };
    tensor_of(h.elem, h.shape, v)
}
