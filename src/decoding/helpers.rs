use crate::decoding::traits::DecodeError;
use crate::prelude::*;

/// Decode a variable-length usize from VSF format: canonical (narrowest-marker) encodings only, and an error rather than a silent wrap when the value doesn't fit usize. See [`crate::ewe`].
pub fn decode_usize(data: &[u8], pointer: &mut usize) -> Result<usize, DecodeError> {
    crate::ewe::read_uint(data, pointer)
}

/// Decode a variable-length isize from VSF format (canonical only, no silent wrap). See [`crate::ewe`].
pub fn decode_isize(data: &[u8], pointer: &mut usize) -> Result<isize, DecodeError> {
    crate::ewe::read_int(data, pointer)
}

/// Decode a variable-length u64 from VSF format (canonical only, no truncation on 32-bit targets). See [`crate::ewe`].
pub fn decode_u64(data: &[u8], pointer: &mut usize) -> Result<u64, DecodeError> {
    crate::ewe::read_uint(data, pointer)
}

/// Decode a variable-length i64 from VSF format (canonical only, no truncation on 32-bit targets). See [`crate::ewe`].
pub fn decode_i64(data: &[u8], pointer: &mut usize) -> Result<i64, DecodeError> {
    crate::ewe::read_int(data, pointer)
}

/// Parse shape dimensions from tensor header
pub fn parse_shape(
    data: &[u8],
    pointer: &mut usize,
    ndim: usize,
) -> Result<Vec<usize>, DecodeError> {
    // Every dimension takes at least one byte on the wire, so a count beyond the bytes left is a lie told to make us allocate (security audit VSF-02: a ten-byte input asked for 2^60 dimensions).
    if short(data, *pointer, ndim) {
        return Err(DecodeError::UnexpectedEofMsg(format!("shape claims {ndim} dimensions, more than the bytes left")));
    }
    let mut shape = Vec::with_capacity(ndim);
    for _ in 0..ndim {
        shape.push(decode_usize(data, pointer)?);
    }
    Ok(shape)
}

/// Whether fewer than `len` bytes remain in `data` after `pointer`. Never computed as `pointer + len`, which a wire length near `usize::MAX` wraps past the check in a release build, letting the slice that follows panic (security audit VSF-01); a pointer past the end has nothing remaining.
#[inline]
pub fn short(data: &[u8], pointer: usize, len: usize) -> bool {
    data.len().checked_sub(pointer).map_or(true, |rem| len > rem)
}

/// `count` elements of `size` bytes each, or an error when the product does not fit a usize (security audit VSF-01: an unchecked product wrapped to a small byte count, passed the length check, and the tensor read past it).
#[inline]
pub fn elements_bytes(count: usize, size: usize) -> Result<usize, DecodeError> {
    count.checked_mul(size).ok_or_else(|| DecodeError::InvalidDataMsg("element count overflows a byte length".into()))
}

/// A `len-1`-encoded length read back: the stored value plus one, or an error for the one stored value that has no length (`usize::MAX`, whose plus-one wrapped to an empty hash or key — security audit VSF-01).
#[inline]
pub fn decode_len_minus_one(data: &[u8], pointer: &mut usize) -> Result<usize, DecodeError> {
    decode_usize(data, pointer)?.checked_add(1).ok_or_else(|| DecodeError::InvalidDataMsg("length overflows".into()))
}
