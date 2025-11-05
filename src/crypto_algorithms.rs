//! Cryptographic algorithm identifiers for VSF hash, signature, and key types
//!
//! Each type (h, g, k) uses a single lowercase ASCII letter (a-z) to identify
//! the algorithm. This gives 26 slots per type, with most reserved for future use.
//!
//! ## Important: Algorithm vs Output Size
//!
//! Each letter represents a **distinct algorithm**, NOT just a different output size:
//! - SHA-256 (`s`) and SHA-512 (`t`) are **different algorithms** (32-bit vs 64-bit internal words, different round counts)
//! - They are not "the same algorithm with different output lengths"
//!
//! ## Variable-Length Output
//!
//! Some algorithms support variable output lengths while maintaining the same algorithm ID:
//! - **BLAKE3**: Can output any length (default 32 bytes, but can produce 1 byte to 2^64 bytes)
//! - **BLAKE2b**: Can output 1-64 bytes
//! - **SHA-3**: Supports "XOF" (extendable output) variants
//!
//! When storing a hash, the actual output length is encoded in the VSF `h` type's length field.
//! The algorithm ID just tells you which hash function was used.

// ==================== MAC ALGORITHMS (a type) ====================

/// HMAC-SHA256 - RECOMMENDED DEFAULT
///
/// **Algorithm:** Hash-based Message Authentication Code using SHA-256
/// **Fixed output:** 32 bytes (256 bits)
/// **Performance:** Fast, widely supported
/// **Security:** 128-bit security level
/// **Use case:** General-purpose authenticated encryption
pub const MAC_HMAC_SHA256: u8 = b'h';

/// HMAC-SHA512
///
/// **Algorithm:** Hash-based Message Authentication Code using SHA-512
/// **Fixed output:** 64 bytes (512 bits)
/// **Performance:** Fast on 64-bit systems
/// **Security:** 256-bit security level
pub const MAC_HMAC_SHA512: u8 = b's';

/// Poly1305
///
/// **Algorithm:** One-time authenticator (Poly1305-AES or standalone)
/// **Fixed output:** 16 bytes (128 bits)
/// **Performance:** Extremely fast
/// **Security:** 128-bit security level
/// **Use case:** Commonly paired with ChaCha20 for authenticated encryption
/// **Note:** Requires unique nonce per message with same key
pub const MAC_POLY1305: u8 = b'p';

/// BLAKE3 keyed mode
///
/// **Algorithm:** BLAKE3 with keying material (MAC mode)
/// **Default output:** 32 bytes (variable length supported)
/// **Performance:** Extremely fast, modern design
/// **Security:** 128-bit security level at 256-bit output
/// **Note:** BLAKE3's keyed mode is a true MAC, not just HMAC-BLAKE3
pub const MAC_BLAKE3: u8 = b'b';

/// CMAC (AES-based)
///
/// **Algorithm:** Cipher-based Message Authentication Code using AES
/// **Fixed output:** 16 bytes (128 bits)
/// **Security:** 128-bit security level
/// **Use case:** AES-based systems, hardware acceleration available
/// **Note:** Slower than HMAC/Poly1305 but useful when AES is already in use
pub const MAC_CMAC: u8 = b'c';

// Reserved MAC slots: a, d, e, f, g, i, j, k, l, m, n, o, q, r, t, u, v, w, x, y, z

/// Get MAC algorithm name from ID
pub fn mac_algorithm_name(id: u8) -> Option<&'static str> {
    match id {
        MAC_HMAC_SHA256 => Some("HMAC-SHA256"),
        MAC_HMAC_SHA512 => Some("HMAC-SHA512"),
        MAC_POLY1305 => Some("Poly1305"),
        MAC_BLAKE3 => Some("BLAKE3-keyed"),
        MAC_CMAC => Some("CMAC-AES"),
        _ => None,
    }
}

/// Get expected MAC tag length in bytes (None = variable length)
pub fn mac_length(id: u8) -> Option<usize> {
    match id {
        MAC_HMAC_SHA256 => Some(32),
        MAC_HMAC_SHA512 => Some(64),
        MAC_POLY1305 => Some(16),
        MAC_BLAKE3 => Some(32), // Default, but supports variable length
        MAC_CMAC => Some(16),
        _ => None,
    }
}

// ==================== HASH ALGORITHMS (h type) ====================

/// BLAKE3 hash - RECOMMENDED DEFAULT
///
/// **Default output:** 32 bytes
/// **Variable output:** Supports any length from 1 byte to 2^64 bytes
/// **Performance:** Extremely fast, modern design
/// **Security:** Full 128-bit collision resistance at 256-bit output
pub const HASH_BLAKE3: u8 = b'b';

/// SHA-256 hash
///
/// **Algorithm:** SHA-2 family with 32-bit words, 64 rounds
/// **Fixed output:** 32 bytes (256 bits)
/// **Security:** 128-bit collision resistance
/// **Note:** This is NOT just SHA-512 with shorter output - it's a different algorithm
pub const HASH_SHA256: u8 = b's';

/// SHA-512 hash
///
/// **Algorithm:** SHA-2 family with 64-bit words, 80 rounds
/// **Fixed output:** 64 bytes (512 bits)
/// **Security:** 256-bit collision resistance
/// **Note:** On 64-bit processors, can be faster than SHA-256 despite longer output
pub const HASH_SHA512: u8 = b't';

// Reserved hash slots: a, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, u, v, w, x, y, z

/// Get hash algorithm name from ID
pub fn hash_algorithm_name(id: u8) -> Option<&'static str> {
    match id {
        HASH_BLAKE3 => Some("BLAKE3"),
        HASH_SHA256 => Some("SHA-256"),
        HASH_SHA512 => Some("SHA-512"),
        _ => None,
    }
}

/// Get expected hash length in bytes
pub fn hash_length(id: u8) -> Option<usize> {
    match id {
        HASH_BLAKE3 => Some(32),
        HASH_SHA256 => Some(32),
        HASH_SHA512 => Some(64),
        _ => None,
    }
}

// ==================== SIGNATURE ALGORITHMS (g type) ====================

/// Ed25519 signature - RECOMMENDED DEFAULT
///
/// **Algorithm:** Edwards-curve Digital Signature Algorithm using Curve25519
/// **Fixed output:** 64 bytes (512 bits)
/// **Performance:** Very fast signing and verification
/// **Security:** ~128-bit security level
/// **Note:** This is a completely different algorithm from ECDSA, not just a different curve
pub const SIG_ED25519: u8 = b'e';

/// ECDSA P-256 signature
///
/// **Algorithm:** Elliptic Curve Digital Signature Algorithm on NIST P-256 curve
/// **Fixed output:** 64 bytes (two 32-byte integers: r and s)
/// **Security:** ~128-bit security level
/// **Use case:** Required for compliance with certain standards
/// **Note:** Different algorithm family than Ed25519 (ECDSA vs EdDSA)
pub const SIG_ECDSA_P256: u8 = b'p';

/// RSA-2048 signature
///
/// **Algorithm:** RSA with 2048-bit modulus
/// **Fixed output:** 256 bytes (2048 bits)
/// **Security:** ~112-bit security level (2048-bit RSA ≈ 112-bit symmetric)
/// **Performance:** Slower than elliptic curve algorithms
/// **Note:** RSA-2048 and RSA-4096 are the same algorithm with different key sizes,
///          but we give them different IDs because the signature size differs
pub const SIG_RSA_2048: u8 = b'r';

// Reserved signature slots: a, b, c, d, f, g, h, i, j, k, l, m, n, o, q, s, t, u, v, w, x, y, z

/// Get signature algorithm name from ID
pub fn signature_algorithm_name(id: u8) -> Option<&'static str> {
    match id {
        SIG_ED25519 => Some("Ed25519"),
        SIG_ECDSA_P256 => Some("ECDSA-P256"),
        SIG_RSA_2048 => Some("RSA-2048"),
        _ => None,
    }
}

/// Get expected signature length in bytes
pub fn signature_length(id: u8) -> Option<usize> {
    match id {
        SIG_ED25519 => Some(64),
        SIG_ECDSA_P256 => Some(64),
        SIG_RSA_2048 => Some(256),
        _ => None,
    }
}

// ==================== KEY TYPES (k type) ====================

/// Ed25519 public key - RECOMMENDED DEFAULT
///
/// **Algorithm:** Edwards-curve public key for signature verification
/// **Fixed size:** 32 bytes
/// **Use with:** Ed25519 signatures (SIG_ED25519)
/// **Performance:** Very fast signature verification
/// **Note:** Ed25519 keys are NOT compatible with X25519 (different curve operations)
pub const KEY_ED25519: u8 = b'e';

/// X25519 public key
///
/// **Algorithm:** Montgomery curve (Curve25519) public key for Diffie-Hellman key exchange
/// **Fixed size:** 32 bytes
/// **Use case:** ECDH key agreement (deriving shared secrets)
/// **Performance:** Very fast key exchange
/// **Note:** X25519 is NOT for signatures - it's for key agreement only
///          (Even though it uses the same underlying curve as Ed25519)
pub const KEY_X25519: u8 = b'x';

/// ECDSA P-256 public key
///
/// **Algorithm:** NIST P-256 elliptic curve public key
/// **Variable size:** 33 bytes (compressed) or 65 bytes (uncompressed)
/// **Use with:** ECDSA P-256 signatures (SIG_ECDSA_P256)
/// **Security:** ~128-bit security level
/// **Note:** Different curve family than Ed25519/X25519 (Weierstrass vs Edwards/Montgomery)
pub const KEY_P256: u8 = b'p';

/// RSA-2048 public key
///
/// **Algorithm:** RSA public key with 2048-bit modulus
/// **Variable size:** ~270 bytes (depends on encoding format)
/// **Use with:** RSA-2048 signatures (SIG_RSA_2048)
/// **Security:** ~112-bit security level
/// **Note:** RSA-2048 and RSA-4096 are the same algorithm, just different key/modulus sizes
pub const KEY_RSA_2048: u8 = b'r';

// Reserved key slots: a, b, c, d, f, g, h, i, j, k, l, m, n, o, q, s, t, u, v, w, y, z

/// Get key algorithm name from ID
///
/// Note: This handles both asymmetric keys (Ed25519, X25519, etc.) and
/// symmetric encryption keys (ChaCha20-Poly1305, AES-256-GCM, etc.)
pub fn key_algorithm_name(id: u8) -> Option<&'static str> {
    match id {
        KEY_ED25519 => Some("Ed25519"),
        KEY_X25519 => Some("X25519"),
        KEY_P256 => Some("ECDSA-P256"),
        KEY_RSA_2048 => Some("RSA-2048"),
        // Symmetric encryption keys (reuse wrap algorithm IDs)
        WRAP_CHACHA20POLY1305 => Some("ChaCha20-Poly1305"),
        WRAP_AES256_GCM => Some("AES-256-GCM"),
        _ => None,
    }
}

/// Get expected public key length in bytes (None = variable length)
pub fn key_length(id: u8) -> Option<usize> {
    match id {
        KEY_ED25519 => Some(32),
        KEY_X25519 => Some(32),
        KEY_P256 => None,     // Can be 33 (compressed) or 65 (uncompressed)
        KEY_RSA_2048 => None, // Variable, typically ~270 bytes
        _ => None,
    }
}

// ==================== WRAPPING/ENCRYPTION ALGORITHMS (v type) ====================

/// ChaCha20-Poly1305 AEAD encryption - RECOMMENDED DEFAULT
///
/// **Algorithm:** ChaCha20 stream cipher + Poly1305 MAC
/// **Key size:** 32 bytes
/// **Nonce size:** 12 bytes
/// **Tag size:** 16 bytes (Poly1305 authentication tag)
/// **Performance:** Extremely fast, constant-time
/// **Security:** 256-bit key strength
/// **Use case:** General-purpose authenticated encryption
pub const WRAP_CHACHA20POLY1305: u8 = b'c';

/// AES-256-GCM encryption
///
/// **Algorithm:** AES-256 in Galois/Counter Mode
/// **Key size:** 32 bytes
/// **Nonce size:** 12 bytes
/// **Tag size:** 16 bytes (GCM authentication tag)
/// **Security:** 256-bit key strength
/// **Use case:** Hardware-accelerated encryption (AES-NI)
pub const WRAP_AES256_GCM: u8 = b'a';

// Reserved wrapping slots: b, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z

/// Get wrapping algorithm name from ID
pub fn wrap_algorithm_name(id: u8) -> Option<&'static str> {
    match id {
        WRAP_CHACHA20POLY1305 => Some("ChaCha20-Poly1305"),
        WRAP_AES256_GCM => Some("AES-256-GCM"),
        _ => None,
    }
}

/// Get expected key length for wrapping algorithm in bytes
pub fn wrap_key_length(id: u8) -> Option<usize> {
    match id {
        WRAP_CHACHA20POLY1305 => Some(32),
        WRAP_AES256_GCM => Some(32),
        _ => None,
    }
}

// ==================== VALIDATION HELPERS ====================

/// Validate MAC algorithm ID is known
pub fn is_valid_mac_algorithm(id: u8) -> bool {
    mac_algorithm_name(id).is_some()
}

/// Validate hash algorithm ID is known
pub fn is_valid_hash_algorithm(id: u8) -> bool {
    hash_algorithm_name(id).is_some()
}

/// Validate signature algorithm ID is known
pub fn is_valid_signature_algorithm(id: u8) -> bool {
    signature_algorithm_name(id).is_some()
}

/// Validate key algorithm ID is known
pub fn is_valid_key_algorithm(id: u8) -> bool {
    key_algorithm_name(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_algorithms() {
        assert_eq!(mac_algorithm_name(MAC_HMAC_SHA256), Some("HMAC-SHA256"));
        assert_eq!(mac_length(MAC_HMAC_SHA256), Some(32));
        assert!(is_valid_mac_algorithm(MAC_HMAC_SHA256));

        assert_eq!(mac_algorithm_name(MAC_POLY1305), Some("Poly1305"));
        assert_eq!(mac_length(MAC_POLY1305), Some(16));

        assert_eq!(mac_algorithm_name(b'z'), None);
        assert!(!is_valid_mac_algorithm(b'z'));
    }

    #[test]
    fn test_hash_algorithms() {
        assert_eq!(hash_algorithm_name(HASH_BLAKE3), Some("BLAKE3"));
        assert_eq!(hash_length(HASH_BLAKE3), Some(32));
        assert!(is_valid_hash_algorithm(HASH_BLAKE3));

        assert_eq!(hash_algorithm_name(b'z'), None);
        assert!(!is_valid_hash_algorithm(b'z'));
    }

    #[test]
    fn test_signature_algorithms() {
        assert_eq!(signature_algorithm_name(SIG_ED25519), Some("Ed25519"));
        assert_eq!(signature_length(SIG_ED25519), Some(64));
        assert!(is_valid_signature_algorithm(SIG_ED25519));

        assert_eq!(signature_algorithm_name(b'z'), None);
        assert!(!is_valid_signature_algorithm(b'z'));
    }

    #[test]
    fn test_key_algorithms() {
        assert_eq!(key_algorithm_name(KEY_ED25519), Some("Ed25519"));
        assert_eq!(key_length(KEY_ED25519), Some(32));
        assert!(is_valid_key_algorithm(KEY_ED25519));

        assert_eq!(key_algorithm_name(b'z'), None);
        assert!(!is_valid_key_algorithm(b'z'));
    }

    #[test]
    fn test_all_ids_are_lowercase() {
        // Verify all defined IDs are lowercase a-z
        assert!((HASH_BLAKE3 >= b'a') && (HASH_BLAKE3 <= b'z'));
        assert!((HASH_SHA256 >= b'a') && (HASH_SHA256 <= b'z'));

        assert!((SIG_ED25519 >= b'a') && (SIG_ED25519 <= b'z'));
        assert!((SIG_ECDSA_P256 >= b'a') && (SIG_ECDSA_P256 <= b'z'));

        assert!((KEY_ED25519 >= b'a') && (KEY_ED25519 <= b'z'));
        assert!((KEY_X25519 >= b'a') && (KEY_X25519 <= b'z'));
    }
}
