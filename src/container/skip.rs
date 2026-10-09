//! Walk a section's bytes recording WHERE each value is, without materialising the big ones. The bulk types — bit-packed planes `p` and wrapped payloads `v` — are located from their headers alone; everything else is small and goes through the ordinary parser, which doubles as the skip.

use crate::decoding::helpers::decode_usize;
use crate::decoding::traits::DecodeError;
use crate::prelude::*;
use crate::types::packed_view::{packed_bytes, shape_elements};
use crate::types::VsfType;

/// What a value is, and where its payload bytes sit (offsets relative to the buffer the walk ran over).
#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    /// A bit-packed tensor: its header decoded, its samples left in place.
    Packed { bit_depth: u8, shape: Vec<usize>, data_start: usize, data_len: usize },
    /// Wrapped bytes under a codec/encryption code, left in place.
    Wrapped { code: u8, data_start: usize, data_len: usize },
    /// Anything else — small; parsed and returned.
    Other(VsfType),
}

/// One value's span inside the buffer: `start..end` is the whole encoding including its type marker.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueSpan {
    pub start: usize,
    pub end: usize,
    pub kind: ValueKind,
}

/// One field: its name and the spans of its values in order (a flag has none).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpan {
    pub name: String,
    pub values: Vec<ValueSpan>,
}

fn eof(what: &str) -> DecodeError {
    DecodeError::UnexpectedEofMsg(what.into())
}

/// Advance past one value, locating a `p` or `v` payload without copying it.
pub fn skip_value(data: &[u8], ptr: &mut usize) -> Result<ValueKind, DecodeError> {
    let start = *ptr;
    match data.get(*ptr).copied() {
        None => Err(eof("value")),
        Some(b'p') => {
            *ptr += 1;
            let ndim = decode_usize(data, ptr)?;
            if ndim > 64 {
                return Err(DecodeError::InvalidTensor(format!("{ndim}-dimensional bit-packed tensor")));
            }
            let bit_depth = *data.get(*ptr).ok_or_else(|| eof("bit depth"))?;
            *ptr += 1;
            if bit_depth == 0 {
                return Err(DecodeError::InvalidTensor("256-bit samples are reserved and unsupported".into()));
            }
            let mut shape = Vec::with_capacity(ndim);
            for _ in 0..ndim {
                shape.push(decode_usize(data, ptr)?);
            }
            let n = shape_elements(&shape).map_err(DecodeError::InvalidTensor)?;
            let data_len = packed_bytes(n, bit_depth).map_err(DecodeError::InvalidTensor)?;
            let data_start = *ptr;
            let end = data_start.checked_add(data_len).filter(|&e| e <= data.len()).ok_or_else(|| eof("bit-packed samples"))?;
            *ptr = end;
            Ok(ValueKind::Packed { bit_depth, shape, data_start, data_len })
        }
        Some(b'v') => {
            *ptr += 1;
            let code = *data.get(*ptr).ok_or_else(|| eof("wrapper code"))?;
            *ptr += 1;
            let bits = decode_usize(data, ptr)?;
            let data_len = bits.div_ceil(8);
            let data_start = *ptr;
            let end = data_start.checked_add(data_len).filter(|&e| e <= data.len()).ok_or_else(|| eof("wrapped payload"))?;
            *ptr = end;
            Ok(ValueKind::Wrapped { code, data_start, data_len })
        }
        Some(_) => {
            *ptr = start;
            Ok(ValueKind::Other(crate::parse(data, ptr)?))
        }
    }
}

/// Walk one field `(name[:v,v,…])` at `ptr`.
pub fn skip_field(data: &[u8], ptr: &mut usize) -> Result<FieldSpan, DecodeError> {
    if data.get(*ptr) != Some(&b'(') {
        return Err(DecodeError::Other(format!("expected '(' at {}", *ptr)));
    }
    *ptr += 1;
    let name = match crate::parse(data, ptr)? {
        VsfType::d(s) => s,
        other => return Err(DecodeError::Other(format!("expected a field name, found {other:?}"))),
    };
    let mut values = Vec::new();
    if data.get(*ptr) == Some(&b':') {
        *ptr += 1;
        loop {
            match data.get(*ptr) {
                None => return Err(eof("field")),
                Some(b')') => break,
                Some(b',') => {
                    *ptr += 1;
                    continue;
                }
                Some(_) => {
                    let start = *ptr;
                    let kind = skip_value(data, ptr)?;
                    values.push(ValueSpan { start, end: *ptr, kind });
                }
            }
        }
    }
    if data.get(*ptr) != Some(&b')') {
        return Err(DecodeError::Other(format!("expected ')' at {}", *ptr)));
    }
    *ptr += 1;
    Ok(FieldSpan { name, values })
}

/// Walk a whole section body `[ … ]` at `ptr` (near form, or far form with its `name n b` hints), returning its name (empty in the near form) and every field's spans.
pub fn skip_section(data: &[u8], ptr: &mut usize) -> Result<(String, Vec<FieldSpan>), DecodeError> {
    if data.get(*ptr) != Some(&b'[') {
        return Err(DecodeError::Other(format!("expected '[' at {}", *ptr)));
    }
    *ptr += 1;
    let mut name = String::new();
    if data.get(*ptr) != Some(&b'(') && data.get(*ptr) != Some(&b']') {
        match crate::parse(data, ptr)? {
            VsfType::d(s) => name = s,
            other => return Err(DecodeError::Other(format!("expected a section name, found {other:?}"))),
        }
        // Far form: n{count} b{length} follow the name.
        let _ = crate::parse(data, ptr)?;
        let _ = crate::parse(data, ptr)?;
    }
    let mut fields = Vec::new();
    loop {
        match data.get(*ptr) {
            None => return Err(eof("section")),
            Some(b']') => {
                *ptr += 1;
                break;
            }
            Some(b'(') => fields.push(skip_field(data, ptr)?),
            Some(_) => *ptr += 1,
        }
    }
    Ok((name, fields))
}
