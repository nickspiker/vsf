//! Main VSF parse dispatcher

use crate::decoding::traits::DecodeError;
use crate::prelude::*;
use crate::types::{VsfType, WaAddress};

// Import sub-parsers from sibling modules
use super::metadata::{
    parse_ascii, parse_backward_version, parse_colour_array, parse_colour_constant, parse_count,
    parse_dtype, parse_eagle_time, parse_hash, parse_key, parse_l_length, parse_length, parse_mac,
    parse_offset, parse_signature, parse_string, parse_version, parse_world_cell, parse_world_coord, parse_wrapped,
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
/// * `data` - The byte slice containing VSF-encoded data * `pointer` - Mutable reference to the current position in the data
///
/// # Returns
/// The parsed VsfType, or an error if parsing fails
///
/// # Example
/// ```ignore
/// let data = vec![b'u', b'3', 42]; let mut pointer = 0; let value = parse(&data, &mut pointer)?; // pointer is now 3, value is VsfType::u3(42)
/// ```
pub fn parse(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Pointer out of bounds".into(),
        ));
    }

    let type_byte = data[*pointer];
    *pointer += 1;

    match type_byte {
        b'u' => parse_unsigned(data, pointer),
        b'i' => parse_signed(data, pointer),
        b'j' => parse_complex(data, pointer),
        #[cfg(feature = "spirix")]
        b's' => parse_spirix_scalar(data, pointer),
        #[cfg(not(feature = "spirix"))]
        b's' => Err(DecodeError::Unsupported(
            "Spirix scalar types require 'spirix' feature".into(),
        )),
        #[cfg(feature = "spirix")]
        b'c' => parse_spirix_circle(data, pointer),
        #[cfg(not(feature = "spirix"))]
        b'c' => Err(DecodeError::Unsupported(
            "Spirix circle types require 'spirix' feature".into(),
        )),
        b'p' => parse_bitpacked_tensor(data, pointer),
        b't' => parse_tensor(data, pointer),
        b'q' => parse_strided_tensor(data, pointer),
        b'x' => parse_string(data, pointer),
        b'e' => parse_eagle_time(data, pointer),
        b'w' => {
            // w3..w7 + bytes = WorldCoord (sized: 2^N bits, 1/2/4/8/16 bytes) w + lowercase = world address family (wa, wb, ...)
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Expected w subtype".into()));
            }
            let next = data[*pointer];
            *pointer += 1;
            match next {
                b'3' | b'4' | b'5' | b'6' | b'7' => parse_world_coord(next, data, pointer),
                b'a' => parse_world_address(data, pointer),
                b'm' => parse_world_cell(data, pointer),
                b'l' => parse_language(data, pointer),
                _ => Err(DecodeError::InvalidDataMsg(format!(
                    "Unknown world subtype: w{}",
                    next as char
                ))),
            }
        }
        b'd' => parse_dtype(data, pointer),
        b'a' => parse_ascii(data, pointer),
        b'o' => parse_offset(data, pointer),
        b'b' => parse_length(data, pointer),
        b'l' => parse_l_length(data, pointer), // length in bytes (wire or file)
        b'h' => parse_hash(data, pointer),
        b'f' => parse_float(data, pointer),
        b'n' => {
            // n + digit = count primitive n + lowercase letter = network family (na, nh, ni, ...)
            if *pointer < data.len() && data[*pointer].is_ascii_lowercase() {
                parse_network_address(data, pointer)
            } else {
                parse_count(data, pointer)
            }
        }
        b'z' => parse_version(data, pointer),
        b'y' => parse_backward_version(data, pointer),
        b'm' => parse_mac(data, pointer),
        b'r' => {
            // Look ahead to distinguish:
            // - rc[a-z]: colour constants
            // - ra/rt/rp: colour arrays
            // - ro[a-z]: renderable objects
            if *pointer < data.len() {
                match data[*pointer] {
                    b'c' => parse_colour_constant(data, pointer),
                    b'a' | b't' | b'p' => {
                        let colour_type = data[*pointer];
                        *pointer += 1;
                        parse_colour_array(data, pointer, colour_type)
                    }
                    b'o' => {
                        *pointer += 1;
                        crate::types::renderable::read(data, pointer)
                    }
                    _ => Err(DecodeError::InvalidDataMsg(format!(
                        "Unknown r-prefixed type: r{}",
                        data[*pointer] as char
                    ))),
                }
            } else {
                Err(DecodeError::UnexpectedEofMsg("Expected r subtype".into()))
            }
        }
        b'g' => parse_signature(data, pointer),
        b'k' => parse_key(data, pointer),
        b'v' => parse_wrapped(data, pointer),
        b'{' => parse_opcode(data, pointer),
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Invalid type marker: {}",
            type_byte as char
        ))),
    }
}

/// Parse a Toka opcode: `{` a b `}`
///
/// Format: 4 bytes total - opening brace, two lowercase letters, closing brace
fn parse_opcode(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Need 3 more bytes: a, b, }
    if crate::decoding::helpers::short(data, *pointer, 3) {
        return Err(DecodeError::UnexpectedEofMsg(
            "Incomplete opcode (need 3 more bytes after '{'.into())".into(),
        ));
    }

    let a = data[*pointer];
    let b = data[*pointer + 1];
    let close = data[*pointer + 2];
    *pointer += 3;

    // Validate closing brace
    if close != b'}' {
        return Err(DecodeError::InvalidDataMsg(format!(
            "Invalid opcode: expected '}}' at position {}, got '{}'",
            *pointer - 1,
            close as char
        )));
    }

    // Validate opcode letters (must be lowercase a-z)
    if !a.is_ascii_lowercase() || !b.is_ascii_lowercase() {
        return Err(DecodeError::InvalidDataMsg(format!(
            "Invalid opcode letters: '{}{}' (must be lowercase a-z)",
            a as char, b as char
        )));
    }

    Ok(VsfType::op(a, b))
}

/// Parse network address family: na, nh, ni, nj, nc, nm, np, ns, nu, nn
fn parse_network_address(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Expected network subtype".into(),
        ));
    }
    let sub = data[*pointer];
    *pointer += 1;

    // Helper: read null-terminated string
    let read_str = |data: &[u8], pointer: &mut usize| -> Result<String, DecodeError> {
        let start = *pointer;
        while *pointer < data.len() && data[*pointer] != 0 {
            *pointer += 1;
        }
        let s = core::str::from_utf8(&data[start..*pointer])
            .map_err(|_| DecodeError::InvalidDataMsg("Invalid UTF-8 in network field".into()))?
            .to_string();
        if *pointer < data.len() {
            *pointer += 1;
        } // skip null
        Ok(s)
    };

    match sub {
        b'a' => {
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg("na scheme".into()));
            }
            let scheme = data[*pointer];
            *pointer += 1;
            let host = read_str(data, pointer)?;
            let port = if *pointer + 2 <= data.len() {
                let p = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
                *pointer += 2;
                if p == 0 {
                    None
                } else {
                    Some(p)
                }
            } else {
                None
            };
            Ok(VsfType::na(scheme, host, port))
        }
        b'h' => Ok(VsfType::nh(read_str(data, pointer)?)),
        b'i' => {
            if crate::decoding::helpers::short(data, *pointer, 4) {
                return Err(DecodeError::UnexpectedEofMsg("ni".into()));
            }
            let a = [
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ];
            *pointer += 4;
            Ok(VsfType::ni(a))
        }
        b'j' => {
            if crate::decoding::helpers::short(data, *pointer, 16) {
                return Err(DecodeError::UnexpectedEofMsg("nj".into()));
            }
            let mut a = [0u8; 16];
            a.copy_from_slice(&data[*pointer..*pointer + 16]);
            *pointer += 16;
            Ok(VsfType::nj(a))
        }
        b'c' => {
            let addr = read_str(data, pointer)?;
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg("nc prefix".into()));
            }
            let prefix = data[*pointer];
            *pointer += 1;
            Ok(VsfType::nc(addr, prefix))
        }
        b'm' => {
            if crate::decoding::helpers::short(data, *pointer, 6) {
                return Err(DecodeError::UnexpectedEofMsg("nm".into()));
            }
            let mut a = [0u8; 6];
            a.copy_from_slice(&data[*pointer..*pointer + 6]);
            *pointer += 6;
            Ok(VsfType::nm(a))
        }
        b'w' => {
            // One length byte, then the octets; a length past 32 is no SSID.
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg("nw length".into()));
            }
            let n = data[*pointer] as usize;
            *pointer += 1;
            if crate::decoding::helpers::short(data, *pointer, n) {
                return Err(DecodeError::UnexpectedEofMsg("nw".into()));
            }
            let s = crate::types::Ssid::new(&data[*pointer..*pointer + n]).ok_or_else(|| DecodeError::InvalidDataMsg("nw: an SSID is at most 32 octets".into()))?;
            *pointer += n;
            Ok(VsfType::nw(s))
        }
        b'p' => {
            if crate::decoding::helpers::short(data, *pointer, 2) {
                return Err(DecodeError::UnexpectedEofMsg("np".into()));
            }
            let p = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::np(p))
        }
        b's' => {
            let host = read_str(data, pointer)?;
            if crate::decoding::helpers::short(data, *pointer, 2) {
                return Err(DecodeError::UnexpectedEofMsg("ns port".into()));
            }
            let port = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::ns(host, port))
        }
        b'u' => Ok(VsfType::nu(read_str(data, pointer)?)),
        b'n' => Ok(VsfType::nn(read_str(data, pointer)?)),
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Unknown network subtype: n{}",
            sub as char
        ))),
    }
}

/// Parse world address family: wa, ...
fn parse_world_address(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Expected world address subtype".into(),
        ));
    }
    let sub = data[*pointer];
    *pointer += 1;
    match sub {
        b'a' => {
            let addr = WaAddress::from_wire(data, pointer)?;
            Ok(VsfType::wa(addr))
        }
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Unknown world address subtype: w{}",
            sub as char
        ))),
    }
}

/// Parse a language (`wl`): three big-endian u16 — language, script, region — each held to its range.
fn parse_language(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    if crate::decoding::helpers::short(data, *pointer, 6) {
        return Err(DecodeError::UnexpectedEofMsg("wl needs 6 bytes".into()));
    }
    let mut bytes = [0u8; 6];
    bytes.copy_from_slice(&data[*pointer..*pointer + 6]);
    *pointer += 6;
    Ok(VsfType::wl(crate::types::Language::from_wire(bytes)?))
}
