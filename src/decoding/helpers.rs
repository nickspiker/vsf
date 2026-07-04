use crate::decoding::traits::DecodeError;
use crate::prelude::*;

/// Decode a variable-length usize from VSF format
pub fn decode_usize(data: &[u8], pointer: &mut usize) -> Result<usize, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Not enough data for size marker".into(),
        ));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u8".into(),
                ));
            }
            let value = data[*pointer] as usize;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u16".into(),
                ));
            }
            let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as usize;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u32".into(),
                ));
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
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u64".into(),
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
            ]) as usize;
            *pointer += 8;
            Ok(value)
        }
        b'7' => {
            *pointer += 1;
            if *pointer + 16 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u128".into(),
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
            ]) as usize;
            *pointer += 16;
            Ok(value)
        }
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Invalid usize size marker: {}",
            data[*pointer]
        ))),
    }
}

/// Decode a variable-length isize from VSF format
pub fn decode_isize(data: &[u8], pointer: &mut usize) -> Result<isize, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Not enough data for size marker".into(),
        ));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i8".into(),
                ));
            }
            let value = data[*pointer] as i8 as isize;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i16".into(),
                ));
            }
            let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as isize;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i32".into(),
                ));
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
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i64".into(),
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
            ]) as isize;
            *pointer += 8;
            Ok(value)
        }
        b'7' => {
            *pointer += 1;
            if *pointer + 16 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i128".into(),
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
            ]) as isize;
            *pointer += 16;
            Ok(value)
        }
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Invalid isize size marker: {}",
            data[*pointer]
        ))),
    }
}

/// Decode a variable-length u64 from VSF format (no truncation on 32-bit targets)
pub fn decode_u64(data: &[u8], pointer: &mut usize) -> Result<u64, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Not enough data for size marker".into(),
        ));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u8".into(),
                ));
            }
            let value = data[*pointer] as u64;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u16".into(),
                ));
            }
            let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as u64;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u32".into(),
                ));
            }
            let value = u32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]) as u64;
            *pointer += 4;
            Ok(value)
        }
        b'6' => {
            *pointer += 1;
            if *pointer + 8 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for u64".into(),
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
            Ok(value)
        }
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Invalid u64 size marker: {}",
            data[*pointer]
        ))),
    }
}

/// Decode a variable-length i64 from VSF format (no truncation on 32-bit targets)
pub fn decode_i64(data: &[u8], pointer: &mut usize) -> Result<i64, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg(
            "Not enough data for size marker".into(),
        ));
    }

    match data[*pointer] {
        b'3' => {
            *pointer += 1;
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i8".into(),
                ));
            }
            let value = data[*pointer] as i8 as i64;
            *pointer += 1;
            Ok(value)
        }
        b'4' => {
            *pointer += 1;
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i16".into(),
                ));
            }
            let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as i64;
            *pointer += 2;
            Ok(value)
        }
        b'5' => {
            *pointer += 1;
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i32".into(),
                ));
            }
            let value = i32::from_be_bytes([
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ]) as i64;
            *pointer += 4;
            Ok(value)
        }
        b'6' => {
            *pointer += 1;
            if *pointer + 8 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "Not enough data for i64".into(),
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
            Ok(value)
        }
        _ => Err(DecodeError::InvalidDataMsg(format!(
            "Invalid i64 size marker: {}",
            data[*pointer]
        ))),
    }
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
