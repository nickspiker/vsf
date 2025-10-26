/// Trait for decoding numbers from VSF variable-length format
///
/// VSF uses a compact encoding where the size marker indicates the byte count:
/// - '3' = 8 bits (2^3)
/// - '4' = 16 bits (2^4)
/// - '5' = 32 bits (2^5)
/// - '6' = 64 bits (2^6)
/// - '7' = 128 bits (2^7)
///
/// The decoder reads the size marker and then reads the appropriate number of bytes.
pub trait DecodeNumber: Sized {
    /// Decode a number from VSF format: [size_marker][value_bytes]
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Invalid size marker
    /// - Not enough bytes in the buffer
    /// - Value doesn't fit in the target type
    fn decode_number(bytes: &[u8]) -> Result<(Self, usize), DecodeError>;
}

/// Trait for decoding numbers in "inclusive" mode
///
/// Inclusive mode is used for self-referential sizes (e.g., header length that includes itself).
/// It subtracts the encoding overhead from the decoded value to get the original.
///
/// # How it works
///
/// For each size tier, there's an overhead that was added during encoding:
/// - u3: overhead = 16 bits (2 bytes: 'u' marker + '3' size marker)
/// - u4: overhead = 24 bits (3 bytes: 'u' + '4' + value)
/// - u5: overhead = 40 bits (5 bytes: 'u' + '5' + value)
/// - etc.
///
/// The decoder reads the encoded value and subtracts the overhead:
/// ```ignore
/// // Encoded: [u][4][0x01, 0x18] → 280
/// // Subtract overhead: 280 - 24 = 256 ✓
/// ```
pub trait DecodeNumberInclusive: Sized {
    /// Decode a number in inclusive mode (subtracts encoding overhead from value)
    ///
    /// Returns the decoded value and the number of bytes consumed.
    fn decode_usize_inclusive(bytes: &[u8]) -> Result<(Self, usize), DecodeError>;
}

/// Errors that can occur during VSF decoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Not enough bytes in the buffer
    UnexpectedEof { expected: usize, got: usize },

    /// Invalid size marker (not '3', '4', '5', '6', or '7')
    InvalidSizeMarker(u8),

    /// Invalid type marker (unexpected first byte)
    InvalidTypeMarker(u8),

    /// Value is too large for the target type
    ValueOutOfRange,

    /// Invalid inclusive encoding (overhead larger than encoded value)
    InvalidInclusive,

    /// Invalid tensor structure
    InvalidTensor(String),

    /// Invalid Spirix type combination
    InvalidSpirix { f: u8, e: u8 },

    /// Generic error with message
    Other(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEof { expected, got } => {
                write!(f, "Unexpected end of file: expected {} bytes, got {}", expected, got)
            }
            DecodeError::InvalidSizeMarker(m) => {
                write!(f, "Invalid size marker: {}", m)
            }
            DecodeError::InvalidTypeMarker(m) => {
                write!(f, "Invalid type marker: {}", *m as char)
            }
            DecodeError::ValueOutOfRange => {
                write!(f, "Value out of range for target type")
            }
            DecodeError::InvalidInclusive => {
                write!(f, "Invalid inclusive encoding")
            }
            DecodeError::InvalidTensor(msg) => {
                write!(f, "Invalid tensor: {}", msg)
            }
            DecodeError::InvalidSpirix { f: frac, e: exp } => {
                write!(f, "Invalid Spirix type: F{}E{}", frac, exp)
            }
            DecodeError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for DecodeError {}
