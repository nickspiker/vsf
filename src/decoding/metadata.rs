//! Metadata parsers

use super::helpers::{decode_isize, decode_usize};
use crate::types::{EtType, VsfType, WorldCoord};
use std::io::{Error, ErrorKind};

// ==================== METADATA ====================

pub fn parse_string(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    use crate::text_encoding::decode_text_with_size;

    // Read character count
    let char_count = decode_usize(data, pointer)?;

    // Rest of data is Huffman-encoded bytes
    let huffman_bytes = &data[*pointer..];

    if huffman_bytes.is_empty() && char_count > 0 {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "No Huffman data for non-zero char count",
        ));
    }

    // Decode using Huffman decoder and get bytes consumed
    let (value, bytes_consumed) = decode_text_with_size(huffman_bytes, char_count)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Huffman decode: {}", e)))?;

    // Advance pointer by actual bytes consumed
    *pointer += bytes_consumed;

    Ok(VsfType::x(value))
}

pub fn parse_eagle_time(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for eagle time type marker",
        ));
    }

    let time_type = data[*pointer];
    *pointer += 1;

    match time_type {
        b'u' => {
            let value = decode_usize(data, pointer)?;
            Ok(VsfType::e(EtType::u(value)))
        }
        b'i' => {
            let value = decode_isize(data, pointer)?;
            Ok(VsfType::e(EtType::i(value)))
        }
        b'f' => {
            // Eagle Time floats: [e][f][4 or 8 bytes]
            // Determine f5 vs f6 by looking at available bytes
            let remaining = data.len() - *pointer;

            if remaining >= 8 {
                // f64 (f6)
                if *pointer + 8 > data.len() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Not enough data for f6",
                    ));
                }
                let value = f64::from_be_bytes([
                    data[*pointer],
                    data[*pointer + 1],
                    data[*pointer + 2],
                    data[*pointer + 3],
                    data[*pointer + 4],
                    data[*pointer + 5],
                    data[*pointer + 6],
                    data[*pointer + 7],
                ]);
                *pointer += 8;
                Ok(VsfType::e(EtType::f6(value)))
            } else if remaining >= 4 {
                // f32 (f5)
                if *pointer + 4 > data.len() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Not enough data for f5",
                    ));
                }
                let value = f32::from_be_bytes([
                    data[*pointer],
                    data[*pointer + 1],
                    data[*pointer + 2],
                    data[*pointer + 3],
                ]);
                *pointer += 4;
                Ok(VsfType::e(EtType::f5(value)))
            } else {
                Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for eagle time float",
                ))
            }
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid eagle time type",
        )),
    }
}

pub fn parse_world_coord(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer + 8 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for world coordinate",
        ));
    }

    let raw = u64::from_be_bytes([
        data[*pointer],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::w(WorldCoord::from_raw(raw)))
}

pub fn parse_dtype(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for dtype",
        ));
    }

    let bytes = &data[*pointer..*pointer + length];

    // Validate ASCII-only (identifiers like "imaging.raw", "iso_speed")
    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "dtype must be ASCII (identifiers only)",
        ));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::d(value))
}

pub fn parse_label(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for label",
        ));
    }

    let bytes = &data[*pointer..*pointer + length];

    // Validate ASCII-only (identifiers like field names, keys)
    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "label must be ASCII (identifiers only)",
        ));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::l(value))
}

pub fn parse_offset(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let offset = decode_usize(data, pointer)?;
    Ok(VsfType::o(offset))
}

pub fn parse_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::b(length))
}

pub fn parse_count(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let count = decode_usize(data, pointer)?;
    Ok(VsfType::n(count))
}

pub fn parse_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::z(version))
}

pub fn parse_backward_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::y(version))
}

pub fn parse_marker_def(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::m(value))
}

pub fn parse_marker_ref(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::r(value))
}

pub fn parse_mac(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for MAC algorithm ID",
        ));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read MAC tag length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    if *pointer + length_bytes > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for MAC tag",
        ));
    }
    let mac_tag = data[*pointer..*pointer + length_bytes].to_vec();
    *pointer += length_bytes;
    Ok(VsfType::a(algorithm, mac_tag))
}

pub fn parse_hash(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for hash algorithm ID",
        ));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read hash length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    if *pointer + length_bytes > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for hash",
        ));
    }
    let hash = data[*pointer..*pointer + length_bytes].to_vec();
    *pointer += length_bytes;
    Ok(VsfType::h(algorithm, hash))
}

pub fn parse_signature(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signature algorithm ID",
        ));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read signature length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    let length = length_bytes;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signature",
        ));
    }
    let sig = data[*pointer..*pointer + length].to_vec();
    *pointer += length;
    Ok(VsfType::g(algorithm, sig))
}

pub fn parse_key(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for key algorithm ID",
        ));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read key length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    if *pointer + length_bytes > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for cryptographic key",
        ));
    }
    let key = data[*pointer..*pointer + length_bytes].to_vec();
    *pointer += length_bytes;
    Ok(VsfType::k(algorithm, key))
}

// ==================== PREAMBLE ====================

/// Parse a preamble from VSF data
///
/// Format: {n[count] b[size] h?[hash] g?[signature]}
///
/// Returns (count, size_bits, hash, signature, bytes_consumed)
pub fn parse_preamble(
    data: &[u8],
    pointer: &mut usize,
) -> Result<(usize, usize, Option<Vec<u8>>, Option<Vec<u8>>), Error> {
    // Expect opening brace
    if *pointer >= data.len() || data[*pointer] != b'{' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Expected '{{' for preamble at byte {}", pointer),
        ));
    }
    *pointer += 1;

    let mut count = None;
    let mut size_bits = None;
    let mut hash = None;
    let mut signature = None;

    // Parse fields until closing brace
    while *pointer < data.len() && data[*pointer] != b'}' {
        let marker = data[*pointer];
        *pointer += 1;

        match marker {
            b'n' => {
                // Parse count
                count = Some(decode_usize(data, pointer)?);
            }
            b'b' => {
                // Parse size in bits
                size_bits = Some(decode_usize(data, pointer)?);
            }
            b'h' => {
                // Parse hash
                let hash_len = decode_usize(data, pointer)?;
                if *pointer + hash_len > data.len() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Preamble hash extends beyond data",
                    ));
                }
                hash = Some(data[*pointer..*pointer + hash_len].to_vec());
                *pointer += hash_len;
            }
            b'g' => {
                // Parse signature
                let sig_len = decode_usize(data, pointer)?;
                if *pointer + sig_len > data.len() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Preamble signature extends beyond data",
                    ));
                }
                signature = Some(data[*pointer..*pointer + sig_len].to_vec());
                *pointer += sig_len;
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Unknown preamble marker: {}", marker as char),
                ));
            }
        }
    }

    // Expect closing brace
    if *pointer >= data.len() || data[*pointer] != b'}' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Expected '}}' to close preamble at byte {}", pointer),
        ));
    }
    *pointer += 1;

    // Verify required fields
    let count = count.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "Missing 'n' (count) in preamble",
        )
    })?;

    let size_bits = size_bits.ok_or_else(|| {
        Error::new(ErrorKind::InvalidData, "Missing 'b' (size) in preamble")
    })?;

    Ok((count, size_bits, hash, signature))
}
