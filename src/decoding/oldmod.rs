//! VSF Decoding Module
//!
//! This module handles decoding VSF binary format back into Rust types.

use crate::types::{EtType, VsfType, Tensor, StridedTensor};
#[allow(unused_imports)]
use spirix::{
    ScalarF6E4, CircleF6E4, // Will use for examples
};
use num_complex::Complex;
use std::io::{Error, ErrorKind};

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
        return Err(Error::new(ErrorKind::UnexpectedEof, "Pointer out of bounds"));
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
        b't' => parse_tensor(data, pointer),
        b'q' => parse_strided_tensor(data, pointer),
        b'x' => parse_string(data, pointer),
        b'e' => parse_eagle_time(data, pointer),
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

// ==================== HELPER FUNCTIONS ====================

/// Decode a variable-length usize from VSF format
fn decode_usize(data: &[u8], pointer: &mut usize) -> Result<usize, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for size marker"));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u8"));
            }
            let value = data[*pointer] as usize;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u16"));
            }
            let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as usize;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u32"));
            }
            let value = u32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]) as usize;
            *pointer += 4;
            Ok(value)
        }
        b'6' => {
            *pointer += 1;
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u64"));
            }
            let value = u64::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
            ]) as usize;
            *pointer += 8;
            Ok(value)
        }
        b'7' => {
            *pointer += 1;
            if *pointer + 16 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u128"));
            }
            let value = u128::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
                data[*pointer + 8],
                data[*pointer + 9],
                data[*pointer + 10],
                data[*pointer + 11],
                data[*pointer + 12],
                data[*pointer + 13],
                data[*pointer + 14],
                data[*pointer + 15],
            ]) as usize;
            *pointer += 16;
            Ok(value)
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid usize size marker: {}", data[*pointer]),
        )),
    }
}

/// Decode a variable-length isize from VSF format
fn decode_isize(data: &[u8], pointer: &mut usize) -> Result<isize, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for size marker"));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i8"));
            }
            let value = data[*pointer] as i8 as isize;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i16"));
            }
            let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as isize;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i32"));
            }
            let value = i32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]) as isize;
            *pointer += 4;
            Ok(value)
        }
        b'6' => {
            *pointer += 1;
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i64"));
            }
            let value = i64::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
            ]) as isize;
            *pointer += 8;
            Ok(value)
        }
        b'7' => {
            *pointer += 1;
            if *pointer + 16 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i128"));
            }
            let value = i128::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
                data[*pointer + 8],
                data[*pointer + 9],
                data[*pointer + 10],
                data[*pointer + 11],
                data[*pointer + 12],
                data[*pointer + 13],
                data[*pointer + 14],
                data[*pointer + 15],
            ]) as isize;
            *pointer += 16;
            Ok(value)
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid isize size marker: {}", data[*pointer]),
        )),
    }
}

// ==================== UNSIGNED INTEGERS ====================

fn parse_unsigned(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for unsigned size marker"));
    }

    let size_byte = data[*pointer];

    // Special case: u0 (boolean) has no size marker '0', just the value directly
    // Format: [b'u', 0 or 255]
    if size_byte == 0 || size_byte == 255 {
        *pointer += 1;
        return Ok(VsfType::u0(size_byte == 255));
    }

    *pointer += 1;

    match size_byte {
        b'0' => {
            // Boolean: u0 with explicit '0' marker (alternative format)
            if *pointer >= data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u0"));
            }
            let value = data[*pointer];
            *pointer += 1;
            match value {
                0 => Ok(VsfType::u0(false)),
                255 => Ok(VsfType::u0(true)),
                _ => Err(Error::new(ErrorKind::InvalidData, "Invalid boolean value")),
            }
        }
        b'3' => {
            // u3: u8
            if *pointer >= data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u3"));
            }
            let value = data[*pointer];
            *pointer += 1;
            Ok(VsfType::u3(value))
        }
        b'4' => {
            // u4: u16
            if *pointer + 2 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u4"));
            }
            let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::u4(value))
        }
        b'5' => {
            // u5: u32
            if *pointer + 4 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u5"));
            }
            let value = u32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]);
            *pointer += 4;
            Ok(VsfType::u5(value))
        }
        b'6' => {
            // u6: u64
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u6"));
            }
            let value = u64::from_be_bytes([
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
            Ok(VsfType::u6(value))
        }
        b'7' => {
            // u7: u128
            if *pointer + 16 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u7"));
            }
            let value = u128::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
                data[*pointer + 8],
                data[*pointer + 9],
                data[*pointer + 10],
                data[*pointer + 11],
                data[*pointer + 12],
                data[*pointer + 13],
                data[*pointer + 14],
                data[*pointer + 15],
            ]);
            *pointer += 16;
            Ok(VsfType::u7(value))
        }
        _ => {
            // Auto-sized u: decode as variable-length
            *pointer -= 1; // Back up to re-read size marker
            let value = decode_usize(data, pointer)?;
            Ok(VsfType::u(value, false))
        }
    }
}

// ==================== SIGNED INTEGERS ====================

fn parse_signed(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for signed size marker"));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'3' => {
            // i3: i8
            if *pointer >= data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i3"));
            }
            let value = data[*pointer] as i8;
            *pointer += 1;
            Ok(VsfType::i3(value))
        }
        b'4' => {
            // i4: i16
            if *pointer + 2 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i4"));
            }
            let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::i4(value))
        }
        b'5' => {
            // i5: i32
            if *pointer + 4 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i5"));
            }
            let value = i32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]);
            *pointer += 4;
            Ok(VsfType::i5(value))
        }
        b'6' => {
            // i6: i64
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i6"));
            }
            let value = i64::from_be_bytes([
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
            Ok(VsfType::i6(value))
        }
        b'7' => {
            // i7: i128
            if *pointer + 16 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i7"));
            }
            let value = i128::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
                data[*pointer + 4],
                data[*pointer + 5],
                data[*pointer + 6],
                data[*pointer + 7],
                data[*pointer + 8],
                data[*pointer + 9],
                data[*pointer + 10],
                data[*pointer + 11],
                data[*pointer + 12],
                data[*pointer + 13],
                data[*pointer + 14],
                data[*pointer + 15],
            ]);
            *pointer += 16;
            Ok(VsfType::i7(value))
        }
        _ => {
            // Auto-sized i: decode as variable-length
            *pointer -= 1; // Back up to re-read size marker
            let value = decode_isize(data, pointer)?;
            Ok(VsfType::i(value))
        }
    }
}

// ==================== IEEE FLOATS ====================

fn parse_float(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for float size marker"));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'5' => {
            // f5: f32
            if *pointer + 4 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f5"));
            }
            let value = f32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]);
            *pointer += 4;
            Ok(VsfType::f5(value))
        }
        b'6' => {
            // f6: f64
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f6"));
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
            Ok(VsfType::f6(value))
        }
        _ => Err(Error::new(ErrorKind::InvalidData, "Invalid float size marker")),
    }
}

// ==================== IEEE COMPLEX ====================

fn parse_complex(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for complex size marker"));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'5' => {
            // j5: Complex<f32>
            if *pointer + 8 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for j5"));
            }
            let re = f32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]);
            *pointer += 4;
            let im = f32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]);
            *pointer += 4;
            Ok(VsfType::j5(Complex::new(re, im)))
        }
        b'6' => {
            // j6: Complex<f64>
            if *pointer + 16 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for j6"));
            }
            let re = f64::from_be_bytes([
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
            let im = f64::from_be_bytes([
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
            Ok(VsfType::j6(Complex::new(re, im)))
        }
        _ => Err(Error::new(ErrorKind::InvalidData, "Invalid complex size marker")),
    }
}

// ==================== METADATA ====================

fn parse_string(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for string"));
    }
    let value = String::from_utf8(data[*pointer..*pointer + length].to_vec())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8 string"))?;
    *pointer += length;
    Ok(VsfType::x(value))
}

fn parse_eagle_time(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for eagle time type marker"));
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
                    return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f6"));
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
                    return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f5"));
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
                Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for eagle time float"))
            }
        }
        _ => Err(Error::new(ErrorKind::InvalidData, "Invalid eagle time type")),
    }
}

fn parse_dtype(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for dtype"));
    }
    let value = String::from_utf8(data[*pointer..*pointer + length].to_vec())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8 in dtype"))?;
    *pointer += length;
    Ok(VsfType::d(value))
}

fn parse_label(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for label"));
    }
    let value = String::from_utf8(data[*pointer..*pointer + length].to_vec())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8 in label"))?;
    *pointer += length;
    Ok(VsfType::l(value))
}

fn parse_offset(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let offset = decode_usize(data, pointer)?;
    Ok(VsfType::o(offset))
}

fn parse_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::b(length))
}

fn parse_count(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let count = decode_usize(data, pointer)?;
    Ok(VsfType::n(count))
}

fn parse_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::z(version))
}

fn parse_backward_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::y(version))
}

fn parse_marker_def(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::m(value))
}

fn parse_marker_ref(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::r(value))
}

fn parse_hash(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for hash"));
    }
    let hash = data[*pointer..*pointer + length].to_vec();
    *pointer += length;
    Ok(VsfType::h(hash))
}

fn parse_signature(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for signature"));
    }
    let sig = data[*pointer..*pointer + length].to_vec();
    *pointer += length;
    Ok(VsfType::g(sig))
}

// ==================== SPIRIX (Stubs) ====================

fn parse_spirix_scalar(_data: &[u8], _pointer: &mut usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Spirix scalar decoding not yet implemented"))
}

fn parse_spirix_circle(_data: &[u8], _pointer: &mut usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Spirix circle decoding not yet implemented"))
}

// ==================== TENSORS ====================

/// Parse shape dimensions from tensor header
fn parse_shape(data: &[u8], pointer: &mut usize, ndim: usize) -> Result<Vec<usize>, Error> {
    let mut shape = Vec::with_capacity(ndim);
    for _ in 0..ndim {
        shape.push(decode_usize(data, pointer)?);
    }
    Ok(shape)
}

/// Parse contiguous tensor: [t][ndim][elem_type][elem_size][shape...][data...]
fn parse_tensor(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse dimension count
    let ndim = decode_usize(data, pointer)?;

    // Parse element type markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for element type"));
    }
    let elem_type = data[*pointer];
    *pointer += 1;
    let elem_size = data[*pointer];
    *pointer += 1;

    // Parse shape
    let shape = parse_shape(data, pointer, ndim)?;
    let total_elements: usize = shape.iter().product();

    // Dispatch based on element type
    match (elem_type, elem_size) {
        (b'u', b'0') => parse_tensor_data_u0(data, pointer, shape, total_elements),
        (b'u', b'3') => parse_tensor_data_u8(data, pointer, shape, total_elements),
        (b'u', b'4') => parse_tensor_data_u16(data, pointer, shape, total_elements),
        (b'u', b'5') => parse_tensor_data_u32(data, pointer, shape, total_elements),
        (b'u', b'6') => parse_tensor_data_u64(data, pointer, shape, total_elements),
        (b'u', b'7') => parse_tensor_data_u128(data, pointer, shape, total_elements),
        (b'i', b'3') => parse_tensor_data_i8(data, pointer, shape, total_elements),
        (b'i', b'4') => parse_tensor_data_i16(data, pointer, shape, total_elements),
        (b'i', b'5') => parse_tensor_data_i32(data, pointer, shape, total_elements),
        (b'i', b'6') => parse_tensor_data_i64(data, pointer, shape, total_elements),
        (b'i', b'7') => parse_tensor_data_i128(data, pointer, shape, total_elements),
        (b'f', b'5') => parse_tensor_data_f32(data, pointer, shape, total_elements),
        (b'f', b'6') => parse_tensor_data_f64(data, pointer, shape, total_elements),
        (b'j', b'5') => parse_tensor_data_c32(data, pointer, shape, total_elements),
        (b'j', b'6') => parse_tensor_data_c64(data, pointer, shape, total_elements),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid tensor element type: {}{}", elem_type as char, elem_size as char),
        )),
    }
}

/// Parse strided tensor: [q][ndim][elem_type][elem_size][shape...][stride...][data...]
fn parse_strided_tensor(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse dimension count
    let ndim = decode_usize(data, pointer)?;

    // Parse element type markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for element type"));
    }
    let elem_type = data[*pointer];
    *pointer += 1;
    let elem_size = data[*pointer];
    *pointer += 1;

    // Parse shape
    let shape = parse_shape(data, pointer, ndim)?;

    // Parse stride
    let stride = parse_shape(data, pointer, ndim)?; // Same format as shape

    let total_elements: usize = shape.iter().product();

    // Dispatch based on element type
    match (elem_type, elem_size) {
        (b'u', b'0') => parse_strided_tensor_data_u0(data, pointer, shape, stride, total_elements),
        (b'u', b'3') => parse_strided_tensor_data_u8(data, pointer, shape, stride, total_elements),
        (b'u', b'4') => parse_strided_tensor_data_u16(data, pointer, shape, stride, total_elements),
        (b'u', b'5') => parse_strided_tensor_data_u32(data, pointer, shape, stride, total_elements),
        (b'u', b'6') => parse_strided_tensor_data_u64(data, pointer, shape, stride, total_elements),
        (b'u', b'7') => parse_strided_tensor_data_u128(data, pointer, shape, stride, total_elements),
        (b'i', b'3') => parse_strided_tensor_data_i8(data, pointer, shape, stride, total_elements),
        (b'i', b'4') => parse_strided_tensor_data_i16(data, pointer, shape, stride, total_elements),
        (b'i', b'5') => parse_strided_tensor_data_i32(data, pointer, shape, stride, total_elements),
        (b'i', b'6') => parse_strided_tensor_data_i64(data, pointer, shape, stride, total_elements),
        (b'i', b'7') => parse_strided_tensor_data_i128(data, pointer, shape, stride, total_elements),
        (b'f', b'5') => parse_strided_tensor_data_f32(data, pointer, shape, stride, total_elements),
        (b'f', b'6') => parse_strided_tensor_data_f64(data, pointer, shape, stride, total_elements),
        (b'j', b'5') => parse_strided_tensor_data_c32(data, pointer, shape, stride, total_elements),
        (b'j', b'6') => parse_strided_tensor_data_c64(data, pointer, shape, stride, total_elements),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid strided tensor element type: {}{}", elem_type as char, elem_size as char),
        )),
    }
}

// ==================== TENSOR DATA PARSERS ====================

// Unsigned integers
fn parse_tensor_data_u0(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    // Bitpacked: 8 bools per byte
    let byte_count = (total_elements + 7) / 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u0 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    let mut byte_idx = 0;
    let mut bit_idx = 0;

    for _ in 0..total_elements {
        if bit_idx == 0 && byte_idx < byte_count {
            // No need to do anything, just read bits
        }
        let byte = data[*pointer + byte_idx];
        let bit = (byte >> (7 - bit_idx)) & 1;
        values.push(bit != 0);

        bit_idx += 1;
        if bit_idx == 8 {
            bit_idx = 0;
            byte_idx += 1;
        }
    }

    *pointer += byte_count;
    Ok(VsfType::t_u0(Tensor::new(shape, values)))
}

fn parse_tensor_data_u8(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u8 tensor"));
    }
    let values = data[*pointer..*pointer + total_elements].to_vec();
    *pointer += total_elements;
    Ok(VsfType::t_u3(Tensor::new(shape, values)))
}

fn parse_tensor_data_u16(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u16 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::t_u4(Tensor::new(shape, values)))
}

fn parse_tensor_data_u32(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u32 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u32::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3]
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_u5(Tensor::new(shape, values)))
}

fn parse_tensor_data_u64(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u64 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u64::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_u6(Tensor::new(shape, values)))
}

fn parse_tensor_data_u128(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for u128 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u128::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
            data[*pointer + 8], data[*pointer + 9], data[*pointer + 10], data[*pointer + 11],
            data[*pointer + 12], data[*pointer + 13], data[*pointer + 14], data[*pointer + 15],
        ]);
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::t_u7(Tensor::new(shape, values)))
}

// Signed integers
fn parse_tensor_data_i8(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i8 tensor"));
    }
    let values: Vec<i8> = data[*pointer..*pointer + total_elements].iter().map(|&b| b as i8).collect();
    *pointer += total_elements;
    Ok(VsfType::t_i3(Tensor::new(shape, values)))
}

fn parse_tensor_data_i16(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i16 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::t_i4(Tensor::new(shape, values)))
}

fn parse_tensor_data_i32(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i32 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i32::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3]
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_i5(Tensor::new(shape, values)))
}

fn parse_tensor_data_i64(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i64 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i64::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_i6(Tensor::new(shape, values)))
}

fn parse_tensor_data_i128(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for i128 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i128::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
            data[*pointer + 8], data[*pointer + 9], data[*pointer + 10], data[*pointer + 11],
            data[*pointer + 12], data[*pointer + 13], data[*pointer + 14], data[*pointer + 15],
        ]);
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::t_i7(Tensor::new(shape, values)))
}

// Floats
fn parse_tensor_data_f32(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f32 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f32::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3]
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_f5(Tensor::new(shape, values)))
}

fn parse_tensor_data_f64(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for f64 tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f64::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_f6(Tensor::new(shape, values)))
}

// Complex
fn parse_tensor_data_c32(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8; // 2 f32s per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for Complex<f32> tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let re = f32::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3]
        ]);
        let im = f32::from_be_bytes([
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7]
        ]);
        values.push(Complex::new(re, im));
        *pointer += 8;
    }
    Ok(VsfType::t_j5(Tensor::new(shape, values)))
}

fn parse_tensor_data_c64(data: &[u8], pointer: &mut usize, shape: Vec<usize>, total_elements: usize) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16; // 2 f64s per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for Complex<f64> tensor"));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let re = f64::from_be_bytes([
            data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
            data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
        ]);
        let im = f64::from_be_bytes([
            data[*pointer + 8], data[*pointer + 9], data[*pointer + 10], data[*pointer + 11],
            data[*pointer + 12], data[*pointer + 13], data[*pointer + 14], data[*pointer + 15],
        ]);
        values.push(Complex::new(re, im));
        *pointer += 16;
    }
    Ok(VsfType::t_j6(Tensor::new(shape, values)))
}

// ==================== STRIDED TENSOR DATA PARSERS ====================
// (Stubs for now - will implement similar to above but with StridedTensor)

fn parse_strided_tensor_data_u0(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u0 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_u8(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u8 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_u16(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u16 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_u32(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u32 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_u64(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u64 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_u128(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided u128 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_i8(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided i8 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_i16(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided i16 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_i32(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided i32 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_i64(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided i64 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_i128(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided i128 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_f32(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided f32 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_f64(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided f64 tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_c32(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided Complex<f32> tensor decoding not yet implemented"))
}

fn parse_strided_tensor_data_c64(_data: &[u8], _pointer: &mut usize, _shape: Vec<usize>, _stride: Vec<usize>, _total: usize) -> Result<VsfType, Error> {
    Err(Error::new(ErrorKind::Other, "Strided Complex<f64> tensor decoding not yet implemented"))
}
