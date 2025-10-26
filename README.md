# VSF (Versatile Storage Format)

**Self-describing, hierarchical, binary format for typed data with cryptographic primitives.**

VSF combines type safety, zero-copy performance, and mathematical correctness in a format that's both human-readable (when decoded) and machine-efficient.

## What Works Now (v0.1.2)

✅ **Core type system** - 211 variants covering:
- IEEE primitives (`u3-u7`, `i3-i7`, `f5-f6`, `j5-j6`)
- Spirix math types (25 scalar + 25 circle variants)
- Multi-dimensional tensors (contiguous + strided)
- Bitpacked tensors (1-256 bits per element)
- Metadata types (strings, time, labels)

✅ **Complete encoding/decoding**
- Full round-trip validation (40/40 tests passing)
- Variable-length integer encoding
- Big-endian byte order
- Zero compiler warnings

✅ **Production-ready core**
- No TODOs, no `unimplemented!()`
- Clean builds in 1.4s
- 7,500+ lines of tested code

## What's Coming Next

🚧 **File I/O** - Write/read VSF files from disk
🚧 **Hierarchical labels** - Organize data with dot notation (`imaging.raw`, `token.identity`)
🚧 **Unboxed sections** - Zero-copy mmap for bulk data
🚧 **Cryptographic types** - Signatures (`g`) and hashes (`h`)
🚧 **VsfBuilder** - Ergonomic file construction

## Design Philosophy

**VSF is unique because it combines:**
- Type safety (strongly typed primitives & tensors)
- Cryptographic primitives (signatures, hashes built-in)
- Hierarchical organization (unlimited nesting)
- Zero-copy sections (unboxed bulk data)
- Mathematical correctness (Spirix integration)
- Temporal anchoring (Eagle Time)

No other format does all of this.

## Basic Usage (Current)
```rust
use vsf::{VsfType, Tensor, BitPackedTensor};

// Create a 12-bit RAW camera image (4096×3072)
let raw = BitPackedTensor {
    shape: vec![4096, 3072],
    bits_per_element: 12,
    data: camera_data,
};

// Encode to bytes
let encoded = VsfType::bp(raw).flatten()?;

// Decode back
let decoded = VsfType::parse(&encoded)?;

// Round-trip verified
assert_eq!(original, decoded);
```

## Planned Usage (File I/O)
```rust
use vsf::VsfFile;

// Write Lumis RAW capture
VsfFile::builder()
    .label("imaging.raw", VsfType::bp(raw_data))
    .metadata("iso_speed", 800u32)
    .metadata("shutter_time_ns", 16_666_667u64)
    .write("photo_001.vsf")?;

// Read back
let file = VsfFile::read("photo_001.vsf")?;
let raw = file.get_bitpacked("imaging.raw")?;
let iso = file.get_metadata::<u32>("iso_speed")?;
```

## Use Cases

**Lumis** - 12-bit RAW camera storage with metadata
**Spirix** - Mathematically correct floating-point serialization
**TOKEN** - Cryptographic identity and attestations
**Photon** - Encrypted message payloads with signatures

## Technical Details

**Variable-length integer encoding:**
```
Size markers (ASCII '3'-'7') indicate bit width:
'3' = 8 bits  (1 byte follows)
'4' = 16 bits (2 bytes follow)
'5' = 32 bits (4 bytes follow)
'6' = 64 bits (8 bytes follow)
'7' = 128 bits (16 bytes follow)

Example: Value 4096
0x34 0x10 0x00  (marker '4' for 16-bit, then big-endian value)
```

**Always uses minimal size for efficiency.**

**Bitpacked tensors:**
```rust
// Store 12-bit camera data efficiently
// 4096×3072 image = 18.9MB (vs 25.2MB as u16)
BitPackedTensor {
    shape: vec![4096, 3072],
    bits_per_element: 12,
    data: packed_bytes,
}
```

## Status: Alpha

VSF's core encoding/decoding is production-ready. File I/O and hierarchical features are in active development (targeting v0.2.0).

**Not stable yet** - Breaking changes possible before v1.0.

## Architecture

See [VSF V2 Architecture Summary](docs/architecture.md) for complete technical specification.

## License

Custom open-source license:
- ✅ Free for any purpose (including commercial)
- ✅ Modify and distribute freely
- ❌ Cannot sell VSF itself or direct derivatives

See LICENSE for full terms.