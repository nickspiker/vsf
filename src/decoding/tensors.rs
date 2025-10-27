//! Tensor parsers

use super::helpers::{decode_usize, parse_shape};
use super::spirix::{
    parse_circle_f3e3, parse_circle_f3e4, parse_circle_f3e5, parse_circle_f3e6, parse_circle_f3e7,
    parse_circle_f4e3, parse_circle_f4e4, parse_circle_f4e5, parse_circle_f4e6, parse_circle_f4e7,
    parse_circle_f5e3, parse_circle_f5e4, parse_circle_f5e5, parse_circle_f5e6, parse_circle_f5e7,
    parse_circle_f6e3, parse_circle_f6e4, parse_circle_f6e5, parse_circle_f6e6, parse_circle_f6e7,
    parse_circle_f7e3, parse_circle_f7e4, parse_circle_f7e5, parse_circle_f7e6, parse_circle_f7e7,
    parse_scalar_f3e3, parse_scalar_f3e4, parse_scalar_f3e5, parse_scalar_f3e6, parse_scalar_f3e7,
    parse_scalar_f4e3, parse_scalar_f4e4, parse_scalar_f4e5, parse_scalar_f4e6, parse_scalar_f4e7,
    parse_scalar_f5e3, parse_scalar_f5e4, parse_scalar_f5e5, parse_scalar_f5e6, parse_scalar_f5e7,
    parse_scalar_f6e3, parse_scalar_f6e4, parse_scalar_f6e5, parse_scalar_f6e6, parse_scalar_f6e7,
    parse_scalar_f7e3, parse_scalar_f7e4, parse_scalar_f7e5, parse_scalar_f7e6, parse_scalar_f7e7,
};
use crate::types::{BitPackedTensor, StridedTensor, Tensor, VsfType};
use num_complex::Complex;
use std::io::{Error, ErrorKind};

// ==================== BITPACKED TENSORS ====================

/// Parse bitpacked tensor: [p][ndim][bit_depth][shape...][data...]
pub fn parse_bitpacked_tensor(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse dimension count
    let ndim = decode_usize(data, pointer)?;

    // Parse bit depth (0x01-0xFF, where 0x00 = 256-bit)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for bit depth",
        ));
    }
    let bit_depth = data[*pointer];
    *pointer += 1;

    // Parse shape
    let shape = parse_shape(data, pointer, ndim)?;

    // Calculate expected byte count
    let total_elements: usize = shape.iter().product();
    let bits_per_sample = if bit_depth == 0 {
        256
    } else {
        bit_depth as usize
    };
    let total_bits = total_elements * bits_per_sample;
    let byte_count = (total_bits + 7) / 8;

    // Read packed data
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            format!(
                "Not enough data for bitpacked tensor: need {} bytes, have {}",
                byte_count,
                data.len() - *pointer
            ),
        ));
    }

    let packed_data = data[*pointer..*pointer + byte_count].to_vec();
    *pointer += byte_count;

    Ok(VsfType::p(BitPackedTensor {
        bit_depth,
        shape,
        data: packed_data,
    }))
}

// ==================== TENSORS ====================

/// Parse contiguous tensor: [t][ndim][elem_type][elem_size][shape...][data...]
pub fn parse_tensor(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse dimension count
    let ndim = decode_usize(data, pointer)?;

    // Parse element type markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for element type",
        ));
    }
    let elem_type = data[*pointer];
    *pointer += 1;
    let elem_size = data[*pointer];
    *pointer += 1;

    // Parse shape
    let shape = parse_shape(data, pointer, ndim)?;
    let total_elements: usize = shape.iter().product();

    // Dispatch based on element type
    match elem_type {
        b'u' => match elem_size {
            b'0' => parse_tensor_data_u0(data, pointer, shape, total_elements),
            b'3' => parse_tensor_data_u8(data, pointer, shape, total_elements),
            b'4' => parse_tensor_data_u16(data, pointer, shape, total_elements),
            b'5' => parse_tensor_data_u32(data, pointer, shape, total_elements),
            b'6' => parse_tensor_data_u64(data, pointer, shape, total_elements),
            b'7' => parse_tensor_data_u128(data, pointer, shape, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid unsigned size: {}", elem_size as char),
            )),
        },
        b'i' => match elem_size {
            b'3' => parse_tensor_data_i8(data, pointer, shape, total_elements),
            b'4' => parse_tensor_data_i16(data, pointer, shape, total_elements),
            b'5' => parse_tensor_data_i32(data, pointer, shape, total_elements),
            b'6' => parse_tensor_data_i64(data, pointer, shape, total_elements),
            b'7' => parse_tensor_data_i128(data, pointer, shape, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid signed size: {}", elem_size as char),
            )),
        },
        b'f' => match elem_size {
            b'5' => parse_tensor_data_f32(data, pointer, shape, total_elements),
            b'6' => parse_tensor_data_f64(data, pointer, shape, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid float size: {}", elem_size as char),
            )),
        },
        b'j' => match elem_size {
            b'5' => parse_tensor_data_j32(data, pointer, shape, total_elements),
            b'6' => parse_tensor_data_j64(data, pointer, shape, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid complex size: {}", elem_size as char),
            )),
        },
        b's' => {
            // Spirix Scalar: elem_size is F marker, need to read E marker
            let f_marker = elem_size;
            let e_marker = data[*pointer];
            *pointer += 1;
            match (f_marker, e_marker) {
                (b'3', b'3') => parse_tensor_data_s33(data, pointer, shape, total_elements),
                (b'3', b'4') => parse_tensor_data_s34(data, pointer, shape, total_elements),
                (b'3', b'5') => parse_tensor_data_s35(data, pointer, shape, total_elements),
                (b'3', b'6') => parse_tensor_data_s36(data, pointer, shape, total_elements),
                (b'3', b'7') => parse_tensor_data_s37(data, pointer, shape, total_elements),
                (b'4', b'3') => parse_tensor_data_s43(data, pointer, shape, total_elements),
                (b'4', b'4') => parse_tensor_data_s44(data, pointer, shape, total_elements),
                (b'4', b'5') => parse_tensor_data_s45(data, pointer, shape, total_elements),
                (b'4', b'6') => parse_tensor_data_s46(data, pointer, shape, total_elements),
                (b'4', b'7') => parse_tensor_data_s47(data, pointer, shape, total_elements),
                (b'5', b'3') => parse_tensor_data_s53(data, pointer, shape, total_elements),
                (b'5', b'4') => parse_tensor_data_s54(data, pointer, shape, total_elements),
                (b'5', b'5') => parse_tensor_data_s55(data, pointer, shape, total_elements),
                (b'5', b'6') => parse_tensor_data_s56(data, pointer, shape, total_elements),
                (b'5', b'7') => parse_tensor_data_s57(data, pointer, shape, total_elements),
                (b'6', b'3') => parse_tensor_data_s63(data, pointer, shape, total_elements),
                (b'6', b'4') => parse_tensor_data_s64(data, pointer, shape, total_elements),
                (b'6', b'5') => parse_tensor_data_s65(data, pointer, shape, total_elements),
                (b'6', b'6') => parse_tensor_data_s66(data, pointer, shape, total_elements),
                (b'6', b'7') => parse_tensor_data_s67(data, pointer, shape, total_elements),
                (b'7', b'3') => parse_tensor_data_s73(data, pointer, shape, total_elements),
                (b'7', b'4') => parse_tensor_data_s74(data, pointer, shape, total_elements),
                (b'7', b'5') => parse_tensor_data_s75(data, pointer, shape, total_elements),
                (b'7', b'6') => parse_tensor_data_s76(data, pointer, shape, total_elements),
                (b'7', b'7') => parse_tensor_data_s77(data, pointer, shape, total_elements),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid Spirix Scalar F{}E{}",
                        f_marker as char, e_marker as char
                    ),
                )),
            }
        }
        b'c' => {
            // Spirix Circle: elem_size is F marker, need to read E marker
            let f_marker = elem_size;
            let e_marker = data[*pointer];
            *pointer += 1;
            match (f_marker, e_marker) {
                (b'3', b'3') => parse_tensor_data_c33(data, pointer, shape, total_elements),
                (b'3', b'4') => parse_tensor_data_c34(data, pointer, shape, total_elements),
                (b'3', b'5') => parse_tensor_data_c35(data, pointer, shape, total_elements),
                (b'3', b'6') => parse_tensor_data_c36(data, pointer, shape, total_elements),
                (b'3', b'7') => parse_tensor_data_c37(data, pointer, shape, total_elements),
                (b'4', b'3') => parse_tensor_data_c43(data, pointer, shape, total_elements),
                (b'4', b'4') => parse_tensor_data_c44(data, pointer, shape, total_elements),
                (b'4', b'5') => parse_tensor_data_c45(data, pointer, shape, total_elements),
                (b'4', b'6') => parse_tensor_data_c46(data, pointer, shape, total_elements),
                (b'4', b'7') => parse_tensor_data_c47(data, pointer, shape, total_elements),
                (b'5', b'3') => parse_tensor_data_c53(data, pointer, shape, total_elements),
                (b'5', b'4') => parse_tensor_data_c54(data, pointer, shape, total_elements),
                (b'5', b'5') => parse_tensor_data_c55(data, pointer, shape, total_elements),
                (b'5', b'6') => parse_tensor_data_c56(data, pointer, shape, total_elements),
                (b'5', b'7') => parse_tensor_data_c57(data, pointer, shape, total_elements),
                (b'6', b'3') => parse_tensor_data_c63(data, pointer, shape, total_elements),
                (b'6', b'4') => parse_tensor_data_j64(data, pointer, shape, total_elements),
                (b'6', b'5') => parse_tensor_data_c65(data, pointer, shape, total_elements),
                (b'6', b'6') => parse_tensor_data_c66(data, pointer, shape, total_elements),
                (b'6', b'7') => parse_tensor_data_c67(data, pointer, shape, total_elements),
                (b'7', b'3') => parse_tensor_data_c73(data, pointer, shape, total_elements),
                (b'7', b'4') => parse_tensor_data_c74(data, pointer, shape, total_elements),
                (b'7', b'5') => parse_tensor_data_c75(data, pointer, shape, total_elements),
                (b'7', b'6') => parse_tensor_data_c76(data, pointer, shape, total_elements),
                (b'7', b'7') => parse_tensor_data_c77(data, pointer, shape, total_elements),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid Spirix Circle F{}E{}",
                        f_marker as char, e_marker as char
                    ),
                )),
            }
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid tensor element type: {}", elem_type as char),
        )),
    }
}

/// Parse strided tensor: [q][ndim][elem_type][elem_size][shape...][stride...][data...]
pub fn parse_strided_tensor(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Parse dimension count
    let ndim = decode_usize(data, pointer)?;

    // Parse element type markers
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for element type",
        ));
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
    match elem_type {
        b'u' => match elem_size {
            b'0' => parse_strided_tensor_data_u0(data, pointer, shape, stride, total_elements),
            b'3' => parse_strided_tensor_data_u8(data, pointer, shape, stride, total_elements),
            b'4' => parse_strided_tensor_data_u16(data, pointer, shape, stride, total_elements),
            b'5' => parse_strided_tensor_data_u32(data, pointer, shape, stride, total_elements),
            b'6' => parse_strided_tensor_data_u64(data, pointer, shape, stride, total_elements),
            b'7' => parse_strided_tensor_data_u128(data, pointer, shape, stride, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid strided unsigned size: {}", elem_size as char),
            )),
        },
        b'i' => match elem_size {
            b'3' => parse_strided_tensor_data_i8(data, pointer, shape, stride, total_elements),
            b'4' => parse_strided_tensor_data_i16(data, pointer, shape, stride, total_elements),
            b'5' => parse_strided_tensor_data_i32(data, pointer, shape, stride, total_elements),
            b'6' => parse_strided_tensor_data_i64(data, pointer, shape, stride, total_elements),
            b'7' => parse_strided_tensor_data_i128(data, pointer, shape, stride, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid strided signed size: {}", elem_size as char),
            )),
        },
        b'f' => match elem_size {
            b'5' => parse_strided_tensor_data_f32(data, pointer, shape, stride, total_elements),
            b'6' => parse_strided_tensor_data_f64(data, pointer, shape, stride, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid strided float size: {}", elem_size as char),
            )),
        },
        b'j' => match elem_size {
            b'5' => parse_strided_tensor_data_j32(data, pointer, shape, stride, total_elements),
            b'6' => parse_strided_tensor_data_j64(data, pointer, shape, stride, total_elements),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid strided complex size: {}", elem_size as char),
            )),
        },
        b's' => {
            // Spirix Scalar: elem_size is F marker, need to read E marker
            let f_marker = elem_size;
            let e_marker = data[*pointer];
            *pointer += 1;
            match (f_marker, e_marker) {
                (b'3', b'3') => {
                    parse_strided_tensor_data_s33(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'4') => {
                    parse_strided_tensor_data_s34(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'5') => {
                    parse_strided_tensor_data_s35(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'6') => {
                    parse_strided_tensor_data_s36(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'7') => {
                    parse_strided_tensor_data_s37(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'3') => {
                    parse_strided_tensor_data_s43(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'4') => {
                    parse_strided_tensor_data_s44(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'5') => {
                    parse_strided_tensor_data_s45(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'6') => {
                    parse_strided_tensor_data_s46(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'7') => {
                    parse_strided_tensor_data_s47(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'3') => {
                    parse_strided_tensor_data_s53(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'4') => {
                    parse_strided_tensor_data_s54(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'5') => {
                    parse_strided_tensor_data_s55(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'6') => {
                    parse_strided_tensor_data_s56(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'7') => {
                    parse_strided_tensor_data_s57(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'3') => {
                    parse_strided_tensor_data_s63(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'4') => {
                    parse_strided_tensor_data_s64(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'5') => {
                    parse_strided_tensor_data_s65(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'6') => {
                    parse_strided_tensor_data_s66(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'7') => {
                    parse_strided_tensor_data_s67(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'3') => {
                    parse_strided_tensor_data_s73(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'4') => {
                    parse_strided_tensor_data_s74(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'5') => {
                    parse_strided_tensor_data_s75(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'6') => {
                    parse_strided_tensor_data_s76(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'7') => {
                    parse_strided_tensor_data_s77(data, pointer, shape, stride, total_elements)
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid strided Spirix Scalar F{}E{}",
                        f_marker as char, e_marker as char
                    ),
                )),
            }
        }
        b'c' => {
            // Spirix Circle: elem_size is F marker, need to read E marker
            let f_marker = elem_size;
            let e_marker = data[*pointer];
            *pointer += 1;
            match (f_marker, e_marker) {
                (b'3', b'3') => {
                    parse_strided_tensor_data_c33(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'4') => {
                    parse_strided_tensor_data_c34(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'5') => {
                    parse_strided_tensor_data_c35(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'6') => {
                    parse_strided_tensor_data_c36(data, pointer, shape, stride, total_elements)
                }
                (b'3', b'7') => {
                    parse_strided_tensor_data_c37(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'3') => {
                    parse_strided_tensor_data_c43(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'4') => {
                    parse_strided_tensor_data_c44(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'5') => {
                    parse_strided_tensor_data_c45(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'6') => {
                    parse_strided_tensor_data_c46(data, pointer, shape, stride, total_elements)
                }
                (b'4', b'7') => {
                    parse_strided_tensor_data_c47(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'3') => {
                    parse_strided_tensor_data_c53(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'4') => {
                    parse_strided_tensor_data_c54(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'5') => {
                    parse_strided_tensor_data_c55(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'6') => {
                    parse_strided_tensor_data_c56(data, pointer, shape, stride, total_elements)
                }
                (b'5', b'7') => {
                    parse_strided_tensor_data_c57(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'3') => {
                    parse_strided_tensor_data_c63(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'4') => {
                    parse_strided_tensor_data_j64(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'5') => {
                    parse_strided_tensor_data_c65(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'6') => {
                    parse_strided_tensor_data_c66(data, pointer, shape, stride, total_elements)
                }
                (b'6', b'7') => {
                    parse_strided_tensor_data_c67(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'3') => {
                    parse_strided_tensor_data_c73(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'4') => {
                    parse_strided_tensor_data_c74(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'5') => {
                    parse_strided_tensor_data_c75(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'6') => {
                    parse_strided_tensor_data_c76(data, pointer, shape, stride, total_elements)
                }
                (b'7', b'7') => {
                    parse_strided_tensor_data_c77(data, pointer, shape, stride, total_elements)
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid strided Spirix Circle F{}E{}",
                        f_marker as char, e_marker as char
                    ),
                )),
            }
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid strided tensor element type: {}", elem_type as char),
        )),
    }
}

// ==================== TENSOR DATA PARSERS ====================

// Unsigned integers
pub fn parse_tensor_data_u0(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    // Bitpacked: 8 bools per byte
    let byte_count = (total_elements + 7) / 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u0 tensor",
        ));
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

pub fn parse_tensor_data_u8(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u8 tensor",
        ));
    }
    let values = data[*pointer..*pointer + total_elements].to_vec();
    *pointer += total_elements;
    Ok(VsfType::t_u3(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_u16(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u16 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::t_u4(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_u32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_u5(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_u64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_u6(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_u128(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u128 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u128::from_be_bytes([
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
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::t_u7(Tensor::new(shape, values)))
}

// Signed integers
pub fn parse_tensor_data_i8(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for i8 tensor",
        ));
    }
    let values: Vec<i8> = data[*pointer..*pointer + total_elements]
        .iter()
        .map(|&b| b as i8)
        .collect();
    *pointer += total_elements;
    Ok(VsfType::t_i3(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_i16(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for i16 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::t_i4(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_i32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for i32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_i5(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_i64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for i64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_i6(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_i128(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for i128 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i128::from_be_bytes([
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
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::t_i7(Tensor::new(shape, values)))
}

// Floats
pub fn parse_tensor_data_f32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for f32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::t_f5(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_f64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for f64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::t_f6(Tensor::new(shape, values)))
}

// Complex
pub fn parse_tensor_data_j32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8; // 2 f32s per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for Complex<f32> tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let re = f32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        let im = f32::from_be_bytes([
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(Complex::new(re, im));
        *pointer += 8;
    }
    Ok(VsfType::t_j5(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_j64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16; // 2 f64s per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for Complex<f64> tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
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
        let im = f64::from_be_bytes([
            data[*pointer + 8],
            data[*pointer + 9],
            data[*pointer + 10],
            data[*pointer + 11],
            data[*pointer + 12],
            data[*pointer + 13],
            data[*pointer + 14],
            data[*pointer + 15],
        ]);
        values.push(Complex::new(re, im));
        *pointer += 16;
    }
    Ok(VsfType::t_j6(Tensor::new(shape, values)))
}

// ==================== STRIDED TENSOR DATA PARSERS ====================

pub fn parse_strided_tensor_data_u0(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    // Bitpacked: 8 bools per byte
    let byte_count = (total_elements + 7) / 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u0 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    let mut byte_idx = 0;
    let mut bit_idx = 0;

    for _ in 0..total_elements {
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
    Ok(VsfType::q_u0(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_u8(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u8 tensor",
        ));
    }
    let values = data[*pointer..*pointer + total_elements].to_vec();
    *pointer += total_elements;
    Ok(VsfType::q_u3(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_u16(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u16 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::q_u4(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_u32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::q_u5(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_u64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::q_u6(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_u128(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided u128 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = u128::from_be_bytes([
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
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::q_u7(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_i8(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    if *pointer + total_elements > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided i8 tensor",
        ));
    }
    let values: Vec<i8> = data[*pointer..*pointer + total_elements]
        .iter()
        .map(|&b| b as i8)
        .collect();
    *pointer += total_elements;
    Ok(VsfType::q_i3(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_i16(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided i16 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
        values.push(val);
        *pointer += 2;
    }
    Ok(VsfType::q_i4(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_i32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided i32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::q_i5(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_i64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided i64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::q_i6(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_i128(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided i128 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = i128::from_be_bytes([
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
        values.push(val);
        *pointer += 16;
    }
    Ok(VsfType::q_i7(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_f32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided f32 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f32::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        values.push(val);
        *pointer += 4;
    }
    Ok(VsfType::q_f5(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_f64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided f64 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = f64::from_be_bytes([
            data[*pointer],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(val);
        *pointer += 8;
    }
    Ok(VsfType::q_f6(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_j32(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8; // 2 floats per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided Complex<f32> tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let re = f32::from_be_bytes([
            data[*pointer + 0],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
        ]);
        let im = f32::from_be_bytes([
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        values.push(Complex::new(re, im));
        *pointer += 8;
    }
    Ok(VsfType::q_j5(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_j64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16; // 2 floats per complex
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided Complex<f64> tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let re = f64::from_be_bytes([
            data[*pointer + 0],
            data[*pointer + 1],
            data[*pointer + 2],
            data[*pointer + 3],
            data[*pointer + 4],
            data[*pointer + 5],
            data[*pointer + 6],
            data[*pointer + 7],
        ]);
        let im = f64::from_be_bytes([
            data[*pointer + 8],
            data[*pointer + 9],
            data[*pointer + 10],
            data[*pointer + 11],
            data[*pointer + 12],
            data[*pointer + 13],
            data[*pointer + 14],
            data[*pointer + 15],
        ]);
        values.push(Complex::new(re, im));
        *pointer += 16;
    }
    Ok(VsfType::q_j6(StridedTensor::new(shape, stride, values)))
}
// ==================== SPIRIX SCALAR TENSOR DATA PARSERS ====================

pub fn parse_tensor_data_s33(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e3(data, pointer)?;
        if let VsfType::s33(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E3"));
        }
    }
    Ok(VsfType::t_s33(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s34(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e4(data, pointer)?;
        if let VsfType::s34(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E4"));
        }
    }
    Ok(VsfType::t_s34(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s35(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e5(data, pointer)?;
        if let VsfType::s35(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E5"));
        }
    }
    Ok(VsfType::t_s35(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s36(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e6(data, pointer)?;
        if let VsfType::s36(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E6"));
        }
    }
    Ok(VsfType::t_s36(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s37(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF3E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e7(data, pointer)?;
        if let VsfType::s37(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E7"));
        }
    }
    Ok(VsfType::t_s37(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s43(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e3(data, pointer)?;
        if let VsfType::s43(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E3"));
        }
    }
    Ok(VsfType::t_s43(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s44(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e4(data, pointer)?;
        if let VsfType::s44(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E4"));
        }
    }
    Ok(VsfType::t_s44(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s45(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e5(data, pointer)?;
        if let VsfType::s45(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E5"));
        }
    }
    Ok(VsfType::t_s45(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s46(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e6(data, pointer)?;
        if let VsfType::s46(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E6"));
        }
    }
    Ok(VsfType::t_s46(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s47(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e7(data, pointer)?;
        if let VsfType::s47(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E7"));
        }
    }
    Ok(VsfType::t_s47(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s53(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e3(data, pointer)?;
        if let VsfType::s53(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E3"));
        }
    }
    Ok(VsfType::t_s53(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s54(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e4(data, pointer)?;
        if let VsfType::s54(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E4"));
        }
    }
    Ok(VsfType::t_s54(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s55(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e5(data, pointer)?;
        if let VsfType::s55(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E5"));
        }
    }
    Ok(VsfType::t_s55(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s56(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e6(data, pointer)?;
        if let VsfType::s56(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E6"));
        }
    }
    Ok(VsfType::t_s56(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s57(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF5E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e7(data, pointer)?;
        if let VsfType::s57(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E7"));
        }
    }
    Ok(VsfType::t_s57(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s63(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e3(data, pointer)?;
        if let VsfType::s63(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E3"));
        }
    }
    Ok(VsfType::t_s63(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e4(data, pointer)?;
        if let VsfType::s64(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E4"));
        }
    }
    Ok(VsfType::t_s64(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s65(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e5(data, pointer)?;
        if let VsfType::s65(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E5"));
        }
    }
    Ok(VsfType::t_s65(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s66(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e6(data, pointer)?;
        if let VsfType::s66(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E6"));
        }
    }
    Ok(VsfType::t_s66(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s67(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF6E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e7(data, pointer)?;
        if let VsfType::s67(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E7"));
        }
    }
    Ok(VsfType::t_s67(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s73(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e3(data, pointer)?;
        if let VsfType::s73(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E3"));
        }
    }
    Ok(VsfType::t_s73(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s74(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e4(data, pointer)?;
        if let VsfType::s74(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E4"));
        }
    }
    Ok(VsfType::t_s74(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s75(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e5(data, pointer)?;
        if let VsfType::s75(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E5"));
        }
    }
    Ok(VsfType::t_s75(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s76(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e6(data, pointer)?;
        if let VsfType::s76(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E6"));
        }
    }
    Ok(VsfType::t_s76(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_s77(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 32;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF7E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e7(data, pointer)?;
        if let VsfType::s77(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E7"));
        }
    }
    Ok(VsfType::t_s77(Tensor::new(shape, values)))
}

// ==================== SPIRIX CIRCLE TENSOR DATA PARSERS ====================

pub fn parse_tensor_data_c33(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e3(data, pointer)?;
        if let VsfType::c33(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E3"));
        }
    }
    Ok(VsfType::t_c33(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c34(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e4(data, pointer)?;
        if let VsfType::c34(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E4"));
        }
    }
    Ok(VsfType::t_c34(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c35(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e5(data, pointer)?;
        if let VsfType::c35(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E5"));
        }
    }
    Ok(VsfType::t_c35(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c36(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e6(data, pointer)?;
        if let VsfType::c36(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E6"));
        }
    }
    Ok(VsfType::t_c36(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c37(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF3E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e7(data, pointer)?;
        if let VsfType::c37(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E7"));
        }
    }
    Ok(VsfType::t_c37(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c43(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e3(data, pointer)?;
        if let VsfType::c43(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E3"));
        }
    }
    Ok(VsfType::t_c43(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c44(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e4(data, pointer)?;
        if let VsfType::c44(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E4"));
        }
    }
    Ok(VsfType::t_c44(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c45(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e5(data, pointer)?;
        if let VsfType::c45(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E5"));
        }
    }
    Ok(VsfType::t_c45(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c46(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e6(data, pointer)?;
        if let VsfType::c46(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E6"));
        }
    }
    Ok(VsfType::t_c46(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c47(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e7(data, pointer)?;
        if let VsfType::c47(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E7"));
        }
    }
    Ok(VsfType::t_c47(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c53(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e3(data, pointer)?;
        if let VsfType::c53(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E3"));
        }
    }
    Ok(VsfType::t_c53(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c54(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e4(data, pointer)?;
        if let VsfType::c54(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E4"));
        }
    }
    Ok(VsfType::t_c54(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c55(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e5(data, pointer)?;
        if let VsfType::c55(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E5"));
        }
    }
    Ok(VsfType::t_c55(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c56(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e6(data, pointer)?;
        if let VsfType::c56(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E6"));
        }
    }
    Ok(VsfType::t_c56(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c57(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF5E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e7(data, pointer)?;
        if let VsfType::c57(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E7"));
        }
    }
    Ok(VsfType::t_c57(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c63(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e3(data, pointer)?;
        if let VsfType::c63(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E3"));
        }
    }
    Ok(VsfType::t_c63(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e4(data, pointer)?;
        if let VsfType::c64(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E4"));
        }
    }
    Ok(VsfType::t_c64(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c65(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e5(data, pointer)?;
        if let VsfType::c65(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E5"));
        }
    }
    Ok(VsfType::t_c65(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c66(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e6(data, pointer)?;
        if let VsfType::c66(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E6"));
        }
    }
    Ok(VsfType::t_c66(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c67(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 32;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF6E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e7(data, pointer)?;
        if let VsfType::c67(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E7"));
        }
    }
    Ok(VsfType::t_c67(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c73(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 33;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e3(data, pointer)?;
        if let VsfType::c73(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E3"));
        }
    }
    Ok(VsfType::t_c73(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c74(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 34;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e4(data, pointer)?;
        if let VsfType::c74(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E4"));
        }
    }
    Ok(VsfType::t_c74(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c75(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 36;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e5(data, pointer)?;
        if let VsfType::c75(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E5"));
        }
    }
    Ok(VsfType::t_c75(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c76(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 40;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e6(data, pointer)?;
        if let VsfType::c76(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E6"));
        }
    }
    Ok(VsfType::t_c76(Tensor::new(shape, values)))
}

pub fn parse_tensor_data_c77(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 48;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF7E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e7(data, pointer)?;
        if let VsfType::c77(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E7"));
        }
    }
    Ok(VsfType::t_c77(Tensor::new(shape, values)))
}

// ==================== STRIDED SPIRIX SCALAR TENSOR DATA PARSERS ====================

pub fn parse_strided_tensor_data_s33(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 2;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF3E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e3(data, pointer)?;
        if let VsfType::s33(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E3"));
        }
    }
    Ok(VsfType::q_s33(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s34(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF3E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e4(data, pointer)?;
        if let VsfType::s34(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E4"));
        }
    }
    Ok(VsfType::q_s34(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s35(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF3E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e5(data, pointer)?;
        if let VsfType::s35(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E5"));
        }
    }
    Ok(VsfType::q_s35(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s36(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF3E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e6(data, pointer)?;
        if let VsfType::s36(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E6"));
        }
    }
    Ok(VsfType::q_s36(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s37(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF3E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f3e7(data, pointer)?;
        if let VsfType::s37(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF3E7"));
        }
    }
    Ok(VsfType::q_s37(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s43(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF4E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e3(data, pointer)?;
        if let VsfType::s43(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E3"));
        }
    }
    Ok(VsfType::q_s43(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s44(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF4E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e4(data, pointer)?;
        if let VsfType::s44(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E4"));
        }
    }
    Ok(VsfType::q_s44(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s45(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF4E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e5(data, pointer)?;
        if let VsfType::s45(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E5"));
        }
    }
    Ok(VsfType::q_s45(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s46(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF4E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e6(data, pointer)?;
        if let VsfType::s46(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E6"));
        }
    }
    Ok(VsfType::q_s46(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s47(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF4E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f4e7(data, pointer)?;
        if let VsfType::s47(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF4E7"));
        }
    }
    Ok(VsfType::q_s47(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s53(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF5E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e3(data, pointer)?;
        if let VsfType::s53(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E3"));
        }
    }
    Ok(VsfType::q_s53(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s54(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF5E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e4(data, pointer)?;
        if let VsfType::s54(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E4"));
        }
    }
    Ok(VsfType::q_s54(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s55(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF5E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e5(data, pointer)?;
        if let VsfType::s55(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E5"));
        }
    }
    Ok(VsfType::q_s55(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s56(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF5E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e6(data, pointer)?;
        if let VsfType::s56(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E6"));
        }
    }
    Ok(VsfType::q_s56(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s57(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF5E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f5e7(data, pointer)?;
        if let VsfType::s57(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF5E7"));
        }
    }
    Ok(VsfType::q_s57(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s63(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF6E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e3(data, pointer)?;
        if let VsfType::s63(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E3"));
        }
    }
    Ok(VsfType::q_s63(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF6E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e4(data, pointer)?;
        if let VsfType::s64(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E4"));
        }
    }
    Ok(VsfType::q_s64(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s65(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF6E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e5(data, pointer)?;
        if let VsfType::s65(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E5"));
        }
    }
    Ok(VsfType::q_s65(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s66(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF6E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e6(data, pointer)?;
        if let VsfType::s66(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E6"));
        }
    }
    Ok(VsfType::q_s66(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s67(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF6E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f6e7(data, pointer)?;
        if let VsfType::s67(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF6E7"));
        }
    }
    Ok(VsfType::q_s67(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s73(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF7E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e3(data, pointer)?;
        if let VsfType::s73(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E3"));
        }
    }
    Ok(VsfType::q_s73(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s74(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF7E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e4(data, pointer)?;
        if let VsfType::s74(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E4"));
        }
    }
    Ok(VsfType::q_s74(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s75(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF7E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e5(data, pointer)?;
        if let VsfType::s75(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E5"));
        }
    }
    Ok(VsfType::q_s75(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s76(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF7E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e6(data, pointer)?;
        if let VsfType::s76(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E6"));
        }
    }
    Ok(VsfType::q_s76(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_s77(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 32;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided ScalarF7E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_scalar_f7e7(data, pointer)?;
        if let VsfType::s77(scalar) = val {
            values.push(scalar);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected ScalarF7E7"));
        }
    }
    Ok(VsfType::q_s77(StridedTensor::new(shape, stride, values)))
}

// ==================== STRIDED SPIRIX CIRCLE TENSOR DATA PARSERS ====================

pub fn parse_strided_tensor_data_c33(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 3;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF3E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e3(data, pointer)?;
        if let VsfType::c33(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E3"));
        }
    }
    Ok(VsfType::q_c33(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c34(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 4;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF3E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e4(data, pointer)?;
        if let VsfType::c34(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E4"));
        }
    }
    Ok(VsfType::q_c34(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c35(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF3E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e5(data, pointer)?;
        if let VsfType::c35(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E5"));
        }
    }
    Ok(VsfType::q_c35(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c36(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF3E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e6(data, pointer)?;
        if let VsfType::c36(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E6"));
        }
    }
    Ok(VsfType::q_c36(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c37(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF3E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f3e7(data, pointer)?;
        if let VsfType::c37(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF3E7"));
        }
    }
    Ok(VsfType::q_c37(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c43(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 5;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF4E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e3(data, pointer)?;
        if let VsfType::c43(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E3"));
        }
    }
    Ok(VsfType::q_c43(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c44(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 6;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF4E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e4(data, pointer)?;
        if let VsfType::c44(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E4"));
        }
    }
    Ok(VsfType::q_c44(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c45(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 8;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF4E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e5(data, pointer)?;
        if let VsfType::c45(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E5"));
        }
    }
    Ok(VsfType::q_c45(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c46(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF4E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e6(data, pointer)?;
        if let VsfType::c46(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E6"));
        }
    }
    Ok(VsfType::q_c46(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c47(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF4E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f4e7(data, pointer)?;
        if let VsfType::c47(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF4E7"));
        }
    }
    Ok(VsfType::q_c47(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c53(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 9;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF5E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e3(data, pointer)?;
        if let VsfType::c53(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E3"));
        }
    }
    Ok(VsfType::q_c53(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c54(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 10;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF5E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e4(data, pointer)?;
        if let VsfType::c54(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E4"));
        }
    }
    Ok(VsfType::q_c54(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c55(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 12;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF5E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e5(data, pointer)?;
        if let VsfType::c55(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E5"));
        }
    }
    Ok(VsfType::q_c55(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c56(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 16;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF5E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e6(data, pointer)?;
        if let VsfType::c56(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E6"));
        }
    }
    Ok(VsfType::q_c56(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c57(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF5E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f5e7(data, pointer)?;
        if let VsfType::c57(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF5E7"));
        }
    }
    Ok(VsfType::q_c57(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c63(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 17;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF6E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e3(data, pointer)?;
        if let VsfType::c63(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E3"));
        }
    }
    Ok(VsfType::q_c63(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c64(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 18;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF6E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e4(data, pointer)?;
        if let VsfType::c64(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E4"));
        }
    }
    Ok(VsfType::q_c64(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c65(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 20;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF6E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e5(data, pointer)?;
        if let VsfType::c65(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E5"));
        }
    }
    Ok(VsfType::q_c65(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c66(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 24;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF6E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e6(data, pointer)?;
        if let VsfType::c66(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E6"));
        }
    }
    Ok(VsfType::q_c66(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c67(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 32;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF6E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f6e7(data, pointer)?;
        if let VsfType::c67(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF6E7"));
        }
    }
    Ok(VsfType::q_c67(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c73(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 33;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF7E3 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e3(data, pointer)?;
        if let VsfType::c73(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E3"));
        }
    }
    Ok(VsfType::q_c73(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c74(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 34;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF7E4 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e4(data, pointer)?;
        if let VsfType::c74(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E4"));
        }
    }
    Ok(VsfType::q_c74(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c75(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 36;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF7E5 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e5(data, pointer)?;
        if let VsfType::c75(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E5"));
        }
    }
    Ok(VsfType::q_c75(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c76(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 40;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF7E6 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e6(data, pointer)?;
        if let VsfType::c76(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E6"));
        }
    }
    Ok(VsfType::q_c76(StridedTensor::new(shape, stride, values)))
}

pub fn parse_strided_tensor_data_c77(
    data: &[u8],
    pointer: &mut usize,
    shape: Vec<usize>,
    stride: Vec<usize>,
    total_elements: usize,
) -> Result<VsfType, Error> {
    let byte_count = total_elements * 48;
    if *pointer + byte_count > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for strided CircleF7E7 tensor",
        ));
    }

    let mut values = Vec::with_capacity(total_elements);
    for _ in 0..total_elements {
        let val = parse_circle_f7e7(data, pointer)?;
        if let VsfType::c77(circle) = val {
            values.push(circle);
        } else {
            return Err(Error::new(ErrorKind::InvalidData, "Expected CircleF7E7"));
        }
    }
    Ok(VsfType::q_c77(StridedTensor::new(shape, stride, values)))
}
