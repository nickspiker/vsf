//! Main VSF parse dispatcher

use crate::types::VsfType;
use std::io::{Error, ErrorKind};

// Import sub-parsers from sibling modules
use super::metadata::{
    parse_backward_version, parse_count, parse_dtype, parse_eagle_time, parse_file_length,
    parse_hash, parse_key, parse_label, parse_length, parse_mac, parse_marker_def,
    parse_marker_ref, parse_offset, parse_signature, parse_string, parse_version,
    parse_world_coord, parse_wrapped,
};
use super::primitives::{parse_complex, parse_float, parse_signed, parse_unsigned};
#[cfg(feature = "spirix")]
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
        #[cfg(feature = "spirix")]
        b's' => parse_spirix_scalar(data, pointer),
        #[cfg(not(feature = "spirix"))]
        b's' => Err(Error::new(
            ErrorKind::Unsupported,
            "Spirix scalar types require 'spirix' feature",
        )),
        #[cfg(feature = "spirix")]
        b'c' => parse_spirix_circle(data, pointer),
        #[cfg(not(feature = "spirix"))]
        b'c' => Err(Error::new(
            ErrorKind::Unsupported,
            "Spirix circle types require 'spirix' feature",
        )),
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
        b'L' => parse_file_length(data, pointer),
        b'n' => parse_count(data, pointer),
        b'z' => parse_version(data, pointer),
        b'y' => parse_backward_version(data, pointer),
        b'm' => parse_marker_def(data, pointer),
        b'r' => parse_marker_ref(data, pointer),
        b'a' => parse_mac(data, pointer),
        b'h' => parse_hash(data, pointer),
        b'g' => parse_signature(data, pointer),
        b'k' => parse_key(data, pointer),
        b'v' => parse_wrapped(data, pointer),
        b'{' => parse_opcode(data, pointer),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid type marker: {}", type_byte as char),
        )),
    }
}

/// Parse a Toka opcode: `{` a b `}`
///
/// Format: 4 bytes total - opening brace, two lowercase letters, closing brace
fn parse_opcode(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Need 3 more bytes: a, b, }
    if *pointer + 3 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Incomplete opcode (need 3 more bytes after '{')",
        ));
    }

    let a = data[*pointer];
    let b = data[*pointer + 1];
    let close = data[*pointer + 2];
    *pointer += 3;

    // Validate closing brace
    if close != b'}' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid opcode: expected '}}' at position {}, got '{}'",
                *pointer - 1,
                close as char
            ),
        ));
    }

    // Validate opcode letters (must be lowercase a-z)
    if !a.is_ascii_lowercase() || !b.is_ascii_lowercase() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid opcode letters: '{}{}' (must be lowercase a-z)",
                a as char, b as char
            ),
        ));
    }

    Ok(VsfType::op(a, b))
}
