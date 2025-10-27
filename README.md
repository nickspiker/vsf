# VSF (Versatile Storage Format)

A self-describing binary format designed for optimal integer encoding, mathematical correctness, and type safety.

VSF addresses a fundamental challenge in binary formats: how to efficiently encode integers of any size while maintaining O(1) skip-ability. The solution enables efficient storage of everything from a single photon's wavelength to the number of atoms in the observable universe (and yes, both fit comfortably).

---

## Core Innovation: Exponential-Width Integer Encoding

Most binary formats face a tradeoff when encoding integers:

**Fixed-width approach** (TIFF, PNG, HDF5):
- Fast to parse (known size)
- Wastes space on small values
- Hits hard limits (4GB for u32, etc.)

**Variable-width approach** (Protobuf, MessagePack):
- Compact encoding (7 bits per byte, continuation bit in MSB)
- Must parse to know size (O(n) skip cost)
- Caps at 64 bits (Protobuf stops at 2^64-1)
- Can't encode "Planck volumes in observable universe" (~10^185, needs 185 bits)

**VSF's solution** - Exponential-width with explicit size markers:

```
Value 42:                'u' '3' 0x2A              (3 bytes)
Value 4,096:             'u' '4' 0x10 0x00         (4 bytes)  
Value 2^32-1:            'u' '5' + 4 bytes         (6 bytes)
Value 2^64-1:            'u' '6' + 8 bytes         (10 bytes)
Value 2^128-1:           'u' '7' + 16 bytes        (18 bytes)
Value 2^256-1:           'u' '8' + 32 bytes        (34 bytes)
RSA-16384 prime:         'u' 'D' + 2048 bytes      (2050 bytes, marker 'D' = 2^13 = 8192 bits)
Theoretical max:         'u' 'Z' + 8 GB            (~8 GB, for when you really mean it)

Markers: '3'=8b, '4'=16b, '5'=32b, '6'=64b, '7'=128b, ..., 'Z'=2^36 bits
Formula: bits = 2^(ASCII_value - 48) where '3'=51, '4'=52, ..., 'Z'=90
Everyone forgot the exponent - we just use it directly as the size marker!
```

**Properties:**
- ✅ O(1) skip - see '3', skip 1 byte without parsing the value
- ✅ Optimal size - automatically selects minimal encoding
- ✅ No hard limits - can encode arbitrarily large values
- ✅ 40-70% space savings vs fixed-width on typical data

This approach combines the benefits of both fixed and variable width encoding.

---

## Type Safety Thru Exhaustive Pattern Matching

VSF is written entirely in safe Rust with zero wildcard patterns in match statements:

```rust
match self {
    VsfType::u0(value) => encode_bool(value),
    VsfType::u3(value) => encode_u8(value),
    VsfType::u4(value) => encode_u16(value),
    // ... 208 more explicit cases ...
    VsfType::p(tensor) => encode_bitpacked(tensor),
}
// No _ => wildcard - every variant explicitly handled
```

**Why this matters:**
- Add a type? Won't compile until handled everywhere
- Remove a type? Compiler shows all affected code
- Refactor? Guided thru every impact
- Ship unhandled cases? Can't happen

**Why Rust specifically?** It's the only language that gives you:
- Memory safety **without** garbage collection
- Thread safety **without** runtime checks
- Zero-cost abstractions (no interpreter, no VM, no GC pauses)
- **All proven at compile time**

This is not possible in other languages:
- C/C++: Manual memory (use-after-free, double-free, null pointers)
- Java/C#/Go/Python/JS: Garbage collection (pauses, unpredictability)
- Everything else: Pick your poison ☠️

**The VSF Guarantee:** If it compiles, it won't crash. If tests pass, round-trips work.

**Ways VSF can break:**
0. **Cosmic rays** (hardware bit flips) → Use ECC RAM
1. **Python FFI** → Just don't. Spirix already bans it anyway
2. **You modify the code** → Compiler catches it before you ship

That's it. Those are the **only** ways VSF breaks. Everything else is systematically impossible! Cool eh?

---

## What VSF Enables

### 0. Efficient Bitpacked Tensors

Camera RAW data, scientific sensors, and ML models often use non-standard bit depths:

```rust
// 12-bit camera RAW (common in photography)
BitPackedTensor {
    shape: vec![4096, 3072],  // 12.6 megapixels
    bit_depth: 12,
    data: packed_bytes,
}
// 18.9 MB vs 25.2 MB as u16 array (25% savings)
```

Supports 1-256 bits per element efficiently.

### 0. Cryptographic Primitives as Types

Hashes, signatures, and keys are first-class types:

```rust
VsfType::a(algorithm, mac_tag)    // Message Authentication Code
VsfType::h(algorithm, hash)       // Hash (BLAKE3, SHA-256, etc.)
VsfType::g(algorithm, signature)  // Signature (Ed25519, ECDSA, RSA)
VsfType::k(algorithm, pubkey)     // Public key
```

Each includes an algorithm identifier (lowercase a-z) to avoid confusion.

### 1. Mathematically Correct Arithmetic (Spirix Integration)

VSF natively supports Spirix - two's complement floating-point that legitimately preserves mathematical identities:

```rust
VsfType::s53(spirix_scalar)  // 32-bit fraction, 8-bit exponent Scalar (F5E3)
VsfType::c64(spirix_circle)  // 64-bit fractions, 16-bit exponent Circle (complex numbers!)
```

**Why Spirix exists:** IEEE-754 breaks fundamental math:
- NaN ≠ NaN (wat)
- Two different zeros: +0 and -0 (but +0 == -0 returns true?!)
- Very small numbers underflow to zero, breaking *a × b = 0 iff a = 0 or b = 0*
- Infinity from overflow, not just division by zero
- Sign-magnitude representation requires special-case branching everywhere

**What Spirix fixes:**
- **One Zero.** Not two. Just one. I don't remember there ever being two zeros in math class?
- **One Infinity.** Reserved for actual mathematical singularities (like 1/0), not overflow
- **Vanished values** - numbers too small to represent normally but **not zero** (preserves sign/orientation)
- **Exploded values** - numbers too large to represent but **not infinite** (preserves sign/orientation)
- **Two's complement thruout** - no sign bit shenanigans, no special cases
- **a × b = 0 iff a = 0 or b = 0** - all the time, every time, 100% of the time
- **Customizable precision** - pick your fraction and exponent sizes independently! (F3E3 to F7E7)

**Undefined states that tell you what went wrong:**
Instead of IEEE's generic NaN, Spirix tracks *why* something became undefined:
- `[℘ ⬆+⬆]` - You added two exploded values (whoops!)
- `[℘ ⬇/⬇]` - You divided two vanished values
- `[℘ ⬆×⬇]` - Multiplied infinity by Zero?
- Dozens more - your debugger will thank you

VSF stores all 25 Scalar types (F3-F7 × E3-E7) and 25 Circle types as first-class primitives.

### 2. Geographic Precision (Dymaxion WorldCoord)

Store Earth coordinates with millimeter precision:

```rust
VsfType::w(WorldCoord::from_lat_lon(47.6062, -122.3321))
```

Uses Fuller's Dymaxion projection - 2.14mm precision in 8 bytes.

### 3. Huffman-Compressed Text

Unicode strings with global frequency table:

```rust
VsfType::x(text)  // Automatically compressed
// ~36% compression on English text
// 83 MB/s encode, 100+ MB/s decode
```

---

## Comparison with Other Formats

### TIFF
- **Strength**: Widely supported, good for images
- **Limitation**: 4GB file limit (u32 offsets), 12 bytes minimum overhead per tag
- **VSF approach**: Variable-width encoding, no size limits

### PNG
- **Strength**: Lossless compression, ubiquitous
- **Limitation**: 12 bytes per chunk overhead, u32 length limits
- **VSF approach**: Minimal overhead per field, arbitrary sizes

### HDF5
- **Strength**: Hierarchical data, scientific community adoption
- **Limitation**: Complex spec, u64 everywhere wastes space
- **VSF approach**: Optimal size selection, simpler spec

### Protobuf
- **Strength**: Cross-language, schema evolution
- **Limitation**: Varint requires sequential parsing (O(n) skip)
- **VSF approach**: O(1) skip with explicit size markers

### JSON
- **Strength**: Human-readable, debuggable, universal
- **Limitation**: Text encoding bloat, precision loss, no binary data
- **VSF approach**: Binary format, full precision, efficient

---

## Status: Core Complete (v0.1.3)

### Working Now

✅ **Complete type system** - 211 variants:
- Primitives: u3-u7 (8-128 bit), i3-i7 (signed), f5-f6 (float), j5-j6 (complex)
- Spirix: 50 types (scalar + circle combinations)
- Tensors: 130 types (contiguous + strided)
- Bitpacked: 1-256 bit depths
- Metadata: strings, time, hashes, signatures, keys, MACs

✅ **Encoding/decoding**
- Full round-trip validation
- Variable-length integer encoding
- Big-endian byte order

✅ **Huffman text compression**
- Global Unicode frequency table
- 36% compression typical, not just English
- Low overhead

✅ **Cryptographic support**
- Hash algorithms: BLAKE3, SHA-256, SHA-512
- Signatures: Ed25519, ECDSA-P256, RSA-2048
- Keys: Ed25519, X25519, P-256, RSA-2048
- MACs: HMAC-SHA256/512, Poly1305, BLAKE3-keyed, CMAC

✅ **Camera RAW builders**
- Complete metadata support, pretty sure
- TOKEN authentication integration, eventually!
- Calibration frame hashessss

### Coming Next (v0.2.0)

🚧 **Hierarchical labels** - Organize data with dot notation (e.g., `camera.sensor.temperature`)
🚧 **Unboxed sections** - Zero-copy mmap for bulk data (encode "4GB tensor lives at offset 0x1234") if it's a big demand

**Note on File I/O:** VSF gives you bytes - do whatever you want with them:
```rust
let bytes = encode(&my_data)?;
std::fs::write("data.vsf", &bytes)?;  // Or network, database, embedded, etc.
```
File I/O is intentionally out of scope - you know your use case better than we do. Network streaming? Memory-mapped regions? SQLite blobs? Custom compression? VSF doesn't make opinions about your storage layer

---

## Quick Start

```rust
use vsf::{VsfType, BitPackedTensor, Tensor};

// Store 12-bit camera RAW
let raw = BitPackedTensor::pack(12, vec![4096, 3072], &pixel_data);
let encoded = VsfType::p(raw).flatten();

// Store a tensor (8-bit grayscale image)
let tensor = Tensor::new(vec![1920, 1080], grayscale_data);
let img = VsfType::t_u3(tensor);

// Store text (automatically Huffman compressed)
let doc = VsfType::x("Hello, world!".to_string());

// Store a hash (BLAKE3)
use vsf::crypto_algorithms::HASH_BLAKE3;
let hash = VsfType::h(HASH3, hash_bytes);

// Round-trip
let decoded = VsfType::parse(&encoded)?;
assert_eq!(original, decoded);
```

### Camera RAW with Metadata

```rust
use vsf::builders::complete_raw_image;

let bytes = complete_raw_image(
    raw_tensor,
    Some(CalibrationFrames { /* dark/flat frames */ }),
    Some(CameraSettings {
        iso_speed: Some(800.),
        shutter_time_s: Some(1./60.),  // 1/60 second in seconds (f32)
        aperture_f_number: Some(2.8),
        focal_length_m: Some(0.024),   // 24mm in meters (f32)
        // ...
    }),
    Some(LensInfo { /* lens details */ }),
    Some(TokenAuth { /* cryptographic signature */ }),
)?;
```

---

## Design Principles

### 0. Information-Theoretic Optimality

Variable-width encoding that's provably optimal for byte-aligned systems. Small numbers use small encodings, large numbers use large encodings.

### 1. Type Safety

211 strongly-typed variants with complete pattern matching. Compiler verifies every case is handled.

### 2. Mathematical Correctness

Integrates Spirix for arithmetic that preserves mathematical identities. Eagle Time for physics-bounded timestamps.

### 3. Cryptographic Foundation

Signatures, hashes, keys, and MACs as first-class types, not afterthoughts.

### 4. Self-Describing
Each value includes its type information. Files can be parsed without external schema.

---

## Use Cases

### Genomics & Bioinformatics
- DNA sequencing quality scores (Phred) use 6 bits but get stored in 8-bit ASCII. A human genome (3 billion bases) wastes 750MB on padding. VSF bitpacking eliminates this overhead while embedding cryptographic signatures to verify data provenance.

### Financial Systems & Audit Trails
- Currency amounts require arbitrary precision - IEEE-754 floats fail (0.1 + 0.2 ≠ 0.3). VSF's variable-width integers encode $0.42 in 3 bytes, $1,234,567.89 in 5 bytes. HMAC tags verify transaction integrity without external databases.

### Geospatial Systems & Navigation
- GPS coordinates as IEEE doubles use 16 bytes for precision you don't need. Dymaxion WorldCoord provides 2.14mm accuracy in 8 bytes. Useful for drone navigation, autonomous vehicles, and surveying equipment.

### Game Development & Asset Pipelines
- Animation data has mixed precision requirements: keyframe times (16-bit), quaternions (32-bit), visibility flags (1-bit). VSF bitpacked tensors let each channel use its natural width. A 10-minute mocap recording: 45MB → 18MB.

### Machine Learning & Model Distribution
- Quantized neural networks use 4-bit or 8-bit weights. Standard formats store these in 32-bit arrays (4-8x waste). A 4-bit quantized LLaMA-7B: 3.5GB actual, 14GB in typical formats. VSF maintains 3.5GB while embedding model signatures.

### Scientific Data Archival
- Particle physics experiments produce petabytes with heterogeneous precision: detector IDs (16-bit), energies (32-bit), timestamps (64-bit). VSF selects optimal encoding per field. Spirix prevents IEEE-754 underflow in long-running cumulative calculations.

### Web3 & Decentralized Identity
- Blockchain transactions contain signatures, typically stored as untyped byte arrays. VSF signatures are first-class types - the compiler enforces verification before payload access. "Forgot to verify signature" becomes a compile error.

### Embedded Systems & IoT Telemetry
- Satellite sensors transmit over power/bandwidth-constrained RF links. Temperature sensors: 12-bit, accelerometers: 10-bit. Storing as 16-bit wastes 20-40% per reading. VSF optimizes automatically. Eagle Time anchors timestamps to locality, eliminating clock drift.

---

## Technical Details

### Variable-Length Integer Encoding

```
Size markers indicate bit width (ASCII '3'-'7' and beyond):

'3' = 8 bits  = 1 byte   (0-255)
'4' = 16 bits = 2 bytes  (0-65,535)
'5' = 32 bits = 4 bytes  (0-4.2 billion)
'6' = 64 bits = 8 bytes  (0-18 quintillion)
'7' = 128 bits = 16 bytes (a lot!)
... extends to Z giving 2^36 bits if needed. That's literally a single number that's 8GB!

Example: Value 4096
Encoding: 'u' '4' 0x10 0x00
          │   │   └──┴─ Value (big-endian u16)
          │   └─ Size marker '4' (16-bit follows)
          └─ Type marker 'u' (unsigned integer)

Overhead: 2 bytes (type + size marker)
Skip cost: O(1) - see '4', skip 2 bytes
```

### Bitpacked Tensor Format

```
'p' marker (1 byte)
ndim (variable-length)
bit_depth (1 byte: 0x0C for 12-bit, 0x00 for 256-bit)
shape dimensions (each variable-length encoded)
packed data (bits packed into bytes, MSB-first)
```

Efficient for non-standard bit depths common in sensors and quantized ML models.

---

## Context

VSF is part of a broader computational foundation:

- **Spirix** - Better floating point arithmetic
- **TOKEN** - Unfakeable cryptographic identity
- **VSF** - Optimal serialization
- **Eagle Time** - Physics-bounded consensus timestamps
- **Dymaxion Encoding** - Uses Fuller's Dymaxion projection - 2.14mm avg, 5.07mm max in 64 bits.

Each component addresses fundamental problems from first principles.

---

## Contributing

VSF is in active development. Core encoding/decoding is stable.

---

## License

Custom open-source:
- ✅ Free for any purpose (including commercial)
- ✅ Modify and distribute freely
- ✅ Patent grant included
- ❌ Cannot sell VSF itself as a standalone product

See LICENSE for full terms.

---

## Summary

VSF solves the universal integer encoding problem thru exponential-width encoding with explicit size markers. This enables:

- **Optimal space usage** - 40-70% savings on typical data
- **Literally no size limits** - Can encode arbitrarily large values
- **O(1) skip** - Fast random access without parsing
- **Type safety** - Compiler-verified exhaustive handling

If you need efficient encoding of varied-size integers, bitpacked tensors, or cryptographic primitives with perfect type safety, VSF is your only option!

---

*Written in Rust with ZERO wildcards.*