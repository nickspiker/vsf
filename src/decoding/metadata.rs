//! Metadata parsers

use super::helpers::{decode_isize, decode_usize};
use crate::types::{EtType, VsfType, WorldCoord};
use crate::text_encoding::decode_text;
use std::io::{Error, ErrorKind};

// ==================== METADATA ====================

pub fn parse_string(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read character count
    let char_count = decode_usize(data, pointer)?;

    // Read byte length of Huffman-encoded data
    let byte_length = decode_usize(data, pointer)?;

    // Verify we have enough data
    if *pointer + byte_length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for Huffman-encoded string",
        ));
    }

    // Extract Huffman-encoded bytes
    let encoded_bytes = &data[*pointer..*pointer + byte_length];
    *pointer += byte_length;

    // Decode using Huffman decoder
    let value = decode_text(encoded_bytes, char_count)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Huffman decode error: {}", e)))?;

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

pub fn parse_hash(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for hash",
        ));
    }
    let hash = data[*pointer..*pointer + length].to_vec();
    *pointer += length;
    Ok(VsfType::h(hash))
}

pub fn parse_signature(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signature",
        ));
    }
    let sig = data[*pointer..*pointer + length].to_vec();
    *pointer += length;
    Ok(VsfType::g(sig))
}
