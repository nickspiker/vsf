//! Toka Tree (Loom) type parsers
//!
//! Toka Tree nodes are encoded as `vt` wrapped data: `v(b't', inner_bytes)`
//! The inner bytes contain: `[type_marker][data...]` where type_marker is one of:
//! - `b` = Box, `n` = Node, `c` = Circle, `l` = Line
//! - `x` = Text, `u` = Button, `p` = Path, `i` = Image, `s` = Surface

use crate::types::VsfType;
#[cfg(feature = "spirix")]
use crate::types::toka_tree::{
    ButtonVariant, PathCommand, TokaBox, TokaButton, TokaCircle, TokaNodeContainer, TokaImage,
    TokaLine, TokaNode, TokaPath, TokaSurface, TokaText,
};
#[cfg(feature = "spirix")]
use spirix::{CircleF4E4, ScalarF4E4};
use std::io::{Error, ErrorKind};

/// Parse a Toka Tree node from vt wrapped VsfType
///
/// Takes a `VsfType::v(b't', inner_data)` and parses the inner data to TokaNode.
#[cfg(feature = "spirix")]
pub fn parse_vt_toka_node(vsf_type: &VsfType) -> Result<TokaNode, Error> {
    match vsf_type {
        VsfType::v(encoding, data) if *encoding == b't' => {
            let mut pointer = 0;
            parse_toka_tree_inner(data, &mut pointer)
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Expected vt wrapped Toka Tree (v with encoding 't')",
        )),
    }
}

/// Parse a Toka Tree node from inner bytes
///
/// Expects inner_data to start with a type marker byte (b/g/c/l/x/u/p/i/s).
/// This function parses the type marker and delegates to specific parsers.
#[cfg(feature = "spirix")]
pub fn parse_toka_tree_inner(data: &[u8], pointer: &mut usize) -> Result<TokaNode, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Missing Toka Tree type marker",
        ));
    }

    let type_byte = data[*pointer];
    *pointer += 1;

    match type_byte {
        b'b' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let colour = parse_rgba(data, pointer)?;
            Ok(TokaNode::Box(TokaBox { pos, size, colour }))
        }
        b'n' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let child_count = data[*pointer] as usize;
            *pointer += 1;
            let mut children = Vec::with_capacity(child_count);
            // Parse each child as a complete vt capsule (nested VSF field)
            for _ in 0..child_count {
                // Parse complete vt capsule: v(b't', inner_bytes)
                let child_vsf = super::parse::parse(data, pointer)?;
                // Unwrap vt capsule and parse inner TokaNode
                let child = parse_vt_toka_node(&child_vsf)?;
                children.push(child);
            }
            Ok(TokaNode::Node(TokaNodeContainer {
                pos,
                size,
                children,
            }))
        }
        b'c' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let span = parse_scalar_f4e4(data, pointer)?;
            let colour = parse_rgba(data, pointer)?;
            Ok(TokaNode::Circle(TokaCircle { pos, span, colour }))
        }
        b'l' => {
            let start = parse_circle_f4e4(data, pointer)?;
            let end = parse_circle_f4e4(data, pointer)?;
            let width = parse_scalar_f4e4(data, pointer)?;
            let colour = parse_rgba(data, pointer)?;
            Ok(TokaNode::Line(TokaLine {
                start,
                end,
                width,
                colour,
            }))
        }
        b'x' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let content_vsf = super::parse::parse(data, pointer)?;
            let content = match content_vsf {
                VsfType::x(s) => s,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Expected text content in TokaText",
                    ))
                }
            };
            let colour = parse_rgba(data, pointer)?;
            Ok(TokaNode::Text(TokaText {
                pos,
                size,
                content,
                colour,
            }))
        }
        b'u' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let label_vsf = super::parse::parse(data, pointer)?;
            let label = match label_vsf {
                VsfType::x(s) => s,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Expected text label in TokaButton",
                    ))
                }
            };
            let variant_byte = data[*pointer];
            *pointer += 1;
            let variant = match variant_byte {
                0 => ButtonVariant::Filled,
                1 => ButtonVariant::Outlined,
                2 => ButtonVariant::Text,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Invalid button variant: {}", variant_byte),
                    ))
                }
            };
            let colour = parse_rgba(data, pointer)?;
            Ok(TokaNode::Button(TokaButton {
                pos,
                size,
                label,
                variant,
                colour,
            }))
        }
        b'p' => {
            let colour = parse_rgba(data, pointer)?;
            let width = parse_scalar_f4e4(data, pointer)?;
            let command_count = data[*pointer] as usize;
            *pointer += 1;
            let mut commands = Vec::with_capacity(command_count);
            for _ in 0..command_count {
                let cmd = parse_path_command(data, pointer)?;
                commands.push(cmd);
            }
            Ok(TokaNode::Path(TokaPath {
                colour,
                width,
                commands,
            }))
        }
        b'i' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let handle = u64::from_be_bytes([
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
            let tint = parse_rgba(data, pointer)?;
            Ok(TokaNode::Image(TokaImage {
                pos,
                size,
                handle,
                tint,
            }))
        }
        b's' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            let size = parse_circle_f4e4(data, pointer)?;
            let handle = u64::from_be_bytes([
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
            Ok(TokaNode::Surface(TokaSurface { pos, size, handle }))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid Toka Tree type marker: {}", type_byte as char),
        )),
    }
}

/// Parse a path command from bytes
#[cfg(feature = "spirix")]
fn parse_path_command(data: &[u8], pointer: &mut usize) -> Result<PathCommand, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Missing path command type",
        ));
    }

    let cmd_byte = data[*pointer];
    *pointer += 1;

    match cmd_byte {
        b'M' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            Ok(PathCommand::MoveTo(pos))
        }
        b'L' => {
            let pos = parse_circle_f4e4(data, pointer)?;
            Ok(PathCommand::LineTo(pos))
        }
        b'Q' => {
            let ctrl = parse_circle_f4e4(data, pointer)?;
            let end = parse_circle_f4e4(data, pointer)?;
            Ok(PathCommand::QuadraticTo { ctrl, end })
        }
        b'C' => {
            let ctrl1 = parse_circle_f4e4(data, pointer)?;
            let ctrl2 = parse_circle_f4e4(data, pointer)?;
            let end = parse_circle_f4e4(data, pointer)?;
            Ok(PathCommand::CubicTo { ctrl1, ctrl2, end })
        }
        b'Z' => Ok(PathCommand::Close),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid path command: {}", cmd_byte as char),
        )),
    }
}

/// Parse a CircleF4E4 from bytes
#[cfg(feature = "spirix")]
fn parse_circle_f4e4(data: &[u8], pointer: &mut usize) -> Result<CircleF4E4, Error> {
    if *pointer + 6 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for CircleF4E4",
        ));
    }

    let real = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let imaginary = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    let exponent = i16::from_be_bytes([data[*pointer + 4], data[*pointer + 5]]);
    *pointer += 6;

    Ok(CircleF4E4 {
        real,
        imaginary,
        exponent,
    })
}

/// Parse a ScalarF4E4 from bytes
#[cfg(feature = "spirix")]
fn parse_scalar_f4e4(data: &[u8], pointer: &mut usize) -> Result<ScalarF4E4, Error> {
    if *pointer + 4 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for ScalarF4E4",
        ));
    }

    let fraction = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
    let exponent = i16::from_be_bytes([data[*pointer + 2], data[*pointer + 3]]);
    *pointer += 4;

    Ok(ScalarF4E4 { fraction, exponent })
}

#[cfg(feature = "spirix")]
fn parse_rgba(data: &[u8], pointer: &mut usize) -> Result<[ScalarF4E4; 4], Error> {
    Ok([
        parse_scalar_f4e4(data, pointer)?,
        parse_scalar_f4e4(data, pointer)?,
        parse_scalar_f4e4(data, pointer)?,
        parse_scalar_f4e4(data, pointer)?,
    ])
}
