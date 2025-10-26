//! Spirix type parsers

use crate::types::VsfType;
use spirix::*;
use std::io::{Error, ErrorKind};

pub fn parse_spirix_scalar(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse F and E markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for Spirix scalar markers",
        ));
    }

    let f_marker = data[*pointer];
    let e_marker = data[*pointer + 1];
    *pointer += 2;

    // Dispatch based on F×E combination
    match (f_marker, e_marker) {
        (b'3', b'3') => parse_scalar_f3e3(data, pointer),
        (b'3', b'4') => parse_scalar_f3e4(data, pointer),
        (b'3', b'5') => parse_scalar_f3e5(data, pointer),
        (b'3', b'6') => parse_scalar_f3e6(data, pointer),
        (b'3', b'7') => parse_scalar_f3e7(data, pointer),
        (b'4', b'3') => parse_scalar_f4e3(data, pointer),
        (b'4', b'4') => parse_scalar_f4e4(data, pointer),
        (b'4', b'5') => parse_scalar_f4e5(data, pointer),
        (b'4', b'6') => parse_scalar_f4e6(data, pointer),
        (b'4', b'7') => parse_scalar_f4e7(data, pointer),
        (b'5', b'3') => parse_scalar_f5e3(data, pointer),
        (b'5', b'4') => parse_scalar_f5e4(data, pointer),
        (b'5', b'5') => parse_scalar_f5e5(data, pointer),
        (b'5', b'6') => parse_scalar_f5e6(data, pointer),
        (b'5', b'7') => parse_scalar_f5e7(data, pointer),
        (b'6', b'3') => parse_scalar_f6e3(data, pointer),
        (b'6', b'4') => parse_scalar_f6e4(data, pointer),
        (b'6', b'5') => parse_scalar_f6e5(data, pointer),
        (b'6', b'6') => parse_scalar_f6e6(data, pointer),
        (b'6', b'7') => parse_scalar_f6e7(data, pointer),
        (b'7', b'3') => parse_scalar_f7e3(data, pointer),
        (b'7', b'4') => parse_scalar_f7e4(data, pointer),
        (b'7', b'5') => parse_scalar_f7e5(data, pointer),
        (b'7', b'6') => parse_scalar_f7e6(data, pointer),
        (b'7', b'7') => parse_scalar_f7e7(data, pointer),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Unsupported Spirix Scalar type: F{}E{}",
                f_marker as char, e_marker as char
            ),
        )),
    }
}

pub fn parse_spirix_circle(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse F and E markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for Spirix circle markers",
        ));
    }

    let f_marker = data[*pointer];
    let e_marker = data[*pointer + 1];
    *pointer += 2;

    // Dispatch based on F×E combination
    match (f_marker, e_marker) {
        (b'3', b'3') => parse_circle_f3e3(data, pointer),
        (b'3', b'4') => parse_circle_f3e4(data, pointer),
        (b'3', b'5') => parse_circle_f3e5(data, pointer),
        (b'3', b'6') => parse_circle_f3e6(data, pointer),
        (b'3', b'7') => parse_circle_f3e7(data, pointer),
        (b'4', b'3') => parse_circle_f4e3(data, pointer),
        (b'4', b'4') => parse_circle_f4e4(data, pointer),
        (b'4', b'5') => parse_circle_f4e5(data, pointer),
        (b'4', b'6') => parse_circle_f4e6(data, pointer),
        (b'4', b'7') => parse_circle_f4e7(data, pointer),
        (b'5', b'3') => parse_circle_f5e3(data, pointer),
        (b'5', b'4') => parse_circle_f5e4(data, pointer),
        (b'5', b'5') => parse_circle_f5e5(data, pointer),
        (b'5', b'6') => parse_circle_f5e6(data, pointer),
        (b'5', b'7') => parse_circle_f5e7(data, pointer),
        (b'6', b'3') => parse_circle_f6e3(data, pointer),
        (b'6', b'4') => parse_circle_f6e4(data, pointer),
        (b'6', b'5') => parse_circle_f6e5(data, pointer),
        (b'6', b'6') => parse_circle_f6e6(data, pointer),
        (b'6', b'7') => parse_circle_f6e7(data, pointer),
        (b'7', b'3') => parse_circle_f7e3(data, pointer),
        (b'7', b'4') => parse_circle_f7e4(data, pointer),
        (b'7', b'5') => parse_circle_f7e5(data, pointer),
        (b'7', b'6') => parse_circle_f7e6(data, pointer),
        (b'7', b'7') => parse_circle_f7e7(data, pointer),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Unsupported Spirix Circle type: F{}E{}",
                f_marker as char, e_marker as char
            ),
        )),
    }
}

// ==================== SCALAR PARSERS ====================

/// Parse ScalarF3E3: [s][3][3][fraction:i8][exponent:i8]
pub fn parse_scalar_f3e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes), E3 = i8 (1 byte)
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E3",
        ));
    }

    let fraction = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::s33(ScalarF3E3 { fraction, exponent }))
}

/// Parse ScalarF3E4: [s][3][4][fraction:i8][exponent:i16]
pub fn parse_scalar_f3e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes), E4 = i16 (2 bytes)
    if *pointer + 3 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E4",
        ));
    }

    let fraction = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::s34(ScalarF3E4 { fraction, exponent }))
}

/// Parse ScalarF3E5: [s][3][5][fraction:i8][exponent:i32]
pub fn parse_scalar_f3e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes), E5 = i32 (4 bytes)
    if *pointer + 5 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E5",
        ));
    }

    let fraction = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::s35(ScalarF3E5 { fraction, exponent }))
}

/// Parse ScalarF3E6: [s][3][6][fraction:i8][exponent:i64]
pub fn parse_scalar_f3e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes), E6 = i64 (8 bytes)
    if *pointer + 9 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E6",
        ));
    }

    let fraction = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::s36(ScalarF3E6 { fraction, exponent }))
}

/// Parse ScalarF3E7: [s][3][7][fraction:i8][exponent:i128]
pub fn parse_scalar_f3e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes), E7 = i128 (16 bytes)
    if *pointer + 17 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E7",
        ));
    }

    let fraction = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::s37(ScalarF3E7 { fraction, exponent }))
}

/// Parse ScalarF4E3: [s][4][3][fraction:i16][exponent:i8]
pub fn parse_scalar_f4e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes), E3 = i8 (1 byte)
    if *pointer + 3 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E3",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::s43(ScalarF4E3 { fraction, exponent }))
}

/// Parse ScalarF4E4: [s][4][4][fraction:i16][exponent:i16]
pub fn parse_scalar_f4e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes), E4 = i16 (2 bytes)
    if *pointer + 4 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E4",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::s44(ScalarF4E4 { fraction, exponent }))
}

/// Parse ScalarF4E5: [s][4][5][fraction:i16][exponent:i32]
pub fn parse_scalar_f4e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes), E5 = i32 (4 bytes)
    if *pointer + 6 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E5",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::s45(ScalarF4E5 { fraction, exponent }))
}

/// Parse ScalarF4E6: [s][4][6][fraction:i16][exponent:i64]
pub fn parse_scalar_f4e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes), E6 = i64 (8 bytes)
    if *pointer + 10 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E6",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::s46(ScalarF4E6 { fraction, exponent }))
}

/// Parse ScalarF4E7: [s][4][7][fraction:i16][exponent:i128]
pub fn parse_scalar_f4e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes), E7 = i128 (16 bytes)
    if *pointer + 18 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E7",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::s47(ScalarF4E7 { fraction, exponent }))
}

/// Parse ScalarF5E3: [s][5][3][fraction:i32][exponent:i8]
pub fn parse_scalar_f5e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes), E3 = i8 (1 byte)
    if *pointer + 5 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E3",
        ));
    }

    let fraction = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::s53(ScalarF5E3 { fraction, exponent }))
}

/// Parse ScalarF5E4: [s][5][4][fraction:i32][exponent:i16]
pub fn parse_scalar_f5e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes), E4 = i16 (2 bytes)
    if *pointer + 6 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E4",
        ));
    }

    let fraction = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::s54(ScalarF5E4 { fraction, exponent }))
}

/// Parse ScalarF5E5: [s][5][5][fraction:i32][exponent:i32]
pub fn parse_scalar_f5e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes), E5 = i32 (4 bytes)
    if *pointer + 8 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E5",
        ));
    }

    let fraction = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::s55(ScalarF5E5 { fraction, exponent }))
}

/// Parse ScalarF5E6: [s][5][6][fraction:i32][exponent:i64]
pub fn parse_scalar_f5e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes), E6 = i64 (8 bytes)
    if *pointer + 12 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E6",
        ));
    }

    let fraction = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::s56(ScalarF5E6 { fraction, exponent }))
}

/// Parse ScalarF5E7: [s][5][7][fraction:i32][exponent:i128]
pub fn parse_scalar_f5e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes), E7 = i128 (16 bytes)
    if *pointer + 20 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E7",
        ));
    }

    let fraction = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::s57(ScalarF5E7 { fraction, exponent }))
}

/// Parse ScalarF6E3: [s][6][3][fraction:i64][exponent:i8]
pub fn parse_scalar_f6e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes), E3 = i8 (1 byte)
    if *pointer + 9 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E3",
        ));
    }

    let fraction = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::s63(ScalarF6E3 { fraction, exponent }))
}

/// Parse ScalarF6E4: [s][6][4][fraction:i64][exponent:i16]
pub fn parse_scalar_f6e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes), E4 = i16 (2 bytes)
    if *pointer + 10 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E4",
        ));
    }

    let fraction = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::s64(ScalarF6E4 { fraction, exponent }))
}

/// Parse ScalarF6E5: [s][6][5][fraction:i64][exponent:i32]
pub fn parse_scalar_f6e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes), E5 = i32 (4 bytes)
    if *pointer + 12 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E5",
        ));
    }

    let fraction = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::s65(ScalarF6E5 { fraction, exponent }))
}

/// Parse ScalarF6E6: [s][6][6][fraction:i64][exponent:i64]
pub fn parse_scalar_f6e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes), E6 = i64 (8 bytes)
    if *pointer + 16 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E6",
        ));
    }

    let fraction = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::s66(ScalarF6E6 { fraction, exponent }))
}

/// Parse ScalarF6E7: [s][6][7][fraction:i64][exponent:i128]
pub fn parse_scalar_f6e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes), E7 = i128 (16 bytes)
    if *pointer + 24 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E7",
        ));
    }

    let fraction = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::s67(ScalarF6E7 { fraction, exponent }))
}

/// Parse ScalarF7E3: [s][7][3][fraction:i128][exponent:i8]
pub fn parse_scalar_f7e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes), E3 = i8 (1 byte)
    if *pointer + 17 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E3",
        ));
    }

    let fraction = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::s73(ScalarF7E3 { fraction, exponent }))
}

/// Parse ScalarF7E4: [s][7][4][fraction:i128][exponent:i16]
pub fn parse_scalar_f7e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes), E4 = i16 (2 bytes)
    if *pointer + 18 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E4",
        ));
    }

    let fraction = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::s74(ScalarF7E4 { fraction, exponent }))
}

/// Parse ScalarF7E5: [s][7][5][fraction:i128][exponent:i32]
pub fn parse_scalar_f7e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes), E5 = i32 (4 bytes)
    if *pointer + 20 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E5",
        ));
    }

    let fraction = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::s75(ScalarF7E5 { fraction, exponent }))
}

/// Parse ScalarF7E6: [s][7][6][fraction:i128][exponent:i64]
pub fn parse_scalar_f7e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes), E6 = i64 (8 bytes)
    if *pointer + 24 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E6",
        ));
    }

    let fraction = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::s76(ScalarF7E6 { fraction, exponent }))
}

/// Parse ScalarF7E7: [s][7][7][fraction:i128][exponent:i128]
pub fn parse_scalar_f7e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes), E7 = i128 (16 bytes)
    if *pointer + 32 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E7",
        ));
    }

    let fraction = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::s77(ScalarF7E7 { fraction, exponent }))
}

// ==================== CIRCLE PARSERS ====================

/// Parse CircleF3E3: [c][3][3][real:i8][imaginary:i8][exponent:i8]
pub fn parse_circle_f3e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes each for real/imag), E3 = i8 (1 byte)
    if *pointer + 3 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E3",
        ));
    }

    let real = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let imaginary = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::c33(CircleF3E3 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF3E4: [c][3][4][real:i8][imaginary:i8][exponent:i16]
pub fn parse_circle_f3e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes each for real/imag), E4 = i16 (2 bytes)
    if *pointer + 4 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E4",
        ));
    }

    let real = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let imaginary = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::c34(CircleF3E4 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF3E5: [c][3][5][real:i8][imaginary:i8][exponent:i32]
pub fn parse_circle_f3e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes each for real/imag), E5 = i32 (4 bytes)
    if *pointer + 6 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E5",
        ));
    }

    let real = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let imaginary = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::c35(CircleF3E5 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF3E6: [c][3][6][real:i8][imaginary:i8][exponent:i64]
pub fn parse_circle_f3e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes each for real/imag), E6 = i64 (8 bytes)
    if *pointer + 10 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E6",
        ));
    }

    let real = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let imaginary = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::c36(CircleF3E6 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF3E7: [c][3][7][real:i8][imaginary:i8][exponent:i128]
pub fn parse_circle_f3e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F3 = i8 (1 bytes each for real/imag), E7 = i128 (16 bytes)
    if *pointer + 18 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E7",
        ));
    }

    let real = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let imaginary = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::c37(CircleF3E7 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF4E3: [c][4][3][real:i16][imaginary:i16][exponent:i8]
pub fn parse_circle_f4e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes each for real/imag), E3 = i8 (1 byte)
    if *pointer + 5 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E3",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let imaginary = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::c43(CircleF4E3 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF4E4: [c][4][4][real:i16][imaginary:i16][exponent:i16]
pub fn parse_circle_f4e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes each for real/imag), E4 = i16 (2 bytes)
    if *pointer + 6 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E4",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let imaginary = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::c44(CircleF4E4 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF4E5: [c][4][5][real:i16][imaginary:i16][exponent:i32]
pub fn parse_circle_f4e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes each for real/imag), E5 = i32 (4 bytes)
    if *pointer + 8 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E5",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let imaginary = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::c45(CircleF4E5 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF4E6: [c][4][6][real:i16][imaginary:i16][exponent:i64]
pub fn parse_circle_f4e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes each for real/imag), E6 = i64 (8 bytes)
    if *pointer + 12 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E6",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let imaginary = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::c46(CircleF4E6 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF4E7: [c][4][7][real:i16][imaginary:i16][exponent:i128]
pub fn parse_circle_f4e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F4 = i16 (2 bytes each for real/imag), E7 = i128 (16 bytes)
    if *pointer + 20 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E7",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let imaginary = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::c47(CircleF4E7 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF5E3: [c][5][3][real:i32][imaginary:i32][exponent:i8]
pub fn parse_circle_f5e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes each for real/imag), E3 = i8 (1 byte)
    if *pointer + 9 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E3",
        ));
    }

    let real = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let imaginary = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::c53(CircleF5E3 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF5E4: [c][5][4][real:i32][imaginary:i32][exponent:i16]
pub fn parse_circle_f5e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes each for real/imag), E4 = i16 (2 bytes)
    if *pointer + 10 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E4",
        ));
    }

    let real = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let imaginary = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::c54(CircleF5E4 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF5E5: [c][5][5][real:i32][imaginary:i32][exponent:i32]
pub fn parse_circle_f5e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes each for real/imag), E5 = i32 (4 bytes)
    if *pointer + 12 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E5",
        ));
    }

    let real = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let imaginary = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::c55(CircleF5E5 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF5E6: [c][5][6][real:i32][imaginary:i32][exponent:i64]
pub fn parse_circle_f5e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes each for real/imag), E6 = i64 (8 bytes)
    if *pointer + 16 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E6",
        ));
    }

    let real = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let imaginary = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::c56(CircleF5E6 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF5E7: [c][5][7][real:i32][imaginary:i32][exponent:i128]
pub fn parse_circle_f5e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F5 = i32 (4 bytes each for real/imag), E7 = i128 (16 bytes)
    if *pointer + 24 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E7",
        ));
    }

    let real = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let imaginary = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::c57(CircleF5E7 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF6E3: [c][6][3][real:i64][imaginary:i64][exponent:i8]
pub fn parse_circle_f6e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes each for real/imag), E3 = i8 (1 byte)
    if *pointer + 17 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E3",
        ));
    }

    let real = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let imaginary = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::c63(CircleF6E3 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF6E4: [c][6][4][real:i64][imaginary:i64][exponent:i16]
pub fn parse_circle_f6e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes each for real/imag), E4 = i16 (2 bytes)
    if *pointer + 18 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E4",
        ));
    }

    let real = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let imaginary = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::c64(CircleF6E4 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF6E5: [c][6][5][real:i64][imaginary:i64][exponent:i32]
pub fn parse_circle_f6e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes each for real/imag), E5 = i32 (4 bytes)
    if *pointer + 20 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E5",
        ));
    }

    let real = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let imaginary = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::c65(CircleF6E5 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF6E6: [c][6][6][real:i64][imaginary:i64][exponent:i64]
pub fn parse_circle_f6e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes each for real/imag), E6 = i64 (8 bytes)
    if *pointer + 24 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E6",
        ));
    }

    let real = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let imaginary = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::c66(CircleF6E6 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF6E7: [c][6][7][real:i64][imaginary:i64][exponent:i128]
pub fn parse_circle_f6e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F6 = i64 (8 bytes each for real/imag), E7 = i128 (16 bytes)
    if *pointer + 32 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E7",
        ));
    }

    let real = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let imaginary = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::c67(CircleF6E7 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF7E3: [c][7][3][real:i128][imaginary:i128][exponent:i8]
pub fn parse_circle_f7e3(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes each for real/imag), E3 = i8 (1 byte)
    if *pointer + 33 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E3",
        ));
    }

    let real = i128::from_be_bytes([
        data[*pointer + 0],
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

    let imaginary = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i8::from_be_bytes([data[*pointer]]);
    *pointer += 1;

    Ok(VsfType::c73(CircleF7E3 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF7E4: [c][7][4][real:i128][imaginary:i128][exponent:i16]
pub fn parse_circle_f7e4(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes each for real/imag), E4 = i16 (2 bytes)
    if *pointer + 34 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E4",
        ));
    }

    let real = i128::from_be_bytes([
        data[*pointer + 0],
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

    let imaginary = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i16::from_be_bytes([data[*pointer + 0], data[*pointer + 1]]);
    *pointer += 2;

    Ok(VsfType::c74(CircleF7E4 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF7E5: [c][7][5][real:i128][imaginary:i128][exponent:i32]
pub fn parse_circle_f7e5(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes each for real/imag), E5 = i32 (4 bytes)
    if *pointer + 36 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E5",
        ));
    }

    let real = i128::from_be_bytes([
        data[*pointer + 0],
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

    let imaginary = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i32::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
    ]);
    *pointer += 4;

    Ok(VsfType::c75(CircleF7E5 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF7E6: [c][7][6][real:i128][imaginary:i128][exponent:i64]
pub fn parse_circle_f7e6(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes each for real/imag), E6 = i64 (8 bytes)
    if *pointer + 40 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E6",
        ));
    }

    let real = i128::from_be_bytes([
        data[*pointer + 0],
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

    let imaginary = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i64::from_be_bytes([
        data[*pointer + 0],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::c76(CircleF7E6 {
        real,
        imaginary,
        exponent,
    }))
}

/// Parse CircleF7E7: [c][7][7][real:i128][imaginary:i128][exponent:i128]
pub fn parse_circle_f7e7(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // F7 = i128 (16 bytes each for real/imag), E7 = i128 (16 bytes)
    if *pointer + 48 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E7",
        ));
    }

    let real = i128::from_be_bytes([
        data[*pointer + 0],
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

    let imaginary = i128::from_be_bytes([
        data[*pointer + 0],
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

    let exponent = i128::from_be_bytes([
        data[*pointer + 0],
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

    Ok(VsfType::c77(CircleF7E7 {
        real,
        imaginary,
        exponent,
    }))
}
