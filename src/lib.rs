pub mod vsf {
    /// A library for working with the Versatile Storage Format (VSF).
    ///
    /// Provides a Rust-ey set of types for representing various data formats,
    /// including integers, floating-point numbers, complex numbers, arrays, and special VSF-specific types.
    use num_complex::Complex;

    #[derive(Debug)]
    pub enum VsfType {
        // Unsigned Integer Types
        u(usize), // Unsigned integer, size is determined by the value
        u3(u8),   // Unsigned 8-bit integer, 2^n notation (2^3=8 bits)
        u4(u16),  // Unsigned 16-bit integer, 2^n notation (2^4=16 bits)
        u5(u32),  // Unsigned 32-bit integer, 2^n notation (2^5=32 bits)
        u6(u64),  // Unsigned 64-bit integer, 2^n notation (2^6=64 bits)
        u7(u128), // Unsigned 128-bit integer, 2^n notation (2^7=128 bits)

        // Signed Integer Types
        s(isize), // Signed integer, size is determined by the value
        s3(i8),   // Signed 8-bit integer
        s4(i16),  // Signed 16-bit integer
        s5(i32),  // Signed 32-bit integer
        s6(i64),  // Signed 64-bit integer
        s7(i128), // Signed 128-bit integer

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
        x(String),      // Unicode text

        // VSF-specific Types
        f(String),  // File type
        l(String),  // Label
        o(usize),   // Offset in bits
        b(usize),   // Length in bits
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
        pub fn flatten(&self) -> Result<Vec<u8>, std::io::Error> {
            match self {
                // Unsigned Integer Types
                VsfType::u(value) => {
                    let mut flat = vec![b'u'];
                    flat.extend_from_slice(&value.encode_number(false));
                    Ok(flat)
                }
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
                VsfType::s(value) => {
                    let mut flat = vec![b's'];
                    flat.extend_from_slice(&(*value as usize).encode_number(false));
                    Ok(flat)
                }
                VsfType::s3(value) => Ok(vec![b's', b'3', *value as u8]),
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
                    flat.extend_from_slice(&values.len().encode_number(false));
                    flat.push(b'u');
                    flat.push(b'3');
                    flat.extend_from_slice(values);
                    Ok(flat)
                }
                VsfType::au4(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                    flat.extend_from_slice(&values.len().encode_number(false));
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
                VsfType::g(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'g');
                    flat.extend_from_slice(&(value.len() * 8).encode_number(false));
                    flat.extend_from_slice(value);
                    Ok(flat)
                }

                // VSF specific types
                VsfType::z(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'z');
                    flat.extend_from_slice(&value.encode_number(false));
                    Ok(flat)
                }
                VsfType::y(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'y');
                    flat.extend_from_slice(&value.encode_number(false));
                    Ok(flat)
                }
                VsfType::b(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'b');
                    flat.extend_from_slice(&value.encode_number(false));
                    Ok(flat)
                }
                VsfType::o(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'o');
                    flat.extend_from_slice(&value.encode_number(false));
                    Ok(flat)
                }
                VsfType::l(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'l');
                    flat.extend_from_slice(&value.len().encode_number(false));
                    flat.extend_from_slice(value.as_bytes());
                    Ok(flat)
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Unsupported type for flattening!",
                )),
            }
        }
    }

    /// Encodes the length of a vector into a VSF-style byte vector. Automatically sizes usize, other datatypes are maintained in bit sizes.
    pub trait EncodeNumber {
        fn encode_number(&self, inclusive: bool) -> Vec<u8>;
    }
    impl EncodeNumber for u8 {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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
    impl EncodeNumber for u16 {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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
    impl EncodeNumber for u32 {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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
    impl EncodeNumber for u64 {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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
    impl EncodeNumber for u128 {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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
    impl EncodeNumber for usize {
        fn encode_number(&self, inclusive: bool) -> Vec<u8> {
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

    pub fn parse(data: &[u8], pointer: &mut usize) -> Result<VsfType, std::io::Error> {
        if *pointer >= data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Pointer out of bounds!",
            ));
        }

        let type_byte = data[*pointer];
        *pointer += 1;
        match type_byte {
            b'u' => {
                let size_byte = data[*pointer];
                *pointer += 1;
                match size_byte {
                    b'1' => {
                        let value = (data[*pointer] & 0b11000000) >> 6;
                        *pointer += 1;
                        Ok(VsfType::u(value as usize))
                    }
                    b'2' => {
                        let value = (data[*pointer] & 0b11110000) >> 4;
                        *pointer += 1;
                        Ok(VsfType::u(value as usize))
                    }
                    b'3' => {
                        let value = data[*pointer];
                        *pointer += 1;
                        Ok(VsfType::u3(value))
                    }
                    b'4' => {
                        let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
                        *pointer += 2;
                        Ok(VsfType::u4(value))
                    }
                    b'5' => {
                        let value = u32::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                        ]);
                        *pointer += 4;
                        Ok(VsfType::u5(value))
                    }
                    b'6' => {
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
                        Ok(VsfType::u6(value))
                    }
                    b'7' => {
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
                        ]);
                        *pointer += 16;
                        Ok(VsfType::u7(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Invalid unsigned integer type!",
                        ))
                    }
                }
            }
            b's' => {
                let size_byte = data[*pointer];
                *pointer += 1;
                match size_byte {
                    b'1' => {
                        let value = (data[*pointer] & 0b11000000) >> 6;
                        *pointer += 1;
                        Ok(VsfType::s(value as isize))
                    }
                    b'2' => {
                        let value = (data[*pointer] & 0b11110000) >> 4;
                        *pointer += 1;
                        Ok(VsfType::s(value as isize))
                    }
                    b'3' => {
                        let value = data[*pointer] as i8;
                        *pointer += 1;
                        Ok(VsfType::s3(value))
                    }
                    b'4' => {
                        let value = i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
                        *pointer += 2;
                        Ok(VsfType::s4(value))
                    }
                    b'5' => {
                        let value = i32::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                        ]);
                        *pointer += 4;
                        Ok(VsfType::s5(value))
                    }
                    b'6' => {
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
                        Ok(VsfType::s6(value))
                    }
                    b'7' => {
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
                        ]);
                        *pointer += 16;
                        Ok(VsfType::s7(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Invalid signed integer type!",
                        ))
                    }
                }
            }
            b'f' => {
                let size_byte = data[*pointer];
                *pointer += 1;
                match size_byte {
                    b'5' => {
                        let value = f32::from_bits(u32::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                        ]));
                        *pointer += 4;
                        Ok(VsfType::e5(value))
                    }
                    b'6' => {
                        let value = f64::from_bits(u64::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                            data[*pointer + 4],
                            data[*pointer + 5],
                            data[*pointer + 6],
                            data[*pointer + 7],
                        ]));
                        *pointer += 8;
                        Ok(VsfType::e6(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Invalid floating point type",
                        ))
                    }
                }
            }
            b'a' => {
                let length = decode_usize(data, pointer)?;
                let array_type = data[*pointer];
                *pointer += 1;
                match array_type {
                    b'u' => {
                        let element_size = data[*pointer];
                        *pointer += 1;
                        match element_size {
                            b'3' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    values.push(data[*pointer]);
                                    *pointer += 1;
                                }
                                Ok(VsfType::au3(values))
                            }
                            b'4' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value =
                                        u16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
                                    *pointer += 2;
                                    values.push(value);
                                }
                                Ok(VsfType::au4(values))
                            }
                            b'5' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value = u32::from_be_bytes([
                                        data[*pointer],
                                        data[*pointer + 1],
                                        data[*pointer + 2],
                                        data[*pointer + 3],
                                    ]);
                                    *pointer += 4;
                                    values.push(value);
                                }
                                Ok(VsfType::au5(values))
                            }
                            b'6' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
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
                                    values.push(value);
                                }
                                Ok(VsfType::au6(values))
                            }
                            b'7' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
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
                                    ]);
                                    *pointer += 16;
                                    values.push(value);
                                }
                                Ok(VsfType::au7(values))
                            }
                            _ => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::UnexpectedEof,
                                    "Invalid unsigned integer array type!",
                                ))
                            }
                        }
                    }
                    b's' => {
                        let element_size = data[*pointer];
                        *pointer += 1;
                        match element_size {
                            b'3' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    values.push(data[*pointer] as i8);
                                    *pointer += 1;
                                }
                                Ok(VsfType::as3(values))
                            }
                            b'4' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value =
                                        i16::from_be_bytes([data[*pointer], data[*pointer + 1]]);
                                    *pointer += 2;
                                    values.push(value);
                                }
                                Ok(VsfType::as4(values))
                            }
                            b'5' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value = i32::from_be_bytes([
                                        data[*pointer],
                                        data[*pointer + 1],
                                        data[*pointer + 2],
                                        data[*pointer + 3],
                                    ]);
                                    *pointer += 4;
                                    values.push(value);
                                }
                                Ok(VsfType::as5(values))
                            }
                            b'6' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
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
                                    values.push(value);
                                }
                                Ok(VsfType::as6(values))
                            }
                            b'7' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
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
                                    ]);
                                    *pointer += 16;
                                    values.push(value);
                                }
                                Ok(VsfType::as7(values))
                            }
                            _ => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::UnexpectedEof,
                                    "Invalid signed integer type!",
                                ))
                            }
                        }
                    }
                    b'f' => {
                        let element_size = data[*pointer];
                        *pointer += 1;
                        match element_size {
                            b'5' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value = f32::from_bits(u32::from_be_bytes([
                                        data[*pointer],
                                        data[*pointer + 1],
                                        data[*pointer + 2],
                                        data[*pointer + 3],
                                    ]));
                                    *pointer += 4;
                                    values.push(value);
                                }
                                Ok(VsfType::af5(values))
                            }
                            b'6' => {
                                let mut values = Vec::with_capacity(length);
                                for _ in 0..length {
                                    let value = f64::from_bits(u64::from_be_bytes([
                                        data[*pointer],
                                        data[*pointer + 1],
                                        data[*pointer + 2],
                                        data[*pointer + 3],
                                        data[*pointer + 4],
                                        data[*pointer + 5],
                                        data[*pointer + 6],
                                        data[*pointer + 7],
                                    ]));
                                    *pointer += 8;
                                    values.push(value);
                                }
                                Ok(VsfType::af6(values))
                            }
                            _ => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::UnexpectedEof,
                                    "Invalid floating point array type!",
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Invalid array type",
                        ))
                    }
                }
            }
            b'i' => {
                let element_size = data[*pointer];
                *pointer += 1;
                match element_size {
                    b'6' => {
                        let re = f32::from_bits(u32::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                        ]));
                        *pointer += 4;
                        let im = f32::from_bits(u32::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                        ]));
                        *pointer += 4;
                        Ok(VsfType::i6(Complex { re, im }))
                    }
                    b'7' => {
                        let re = f64::from_bits(u64::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                            data[*pointer + 4],
                            data[*pointer + 5],
                            data[*pointer + 6],
                            data[*pointer + 7],
                        ]));
                        *pointer += 8;
                        let im = f64::from_bits(u64::from_be_bytes([
                            data[*pointer],
                            data[*pointer + 1],
                            data[*pointer + 2],
                            data[*pointer + 3],
                            data[*pointer + 4],
                            data[*pointer + 5],
                            data[*pointer + 6],
                            data[*pointer + 7],
                        ]));
                        *pointer += 8;
                        Ok(VsfType::i7(Complex { re, im }))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Invalid complex number type!",
                        ))
                    }
                }
            }
            b'x' => {
                let mut length = decode_usize(data, pointer)?;
                let value = String::from_utf8(data[*pointer..*pointer + length].to_vec()).map_err(
                    |_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid UTF-8 string!",
                        )
                    },
                )?;
                *pointer += length;
                Ok(VsfType::x(value))
            }
            b'z' => {
                let version = decode_usize(data, pointer)?;
                Ok(VsfType::z(version))
            }
            b'y' => {
                let backward_version = decode_usize(data, pointer)?;
                Ok(VsfType::y(backward_version))
            }
            b'l' => {
                let length = decode_usize(data, pointer)?;
                let value = String::from_utf8(data[*pointer..*pointer + length].to_vec()).map_err(
                    |_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Label is not valid UTF-8!",
                        )
                    },
                )?;
                *pointer += length;
                Ok(VsfType::l(value))
            }
            b'o' => {
                let offset = decode_usize(data, pointer)?;
                Ok(VsfType::o(offset))
            }
            b'b' => {
                let length = decode_usize(data, pointer)?;
                Ok(VsfType::b(length))
            }
            b'g' => {
                let mut signature_length = decode_usize(data, pointer)?;
                if signature_length % 8 != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Signature length does not land on a byte boundary!",
                    ));
                }
                signature_length /= 8;
                let value = data[*pointer..*pointer + signature_length].to_vec();
                *pointer += signature_length;
                Ok(VsfType::g(value))
            }
            b'h' => {
                let mut hash_length = decode_usize(data, pointer)?;
                if hash_length % 8 != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Hash length does not land on a byte boundary!",
                    ));
                }
                hash_length /= 8;
                let value = data[*pointer..*pointer + hash_length].to_vec();
                *pointer += hash_length;
                Ok(VsfType::h(value))
            }

            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Invalid type identifier",
                ))
            }
        }
    }
    fn decode_usize(data: &[u8], pointer: &mut usize) -> Result<usize, std::io::Error> {
        match data[*pointer] {
            b'1' => {
                *pointer += 1;
                let value = (data[*pointer] & 0b11000000) >> 6;
                *pointer += 1;
                Ok(value as usize)
            }
            b'2' => {
                *pointer += 1;
                let value = (data[*pointer] & 0b11110000) >> 4;
                *pointer += 1;
                Ok(value as usize)
            }
            b'3' => {
                *pointer += 1;
                let value = data[*pointer] as usize;
                *pointer += 1;
                Ok(value)
            }
            b'4' => {
                *pointer += 1;
                let value = u16::from_be_bytes([data[*pointer], data[*pointer + 1]]) as usize;
                *pointer += 2;
                Ok(value)
            }
            b'5' => {
                *pointer += 1;
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
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid usize encoding!",
            )),
        }
    }
}
