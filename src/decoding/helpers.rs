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
    let mut shape = Vec::with_capacity(ndim);
    for _ in 0..ndim {
        shape.push(decode_usize(data, pointer)?);
    }
    Ok(shape)
}
