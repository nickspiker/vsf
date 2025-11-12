//! # VSF (Versatile Storage Format)
//!
//! Self-describing binary format with hierarchical structure, strong typing, and cryptographic primitives.
//!
//! ## Features
//!
//! - **Self-describing**: Type markers embedded in the data stream
//! - **Hierarchical**: Offset-based seeking, unlimited nesting depth
//! - **Strongly-typed**: Primitives (u0-u7, i3-i7, f32, f64, complex), tensors, Spirix Scalars and Circles
//! - **Cryptographic**: Built-in BLAKE3 hashing and Ed25519 signing
//! - **Eagle Time**: universal timestamps
//! - **Huffman text compression**: ~2× compression over UTF-8 for strings
//!
//! ## Core Type System
//!
//! ### Primitives
//! - **Integers**: `u0`-`u7` (unsigned), `i3`-`i7` (signed)
//! - **IEEE Floats**: `f5` (f32), `f6` (f64)
//! - **IEEE Complex**: `j5` (Complex<f32>), `j6` (Complex<f64>)
//! - **Spirix**: `s33`-`s77` (Scalar), `c33`-`c77` (Circle)
//!
//! ### Tensors
//! - **Contiguous** (`t`): Row-major multi-dimensional arrays (1D-4D)
//! - **Strided** (`q`): Non-contiguous views with explicit stride
//!
//! ### Metadata and Labels
//!
//! VSF uses labels within sections for metadata:
//!
//! - `l`: Label text - identifies a field within a section (e.g., "shutter_speed", "author")
//! - Section fields can contain multiple values: `(label:value1,value2,value3)`
//! - Sections can contain hierarchical fields: `[dImaging (lshutter_speed:f6{0.01})(laperture:f5{2.8})]`
//!
//! **Other Metadata Types:**
//! - `x`: Unicode text strings
//! - `e`: Eagle Time (seconds since lunar landing)
//! - `d`: Data type identifier
//! - `o`: Byte offsets
//! - `b`: Byte lengths
//! - `n`: Counts
//! - `g`: Cryptographic signatures
//! - `h`: Cryptographic hashes
//!
//! ## File Structure
//!
//! VSF files follow a hierarchical structure:
//!
//! ```text
//! RÅ<                                  Magic number + header start
//!   b?{header_length}                  Header size
//!   z?{version}                        Format version
//!   y?{backward_version}               Backward compatibility version
//!   ef5{creation_time}                 Eagle Time creation timestamp (f32, ~2min precision)
//!   hp{31}{provenance_hash}            Provenance: BLAKE3 hash of content (required, always 32 bytes)
//!   ge{64}{signature}                  Ed25519 signature over hp (optional)
//!   hb{31}{rolling_hash}               Rolling: BLAKE3 of current state with History (optional)
//!   k?{key}                            File-level encryption key (optional)
//!   n?{label_count}                    Number of label records
//!   (dRAW:h?{hash},g?{sig},k?{key},o?{offset},b?{size},n?{count})     Label record
//!   (dThumbnail:h?{hash},g?{sig},k?{key},o?{offset},b?{size},n?{count})
//!   (dHistory:h?{hash},o?{offset},b?{size},n?{count})                 History tracking
//!   ...
//! >                                    Header end
//!
//! [dRAW (section_fields...)]           Section data at offset
//! [dThumbnail (section_fields...)]
//! [dHistory (history_entries...)]
//! ```
//!
//! **Hash Strategy (Always BLAKE3):**
//! - **hp** (hash provenance): Content identity - BLAKE3 hash of immutable content. Required.
//!   Computed with hp field as zeros, then filled in. Creates stable identifier for original content.
//! - **ge** (signature): Optional Ed25519 signature over hp. Signs the computed hash to verify creator.
//! - **hb** (hash rolling): Current file state - Optional BLAKE3 hash including History section.
//!   Updates when History updates. Useful for tracking mutable file evolution.
//!
//! **Provenance Verification:**
//! To verify a file's provenance, zero the hp field and compute BLAKE3 - it should match the stored hp.
//! If present, verify the ge signature against hp to authenticate the creator.
//!
//! **Terminology:**
//! - **Header**: Everything between `RÅ<` and `>`
//! - **Header field**: Individual entries like `b?{length}`, `z?{version}`
//! - **Label record**: The `(d"name":h,g,k,o,b,n)` entries that point to sections
//! - **Section**: Actual data blocks after the header, located at specified offsets
//! - **Section field**: Individual `(field)` entries within a section's data
//! - **`?` and `{}`**: `?` indicates length (ASCII 0-Z), `{}` indicates binary data
//!
//! The `:` and `,` separators in label records make the format human-readable in hex editors
//! and aid in forensics and corruption analysis with minimal overhead.
//!
//! ## Section Flattening Example
//!
//! A section with hierarchical fields for camera metadata:
//!
//! ```text
//! [dImaging
//!   (lshutter_speed:f6{0.01})      // 1/100s as f64
//!   (laperture:f5{2.8})            // f/2.8 as f32
//!   (liso:u4{400})                 // ISO 400
//! ]
//! ```
//!
//! Which flattens to:
//!
//! ```text
//! '[' + 'd' + '3' + {7u8} + "Imaging" +
//! '(' + 'l' + '3' + {13u8} + "shutter_speed" + ':' + 'f' + '6' + {0.01f64} + ')' +
//! '(' + 'l' + '3' + {8u8} + "aperture"      + ':' + 'f' + '5' + {2.8f32} + ')' +
//! '(' + 'l' + '3' + {3u8} + "iso" + ':' + 'u' + '4'+ {400u16} + ')' + ']'
//! ```
//!
//! Where 'char' indicates a single byte character, and "string" indicates ASCII text bytes.
//!
//! And the final flattened byte stream is:
//!
//! ```text
//! [d3{0x07}Imaging(l3{0x0D}shutter_speed:f6{0x7B 14 AE 47 E1 7A 84 3F})(l3{0x08}aperture:f5{0x33 33 33 40})(l3{0x03}iso:u4{0x01 90})]
//! ```
//!
//! Each section field is enclosed by `()`'s and always starts with a text identifier (`l` marker + ASCII string), 
//! followed by `:` and its value(s) separated by `,`. Section fields are flattened sequentially, creating a 
//! self-describing stream.
//!
//! ## Optional History Section (Will change heavily as design matures)
//!
//! For applications requiring detailed tracking beyond the immutable creation timestamp:
//!
//! ```text
//! [dHistory
//!  (ef6{1234567890.5},hb{256}{hash_at_creation},ltool:x{Lumis},lversion:z{0.1.2},lhost:x{workstation-sea})
//!  (ef6{1234567920.3},hb{256}{hash_after_modify},ltool:x{Photon},laction:x{modified},lhost:x{laptop-pdx})
//!  (ef6{1234567950.1},hb{256}{hash_after_access},laction:x{accessed},lhost:x{phone-mobile})
//! ]
//! ```
//!
//! Each history entry records the file's `hb` hash at that point in time, creating a verifiable
//! chain of file states. To verify history integrity, recompute `hb` for each historical state
//! by truncating the History section to that entry.
//!
//! Which flattens to:
//!
//! ```text
//! '[' + 'd' + '1' + {7u8} + "History" +
//! '(' + 'e' + 'f' + '6' + {1234567890.5f64} + ',' +
//!       'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' +
//!       'l' + '1' + {4u8} + "tool" + ':' + 'x' + '1' + {5u8} + "Lumis" + ',' +
//!       'l' + '1' + {7u8} + "version" + ':' + 'z' + '1' + {5u8} + "0.1.2" + ',' +
//!       'l' + '1' + {4u8} + "host" + ':' + 'x' + '2' + {15u8} + "workstation-sea" + ')' +
//! '(' + 'e' + 'f' + '6' + {1234567920.3f64} + ',' +
//!       'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' +
//!       'l' + '1' + {4u8} + "tool" + ':' + 'x' + '1' + {6u8} + "Photon" + ',' +
//!       'l' + '1' + {6u8} + "action" + ':' + 'x' + '1' + {8u8} + "modified" + ',' +
//!       'l' + '1' + {4u8} + "host" + ':' + 'x' + '1' + {10u8} + "laptop-pdx" + ')' +
//! '(' + 'e' + 'f' + '6' + {1234567950.1f64} + ',' +
//!       'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' +
//!       'l' + '1' + {6u8} + "action" + ':' + 'x' + '1' + {8u8} + "accessed" + ',' +
//!       'l' + '1' + {4u8} + "host" + ':' + 'x' + '1' + {12u8} + "mobile" + ')' + ']'
//! ```
//!
//! Each history entry is a complete event enclosed in `()`'s with timestamp, tool, action, and context.
//! The History section has its own hash in the header label record for integrity verification, but is
//! NOT included in `hs` (static content hash). It IS included in `hb` (rolling file hash).
//! ```
//!
//! ## Quick Start
//!
//! ```
//! use vsf::{VsfType, VsfBuilder, Tensor, parse};
//!
//! // Encode a tensor
//! let tensor = Tensor::new(vec![3, 4], vec![1u16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
//! let encoded = VsfType::t_u4(tensor).flatten();
//!
//! // Decode it back
//! let mut ptr = 0;
//! let decoded = parse(&encoded, &mut ptr).unwrap();
//!
//! // Build a complete VSF file with header
//! let vsf_file = VsfBuilder::new()
//!     .add_section("metadata", vec![
//!         ("width".to_string(), VsfType::u(1920, false)),
//!         ("height".to_string(), VsfType::u(1080, false)),
//!     ])
//!     .add_unboxed("pixels", vec![0xFF; 1024])
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Eagle Time
//!
//! Eagle Time counts seconds (or fractional seconds) since 1969-07-20 20:17:40 UTC (lunar landing).
//! Always coordinated, no timezones, no daylight saving. Just one single unambiguous time standard.
//!
//! ## Module Structure
//!
//! - `types` - Core type definitions (VsfType, Tensor, EagleTime, WorldCoord)
//! - `encoding` - Binary serialization (exponential-width integers, flatten)
//! - `decoding` - Binary parsing
//! - `file_format` - VSF file headers and sections
//! - `vsf_builder` - High-level builder for complete files
//! - `verification` - Cryptographic hashing and signing
//! - `text_encoding` - Huffman compression for Unicode strings
//! - `colour` - Colourspace conversions (VSF RGB, Rec.2020, sRGB, XYZ)
//! - `builders` - Domain-specific builders (RAW images)
//!

// VSF format version constants
/// Current VSF format version
pub const VSF_VERSION: usize = 2;

/// Backward compatibility version (oldest version this implementation can read)
pub const VSF_BACKWARD_COMPAT: usize = 2;

// Core type system
pub mod types;

// Binary encoding
pub mod encoding;

// Binary decoding
pub mod decoding;

// High-level builders for common use cases
pub mod builders;

// Huffman text encoding for `x` marker
pub mod text_encoding;

// VSF file format with headers and labels
pub mod file_format;

// VSF file builder
pub mod vsf_builder;

// Cryptographic algorithm identifiers (h, g, k types)
pub mod crypto_algorithms;

// Verification functions for hashing and signing VSF files
pub mod verification;

// Colour system (spectral and legacy colourspaces)
pub mod colour;

// Re-export main types
pub use types::{
    datetime_to_eagle_time, EagleTime, EtType, LayoutOrder, StridedTensor, Tensor, VsfType,
    WorldCoord,
};

// Re-export colour conversion types
pub use colour::convert::{ColourFormat, RgbLinear, RgbaLinear};

// Re-export encoding traits
pub use encoding::{EncodeNumber, EncodeNumberInclusive};

// Re-export decoding function
pub use decoding::parse;

// Re-export file format and builder
pub use file_format::{validate_name, LabelDefinition, VsfHeader, VsfSection};
pub use vsf_builder::VsfBuilder;

// RAW image builders and parser
pub use builders::{
    build_raw_image,
    lumis_raw_capture,
    parse_raw_image,
    // Newtype wrappers for type safety
    Aperture,
    BlackLevel,
    CalibrationHash,
    // Builder structs
    CameraBuilder,
    CameraSettings,
    CfaPattern,
    ExposureCompensation,
    FlashFired,
    FocalLength,
    FocusDistance,
    IsoSpeed,
    LensBuilder,
    LensInfo,
    Magic9,
    Manufacturer,
    MeteringMode,
    ModelName,
    ParsedRawImage,
    RawImageBuilder,
    RawMetadata,
    RawMetadataBuilder,
    SerialNumber,
    ShutterTime,
    WhiteLevel,
};

// Coming soon
// pub mod registry;  // Metadata key registry

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let tensor = Tensor::new(vec![3, 4], vec![0u8; 12]);
        assert_eq!(tensor.len(), 12);
        assert_eq!(tensor.ndim(), 2);
        assert_eq!(tensor.shape, vec![3, 4]);
        assert!(!tensor.is_empty());
    }

    #[test]
    fn test_strided_tensor_contiguous() {
        // Row-major (contiguous)
        let row_major = StridedTensor::new(vec![3, 4], vec![4, 1], vec![0u8; 12]);
        assert!(row_major.is_contiguous());

        // Column-major (non-contiguous in row-major memory)
        let col_major = StridedTensor::new(vec![3, 4], vec![1, 3], vec![0u8; 12]);
        assert!(!col_major.is_contiguous());
    }

    #[test]
    fn test_tensor_dimensions() {
        let t1d = Tensor::new(vec![100], vec![0u32; 100]);
        assert_eq!(t1d.ndim(), 1);

        let t2d = Tensor::new(vec![10, 20], vec![0u16; 200]);
        assert_eq!(t2d.ndim(), 2);

        let t3d = Tensor::new(vec![5, 10, 20], vec![0u8; 1000]);
        assert_eq!(t3d.ndim(), 3);

        let t4d = Tensor::new(vec![2, 3, 4, 5], vec![0f32; 120]);
        assert_eq!(t4d.ndim(), 4);
    }

    #[test]
    #[should_panic(expected = "Data length 10 doesn't match shape")]
    fn test_tensor_size_validation() {
        // This should panic: 3×4 = 12, but we only gave 10 elements
        Tensor::new(vec![3, 4], vec![0u8; 10]);
    }

    // ==================== ROUND-TRIP TESTS ====================

    #[test]
    fn test_roundtrip_unsigned() {
        // u0 (bool)
        let val = VsfType::u0(true);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert_eq!(ptr, flat.len());
        if let VsfType::u0(v) = parsed {
            assert_eq!(v, true);
        } else {
            panic!("Expected u0");
        }

        // u3
        let val = VsfType::u3(42);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert_eq!(ptr, flat.len());
        if let VsfType::u3(v) = parsed {
            assert_eq!(v, 42);
        } else {
            panic!("Expected u3");
        }

        // u4
        let val = VsfType::u4(1000);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::u4(v) = parsed {
            assert_eq!(v, 1000);
        } else {
            panic!("Expected u4");
        }

        // u5
        let val = VsfType::u5(100000);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::u5(v) = parsed {
            assert_eq!(v, 100000);
        } else {
            panic!("Expected u5");
        }
    }

    #[test]
    fn test_roundtrip_signed() {
        // i3
        let val = VsfType::i3(-42);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::i3(v) = parsed {
            assert_eq!(v, -42);
        } else {
            panic!("Expected i3");
        }

        // i5
        let val = VsfType::i5(-100000);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::i5(v) = parsed {
            assert_eq!(v, -100000);
        } else {
            panic!("Expected i5");
        }
    }

    #[test]
    fn test_roundtrip_float() {
        // f5
        let val = VsfType::f5(3.14159);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::f5(v) = parsed {
            assert!((v - 3.14159).abs() < 0.00001);
        } else {
            panic!("Expected f5");
        }

        // f6
        let val = VsfType::f6(2.718281828459045);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::f6(v) = parsed {
            assert!((v - 2.718281828459045).abs() < 1e-15);
        } else {
            panic!("Expected f6");
        }
    }

    #[test]
    fn test_roundtrip_complex() {
        use num_complex::Complex;

        // j5
        let val = VsfType::j5(Complex::new(1.0f32, 2.0f32));
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::j5(v) = parsed {
            assert!((v.re - 1.0).abs() < 0.00001);
            assert!((v.im - 2.0).abs() < 0.00001);
        } else {
            panic!("Expected j5");
        }

        // j6
        let val = VsfType::j6(Complex::new(3.14, -2.71));
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::j6(v) = parsed {
            assert!((v.re - 3.14).abs() < 1e-15);
            assert!((v.im + 2.71).abs() < 1e-15);
        } else {
            panic!("Expected j6");
        }
    }

    #[test]
    fn test_roundtrip_string() {
        let val = VsfType::x("Hello, VSF!".to_string());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::x(v) = parsed {
            assert_eq!(v, "Hello, VSF!");
        } else {
            panic!("Expected x");
        }
    }

    #[test]
    fn test_roundtrip_metadata() {
        // Label
        let val = VsfType::l("test_label".to_string());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::l(v) = parsed {
            assert_eq!(v, "test_label");
        } else {
            panic!("Expected l");
        }

        // Offset
        let val = VsfType::o(1024);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::o(v) = parsed {
            assert_eq!(v, 1024);
        } else {
            panic!("Expected o");
        }

        // Version
        let val = VsfType::z(42);
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::z(v) = parsed {
            assert_eq!(v, 42);
        } else {
            panic!("Expected z");
        }
    }

    #[test]
    fn test_roundtrip_eagle_time() {
        // Eagle time with usize
        let val = VsfType::e(EtType::u(1000000));
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::e(EtType::u(v)) = parsed {
            assert_eq!(v, 1000000);
        } else {
            panic!("Expected e(u)");
        }

        // Eagle time with f64
        let val = VsfType::e(EtType::f6(123456.789));
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::e(EtType::f6(v)) = parsed {
            assert!((v - 123456.789).abs() < 1e-10);
        } else {
            panic!("Expected e(f6)");
        }
    }

    #[test]
    fn test_roundtrip_tensor_small() {
        // 2D tensor of u16 (3x4)
        let data = vec![1u16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let tensor = Tensor::new(vec![3, 4], data.clone());
        let val = VsfType::t_u4(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::t_u4(t) = parsed {
            assert_eq!(t.shape, vec![3, 4]);
            assert_eq!(t.data, data);
            assert_eq!(ptr, flat.len()); // Consumed all bytes
        } else {
            panic!("Expected t_u4 tensor");
        }
    }

    #[test]
    fn test_roundtrip_tensor_1d() {
        // 1D tensor of i32
        let data = vec![-100i32, 0, 100, 200, -50];
        let tensor = Tensor::new(vec![5], data.clone());
        let val = VsfType::t_i5(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::t_i5(t) = parsed {
            assert_eq!(t.shape, vec![5]);
            assert_eq!(t.data, data);
        } else {
            panic!("Expected t_i5 tensor");
        }
    }

    #[test]
    fn test_roundtrip_tensor_f64() {
        // 2D tensor of f64 (2x3)
        let data = vec![1.1f64, 2.2, 3.3, 4.4, 5.5, 6.6];
        let tensor = Tensor::new(vec![2, 3], data.clone());
        let val = VsfType::t_f6(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::t_f6(t) = parsed {
            assert_eq!(t.shape, vec![2, 3]);
            for (a, b) in t.data.iter().zip(data.iter()) {
                assert!((a - b).abs() < 1e-10);
            }
        } else {
            panic!("Expected t_f6 tensor");
        }
    }

    #[test]
    fn test_roundtrip_spirix_f4e3() {
        use spirix::{CircleF4E3, ScalarF4E3};

        // ScalarF4E3
        let scalar = ScalarF4E3 {
            fraction: 12345,
            exponent: -42,
        };
        let val = VsfType::s43(scalar);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::s43(s) = parsed {
            assert_eq!(s.fraction, 12345);
            assert_eq!(s.exponent, -42);
            assert_eq!(ptr, flat.len()); // Consumed all bytes
        } else {
            panic!("Expected s43");
        }

        // CircleF4E3
        let circle = CircleF4E3 {
            real: 100,
            imaginary: -200,
            exponent: 5,
        };
        let val = VsfType::c43(circle);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::c43(c) = parsed {
            assert_eq!(c.real, 100);
            assert_eq!(c.imaginary, -200);
            assert_eq!(c.exponent, 5);
            assert_eq!(ptr, flat.len()); // Consumed all bytes
        } else {
            panic!("Expected c43");
        }
    }

    #[test]
    fn test_roundtrip_bitpacked_12bit() {
        use crate::types::BitPackedTensor;

        // Lumis 12-bit RAW: small 10x20 sensor
        let samples: Vec<u64> = (0..200).map(|i| (i * 17) % 4096).collect(); // 12-bit values
        let tensor = BitPackedTensor::pack(12, vec![10, 20], &samples);

        // Encode
        let val = VsfType::p(tensor);
        let flat = val.flatten();

        // Decode
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::p(decoded) = parsed {
            assert_eq!(decoded.bit_depth, 12);
            assert_eq!(decoded.shape, vec![10, 20]);
            assert_eq!(ptr, flat.len()); // Consumed all bytes

            // Unpack and verify
            let unpacked = decoded.unpack().into_u64();
            assert_eq!(unpacked.len(), 200);
            for (i, &val) in unpacked.iter().enumerate() {
                assert_eq!(val, samples[i], "Sample {} mismatch", i);
            }
        } else {
            panic!("Expected bitpacked tensor");
        }
    }

    #[test]
    fn test_roundtrip_bitpacked_1bit() {
        use crate::types::BitPackedTensor;

        // 1-bit boolean-like tensor
        let samples: Vec<u64> = vec![1, 0, 1, 1, 0, 0, 1, 0];
        let tensor = BitPackedTensor::pack(1, vec![8], &samples);

        let val = VsfType::p(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::p(decoded) = parsed {
            assert_eq!(decoded.bit_depth, 1);
            assert_eq!(decoded.shape, vec![8]);
            assert_eq!(decoded.data.len(), 1); // 8 bits = 1 byte

            let unpacked = decoded.unpack().into_u64();
            assert_eq!(unpacked, samples);
        } else {
            panic!("Expected bitpacked tensor");
        }
    }

    #[test]
    fn test_roundtrip_bitpacked_13bit() {
        use crate::types::BitPackedTensor;

        // 13-bit arbitrary depth
        let samples: Vec<u64> = vec![0, 8191, 4096, 2048, 1024];
        let tensor = BitPackedTensor::pack(13, vec![5], &samples);

        let val = VsfType::p(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::p(decoded) = parsed {
            assert_eq!(decoded.bit_depth, 13);
            assert_eq!(decoded.shape, vec![5]);

            let unpacked = decoded.unpack().into_u64();
            assert_eq!(unpacked, samples);
        } else {
            panic!("Expected bitpacked tensor");
        }
    }

    #[test]
    #[should_panic(expected = "Cannot pack")]
    fn test_bitpacked_type_overflow() {
        use crate::types::BitPackedTensor;

        // Try to pack 12-bit values into u8 (type too small)
        let samples: Vec<u8> = vec![255]; // u8 only holds 8 bits, need 12
        BitPackedTensor::pack(12, vec![1], &samples); // Should panic: type capacity exceeded
    }

    #[test]
    fn test_bitpacked_value_masking() {
        use crate::types::BitPackedTensor;

        // Values exceeding bit_depth are masked (no panic, just truncation)
        let samples: Vec<u64> = vec![4096]; // 4096 = 0x1000, 12-bit max is 4095
        let tensor = BitPackedTensor::pack(12, vec![1], &samples); // No panic!

        // Unpack should give masked value: 4096 & 0xFFF = 0
        let unpacked = tensor.unpack().into_u64();
        assert_eq!(unpacked[0], 0); // Low 12 bits of 4096 (0x1000) = 0
    }

    #[test]
    fn test_world_coord_xyz_roundtrip() {
        use crate::types::WorldCoord;

        // Test XYZ round-trip (simpler, no lat/lon conversion)
        let coord = WorldCoord::from_xyz(0.5, 0.5, 0.7071); // Normalized point
        let (x, y, z) = coord.to_xyz();

        // Should be very close (Dymaxion has ~2mm error on Earth radius ~6371km)
        assert!((x - 0.5).abs() < 0.01, "X error: {}", (x - 0.5).abs());
        assert!((y - 0.5).abs() < 0.01, "Y error: {}", (y - 0.5).abs());
        assert!((z - 0.7071).abs() < 0.01, "Z error: {}", (z - 0.7071).abs());
    }

    #[test]
    fn test_roundtrip_world_coord() {
        use crate::types::WorldCoord;

        // Test with a simple coordinate (0, 0) - equator, prime meridian
        let coord = WorldCoord::from_lat_lon(0.0, 0.0);
        let val = VsfType::w(coord);
        let flat = val.flatten();

        assert_eq!(flat[0], b'w');
        assert_eq!(flat.len(), 9); // 1 marker + 8 bytes for u64

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert_eq!(ptr, flat.len());

        if let VsfType::w(decoded) = parsed {
            assert_eq!(decoded.raw(), coord.raw());
            let (lat, lon) = decoded.to_lat_lon();
            // Test at equator/prime meridian for simplicity
            println!("Decoded (0,0): lat={}, lon={}", lat, lon);
            assert!(lat.abs() < 1.0, "Lat error: {}", lat.abs());
            assert!(lon.abs() < 1.0, "Lon error: {}", lon.abs());
        } else {
            panic!("Expected WorldCoord");
        }
    }

    #[test]
    fn test_world_coord_word_encoding() {
        use crate::types::WorldCoord;

        // Test word encoding round-trip
        let coord = WorldCoord::from_lat_lon(51.5074, -0.1278); // London
        let words = coord.to_words();

        // Should be 7 words
        assert_eq!(words.split_whitespace().count(), 7);

        // Decode back
        let decoded = WorldCoord::from_words(&words).expect("Should decode valid words");
        assert_eq!(decoded.raw(), coord.raw());
    }

    #[test]
    fn test_wrapped_type_roundtrip() {
        // Test the 'v' wrapped data type
        let original_data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Wrap with algorithm 'z' (zstd compression - simulated)
        let wrapped = VsfType::v(b'z', original_data.clone());
        let flat = wrapped.flatten();

        // Verify encoding
        assert_eq!(flat[0], b'v'); // Marker
        assert_eq!(flat[1], b'z'); // Algorithm

        // Parse back
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        if let VsfType::v(alg, data) = parsed {
            assert_eq!(alg, b'z');
            assert_eq!(data, original_data);
            assert_eq!(ptr, flat.len()); // Consumed all bytes
        } else {
            panic!("Expected wrapped type");
        }
    }

    #[test]
    fn test_wrapped_type_algorithms() {
        // Test different algorithm identifiers
        let algorithms = vec![b'z', b'r', b'x', b'e'];
        let test_data = vec![0xAB; 100];

        for alg in algorithms {
            let wrapped = VsfType::v(alg, test_data.clone());
            let flat = wrapped.flatten();

            let mut ptr = 0;
            let parsed = parse(&flat, &mut ptr).unwrap();

            if let VsfType::v(parsed_alg, parsed_data) = parsed {
                assert_eq!(parsed_alg, alg);
                assert_eq!(parsed_data, test_data);
            } else {
                panic!("Expected wrapped type for algorithm '{}'", alg as char);
            }
        }
    }
}
