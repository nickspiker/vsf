//! Primitive type parsers (unsigned, signed, float, complex)

use super::helpers::{decode_isize, decode_usize};
use crate::types::VsfType;
use num_complex::Complex;
use std::io::{Error, ErrorKind};

// ==================== UNSIGNED INTEGERS ====================

pub fn parse_unsigned(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for unsigned size marker",
        ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u0",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u3",
                ));
            }
            let value = data[*pointer];
            *pointer += 1;
            Ok(VsfType::u3(value))
        }
        b'4' => {
            // u4: u16
            if *pointer + 2 > data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u4",
                ));
            }
            let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::u4(value))
        }
        b'5' => {
            // u5: u32
            if *pointer + 4 > data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u5",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u6",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for u7",
                ));
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

pub fn parse_signed(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signed size marker",
        ));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'3' => {
            // i3: i8
            if *pointer >= data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for i3",
                ));
            }
            let value = data[*pointer] as i8;
            *pointer += 1;
            Ok(VsfType::i3(value))
        }
        b'4' => {
            // i4: i16
            if *pointer + 2 > data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for i4",
                ));
            }
            let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::i4(value))
        }
        b'5' => {
            // i5: i32
            if *pointer + 4 > data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for i5",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for i6",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for i7",
                ));
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

pub fn parse_float(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for float size marker",
        ));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'5' => {
            // f5: f32
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
            Ok(VsfType::f5(value))
        }
        b'6' => {
            // f6: f64
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
            Ok(VsfType::f6(value))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid float size marker",
        )),
    }
}

// ==================== IEEE COMPLEX ====================

pub fn parse_complex(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for complex size marker",
        ));
    }

    let size_byte = data[*pointer];
    *pointer += 1;

    match size_byte {
        b'5' => {
            // j5: Complex<f32>
            if *pointer + 8 > data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for j5",
                ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for j6",
                ));
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
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid complex size marker",
        )),
    }
}
