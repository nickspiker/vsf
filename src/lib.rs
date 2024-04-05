/// A library for working with the Versatile Storage Format (VSF).
///
/// Provides a Rust-ey set of types for representing various data formats,
/// including integers, floating-point numbers, complex numbers, arrays, and special VSF-specific types.
use num_complex::Complex;
use bitvec;
pub enum VsfType {
    // Unsigned Integer Types
    u0(bool), // Boolean, stored 8 bit aligned, recomend filling all 8 bits
    u3(u8),   // Unsigned 8-bit integer, 2^n notation (2^3=8 bits)
    u4(u16),  // Unsigned 16-bit integer, 2^n notation (2^4=16 bits)
    u5(u32),  // Unsigned 32-bit integer, 2^n notation (2^5=32 bits)
    u6(u64),  // Unsigned 64-bit integer, 2^n notation (2^6=64 bits)
    u7(u128), // Unsigned 128-bit integer, 2^n notation (2^7=128 bits)
    u(usize), // Unsigned integer of automatic size

    // Signed Integer Types
    s3(u8),   // Signed 8-bit integer
    s4(u16),  // Signed 16-bit integer
    s5(u32),  // Signed 32-bit integer
    s6(u64),  // Signed 64-bit integer
    s7(u128), // Signed 128-bit integer
    s(usize), // Signed integer of automatic size

    // IEEE 754 Floating-point Types
    f5(f32), // 32-bit floating point, 2^n notation, n is always bit count
    f6(f64), // 64-bit floating point

    // Unsigned Integer Arrays
    au0(Vec<bool>), // Array of Boolean, end bits are padded with 0's to align to 8 bits
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
    t(String), // Unicode text

    // VSF-specific Types
    d(String), // Data type
    x(String), // Label
    l(usize),  // Length in bits
    o(usize),  // Offset in bits
    z(usize),  // Version
    y(usize),  // Backward version
    m(usize),  // Marker definition
    r(usize),  // Marker
}

impl VsfType {
   pub fn flatten(&self) -> Result<Vec<u8>, String> {
        match self {
            // Unsigned Integer Types
            VsfType::u0(value)=>Ok(vec![b'u', b'0', if *value {255} else {0}]),
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
            VsfType::f5(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![b'f', b'5', bytes[0], bytes[1], bytes[2], bytes[3]])
            }
            VsfType::f6(value) => {
                let bytes = value.to_be_bytes();
                Ok(vec![
                    b'f', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                    bytes[6], bytes[7],
                ])
            }

            // Unsigned Integer Vectors
            VsfType::au0(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&values.len().encode_length(false));
                flat.push(b'u');
                flat.push(b'0');
                let mut byte = 0;
                for value in 0..values.len(){
                    byte |= if values[value] {1} else {0} << value % 8;
                    if value % 8 == 7 {
                        flat.push(byte);
                        byte = 0;
                    }
                }
                if values.len() % 8 != 0 {
                    flat.push(byte);
                }
                Ok(flat)
            }
            VsfType::au3(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&values.len().encode_length(false));
                flat.push(b'u');
                flat.push(b'3');
                flat.extend_from_slice(values);
                Ok(flat)
            }
            VsfType::au4(values) => {
                let mut flat = Vec::new();
                flat.push(b'a');
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
                flat.push(b'e');
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
                flat.extend_from_slice(&values.len().encode_length(false));
                flat.push(b'e');
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
                flat.extend_from_slice(&values.len().encode_length(false));
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
            VsfType::t(value) | VsfType::d(value) | VsfType::x(value) => {
                let mut flat = Vec::new();
                let type_identifier = match self {
                    VsfType::t(_) => b't', // Unicode text
                    VsfType::d(_) => b'd', // Label
                    VsfType::x(_) => b'x', // Data type
                    _ => return Err(String::from("Unsupported text type for flattening")),
                };
                flat.push(type_identifier);
                let bytes = value.as_bytes();
                let bits_length = bytes.len() * 8;
                flat.extend_from_slice(&bits_length.encode_length(false));
                flat.extend_from_slice(bytes);
                Ok(flat)
            },
            _ => Err(String::from("Unsupported type for flattening")),
        }
    }
}

/// Encodes the length of a vector into a VSF-style byte vector. Automatically sizes usize, other datatypes are maintained in bit sizes.
pub trait EncodeLength {
    fn encode_length(&self, inclusive: bool) -> Vec<u8>;
}
impl EncodeLength for u8 {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        if inclusive {
            if *self < 254 {
                //2^3-1-1
                vec![b'3', *self + 2]
            } else {
                let bytes = (*self as u16 + 3).to_be_bytes();
                vec![b'4', bytes[0], bytes[1]]
            }
        } else {
            vec![b'3', *self]
        }
    }
}
impl EncodeLength for u16 {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        if inclusive {
            if *self < 65533 {
                //2^4-1-2
                let bytes = (*self + 3).to_be_bytes();
                vec![b'4', bytes[0], bytes[1]]
            } else {
                let bytes = (*self as u32 + 5).to_be_bytes();
                vec![b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
            }
        } else {
            let bytes = self.to_be_bytes();
            vec![b'4', bytes[0], bytes[1]]
        }
    }
}
impl EncodeLength for u32 {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        if inclusive {
            if *self < 4294967293 {
                //2^5-1-4
                let bytes = (*self + 5).to_be_bytes();
                vec![b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
            } else {
                let bytes = (*self as u64 + 9).to_be_bytes();
                vec![
                    b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7],
                ]
            }
        } else {
            let bytes = self.to_be_bytes();
            vec![b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
        }
    }
}
impl EncodeLength for u64 {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        if inclusive {
            if *self < 18446744073709551613 {
                //2^6-1-8
                let bytes = (*self + 9).to_be_bytes();
                vec![
                    b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7],
                ]
            } else {
                let bytes = (*self as u128 + 17).to_be_bytes();
                vec![
                    b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
                    bytes[14], bytes[15],
                ]
            }
        } else {
            let bytes = self.to_be_bytes();
            vec![
                b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                bytes[7],
            ]
        }
    }
}
impl EncodeLength for u128 {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        if inclusive {
            let bytes = (*self as u128 + 17).to_be_bytes();
            vec![
                b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
                bytes[14], bytes[15],
            ]
        } else {
            let bytes = self.to_be_bytes();
            vec![
                b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
                bytes[14], bytes[15],
            ]
        }
    }
}
impl EncodeLength for usize {
    fn encode_length(&self, inclusive: bool) -> Vec<u8> {
        let mut flat = Vec::new();
        if inclusive {
            if *self < (std::u8::MAX / 2) as usize {
                flat.push(b'3'); // Indicate that length fits in one byte (2^n notation, 2^3=8 bits)
                flat.push((*self + 2) as u8);
            } else if *self < (std::u16::MAX / 2) as usize {
                flat.push(b'4'); // Indicate that length fits in two bytes (2^4=16 bits)
                flat.extend_from_slice(&(*self as u16 + 3).to_be_bytes());
            } else if *self < (std::u32::MAX / 2) as usize {
                flat.push(b'5'); // Indicate that length fits in four bytes (2^5=32 bits)
                flat.extend_from_slice(&(*self as u32 + 5).to_be_bytes());
            } else if *self < (std::u64::MAX / 2) as usize {
                flat.push(b'6'); // Indicate that length fits in eight bytes (2^6=64 bits)
                flat.extend_from_slice(&(*self as u64 + 9).to_be_bytes());
            } else {
                flat.push(b'7'); // Indicate that length fits in sixteen bytes (2^7=128 bits)
                flat.extend_from_slice(&(*self as u128 + 17).to_be_bytes());
            }
            flat
        } else {
            if *self < (std::u8::MAX / 2) as usize {
                flat.push(b'3'); // Indicate that length fits in one byte (2^n notation, 2^3=8 bits)
                flat.push(*self as u8);
            } else if *self < (std::u16::MAX / 2) as usize {
                flat.push(b'4'); // Indicate that length fits in two bytes (2^4=16 bits)
                flat.extend_from_slice(&(*self as u16).to_be_bytes());
            } else if *self < (std::u32::MAX / 2) as usize {
                flat.push(b'5'); // Indicate that length fits in four bytes (2^5=32 bits)
                flat.extend_from_slice(&(*self as u32).to_be_bytes());
            } else if *self < (std::u64::MAX / 2) as usize {
                flat.push(b'6'); // Indicate that length fits in eight bytes (2^6=64 bits)
                flat.extend_from_slice(&(*self as u64).to_be_bytes());
            } else {
                flat.push(b'7'); // Indicate that length fits in sixteen bytes (2^7=128 bits)
                flat.extend_from_slice(&(*self as u128).to_be_bytes());
            }
            flat
        }
    }
}

/// Represents spectral image data where pixel values range from black (0) to white (1),
/// with allowance for values beyond this range to accommodate noise and other factors.
/// The '5' in `VsfSpectralImage5` signifies the use of 32-bit floating-point numbers for storage (2^5 = 32 bits).
pub struct VsfSpectralImage5 {
    // Width of the image in pixels.
    width: usize,

    // Height of the image in pixels.
    height: usize,

    // Number of spectral channels.
    channels: usize,

    // Start wavelengths (in meters) for each channel.
    starts: Vec<f32>,

    // Stop wavelengths (in meters) for each channel.
    stops: Vec<f32>,

    // Spectral response curves for each channel.
    spectral: Vec<Vec<f32>>,

    // Image data stored as a flat array.
    image: Vec<f32>,

    // Aspect ratio of the image (width / height).
    aspect_ratio: f32,

    // Pixel resolution in width direction (pixels per meter), optional.
    width_resolution: Option<f32>,

    // Pixel resolution in height direction (pixels per meter), optional.
    height_resolution: Option<f32>,

    // Fill factor of the sensor, optional. Represents the percentage of each pixel's area that is sensitive to light.
    fill_factor: Option<f32>,

    // Specifies if the image data is interleaved by channel (true) or not.
    interleaved: bool,

    // Indicates if the image data is in row scan order (true, English reading order) or column scan order (false).
    row_scan: bool,
}