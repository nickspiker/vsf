// Allow deprecated items within the crate - VSF must handle legacy types internally. External users will still see deprecation warnings when they use legacy APIs.
#![allow(deprecated)]
#![cfg_attr(not(feature = "std"), no_std)]

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
//! - `x`: Huffman compressed Unicode text strings
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
//! z3{5}                                Format version (FIRST - determines encoding)
//! y3{5}                                Backward compatibility version
//! b#{header length}                    Header size (now we know how to encode it!)
//! L#{file length}                      Total file size in bytes (optional, for TCP streaming without parse-as-you-go)
//! eu6{current time as u64}             Eagle Time when emitted (u64 oscillations, 704ps precision). OPTIONAL — devices without a clock (no RTC, no network, pre-handshake) omit the `e` field entirely rather than lie with a placeholder. Readers detect absence by the next byte being `h` (provenance hash) instead of `e`.
//! hp3{31}{provenance hash}             Provenance: BLAKE3 hash of content (required, always 32 bytes)
//! ge{64}{signature}                    Ed25519 signature over entire file AFTER provinence hash is patched in (optional, rolling or provinence, must have one or the other)
//! hb{31}{rolling_hash}                 Rolling: BLAKE3 of current state with History (optional)
//! k#{key}                              File-level encryption key (optional)
//! n#{field count}                      Number of fields
//! (d3{9}raw_image:h#{hash},o#{offset},b#{size},n#{count})     Field with values
//! (d3{9}thumbnail:h#{hash},o#{offset},b#{size},n#{count}) ...
//! >                                    Header end
//!
//! [(section_fields...)...]           Section data at offset for RAW image, note that if section is not encrypted and closer than 1MB from the header, section name, count and length are not required. otherwise all three. [d3{9}thumbnailn#{number of fields}b#{length of section}(section_fields...)...]
//! ```
//!
//! **Hash Strategy (Always BLAKE3):**
//! - **hp** (hash provenance): Content identity - BLAKE3 hash of immutable content. Required. Computed with hp field as zeros, then filled in. Creates stable identifier for original content.
//! - **ge** (signature): Optional Ed25519 signature. When signing, compute hp, sign it, then **replace** hp bytes with ge signature.
//! - **hb** (hash rolling): Current file state - Optional BLAKE3 hash including History section. Updates when History updates. Useful for tracking mutable file evolution. ge or hb, must have one.
//!
//! **Provenance Verification:** To verify a file's provenance, zero the hp and signature/rolling hash fields and compute BLAKE3 - it will match the stored hp if original. If present, verify the ge signature against hp to authenticate the creator.
//!
//! **Terminology:**
//! - **Header**: Everything between `RÅ<` and `>`
//! - **Provenance primitives**: Version, timestamp, hash, signature (NOT wrapped in `()`)
//! - **Header field**: Section pointer `(d"name" o b n)` with POSITIONAL values (no `:` or `,`)
//! - **Section**: Actual data blocks after the header, located at specified offsets
//! - **Section field**: Individual `(field:value)` or `(field:v0,v1)` entries within a section
//! - **`?` and `{}`**: `?` indicates length (ASCII 0-Z), `{}` indicates binary data
//!
//! The `:` and `,` separators in label records make the format human-readable in hex editors and aid in forensics and corruption analysis with minimal overhead.
//!
//! ## Section Flattening Example
//!
//! A section with hierarchical fields for camera metadata:
//!
//! ```text
//! [d{Imaging} (l{shutter_speed}:f6{0.01})      // 1/100s as f64 (l{aperture}:f5{2.8})            // f/2.8 as f32 (l{iso}:u4{400})                 // ISO 400 ]
//! ```
//!
//! Which flattens to:
//!
//! ```text
//! '[' + 'd' + '3' + {7u8} + "Imaging" + '(' + 'l' + '3' + {13u8} + "shutter_speed" + ':' + 'f' + '6' + {0.01f64} + ')' + '(' + 'l' + '3' + {8u8} + "aperture"      + ':' + 'f' + '5' + {2.8f32} + ')' + '(' + 'l' + '3' + {3u8} + "iso" + ':' + 'u' + '4'+ {400u16} + ')' + ']'
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
//! Each section field is enclosed by `()`'s and always starts with a text identifier (`l` marker + ASCII string), followed by `:` and its value(s) separated by `,`. Section fields are flattened sequentially, creating a self-describing stream.
//!
//! ## Optional History Section (Will change heavily as design matures)
//!
//! For applications requiring detailed tracking beyond the immutable creation timestamp:
//!
//! ```text
//! [dHistory (ef6{1234567890.5},hb{256}{hash_at_creation},ltool:x{Lumis},lversion:z{0.1.2},lhost:x{workstation-sea}) (ef6{1234567920.3},hb{256}{hash_after_modify},ltool:x{Photon},laction:x{modified},lhost:x{laptop-pdx}) (ef6{1234567950.1},hb{256}{hash_after_access},laction:x{accessed},lhost:x{phone-mobile}) ]
//! ```
//!
//! Each history entry records the file's `hb` hash at that point in time, creating a verifiable chain of file states. To verify history integrity, recompute `hb` for each historical state by truncating the History section to that entry.
//!
//! Which flattens to:
//!
//! ```text
//! '[' + 'd' + '1' + {7u8} + "History" + '(' + 'e' + 'f' + '6' + {1234567890.5f64} + ',' + 'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' + 'l' + '1' + {4u8} + "tool" + ':' + 'x' + '1' + {5u8} + "Lumis" + ',' + 'l' + '1' + {7u8} + "version" + ':' + 'z' + '1' + {5u8} + "0.1.2" + ',' + 'l' + '1' + {4u8} + "host" + ':' + 'x' + '2' + {15u8} + "workstation-sea" + ')' + '(' + 'e' + 'f' + '6' + {1234567920.3f64} + ',' + 'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' + 'l' + '1' + {4u8} + "tool" + ':' + 'x' + '1' + {6u8} + "Photon" + ',' + 'l' + '1' + {6u8} + "action" + ':' + 'x' + '1' + {8u8} + "modified" + ',' + 'l' + '1' + {4u8} + "host" + ':' + 'x' + '1' + {10u8} + "laptop-pdx" + ')' + '(' + 'e' + 'f' + '6' + {1234567950.1f64} + ',' + 'h' + 'b' + '3' + {32u8} + {32 bytes BLAKE3 hash} + ',' + 'l' + '1' + {6u8} + "action" + ':' + 'x' + '1' + {8u8} + "accessed" + ',' + 'l' + '1' + {4u8} + "host" + ':' + 'x' + '1' + {12u8} + "mobile" + ')' + ']'
//! ```
//!
//! Each history entry is a complete event enclosed in `()`'s with timestamp, tool, action, and context. The History section has its own hash in the header label record for integrity verification, but is NOT included in `hs` (static content hash). It IS included in `hb` (rolling file hash).
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
//! ## Eagle Time Formats
//!
//! Eagle Time counts oscillations since 1969-07-20 20:17:40 UTC (Apollo 11 lunar landing). Always coordinated, no timezones, no daylight saving. One universal time standard.
//!
//! - **eu6**: 64-bit oscillation count (`u64`) - 704ps precision, deterministic integer timestamps (default)
//! - **ef5**: 32-bit float (`f32`) - ~2 minute precision, legacy compact format
//! - **ef6**: 64-bit float (`f64`) - ~200ns precision, legacy high-accuracy format
//!
//! The format version doesn't change the epoch or oscillation frequency - 1,420,407,826 Hz (21cm hydrogen line).
//!
//! ### Optional in the header
//!
//! Eagle Time is OPTIONAL in the file header. Devices that genuinely cannot know what time it is — embedded sensors without an RTC, freshly-booted SoCs before any network sync, single-purpose ASICs — omit the `e` field rather than emit a placeholder. A reader that finds the next byte after `l` (file length) is `h` (provenance hash) rather than `e` knows the producer was clockless and skips the field. Devices that DO know the time (host CLIs via `nunc-time`, kernels via QTIMER, anything network-synced) include it normally. Lying about the time is worse than admitting you don't know it.
//!
//! ## Parsing and Encoding
//!
//! **Element-level parsing:**
//! ```ignore
//! use vsf::parse; let data = vec![b'u', b'3', 42]; let mut ptr = 0; let value = parse(&data, &mut ptr)?;  // Parses one VsfType element
//! ```
//!
//! **Header encoding (VsfHeader):**
//! ```ignore
//! use vsf::file_format::VsfHeader; let mut header = VsfHeader::new(version, backward_compat); header.add_field(field); let bytes = header.encode()?;  // Encodes header to bytes
//! ```
//!
//! **Header decoding (VsfHeader):**
//! ```ignore
//! use vsf::file_format::VsfHeader; let (header, header_end) = VsfHeader::decode(&bytes)?;  // Parses the full RÅ< header
//! ```
//! `VsfHeader::decode()` is implemented at [file_format.rs](src/file_format.rs) (`VsfHeader::decode`); it returns the parsed header plus the byte offset where the header ends.
//!
//! ## Parsing APIs: Two Tiers
//!
//! VSF provides two parsing approaches for sections, each suited to different use cases:
//!
//! ### Low-Level: `VsfSection::parse()` ([file_format.rs](src/file_format.rs))
//!
//! Schema-agnostic parsing that extracts raw data without validation:
//!
//!
//! ```ignore
//! use vsf::VsfSection;
//!
//! let mut ptr = 0; let section = VsfSection::parse(&bytes, &mut ptr)?; // Returns VsfSection with name and Vec<VsfField> // No schema required, no validation performed
//! ```
//!
//! **Use when:**
//! - Reading unknown/arbitrary VSF data
//! - Debugging or inspecting files
//! - Building tooling that handles any section type
//! - You don't have or need a schema
//!
//! ### High-Level: `SectionBuilder::parse()` ([schema/section.rs](src/schema/section.rs))
//!
//! Schema-validated parsing for type-safe workflows:
//!
//!
//! ```ignore
//! use vsf::schema::{SectionSchema, SectionBuilder, TypeConstraint};
//!
//! let schema = SectionSchema::new("camera") .field("iso", TypeConstraint::AnyUnsigned) .field("shutter", TypeConstraint::AnyFloat);
//!
//! let builder = SectionBuilder::parse(schema, &section_bytes)?; // Validates section name matches schema // Validates each field against type constraints // Returns SectionBuilder for modify → re-encode workflow
//! ```
//!
//! **Use when:**
//! - You know the expected structure
//! - Type safety and validation matter
//! - You need to modify and re-encode sections
//! - Building applications with defined schemas
//!
//! Both parse the same `[d"name"(d"field":value)...]` binary format—`SectionBuilder` adds schema enforcement on top of the low-level parsing.
//!
//! ## Module Structure
//!
//! - `types` - Core type definitions (VsfType, Tensor, EagleTime, WorldCoord)
//! - `encoding` - Binary serialization (exponential-width integers, flatten)
//! - `decoding` - Binary parsing with `parse()` function
//! - `file_format` - VSF file headers and sections (VsfHeader, VsfSection)
//! - `vsf_builder` - High-level builder for complete files
//! - `schema` - Type-safe section schemas with field validation and parse→modify→encode
//! - `verification` - Cryptographic hashing and signing
//! - `crypto_algorithms` - Algorithm identifiers for hashes, signatures, keys, MACs
//! - `decrypt` - Decryption utilities (requires `crypto` feature)
//! - `text_encoding` - Huffman compression for Unicode strings (requires `text` feature)
//! - `colour` - Colourspace conversions (VSF RGB, Rec.2020, sRGB, XYZ)
//! - `builders` - Domain-specific builders (RAW images)
//! - `inspect` - Inspection and formatting utilities (requires `inspect` feature)
//!

extern crate alloc;

/// Build an error string for an unexpected VsfType variant at decode time.
///
/// With the `errors-verbose` feature (default-on), expands to `format!` of the expected-label plus a `{:?}` of the value. Without it, expands to a static string with just the expected label and drops the value via `let _ = &v;`.
///
/// Disabling `errors-verbose` lets the linker remove `<VsfType as Debug>::fmt` — which transitively removes the IEEE-754 grisu/dragon float formatters (~10 KB on cortex-m) since they're only kept alive by the float variants' Debug arms.
#[macro_export]
macro_rules! type_mismatch_err {
    ($expected:literal, $got:expr) => {{
        #[cfg(feature = "errors-verbose")]
        {
            ::alloc::format!(concat!($expected, ", got {:?}"), $got)
        }
        #[cfg(not(feature = "errors-verbose"))]
        {
            let _ = &$got;
            <::alloc::string::String as ::core::convert::From<&str>>::from($expected)
        }
    }};
}

/// no_std-friendly prelude: re-exports of the `alloc` types that std's prelude provides automatically. Individual files import via `use crate::prelude::*;` to stay compatible across `std` and `no_std + alloc` builds.
pub mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

// VSF format version constants
/// Current VSF format version v9: spirix payloads (rd/rb/rw/rq and everything nesting them) reinterpreted under spirix 0.1 semantics: implicit-sign Scalar fraction, AMBIG=0 exponent convention, Circle keeps the N1 fraction. Same bytes decode to DIFFERENT values than v8, so this is a wire break for spirix-carrying files even tho non-spirix files are bit-identical. v8: x text codebook remapped to cover the full Unicode codespace (all 1,112,064 codepoints pre-assigned) + NFC canonicalization at the encoder boundary. Changed the Huffman bitstream for every x value — even pure-ASCII strings — so v7 x bytes and v8 x bytes are mutually unintelligible. v7: Added opcodes (op type), literal VSF format, proper bracket notation (⦉⦊ vs {})
pub const VSF_VERSION: usize = 9;

/// Oldest format version this BUILD can read — the floor is build-time chosen, cargo-style. Default is the current version only: wire breaks are never silently bridged, and a too-old file is rejected by name at the header. The plan for honoring old archives: a `compat-v*` feature per legacy dialect lowers this floor AND compiles in that era's decode paths, dispatched on the file's z at parse time; when none is enabled, old files get the loud version error (never a wrong parse). No compat feature may ship as a floor-only no-op — lowering the floor without the era's real readers recreates silent misinterpretation. None exist yet: a true `compat-v8` needs spirix 0.0.x payload decode vendored (the old Scalar bit semantics), and a true `compat-v7` needs the pre-remap x codebook vendored from git (c12c7e2~1) plus the l-as-ASCII-label mapping, and v7 sub-dialects (l→a marker remap ~0.3.5, x codebook remap 0.4.0) shipped without z bumps, so "v7" in the field is ambiguous anyway — per-archive forensics if real v7 data ever needs recovering.
pub const VSF_BACKWARD_COMPAT: usize = 9;

// THE GATE: the crate's semver-breaking number IS the format version, enforced at compile time.
// While vsf is 0.x the breaking number is the MINOR (0.9.z ⇒ format v9); at 1.0+ it becomes the MAJOR.
// cargo publish runs a verify build, so a stale VSF_VERSION cannot reach crates.io — it fails right here first.
// Four VSF releases (plus one spirix) have been yanked because a human forgot this bump; no human step remains.
const fn semver_breaking_number(version: &str) -> usize {
    let b = version.as_bytes();
    let mut i = 0;
    let mut major = 0;
    while i < b.len() && b[i] != b'.' {
        major = major * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    if major > 0 {
        return major;
    }
    i += 1;
    let mut minor = 0;
    while i < b.len() && b[i] != b'.' {
        minor = minor * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    minor
}
const _: () = assert!(
    VSF_VERSION == semver_breaking_number(env!("CARGO_PKG_VERSION")),
    "VSF_VERSION does not match the semver-breaking number in Cargo.toml. Bump VSF_VERSION (and review VSF_BACKWARD_COMPAT with it) — the z marker is the wire contract, not a formality."
);
const _: () = assert!(
    VSF_BACKWARD_COMPAT <= VSF_VERSION,
    "VSF_BACKWARD_COMPAT cannot exceed VSF_VERSION — the compat floor can only reach backward."
);

// Core type system
pub mod types;

// Binary encoding
pub mod encoding;

// Binary decoding
pub mod decoding;

// High-level builders for common use cases
pub mod builders;

// Huffman text encoding for `x` marker
#[cfg(any(feature = "text", feature = "text-encode"))]
pub mod text_encoding;

// VSF file format with headers and labels
pub mod file_format;

// VSF file builder
pub mod vsf_builder;

// Schema system for type-safe sections (experimental)
pub mod schema;

// Cryptographic algorithm identifiers (h, g, k types)
pub mod crypto_algorithms;

// Verification functions for hashing and signing VSF files
pub mod verification;

// Handle identity: plaintext → proof-of-work → public ID (requires ihi, gated to break cycle: ihi depends on vsf for text-encode)
#[cfg(feature = "handle")]
pub mod handle;

// Decryption utilities
#[cfg(feature = "crypto")]
pub mod decrypt;

// Colour system (spectral and legacy colourspaces)
pub mod colour;

// Theme definitions for inspection output
pub mod themes;

// Inspection and formatting utilities (coloured output)
#[cfg(feature = "inspect")]
pub mod inspect;

// VSF image decode (tensor + rav1d AV1) → α + darkness buffer. Counterpart to the `vsfimg` encoder; feeds fluor's `Icon` and toka's `Canvas` (shared α+darkness convention). Pulls rav1d, so it's opt-in — off for size-critical consumers (the fgtw-bootstrap worker).
#[cfg(feature = "image-decode")]
pub mod image;

// VSF-Image v0 — spectral-first raw image container (K channels of sensor counts + per-channel spectral response + ihi provenance ingredients). Pure VSF types, no heavy deps, so it's unconditional. Consumed by opsin (viewer/converter) and chameleon.
pub mod spectral_image;

// Re-export main types
pub use types::{
    datetime_to_eagle_time, BitPackedTensor, EagleTime, EtType, LayoutOrder, NaScheme,
    StridedTensor, Tensor, VsfType, WaAddress, WorldCoord, OSCILLATIONS_PER_SECOND,
};
#[cfg(feature = "std")]
pub use types::{eagle_time_nanos, eagle_time_oscillations};

// Re-export colour conversion types
pub use colour::convert::{ColourFormat, RgbLinearF32, RgbaLinearF32};

// Re-export encoding traits
pub use encoding::{EncodeNumber, EncodeNumberInclusive};

// Re-export decoding function
pub use decoding::parse;

// Re-export file format and builder
pub use file_format::{validate_name, HeaderField, VsfField, VsfHeader, VsfSection};
pub use vsf_builder::{SectionMeta, VsfBuilder};

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

// Coming soon pub mod registry;  // Metadata key registry

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

    #[cfg(feature = "text-encode")]
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
    #[allow(non_snake_case)]
    fn test_roundtrip_hP() {
        let bytes = vec![0xABu8; 32];
        let val = VsfType::hP(bytes.clone());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert!(
            matches!(parsed, VsfType::hP(ref b) if *b == bytes),
            "hP round-trip failed"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_roundtrip_hR() {
        let bytes = vec![0xCDu8; 32];
        let val = VsfType::hR(bytes.clone());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert!(
            matches!(parsed, VsfType::hR(ref b) if *b == bytes),
            "hR round-trip failed"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_roundtrip_hI() {
        let bytes = vec![0x11u8; 32];
        let val = VsfType::hI(bytes.clone());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert!(
            matches!(parsed, VsfType::hI(ref b) if *b == bytes),
            "hI round-trip failed"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_roundtrip_hV() {
        let bytes = vec![0x22u8; 32];
        let val = VsfType::hV(bytes.clone());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        assert!(
            matches!(parsed, VsfType::hV(ref b) if *b == bytes),
            "hV round-trip failed"
        );
    }

    #[test]
    fn test_roundtrip_metadata() {
        // ASCII text
        let val = VsfType::a("test_label".to_string());
        let flat = val.flatten();
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::a(v) = parsed {
            assert_eq!(v, "test_label");
        } else {
            panic!("Expected a");
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
        // e6: canonical 64-bit oscillation count
        let val = VsfType::e(EtType::e6(1000000));
        let flat = val.flatten();
        assert_eq!(flat[0], b'e');
        assert_eq!(flat[1], b'6');
        assert_eq!(flat.len(), 10); // 2 tag + 8 bytes
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::e(EtType::e6(v)) = parsed {
            assert_eq!(v, 1000000);
        } else {
            panic!("Expected e(e6)");
        }

        // e5: 32-bit oscillation count
        let val = VsfType::e(EtType::e5(42000));
        let flat = val.flatten();
        assert_eq!(flat[0], b'e');
        assert_eq!(flat[1], b'5');
        assert_eq!(flat.len(), 6); // 2 tag + 4 bytes
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::e(EtType::e5(v)) = parsed {
            assert_eq!(v, 42000);
        } else {
            panic!("Expected e(e5)");
        }

        // e7: 128-bit oscillation count
        let val = VsfType::e(EtType::e7(999_999_999_999_999_999_i128));
        let flat = val.flatten();
        assert_eq!(flat[0], b'e');
        assert_eq!(flat[1], b'7');
        assert_eq!(flat.len(), 18); // 2 tag + 16 bytes
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::e(EtType::e7(v)) = parsed {
            assert_eq!(v, 999_999_999_999_999_999_i128);
        } else {
            panic!("Expected e(e7)");
        }

        // deprecated ef6: still round-trips
        #[allow(deprecated)]
        {
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
        // 1D tensor of i32 - encodes as tensor, decodes as vector (1D optimization)
        let data = vec![-100i32, 0, 100, 200, -50];
        let tensor = Tensor::new(vec![5], data.clone());
        let val = VsfType::t_i5(tensor);
        let flat = val.flatten();

        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();

        // 1D tensors decode as vectors (compact representation)
        if let VsfType::v_i5(v) = parsed {
            assert_eq!(v.data, data);
        } else {
            panic!("Expected v_i5 vector, got {:?}", parsed);
        }
    }

    // ==================== 1D VECTOR OPTIMIZATION TESTS ====================

    #[test]
    fn test_1d_vector_optimization_unsigned() {
        // Test all unsigned types with 1D vector optimization 1D tensors encode with 'tn' format and decode as vectors Exception: u8 stays as tensor (since Vec<u8> == raw bytes)

        // u8 vector - special case: stays as tensor
        let data_u8 = vec![1u8, 2, 3, 4, 5, 10, 20, 30, 40, 50, 100, 200];
        let tensor_u8 = Tensor::new(vec![12], data_u8.clone());
        let val_u8 = VsfType::t_u3(tensor_u8);
        let flat_u8 = val_u8.flatten();

        // Check that it uses 'n' format (compact)
        assert_eq!(flat_u8[0], b't');
        assert_eq!(flat_u8[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_u8 = parse(&flat_u8, &mut ptr).unwrap();
        // u8 is special: decodes back as tensor (raw byte compatibility)
        if let VsfType::t_u3(t) = parsed_u8 {
            assert_eq!(t.shape, vec![12]);
            assert_eq!(t.data, data_u8);
            assert_eq!(ptr, flat_u8.len());
        } else {
            panic!("Expected t_u3 tensor, got {:?}", parsed_u8);
        }

        // u16 vector
        let data_u16 = vec![100u16, 200, 300, 400, 500, 1000, 2000, 3000];
        let tensor_u16 = Tensor::new(vec![8], data_u16.clone());
        let val_u16 = VsfType::t_u4(tensor_u16);
        let flat_u16 = val_u16.flatten();

        assert_eq!(flat_u16[0], b't');
        assert_eq!(flat_u16[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_u16 = parse(&flat_u16, &mut ptr).unwrap();
        if let VsfType::v_u4(v) = parsed_u16 {
            assert_eq!(v.data, data_u16);
        } else {
            panic!("Expected v_u4 vector, got {:?}", parsed_u16);
        }

        // u32 vector
        let data_u32 = vec![100000u32, 200000, 300000, 400000, 500000];
        let tensor_u32 = Tensor::new(vec![5], data_u32.clone());
        let val_u32 = VsfType::t_u5(tensor_u32);
        let flat_u32 = val_u32.flatten();

        assert_eq!(flat_u32[0], b't');
        assert_eq!(flat_u32[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_u32 = parse(&flat_u32, &mut ptr).unwrap();
        if let VsfType::v_u5(v) = parsed_u32 {
            assert_eq!(v.data, data_u32);
        } else {
            panic!("Expected v_u5 vector, got {:?}", parsed_u32);
        }

        // u64 vector
        let data_u64 = vec![1_000_000_000u64, 2_000_000_000, 3_000_000_000];
        let tensor_u64 = Tensor::new(vec![3], data_u64.clone());
        let val_u64 = VsfType::t_u6(tensor_u64);
        let flat_u64 = val_u64.flatten();

        assert_eq!(flat_u64[0], b't');
        assert_eq!(flat_u64[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_u64 = parse(&flat_u64, &mut ptr).unwrap();
        if let VsfType::v_u6(v) = parsed_u64 {
            assert_eq!(v.data, data_u64);
        } else {
            panic!("Expected v_u6 vector, got {:?}", parsed_u64);
        }
    }

    #[test]
    fn test_1d_vector_optimization_signed() {
        // Test all signed types with 1D vector optimization 1D tensors encode with 'tn' format and decode as vectors

        // i8 vector
        let data_i8 = vec![-10i8, -5, 0, 5, 10, 20, -20, 30];
        let tensor_i8 = Tensor::new(vec![8], data_i8.clone());
        let val_i8 = VsfType::t_i3(tensor_i8);
        let flat_i8 = val_i8.flatten();

        assert_eq!(flat_i8[0], b't');
        assert_eq!(flat_i8[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_i8 = parse(&flat_i8, &mut ptr).unwrap();
        if let VsfType::v_i3(v) = parsed_i8 {
            assert_eq!(v.data, data_i8);
        } else {
            panic!("Expected v_i3 vector, got {:?}", parsed_i8);
        }

        // i16 vector
        let data_i16 = vec![-1000i16, -500, 0, 500, 1000];
        let tensor_i16 = Tensor::new(vec![5], data_i16.clone());
        let val_i16 = VsfType::t_i4(tensor_i16);
        let flat_i16 = val_i16.flatten();

        assert_eq!(flat_i16[0], b't');
        assert_eq!(flat_i16[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_i16 = parse(&flat_i16, &mut ptr).unwrap();
        if let VsfType::v_i4(v) = parsed_i16 {
            assert_eq!(v.data, data_i16);
        } else {
            panic!("Expected v_i4 vector, got {:?}", parsed_i16);
        }

        // i32 vector
        let data_i32 = vec![-100000i32, 0, 100000, 200000, -50000];
        let tensor_i32 = Tensor::new(vec![5], data_i32.clone());
        let val_i32 = VsfType::t_i5(tensor_i32);
        let flat_i32 = val_i32.flatten();

        assert_eq!(flat_i32[0], b't');
        assert_eq!(flat_i32[1], b'n'); // Should use 'n' for 1D

        let mut ptr = 0;
        let parsed_i32 = parse(&flat_i32, &mut ptr).unwrap();
        if let VsfType::v_i5(v) = parsed_i32 {
            assert_eq!(v.data, data_i32);
        } else {
            panic!("Expected v_i5 vector, got {:?}", parsed_i32);
        }
    }

    #[test]
    fn test_1d_vector_vs_multi_dim_encoding() {
        // Verify that 1D uses compact format while multi-dim uses full format

        // 1D vector
        let data_1d = vec![1u16, 2, 3, 4, 5];
        let tensor_1d = Tensor::new(vec![5], data_1d.clone());
        let val_1d = VsfType::t_u4(tensor_1d);
        let flat_1d = val_1d.flatten();

        // 2D tensor
        let data_2d = vec![1u16, 2, 3, 4, 5, 6];
        let tensor_2d = Tensor::new(vec![2, 3], data_2d.clone());
        let val_2d = VsfType::t_u4(tensor_2d);
        let flat_2d = val_2d.flatten();

        // 1D should use 'n' format
        assert_eq!(flat_1d[0], b't');
        assert_eq!(flat_1d[1], b'n');

        // 2D should use 'u' (ndim) format
        assert_eq!(flat_2d[0], b't');
        assert_ne!(flat_2d[1], b'n'); // Should NOT be 'n'

        // 1D should be more compact Format comparison: 1D: t n <count> u 4 <data> 2D: t <ndim> u 4 <shape[0]> <shape[1]> <data> For small shapes, 1D should save bytes
        assert!(flat_1d.len() <= flat_2d.len());
    }

    #[test]
    fn test_1d_vector_large() {
        // Test with a larger vector (FGTW use case: 32-byte hashes)
        let hash_data = vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB,
            0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xAB, 0xCD, 0xEF,
        ];

        let tensor = Tensor::new(vec![32], hash_data.clone());
        let val = VsfType::t_u3(tensor);
        let flat = val.flatten();

        // Verify compact format
        assert_eq!(flat[0], b't');
        assert_eq!(flat[1], b'n');

        // Round-trip
        let mut ptr = 0;
        let parsed = parse(&flat, &mut ptr).unwrap();
        if let VsfType::t_u3(t) = parsed {
            assert_eq!(t.shape, vec![32]);
            assert_eq!(t.data, hash_data);
            assert_eq!(ptr, flat.len());
        } else {
            panic!("Expected t_u3 tensor");
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
    #[cfg(feature = "spirix")]
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
        assert_eq!(flat[1], b'6'); // w6 = WorldCoord 64-bit standard precision
        assert_eq!(flat.len(), 10); // "w6" + 8 bytes for u64

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
