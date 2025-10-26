# VSF V2 - Complete Architecture Summary

## Core Understanding

VSF (Versatile Storage Format) is a **self-describing, hierarchical, binary format** that combines:
- Type safety (strongly typed primitives & tensors)
- Cryptographic primitives (signatures, hashes built-in)
- Hierarchical organization (unlimited nesting with offset-based seeking)
- Zero-copy sections (unboxed bulk data)
- Mathematical correctness (Spirix integration)
- Temporal anchoring (Eagle Time)

**It's genuinely unique** - nothing else combines these features.

---

## Binary Encoding - The Critical Part I Finally Got

### Variable-Length Integer Encoding

**Every integer uses a size marker followed by value bytes:**

```
Size markers (ASCII characters):
'3' (0x33) = 2^3 = 8 bits  → 1 byte follows
'4' (0x34) = 2^4 = 16 bits → 2 bytes follow
'5' (0x35) = 2^5 = 32 bits → 4 bytes follow
'6' (0x36) = 2^6 = 64 bits → 8 bytes follow
'7' (0x37) = 2^7 = 128 bits → 16 bytes follow
```

**ALWAYS use the smallest size that fits the value.**

**Example: Value 4096**
```
Correct:  0x34 0x10 0x00  (size='4' for 16-bit, then 4096 in big-endian)
Wrong:    0x35 0x00 0x00 0x10 0x00  (wasteful 32-bit encoding)
```

**This is a linear byte stream - no nested brackets, just sequential bytes.**

---

## Type System

### Primitives (Single Values)

**IEEE types (single letter):**
- `u3`-`u7`: Unsigned (u8, u16, u32, u64, u128)
- `i3`-`i7`: Signed (i8, i16, i32, i64, i128)
- `f5`, `f6`: Float (f32, f64)
- `j5`, `j6`: Complex (Complex<f32>, Complex<f64>)

**Spirix types (two letters):**
- `s33`-`s77`: Scalar (25 F×E combinations)
- `c33`-`c77`: Circle (25 F×E combinations)

Example: `s64` = Spirix ScalarF6E4 (64-bit fraction, 16-bit exponent)

---

### Tensors (Multi-Dimensional Arrays)

**Use `Vec<usize>` for shape (NOT const generic)** because:
- Dimensions unknown at parse time
- Simpler enum (15 variants vs 200+)
- Naturally handles any dimension count

```rust
pub struct Tensor<T> {
    pub shape: Vec<usize>,    // [4096, 3072] or [100, 200, 50]
    pub data: Vec<T>,
}

pub struct StridedTensor<T> {
    pub shape: Vec<usize>,
    pub stride: Vec<usize>,   // Explicit memory layout
    pub data: Vec<T>,
}
```

**Two tensor types:**
- `t` (0x74): Contiguous, row-major (95% of cases)
- `q` (0x71): Strided, explicit stride (slices, column-major, weird layouts)

**VsfType enum simplified:**
```rust
pub enum VsfType {
    t_u4(Tensor<u16>),      // Not t1u4, t2u4, t3u4 - just t_u4 handles all dimensions
    t_f6(Tensor<f64>),
    q_u4(StridedTensor<u16>),
    // ... one variant per element type, NOT per dimension
}
```

---

### BitPacked Tensors (CRITICAL FOR LUMIS)

**For non-byte-aligned data (10-bit, 12-bit, 13-bit, etc.):**

```rust
pub struct BitPackedTensor {
    pub shape: Vec<usize>,
    pub bits_per_element: u8,    // 1-64 bits
    pub data: Vec<u8>,           // Packed bytes
}

// VsfType variant
VsfType::bp(BitPackedTensor)
```

**Why needed:**
- Cameras output 10-bit, 12-bit, 14-bit RAW
- Storing 12-bit in u16 wastes 25% space
- Bitpacking: 4096×3072 image = 18.9MB vs 25.2MB

**Binary encoding:**
```
[b][p][bits_per_element][dim_count][shapes...][packed_data]

Example: 12-bit RAW (4096×3072)
0x62 0x70           'b' 'p'
0x33 0x0C           bits: '3' (8-bit), value 12
0x33 0x02           dims: '3', value 2
0x34 0x10 0x00      shape[0]: '4' (16-bit), 4096
0x34 0x0C 0x00      shape[1]: '4' (16-bit), 3072
[18,874,368 bytes]  Packed data
```

---

## Tensor Binary Encoding (The Part That Took Forever)

### Contiguous Tensor Format

```
[t marker: 0x74]
[dim_count: size_marker + value]
[element_type_markers]
[dim_0_size: size_marker + value]
[dim_1_size: size_marker + value]
[dim_N_size: size_marker + value]
[raw_data: packed element bytes]
```

**Lumis RAW example (2D u16, 4096×3072):**
```
0x74                't' marker
0x33 0x02           dim_count: size='3' (8-bit), value=2
0x75 0x34           element: 'u' '4' (unsigned 16-bit)
0x34 0x10 0x00      dim[0]: size='4' (16-bit), value=4096
0x34 0x0C 0x00      dim[1]: size='4' (16-bit), value=3072
[25,165,824 bytes]  Raw pixel data (4096×3072×2 bytes)
```

**Key insights:**
- Dimension count and sizes are encoded (size marker + value)
- Element type markers are NOT encoded (just raw ASCII bytes)
- No type markers on dimension sizes (they're always unsigned)
- Use minimal size that fits each value

### Strided Tensor Format

```
[q marker: 0x71]
[dim_count: encoded]
[element_type_markers]
[shapes: encoded]
[strides: encoded]   ← Additional stride info
[raw_data]
```

---

## File Structure

### Header Format

```
RÅ<                              Magic number (0x52 0xC3 0x85) + '<'
  b[header_length]               Header size in bits (encoded)
  z[version] y[backward_version] Version info (encoded)
  n[label_count]                 Number of sections (encoded)
  
  (d[label_name] o[offset] b[size] n[count])  Label definitions
  (d[label_name] o[offset] b[size] n[count])
  ...
>                                Header end marker

[Data sections at offsets...]
```

### Label Definitions

**Three types of sections:**

1. **Inline nested (small data):**
```
(label: [immediate fields])
```

2. **Referenced sections (large/optional):**
```
(label: o[offset] b[size] n[count])
```
- `n[count] > 0`: Structured data (parse `count` items)
- `n[0]`: Unboxed blob (raw bytes, no structure)

3. **Nested inline structures:**
```
[
  (field1: value1)
  (field2: value2)
  (nested: [
    (subfield: value)
  ])
]
```

---

## Unboxed Bulk Data (CRITICAL OPTIMIZATION)

**Convention: `n[0]` means "raw blob, no parsing"**

```
RÅ
  n[2]
  (d[metadata] o[500] b[1000] n[15])     ← Structured (parse 15 items)
  (d[raw_pixels] o[1500] b[25MB] n[0])   ← Unboxed (just bytes until EOF)
>

[500]: [structured metadata with markers]
[1500]: FFFFFFFFFF... [25MB raw pixels, NO markers, just bytes]
```

**Benefits:**
- Zero-copy mmap() directly
- No parsing overhead
- TIFF-style performance
- Multiple unboxed sections via different offsets

**Reading:**
```rust
if label.count == 0 {
    // Unboxed: raw slice
    &file[offset..offset+size]
} else {
    // Structured: parse with markers
    parse_structured(file, offset, count)
}
```

---

## Hierarchical Structure

**Domains use dot notation:**
```
imaging.raw              ← Lumis RAW sensor data
imaging.photo            ← Processed photos
imaging.video            ← Video frames

medical.patient          ← Demographics
medical.exam             ← Physical exams
medical.lab              ← Lab results
medical.imaging          ← X-rays, CT, MRI

geo.location             ← Coordinates
geo.address              ← Street addresses
geo.boundaries           ← Polygons

token.identity           ← Public keys, trust
token.transaction        ← Balances, transfers
token.network            ← Node reputation
```

**Metadata Registry:**
- Canonical keys: `iso_speed`, `shutter_time_ns`, `focal_length_mm`
- Aliases: `["ISO", "iso", "ISOSpeed"]` all normalize to `iso_speed`
- Type enforcement: `iso_speed` must be `u5`
- Units in name: `_ns`, `_mm`, `_kg`, `_deg`

---

## Cryptographic Primitives

**Built into format:**
- `g(Vec<u8>)`: Signatures (Ed25519, etc.)
- `h(Vec<u8>)`: Hashes (SHA256, Blake3, etc.)

**Each section can be signed:**
```
[
  (data: [...])
  (signature: g[ed25519_sig])
]
```

**Entire file can be signed:**
```
RÅ<header>
[all_data]
(signature: g[file_signature])
```

---

## Eagle Time

**Epoch: 1969-07-20 20:17:40 UTC (Apollo 11 lunar landing)**

```rust
pub enum EtType {
    u(usize), u5(u32), u6(u64), u7(u128),
    i(isize), i5(i32), i6(i64), i7(i128),
    f6(f64),
}
```

**Binary markers:**
- Uppercase `T` + size: Integer seconds (T5, T6, T7)
- Lowercase `t` + size: Float seconds (t6)

**Why Eagle Time:**
- Non-political universal reference
- TOKEN system standard
- Precise temporal anchoring

---

## Version Compatibility

**Two version fields:**
- `z(version)`: Current format version
- `y(backward_version)`: Oldest version that can read this

**Example:**
- File written as V2: `z(2) y(1)`
  - V2 parsers: Full support
  - V1 parsers: Can read, may ignore new types
  - V0 parsers: Cannot read

---

## Parsing Flow

```rust
let mut pointer = 0;

// 1. Magic number
assert_eq!(&data[0..3], b"R\xC3\x85");
pointer = 3;

// 2. Header start
assert_eq!(data[pointer], b'<');
pointer += 1;

// 3. Header length
let header_len = parse(data, &mut pointer)?;  // Advances pointer

// 4. Version info
let version = parse(data, &mut pointer)?;
let backward = parse(data, &mut pointer)?;

// 5. Label count
let label_count = parse(data, &mut pointer)?;

// 6. Label definitions
for _ in 0..label_count {
    assert_eq!(data[pointer], b'(');
    pointer += 1;
    
    let label_name = parse(data, &mut pointer)?;
    let offset = parse(data, &mut pointer)?;
    let size = parse(data, &mut pointer)?;
    let count = parse(data, &mut pointer)?;
    
    assert_eq!(data[pointer], b')');
    pointer += 1;
} 

// 7. Header end
assert_eq!(data[pointer], b'>');
pointer += 1;

// 8. Seek to data sections using offsets
```

**Key: `parse()` function handles variable-length encoding and advances pointer automatically.**

---

## Implementation Priority

**Phase 1 (NOW - Agent's task):**
1. ✅ Create `Tensor<T>` and `StridedTensor<T>` with `Vec<usize>` shape
2. ✅ Create `BitPackedTensor` with pack/unpack functions
3. ✅ Update `VsfType` enum (simplified tensor variants)
4. ✅ Implement `flatten()` for tensors and bitpacked
5. ✅ Implement `parse()` for tensors and bitpacked
6. ✅ Helper function `encode_minimal_size()`
7. ✅ Tests for Lumis RAW (4096×3072×12bit)

**Phase 2 (Later):**
- Metadata registry implementation
- VsfBuilder helper
- Compression wrappers (zstd)
- Spirix tensor support
- Nested structure helpers

---

## Critical Design Decisions Made

1. **Vec<usize> for shape** - Dynamic, not const generic
2. **Two tensor types** - Contiguous (`t`) vs Strided (`q`)
3. **BitPacked is separate type** - Can't use regular tensors for non-byte-aligned
4. **Unboxed bulk data** - `n[0]` convention for zero-copy
5. **Hierarchical domains** - Dot notation for namespace organization
6. **Variable-length encoding** - Size marker (ASCII '3'-'7') + value bytes
7. **Always use minimal size** - 4096 fits in 16-bit, so use '4' not '5'
8. **Big-endian byte order** - Standard for network/file formats
9. **Type markers are raw ASCII** - Not encoded (just 'u' '4', not encoded values)
10. **Dimension/stride sizes ARE encoded** - Each with size marker + value

---

## What Makes VSF Special

vs **Protobuf**: Self-describing, no schema, crypto primitives, seekable
vs **HDF5**: Simpler, crypto, not just scientific, Eagle Time
vs **TIFF**: Not just images, hierarchical, typed tensors, versioned
vs **JSON/XML**: Binary (compact), typed, zero-copy, signable
vs **SQLite**: Hierarchical, mixed media, simpler, no query engine

**VSF is the only format that does ALL of this.**

---

This is everything Agent needs to implement VSF V2 correctly. The bitpacked tensors are TOP priority for Lumis.