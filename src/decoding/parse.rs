//! Main VSF parse dispatcher

use crate::types::VsfType;
use std::io::{Error, ErrorKind};

// Import sub-parsers from sibling modules
use super::metadata::{
    parse_backward_version, parse_count, parse_dtype, parse_eagle_time, parse_hash, parse_label,
    parse_length, parse_marker_def, parse_marker_ref, parse_offset, parse_signature, parse_string,
    parse_version, parse_world_coord,
};
use super::primitives::{parse_complex, parse_float, parse_signed, parse_unsigned};
use super::spirix::{parse_spirix_circle, parse_spirix_scalar};
use super::tensors::{parse_bitpacked_tensor, parse_strided_tensor, parse_tensor};

/// Parse VSF binary data into a VsfType
///
/// The pointer is advanced as bytes are consumed.
///
/// # Arguments
/// * `data` - The byte slice containing VSF-encoded data
/// * `pointer` - Mutable reference to the current position in the data
///
/// # Returns
/// The parsed VsfType, or an error if parsing fails
///
/// # Example
/// ```ignore
/// let data = vec![b'u', b'3', 42];
/// let mut pointer = 0;
/// let value = parse(&data, &mut pointer)?;
/// // pointer is now 3, value is VsfType::u3(42)
/// ```
pub fn parse(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Pointer out of bounds",
        ));
    }

    let type_byte = data[*pointer];
    *pointer += 1;

    match type_byte {
        b'u' => parse_unsigned(data, pointer),
        b'i' => parse_signed(data, pointer),
        b'f' => parse_float(data, pointer),
        b'j' => parse_complex(data, pointer),
        b's' => parse_spirix_scalar(data, pointer),
        b'c' => parse_spirix_circle(data, pointer),
        b'p' => parse_bitpacked_tensor(data, pointer),
        b't' => parse_tensor(data, pointer),
        b'q' => parse_strided_tensor(data, pointer),
        b'x' => parse_string(data, pointer),
        b'e' => parse_eagle_time(data, pointer),
        b'w' => parse_world_coord(data, pointer),
        b'd' => parse_dtype(data, pointer),
        b'l' => parse_label(data, pointer),
        b'o' => parse_offset(data, pointer),
        b'b' => parse_length(data, pointer),
        b'n' => parse_count(data, pointer),
        b'z' => parse_version(data, pointer),
        b'y' => parse_backward_version(data, pointer),
        b'm' => parse_marker_def(data, pointer),
        b'r' => parse_marker_ref(data, pointer),
        b'h' => parse_hash(data, pointer),
        b'g' => parse_signature(data, pointer),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type marker: {}", type_byte as char),
        )),
    }
}
