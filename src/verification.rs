//! VSF verification functions for hashing and signing
//!
//! This module provides standalone functions for adding cryptographic verification
//! to VSF files after they've been built. Two independent strategies are supported:
//!
//! **Strategy 1: Full File Hash**
//! - Single hash in header covering entire file
//! - Simple integrity check for archives
//! - Use `add_file_hash()` function
//!
//! **Strategy 2: Per-Section Hash/Signature**
//! - Hash/signature stored in label definition
//! - Signs only specific sections (e.g., lock image data, allow metadata edits)
//! - Use `add_section_hash()` or `sign_section()` functions
//!
//! # Example
//! ```ignore
//! use vsf::builders::RawImageBuilder;
//! use vsf::verification::{add_file_hash, add_section_hash, sign_section};
//!
//! // Build the VSF
//! let bytes = raw.build()?;
//!
//! // Add verification as needed
//! let bytes = add_file_hash(bytes)?;                    // Strategy 1
//! let bytes = add_section_hash(bytes, "raw")?;          // Strategy 2 (hash)
//! let bytes = sign_section(bytes, "raw", &key)?;        // Strategy 2 (sign)
//! ```

use crate::crypto_algorithms::HASH_BLAKE3;
use crate::decoding::parse;
use crate::types::VsfType;

/// Add a full-file BLAKE3 hash to the VSF header (Strategy 1)
///
/// This function:
/// 1. Checks if a file hash already exists in the header
/// 2. If not, rebuilds the header with `hb3[32][zeros]` placeholder
/// 3. Hashes the entire file (with zeros in place)
/// 4. Writes the computed hash into the placeholder position
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns
/// Modified VSF bytes with file hash in header
///
/// # Example
/// ```ignore
/// let raw_bytes = raw.build()?;
/// let verified_bytes = add_file_hash(raw_bytes)?;
/// ```
pub fn add_file_hash(mut vsf_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    // Verify magic number
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }
    if &vsf_bytes[0..3] != "RÅ".as_bytes() || vsf_bytes[3] != b'<' {
        return Err("Invalid VSF magic number".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length
    let header_length_type = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bits = match header_length_type {
        VsfType::b(bits) => bits,
        _ => return Err("Expected b type for header length".to_string()),
    };

    // Parse version and backward compat
    let _version =
        parse(&vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Check if file hash already exists
    let hash_position = pointer;
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'h' {
        // Hash already exists, parse it
        let hash_type = parse(&vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse existing hash: {}", e))?;

        if let VsfType::h(alg, hash_bytes) = hash_type {
            if alg != HASH_BLAKE3 {
                return Err(format!(
                    "Unexpected hash algorithm: expected BLAKE3 ({}), found {}",
                    HASH_BLAKE3, alg
                ));
            }
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }

            // Zero out the 32 hash bytes (keep marker 'h' and algorithm and size)
            let hash_value_start = hash_position + 1 + 1 + 1 + 1; // 'h' + alg + '[' + size + ']'
                                                                  // Actually we need to find where the hash bytes start after the marker encoding
                                                                  // Let me re-parse to find exact position

            // For now, let's rebuild with zeros, hash, and write back
            let mut temp_bytes = vsf_bytes.clone();

            // Find the hash value bytes (after 'h', alg byte, size encoding)
            let hash_start = find_hash_value_position(&temp_bytes, hash_position)?;

            // Zero out the 32 hash bytes
            for i in 0..32 {
                temp_bytes[hash_start + i] = 0;
            }

            // Compute BLAKE3 hash of entire file
            let computed_hash = blake3::hash(&temp_bytes);

            // Write hash into the placeholder
            vsf_bytes[hash_start..hash_start + 32].copy_from_slice(computed_hash.as_bytes());

            return Ok(vsf_bytes);
        }
    }

    // No hash exists, need to rebuild header with hash placeholder
    // This is more complex - we need to:
    // 1. Parse the entire header
    // 2. Rebuild it with hash placeholder after version/backward
    // 3. Adjust all offsets in label definitions
    // 4. Hash the file
    // 5. Write hash into placeholder

    Err("Adding file hash to existing VSF without hash not yet implemented. Build with placeholder first.".to_string())
}

/// Find the position of hash value bytes within the encoded hash type
fn find_hash_value_position(data: &[u8], hash_marker_pos: usize) -> Result<usize, String> {
    // Re-parse the hash to find where the value bytes start
    let mut pos = hash_marker_pos;

    // Parse the hash using the decode function
    let hash_type = parse(data, &mut pos).map_err(|e| {
        format!(
            "Failed to parse hash at position {}: {}",
            hash_marker_pos, e
        )
    })?;

    match hash_type {
        VsfType::h(_, hash_bytes) => {
            // pos now points AFTER the hash
            // Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected hash type".to_string()),
    }
}

/// Add a BLAKE3 hash to a specific section's label definition (Strategy 2)
///
/// This function:
/// 1. Finds the specified section in the header
/// 2. Rebuilds the label definition with `hb3[32][zeros]` if not present
/// 3. Hashes the `{preamble}[section]` bytes only
/// 4. Writes the computed hash into the label definition
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
/// * `section` - Name of the section to hash (e.g., "raw")
///
/// # Returns
/// Modified VSF bytes with section hash in label definition
///
/// # Example
/// ```ignore
/// let bytes = add_section_hash(bytes, "raw")?;
/// ```
pub fn add_section_hash(vsf_bytes: Vec<u8>, section: &str) -> Result<Vec<u8>, String> {
    // TODO: Implement section-specific hashing
    Err("add_section_hash not yet implemented".to_string())
}

/// Sign a specific section with Ed25519 (Strategy 2)
///
/// This function:
/// 1. Finds the specified section in the header
/// 2. Rebuilds the label definition with `g4[64][zeros]` if not present
/// 3. Signs the `{preamble}[section]` bytes only
/// 4. Writes the signature into the label definition
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
/// * `section` - Name of the section to sign (e.g., "raw")
/// * `signing_key` - Ed25519 signing key (32 bytes)
///
/// # Returns
/// Modified VSF bytes with section signature in label definition
///
/// # Example
/// ```ignore
/// let key = load_signing_key()?;
/// let bytes = sign_section(bytes, "raw", &key)?;
/// ```
pub fn sign_section(
    vsf_bytes: Vec<u8>,
    section: &str,
    signing_key: &[u8],
) -> Result<Vec<u8>, String> {
    // TODO: Implement section signing
    Err("sign_section not yet implemented".to_string())
}

/// Verify the full-file hash in a VSF header
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns
/// `Ok(())` if hash is valid, `Err` with description if invalid or missing
pub fn verify_file_hash(vsf_bytes: &[u8]) -> Result<(), String> {
    // Verify magic number
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }
    if &vsf_bytes[0..3] != "RÅ".as_bytes() || vsf_bytes[3] != b'<' {
        return Err("Invalid VSF magic number".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length
    let header_length_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;
    let _header_length_bits = match header_length_type {
        VsfType::b(bits) => bits,
        _ => return Err("Expected b type for header length".to_string()),
    };

    // Parse version and backward compat
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Check if file hash exists
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'h' {
        return Err("No file hash found in header".to_string());
    }

    // Parse the hash
    let hash_type =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse hash: {}", e))?;

    let stored_hash = match hash_type {
        VsfType::h(alg, hash_bytes) => {
            if alg != HASH_BLAKE3 {
                return Err(format!(
                    "Unexpected hash algorithm: expected BLAKE3 ({}), found {}",
                    HASH_BLAKE3, alg
                ));
            }
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }
            hash_bytes
        }
        _ => return Err("Expected hash type in header".to_string()),
    };

    // Create a copy with zeroed hash
    let mut temp_bytes = vsf_bytes.to_vec();
    let hash_start = find_hash_value_position(&temp_bytes, hash_position)?;

    // Zero out the 32 hash bytes
    for i in 0..32 {
        temp_bytes[hash_start + i] = 0;
    }

    // Compute BLAKE3 hash of entire file
    let computed_hash = blake3::hash(&temp_bytes);

    // Compare
    if computed_hash.as_bytes() == stored_hash.as_slice() {
        Ok(())
    } else {
        Err("File hash verification failed: computed hash does not match stored hash".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::RawImageBuilder;
    use crate::types::BitPackedTensor;

    #[test]
    fn test_add_and_verify_file_hash() {
        use crate::file_format::VsfSection;
        use crate::vsf_builder::VsfBuilder;

        // Create a simple VSF file (hash is automatic now)
        let mut section = VsfSection::new("test");
        section.add_item("value", VsfType::u(42, false));

        let builder = VsfBuilder::new()
            .add_section("test", vec![("value".to_string(), VsfType::u(42, false))]);

        let verified_bytes = builder.build().unwrap();

        // The file should have a computed hash (automatic)
        assert!(verified_bytes.len() > 50); // Has header + hash + section

        // Verify the hash
        let result = verify_file_hash(&verified_bytes);
        assert!(result.is_ok(), "Hash verification should succeed");
    }

    #[test]
    fn test_automatic_hash_inclusion() {
        // All VSF files now automatically include a hash - test RAW image
        let samples: Vec<u64> = (0..16).collect();
        let image = BitPackedTensor::pack(8, vec![4, 4], &samples);
        let raw = RawImageBuilder::new(image);
        let bytes = raw.build().unwrap();

        // Hash should be present and valid (automatic)
        let result = verify_file_hash(&bytes);
        assert!(
            result.is_ok(),
            "All VSF files should have valid hash automatically"
        );
    }

    #[test]
    fn test_verify_hash_integrity() {
        // Test that hash actually catches corruption
        let samples: Vec<u64> = (0..16).collect();
        let image = BitPackedTensor::pack(8, vec![4, 4], &samples);
        let raw = RawImageBuilder::new(image);
        let mut bytes = raw.build().unwrap();

        // Corrupt a byte in the data section (not in the hash itself)
        let corruption_index = bytes.len() - 10;
        bytes[corruption_index] ^= 0xFF;

        // Hash verification should fail
        let result = verify_file_hash(&bytes);
        assert!(
            result.is_err(),
            "Corrupted file should fail hash verification"
        );
    }
}
