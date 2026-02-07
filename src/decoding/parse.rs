//! Main VSF parse dispatcher

use crate::types::VsfType;
use std::io::{Error, ErrorKind};

// Import sub-parsers from sibling modules
use super::metadata::{
    parse_backward_version, parse_colour_constant, parse_count, parse_dtype, parse_eagle_time,
    parse_file_length, parse_hash, parse_key, parse_label, parse_length, parse_mac,
    parse_marker_def, parse_marker_ref, parse_offset, parse_signature, parse_string,
    parse_version, parse_world_coord, parse_wrapped,
};
use super::primitives::{parse_complex, parse_float, parse_signed, parse_unsigned};
#[cfg(feature = "spirix")]
use super::spirix::{parse_spirix_circle, parse_spirix_scalar};
use super::tensors::{parse_bitpacked_tensor, parse_strided_tensor, parse_tensor};
#[cfg(feature = "spirix")]
use crate::types::{Fill, GradientStop, GradientVariant, Stroke, StrokeCap, StrokeJoin};

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
        b'r' => {
            // Look ahead to distinguish:
            // - rc[a-z]: colour constants
            // - ro[a-z]: renderable objects
            // - r + usize: marker refs
            if *pointer < data.len() {
                match data[*pointer] {
                    b'c' => parse_colour_constant(data, pointer),
                    #[cfg(feature = "spirix")]
                    b'o' => parse_renderable_object(data, pointer),
                    _ => parse_marker_ref(data, pointer),
                }
            } else {
                parse_marker_ref(data, pointer)
            }
        }
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

/// Parse renderable object types: `ro[a-z]`
///
/// Format: 3+ bytes - 'r', 'o', type letter, then type-specific data
#[cfg(feature = "spirix")]
fn parse_renderable_object(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    use spirix::{CircleF4E4, ScalarF4E4};

    // Already consumed 'r', now expect 'o' and type letter
    if *pointer + 2 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Incomplete renderable object (need 2 more bytes after 'r')",
        ));
    }

    let second = data[*pointer];
    let third = data[*pointer + 1];
    *pointer += 2;

    // Validate 'o' marker
    if second != b'o' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid renderable object: expected 'o', got '{}'",
                second as char
            ),
        ));
    }

    // Parse based on type letter
    match third {
        b'b' => {
            // rob: Box (pos, size, fill, stroke, children)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            let children = parse_children(data, pointer)?;
            Ok(VsfType::rob(pos, size, fill, stroke, children))
        }
        b'c' => {
            // roc: Circle (center, radius, fill, stroke)
            let center = parse_c44(data, pointer)?;
            let radius = parse_s44(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            Ok(VsfType::roc(center, radius, fill, stroke))
        }
        b'g' => {
            // rog: Gradient (variant, stops)
            let variant = parse_gradient_variant(data, pointer)?;
            let stops = parse_gradient_stops(data, pointer)?;
            Ok(VsfType::rog(variant, stops))
        }
        b'n' => {
            // ron: Node (pos, size, children)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let children = parse_children(data, pointer)?;
            Ok(VsfType::ron(pos, size, children))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown renderable object type: ro{}", third as char),
        )),
    }
}

// Helper functions for parsing renderable object components

#[cfg(feature = "spirix")]
fn parse_c44(data: &[u8], pointer: &mut usize) -> Result<spirix::CircleF4E4, Error> {
    if *pointer + 6 > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for c44"));
    }
    let real = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let imaginary = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    let exponent = i16::from_be_bytes([data[*pointer + 4], data[*pointer + 5]]);
    *pointer += 6;
    Ok(spirix::CircleF4E4 { real, imaginary, exponent })
}

#[cfg(feature = "spirix")]
fn parse_s44(data: &[u8], pointer: &mut usize) -> Result<spirix::ScalarF4E4, Error> {
    if *pointer + 4 > data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for s44"));
    }
    let fraction = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let exponent = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    *pointer += 4;
    Ok(spirix::ScalarF4E4 { fraction, exponent })
}

#[cfg(feature = "spirix")]
fn parse_fill(data: &[u8], pointer: &mut usize) -> Result<Fill, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for fill type"));
    }
    let fill_type = data[*pointer];
    *pointer += 1;

    match fill_type {
        0x00 => {
            // Solid colour
            let colour = parse(data, pointer)?;
            Ok(Fill::Solid(Box::new(colour)))
        }
        0x01 => {
            // Gradient
            let gradient = parse(data, pointer)?;
            Ok(Fill::Gradient(Box::new(gradient)))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid fill type: {}", fill_type),
        )),
    }
}

#[cfg(feature = "spirix")]
fn parse_option_stroke(data: &[u8], pointer: &mut usize) -> Result<Option<Stroke>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for stroke option"));
    }
    let has_stroke = data[*pointer];
    *pointer += 1;

    match has_stroke {
        0x00 => Ok(None),
        0x01 => {
            // Parse stroke: width, colour, join, cap
            let width = parse_s44(data, pointer)?;
            let colour = parse(data, pointer)?;

            if *pointer + 2 > data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for stroke properties"));
            }
            let join = match data[*pointer] {
                0 => StrokeJoin::Miter,
                1 => StrokeJoin::Round,
                2 => StrokeJoin::Bevel,
                _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid stroke join")),
            };
            let cap = match data[*pointer + 1] {
                0 => StrokeCap::Butt,
                1 => StrokeCap::Round,
                2 => StrokeCap::Square,
                _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid stroke cap")),
            };
            *pointer += 2;

            Ok(Some(Stroke {
                width,
                colour: Box::new(colour),
                join,
                cap,
            }))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid stroke option: {}", has_stroke),
        )),
    }
}

#[cfg(feature = "spirix")]
fn parse_gradient_variant(data: &[u8], pointer: &mut usize) -> Result<GradientVariant, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for gradient variant"));
    }
    let variant_type = data[*pointer];
    *pointer += 1;

    match variant_type {
        0 => {
            // Linear
            let start = parse_c44(data, pointer)?;
            let end = parse_c44(data, pointer)?;
            Ok(GradientVariant::Linear { start, end })
        }
        1 => {
            // Radial
            let center = parse_c44(data, pointer)?;
            let radius = parse_s44(data, pointer)?;
            Ok(GradientVariant::Radial { center, radius })
        }
        2 => {
            // Conic
            let center = parse_c44(data, pointer)?;
            let angle = parse_s44(data, pointer)?;
            Ok(GradientVariant::Conic { center, angle })
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid gradient variant: {}", variant_type),
        )),
    }
}

#[cfg(feature = "spirix")]
fn parse_gradient_stops(data: &[u8], pointer: &mut usize) -> Result<Vec<GradientStop>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for gradient stop count"));
    }
    let count = data[*pointer] as usize;
    *pointer += 1;

    let mut stops = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = parse_s44(data, pointer)?;
        let mut colour = [spirix::ScalarF4E4::ZERO; 4];
        for channel in &mut colour {
            *channel = parse_s44(data, pointer)?;
        }
        stops.push(GradientStop { offset, colour });
    }
    Ok(stops)
}

#[cfg(feature = "spirix")]
fn parse_children(data: &[u8], pointer: &mut usize) -> Result<Vec<VsfType>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Not enough data for children count"));
    }
    let count = data[*pointer] as usize;
    *pointer += 1;

    let mut children = Vec::with_capacity(count);
    for _ in 0..count {
        children.push(parse(data, pointer)?);
    }
    Ok(children)
}
