use num_complex::Complex;
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
use super::tensor::{BitPackedTensor, StridedTensor, Tensor};
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
/// ## Metadata
/// - `x`: Unicode string
/// - `e`: Eagle Time
/// - `d`: Data type name
/// - `l`: Label
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
    // F3 (i8 fraction)
    s33(ScalarF3E3),
    s34(ScalarF3E4),
    s35(ScalarF3E5),
    s36(ScalarF3E6),
    s37(ScalarF3E7),
    // F4 (i16 fraction)
    s43(ScalarF4E3),
    s44(ScalarF4E4),
    s45(ScalarF4E5),
    s46(ScalarF4E6),
    s47(ScalarF4E7),
    // F5 (i32 fraction)
    s53(ScalarF5E3),
    s54(ScalarF5E4),
    s55(ScalarF5E5),
    s56(ScalarF5E6),
    s57(ScalarF5E7),
    // F6 (i64 fraction)
    s63(ScalarF6E3),
    s64(ScalarF6E4),
    s65(ScalarF6E5),
    s66(ScalarF6E6),
    s67(ScalarF6E7),
    // F7 (i128 fraction)
    s73(ScalarF7E3),
    s74(ScalarF7E4),
    s75(ScalarF7E5),
    s76(ScalarF7E6),
    s77(ScalarF7E7),

    // ==================== SPIRIX CIRCLES ====================
    // F3 (i8 fraction)
    c33(CircleF3E3),
    c34(CircleF3E4),
    c35(CircleF3E5),
    c36(CircleF3E6),
    c37(CircleF3E7),
    // F4 (i16 fraction)
    c43(CircleF4E3),
    c44(CircleF4E4),
    c45(CircleF4E5),
    c46(CircleF4E6),
    c47(CircleF4E7),
    // F5 (i32 fraction)
    c53(CircleF5E3),
    c54(CircleF5E4),
    c55(CircleF5E5),
    c56(CircleF5E6),
    c57(CircleF5E7),
    // F6 (i64 fraction)
    c63(CircleF6E3),
    c64(CircleF6E4),
    c65(CircleF6E5),
    c66(CircleF6E6),
    c67(CircleF6E7),
    // F7 (i128 fraction)
    c73(CircleF7E3),
    c74(CircleF7E4),
    c75(CircleF7E5),
    c76(CircleF7E6),
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
    t_s33(Tensor<ScalarF3E3>),
    t_s34(Tensor<ScalarF3E4>),
    t_s35(Tensor<ScalarF3E5>),
    t_s36(Tensor<ScalarF3E6>),
    t_s37(Tensor<ScalarF3E7>),
    // F4 (i16 fraction)
    t_s43(Tensor<ScalarF4E3>),
    t_s44(Tensor<ScalarF4E4>),
    t_s45(Tensor<ScalarF4E5>),
    t_s46(Tensor<ScalarF4E6>),
    t_s47(Tensor<ScalarF4E7>),
    // F5 (i32 fraction)
    t_s53(Tensor<ScalarF5E3>),
    t_s54(Tensor<ScalarF5E4>),
    t_s55(Tensor<ScalarF5E5>),
    t_s56(Tensor<ScalarF5E6>),
    t_s57(Tensor<ScalarF5E7>),
    // F6 (i64 fraction)
    t_s63(Tensor<ScalarF6E3>),
    t_s64(Tensor<ScalarF6E4>),
    t_s65(Tensor<ScalarF6E5>),
    t_s66(Tensor<ScalarF6E6>),
    t_s67(Tensor<ScalarF6E7>),
    // F7 (i128 fraction)
    t_s73(Tensor<ScalarF7E3>),
    t_s74(Tensor<ScalarF7E4>),
    t_s75(Tensor<ScalarF7E5>),
    t_s76(Tensor<ScalarF7E6>),
    t_s77(Tensor<ScalarF7E7>),

    // ==================== SPIRIX CIRCLE TENSORS ====================
    // F3 (i8 fraction)
    t_c33(Tensor<CircleF3E3>),
    t_c34(Tensor<CircleF3E4>),
    t_c35(Tensor<CircleF3E5>),
    t_c36(Tensor<CircleF3E6>),
    t_c37(Tensor<CircleF3E7>),
    // F4 (i16 fraction)
    t_c43(Tensor<CircleF4E3>),
    t_c44(Tensor<CircleF4E4>),
    t_c45(Tensor<CircleF4E5>),
    t_c46(Tensor<CircleF4E6>),
    t_c47(Tensor<CircleF4E7>),
    // F5 (i32 fraction)
    t_c53(Tensor<CircleF5E3>),
    t_c54(Tensor<CircleF5E4>),
    t_c55(Tensor<CircleF5E5>),
    t_c56(Tensor<CircleF5E6>),
    t_c57(Tensor<CircleF5E7>),
    // F6 (i64 fraction)
    t_c63(Tensor<CircleF6E3>),
    t_c64(Tensor<CircleF6E4>),
    t_c65(Tensor<CircleF6E5>),
    t_c66(Tensor<CircleF6E6>),
    t_c67(Tensor<CircleF6E7>),
    // F7 (i128 fraction)
    t_c73(Tensor<CircleF7E3>),
    t_c74(Tensor<CircleF7E4>),
    t_c75(Tensor<CircleF7E5>),
    t_c76(Tensor<CircleF7E6>),
    t_c77(Tensor<CircleF7E7>),

    // ==================== STRIDED TENSORS (WITH STRIDE) ====================
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
    q_s33(StridedTensor<ScalarF3E3>),
    q_s34(StridedTensor<ScalarF3E4>),
    q_s35(StridedTensor<ScalarF3E5>),
    q_s36(StridedTensor<ScalarF3E6>),
    q_s37(StridedTensor<ScalarF3E7>),
    // F4 (i16 fraction)
    q_s43(StridedTensor<ScalarF4E3>),
    q_s44(StridedTensor<ScalarF4E4>),
    q_s45(StridedTensor<ScalarF4E5>),
    q_s46(StridedTensor<ScalarF4E6>),
    q_s47(StridedTensor<ScalarF4E7>),
    // F5 (i32 fraction)
    q_s53(StridedTensor<ScalarF5E3>),
    q_s54(StridedTensor<ScalarF5E4>),
    q_s55(StridedTensor<ScalarF5E5>),
    q_s56(StridedTensor<ScalarF5E6>),
    q_s57(StridedTensor<ScalarF5E7>),
    // F6 (i64 fraction)
    q_s63(StridedTensor<ScalarF6E3>),
    q_s64(StridedTensor<ScalarF6E4>),
    q_s65(StridedTensor<ScalarF6E5>),
    q_s66(StridedTensor<ScalarF6E6>),
    q_s67(StridedTensor<ScalarF6E7>),
    // F7 (i128 fraction)
    q_s73(StridedTensor<ScalarF7E3>),
    q_s74(StridedTensor<ScalarF7E4>),
    q_s75(StridedTensor<ScalarF7E5>),
    q_s76(StridedTensor<ScalarF7E6>),
    q_s77(StridedTensor<ScalarF7E7>),

    // ==================== SPIRIX CIRCLE STRIDED TENSORS ====================
    // F3 (i8 fraction)
    q_c33(StridedTensor<CircleF3E3>),
    q_c34(StridedTensor<CircleF3E4>),
    q_c35(StridedTensor<CircleF3E5>),
    q_c36(StridedTensor<CircleF3E6>),
    q_c37(StridedTensor<CircleF3E7>),
    // F4 (i16 fraction)
    q_c43(StridedTensor<CircleF4E3>),
    q_c44(StridedTensor<CircleF4E4>),
    q_c45(StridedTensor<CircleF4E5>),
    q_c46(StridedTensor<CircleF4E6>),
    q_c47(StridedTensor<CircleF4E7>),
    // F5 (i32 fraction)
    q_c53(StridedTensor<CircleF5E3>),
    q_c54(StridedTensor<CircleF5E4>),
    q_c55(StridedTensor<CircleF5E5>),
    q_c56(StridedTensor<CircleF5E6>),
    q_c57(StridedTensor<CircleF5E7>),
    // F6 (i64 fraction)
    q_c63(StridedTensor<CircleF6E3>),
    q_c64(StridedTensor<CircleF6E4>),
    q_c65(StridedTensor<CircleF6E5>),
    q_c66(StridedTensor<CircleF6E6>),
    q_c67(StridedTensor<CircleF6E7>),
    // F7 (i128 fraction)
    q_c73(StridedTensor<CircleF7E3>),
    q_c74(StridedTensor<CircleF7E4>),
    q_c75(StridedTensor<CircleF7E5>),
    q_c76(StridedTensor<CircleF7E6>),
    q_c77(StridedTensor<CircleF7E7>),

    // ==================== BITPACKED TENSORS ====================
    p(BitPackedTensor), // Bitpacked tensor (1-256 bits per sample)

    // ==================== METADATA & SPECIAL TYPES ====================
    x(String),     // Unicode text
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
    d(String),      // Data type name
    l(String),      // Label
    o(usize),       // Offset in Bytes
    b(usize, bool), // Length in Bytes (value, inclusive_mode)
    n(usize),       // Number/count
    z(usize),       // Version
    y(usize),       // Backward version
    m(usize),       // Marker

    // ==================== CRYPTOGRAPHIC TYPES ====================
    // Hash algorithms - store (length-1) as single byte, then data
    hb3(Vec<u8>), // BLAKE3 hash (u3 length: 1-256 Bytes)
    hb4(Vec<u8>), // BLAKE3 hash (u4 length: 257-65536 Bytes)
    h23(Vec<u8>), // SHA-256 hash (u3 length: always 32 Bytes)
    h53(Vec<u8>), // SHA-512 hash (u3 length: always 64 Bytes)

    // Signature algorithms - store (length-1) as single byte, then data
    ge3(Vec<u8>), // Ed25519 signature (u3 length: always 64 Bytes)
    gp3(Vec<u8>), // ECDSA-P256 signature (u3 length: always 64 Bytes)
    gr4(Vec<u8>), // RSA-2048 signature (u4 length: always 256 Bytes)

    // Cryptographic keys - store (length-1) as single byte, then data
    ke3(Vec<u8>), // Ed25519 public key (u3 length: always 32 Bytes)
    kx3(Vec<u8>), // X25519 key (u3 length: always 32 Bytes)
    kp3(Vec<u8>), // ECDSA-P256 key (u3 length: always 32 Bytes)
    kc3(Vec<u8>), // ChaCha20-Poly1305 key (u3 length: always 32 Bytes)
    ka3(Vec<u8>), // AES-256-GCM key (u3 length: always 32 Bytes)

    // MAC (Message Authentication Code) - store (length-1) as single byte, then data
    ah3(Vec<u8>), // HMAC-SHA256 (u3 length: always 32 Bytes)
    as3(Vec<u8>), // HMAC-SHA512 (u3 length: always 64 Bytes)
    ap3(Vec<u8>), // Poly1305 (u3 length: always 16 Bytes)
    ab3(Vec<u8>), // BLAKE3-keyed (u3 length: 1-256 Bytes, default 32)
    ac3(Vec<u8>), // CMAC-AES (u3 length: always 16 Bytes)

    // ==================== WRAPPED/ENCODED DATA (OPTIONAL) ====================
    /// Wrapped/encoded VSF data with compression, error correction, or encryption
    ///
    /// Format: v[algorithm][encoded_data]
    ///
    /// Algorithm identifiers (single ASCII character):
    /// - 'z' = zstd compression
    /// - 'r' = Reed-Solomon error correction
    /// - 'x' = XZ/LZMA compression
    /// - 'e' = Encryption (algorithm-specific)
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
    /// This is OPTIONAL - core VSF doesn't require it.
    /// Use when your application needs compression, error correction, or encryption.
    v(u8, Vec<u8>), // Wrapped data (algorithm byte, encoded Bytes)
}
