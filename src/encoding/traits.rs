/// Trait for encoding numbers into VSF variable-length format
///
/// VSF uses a compact encoding where the size marker indicates the byte count:
/// - '3' = 8 bits (2^3)
/// - '4' = 16 bits (2^4)
/// - '5' = 32 bits (2^5)
/// - '6' = 64 bits (2^6)
/// - '7' = 128 bits (2^7)
///
/// The smallest size that can hold the value is automatically chosen.
pub trait EncodeNumber {
    /// Encode this number into VSF format: [size_marker][value_bytes]
    ///
    /// # Examples
    /// ```ignore
    /// 42u8.encode_number()    → [b'3', 0x2A]
    /// 300u16.encode_number()  → [b'4', 0x01, 0x2C]
    /// 100000u32.encode_number() → [b'5', 0x00, 0x01, 0x86, 0xA0]
    /// ```
    fn encode_number(&self) -> Vec<u8>;
}

/// Trait for encoding numbers in "inclusive" mode
///
/// Inclusive mode is used for self-referential sizes (e.g., header length that includes itself).
/// It adds the encoding overhead to the value before encoding, ensuring that when decoded
/// and the size of the encoding itself is added back, you get the original value.
///
/// # How it works
///
/// For each size tier, there's an overhead:
/// - u3: overhead = 16 bits (2 bytes: 'u' marker + '3' size marker)
/// - u4: overhead = 24 bits (3 bytes: 'u' + '4' + value)
/// - u5: overhead = 40 bits (5 bytes: 'u' + '5' + value)
/// - etc.
///
/// The encoding adds this overhead to the value, so:
/// ```ignore
/// // Normal: 256 → [u][4][0x01, 0x00] (3 bytes)
/// // Inclusive: 256 → 256 + 24 = 280 → [u][4][0x01, 0x18] (3 bytes)
/// // When decoded: 280 - 24 = 256 ✓
/// ```
///
/// This ensures values like 256 can be encoded even when they cause size overflow,
/// and self-referential sizes (like "this header is N bytes including this field") work correctly.
pub trait EncodeNumberInclusive {
    /// Encode this number in inclusive mode (adds encoding overhead to value)
    fn encode_usize_inclusive(&self) -> Vec<u8>;
}
