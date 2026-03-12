//! Main VSF parse dispatcher

use crate::types::VsfType;
use std::io::{Error, ErrorKind};

// Import sub-parsers from sibling modules
use super::metadata::{
    parse_backward_version, parse_colour_array, parse_colour_constant, parse_count, parse_dtype,
    parse_eagle_time, parse_file_length, parse_hash, parse_key, parse_label, parse_length,
    parse_mac, parse_marker_def, parse_marker_ref, parse_offset, parse_signature, parse_string,
    parse_version, parse_world_coord, parse_wrapped,
};
use super::helpers::decode_usize;
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
            // - ra/rt/rp: colour arrays
            // - ro[a-z]: renderable objects
            // - r + usize: marker refs
            if *pointer < data.len() {
                match data[*pointer] {
                    b'c' => parse_colour_constant(data, pointer),
                    b'a' | b't' | b'p' => {
                        let colour_type = data[*pointer];
                        *pointer += 1;
                        parse_colour_array(data, pointer, colour_type)
                    }
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
        b'e' => {
            // roe: Ellipse (center, size, fill, stroke)
            let center = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            Ok(VsfType::roe(center, size, fill, stroke))
        }
        b'l' => {
            // rol: Line (start, end, width, colour)
            let start = parse_c44(data, pointer)?;
            let end = parse_c44(data, pointer)?;
            let width = parse_s44(data, pointer)?;
            let colour = Box::new(parse(data, pointer)?);
            Ok(VsfType::rol(start, end, width, colour))
        }
        b'p' => {
            // rop: Path (commands, fill, stroke)
            let commands = parse_path_commands(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            Ok(VsfType::rop(commands, fill, stroke))
        }
        b'o' => {
            // roo: Polyline (points, width, colour, closed)
            let points = parse_points(data, pointer)?;
            let width = parse_s44(data, pointer)?;
            let colour = Box::new(parse(data, pointer)?);
            let closed = parse_bool(data, pointer)?;
            Ok(VsfType::roo(points, width, colour, closed))
        }
        b'r' => {
            // ror: NURBS (control_points, knots, degree, fill, stroke)
            let control_points = parse_points(data, pointer)?;
            let knots = parse_scalars(data, pointer)?;
            let degree = parse_u8(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            Ok(VsfType::ror(control_points, knots, degree, fill, stroke))
        }
        b'x' => {
            // rox: Spline (control_points, type, fill, stroke)
            let control_points = parse_points(data, pointer)?;
            let spline_type = parse_spline_type(data, pointer)?;
            let fill = parse_fill(data, pointer)?;
            let stroke = parse_option_stroke(data, pointer)?;
            Ok(VsfType::rox(control_points, spline_type, fill, stroke))
        }
        b't' => {
            // rot: Text (pos, text (l or x), size, colour, style)
            let pos = parse_c44(data, pointer)?;
            let text = Box::new(parse(data, pointer)?); // Can be l or x type
            let size = parse_s44(data, pointer)?;
            let colour = Box::new(parse(data, pointer)?);
            // Parse TextStyle: tagged fields terminated by 0x00.
            // Tags: b'l'=left, b'r'=right (no value), b'f'+32=font hash,
            //       b'e'/b'k'/b'w'/b'i'/b'x' + s44 (4 bytes each).
            let style = parse_text_style(data, pointer)?;
            Ok(VsfType::rot(pos, text, size, colour, style))
        }
        b'u' => {
            // rou: Button (pos, size, label, variant, colour)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let label_vsf = parse(data, pointer)?;
            let label = match label_vsf {
                VsfType::x(s) | VsfType::l(s) => s,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Expected string for label",
                    ))
                }
            };
            let variant = parse_button_variant(data, pointer)?;
            let colour = Box::new(parse(data, pointer)?);
            Ok(VsfType::rou(pos, size, label, variant, colour))
        }
        b'q' => {
            // roq: TextInput (pos, size, placeholder, colour)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let placeholder_vsf = parse(data, pointer)?;
            let placeholder = match placeholder_vsf {
                VsfType::x(s) | VsfType::l(s) => s,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Expected string for placeholder",
                    ))
                }
            };
            let colour = Box::new(parse(data, pointer)?);
            Ok(VsfType::roq(pos, size, placeholder, colour))
        }
        b'a' => {
            // roa: Array/SubTable (cols, rows, children)
            let cols = decode_usize(data, pointer)?;
            let rows = decode_usize(data, pointer)?;
            let count = decode_usize(data, pointer)?;
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(parse(data, pointer)?);
            }
            Ok(VsfType::roa(cols, rows, children))
        }
        b'i' => {
            // roi: Image (pos, size, handle, tint)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let handle_vsf = parse(data, pointer)?;
            let handle = match handle_vsf {
                VsfType::u6(h) => h,
                _ => return Err(Error::new(ErrorKind::InvalidData, "Expected u6 for handle")),
            };
            let tint = Box::new(parse(data, pointer)?);
            Ok(VsfType::roi(pos, size, handle, tint))
        }
        b'f' => {
            // rof: Surface (pos, size, handle)
            let pos = parse_c44(data, pointer)?;
            let size = parse_c44(data, pointer)?;
            let handle_vsf = parse(data, pointer)?;
            let handle = match handle_vsf {
                VsfType::u6(h) => h,
                _ => return Err(Error::new(ErrorKind::InvalidData, "Expected u6 for handle")),
            };
            Ok(VsfType::rof(pos, size, handle))
        }
        b'm' => {
            // rom: Mask (shape, children)
            let shape = Box::new(parse(data, pointer)?);
            let children = parse_children(data, pointer)?;
            Ok(VsfType::rom(shape, children))
        }
        b'w' => {
            // row: Group (transform, children)
            let transform = parse_transform(data, pointer)?;
            let children = parse_children(data, pointer)?;
            Ok(VsfType::row(transform, children))
        }
        b'k' => {
            // rok: Stroke (width, colour, join, cap)
            let width = parse_s44(data, pointer)?;
            let colour = Box::new(parse(data, pointer)?);
            let join = parse_stroke_join(data, pointer)?;
            let cap = parse_stroke_cap(data, pointer)?;
            Ok(VsfType::rok(width, colour, join, cap))
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
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for c44",
        ));
    }
    let real = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let imaginary = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    let exponent = i16::from_be_bytes([data[*pointer + 4], data[*pointer + 5]]);
    *pointer += 6;
    Ok(spirix::CircleF4E4 {
        real,
        imaginary,
        exponent,
    })
}

#[cfg(feature = "spirix")]
fn parse_s44(data: &[u8], pointer: &mut usize) -> Result<spirix::ScalarF4E4, Error> {
    if *pointer + 4 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for s44",
        ));
    }
    let fraction = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let exponent = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    *pointer += 4;
    Ok(spirix::ScalarF4E4 { fraction, exponent })
}

#[cfg(feature = "spirix")]
fn parse_text_style(
    data: &[u8],
    pointer: &mut usize,
) -> Result<Option<crate::types::TextStyle>, Error> {
    use crate::types::TextStyle;

    if *pointer >= data.len() || data[*pointer] == 0x00 {
        // No style or empty style — consume the terminator if present
        if *pointer < data.len() && data[*pointer] == 0x00 {
            *pointer += 1;
        }
        return Ok(None);
    }

    let mut style = TextStyle::default();

    loop {
        if *pointer >= data.len() {
            return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected end in TextStyle"));
        }
        let tag = data[*pointer];
        *pointer += 1;

        match tag {
            0x00 => break, // terminator
            b'l' => style.align = Some(1), // left (no value bytes)
            b'r' => style.align = Some(2), // right (no value bytes)
            b'f' => {
                // font hash: 32 bytes
                if *pointer + 32 > data.len() {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "TextStyle: truncated font hash"));
                }
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&data[*pointer..*pointer + 32]);
                *pointer += 32;
                style.font = Some(hash);
            }
            b'e' => { style.leading = Some(parse_s44(data, pointer)?); }
            b'k' => { style.kerning = Some(parse_s44(data, pointer)?); }
            b'w' => { style.weight  = Some(parse_s44(data, pointer)?); }
            b'i' => { style.tilt    = Some(parse_s44(data, pointer)?); }
            b'x' => { style.wrap    = Some(parse_s44(data, pointer)?); }
            other => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("TextStyle: unknown tag {other:#04x}"),
                ));
            }
        }
    }

    if style.is_default() { Ok(None) } else { Ok(Some(style)) }
}

#[cfg(feature = "spirix")]
fn parse_fill(data: &[u8], pointer: &mut usize) -> Result<Fill, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for fill type",
        ));
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
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for stroke option",
        ));
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for stroke properties",
                ));
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
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for gradient variant",
        ));
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
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for gradient stop count",
        ));
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
    // Expect '(' to start children
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Expected '(' for children",
        ));
    }
    if data[*pointer] != b'(' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Expected '(' for children, got {:02x}", data[*pointer]),
        ));
    }
    *pointer += 1;

    // Parse children until ')'
    let mut children = Vec::new();
    while *pointer < data.len() && data[*pointer] != b')' {
        children.push(parse(data, pointer)?);
    }

    // Expect ')' to end children
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Expected ')' to close children",
        ));
    }
    if data[*pointer] != b')' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Expected ')' to close children",
        ));
    }
    *pointer += 1;

    Ok(children)
}

#[cfg(feature = "spirix")]
fn parse_path_commands(
    data: &[u8],
    pointer: &mut usize,
) -> Result<Vec<crate::types::PathCommand>, Error> {
    use crate::types::PathCommand;

    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for command count",
        ));
    }
    let count = data[*pointer] as usize;
    *pointer += 1;

    let mut commands = Vec::with_capacity(count);
    for _ in 0..count {
        if *pointer >= data.len() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Not enough data for command type",
            ));
        }
        let cmd_type = data[*pointer];
        *pointer += 1;

        let command = match cmd_type {
            0 => PathCommand::MoveTo(parse_c44(data, pointer)?),
            1 => PathCommand::LineTo(parse_c44(data, pointer)?),
            2 => {
                let ctrl = parse_c44(data, pointer)?;
                let end = parse_c44(data, pointer)?;
                PathCommand::QuadraticTo(ctrl, end)
            }
            3 => {
                let ctrl1 = parse_c44(data, pointer)?;
                let ctrl2 = parse_c44(data, pointer)?;
                let end = parse_c44(data, pointer)?;
                PathCommand::CubicTo(ctrl1, ctrl2, end)
            }
            4 => PathCommand::Close,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid path command type: {}", cmd_type),
                ))
            }
        };
        commands.push(command);
    }
    Ok(commands)
}

#[cfg(feature = "spirix")]
fn parse_points(data: &[u8], pointer: &mut usize) -> Result<Vec<spirix::CircleF4E4>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for point count",
        ));
    }
    let count = data[*pointer] as usize;
    *pointer += 1;

    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        points.push(parse_c44(data, pointer)?);
    }
    Ok(points)
}

#[cfg(feature = "spirix")]
fn parse_scalars(data: &[u8], pointer: &mut usize) -> Result<Vec<spirix::ScalarF4E4>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for scalar count",
        ));
    }
    let count = data[*pointer] as usize;
    *pointer += 1;

    let mut scalars = Vec::with_capacity(count);
    for _ in 0..count {
        scalars.push(parse_s44(data, pointer)?);
    }
    Ok(scalars)
}

#[cfg(feature = "spirix")]
fn parse_bool(data: &[u8], pointer: &mut usize) -> Result<bool, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for bool",
        ));
    }
    let value = data[*pointer];
    *pointer += 1;
    Ok(value != 0)
}

#[cfg(feature = "spirix")]
fn parse_u8(data: &[u8], pointer: &mut usize) -> Result<u8, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for u8",
        ));
    }
    let value = data[*pointer];
    *pointer += 1;
    Ok(value)
}

#[cfg(feature = "spirix")]
fn parse_spline_type(data: &[u8], pointer: &mut usize) -> Result<crate::types::SplineType, Error> {
    use crate::types::SplineType;

    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for spline type",
        ));
    }
    let type_byte = data[*pointer];
    *pointer += 1;

    match type_byte {
        0 => Ok(SplineType::Bezier),
        1 => Ok(SplineType::Cubic),
        2 => Ok(SplineType::CatmullRom),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid spline type: {}", type_byte),
        )),
    }
}

#[cfg(feature = "spirix")]
#[allow(dead_code)]
fn parse_option_string(data: &[u8], pointer: &mut usize) -> Result<Option<String>, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for option flag",
        ));
    }
    let has_value = data[*pointer];
    *pointer += 1;

    if has_value == 0 {
        Ok(None)
    } else {
        let string_vsf = parse(data, pointer)?;
        match string_vsf {
            VsfType::x(s) => Ok(Some(s)),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "Expected string in Option<String>",
            )),
        }
    }
}

#[cfg(feature = "spirix")]
fn parse_button_variant(
    data: &[u8],
    pointer: &mut usize,
) -> Result<crate::types::ButtonVariant, Error> {
    use crate::types::ButtonVariant;

    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for button variant",
        ));
    }
    let variant_byte = data[*pointer];
    *pointer += 1;

    match variant_byte {
        0 => Ok(ButtonVariant::Filled),
        1 => Ok(ButtonVariant::Outlined),
        2 => Ok(ButtonVariant::Text),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid button variant: {}", variant_byte),
        )),
    }
}

#[cfg(feature = "spirix")]
fn parse_transform(data: &[u8], pointer: &mut usize) -> Result<crate::types::Transform, Error> {
    use crate::types::Transform;

    // Parse translate (Option<c44>)
    let translate = if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for translate option",
        ));
    } else if data[*pointer] == 0 {
        *pointer += 1;
        None
    } else {
        *pointer += 1;
        Some(parse_c44(data, pointer)?)
    };

    // Parse rotate (Option<s44>)
    let rotate = if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for rotate option",
        ));
    } else if data[*pointer] == 0 {
        *pointer += 1;
        None
    } else {
        *pointer += 1;
        Some(parse_s44(data, pointer)?)
    };

    // Parse scale (Option<c44>)
    let scale = if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for scale option",
        ));
    } else if data[*pointer] == 0 {
        *pointer += 1;
        None
    } else {
        *pointer += 1;
        Some(parse_c44(data, pointer)?)
    };

    // Parse origin (Option<c44>)
    let origin = if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for origin option",
        ));
    } else if data[*pointer] == 0 {
        *pointer += 1;
        None
    } else {
        *pointer += 1;
        Some(parse_c44(data, pointer)?)
    };

    Ok(Transform {
        translate,
        rotate,
        scale,
        origin,
    })
}

#[cfg(feature = "spirix")]
fn parse_stroke_join(data: &[u8], pointer: &mut usize) -> Result<StrokeJoin, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for stroke join",
        ));
    }
    let join_byte = data[*pointer];
    *pointer += 1;

    match join_byte {
        0 => Ok(StrokeJoin::Miter),
        1 => Ok(StrokeJoin::Round),
        2 => Ok(StrokeJoin::Bevel),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid stroke join: {}", join_byte),
        )),
    }
}

#[cfg(feature = "spirix")]
fn parse_stroke_cap(data: &[u8], pointer: &mut usize) -> Result<StrokeCap, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for stroke cap",
        ));
    }
    let cap_byte = data[*pointer];
    *pointer += 1;

    match cap_byte {
        0 => Ok(StrokeCap::Butt),
        1 => Ok(StrokeCap::Round),
        2 => Ok(StrokeCap::Square),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid stroke cap: {}", cap_byte),
        )),
    }
}
