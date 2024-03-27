/// A library for working with the Versatile Storage Format (VSF).
/// 
/// Provides a Rust-ey set of types for representing various data formats,
/// including integers, floating-point numbers, complex numbers, arrays, and special VSF-specific types.
use num_complex::Complex;
enum VsfType {
    // Unsigned Integer Types
    u3(u8),   // Unsigned 8-bit integer, 2^n notation (2^3=8 bits)
    u4(u16),  // Unsigned 16-bit integer, 2^n notation (2^4=16 bits)
    u5(u32),  // Unsigned 32-bit integer, 2^n notation (2^5=32 bits)
    u6(u64),  // Unsigned 64-bit integer, 2^n notation (2^6=64 bits)
    u7(u128), // Unsigned 128-bit integer, 2^n notation (2^7=128 bits)

    // Signed Integer Types
    s3(u8),   // Signed 8-bit integer
    s4(u16),  // Signed 16-bit integer
    s5(u32),  // Signed 32-bit integer
    s6(u64),  // Signed 64-bit integer
    s7(u128), // Signed 128-bit integer

    // IEEE 754 Floating-point Types
    e5(f32), // 32-bit floating point, 2^n notation, n is always bit count
    e6(f64), // 64-bit floating point

    // Unsigned Integer Arrays
    au3(Vec<u8>),   // Array of Unsigned 8-bit integer
    au4(Vec<u16>),  // Array of Unsigned 16-bit integer
    au5(Vec<u32>),  // Array of Unsigned 32-bit integer
    au6(Vec<u64>),  // Array of Unsigned 64-bit integer
    au7(Vec<u128>), // Array of Unsigned 128-bit integer

    // Signed Integer Arrays
    as3(Vec<i8>),   // Array of Signed 8-bit integer
    as4(Vec<i16>),  // Array of Signed 16-bit integer
    as5(Vec<i32>),  // Array of Signed 32-bit integer
    as6(Vec<i64>),  // Array of Signed 64-bit integer
    as7(Vec<i128>), // Array of Signed 128-bit integer

    // Floating-point Arrays
    af5(Vec<f32>), // Array of 32-bit floating point
    af6(Vec<f64>), // Array of 64-bit floating point

    // Complex Numbers
    i6(Complex<f32>),       // Single complex number with f32 components
    i7(Complex<f64>),       // Single complex number with f64 components
    ai6(Vec<Complex<f32>>), // Array of complex numbers with f32 components
    ai7(Vec<Complex<f64>>), // Array of complex numbers with f64 components

    // Special Types
    u0(bool),       // Boolean, stored 8 bit aligned, recomend filling all 8 bits
    au0(Vec<bool>), // Array of Boolean, extra bits are filled with 0 to align to 8 bits
    u(String),     // Unicode text

    // VSF-specific Types
    s(usize),   // File size in bits
    o(usize),   // Offset in bits
    l(usize),   // Length in bits
    z(usize),   // Version
    y(usize),   // Backward version
    m(usize),   // Marker definition
    r(usize),   // Marker
    k(usize),   // Keyframe
    e(usize),   // Frame
    h(Vec<u8>), // Hash
    g(Vec<u8>), // Signature
}

impl VsfType {
    fn flatten(&self) -> Result<Vec<u8>, String> {
        match self {
            // Unsigned Integer Types
            VsfType::u3(value) => Ok(vec![b'u', b'3', *value]),
            VsfType::u4(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b'u', b'4', bytes[0], bytes[1]])
            }
            VsfType::u5(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b'u', b'5', bytes[0], bytes[1], bytes[2], bytes[3]])
            }
            VsfType::u6(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b'u', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ])
            }
            VsfType::u7(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b'u', b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
                    bytes[13], bytes[14], bytes[15],
                ])
            }

            // Signed Integer Types
            VsfType::s3(value) => Ok(vec![b's', b'3', *value]),
            VsfType::s4(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b's', b'4', bytes[0], bytes[1]])
            }
            VsfType::s5(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b's', b'5', bytes[0], bytes[1], bytes[2], bytes[3]])
            }
            VsfType::s6(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b's', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ])
            }
            VsfType::s7(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b's', b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
                    bytes[13], bytes[14], bytes[15],
                ])
            }

            // Floating-point Types
            VsfType::e5(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b'f', b'5', bytes[0], bytes[1], bytes[2], bytes[3]])
            }
            VsfType::e6(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b'f', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ])
            }

            // Unsigned Integer Vectors
            VsfType::au3(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'u');
                flat.push(b'3');
                flat.extend_from_slice(values);
                Ok(flat)
            }
            VsfType::au4(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'u');
                flat.push(b'4');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::au5(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'u');
                flat.push(b'5');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::au6(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'u');
                flat.push(b'6');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::au7(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'u');
                flat.push(b'7');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }

            // Signed Integer Vectors
            VsfType::as3(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b's');
                flat.push(b'3');
                for value in values {
                    flat.push(*value as u8);
                }
                Ok(flat)
            }
            VsfType::as4(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b's');
                flat.push(b'4');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::as5(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b's');
                flat.push(b'5');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::as6(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b's');
                flat.push(b'6');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::as7(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b's');
                flat.push(b'7');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }

            // Floating-point Vectors
            VsfType::af5(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'f');
                flat.push(b'5');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::af6(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'f');
                flat.push(b'6');
                for value in values {
                    let bytes = value.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }

            // Complex Numbers
            VsfType::i6(value) => {
                let mut flat = Vec::new();
                flat.push(b'i');
                flat.push(b'6');
                let bytes = value.re.to_be_bytes();
                flat.extend_from_slice(&bytes);
                let bytes = value.im.to_be_bytes();
                flat.extend_from_slice(&bytes);
                Ok(flat)
            }
            VsfType::i7(value) => {
                let mut flat = Vec::new();
                flat.push(b'i');
                flat.push(b'7');
                let bytes = value.re.to_be_bytes();
                flat.extend_from_slice(&bytes);
                let bytes = value.im.to_be_bytes();
                flat.extend_from_slice(&bytes);
                Ok(flat)
            }
            VsfType::ai6(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'i');
                flat.push(b'6');
                for value in values {
                    let bytes = value.re.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                    let bytes = value.im.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            VsfType::ai7(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&encode_length(values.len()));
                flat.push(b'i');
                flat.push(b'7');
                for value in values {
                    let bytes = value.re.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                    let bytes = value.im.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                }
                Ok(flat)
            }
            _ => Err(String::from("Unsupported type for flattening")),
        }
    }
}

fn encode_length(length: usize) -> Vec<u8> {
    let mut flat = Vec::new();
    if length < (std::u8::MAX / 2) as usize {
        flat.push(b'3'); // Indicate that length fits in one byte (2^n notation, 2^3=8 bits)
        flat.push(length as u8);
    } else if length < (std::u16::MAX / 2) as usize {
        flat.push(b'4'); // Indicate that length fits in two bytes (2^4=16 bits)
        flat.extend_from_slice(&(length as u16).to_be_bytes());
    } else if length < (std::u32::MAX / 2) as usize {
        flat.push(b'5'); // Indicate that length fits in four bytes (2^5=32 bits)
        flat.extend_from_slice(&(length as u32).to_be_bytes());
    } else if length < (std::u64::MAX / 2) as usize {
        flat.push(b'6'); // Indicate that length fits in eight bytes (2^6=64 bits)
        flat.extend_from_slice(&(length as u64).to_be_bytes());
    } else {
        flat.push(b'7'); // Indicate that length fits in sixteen bytes (2^7=128 bits)
        flat.extend_from_slice(&(length as u128).to_be_bytes());
    }
    flat
}

/// The main object for working with VSF. It allows storing mixed types
/// and provides functionality to flatten the entire structure into
/// a single byte vector.
struct VsfObject {
    /// The container for storing various `VsfValue` items.
    values: Vec<Box<VsfType>>,
}
