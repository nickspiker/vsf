use num_complex::Complex;

#[cfg(feature = "spirix")]
use spirix::{
    // All 25 valid Circle combinations (F3-F7 × E3-E7)
    CircleF3E3,
    CircleF3E4,
    CircleF3E5,
    CircleF3E6,
    CircleF3E7,
    CircleF4E3,
    CircleF4E4,
    CircleF4E5,
    CircleF4E6,
    CircleF4E7,
    CircleF5E3,
    CircleF5E4,
    CircleF5E5,
    CircleF5E6,
    CircleF5E7,
    CircleF6E3,
    CircleF6E4,
    CircleF6E5,
    CircleF6E6,
    CircleF6E7,
    CircleF7E3,
    CircleF7E4,
    CircleF7E5,
    CircleF7E6,
    CircleF7E7,
    // All 25 valid Scalar combinations (F3-F7 × E3-E7)
    ScalarF3E3,
    ScalarF3E4,
    ScalarF3E5,
    ScalarF3E6,
    ScalarF3E7,
    ScalarF4E3,
    ScalarF4E4,
    ScalarF4E5,
    ScalarF4E6,
    ScalarF4E7,
    ScalarF5E3,
    ScalarF5E4,
    ScalarF5E5,
    ScalarF5E6,
    ScalarF5E7,
    ScalarF6E3,
    ScalarF6E4,
    ScalarF6E5,
    ScalarF6E6,
    ScalarF6E7,
    ScalarF7E3,
    ScalarF7E4,
    ScalarF7E5,
    ScalarF7E6,
    ScalarF7E7,
};

use super::eagle_time_type::EtType;
use super::tensor::{BitPackedTensor, StridedTensor, Tensor, Vector};
use super::world_coord::WorldCoord;

/// Main VSF type enum representing all supported data types
///
/// # Type System (V2)
///
/// ## Primitives
/// - `u0`, `u`, `u3`-`u7`: Unsigned integers (bool, auto, u8, u16, u32, u64, u128)
/// - `i`, `i3`-`i7`: Signed integers (auto, i8, i16, i32, i64, i128)
/// - `f5`, `f6`: IEEE 754 floats (f32, f64)
/// - `j5`, `j6`: IEEE 754 complex (Complex<f32>, Complex<f64>)
///
/// ## Spirix Types
/// - `s33`-`s77`: Spirix Scalars (25 F×E combinations)
/// - `c33`-`c77`: Spirix Circles (25 F×E combinations)
///
/// ## Tensors (Dynamic Dimensionality)
/// - `t_*`: Contiguous tensors (row-major, any number of dimensions)
/// - `q_*`: Strided tensors (explicit stride, any dimensions)
///
/// Element types supported in tensors:
/// - Primitives: u0, u3-u7, i3-i7, f5-f6, j5-j6
/// - Spirix: s33-s77, c33-c77 (common types)
///
/// ## Text Types
/// VSF provides three distinct text types for different use cases:
///
/// - `d`: Dictionary key - Internal naming (section names, field names, dictionary keys)
/// - `x`: UTF-8 text - User-facing text with full Unicode support
/// - `l`: ASCII text - User-facing text restricted to ASCII characters
///
/// Example usage in named fields:
/// ```text
/// (d"name":l"nick")      // Field name uses 'd', ASCII value uses 'l'
/// (d"greeting":x"Hello 世界")  // Field name uses 'd', Unicode value uses 'x'
/// ```
///
/// ## Other Metadata
/// - `e`: Eagle Time
/// - `o`: Offset in bits
/// - `b`: Length in bits
/// - `n`: Number/count
/// - `z`: Version
/// - `y`: Backward version
/// - `m`: Marker definition
/// - `r`: Marker reference
/// - `a`: Message Authentication Code (MAC)
/// - `h`: Hash
/// - `g`: Signature
/// - `k`: Cryptographic key
/// - `w`: World coordinate (Dymaxion icosahedral)
/// - `v`: Wrapped/encoded data (optional: compression, error correction, encryption)
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum VsfType {
    // ==================== UNSIGNED INTEGERS ====================
    u0(bool),       // Boolean
    u(usize, bool), // Auto-sized unsigned (value, inclusive_mode)
    u3(u8),         // 8-bit unsigned
    u4(u16),        // 16-bit unsigned
    u5(u32),        // 32-bit unsigned
    u6(u64),        // 64-bit unsigned
    u7(u128),       // 128-bit unsigned

    // ==================== SIGNED INTEGERS ====================
    i(isize), // Auto-sized signed
    i3(i8),   // 8-bit signed
    i4(i16),  // 16-bit signed
    i5(i32),  // 32-bit signed
    i6(i64),  // 64-bit signed
    i7(i128), // 128-bit signed

    // ==================== IEEE FLOATS ====================
    f5(f32), // 32-bit float
    f6(f64), // 64-bit float

    // ==================== IEEE COMPLEX ====================
    j5(Complex<f32>), // Complex<f32>
    j6(Complex<f64>), // Complex<f64>

    // ==================== SPIRIX SCALARS ====================
    #[cfg(feature = "spirix")]
    // F3 (i8 fraction)
    s33(ScalarF3E3),
    #[cfg(feature = "spirix")]
    s34(ScalarF3E4),
    #[cfg(feature = "spirix")]
    s35(ScalarF3E5),
    #[cfg(feature = "spirix")]
    s36(ScalarF3E6),
    #[cfg(feature = "spirix")]
    s37(ScalarF3E7),
    #[cfg(feature = "spirix")]
    // F4 (i16 fraction)
    s43(ScalarF4E3),
    #[cfg(feature = "spirix")]
    s44(ScalarF4E4),
    #[cfg(feature = "spirix")]
    s45(ScalarF4E5),
    #[cfg(feature = "spirix")]
    s46(ScalarF4E6),
    #[cfg(feature = "spirix")]
    s47(ScalarF4E7),
    #[cfg(feature = "spirix")]
    // F5 (i32 fraction)
    s53(ScalarF5E3),
    #[cfg(feature = "spirix")]
    s54(ScalarF5E4),
    #[cfg(feature = "spirix")]
    s55(ScalarF5E5),
    #[cfg(feature = "spirix")]
    s56(ScalarF5E6),
    #[cfg(feature = "spirix")]
    s57(ScalarF5E7),
    #[cfg(feature = "spirix")]
    // F6 (i64 fraction)
    s63(ScalarF6E3),
    #[cfg(feature = "spirix")]
    s64(ScalarF6E4),
    #[cfg(feature = "spirix")]
    s65(ScalarF6E5),
    #[cfg(feature = "spirix")]
    s66(ScalarF6E6),
    #[cfg(feature = "spirix")]
    s67(ScalarF6E7),
    #[cfg(feature = "spirix")]
    // F7 (i128 fraction)
    s73(ScalarF7E3),
    #[cfg(feature = "spirix")]
    s74(ScalarF7E4),
    #[cfg(feature = "spirix")]
    s75(ScalarF7E5),
    #[cfg(feature = "spirix")]
    s76(ScalarF7E6),
    #[cfg(feature = "spirix")]
    s77(ScalarF7E7),

    // ==================== SPIRIX CIRCLES ====================
    #[cfg(feature = "spirix")]
    // F3 (i8 fraction)
    c33(CircleF3E3),
    #[cfg(feature = "spirix")]
    c34(CircleF3E4),
    #[cfg(feature = "spirix")]
    c35(CircleF3E5),
    #[cfg(feature = "spirix")]
    c36(CircleF3E6),
    #[cfg(feature = "spirix")]
    c37(CircleF3E7),
    #[cfg(feature = "spirix")]
    // F4 (i16 fraction)
    c43(CircleF4E3),
    #[cfg(feature = "spirix")]
    c44(CircleF4E4),
    #[cfg(feature = "spirix")]
    c45(CircleF4E5),
    #[cfg(feature = "spirix")]
    c46(CircleF4E6),
    #[cfg(feature = "spirix")]
    c47(CircleF4E7),
    #[cfg(feature = "spirix")]
    // F5 (i32 fraction)
    c53(CircleF5E3),
    #[cfg(feature = "spirix")]
    c54(CircleF5E4),
    #[cfg(feature = "spirix")]
    c55(CircleF5E5),
    #[cfg(feature = "spirix")]
    c56(CircleF5E6),
    #[cfg(feature = "spirix")]
    c57(CircleF5E7),
    #[cfg(feature = "spirix")]
    // F6 (i64 fraction)
    c63(CircleF6E3),
    #[cfg(feature = "spirix")]
    c64(CircleF6E4),
    #[cfg(feature = "spirix")]
    c65(CircleF6E5),
    #[cfg(feature = "spirix")]
    c66(CircleF6E6),
    #[cfg(feature = "spirix")]
    c67(CircleF6E7),
    #[cfg(feature = "spirix")]
    // F7 (i128 fraction)
    c73(CircleF7E3),
    #[cfg(feature = "spirix")]
    c74(CircleF7E4),
    #[cfg(feature = "spirix")]
    c75(CircleF7E5),
    #[cfg(feature = "spirix")]
    c76(CircleF7E6),
    #[cfg(feature = "spirix")]
    c77(CircleF7E7),

    // ==================== CONTIGUOUS TENSORS (DYNAMIC DIMS) ====================
    // Primitive element types
    t_u0(Tensor<bool>),
    t_u3(Tensor<u8>),
    t_u4(Tensor<u16>),
    t_u5(Tensor<u32>),
    t_u6(Tensor<u64>),
    t_u7(Tensor<u128>),
    t_i3(Tensor<i8>),
    t_i4(Tensor<i16>),
    t_i5(Tensor<i32>),
    t_i6(Tensor<i64>),
    t_i7(Tensor<i128>),
    t_f5(Tensor<f32>),
    t_f6(Tensor<f64>),
    t_j5(Tensor<Complex<f32>>),
    t_j6(Tensor<Complex<f64>>),

    // ==================== SPIRIX SCALAR TENSORS ====================
    // F3 (i8 fraction)
    #[cfg(feature = "spirix")]
    t_s33(Tensor<ScalarF3E3>),
    #[cfg(feature = "spirix")]
    t_s34(Tensor<ScalarF3E4>),
    #[cfg(feature = "spirix")]
    t_s35(Tensor<ScalarF3E5>),
    #[cfg(feature = "spirix")]
    t_s36(Tensor<ScalarF3E6>),
    #[cfg(feature = "spirix")]
    t_s37(Tensor<ScalarF3E7>),
    // F4 (i16 fraction)
    #[cfg(feature = "spirix")]
    t_s43(Tensor<ScalarF4E3>),
    #[cfg(feature = "spirix")]
    t_s44(Tensor<ScalarF4E4>),
    #[cfg(feature = "spirix")]
    t_s45(Tensor<ScalarF4E5>),
    #[cfg(feature = "spirix")]
    t_s46(Tensor<ScalarF4E6>),
    #[cfg(feature = "spirix")]
    t_s47(Tensor<ScalarF4E7>),
    // F5 (i32 fraction)
    #[cfg(feature = "spirix")]
    t_s53(Tensor<ScalarF5E3>),
    #[cfg(feature = "spirix")]
    t_s54(Tensor<ScalarF5E4>),
    #[cfg(feature = "spirix")]
    t_s55(Tensor<ScalarF5E5>),
    #[cfg(feature = "spirix")]
    t_s56(Tensor<ScalarF5E6>),
    #[cfg(feature = "spirix")]
    t_s57(Tensor<ScalarF5E7>),
    // F6 (i64 fraction)
    #[cfg(feature = "spirix")]
    t_s63(Tensor<ScalarF6E3>),
    #[cfg(feature = "spirix")]
    t_s64(Tensor<ScalarF6E4>),
    #[cfg(feature = "spirix")]
    t_s65(Tensor<ScalarF6E5>),
    #[cfg(feature = "spirix")]
    t_s66(Tensor<ScalarF6E6>),
    #[cfg(feature = "spirix")]
    t_s67(Tensor<ScalarF6E7>),
    // F7 (i128 fraction)
    #[cfg(feature = "spirix")]
    t_s73(Tensor<ScalarF7E3>),
    #[cfg(feature = "spirix")]
    t_s74(Tensor<ScalarF7E4>),
    #[cfg(feature = "spirix")]
    t_s75(Tensor<ScalarF7E5>),
    #[cfg(feature = "spirix")]
    t_s76(Tensor<ScalarF7E6>),
    #[cfg(feature = "spirix")]
    t_s77(Tensor<ScalarF7E7>),

    // ==================== SPIRIX CIRCLE TENSORS ====================
    // F3 (i8 fraction)
    #[cfg(feature = "spirix")]
    t_c33(Tensor<CircleF3E3>),
    #[cfg(feature = "spirix")]
    t_c34(Tensor<CircleF3E4>),
    #[cfg(feature = "spirix")]
    t_c35(Tensor<CircleF3E5>),
    #[cfg(feature = "spirix")]
    t_c36(Tensor<CircleF3E6>),
    #[cfg(feature = "spirix")]
    t_c37(Tensor<CircleF3E7>),
    // F4 (i16 fraction)
    #[cfg(feature = "spirix")]
    t_c43(Tensor<CircleF4E3>),
    #[cfg(feature = "spirix")]
    t_c44(Tensor<CircleF4E4>),
    #[cfg(feature = "spirix")]
    t_c45(Tensor<CircleF4E5>),
    #[cfg(feature = "spirix")]
    t_c46(Tensor<CircleF4E6>),
    #[cfg(feature = "spirix")]
    t_c47(Tensor<CircleF4E7>),
    // F5 (i32 fraction)
    #[cfg(feature = "spirix")]
    t_c53(Tensor<CircleF5E3>),
    #[cfg(feature = "spirix")]
    t_c54(Tensor<CircleF5E4>),
    #[cfg(feature = "spirix")]
    t_c55(Tensor<CircleF5E5>),
    #[cfg(feature = "spirix")]
    t_c56(Tensor<CircleF5E6>),
    #[cfg(feature = "spirix")]
    t_c57(Tensor<CircleF5E7>),
    // F6 (i64 fraction)
    #[cfg(feature = "spirix")]
    t_c63(Tensor<CircleF6E3>),
    #[cfg(feature = "spirix")]
    t_c64(Tensor<CircleF6E4>),
    #[cfg(feature = "spirix")]
    t_c65(Tensor<CircleF6E5>),
    #[cfg(feature = "spirix")]
    t_c66(Tensor<CircleF6E6>),
    #[cfg(feature = "spirix")]
    t_c67(Tensor<CircleF6E7>),
    // F7 (i128 fraction)
    #[cfg(feature = "spirix")]
    t_c73(Tensor<CircleF7E3>),
    #[cfg(feature = "spirix")]
    t_c74(Tensor<CircleF7E4>),
    #[cfg(feature = "spirix")]
    t_c75(Tensor<CircleF7E5>),
    #[cfg(feature = "spirix")]
    t_c76(Tensor<CircleF7E6>),
    #[cfg(feature = "spirix")]
    t_c77(Tensor<CircleF7E7>),

    // ==================== STRIDED TENSORS ====================
    // Primitive element types
    q_u0(StridedTensor<bool>),
    q_u3(StridedTensor<u8>),
    q_u4(StridedTensor<u16>),
    q_u5(StridedTensor<u32>),
    q_u6(StridedTensor<u64>),
    q_u7(StridedTensor<u128>),
    q_i3(StridedTensor<i8>),
    q_i4(StridedTensor<i16>),
    q_i5(StridedTensor<i32>),
    q_i6(StridedTensor<i64>),
    q_i7(StridedTensor<i128>),
    q_f5(StridedTensor<f32>),
    q_f6(StridedTensor<f64>),
    q_j5(StridedTensor<Complex<f32>>),
    q_j6(StridedTensor<Complex<f64>>),

    // ==================== SPIRIX SCALAR STRIDED TENSORS ====================
    // F3 (i8 fraction)
    #[cfg(feature = "spirix")]
    q_s33(StridedTensor<ScalarF3E3>),
    #[cfg(feature = "spirix")]
    q_s34(StridedTensor<ScalarF3E4>),
    #[cfg(feature = "spirix")]
    q_s35(StridedTensor<ScalarF3E5>),
    #[cfg(feature = "spirix")]
    q_s36(StridedTensor<ScalarF3E6>),
    #[cfg(feature = "spirix")]
    q_s37(StridedTensor<ScalarF3E7>),
    // F4 (i16 fraction)
    #[cfg(feature = "spirix")]
    q_s43(StridedTensor<ScalarF4E3>),
    #[cfg(feature = "spirix")]
    q_s44(StridedTensor<ScalarF4E4>),
    #[cfg(feature = "spirix")]
    q_s45(StridedTensor<ScalarF4E5>),
    #[cfg(feature = "spirix")]
    q_s46(StridedTensor<ScalarF4E6>),
    #[cfg(feature = "spirix")]
    q_s47(StridedTensor<ScalarF4E7>),
    // F5 (i32 fraction)
    #[cfg(feature = "spirix")]
    q_s53(StridedTensor<ScalarF5E3>),
    #[cfg(feature = "spirix")]
    q_s54(StridedTensor<ScalarF5E4>),
    #[cfg(feature = "spirix")]
    q_s55(StridedTensor<ScalarF5E5>),
    #[cfg(feature = "spirix")]
    q_s56(StridedTensor<ScalarF5E6>),
    #[cfg(feature = "spirix")]
    q_s57(StridedTensor<ScalarF5E7>),
    // F6 (i64 fraction)
    #[cfg(feature = "spirix")]
    q_s63(StridedTensor<ScalarF6E3>),
    #[cfg(feature = "spirix")]
    q_s64(StridedTensor<ScalarF6E4>),
    #[cfg(feature = "spirix")]
    q_s65(StridedTensor<ScalarF6E5>),
    #[cfg(feature = "spirix")]
    q_s66(StridedTensor<ScalarF6E6>),
    #[cfg(feature = "spirix")]
    q_s67(StridedTensor<ScalarF6E7>),
    // F7 (i128 fraction)
    #[cfg(feature = "spirix")]
    q_s73(StridedTensor<ScalarF7E3>),
    #[cfg(feature = "spirix")]
    q_s74(StridedTensor<ScalarF7E4>),
    #[cfg(feature = "spirix")]
    q_s75(StridedTensor<ScalarF7E5>),
    #[cfg(feature = "spirix")]
    q_s76(StridedTensor<ScalarF7E6>),
    #[cfg(feature = "spirix")]
    q_s77(StridedTensor<ScalarF7E7>),

    // ==================== SPIRIX CIRCLE STRIDED TENSORS ====================
    // F3 (i8 fraction)
    #[cfg(feature = "spirix")]
    q_c33(StridedTensor<CircleF3E3>),
    #[cfg(feature = "spirix")]
    q_c34(StridedTensor<CircleF3E4>),
    #[cfg(feature = "spirix")]
    q_c35(StridedTensor<CircleF3E5>),
    #[cfg(feature = "spirix")]
    q_c36(StridedTensor<CircleF3E6>),
    #[cfg(feature = "spirix")]
    q_c37(StridedTensor<CircleF3E7>),
    // F4 (i16 fraction)
    #[cfg(feature = "spirix")]
    q_c43(StridedTensor<CircleF4E3>),
    #[cfg(feature = "spirix")]
    q_c44(StridedTensor<CircleF4E4>),
    #[cfg(feature = "spirix")]
    q_c45(StridedTensor<CircleF4E5>),
    #[cfg(feature = "spirix")]
    q_c46(StridedTensor<CircleF4E6>),
    #[cfg(feature = "spirix")]
    q_c47(StridedTensor<CircleF4E7>),
    // F5 (i32 fraction)
    #[cfg(feature = "spirix")]
    q_c53(StridedTensor<CircleF5E3>),
    #[cfg(feature = "spirix")]
    q_c54(StridedTensor<CircleF5E4>),
    #[cfg(feature = "spirix")]
    q_c55(StridedTensor<CircleF5E5>),
    #[cfg(feature = "spirix")]
    q_c56(StridedTensor<CircleF5E6>),
    #[cfg(feature = "spirix")]
    q_c57(StridedTensor<CircleF5E7>),
    // F6 (i64 fraction)
    #[cfg(feature = "spirix")]
    q_c63(StridedTensor<CircleF6E3>),
    #[cfg(feature = "spirix")]
    q_c64(StridedTensor<CircleF6E4>),
    #[cfg(feature = "spirix")]
    q_c65(StridedTensor<CircleF6E5>),
    #[cfg(feature = "spirix")]
    q_c66(StridedTensor<CircleF6E6>),
    #[cfg(feature = "spirix")]
    q_c67(StridedTensor<CircleF6E7>),
    // F7 (i128 fraction)
    #[cfg(feature = "spirix")]
    q_c73(StridedTensor<CircleF7E3>),
    #[cfg(feature = "spirix")]
    q_c74(StridedTensor<CircleF7E4>),
    #[cfg(feature = "spirix")]
    q_c75(StridedTensor<CircleF7E5>),
    #[cfg(feature = "spirix")]
    q_c76(StridedTensor<CircleF7E6>),
    #[cfg(feature = "spirix")]
    q_c77(StridedTensor<CircleF7E7>),

    // ==================== BITPACKED TENSORS ====================
    p(BitPackedTensor), // Bitpacked tensor (1-256 bits per sample)

    // ==================== METADATA & SPECIAL TYPES ====================
    x(String),     // UTF-8 text (Unicode, user-facing, Huffman compressed)
    e(EtType),     // Eagle Time
    w(WorldCoord), // World coordinate (Dymaxion icosahedral)

    // ==================== COLOUR TYPES ====================
    /// VSF Colour Encoding
    ///
    /// # Format Overview
    ///
    /// ## 0. General Format: `r[channels][depth][data]`
    /// - **channels**: Single byte base-36 digit (0-9, A-Z) = 0-35 channels
    /// - **depth**: Single byte digit (0-9) where bits_per_channel = 2^depth
    ///   - 0 → 1 bit, 1 → 2 bits, 2 → 4 bits, 3 → 8 bits
    ///   - 4 → 16 bits, 5 → 32 bits, 6 → 64 bits, 7 → 128 bits, 8 → 256 bits, 9 → 512 bits. Uppercase letters might be used for depths > 9 or float types in future versions.
    /// - **data**: channel_count × (2^depth / 8) bytes
    ///
    /// Examples:
    /// - `r33[3 bytes]` = 3 channels × 8 bits = RGB
    /// - `r45[8 bytes]` = 4 channels × 32 bits = RGBA (integer)
    /// - `rG3[16 bytes]` = 16 channels × 8 bits = multispectral
    ///
    /// ## 1. Named Shortcuts (zero-data, 2 bytes total)
    /// - `rb` = Blue       - `rc` = Cyan      - `rg` = Middle grey (50%)
    /// - `rj` = Magenta    - `rk` = Black     - `rl` = Lime
    /// - `rn` = Green      - `ro` = Orange    - `rq` = Aqua
    /// - `rr` = Red        - `rv` = Violet    - `rw` = White
    /// - `ry` = Yellow
    ///
    /// ## 2. Format Shortcuts (with data, where {#} indicates size in Bytes)
    /// Greyscale:
    /// - `re{1}` = 8-bit greyscale
    /// - `rx{2}` = 16-bit greyscale
    /// - `rz{4}` = 32-bit float greyscale
    ///
    /// Packed RGB:
    /// - `ri{1}` = 8-bit packed RGB (6×7×6): `((R*7)+G)*6+B` where R∈[0,5], G∈[0,6], B∈[0,5]
    /// - `rp{2}` = 16-bit packed RGB (5-6-5): `RRRRR GGGGGG BBBBB` (bit-aligned)
    ///
    /// Standard RGB/RGBA:
    /// - `ru{3}` = 24-bit RGB (8 bits per channel)
    /// - `rs{6}` = 48-bit RGB (16 bits per channel)
    /// - `rf{12}` = 96-bit RGB (32-bit float × 3)
    /// - `ra{4}` = 32-bit RGBA (8 bits per channel)
    /// - `rt{8}` = 64-bit RGBA (16 bits per channel)
    /// - `rh{16}` = 128-bit RGBA (32-bit float × 4)
    ///
    /// ## 3. Magic Matrix: `rm[f5][N][3]{matrix_data}{gamma}`
    /// Colour transform matrix: N input channels → 3 LMS outputs
    /// - Format follows tensor notation: 'f' '5' [N] [3] [N×3×4 bytes matrix] [4 bytes gamma]
    /// - Matrix: N×3 f32 values (4 bytes each)
    /// - Gamma: Single f32 value (4 bytes), no type prefix
    /// - Total: 2 + 2 + size(N) + size(3) + (N×3×4) + 4 bytes
    ///
    /// # Examples
    /// ```ignore
    /// // Purple using 6×7×6 packing: ri{0x83}
    /// // RGB = (130, 0, 255) → (3, 0, 5) → ((3*7)+0)*6+5 = 131 = 0x83
    ///
    /// // Standard RGB red: ru{255, 0, 0}
    /// // RGBA semi-transparent blue: ra{0, 0, 255, 128}
    /// // 16-channel spectral: rG3[16 bytes]
    /// ```
    // General format colour
    r(u8, u8, Vec<u8>), // r(channels_base36, depth_exp, data)

    // Named shortcuts (zero-data)
    rb, // Blue
    rc, // Cyan
    rg, // Grey
    rj, // Magenta
    rk, // Black
    rl, // Lime
    rn, // Green
    ro, // Orange
    rq, // Aqua
    rr, // Red
    rv, // Purple
    rw, // White
    ry, // Yellow

    // Format shortcuts (with data)
    re(u8),       // 8-bit greyscale
    rx(u16),      // 16-bit greyscale
    rz(f32),      // 32-bit float greyscale
    ri(u8),       // 8-bit packed RGB (6×7×6)
    rp(u16),      // 16-bit packed RGB (5-6-5)
    ru([u8; 3]),  // 24-bit RGB (8bpc)
    rs([u16; 3]), // 48-bit RGB (16bpc)
    rf([f32; 3]), // 96-bit RGB (32f×3)
    ra([u8; 4]),  // 32-bit RGBA (8bpc)
    rt([u16; 4]), // 64-bit RGBA (16bpc)
    rh([f32; 4]), // 128-bit RGBA (32f×4)

    // Magic matrix colour transform
    rm(usize, usize, Vec<f32>, f32), // rm(input_channels, output_channels, matrix_NxM, gamma)
    // Where:
    // input_channels - Number of input colour channels (N)
    // output_channels - Number of output colour channels (M, usually 3 for LMS)
    // matrix_NxM - Flattened N×M matrix as Vec<f32>
    // gamma - Gamma correction value as f32

    // VSF Structure
    d(String),      // Dictionary key (internal naming: section names, field names, keys)
    l(String),      // ASCII text (user-facing, ASCII-only alternative to x)
    o(usize),       // Offset in Bytes
    b(usize, bool), // Length in Bytes (value, inclusive_mode)
    n(usize),       // Number/count
    z(usize),       // Version
    y(usize),       // Backward version
    m(usize),       // Marker

    // ==================== CRYPTOGRAPHIC TYPES ====================
    // Hash algorithms
    hp(Vec<u8>), // BLAKE3 provenance hash (immutable content identity)
    hb(Vec<u8>), // BLAKE3 rolling hash (current file state)
    hs(Vec<u8>), // SHA hash (SHA-256, SHA-512, etc.)

    // Signature algorithms
    ge(Vec<u8>), // Ed25519 signature
    gp(Vec<u8>), // ECDSA-P256 signature
    #[deprecated(
        since = "0.1.7",
        note = "RSA is legacy - prefer Ed25519 for new applications"
    )]
    gr(Vec<u8>), // RSA signature (deprecated)

    // Cryptographic keys
    ke(Vec<u8>), // Ed25519 public key (32B)
    kx(Vec<u8>), // X25519 public key (32B)
    kp(Vec<u8>), // ECDSA/ECDH P-curve keys (33/65B P-256, 49/97B P-384 - size disambiguates)
    kk(Vec<u8>), // secp256k1 public key (33B compressed)
    kc(Vec<u8>), // ChaCha20-Poly1305 symmetric key (32B)
    ka(Vec<u8>), // AES-256-GCM symmetric key (32B)
    km(Vec<u8>), // ML-KEM public key (800/1184/1568B for 512/768/1024 - size disambiguates)
    kf(Vec<u8>), // FrodoKEM public key (9616/15632/21520B for 640/976/1344 - size disambiguates)
    kl(Vec<u8>), // Classic McEliece public key (up to ~1MB - size disambiguates variant)

    // Shared secret (output of key agreement/decapsulation)
    ks(Vec<u8>), // Shared secret / derived key material (typically 32B)

    // MAC (Message Authentication Code)
    ah(Vec<u8>), // HMAC-SHA256 (32B)
    at(Vec<u8>), // HMAC-SHA512 (64B) - 't' for "twelve-eight" (512 bits)
    ap(Vec<u8>), // Poly1305 (16B)
    ab(Vec<u8>), // BLAKE3-keyed (variable, default 32B)
    ac(Vec<u8>), // CMAC-AES (16B)

    // ==================== VECTORS (1D CONTIGUOUS) ====================
    /// 1D contiguous vectors - compact encoding with count marker
    /// Binary format: [t][n][count][type][data...]
    /// Where n indicates 1D (count) vs multi-dimensional (shape)
    /// Primitive element types
    v_u0(Vector<bool>), // Bit-packed bools (8 per byte)
    v_u3(Vector<u8>),
    v_u4(Vector<u16>),
    v_u5(Vector<u32>),
    v_u6(Vector<u64>),
    v_u7(Vector<u128>),
    v_i3(Vector<i8>),
    v_i4(Vector<i16>),
    v_i5(Vector<i32>),
    v_i6(Vector<i64>),
    v_i7(Vector<i128>),
    v_f5(Vector<f32>),
    v_f6(Vector<f64>),
    v_j5(Vector<Complex<f32>>),
    v_j6(Vector<Complex<f64>>),

    // ==================== WRAPPED/ENCODED DATA ====================
    /// Wrapped/encoded VSF data with compression, error correction, encryption, units or other encoding
    ///
    /// Format: v[encoding][encoded_data]
    ///
    /// Encoding identifiers (single ASCII character):
    /// - 'a' = AV1 video codec (image/video compression)
    /// - 'z' = zstd compression
    /// - 'r' = Reed-Solomon error correction
    /// - 'x' = XZ/LZMA compression
    /// - 'e' = Encryption (algorithm-specific)
    /// - 'u' = Measurement in specified units
    ///
    /// Example usage:
    /// ```ignore
    /// // Compress VSF Bytes with zstd
    /// let original = VsfType::t_u3(tensor);
    /// let compressed = compress_zstd(&original.flatten());
    /// let wrapped = VsfType::v(b'z', compressed);
    ///
    /// // Can nest wrappers (compress then error-correct)
    /// let inner = VsfType::v(b'z', compressed_bytes);
    /// let outer = VsfType::v(b'r', reed_solomon_encode(&inner.flatten()));
    /// ```
    ///
    /// Use when your application needs compression, error correction, encryption, units or other encoding.
    v(u8, Vec<u8>), // Wrapped data (encoding byte, encoded Bytes)
}
