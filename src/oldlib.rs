/// VSF (Versatile Storage Format) Module
///
/// This module provides types and functions for working with the Versatile Storage Format (VSF),
/// a flexible binary format designed for efficient storage and retrieval of various data types.
/// It is part of the larger TOKEN system (in development).
///
/// # Features
///
/// - Support for a wide range of data types, including integers, floating-point numbers,
///   complex numbers, and arrays.
/// - Compact binary representation with variable-length encoding for efficient storage.
/// - Extensible format with version tracking for backward compatibility.
/// - Support for labels, offsets, and other metadata to describe the stored data.
///
/// # Main Types
///
/// - `VsfType`: An enum representing all supported VSF data types.
/// - `EncodeNumber`: A trait for encoding numbers into the VSF format.
///
/// # Key Functions
///
/// - `parse`: Parses VSF-encoded data into Rust types.
/// - `flatten`: Converts Rust types into VSF-encoded data.
///
/// # Usage
///
/// Typical usage involves building a vector of VSF types and then flattening them:
///
/// 1. Create a vector to hold VSF data.
/// 2. Push `VsfType` instances onto this vector to represent your data.
/// 3. Use the `flatten` method on each `VsfType` to convert it into VSF-encoded byte vectors.
/// 4. Combine these byte vectors to create the final VSF structure.
///
/// # Example
///
/// ```
/// use vsf::{VsfType, parse, EncodeNumber};
///
/// fn main() -> () {
///     let mut vsf_vector = Vec::new();
///     
///     // Add VSF header
///     vsf_vector.push(b"R\xC3\x85<".to_vec());
///     
///     // Add version information
///     vsf_vector.push(VsfType::z(1).flatten().unwrap());
///     vsf_vector.push(VsfType::y(1).flatten().unwrap());
///     
///     // Add data type and other metadata
///     vsf_vector.push(b"(".to_vec());
///     vsf_vector.push(VsfType::d("example data".to_owned()).flatten().unwrap());
///     vsf_vector.push(VsfType::o(128).flatten().unwrap());
///     vsf_vector.push(VsfType::b(64).flatten().unwrap());
///     vsf_vector.push(VsfType::c(1).flatten().unwrap());
///     vsf_vector.push(b")>".to_vec());
///     
///     // Add actual data
///     vsf_vector.push(VsfType::u5(42).flatten().unwrap());
///     
///     // Combine all parts into a single VSF byte vector
///     let vsf_data: Vec<u8> = vsf_vector.into_iter().flatten().collect();
/// }
/// ```
///
/// # Note
///
/// This module is part of the TOKEN system, which is currently a work in progress.
/// The API and usage patterns may evolve as the system develops.
///
pub mod vsf {

    use num_complex::Complex;

    #[allow(non_camel_case_types)]
    use chrono::{DateTime, Duration, TimeZone, Utc};

    /// EagleTime represents a point in time in the Eagle Time standard.
    /// It stores the number of seconds since the Eagle lunar landing.
    #[derive(Debug, Clone)]
    pub struct EagleTime {
        et_seconds: EtType,
    }

    impl EagleTime {
        /// Creates a new EagleTime instance from a VsfType.
        pub fn new(value: VsfType) -> Self {
            let et_seconds = match value {
                VsfType::f6(v) => EtType::f6(v),
                VsfType::u(v, false) => EtType::u(v),
                VsfType::u5(v) => EtType::u5(v),
                VsfType::u6(v) => EtType::u6(v),
                VsfType::u7(v) => EtType::u7(v),
                VsfType::s(v) => EtType::s(v),
                VsfType::s5(v) => EtType::s5(v),
                VsfType::s6(v) => EtType::s6(v),
                VsfType::s7(v) => EtType::s7(v),
                _ => panic!("EagleTime must be created with a EtType variant"),
            };
            EagleTime { et_seconds }
        }

        /// Converts the current EagleTime to a VsfType.
        pub fn to_vsf_type(&self) -> VsfType {
            match self.et_seconds {
                EtType::f6(v) => VsfType::f6(v),
                EtType::u(v) => VsfType::u(v, false),
                EtType::u5(v) => VsfType::u5(v),
                EtType::u6(v) => VsfType::u6(v),
                EtType::u7(v) => VsfType::u7(v),
                EtType::s(v) => VsfType::s(v),
                EtType::s5(v) => VsfType::s5(v),
                EtType::s6(v) => VsfType::s6(v),
                EtType::s7(v) => VsfType::s7(v),
                _ => panic!("Unexpected EtType variant"),
            }
        }

        /// Converts the EagleTime to a UTC DateTime.
        pub fn to_datetime(&self) -> DateTime<Utc> {
            let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
            let duration: Duration = match self.et_seconds {
                EtType::f6(v) => Duration::from_std(std::time::Duration::from_secs_f64(v))
                    .unwrap_or_else(|_| panic!("Invalid duration")),
                EtType::u(v) => Duration::seconds(v as i64),
                EtType::u5(v) => Duration::seconds(v as i64),
                EtType::u6(v) => Duration::seconds(v as i64),
                EtType::u7(v) => Duration::seconds(v as i64),
                EtType::s(v) => Duration::seconds(v as i64),
                EtType::s5(v) => Duration::seconds(v as i64),
                EtType::s6(v) => Duration::seconds(v),
                EtType::s7(v) => Duration::seconds(v as i64),
                _ => panic!("Unexpected EtType variant"),
            };
            eagle_epoch + duration
        }
    }

    /// Converts a UTC DateTime to Eagle Time
    pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
        let eagle = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap(); // Lunar landing
        let seconds_since_landing = dt - eagle;
        let et_seconds = seconds_since_landing.num_seconds() as f64;
        EagleTime::new(VsfType::f6(et_seconds))
    }

    #[allow(non_camel_case_types)]
    pub enum VsfType {
        // Unsigned Integer Types
        u(usize, bool), // (number, inclusive) Unsigned integer, size is determined by the value
        u3(u8),         // Unsigned 8-bit integer, 2^n notation (2^3=8 bits)
        u4(u16),        // Unsigned 16-bit integer, 2^n notation (2^4=16 bits)
        u5(u32),        // Unsigned 32-bit integer, 2^n notation (2^5=32 bits)
        u6(u64),        // Unsigned 64-bit integer, 2^n notation (2^6=64 bits)
        u7(u128),       // Unsigned 128-bit integer, 2^n notation (2^7=128 bits)

        // Signed Integer Types
        s(isize), // Signed integer, size is determined by the value
        s3(i8),   // Signed 8-bit integer
        s4(i16),  // Signed 16-bit integer
        s5(i32),  // Signed 32-bit integer
        s6(i64),  // Signed 64-bit integer
        s7(i128), // Signed 128-bit integer

        // IEEE 754 Floating-point Types
        f5(f32), // 32-bit floating point, 2^n notation, n is always bit count
        f6(f64), // 64-bit floating point

        // Unsigned Integer Arrays
        au_(Vec<usize>), // Array of Unsigned integer, size is determined by the largest value
        au3(Vec<u8>),    // Array of Unsigned 8-bit integer
        au4(Vec<u16>),   // Array of Unsigned 16-bit integer
        au5(Vec<u32>),   // Array of Unsigned 32-bit integer
        au6(Vec<u64>),   // Array of Unsigned 64-bit integer
        au7(Vec<u128>),  // Array of Unsigned 128-bit integer

        // Signed Integer Arrays
        as_(Vec<isize>), // Array of Signed integer, size is determined by the value with the largest magnitude
        as3(Vec<i8>),    // Array of Signed 8-bit integer
        as4(Vec<i16>),   // Array of Signed 16-bit integer
        as5(Vec<i32>),   // Array of Signed 32-bit integer
        as6(Vec<i64>),   // Array of Signed 64-bit integer
        as7(Vec<i128>),  // Array of Signed 128-bit integer

        // Floating-point Arrays
        af5(Vec<f32>), // Array of 32-bit floating point
        af6(Vec<f64>), // Array of 64-bit floating point

        // Complex Numbers
        i6(Complex<f32>),       // Single complex number with f32 components
        i7(Complex<f64>),       // Single complex number with f64 components
        ai6(Vec<Complex<f32>>), // Array of complex numbers with f32 components
        ai7(Vec<Complex<f64>>), // Array of complex numbers with f64 components

        // Special Types
        u0(bool),       // Boolean, stored 8 bit aligned, recomend filling all 8 bits.
        au0(Vec<bool>), // Array of Boolean, extra bits are filled with 0 to align to 8 bits
        x(String),      // Unicode text
        et(EtType),     // Eagle Time

        // VSF-specific Types
        d(String),  // Data type
        l(String),  // Label
        o(usize),   // Offset in bits
        b(usize),   // Length in bits
        c(usize),   // Label count
        z(usize),   // Version
        y(usize),   // Backward version
        m(usize),   // Marker definition
        r(usize),   // Marker
        h(Vec<u8>), // Hash
        g(Vec<u8>), // Signature
    }

    #[derive(Debug, Clone)]
    #[allow(non_camel_case_types)]
    pub enum EtType {
        u(usize),
        u5(u32),
        u6(u64),
        u7(u128),
        s(isize),
        s5(i32),
        s6(i64),
        s7(i128),
        f5(f32),
        f6(f64),
    }

    #[derive(Debug, Clone)]
    #[allow(non_camel_case_types)]
    pub enum NumericType {
        u(usize),
        u3(u8),
        u4(u16),
        u5(u32),
        u6(u64),
        u7(u128),
        s(isize),
        s3(i8),
        s4(i16),
        s5(i32),
        s6(i64),
        s7(i128),
        f5(f32),
        f6(f64),
        i6(Complex<f32>),
        i7(Complex<f64>),
    }

    #[derive(Debug, Clone)]
    #[allow(non_camel_case_types)]
    pub enum ArrayType {
        au_(Vec<usize>),
        au3(Vec<u8>),
        au4(Vec<u16>),
        au5(Vec<u32>),
        au6(Vec<u64>),
        au7(Vec<u128>),
        as_(Vec<isize>),
        as3(Vec<i8>),
        as4(Vec<i16>),
        as5(Vec<i32>),
        as6(Vec<i64>),
        as7(Vec<i128>),
        af5(Vec<f32>),
        af6(Vec<f64>),
        ai6(Vec<Complex<f32>>),
        ai7(Vec<Complex<f64>>),
    }

    impl VsfType {
        pub fn flatten(&self) -> Vec<u8> {
            match self {
                // Unsigned Integer Types
                VsfType::u0(value) => {
                    let mut flat = vec![b'u'];
                    if *value {
                        flat.push(255);
                    } else {
                        flat.push(0);
                    }
                    flat
                }
                VsfType::u(value, inclusive) => {
                    let mut flat = Vec::new();
                    if *inclusive {
                        flat.extend_from_slice(&value.encode_usize_inclusive());
                    } else {
                        flat.push(b'u');
                        flat.extend_from_slice(&value.encode_number());
                    }
                    flat
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

                // Signed Integer Types
                VsfType::s(value) => {
                    let mut flat = vec![b's'];
                    flat.extend_from_slice(&value.encode_number());
                    flat
                }
                VsfType::s3(value) => {
                    let mut flat = vec![b's'];

                    flat
                }
                VsfType::s4(value) => {
                    let bytes = value.to_be_bytes();
                    vec![b's', b'4', bytes[0], bytes[1]]
                }
                VsfType::s5(value) => {
                    let bytes = value.to_be_bytes();
                    vec![b's', b'5', bytes[0], bytes[1], bytes[2], bytes[3]]
                }
                VsfType::s6(value) => {
                    let bytes = value.to_be_bytes();
                    vec![
                        b's', b'6', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                        bytes[6], bytes[7],
                    ]
                }
                VsfType::s7(value) => {
                    let bytes = value.to_be_bytes();
                    vec![
                        b's', b'7', bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
                        bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
                        bytes[13], bytes[14], bytes[15],
                    ]
                }

                // Floating-point Types
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

                // Unsigned Integer Vectors
                VsfType::au3(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'3');
                    flat.extend_from_slice(values);
                    flat
                }
                VsfType::au4(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'4');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::au5(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'5');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::au6(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'6');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::au7(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'u');
                    flat.push(b'7');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }

                // Signed Integer Vectors
                VsfType::as3(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b's');
                    flat.push(b'3');
                    for value in values {
                        flat.push(*value as u8);
                    }
                    flat
                }
                VsfType::as4(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b's');
                    flat.push(b'4');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::as5(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b's');
                    flat.push(b'5');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::as6(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b's');
                    flat.push(b'6');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::as7(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b's');
                    flat.push(b'7');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }

                // Floating-point Vectors
                VsfType::af5(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'f');
                    flat.push(b'5');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::af6(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'f');
                    flat.push(b'6');
                    for value in values {
                        let bytes = value.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
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
                    flat
                }
                VsfType::i7(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'i');
                    flat.push(b'7');
                    let bytes = value.re.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                    let bytes = value.im.to_be_bytes();
                    flat.extend_from_slice(&bytes);
                    flat
                }

                // Complex Number Vectors
                VsfType::ai6(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'6');
                    for value in values {
                        let bytes = value.re.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                        let bytes = value.im.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }
                VsfType::ai7(values) => {
                    let mut flat = Vec::new();
                    flat.push(b'a');
                    flat.extend_from_slice(&values.len().encode_number());
                    flat.push(b'i');
                    flat.push(b'7');
                    for value in values {
                        let bytes = value.re.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                        let bytes = value.im.to_be_bytes();
                        flat.extend_from_slice(&bytes);
                    }
                    flat
                }

                // Unicode text
                VsfType::x(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'x');
                    flat.extend_from_slice(&value.len().encode_number());
                    flat.extend_from_slice(value.as_bytes());
                    flat
                }

                // Signature
                VsfType::g(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'g');
                    flat.extend_from_slice(&(value.len() * 8).encode_number());
                    flat.extend_from_slice(value);
                    flat
                }

                // Eagle Time
                VsfType::et(value) => {
                    let mut flat = Vec::new();
                    match value {
                        EtType::u(value) => {
                            flat.push(b'T');
                            flat.push(b'6');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::u5(value) => {
                            flat.push(b'T');
                            flat.push(b'5');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::u6(value) => {
                            flat.push(b'T');
                            flat.push(b'6');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::u7(value) => {
                            flat.push(b'T');
                            flat.push(b'7');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::s(value) => {
                            flat.push(b'T');
                            flat.push(b'6');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::s5(value) => {
                            flat.push(b'T');
                            flat.push(b'5');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::s6(value) => {
                            flat.push(b'T');
                            flat.push(b'6');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::s7(value) => {
                            flat.push(b'T');
                            flat.push(b'7');
                            flat.extend_from_slice(&value.encode_number());
                        }
                        EtType::f6(value) => {
                            flat.push(b't');
                            flat.push(b'6');
                            flat.extend_from_slice(&value.to_be_bytes());
                        }
                        _ => (),
                    }
                    flat
                }

                // VSF specific types
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
                VsfType::b(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'b');
                    flat.extend_from_slice(&value.encode_number());
                    flat
                }
                VsfType::o(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'o');
                    flat.extend_from_slice(&value.encode_number());
                    flat
                }
                VsfType::l(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'l');
                    flat.extend_from_slice(&value.len().encode_number());
                    flat.extend_from_slice(value.as_bytes());
                    flat
                }
                VsfType::d(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'd');
                    flat.extend_from_slice(&value.len().encode_number());
                    flat.extend_from_slice(value.as_bytes());
                    flat
                }
                VsfType::c(value) => {
                    let mut flat = Vec::new();
                    flat.push(b'c');
                    flat.extend_from_slice(&value.encode_number());
                    flat
                }
                _ => Vec::new(),
            }
        }
    }

    trait EncodeNumber {
        fn encode_number(&self) -> Vec<u8>;
    }
    trait EncodeNumberInclusive {
        fn encode_usize_inclusive(&self) -> Vec<u8>;
    }

    impl EncodeNumber for u8 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'3');
            result.push(*self);
            result
        }
    }
    impl EncodeNumber for u16 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'4');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for u32 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'5');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for u64 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'6');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for u128 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'7');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for usize {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            if *self <= std::u8::MAX as usize {
                result.push(b'3');
                result.push(*self as u8);
            } else if *self <= std::u16::MAX as usize {
                result.push(b'4');
                result.extend_from_slice(&(*self as u16).to_be_bytes());
            } else if *self <= std::u32::MAX as usize {
                result.push(b'5');
                result.extend_from_slice(&(*self as u32).to_be_bytes());
            } else if *self <= std::u64::MAX as usize {
                result.push(b'6');
                result.extend_from_slice(&(*self as u64).to_be_bytes());
            } else {
                result.push(b'7');
                result.extend_from_slice(&(*self as u128).to_be_bytes());
            }
            result
        }
    }
    impl EncodeNumberInclusive for usize {
        fn encode_usize_inclusive(&self) -> Vec<u8> {
            let mut base = 8;
            let mut size = 8;
            let mut result = vec![b'u'];
            let mut adder = base + size;
            let mut cutoff = 0x80 - adder + 0x80;
            if *self < cutoff {
                result.push(b'3');
                result.push((*self + adder) as u8);
                return result;
            }
            size = size * 2;
            adder = base + size;
            cutoff = std::u8::MAX as usize - adder;
            if *self <= cutoff {
                result.push(b'4');
                result.extend_from_slice(&(*self + adder).to_be_bytes());
                return result;
            }
            size = size * 2;
            adder = base + size;
            cutoff = std::u16::MAX as usize - adder;
            if *self <= cutoff {
                result.push(b'5');
                result.extend_from_slice(&(*self + adder).to_be_bytes());
                return result;
            }
            size = size * 2;
            adder = base + size;
            cutoff = std::u32::MAX as usize - adder;
            if *self <= cutoff {
                result.push(b'6');
                result.extend_from_slice(&(*self + adder).to_be_bytes());
                return result;
            }
            size = size * 2;
            adder = base + size;
            cutoff = std::u64::MAX as usize - adder;
            if *self <= cutoff {
                result.push(b'7');
                result.extend_from_slice(&(*self + adder).to_be_bytes());
                return result;
            }
            size = size * 2;
            adder = base + size;
            cutoff = std::u128::MAX as usize - adder;
            if *self <= cutoff {
                result.push(b'8');
                result.extend_from_slice(&(*self + adder).to_be_bytes());
                return result;
            }
            panic!("u8 and larger currently not implemented!");
        }
    }
    impl EncodeNumber for i8 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'3');
            result.push(*self as u8);
            result
        }
    }
    impl EncodeNumber for i16 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'4');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for i32 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'5');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for i64 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'6');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for i128 {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            result.push(b'7');
            result.extend_from_slice(&self.to_be_bytes());
            result
        }
    }
    impl EncodeNumber for isize {
        fn encode_number(&self) -> Vec<u8> {
            let mut result = Vec::new();
            if *self >= 0 {
                if *self <= std::u8::MAX as isize {
                    result.push(b'3');
                    result.extend_from_slice(&(*self).to_be_bytes());
                } else if *self <= std::u16::MAX as isize {
                    result.push(b'4');
                    result.extend_from_slice(&(*self).to_be_bytes());
                } else if *self <= std::u32::MAX as isize {
                    result.push(b'5');
                    result.extend_from_slice(&(*self).to_be_bytes());
                } else if *self <= std::u64::MAX as isize {
                    result.push(b'6');
                    result.extend_from_slice(&(*self as u64).to_be_bytes());
                } else {
                    result.push(b'7');
                    result.extend_from_slice(&(*self as u128).to_be_bytes());
                }
            } else {
                if *self >= std::i8::MIN as isize {
                    result.push(b'3');
                    result.extend_from_slice(&(*self as i8).to_be_bytes());
                } else if *self >= std::i16::MIN as isize {
                    result.push(b'4');
                    result.extend_from_slice(&(*self as i16).to_be_bytes());
                } else if *self >= std::i32::MIN as isize {
                    result.push(b'5');
                    result.extend_from_slice(&(*self as i32).to_be_bytes());
                } else if *self >= std::i64::MIN as isize {
                    result.push(b'6');
                    result.extend_from_slice(&(*self as i64).to_be_bytes());
                } else {
                    result.push(b'7');
                    result.extend_from_slice(&(*self as i128).to_be_bytes());
                }
            }
            result
        }
    }

    pub fn parse(data: &[u8], pointer: &mut usize) -> Result<VsfType, std::io::Error> {
        if *pointer >= data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
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
                    0 => Ok(VsfType::u0(false)),
                    255 => Ok(VsfType::u0(true)),
                    b'0' => {
                        let value = data[*pointer];
                        *pointer += 1;
                        match value {
                            0 => Ok(VsfType::u0(false)),
                            255 => Ok(VsfType::u0(true)),
                            _ => Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Invalid boolean value!",
                            )),
                        }
                    }
                    b'1' => {
                        let value = (data[*pointer] & 0b11000000) >> 6;
                        *pointer += 1;
                        Ok(VsfType::u(value as usize, false))
                    }
                    b'2' => {
                        let value = (data[*pointer] & 0b11110000) >> 4;
                        *pointer += 1;
                        Ok(VsfType::u(value as usize, false))
                    }
                    b'3' => {
                        let value = data[*pointer];
                        *pointer += 1;
                        Ok(VsfType::u3(value))
                    }
                    b'4' => {
                        let value =
                            u16::from_be_bytes(data[*pointer..*pointer + 2].try_into().unwrap());
                        *pointer += 2;
                        Ok(VsfType::u4(value))
                    }
                    b'5' => {
                        let value =
                            u32::from_be_bytes(data[*pointer..*pointer + 4].try_into().unwrap());
                        *pointer += 4;
                        Ok(VsfType::u5(value))
                    }
                    b'6' => {
                        let value =
                            u64::from_be_bytes(data[*pointer..*pointer + 8].try_into().unwrap());
                        *pointer += 8;
                        Ok(VsfType::u6(value))
                    }
                    b'7' => {
                        let value =
                            u128::from_be_bytes(data[*pointer..*pointer + 16].try_into().unwrap());
                        *pointer += 16;
                        Ok(VsfType::u7(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
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
                        let value =
                            i16::from_be_bytes(data[*pointer..*pointer + 2].try_into().unwrap());
                        *pointer += 2;
                        Ok(VsfType::s4(value))
                    }
                    b'5' => {
                        let value =
                            i32::from_be_bytes(data[*pointer..*pointer + 4].try_into().unwrap());
                        *pointer += 4;
                        Ok(VsfType::s5(value))
                    }
                    b'6' => {
                        let value =
                            i64::from_be_bytes(data[*pointer..*pointer + 8].try_into().unwrap());
                        *pointer += 8;
                        Ok(VsfType::s6(value))
                    }
                    b'7' => {
                        let value =
                            i128::from_be_bytes(data[*pointer..*pointer + 16].try_into().unwrap());
                        *pointer += 16;
                        Ok(VsfType::s7(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
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
                        Ok(VsfType::f5(value))
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
                        Ok(VsfType::f6(value))
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
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
                                    std::io::ErrorKind::Other,
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
                                    std::io::ErrorKind::Other,
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
                                    std::io::ErrorKind::Other,
                                    "Invalid floating point array type!",
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
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
                            std::io::ErrorKind::Other,
                            "Invalid complex number type!",
                        ))
                    }
                }
            }
            b'x' => {
                let length = decode_usize(data, pointer)?;
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
            b'c' => {
                let count = decode_usize(data, pointer)?;
                Ok(VsfType::c(count))
            }
            b'd' => {
                let length = decode_usize(data, pointer)?;
                let value = String::from_utf8(data[*pointer..*pointer + length].to_vec()).map_err(
                    |_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid data name!"),
                )?;
                *pointer += length;
                Ok(VsfType::d(value))
            }

            b'g' => {
                let mut signature_length = decode_usize(data, pointer)?;
                if signature_length % 8 != 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
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
                        std::io::ErrorKind::Other,
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
                    std::io::ErrorKind::Other,
                    format!("Invalid type identifier '{}'", type_byte as char),
                ))
            }
        }
    }
    // Update the make_header function to use the new VsfType::u
    pub fn make_header(
        version: usize,
        backward_version: usize,
        file_type: String,
        main_label_length_in_bits: usize,
        main_label_count: usize,
        ai_fingerprint: Vec<u8>,
        owner_public_address: [u8; 32],
        hardware_public_address: [u8; 32],
        timestamp: u64,
        thumbnail_data: Option<Vec<u8>>,
    ) -> Vec<u8> {
        // Magic Number
        let mut header = vec!["RÅ".as_bytes().to_owned()];
        header[0].extend_from_slice(&VsfType::z(version).flatten());
        header[0].extend_from_slice(&VsfType::y(backward_version).flatten());
        header[0].extend_from_slice(&VsfType::d(file_type).flatten());
        header[0].extend_from_slice(&VsfType::b(main_label_length_in_bits).flatten());
        header[0].extend_from_slice(&VsfType::c(main_label_count).flatten());
        if let Some(fingerprint) = ai_fingerprint {
            header[0].extend_from_slice(&VsfType::f(fingerprint).flatten());
        }
        header[0].extend_from_slice(&VsfType::p(owner_public_address).flatten());
        header[0].extend_from_slice(&VsfType::e(hardware_public_address).flatten());
        header[0].extend_from_slice(&VsfType::t(timestamp).flatten());

        if let Some(thumb) = thumbnail_data {
            header[0].extend_from_slice(&VsfType::x("thumbnail".to_string()).flatten());
            header[0].extend_from_slice(&VsfType::b(thumb.len() * 8).flatten());
            header[0].extend_from_slice(&thumb);
        }

        header[0].extend_from_slice(b">");

        let mut flat = Vec::new();
        for line in header {
            flat.extend_from_slice(&line);
        }
        flat
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
