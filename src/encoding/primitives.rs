use super::traits::{EncodeNumber, EncodeNumberInclusive};

// ==================== UNSIGNED INTEGER ENCODING ====================

impl EncodeNumber for u8 {
    fn encode_number(&self) -> Vec<u8> {
        vec![b'3', *self]
    }
}

impl EncodeNumber for u16 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'4'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for u32 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'5'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for u64 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'6'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for u128 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'7'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for usize {
    fn encode_number(&self) -> Vec<u8> {
        // Auto-select smallest size that fits
        if *self <= u8::MAX as usize {
            vec![b'3', *self as u8]
        } else if *self <= u16::MAX as usize {
            let mut result = vec![b'4'];
            result.extend_from_slice(&(*self as u16).to_be_bytes());
            result
        } else if *self <= u32::MAX as usize {
            let mut result = vec![b'5'];
            result.extend_from_slice(&(*self as u32).to_be_bytes());
            result
        } else if *self <= u64::MAX as usize {
            let mut result = vec![b'6'];
            result.extend_from_slice(&(*self as u64).to_be_bytes());
            result
        } else {
            let mut result = vec![b'7'];
            result.extend_from_slice(&(*self as u128).to_be_bytes());
            result
        }
    }
}

impl EncodeNumberInclusive for usize {
    fn encode_usize_inclusive(&self) -> Vec<u8> {
        let base = 8; // 'u' marker: 8 bits
        let mut size = 8; // Size marker: 8 bits initially
        let mut result = vec![b'u'];

        // Try u3 (8-bit)
        let mut adder = base + size; // 16 bits overhead
        let mut cutoff = 0x80 - adder + 0x80;
        if *self < cutoff {
            result.push(b'3');
            result.push((*self + adder) as u8);
            return result;
        }

        // Try u4 (16-bit)
        size = size * 2;
        adder = base + size; // 24 bits overhead
        cutoff = u16::MAX as usize - adder; // FIX: use u16 max for u4
        if *self <= cutoff {
            result.push(b'4');
            result.extend_from_slice(&((*self + adder) as u16).to_be_bytes());
            return result;
        }

        // Try u5 (32-bit)
        size = size * 2;
        adder = base + size; // 40 bits overhead
        cutoff = u32::MAX as usize - adder; // FIX: use u32 max for u5
        if *self <= cutoff {
            result.push(b'5');
            result.extend_from_slice(&((*self + adder) as u32).to_be_bytes());
            return result;
        }

        // Try u6 (64-bit)
        size = size * 2;
        adder = base + size; // 72 bits overhead
        cutoff = u64::MAX as usize - adder; // FIX: use u64 max for u6
        if *self <= cutoff {
            result.push(b'6');
            result.extend_from_slice(&((*self + adder) as u64).to_be_bytes());
            return result;
        }

        // Try u7 (128-bit)
        size = size * 2;
        adder = base + size; // 136 bits overhead
        cutoff = u128::MAX as usize - adder; // FIX: use u128 max for u7
        if *self <= cutoff {
            result.push(b'7');
            result.extend_from_slice(&((*self + adder) as u128).to_be_bytes());
            return result;
        }

        // Try u8 (256-bit) - not yet implemented
        size = size * 2;
        adder = base + size;
        cutoff = u128::MAX as usize - adder;
        if *self <= cutoff {
            result.push(b'8');
            result.extend_from_slice(&((*self + adder) as u128).to_be_bytes());
            return result;
        }

        panic!("Value too large for inclusive encoding (u8 not implemented)");
    }
}

// ==================== SIGNED INTEGER ENCODING ====================

impl EncodeNumber for i8 {
    fn encode_number(&self) -> Vec<u8> {
        vec![b'3', *self as u8]
    }
}

impl EncodeNumber for i16 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'4'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for i32 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'5'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for i64 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'6'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for i128 {
    fn encode_number(&self) -> Vec<u8> {
        let mut result = vec![b'7'];
        result.extend_from_slice(&self.to_be_bytes());
        result
    }
}

impl EncodeNumber for isize {
    fn encode_number(&self) -> Vec<u8> {
        // Auto-select smallest size that fits
        // For positive values, use unsigned limits
        // For negative values, use signed limits
        if *self >= 0 {
            if *self <= u8::MAX as isize {
                vec![b'3', *self as u8]
            } else if *self <= u16::MAX as isize {
                let mut result = vec![b'4'];
                result.extend_from_slice(&(*self as i16).to_be_bytes());
                result
            } else if *self <= u32::MAX as isize {
                let mut result = vec![b'5'];
                result.extend_from_slice(&(*self as i32).to_be_bytes());
                result
            } else if *self <= u64::MAX as isize {
                let mut result = vec![b'6'];
                result.extend_from_slice(&(*self as i64).to_be_bytes());
                result
            } else {
                let mut result = vec![b'7'];
                result.extend_from_slice(&(*self as i128).to_be_bytes());
                result
            }
        } else {
            if *self >= i8::MIN as isize {
                vec![b'3', *self as u8]
            } else if *self >= i16::MIN as isize {
                let mut result = vec![b'4'];
                result.extend_from_slice(&(*self as i16).to_be_bytes());
                result
            } else if *self >= i32::MIN as isize {
                let mut result = vec![b'5'];
                result.extend_from_slice(&(*self as i32).to_be_bytes());
                result
            } else if *self >= i64::MIN as isize {
                let mut result = vec![b'6'];
                result.extend_from_slice(&(*self as i64).to_be_bytes());
                result
            } else {
                let mut result = vec![b'7'];
                result.extend_from_slice(&(*self as i128).to_be_bytes());
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_u8() {
        assert_eq!(42u8.encode_number(), vec![b'3', 42]);
        assert_eq!(255u8.encode_number(), vec![b'3', 255]);
    }

    #[test]
    fn test_encode_u16() {
        assert_eq!(300u16.encode_number(), vec![b'4', 0x01, 0x2C]);
        assert_eq!(4096u16.encode_number(), vec![b'4', 0x10, 0x00]);
    }

    #[test]
    fn test_encode_usize_auto() {
        // Should pick smallest size
        assert_eq!(42usize.encode_number(), vec![b'3', 42]);
        assert_eq!(300usize.encode_number(), vec![b'4', 0x01, 0x2C]);
        assert_eq!(
            100000usize.encode_number(),
            vec![b'5', 0x00, 0x01, 0x86, 0xA0]
        );
    }

    #[test]
    fn test_encode_inclusive() {
        // Value 256 in inclusive mode
        // overhead for u4 = 24 bits = 3 bytes
        // 256 + 24 = 280 = 0x0118
        let result = 256usize.encode_usize_inclusive();
        assert_eq!(result[0], b'u');
        assert_eq!(result[1], b'4');
        // 280 in big-endian
        assert_eq!(result[2], 0x01);
        assert_eq!(result[3], 0x18);
    }

    #[test]
    fn test_encode_isize_positive() {
        assert_eq!(42isize.encode_number(), vec![b'3', 42]);
    }

    #[test]
    fn test_encode_isize_negative() {
        assert_eq!((-42isize).encode_number(), vec![b'3', 0xD6]); // -42 as two's complement
    }
}
