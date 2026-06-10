use crate::prelude::*;
use super::traits::{EncodeNumber, EncodeNumberInclusive};
#[cfg(any(feature = "text", feature = "text-encode"))]
use crate::text_encoding::encode_text;
use crate::types::{EtType, VsfType};

/// Create a hash placeholder with zeros (for provenance hash computation, etc.)
///
/// This function creates a properly formatted hash field filled with zeros, used when the actual hash will be computed and written later.
///
/// # Arguments
/// * `hash_type` - The hash subtype: `b'b'` (BLAKE3), `b's'` (SHA2), or `b'p'` (provenance)
/// * `len` - The length of the hash in bytes (e.g., 32 for BLAKE3)
///
/// # Returns
/// A Vec<u8> containing: [b'h', hash_type, encoded_length, zeros...]
pub fn hash_placeholder(hash_type: u8, len: usize) -> Vec<u8> {
    let mut bytes = vec![b'h', hash_type];
    bytes.extend_from_slice(&(len - 1).encode_number());
    bytes.resize(bytes.len() + len, 0);
    bytes
}

impl VsfType {
    /// Flatten this VsfType into its binary representation
    ///
    /// Returns a Vec<u8> containing the encoded bytes ready to write to a file.
    pub fn flatten(&self) -> Vec<u8> {
        match self {
            // ==================== UNSIGNED INTEGERS ====================
            VsfType::u0(value) => {
                vec![b'u', if *value { 255 } else { 0 }]
            }

            VsfType::u(value, inclusive) => {
                if *inclusive {
                    value.encode_usize_inclusive()
                } else {
                    let mut flat = vec![b'u'];
                    flat.extend_from_slice(&value.encode_number());
                    flat
                }
            }

            VsfType::u3(value) => vec![b'u', b'3', *value],

            VsfType::u4(value) => {
                let mut flat = vec![b'u'];
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::u5(value) => {
                let mut flat = vec![b'u'];
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::u6(value) => {
                let mut flat = vec![b'u'];
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::u7(value) => {
                let mut flat = vec![b'u'];
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            // ==================== SIGNED INTEGERS ====================
            VsfType::i(value) => {
                let mut flat = vec![b'i'];
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::i3(value) => {
                let bytes = value.to_be_bytes();
                vec![b'i', b'3', bytes[0]]
            }

            VsfType::i4(value) => {
                let bytes = value.to_be_bytes();
                vec![b'i', b'4', bytes[0], bytes[1]]
            }

            VsfType::i5(value) => {
                let bytes = value.to_be_bytes();
                vec![b'i', b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
            }

            VsfType::i6(value) => {
                let bytes = value.to_be_bytes();
                vec![
                    b'i', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ]
            }

            VsfType::i7(value) => {
                let bytes = value.to_be_bytes();
                vec![
                    b'i', b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
                    bytes[13], bytes[14], bytes[15],
                ]
            }

            // ==================== IEEE FLOATS ====================
            VsfType::f5(value) => {
                let bytes = value.to_be_bytes();
                vec![b'f', b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
            }

            VsfType::f6(value) => {
                let bytes = value.to_be_bytes();
                vec![
                    b'f', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ]
            }

            // ==================== IEEE COMPLEX ====================
            VsfType::j5(value) => {
                let mut flat = Vec::new();
                flat.push(b'j');
                flat.push(b'5');
                flat.extend_from_slice(&value.re.to_be_bytes());
                flat.extend_from_slice(&value.im.to_be_bytes());
                flat
            }

            VsfType::j6(value) => {
                let mut flat = Vec::new();
                flat.push(b'j');
                flat.push(b'6');
                flat.extend_from_slice(&value.re.to_be_bytes());
                flat.extend_from_slice(&value.im.to_be_bytes());
                flat
            }

            // ==================== METADATA & SPECIAL ====================
            VsfType::x(value) => {
                #[cfg(any(feature = "text", feature = "text-encode"))]
                {
                    let mut flat = Vec::new();
                    flat.push(b'x');

                    // Huffman-encode the text (no internal header). encode_text NFC-normalizes
                    // the input first and returns the post-NFC char count alongside the bytes —
                    // we MUST use that count, not value.chars().count(), because NFC can change
                    // the codepoint sequence length (e.g. precomposed → decomposed under NFD
                    // would have a different count, and vice versa).
                    let (encoded_text, char_count) = encode_text(value);

                    flat.extend_from_slice(&char_count.encode_number());
                    flat.extend_from_slice(&encoded_text);

                    flat
                }
                #[cfg(not(any(feature = "text", feature = "text-encode")))]
                {
                    panic!(
                        "VsfType::x requires 'text' or 'text-encode' feature for Huffman compression. \
                         Use VsfType::a for ASCII text instead. Message: '{}'",
                        value
                    );
                }
            }

            VsfType::e(value) => {
                #[allow(deprecated)]
                let flat = match value {
                    EtType::e5(v) => {
                        // e5 = 32-bit oscillation count: 2 tag bytes + 4 data bytes
                        let mut f = vec![b'e', b'5'];
                        f.extend_from_slice(&v.to_be_bytes());
                        f
                    }
                    EtType::e6(v) => {
                        // e6 = 64-bit oscillation count: 2 tag bytes + 8 data bytes
                        let mut f = vec![b'e', b'6'];
                        f.extend_from_slice(&v.to_be_bytes());
                        f
                    }
                    EtType::e7(v) => {
                        // e7 = 128-bit oscillation count: 2 tag bytes + 16 data bytes
                        let mut f = vec![b'e', b'7'];
                        f.extend_from_slice(&v.to_be_bytes());
                        f
                    }
                    EtType::f5(v) => {
                        // deprecated ef5: still encodeable for round-trip compat
                        let mut f = vec![b'e', b'f', b'5'];
                        f.extend_from_slice(&v.to_be_bytes());
                        f
                    }
                    EtType::f6(v) => {
                        // deprecated ef6: still encodeable for round-trip compat
                        let mut f = vec![b'e', b'f', b'6'];
                        f.extend_from_slice(&v.to_be_bytes());
                        f
                    }
                };
                flat
            }

            VsfType::w(coord) => {
                // Wire: "w6" + 8 bytes big-endian u64. Sized-variant convention: 2^6 = 64 bits. w5 = u32 coarse, w6 = u64 standard, w7 = u128 fine.
                let mut flat = vec![b'w', b'6'];
                flat.extend_from_slice(&coord.raw().to_be_bytes());
                flat
            }

            VsfType::wa(addr) => {
                let mut flat = vec![b'w', b'a'];
                flat.extend_from_slice(&addr.to_wire());
                flat
            }

            // ==================== COLOUR TYPES ====================

            // General format: r[channels_base36][depth_exp][data]
            VsfType::r(channels, depth, data) => {
                let mut flat = vec![b'r'];
                // Channel count as base-36 digit
                let channel_char = if *channels < 10 {
                    b'0' + channels
                } else {
                    b'A' + (channels - 10)
                };
                flat.push(channel_char);
                // Depth exponent as single digit
                flat.push(b'0' + depth);
                // Data bytes
                flat.extend_from_slice(data);
                flat
            }

            // Named shortcuts (zero-data) - 'c' prefix for colour
            VsfType::rcb => vec![b'r', b'c', b'b'], // Blue
            VsfType::rcc => vec![b'r', b'c', b'c'], // Cyan
            VsfType::rcg => vec![b'r', b'c', b'g'], // Middle grey
            VsfType::rcj => vec![b'r', b'c', b'j'], // Magenta
            VsfType::rck => vec![b'r', b'c', b'k'], // Black
            VsfType::rcl => vec![b'r', b'c', b'l'], // Lime
            VsfType::rcn => vec![b'r', b'c', b'n'], // Green
            VsfType::rco => vec![b'r', b'c', b'o'], // Orange
            VsfType::rcq => vec![b'r', b'c', b'q'], // Aqua
            VsfType::rcr => vec![b'r', b'c', b'r'], // Red
            VsfType::rcv => vec![b'r', b'c', b'v'], // Violet
            VsfType::rcw => vec![b'r', b'c', b'w'], // White
            VsfType::rcy => vec![b'r', b'c', b'y'], // Yellow

            // Format shortcuts with data
            VsfType::re(grey) => vec![b'r', b'e', *grey],
            VsfType::rx(grey) => {
                let mut flat = vec![b'r', b'x'];
                flat.extend_from_slice(&grey.to_be_bytes());
                flat
            }
            VsfType::rz(grey) => {
                let mut flat = vec![b'r', b'z'];
                flat.extend_from_slice(&grey.to_be_bytes());
                flat
            }

            // Packed RGB
            VsfType::ri(packed) => vec![b'r', b'i', *packed],
            VsfType::rp(packed) => {
                let mut flat = vec![b'r', b'p'];
                flat.extend_from_slice(&packed.to_be_bytes());
                flat
            }

            // Standard RGB
            VsfType::ru(rgb) => {
                let mut flat = vec![b'r', b'u'];
                flat.extend_from_slice(rgb);
                flat
            }
            VsfType::rs(rgb) => {
                let mut flat = vec![b'r', b's'];
                for channel in rgb {
                    flat.extend_from_slice(&channel.to_be_bytes());
                }
                flat
            }
            VsfType::rf(rgb) => {
                let mut flat = vec![b'r', b'f'];
                for channel in rgb {
                    flat.extend_from_slice(&channel.to_be_bytes());
                }
                flat
            }

            // Standard RGBA
            VsfType::ra(rgba) => {
                let mut flat = vec![b'r', b'a'];
                flat.extend_from_slice(rgba);
                flat
            }
            VsfType::rt(rgba) => {
                let mut flat = vec![b'r', b't'];
                for channel in rgba {
                    flat.extend_from_slice(&channel.to_be_bytes());
                }
                flat
            }
            VsfType::rh(rgba) => {
                let mut flat = vec![b'r', b'h'];
                for channel in rgba {
                    flat.extend_from_slice(&channel.to_be_bytes());
                }
                flat
            }

            // Spirix ScalarF4E4 colour formats
            #[cfg(feature = "spirix")]
            VsfType::rd(grey) => {
                let mut flat = vec![b'r', b'd'];
                flat.extend_from_slice(&grey.fraction.to_be_bytes());
                flat.extend_from_slice(&grey.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::rb(rgb) => {
                let mut flat = vec![b'r', b'b'];
                for channel in rgb {
                    flat.extend_from_slice(&channel.fraction.to_be_bytes());
                    flat.extend_from_slice(&channel.exponent.to_be_bytes());
                }
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::rw(rgba) => {
                let mut flat = vec![b'r', b'w'];
                for channel in rgba {
                    flat.extend_from_slice(&channel.fraction.to_be_bytes());
                    flat.extend_from_slice(&channel.exponent.to_be_bytes());
                }
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::rq(frac_exp, exp_exp, channels, data) => {
                let mut flat = vec![b'r', b'q', *frac_exp, *exp_exp, *channels];
                flat.extend_from_slice(data);
                flat
            }

            // Magic matrix: rm[N][M][matrix_data][gamma]
            VsfType::rm(input_channels, output_channels, matrix, gamma) => {
                let mut flat = vec![b'r', b'm'];
                // Encode input and output channel counts
                flat.extend_from_slice(&input_channels.encode_number());
                flat.extend_from_slice(&output_channels.encode_number());
                // Matrix data (N×M f32 values, raw bytes)
                for value in matrix {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                // Gamma (f32, raw bytes)
                flat.extend_from_slice(&gamma.to_be_bytes());
                flat
            }

            // ==================== RENDERABLE OBJECT TYPES ==================== Scene graph primitives (ro* prefix - 3 letters)
            #[cfg(feature = "spirix")]
            VsfType::rob(pos, size, fill, stroke, children) => {
                let mut flat = vec![b'r', b'o', b'b'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode fill
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                // Encode stroke (Option)
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        // Encode stroke: width, colour, join, cap
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                // Encode children with parentheses (no count needed)
                flat.push(b'(');
                for child in children {
                    flat.extend_from_slice(&child.flatten());
                }
                flat.push(b')');
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roc(center, radius, fill, stroke) => {
                let mut flat = vec![b'r', b'o', b'c'];
                // Encode center (c44)
                flat.extend_from_slice(&center.real.to_be_bytes());
                flat.extend_from_slice(&center.imaginary.to_be_bytes());
                flat.extend_from_slice(&center.exponent.to_be_bytes());
                // Encode radius (s44)
                flat.extend_from_slice(&radius.fraction.to_be_bytes());
                flat.extend_from_slice(&radius.exponent.to_be_bytes());
                // Encode fill and stroke (same as rob)
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rog(variant, stops) => {
                let mut flat = vec![b'r', b'o', b'g'];
                // Encode variant type and parameters
                match variant {
                    crate::types::GradientVariant::Linear { start, end } => {
                        flat.push(0);
                        flat.extend_from_slice(&start.real.to_be_bytes());
                        flat.extend_from_slice(&start.imaginary.to_be_bytes());
                        flat.extend_from_slice(&start.exponent.to_be_bytes());
                        flat.extend_from_slice(&end.real.to_be_bytes());
                        flat.extend_from_slice(&end.imaginary.to_be_bytes());
                        flat.extend_from_slice(&end.exponent.to_be_bytes());
                    }
                    crate::types::GradientVariant::Radial { center, radius } => {
                        flat.push(1);
                        flat.extend_from_slice(&center.real.to_be_bytes());
                        flat.extend_from_slice(&center.imaginary.to_be_bytes());
                        flat.extend_from_slice(&center.exponent.to_be_bytes());
                        flat.extend_from_slice(&radius.fraction.to_be_bytes());
                        flat.extend_from_slice(&radius.exponent.to_be_bytes());
                    }
                    crate::types::GradientVariant::Conic { center, angle } => {
                        flat.push(2);
                        flat.extend_from_slice(&center.real.to_be_bytes());
                        flat.extend_from_slice(&center.imaginary.to_be_bytes());
                        flat.extend_from_slice(&center.exponent.to_be_bytes());
                        flat.extend_from_slice(&angle.fraction.to_be_bytes());
                        flat.extend_from_slice(&angle.exponent.to_be_bytes());
                    }
                }
                // Encode stops
                flat.push(stops.len() as u8);
                for stop in stops {
                    flat.extend_from_slice(&stop.offset.fraction.to_be_bytes());
                    flat.extend_from_slice(&stop.offset.exponent.to_be_bytes());
                    for channel in &stop.colour {
                        flat.extend_from_slice(&channel.fraction.to_be_bytes());
                        flat.extend_from_slice(&channel.exponent.to_be_bytes());
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::ron(pos, size, children) => {
                let mut flat = vec![b'r', b'o', b'n'];
                // Encode pos and size (same as rob)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode children with parentheses (same as rob, row, etc.)
                flat.push(b'(');
                for child in children {
                    flat.extend_from_slice(&child.flatten());
                }
                flat.push(b')');
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roe(center, size, fill, stroke) => {
                let mut flat = vec![b'r', b'o', b'e'];
                // Encode center (c44)
                flat.extend_from_slice(&center.real.to_be_bytes());
                flat.extend_from_slice(&center.imaginary.to_be_bytes());
                flat.extend_from_slice(&center.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode fill
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                // Encode stroke
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rol(start, end, width, colour) => {
                let mut flat = vec![b'r', b'o', b'l'];
                // Encode start (c44)
                flat.extend_from_slice(&start.real.to_be_bytes());
                flat.extend_from_slice(&start.imaginary.to_be_bytes());
                flat.extend_from_slice(&start.exponent.to_be_bytes());
                // Encode end (c44)
                flat.extend_from_slice(&end.real.to_be_bytes());
                flat.extend_from_slice(&end.imaginary.to_be_bytes());
                flat.extend_from_slice(&end.exponent.to_be_bytes());
                // Encode width (s44)
                flat.extend_from_slice(&width.fraction.to_be_bytes());
                flat.extend_from_slice(&width.exponent.to_be_bytes());
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rop(commands, fill, stroke) => {
                let mut flat = vec![b'r', b'o', b'p'];
                // Encode command count
                flat.push(commands.len() as u8);
                // Encode each command
                for cmd in commands {
                    match cmd {
                        crate::types::PathCommand::MoveTo(pos) => {
                            flat.push(0);
                            flat.extend_from_slice(&pos.real.to_be_bytes());
                            flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                            flat.extend_from_slice(&pos.exponent.to_be_bytes());
                        }
                        crate::types::PathCommand::LineTo(pos) => {
                            flat.push(1);
                            flat.extend_from_slice(&pos.real.to_be_bytes());
                            flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                            flat.extend_from_slice(&pos.exponent.to_be_bytes());
                        }
                        crate::types::PathCommand::QuadraticTo(ctrl, end) => {
                            flat.push(2);
                            flat.extend_from_slice(&ctrl.real.to_be_bytes());
                            flat.extend_from_slice(&ctrl.imaginary.to_be_bytes());
                            flat.extend_from_slice(&ctrl.exponent.to_be_bytes());
                            flat.extend_from_slice(&end.real.to_be_bytes());
                            flat.extend_from_slice(&end.imaginary.to_be_bytes());
                            flat.extend_from_slice(&end.exponent.to_be_bytes());
                        }
                        crate::types::PathCommand::CubicTo(ctrl1, ctrl2, end) => {
                            flat.push(3);
                            flat.extend_from_slice(&ctrl1.real.to_be_bytes());
                            flat.extend_from_slice(&ctrl1.imaginary.to_be_bytes());
                            flat.extend_from_slice(&ctrl1.exponent.to_be_bytes());
                            flat.extend_from_slice(&ctrl2.real.to_be_bytes());
                            flat.extend_from_slice(&ctrl2.imaginary.to_be_bytes());
                            flat.extend_from_slice(&ctrl2.exponent.to_be_bytes());
                            flat.extend_from_slice(&end.real.to_be_bytes());
                            flat.extend_from_slice(&end.imaginary.to_be_bytes());
                            flat.extend_from_slice(&end.exponent.to_be_bytes());
                        }
                        crate::types::PathCommand::Close => {
                            flat.push(4);
                        }
                    }
                }
                // Encode fill
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                // Encode stroke
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roo(points, width, colour, closed) => {
                let mut flat = vec![b'r', b'o', b'o'];
                // Encode point count
                flat.push(points.len() as u8);
                // Encode each point (c44)
                for point in points {
                    flat.extend_from_slice(&point.real.to_be_bytes());
                    flat.extend_from_slice(&point.imaginary.to_be_bytes());
                    flat.extend_from_slice(&point.exponent.to_be_bytes());
                }
                // Encode width (s44)
                flat.extend_from_slice(&width.fraction.to_be_bytes());
                flat.extend_from_slice(&width.exponent.to_be_bytes());
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                // Encode closed flag
                flat.push(if *closed { 1 } else { 0 });
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::ror(control_points, knots, degree, fill, stroke) => {
                let mut flat = vec![b'r', b'o', b'r'];
                // Encode control point count
                flat.push(control_points.len() as u8);
                // Encode each control point (c44)
                for point in control_points {
                    flat.extend_from_slice(&point.real.to_be_bytes());
                    flat.extend_from_slice(&point.imaginary.to_be_bytes());
                    flat.extend_from_slice(&point.exponent.to_be_bytes());
                }
                // Encode knot count
                flat.push(knots.len() as u8);
                // Encode each knot (s44)
                for knot in knots {
                    flat.extend_from_slice(&knot.fraction.to_be_bytes());
                    flat.extend_from_slice(&knot.exponent.to_be_bytes());
                }
                // Encode degree
                flat.push(*degree);
                // Encode fill
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                // Encode stroke
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rox(control_points, spline_type, fill, stroke) => {
                let mut flat = vec![b'r', b'o', b'x'];
                // Encode control point count
                flat.push(control_points.len() as u8);
                // Encode each control point (c44)
                for point in control_points {
                    flat.extend_from_slice(&point.real.to_be_bytes());
                    flat.extend_from_slice(&point.imaginary.to_be_bytes());
                    flat.extend_from_slice(&point.exponent.to_be_bytes());
                }
                // Encode spline type
                flat.push(match spline_type {
                    crate::types::SplineType::Bezier => 0,
                    crate::types::SplineType::Cubic => 1,
                    crate::types::SplineType::CatmullRom => 2,
                });
                // Encode fill
                match fill {
                    crate::types::Fill::Solid(colour) => {
                        flat.push(0x00);
                        flat.extend_from_slice(&colour.flatten());
                    }
                    crate::types::Fill::Gradient(grad) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&grad.flatten());
                    }
                }
                // Encode stroke
                match stroke {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.width.fraction.to_be_bytes());
                        flat.extend_from_slice(&s.width.exponent.to_be_bytes());
                        flat.extend_from_slice(&s.colour.flatten());
                        flat.push(match s.join {
                            crate::types::StrokeJoin::Miter => 0,
                            crate::types::StrokeJoin::Round => 1,
                            crate::types::StrokeJoin::Bevel => 2,
                        });
                        flat.push(match s.cap {
                            crate::types::StrokeCap::Butt => 0,
                            crate::types::StrokeCap::Round => 1,
                            crate::types::StrokeCap::Square => 2,
                        });
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rot(pos, text, size, colour, style) => {
                let mut flat = vec![b'r', b'o', b't'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode text (already a VsfType - l or x)
                flat.extend_from_slice(&text.flatten());
                // Encode size (s44)
                flat.extend_from_slice(&size.fraction.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                // Encode optional TextStyle — tagged fields, terminated by 0x00. Tags: b'l'=left, b'r'=right (no value), b'f'+32=font hash, b'e'/b'k'/b'w'/b'i'/b'x' + s44 (4 bytes each). Absence of l/r tag = center (default).
                match style {
                    None => flat.push(0x00),
                    Some(s) => {
                        if let Some(hash) = &s.font {
                            flat.push(b'f');
                            flat.extend_from_slice(hash);
                        }
                        match s.align {
                            Some(1) => flat.push(b'l'),
                            Some(2) => flat.push(b'r'),
                            _ => {} // center = default, no tag needed
                        }
                        if let Some(v) = s.leading {
                            flat.push(b'e');
                            flat.extend_from_slice(&v.fraction.to_be_bytes());
                            flat.extend_from_slice(&v.exponent.to_be_bytes());
                        }
                        if let Some(v) = s.kerning {
                            flat.push(b'k');
                            flat.extend_from_slice(&v.fraction.to_be_bytes());
                            flat.extend_from_slice(&v.exponent.to_be_bytes());
                        }
                        if let Some(v) = s.weight {
                            flat.push(b'w');
                            flat.extend_from_slice(&v.fraction.to_be_bytes());
                            flat.extend_from_slice(&v.exponent.to_be_bytes());
                        }
                        if let Some(v) = s.tilt {
                            flat.push(b'i');
                            flat.extend_from_slice(&v.fraction.to_be_bytes());
                            flat.extend_from_slice(&v.exponent.to_be_bytes());
                        }
                        if let Some(v) = s.wrap {
                            flat.push(b'x');
                            flat.extend_from_slice(&v.fraction.to_be_bytes());
                            flat.extend_from_slice(&v.exponent.to_be_bytes());
                        }
                        flat.push(0x00); // terminator
                    }
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rou(pos, size, label, variant, colour) => {
                let mut flat = vec![b'r', b'o', b'u'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode label (ASCII string)
                let label_vsf = VsfType::a(label.clone());
                flat.extend_from_slice(&label_vsf.flatten());
                // Encode variant
                flat.push(match variant {
                    crate::types::ButtonVariant::Filled => 0,
                    crate::types::ButtonVariant::Outlined => 1,
                    crate::types::ButtonVariant::Text => 2,
                });
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roq(pos, size, placeholder, colour) => {
                let mut flat = vec![b'r', b'o', b'q'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode placeholder (ASCII, no Huffman dependency)
                let placeholder_vsf = VsfType::a(placeholder.clone());
                flat.extend_from_slice(&placeholder_vsf.flatten());
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roa(cols, rows, children) => {
                let mut flat = vec![b'r', b'o', b'a'];
                flat.extend_from_slice(&cols.encode_number());
                flat.extend_from_slice(&rows.encode_number());
                flat.extend_from_slice(&children.len().encode_number());
                for child in children {
                    flat.extend_from_slice(&child.flatten());
                }
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::roi(pos, size, handle, tint) => {
                let mut flat = vec![b'r', b'o', b'i'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode handle (u64 as u6)
                let handle_vsf = VsfType::u6(*handle);
                flat.extend_from_slice(&handle_vsf.flatten());
                // Encode tint colour
                flat.extend_from_slice(&tint.flatten());
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rof(pos, size, handle) => {
                let mut flat = vec![b'r', b'o', b'f'];
                // Encode pos (c44)
                flat.extend_from_slice(&pos.real.to_be_bytes());
                flat.extend_from_slice(&pos.imaginary.to_be_bytes());
                flat.extend_from_slice(&pos.exponent.to_be_bytes());
                // Encode size (c44)
                flat.extend_from_slice(&size.real.to_be_bytes());
                flat.extend_from_slice(&size.imaginary.to_be_bytes());
                flat.extend_from_slice(&size.exponent.to_be_bytes());
                // Encode handle (u64 as u6)
                let handle_vsf = VsfType::u6(*handle);
                flat.extend_from_slice(&handle_vsf.flatten());
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rom(shape, children) => {
                let mut flat = vec![b'r', b'o', b'm'];
                // Encode shape (any VsfType)
                flat.extend_from_slice(&shape.flatten());
                // Encode children with parentheses (no count needed)
                flat.push(b'(');
                for child in children {
                    flat.extend_from_slice(&child.flatten());
                }
                flat.push(b')');
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::row(transform, children) => {
                let mut flat = vec![b'r', b'o', b'w'];
                // Encode transform translate (Option<c44>)
                match &transform.translate {
                    None => flat.push(0x00),
                    Some(t) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&t.real.to_be_bytes());
                        flat.extend_from_slice(&t.imaginary.to_be_bytes());
                        flat.extend_from_slice(&t.exponent.to_be_bytes());
                    }
                }
                // rotate (Option<s44>)
                match &transform.rotate {
                    None => flat.push(0x00),
                    Some(r) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&r.fraction.to_be_bytes());
                        flat.extend_from_slice(&r.exponent.to_be_bytes());
                    }
                }
                // scale (Option<c44>)
                match &transform.scale {
                    None => flat.push(0x00),
                    Some(s) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&s.real.to_be_bytes());
                        flat.extend_from_slice(&s.imaginary.to_be_bytes());
                        flat.extend_from_slice(&s.exponent.to_be_bytes());
                    }
                }
                // origin (Option<c44>)
                match &transform.origin {
                    None => flat.push(0x00),
                    Some(o) => {
                        flat.push(0x01);
                        flat.extend_from_slice(&o.real.to_be_bytes());
                        flat.extend_from_slice(&o.imaginary.to_be_bytes());
                        flat.extend_from_slice(&o.exponent.to_be_bytes());
                    }
                }
                // Encode children with parentheses (no count needed)
                flat.push(b'(');
                for child in children {
                    flat.extend_from_slice(&child.flatten());
                }
                flat.push(b')');
                flat
            }

            #[cfg(feature = "spirix")]
            VsfType::rok(width, colour, join, cap) => {
                let mut flat = vec![b'r', b'o', b'k'];
                // Encode width (s44)
                flat.extend_from_slice(&width.fraction.to_be_bytes());
                flat.extend_from_slice(&width.exponent.to_be_bytes());
                // Encode colour
                flat.extend_from_slice(&colour.flatten());
                // Encode join
                flat.push(match join {
                    crate::types::StrokeJoin::Miter => 0,
                    crate::types::StrokeJoin::Round => 1,
                    crate::types::StrokeJoin::Bevel => 2,
                });
                // Encode cap
                flat.push(match cap {
                    crate::types::StrokeCap::Butt => 0,
                    crate::types::StrokeCap::Round => 1,
                    crate::types::StrokeCap::Square => 2,
                });
                flat
            }

            // VSF Metadata
            VsfType::d(value) => {
                let mut flat = Vec::new();
                flat.push(b'd');
                flat.extend_from_slice(&value.len().encode_number());
                flat.extend_from_slice(value.as_bytes());
                flat
            }

            VsfType::a(value) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&value.len().encode_number());
                flat.extend_from_slice(value.as_bytes());
                flat
            }

            VsfType::o(value) => {
                let mut flat = Vec::new();
                flat.push(b'o');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::b(value, _inclusive) => {
                // Note: inclusive mode is handled by the stabilization loop in vsf_builder The value already includes its own encoding size when needed
                let mut flat = Vec::new();
                flat.push(b'b');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::l(value, _inclusive) => {
                // Length in bytes (wire or file). Inclusive mode handled by stabilization.
                let mut flat = Vec::new();
                flat.push(b'l');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::na(scheme, host, port) => {
                let mut flat = vec![b'n', b'a'];
                flat.push(*scheme);
                flat.extend_from_slice(host.as_bytes());
                flat.push(0); // null-terminate host
                if let Some(p) = port { flat.extend_from_slice(&p.to_be_bytes()); }
                flat
            }
            VsfType::nh(host) => {
                let mut flat = vec![b'n', b'h'];
                flat.extend_from_slice(host.as_bytes());
                flat.push(0);
                flat
            }
            VsfType::ni(addr) => { let mut f = vec![b'n', b'i']; f.extend_from_slice(addr); f }
            VsfType::nj(addr) => { let mut f = vec![b'n', b'j']; f.extend_from_slice(addr); f }
            VsfType::nc(addr, prefix) => {
                let mut flat = vec![b'n', b'c'];
                flat.extend_from_slice(addr.as_bytes());
                flat.push(0);
                flat.push(*prefix);
                flat
            }
            VsfType::nm(mac) => { let mut f = vec![b'n', b'm']; f.extend_from_slice(mac); f }
            VsfType::np(port) => { let mut f = vec![b'n', b'p']; f.extend_from_slice(&port.to_be_bytes()); f }
            VsfType::ns(host, port) => {
                let mut flat = vec![b'n', b's'];
                flat.extend_from_slice(host.as_bytes());
                flat.push(0);
                flat.extend_from_slice(&port.to_be_bytes());
                flat
            }
            VsfType::nu(url) => {
                let mut flat = vec![b'n', b'u'];
                flat.extend_from_slice(url.as_bytes());
                flat.push(0);
                flat
            }
            VsfType::nn(name) => {
                let mut flat = vec![b'n', b'n'];
                flat.extend_from_slice(name.as_bytes());
                flat.push(0);
                flat
            }

            VsfType::n(value) => {
                let mut flat = Vec::new();
                flat.push(b'n');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::z(value) => {
                let mut flat = Vec::new();
                flat.push(b'z');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::y(value) => {
                let mut flat = Vec::new();
                flat.push(b'y');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            // ==================== HASHES ====================
            VsfType::hp(value) => {
                let mut flat = vec![b'h', b'p'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual hash bytes
                flat
            }

            VsfType::hb(value) => {
                let mut flat = vec![b'h', b'b'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual hash bytes
                flat
            }

            VsfType::hs(value) => {
                let mut flat = vec![b'h', b's'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual hash bytes
                flat
            }

            VsfType::hm(value) => {
                let mut flat = vec![b'h', b'm'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual hash bytes
                flat
            }

            VsfType::hg(value) => {
                let mut flat = vec![b'h', b'g'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual hash bytes
                flat
            }

            VsfType::hc(value) => {
                let mut flat = vec![b'h', b'c'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::hk(value) => {
                let mut flat = vec![b'h', b'k'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            // Application-specific hashes
            VsfType::hP(value) => {
                let mut flat = vec![b'h', b'P'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }
            VsfType::hR(value) => {
                let mut flat = vec![b'h', b'R'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            // ==================== SIGNATURES ====================
            VsfType::ge(value) => {
                let mut flat = vec![b'g', b'e'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual signature bytes
                flat
            }

            VsfType::gp(value) => {
                let mut flat = vec![b'g', b'p'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual signature bytes
                flat
            }

            VsfType::gr(value) => {
                let mut flat = vec![b'g', b'r'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value); // Write actual signature bytes
                flat
            }

            VsfType::gd(value) => {
                let mut flat = vec![b'g', b'd'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::gs(value) => {
                let mut flat = vec![b'g', b's'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::gf(value) => {
                let mut flat = vec![b'g', b'f'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            // ==================== KEYS ====================
            VsfType::ke(value) => {
                let mut flat = vec![b'k', b'e'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kx(value) => {
                let mut flat = vec![b'k', b'x'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kp(value) => {
                let mut flat = vec![b'k', b'p'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kc(value) => {
                let mut flat = vec![b'k', b'c'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ka(value) => {
                let mut flat = vec![b'k', b'a'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kk(value) => {
                let mut flat = vec![b'k', b'k'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::km(value) => {
                let mut flat = vec![b'k', b'm'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kf(value) => {
                let mut flat = vec![b'k', b'f'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kl(value) => {
                let mut flat = vec![b'k', b'l'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kn(value) => {
                let mut flat = vec![b'k', b'n'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kh(value) => {
                let mut flat = vec![b'k', b'h'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kd(value) => {
                let mut flat = vec![b'k', b'd'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kb(value) => {
                let mut flat = vec![b'k', b'b'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            // ==================== SHARED SECRETS (typed by algorithm) ====================
            VsfType::ksx(value) => {
                let mut flat = vec![b'k', b's', b'x'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksp(value) => {
                let mut flat = vec![b'k', b's', b'p'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksk(value) => {
                let mut flat = vec![b'k', b's', b'k'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksf(value) => {
                let mut flat = vec![b'k', b's', b'f'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksn(value) => {
                let mut flat = vec![b'k', b's', b'n'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksl(value) => {
                let mut flat = vec![b'k', b's', b'l'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksh(value) => {
                let mut flat = vec![b'k', b's', b'h'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ksm(value) => {
                let mut flat = vec![b'k', b's', b'm'];
                flat.extend_from_slice(&(value.len() - 1).encode_number());
                flat.extend_from_slice(value);
                flat
            }

            // ==================== MAC (MESSAGE AUTHENTICATION CODE) ====================
            VsfType::mh(value) => {
                let mut flat = vec![b'm', b'h'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::mp(value) => {
                let mut flat = vec![b'm', b'p'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::mb(value) => {
                let mut flat = vec![b'm', b'b'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            VsfType::mc(value) => {
                let mut flat = vec![b'm', b'c'];
                flat.extend_from_slice(&(value.len() - 1).encode_number()); // Store (len-1) in bytes
                flat.extend_from_slice(value);
                flat
            }

            // ==================== WRAPPED/ENCODED DATA ====================
            VsfType::v(algorithm, value) => {
                let mut flat = Vec::new();
                flat.push(b'v');
                flat.push(*algorithm); // Algorithm ID (e.g., b'z' for zstd, b'r' for Reed-Solomon)
                flat.extend_from_slice(&(value.len() << 3).encode_number()); // Convert bytes to bits
                flat.extend_from_slice(value);
                flat
            }

            // ==================== BITPACKED TENSORS ====================
            VsfType::p(tensor) => {
                // Validate dimensions
                assert!(
                    !tensor.shape.is_empty(),
                    "Bitpacked tensor must have at least one dimension"
                );

                // Validate data length
                let total_elements: usize = tensor.shape.iter().product();
                let bits_per_sample = if tensor.bit_depth == 0 {
                    256
                } else {
                    tensor.bit_depth as usize
                };
                let total_bits = total_elements * bits_per_sample;
                let expected_bytes = total_bits.div_ceil(8);
                assert_eq!(
                    tensor.data.len(),
                    expected_bytes,
                    "Bitpacked tensor data length {} doesn't match expected {} bytes for {} elements at {} bits/sample",
                    tensor.data.len(),
                    expected_bytes,
                    total_elements,
                    bits_per_sample
                );

                let mut flat = vec![b'p'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(tensor.bit_depth); // 0x0C for 12-bit, 0x00 for 256-bit
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                flat.extend_from_slice(&tensor.data);
                flat
            }

            // ==================== CONTIGUOUS TENSORS ====================
            VsfType::t_u0(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'0');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                // Pack bools into bytes (8 per byte)
                let mut byte = 0u8;
                let mut bit_pos = 0;
                for &value in &tensor.data {
                    if value {
                        byte |= 1 << (7 - bit_pos);
                    }
                    bit_pos += 1;
                    if bit_pos == 8 {
                        flat.push(byte);
                        byte = 0;
                        bit_pos = 0;
                    }
                }
                // Push partial byte if any
                if bit_pos > 0 {
                    flat.push(byte);
                }
                flat
            }

            VsfType::t_u3(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'3');
                    flat.extend_from_slice(&tensor.data);
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'u');
                    flat.push(b'3');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    flat.extend_from_slice(&tensor.data);
                }
                flat
            }

            VsfType::t_u4(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'4');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'u');
                    flat.push(b'4');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_u5(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'5');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'u');
                    flat.push(b'5');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_u6(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'6');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'u');
                    flat.push(b'6');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_u7(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'7');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'u');
                    flat.push(b'7');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            // Signed integer tensors
            VsfType::t_i3(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'3');
                    for &value in &tensor.data {
                        flat.push(value as u8);
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'i');
                    flat.push(b'3');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.push(value as u8);
                    }
                }
                flat
            }

            VsfType::t_i4(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'4');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'i');
                    flat.push(b'4');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_i5(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'5');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'i');
                    flat.push(b'5');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_i6(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'6');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'i');
                    flat.push(b'6');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::t_i7(tensor) => {
                let mut flat = vec![b't'];
                // 1D optimization: use 'n' for element count instead of ndim+shape
                if tensor.ndim() == 1 {
                    flat.push(b'n');
                    flat.extend_from_slice(&tensor.data.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'7');
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                } else {
                    flat.extend_from_slice(&tensor.ndim().encode_number());
                    flat.push(b'i');
                    flat.push(b'7');
                    for &dim in &tensor.shape {
                        flat.extend_from_slice(&dim.encode_number());
                    }
                    for &value in &tensor.data {
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            // Float tensors
            VsfType::t_f5(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'f');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_f6(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'f');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            // Complex tensors
            VsfType::t_j5(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'j');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for value in &tensor.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            VsfType::t_j6(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'j');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for value in &tensor.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            // ==================== VECTORS (1D CONTIGUOUS) ====================
            VsfType::v_u0(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'0');
                // Bit-pack bools (8 per byte)
                let mut byte = 0u8;
                let mut bit_pos = 0;
                for &value in &vector.data {
                    if value {
                        byte |= 1 << (7 - bit_pos);
                    }
                    bit_pos += 1;
                    if bit_pos == 8 {
                        flat.push(byte);
                        byte = 0;
                        bit_pos = 0;
                    }
                }
                // Push partial byte if any
                if bit_pos > 0 {
                    flat.push(byte);
                }
                flat
            }

            VsfType::v_u3(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'3');
                flat.extend_from_slice(&vector.data);
                flat
            }

            VsfType::v_u4(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'4');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_u5(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'5');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_u6(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'6');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_u7(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'u');
                flat.push(b'7');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_i3(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'i');
                flat.push(b'3');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_i4(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'i');
                flat.push(b'4');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_i5(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'i');
                flat.push(b'5');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_i6(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'i');
                flat.push(b'6');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_i7(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'i');
                flat.push(b'7');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_f5(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'f');
                flat.push(b'5');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_f6(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'f');
                flat.push(b'6');
                for &value in &vector.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::v_j5(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'j');
                flat.push(b'5');
                for value in &vector.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            VsfType::v_j6(vector) => {
                let mut flat = vec![b't', b'n'];
                flat.extend_from_slice(&vector.data.len().encode_number());
                flat.push(b'j');
                flat.push(b'6');
                for value in &vector.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            // ==================== STRIDED TENSORS ====================
            VsfType::q_u0(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'0');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                // Pack bools
                let mut byte = 0u8;
                let mut bit_pos = 0;
                for &value in &tensor.data {
                    if value {
                        byte |= 1 << (7 - bit_pos);
                    }
                    bit_pos += 1;
                    if bit_pos == 8 {
                        flat.push(byte);
                        byte = 0;
                        bit_pos = 0;
                    }
                }
                if bit_pos > 0 {
                    flat.push(byte);
                }
                flat
            }

            VsfType::q_u3(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'3');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                flat.extend_from_slice(&tensor.data);
                flat
            }

            VsfType::q_u4(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'4');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_u5(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_u6(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_u7(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'7');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_i3(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'3');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.push(value as u8);
                }
                flat
            }

            VsfType::q_i4(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'4');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_i5(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_i6(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_i7(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'7');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_f5(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'f');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_f6(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'f');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::q_j5(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'j');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for value in &tensor.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            VsfType::q_j6(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'j');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }
                for value in &tensor.data {
                    flat.extend_from_slice(&value.re.to_be_bytes());
                    flat.extend_from_slice(&value.im.to_be_bytes());
                }
                flat
            }

            // ==================== SPIRIX SCALARS (PRIMITIVES) ==================== Format: [s][F][E][fraction_bytes][exponent_bytes]
            #[cfg(feature = "spirix")]
            VsfType::s33(v) => {
                let mut flat = vec![b's', b'3', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s34(v) => {
                let mut flat = vec![b's', b'3', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s35(v) => {
                let mut flat = vec![b's', b'3', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s36(v) => {
                let mut flat = vec![b's', b'3', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s37(v) => {
                let mut flat = vec![b's', b'3', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s43(v) => {
                let mut flat = vec![b's', b'4', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s44(v) => {
                let mut flat = vec![b's', b'4', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s45(v) => {
                let mut flat = vec![b's', b'4', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s46(v) => {
                let mut flat = vec![b's', b'4', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s47(v) => {
                let mut flat = vec![b's', b'4', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s53(v) => {
                let mut flat = vec![b's', b'5', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s54(v) => {
                let mut flat = vec![b's', b'5', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s55(v) => {
                let mut flat = vec![b's', b'5', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s56(v) => {
                let mut flat = vec![b's', b'5', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s57(v) => {
                let mut flat = vec![b's', b'5', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s63(v) => {
                let mut flat = vec![b's', b'6', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s64(v) => {
                let mut flat = vec![b's', b'6', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s65(v) => {
                let mut flat = vec![b's', b'6', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s66(v) => {
                let mut flat = vec![b's', b'6', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s67(v) => {
                let mut flat = vec![b's', b'6', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s73(v) => {
                let mut flat = vec![b's', b'7', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s74(v) => {
                let mut flat = vec![b's', b'7', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s75(v) => {
                let mut flat = vec![b's', b'7', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s76(v) => {
                let mut flat = vec![b's', b'7', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::s77(v) => {
                let mut flat = vec![b's', b'7', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }

            // ==================== SPIRIX CIRCLES (PRIMITIVES) ==================== Format: [c][F][E][real_bytes][imaginary_bytes][exponent_bytes]
            #[cfg(feature = "spirix")]
            VsfType::c33(v) => {
                let mut flat = vec![b'c', b'3', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c34(v) => {
                let mut flat = vec![b'c', b'3', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c35(v) => {
                let mut flat = vec![b'c', b'3', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c36(v) => {
                let mut flat = vec![b'c', b'3', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c37(v) => {
                let mut flat = vec![b'c', b'3', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c43(v) => {
                let mut flat = vec![b'c', b'4', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c44(v) => {
                let mut flat = vec![b'c', b'4', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c45(v) => {
                let mut flat = vec![b'c', b'4', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c46(v) => {
                let mut flat = vec![b'c', b'4', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c47(v) => {
                let mut flat = vec![b'c', b'4', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c53(v) => {
                let mut flat = vec![b'c', b'5', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c54(v) => {
                let mut flat = vec![b'c', b'5', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c55(v) => {
                let mut flat = vec![b'c', b'5', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c56(v) => {
                let mut flat = vec![b'c', b'5', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c57(v) => {
                let mut flat = vec![b'c', b'5', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c63(v) => {
                let mut flat = vec![b'c', b'6', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c64(v) => {
                let mut flat = vec![b'c', b'6', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c65(v) => {
                let mut flat = vec![b'c', b'6', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c66(v) => {
                let mut flat = vec![b'c', b'6', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c67(v) => {
                let mut flat = vec![b'c', b'6', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c73(v) => {
                let mut flat = vec![b'c', b'7', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c74(v) => {
                let mut flat = vec![b'c', b'7', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c75(v) => {
                let mut flat = vec![b'c', b'7', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c76(v) => {
                let mut flat = vec![b'c', b'7', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::c77(v) => {
                let mut flat = vec![b'c', b'7', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            // ==================== SPIRIX SCALAR TENSORS ==================== Format: [t][dim_count][s][F][E][shape...][data...]
            #[cfg(feature = "spirix")]
            VsfType::t_s33(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s34(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s35(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s36(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s37(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s43(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s44(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s45(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s46(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s47(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s53(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s54(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s55(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s56(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s57(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s63(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s64(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s65(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s66(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s67(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s73(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s74(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s75(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s76(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_s77(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            // ==================== SPIRIX CIRCLE TENSORS ==================== Format: [t][dim_count][c][F][E][shape...][data...]
            #[cfg(feature = "spirix")]
            VsfType::t_c33(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'3');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c34(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'3');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c35(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'3');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c36(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'3');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c37(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'3');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c43(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'4');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c44(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'4');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c45(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'4');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c46(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'4');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c47(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'4');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c53(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'5');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c54(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'5');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c55(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'5');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c56(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'5');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c57(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'5');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c63(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'6');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c64(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'6');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c65(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'6');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c66(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'6');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c67(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'6');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c73(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'7');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c74(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'7');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c75(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'7');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c76(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'7');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::t_c77(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'c');
                flat.push(b'7');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                // Encode each Scalar as fraction + exponent
                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }

            // ==================== SPIRIX STRIDED TENSORS ==================== Same as above but with 'q' marker and stride encoding
            #[cfg(feature = "spirix")]
            VsfType::q_s33(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s34(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s35(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s36(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s37(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s43(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s44(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s45(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s46(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s47(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s53(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s54(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s55(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s56(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s57(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s63(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s64(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s65(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s66(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s67(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s73(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s74(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s75(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s76(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_s77(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.fraction.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c33(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c34(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c35(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c36(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c37(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'3');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c43(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c44(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c45(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c46(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c47(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'4');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c53(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c54(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c55(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c56(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c57(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'5');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c63(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c64(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c65(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c66(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c67(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'6');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c73(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'3');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c74(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'4');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c75(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'5');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c76(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'6');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }
            #[cfg(feature = "spirix")]
            VsfType::q_c77(tensor) => {
                let mut flat = vec![b'q'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b's');
                flat.push(b'7');
                flat.push(b'7');

                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }

                for &stride in &tensor.stride {
                    flat.extend_from_slice(&stride.encode_number());
                }

                for value in &tensor.data {
                    flat.extend_from_slice(&value.real.to_be_bytes());
                    flat.extend_from_slice(&value.imaginary.to_be_bytes());
                    flat.extend_from_slice(&value.exponent.to_be_bytes());
                }

                flat
            }

            // ==================== TOKA OPCODES ====================
            VsfType::op(a, b) => {
                // Format: { a b } (4 bytes)
                vec![b'{', *a, *b, b'}']
            }
        }
    }

    /// Calculate the byte length of this VsfType when flattened, WITHOUT actually flattening
    ///
    /// This is much faster than flatten().len() and doesn't allocate.
    pub fn byte_len(&self) -> usize {
        match self {
            // ==================== UNSIGNED INTEGERS ====================
            VsfType::u0(_) => 2, // 'u' + value byte

            VsfType::u(value, inclusive) => {
                if *inclusive {
                    // Inclusive mode - would call encode_usize_inclusive() This is complex, so for now just flatten and measure TODO: optimize this later
                    self.flatten().len()
                } else {
                    1 + encoded_usize_len(*value) // 'u' + encoded number
                }
            }

            VsfType::u3(_) => 3,                      // 'u' + '3' + byte
            VsfType::u4(_) => 1 + encoded_u16_len(),  // 'u' + encoded u16
            VsfType::u5(_) => 1 + encoded_u32_len(),  // 'u' + encoded u32
            VsfType::u6(_) => 1 + encoded_u64_len(),  // 'u' + encoded u64
            VsfType::u7(_) => 1 + encoded_u128_len(), // 'u' + encoded u128

            // ==================== SIGNED INTEGERS ====================
            VsfType::i(value) => 1 + encoded_isize_len(*value), // 'i' + encoded number
            VsfType::i3(_) => 3,                                // 'i' + '3' + byte
            VsfType::i4(_) => 1 + encoded_i16_len(),            // 'i' + encoded i16
            VsfType::i5(_) => 1 + encoded_i32_len(),            // 'i' + encoded i32
            VsfType::i6(_) => 1 + encoded_i64_len(),            // 'i' + encoded i64
            VsfType::i7(_) => 1 + encoded_i128_len(),           // 'i' + encoded i128

            // ==================== FLOATS ====================
            VsfType::f5(_) => 6,  // 'f' + '5' + 4 bytes
            VsfType::f6(_) => 10, // 'f' + '6' + 8 bytes

            // ==================== COMPLEX ====================
            VsfType::j5(_) => 10, // 'j' + '5' + 8 bytes (2×f32)
            VsfType::j6(_) => 18, // 'j' + '6' + 16 bytes (2×f64)

            // ==================== VSF STRUCTURE ====================
            VsfType::d(s) => {
                // 'd' + encoded_string
                #[cfg(feature = "text")]
                let encoded_len = encode_text(s).0.len();
                #[cfg(not(feature = "text"))]
                let encoded_len = s.len();
                1 + encoded_usize_len(encoded_len) + encoded_len
            }

            VsfType::a(s) => {
                // 'a' + encoded_string (ASCII text)
                #[cfg(feature = "text")]
                let encoded_len = encode_text(s).0.len();
                #[cfg(not(feature = "text"))]
                let encoded_len = s.len();
                1 + encoded_usize_len(encoded_len) + encoded_len
            }

            VsfType::o(offset) => {
                // 'o' + encoded offset (as usize)
                1 + encoded_usize_len(*offset)
            }

            VsfType::b(size, _inclusive) => {
                // 'b' + encoded size (as usize) Note: inclusive mode is handled by stabilization, encoding is same
                1 + encoded_usize_len(*size)
            }

            VsfType::l(size, _inclusive) => {
                // 'l' + encoded size (length in bytes, wire or file)
                1 + encoded_usize_len(*size)
            }

            VsfType::na(_, host, port) => { 2 + 1 + host.len() + 1 + if port.is_some() { 2 } else { 0 } }
            VsfType::nh(host) => { 2 + host.len() + 1 }
            VsfType::ni(_) => { 2 + 4 }
            VsfType::nj(_) => { 2 + 16 }
            VsfType::nc(addr, _) => { 2 + addr.len() + 1 + 1 }
            VsfType::nm(_) => { 2 + 6 }
            VsfType::np(_) => { 2 + 2 }
            VsfType::ns(host, _) => { 2 + host.len() + 1 + 2 }
            VsfType::nu(url) => { 2 + url.len() + 1 }
            VsfType::nn(name) => { 2 + name.len() + 1 }

            VsfType::n(count) => {
                // 'n' + encoded count (as usize)
                1 + encoded_usize_len(*count)
            }

            // ==================== VERSION/COMPAT ====================
            VsfType::z(ver) => 1 + encoded_usize_len(*ver), // 'z' + encoded version
            VsfType::y(compat) => 1 + encoded_usize_len(*compat), // 'y' + encoded compat

            // ==================== CRYPTO PRIMITIVES ====================
            VsfType::hb(bytes) | VsfType::hs(bytes) | VsfType::hm(bytes) | VsfType::hg(bytes) => {
                // prefix + encoded_length + data
                2 + encoded_usize_len(bytes.len()) + bytes.len()
            }

            VsfType::ge(bytes) | VsfType::gp(bytes) | VsfType::gr(bytes) => {
                // prefix + encoded_length + data
                2 + encoded_usize_len(bytes.len()) + bytes.len()
            }

            VsfType::ke(bytes)
            | VsfType::kx(bytes)
            | VsfType::kp(bytes)
            | VsfType::kk(bytes)
            | VsfType::kc(bytes)
            | VsfType::ka(bytes)
            | VsfType::km(bytes)
            | VsfType::kf(bytes)
            | VsfType::kl(bytes)
            | VsfType::kn(bytes)
            | VsfType::kh(bytes)
            | VsfType::kd(bytes)
            | VsfType::kb(bytes) => {
                // prefix + encoded_length + data
                2 + encoded_usize_len(bytes.len()) + bytes.len()
            }

            // Shared secrets have 3-byte prefix (ksx, ksp, etc.)
            VsfType::ksx(bytes)
            | VsfType::ksp(bytes)
            | VsfType::ksk(bytes)
            | VsfType::ksf(bytes)
            | VsfType::ksn(bytes)
            | VsfType::ksl(bytes)
            | VsfType::ksh(bytes)
            | VsfType::ksm(bytes) => {
                // 3-byte prefix + encoded_length + data
                3 + encoded_usize_len(bytes.len()) + bytes.len()
            }

            VsfType::mh(bytes) | VsfType::mp(bytes) | VsfType::mb(bytes) | VsfType::mc(bytes) => {
                // prefix + encoded_length + data (MAC family)
                2 + encoded_usize_len(bytes.len()) + bytes.len()
            }

            // Toka opcodes: { a b } = 4 bytes
            VsfType::op(_, _) => 4,

            // For complex types, fall back to flatten().len() TODO: add optimized versions for these
            _ => self.flatten().len(),
        }
    }
}

/// Helper: calculate encoded length of usize value
fn encoded_usize_len(value: usize) -> usize {
    if value <= u8::MAX as usize {
        2 // '3' + 1 byte
    } else if value <= u16::MAX as usize {
        3 // '4' + 2 bytes
    } else if value <= u32::MAX as usize {
        5 // '5' + 4 bytes
    } else if value <= u64::MAX as usize {
        9 // '6' + 8 bytes
    } else {
        17 // '7' + 16 bytes
    }
}

/// Helper: calculate encoded length of isize value
fn encoded_isize_len(value: isize) -> usize {
    if value >= i8::MIN as isize && value <= i8::MAX as isize {
        2 // '3' + 1 byte
    } else if value >= i16::MIN as isize && value <= i16::MAX as isize {
        3 // '4' + 2 bytes
    } else if value >= i32::MIN as isize && value <= i32::MAX as isize {
        5 // '5' + 4 bytes
    } else if value >= i64::MIN as isize && value <= i64::MAX as isize {
        9 // '6' + 8 bytes
    } else {
        17 // '7' + 16 bytes
    }
}

fn encoded_u16_len() -> usize {
    3
} // '4' + 2 bytes
fn encoded_u32_len() -> usize {
    5
} // '5' + 4 bytes
fn encoded_u64_len() -> usize {
    9
} // '6' + 8 bytes
fn encoded_u128_len() -> usize {
    17
} // '7' + 16 bytes

fn encoded_i16_len() -> usize {
    3
} // '4' + 2 bytes
fn encoded_i32_len() -> usize {
    5
} // '5' + 4 bytes
fn encoded_i64_len() -> usize {
    9
} // '6' + 8 bytes
fn encoded_i128_len() -> usize {
    17
} // '7' + 16 bytes

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex;

    #[test]
    fn test_flatten_unsigned() {
        assert_eq!(VsfType::u3(42).flatten(), vec![b'u', b'3', 42]);
        assert_eq!(
            VsfType::u5(100000).flatten(),
            vec![b'u', b'5', 0x00, 0x01, 0x86, 0xA0]
        );
    }

    #[test]
    fn test_flatten_signed() {
        assert_eq!(VsfType::i3(-42).flatten(), vec![b'i', b'3', 0xD6]);
    }

    #[test]
    fn test_flatten_float() {
        let result = VsfType::f5(3.14f32).flatten();
        assert_eq!(result[0], b'f');
        assert_eq!(result[1], b'5');
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_flatten_complex() {
        let z = Complex {
            re: 1.5f32,
            im: 2.5f32,
        };
        let result = VsfType::j5(z).flatten();
        assert_eq!(result[0], b'j');
        assert_eq!(result[1], b'5');
        assert_eq!(result.len(), 10); // 2 + 4 + 4
    }

    #[test]
    fn test_flatten_string() {
        let result = VsfType::x("hello".to_string()).flatten();
        assert_eq!(result[0], b'x');
        // New format: x [char_count] [huffman_bytes]
        assert_eq!(result[1], b'3'); // char_count=5, encoded as u8
        assert_eq!(result[2], 5); // 5 characters
                                  // result[3..] is Huffman-encoded data

        // Overhead: 3 bytes (x + marker + count) For short strings, Huffman may not compress
        assert!(result.len() >= 3); // At minimum: x + char_count_marker + char_count
    }

    #[test]
    fn test_flatten_metadata() {
        assert_eq!(VsfType::z(1).flatten(), vec![b'z', b'3', 1]);
        assert_eq!(VsfType::n(42).flatten(), vec![b'n', b'3', 42]);
    }

    #[test]
    fn test_flatten_bool() {
        assert_eq!(VsfType::u0(true).flatten(), vec![b'u', 255]);
        assert_eq!(VsfType::u0(false).flatten(), vec![b'u', 0]);
    }

    #[test]
    fn test_flatten_tensor_lumis_raw() {
        // Lumis RAW: 4096×3072 u16 tensor
        use crate::types::Tensor;

        let tensor = Tensor::new(vec![4096, 3072], vec![0u16; 4096 * 3072]);

        let result = VsfType::t_u4(tensor).flatten();

        // Check structure
        assert_eq!(result[0], b't'); // Tensor marker
        assert_eq!(result[1], b'3'); // Dim count size marker
        assert_eq!(result[2], 2); // 2 dimensions
        assert_eq!(result[3], b'u'); // Element type
        assert_eq!(result[4], b'4'); // u16

        // Dimension 0: 4096 needs u16 ('4')
        assert_eq!(result[5], b'4'); // Size marker
        assert_eq!(result[6], 0x10); // 4096 = 0x1000
        assert_eq!(result[7], 0x00);

        // Dimension 1: 3072 needs u16 ('4')
        assert_eq!(result[8], b'4'); // Size marker
        assert_eq!(result[9], 0x0C); // 3072 = 0x0C00
        assert_eq!(result[10], 0x00);

        // Data starts at byte 11 Total size = 11 (header) + 4096*3072*2 (data) = 25,165,835 bytes
        assert_eq!(result.len(), 11 + 4096 * 3072 * 2);
    }

    #[test]
    fn test_flatten_tensor_small() {
        // Small 24×71 u8 image
        use crate::types::Tensor;

        let tensor = Tensor::new(vec![24, 71], vec![0u8; 24 * 71]);

        let result = VsfType::t_u3(tensor).flatten();

        assert_eq!(result[0], b't');
        assert_eq!(result[1], b'3'); // Dim count
        assert_eq!(result[2], 2);
        assert_eq!(result[3], b'u');
        assert_eq!(result[4], b'3');

        // Both dimensions fit in u8
        assert_eq!(result[5], b'3');
        assert_eq!(result[6], 24);
        assert_eq!(result[7], b'3');
        assert_eq!(result[8], 71);

        // Header = 9 bytes, data = 24*71 = 1704 bytes
        assert_eq!(result.len(), 9 + 24 * 71);
    }

    #[test]
    fn test_flatten_strided_tensor() {
        // Column-major 100×50 f64 matrix
        use crate::types::StridedTensor;

        let tensor = StridedTensor::new(
            vec![100, 50],
            vec![1, 100], // Column-major stride
            vec![0.0f64; 100 * 50],
        );

        let result = VsfType::q_f6(tensor).flatten();

        assert_eq!(result[0], b'q'); // Strided marker
        assert_eq!(result[1], b'3'); // Dim count size
        assert_eq!(result[2], 2); // 2 dimensions
        assert_eq!(result[3], b'f');
        assert_eq!(result[4], b'6');

        // Shape
        assert_eq!(result[5], b'3');
        assert_eq!(result[6], 100);
        assert_eq!(result[7], b'3');
        assert_eq!(result[8], 50);

        // Stride
        assert_eq!(result[9], b'3');
        assert_eq!(result[10], 1);
        assert_eq!(result[11], b'3');
        assert_eq!(result[12], 100);

        // Data starts at byte 13
        assert_eq!(result.len(), 13 + 100 * 50 * 8);
    }

    #[test]
    #[cfg(feature = "spirix")]
    fn test_flatten_spirix_scalar() {
        use spirix::ScalarF6E4;

        let scalar = ScalarF6E4::from(42.5);
        let result = VsfType::s64(scalar).flatten();

        assert_eq!(result[0], b's');
        assert_eq!(result[1], b'6');
        assert_eq!(result[2], b'4');
        // 3 marker bytes + 8 fraction bytes + 2 exponent bytes = 13 total
        assert_eq!(result.len(), 13);
    }

    #[test]
    #[cfg(feature = "spirix")]
    fn test_flatten_spirix_circle() {
        use spirix::CircleF6E4;

        let circle = CircleF6E4::from((1.5, 2.5));
        let result = VsfType::c64(circle).flatten();

        assert_eq!(result[0], b'c');
        assert_eq!(result[1], b'6');
        assert_eq!(result[2], b'4');
        // 3 marker bytes + 8 real + 8 imag + 2 exp = 21 total
        assert_eq!(result.len(), 21);
    }

    #[test]
    #[cfg(feature = "spirix")]
    fn test_flatten_spirix_tensor() {
        use crate::types::Tensor;
        use spirix::ScalarF6E4;

        let tensor = Tensor::new(vec![10, 20], vec![ScalarF6E4::from(42.0); 200]);

        let result = VsfType::t_s64(tensor).flatten();

        assert_eq!(result[0], b't');
        assert_eq!(result[1], b'3');
        assert_eq!(result[2], 2); // 2D
        assert_eq!(result[3], b's');
        assert_eq!(result[4], b'6');
        assert_eq!(result[5], b'4');

        // Header: 1 + 2 + 3 + 2 + 2 = 10 bytes Data: 200 elements × 10 bytes each = 2000 bytes
        assert_eq!(result.len(), 10 + 200 * 10);
    }
}
