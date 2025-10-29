use super::traits::{EncodeNumber, EncodeNumberInclusive};
use crate::text_encoding::encode_text;
use crate::types::{EtType, VsfType};

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
                let mut flat = Vec::new();
                flat.push(b'x');

                // Huffman-encode the text (no internal header)
                let encoded_text = encode_text(value);

                // Encode ONLY character count (for Huffman decoder)
                // VSF structure handles byte boundaries
                let char_count = value.chars().count();
                flat.extend_from_slice(&char_count.encode_number());

                // Append Huffman bytes directly
                flat.extend_from_slice(&encoded_text);

                flat
            }

            VsfType::e(value) => {
                let mut flat = Vec::new();
                match value {
                    EtType::u(value) => {
                        flat.push(b'e');
                        flat.push(b'u');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::u5(value) => {
                        flat.push(b'e');
                        flat.push(b'u');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::u6(value) => {
                        flat.push(b'e');
                        flat.push(b'u');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::u7(value) => {
                        flat.push(b'e');
                        flat.push(b'u');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::i(value) => {
                        flat.push(b'e');
                        flat.push(b'i');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::i5(value) => {
                        flat.push(b'e');
                        flat.push(b'i');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::i6(value) => {
                        flat.push(b'e');
                        flat.push(b'i');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::i7(value) => {
                        flat.push(b'e');
                        flat.push(b'i');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    EtType::f5(value) => {
                        flat.push(b'e');
                        flat.push(b'f');
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                    EtType::f6(value) => {
                        flat.push(b'e');
                        flat.push(b'f');
                        flat.extend_from_slice(&value.to_be_bytes());
                    }
                }
                flat
            }

            VsfType::w(coord) => {
                let mut flat = vec![b'w'];
                flat.extend_from_slice(&coord.raw().to_be_bytes());
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

            VsfType::l(value) => {
                let mut flat = Vec::new();
                flat.push(b'l');
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
                // Note: inclusive mode is handled by the stabilization loop in vsf_builder
                // The value already includes its own encoding size when needed
                let mut flat = Vec::new();
                flat.push(b'b');
                flat.extend_from_slice(&value.encode_number());
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

            VsfType::m(value) => {
                let mut flat = Vec::new();
                flat.push(b'm');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            VsfType::r(value) => {
                let mut flat = Vec::new();
                flat.push(b'r');
                flat.extend_from_slice(&value.encode_number());
                flat
            }

            // ==================== HASHES ====================
            VsfType::hb3(value) => {
                let mut flat = vec![b'h', b'b', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::hb4(value) => {
                let mut flat = vec![b'h', b'b', b'4'];
                flat.extend_from_slice(&((value.len() - 1) as u16).to_be_bytes()); // Store (len-1) as u16
                flat.extend_from_slice(value);
                flat
            }

            VsfType::h23(value) => {
                let mut flat = vec![b'h', b'2', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::h53(value) => {
                let mut flat = vec![b'h', b'5', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            // ==================== SIGNATURES ====================
            VsfType::ge3(value) => {
                let mut flat = vec![b'g', b'e', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::gp3(value) => {
                let mut flat = vec![b'g', b'p', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::gr4(value) => {
                let mut flat = vec![b'g', b'r', b'4'];
                flat.extend_from_slice(&((value.len() - 1) as u16).to_be_bytes()); // Store (len-1) as u16
                flat.extend_from_slice(value);
                flat
            }

            // ==================== KEYS ====================
            VsfType::ke3(value) => {
                let mut flat = vec![b'k', b'e', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kx3(value) => {
                let mut flat = vec![b'k', b'x', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kp3(value) => {
                let mut flat = vec![b'k', b'p', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::kc3(value) => {
                let mut flat = vec![b'k', b'c', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ka3(value) => {
                let mut flat = vec![b'k', b'a', b'3'];
                flat.push((value.len() - 1) as u8); // Store (len-1) as single byte
                flat.extend_from_slice(value);
                flat
            }

            // ==================== MAC (MESSAGE AUTHENTICATION CODE) ====================
            VsfType::ah3(value) => {
                let mut flat = vec![b'a', b'h', b'3'];
                flat.push((value.len() - 1) as u8);
                flat.extend_from_slice(value);
                flat
            }

            VsfType::as3(value) => {
                let mut flat = vec![b'a', b's', b'3'];
                flat.push((value.len() - 1) as u8);
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ap3(value) => {
                let mut flat = vec![b'a', b'p', b'3'];
                flat.push((value.len() - 1) as u8);
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ab3(value) => {
                let mut flat = vec![b'a', b'b', b'3'];
                flat.push((value.len() - 1) as u8);
                flat.extend_from_slice(value);
                flat
            }

            VsfType::ac3(value) => {
                let mut flat = vec![b'a', b'c', b'3'];
                flat.push((value.len() - 1) as u8);
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
                let expected_bytes = (total_bits + 7) / 8;
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
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'3');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                flat.extend_from_slice(&tensor.data);
                flat
            }

            VsfType::t_u4(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'4');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_u5(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_u6(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_u7(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'u');
                flat.push(b'7');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            // Signed integer tensors
            VsfType::t_i3(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'3');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.push(value as u8);
                }
                flat
            }

            VsfType::t_i4(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'4');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_i5(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'5');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_i6(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'6');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
                }
                flat
            }

            VsfType::t_i7(tensor) => {
                let mut flat = vec![b't'];
                flat.extend_from_slice(&tensor.ndim().encode_number());
                flat.push(b'i');
                flat.push(b'7');
                for &dim in &tensor.shape {
                    flat.extend_from_slice(&dim.encode_number());
                }
                for &value in &tensor.data {
                    flat.extend_from_slice(&value.to_be_bytes());
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

            // ==================== SPIRIX SCALARS (PRIMITIVES) ====================
            // Format: [s][F][E][fraction_bytes][exponent_bytes]
            VsfType::s33(v) => {
                let mut flat = vec![b's', b'3', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s34(v) => {
                let mut flat = vec![b's', b'3', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s35(v) => {
                let mut flat = vec![b's', b'3', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s36(v) => {
                let mut flat = vec![b's', b'3', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s37(v) => {
                let mut flat = vec![b's', b'3', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s43(v) => {
                let mut flat = vec![b's', b'4', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s44(v) => {
                let mut flat = vec![b's', b'4', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s45(v) => {
                let mut flat = vec![b's', b'4', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s46(v) => {
                let mut flat = vec![b's', b'4', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s47(v) => {
                let mut flat = vec![b's', b'4', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s53(v) => {
                let mut flat = vec![b's', b'5', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s54(v) => {
                let mut flat = vec![b's', b'5', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s55(v) => {
                let mut flat = vec![b's', b'5', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s56(v) => {
                let mut flat = vec![b's', b'5', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s57(v) => {
                let mut flat = vec![b's', b'5', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s63(v) => {
                let mut flat = vec![b's', b'6', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s64(v) => {
                let mut flat = vec![b's', b'6', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s65(v) => {
                let mut flat = vec![b's', b'6', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s66(v) => {
                let mut flat = vec![b's', b'6', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s67(v) => {
                let mut flat = vec![b's', b'6', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s73(v) => {
                let mut flat = vec![b's', b'7', b'3'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s74(v) => {
                let mut flat = vec![b's', b'7', b'4'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s75(v) => {
                let mut flat = vec![b's', b'7', b'5'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s76(v) => {
                let mut flat = vec![b's', b'7', b'6'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::s77(v) => {
                let mut flat = vec![b's', b'7', b'7'];
                flat.extend_from_slice(&v.fraction.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }

            // ==================== SPIRIX CIRCLES (PRIMITIVES) ====================
            // Format: [c][F][E][real_bytes][imaginary_bytes][exponent_bytes]
            VsfType::c33(v) => {
                let mut flat = vec![b'c', b'3', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c34(v) => {
                let mut flat = vec![b'c', b'3', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c35(v) => {
                let mut flat = vec![b'c', b'3', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c36(v) => {
                let mut flat = vec![b'c', b'3', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c37(v) => {
                let mut flat = vec![b'c', b'3', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c43(v) => {
                let mut flat = vec![b'c', b'4', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c44(v) => {
                let mut flat = vec![b'c', b'4', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c45(v) => {
                let mut flat = vec![b'c', b'4', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c46(v) => {
                let mut flat = vec![b'c', b'4', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c47(v) => {
                let mut flat = vec![b'c', b'4', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c53(v) => {
                let mut flat = vec![b'c', b'5', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c54(v) => {
                let mut flat = vec![b'c', b'5', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c55(v) => {
                let mut flat = vec![b'c', b'5', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c56(v) => {
                let mut flat = vec![b'c', b'5', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c57(v) => {
                let mut flat = vec![b'c', b'5', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c63(v) => {
                let mut flat = vec![b'c', b'6', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c64(v) => {
                let mut flat = vec![b'c', b'6', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c65(v) => {
                let mut flat = vec![b'c', b'6', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c66(v) => {
                let mut flat = vec![b'c', b'6', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c67(v) => {
                let mut flat = vec![b'c', b'6', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c73(v) => {
                let mut flat = vec![b'c', b'7', b'3'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c74(v) => {
                let mut flat = vec![b'c', b'7', b'4'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c75(v) => {
                let mut flat = vec![b'c', b'7', b'5'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c76(v) => {
                let mut flat = vec![b'c', b'7', b'6'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            VsfType::c77(v) => {
                let mut flat = vec![b'c', b'7', b'7'];
                flat.extend_from_slice(&v.real.to_be_bytes());
                flat.extend_from_slice(&v.imaginary.to_be_bytes());
                flat.extend_from_slice(&v.exponent.to_be_bytes());
                flat
            }
            // ==================== SPIRIX SCALAR TENSORS ====================
            // Format: [t][dim_count][s][F][E][shape...][data...]
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
            // ==================== SPIRIX CIRCLE TENSORS ====================
            // Format: [t][dim_count][c][F][E][shape...][data...]
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

            // ==================== SPIRIX STRIDED TENSORS ====================
            // Same as above but with 'q' marker and stride encoding
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
                    // Inclusive mode - would call encode_usize_inclusive()
                    // This is complex, so for now just flatten and measure
                    // TODO: optimize this later
                    self.flatten().len()
                } else {
                    1 + encoded_usize_len(*value) // 'u' + encoded number
                }
            }

            VsfType::u3(_) => 3,  // 'u' + '3' + byte
            VsfType::u4(_) => 1 + encoded_u16_len(), // 'u' + encoded u16
            VsfType::u5(_) => 1 + encoded_u32_len(), // 'u' + encoded u32
            VsfType::u6(_) => 1 + encoded_u64_len(), // 'u' + encoded u64
            VsfType::u7(_) => 1 + encoded_u128_len(), // 'u' + encoded u128

            // ==================== SIGNED INTEGERS ====================
            VsfType::i(value) => 1 + encoded_isize_len(*value), // 'i' + encoded number
            VsfType::i3(_) => 3,  // 'i' + '3' + byte
            VsfType::i4(_) => 1 + encoded_i16_len(), // 'i' + encoded i16
            VsfType::i5(_) => 1 + encoded_i32_len(), // 'i' + encoded i32
            VsfType::i6(_) => 1 + encoded_i64_len(), // 'i' + encoded i64
            VsfType::i7(_) => 1 + encoded_i128_len(), // 'i' + encoded i128

            // ==================== FLOATS ====================
            VsfType::f5(_) => 6, // 'f' + '5' + 4 bytes
            VsfType::f6(_) => 10, // 'f' + '6' + 8 bytes

            // ==================== COMPLEX ====================
            VsfType::j5(_) => 10, // 'j' + '5' + 8 bytes (2×f32)
            VsfType::j6(_) => 18, // 'j' + '6' + 16 bytes (2×f64)

            // ==================== VSF STRUCTURE ====================
            VsfType::d(s) => {
                // 'd' + encoded_string
                let encoded_str = encode_text(s);
                1 + encoded_usize_len(encoded_str.len()) + encoded_str.len()
            }

            VsfType::l(s) => {
                // 'l' + encoded_string
                let encoded_str = encode_text(s);
                1 + encoded_usize_len(encoded_str.len()) + encoded_str.len()
            }

            VsfType::o(offset) => {
                // 'o' + encoded offset (as usize)
                1 + encoded_usize_len(*offset)
            }

            VsfType::b(size, _inclusive) => {
                // 'b' + encoded size (as usize)
                // Note: inclusive mode is handled by stabilization, encoding is same
                1 + encoded_usize_len(*size)
            }

            VsfType::n(count) => {
                // 'n' + encoded count (as usize)
                1 + encoded_usize_len(*count)
            }

            // ==================== VERSION/COMPAT ====================
            VsfType::z(ver) => 1 + encoded_usize_len(*ver), // 'z' + encoded version
            VsfType::y(compat) => 1 + encoded_usize_len(*compat), // 'y' + encoded compat

            // ==================== CRYPTO PRIMITIVES ====================
            VsfType::hb3(bytes) | VsfType::hb4(bytes) => {
                // 'h' + algo + '[' + size + ']' + data
                1 + 1 + 1 + encoded_usize_len(bytes.len()) + 1 + bytes.len()
            }

            VsfType::ge3(bytes) => {
                // 'g' + algo + '[' + size + ']' + data
                1 + 1 + 1 + encoded_usize_len(bytes.len()) + 1 + bytes.len()
            }

            VsfType::kc3(bytes) => {
                // 'k' + algo + '[' + size + ']' + data
                1 + 1 + 1 + encoded_usize_len(bytes.len()) + 1 + bytes.len()
            }

            // For complex types, fall back to flatten().len()
            // TODO: add optimized versions for these
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

fn encoded_u16_len() -> usize { 3 } // '4' + 2 bytes
fn encoded_u32_len() -> usize { 5 } // '5' + 4 bytes
fn encoded_u64_len() -> usize { 9 } // '6' + 8 bytes
fn encoded_u128_len() -> usize { 17 } // '7' + 16 bytes

fn encoded_i16_len() -> usize { 3 } // '4' + 2 bytes
fn encoded_i32_len() -> usize { 5 } // '5' + 4 bytes
fn encoded_i64_len() -> usize { 9 } // '6' + 8 bytes
fn encoded_i128_len() -> usize { 17 } // '7' + 16 bytes

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

        // Overhead: 3 bytes (x + marker + count)
        // For short strings, Huffman may not compress
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

        // Data starts at byte 11
        // Total size = 11 (header) + 4096*3072*2 (data) = 25,165,835 bytes
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

        // Header: 1 + 2 + 3 + 2 + 2 = 10 bytes
        // Data: 200 elements × 10 bytes each = 2000 bytes
        assert_eq!(result.len(), 10 + 200 * 10);
    }
}
