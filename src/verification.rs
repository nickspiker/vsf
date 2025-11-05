//! VSF verification functions for hashing and signing
//!
//! This module provides standalone functions for adding cryptographic verification
//! to VSF files after they've been built. Two independent strategies are supported:
//!
//! - Single hash in header covering entire file
//! - Simple integrity check for archives
//! - Use `add_file_hash()` function
//!
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
//! ```

use crate::crypto_algorithms::HASH_BLAKE3;
use crate::decoding::parse;
use crate::encoding::traits::EncodeNumber;
use crate::file_format::LabelDefinition;
use crate::types::VsfType;

/// Helper struct for complete header information
struct ParsedHeader {
    version: usize,
    backward_compat: usize,
    labels: Vec<LabelDefinition>,
    header_end: usize, // Byte position where header ends (after '>')
}

/// Parse complete VSF header including all label crypto fields
fn parse_full_header(data: &[u8]) -> Result<ParsedHeader, String> {
    if data.len() < 4 {
        return Err("File too small".to_string());
    }
    if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid magic number".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length (we'll recalculate this when rebuilding)
    let _ =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Parse version
    let version_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let version = match version_type {
        VsfType::z(v) => v,
        _ => return Err("Expected z type for version".to_string()),
    };

    // Parse backward compat
    let backward_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let backward_compat = match backward_type {
        VsfType::y(v) => v,
        _ => return Err("Expected y type for backward compat".to_string()),
    };

    // Skip file hash
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse file hash: {}", e))?;

    // Parse label count
    let label_count_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse label count: {}", e))?;
    let label_count = match label_count_type {
        VsfType::n(count) => count,
        _ => return Err("Expected n type for label count".to_string()),
    };

    // Parse each label
    let mut labels = Vec::new();
    for _ in 0..label_count {
        if data[pointer] != b'(' {
            return Err("Expected '(' for label".to_string());
        }
        pointer += 1;

        // Parse name
        let name_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse label name: {}", e))?;
        let name = match name_type {
            VsfType::d(n) => n,
            _ => return Err("Expected d type for label name".to_string()),
        };

        // Parse optional crypto fields
        let mut hash = None;
        let mut signature = None;
        let mut key = None;
        let mut wrap = None;

        while pointer < data.len() && matches!(data[pointer], b'h' | b'g' | b'k' | b'v') {
            let crypto_type = parse(data, &mut pointer)
                .map_err(|e| format!("Failed to parse crypto field: {}", e))?;
            match crypto_type {
                VsfType::hb3(_) | VsfType::hb4(_) | VsfType::h23(_) | VsfType::h53(_) => {
                    hash = Some(crypto_type)
                }
                VsfType::ge3(_) | VsfType::gp3(_) | VsfType::gr4(_) => {
                    signature = Some(crypto_type)
                }
                VsfType::ke3(_)
                | VsfType::kx3(_)
                | VsfType::kp3(_)
                | VsfType::kc3(_)
                | VsfType::ka3(_) => key = Some(crypto_type),
                VsfType::v(_, _) => wrap = Some(crypto_type),
                _ => {}
            }
        }

        // Parse offset, size, count
        let offset_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse offset: {}", e))?;
        let offset_bytes = match offset_type {
            VsfType::o(bytes) => bytes,
            _ => return Err("Expected o type for offset".to_string()),
        };

        let size_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse size: {}", e))?;
        let size_bytes = match size_type {
            VsfType::b(bytes, _) => bytes,
            _ => return Err("Expected b type for size".to_string()),
        };

        // Child count is optional if encrypted (has wrap field)
        let child_count = if wrap.is_some() {
            // Encrypted blobs have no child count (implied n[0])
            0
        } else {
            // Parse child count
            let count_type =
                parse(data, &mut pointer).map_err(|e| format!("Failed to parse count: {}", e))?;
            match count_type {
                VsfType::n(count) => count,
                _ => return Err("Expected n type for child count".to_string()),
            }
        };

        if data[pointer] != b')' {
            return Err("Expected ')' after label".to_string());
        }
        pointer += 1;

        labels.push(LabelDefinition {
            name,
            hash,
            signature,
            key,
            wrap,
            offset_bytes,
            size_bytes,
            child_count,
        });
    }

    // Find header end '>'
    if data[pointer] != b'>' {
        return Err("Expected '>' at end of header".to_string());
    }
    let header_end = pointer + 1;

    Ok(ParsedHeader {
        version,
        backward_compat,
        labels,
        header_end,
    })
}

/// Rebuild VSF file with modified header labels
fn rebuild_with_header(
    old_data: &[u8],
    mut labels: Vec<LabelDefinition>,
    version: usize,
    backward_compat: usize,
    old_header_end: usize,
) -> Result<Vec<u8>, String> {
    use crate::file_format::VsfHeader;

    let old_header_size = old_header_end;

    // Stabilization loop - iterate until header size and offsets converge
    const MAX_ITERATIONS: usize = 10;
    let mut prev_header_size = old_header_size;

    for _iteration in 0..MAX_ITERATIONS {
        // Calculate what the new header size will be
        let mut test_header = VsfHeader::new(version, backward_compat);
        test_header.file_hash = Some(VsfType::hb3(vec![0u8; 32]));
        for label in &labels {
            test_header.add_label(label.clone());
        }
        let mut test_encoded = test_header.encode()?;
        VsfHeader::update_header_length(&mut test_encoded)?;
        let new_header_size = test_encoded.len();

        // Check if converged
        if new_header_size == prev_header_size {
            // Build final header with these offsets
            let mut final_header = VsfHeader::new(version, backward_compat);
            final_header.file_hash = Some(VsfType::hb3(vec![0u8; 32]));
            for label in labels {
                final_header.add_label(label);
            }
            let mut new_file = final_header.encode()?;
            VsfHeader::update_header_length(&mut new_file)?;

            // Append section data
            new_file.extend_from_slice(&old_data[old_header_end..]);

            // Compute and write file hash
            let hash = compute_file_hash(&new_file)?;
            return write_file_hash(new_file, &hash);
        }

        // Adjust offsets for next iteration
        let offset_adjustment = new_header_size as isize - prev_header_size as isize;

        for label in &mut labels {
            label.offset_bytes = ((label.offset_bytes as isize) + offset_adjustment) as usize;
        }

        prev_header_size = new_header_size;
    }

    Err(format!(
        "Failed to stabilize header after {} iterations",
        MAX_ITERATIONS
    ))
}

/// Compute BLAKE3 hash of VSF file (with hash placeholder zeroed)
///
/// This function computes the file hash WITHOUT modifying the input.
/// It expects the file to already have a hash placeholder (hb3[32][zeros]).
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hash placeholder
///
/// # Returns
/// 32-byte BLAKE3 hash
///
pub fn compute_file_hash(vsf_bytes: &[u8]) -> Result<[u8; 32], String> {
    // Verify magic number
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }
    if &vsf_bytes[0..3] != "RÅ".as_bytes() || vsf_bytes[3] != b'<' {
        return Err("Invalid VSF magic number".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length
    let _header_length_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Parse version and backward compat
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Find hash placeholder
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'h' {
        return Err("No file hash placeholder found".to_string());
    }

    // Parse hash to find position
    let hash_type =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse hash: {}", e))?;

    match hash_type {
        VsfType::hb3(hash_bytes) | VsfType::hb4(hash_bytes) => {
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }

            // Clone file and zero out the hash bytes
            let mut temp_bytes = vsf_bytes.to_vec();
            let hash_start = find_hash_value_position(&temp_bytes, hash_position)?;

            for i in 0..32 {
                temp_bytes[hash_start + i] = 0;
            }

            // Compute BLAKE3 hash of entire file
            let computed_hash = blake3::hash(&temp_bytes);
            Ok(*computed_hash.as_bytes())
        }
        _ => Err("Expected BLAKE3 hash (hb3 or hb4)".to_string()),
    }
}

/// Write computed hash into the file hash placeholder
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hash placeholder
/// * `hash` - 32-byte BLAKE3 hash to write
///
/// # Returns
/// Modified VSF bytes with hash written
///
pub fn write_file_hash(mut vsf_bytes: Vec<u8>, hash: &[u8; 32]) -> Result<Vec<u8>, String> {
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse header length
    let _header_length_type = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Parse version and backward compat
    let _version =
        parse(&vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Find hash placeholder position
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'h' {
        return Err("No file hash placeholder found".to_string());
    }

    // Find the hash value bytes position
    let hash_start = find_hash_value_position(&vsf_bytes, hash_position)?;

    // Write hash into the placeholder
    vsf_bytes[hash_start..hash_start + 32].copy_from_slice(hash);

    Ok(vsf_bytes)
}

/// Legacy function for backward compatibility
///
/// This function combines compute_file_hash and write_file_hash.
/// New code should use the separate functions instead.
///
#[deprecated(note = "Use compute_file_hash() and write_file_hash() separately")]
pub fn add_file_hash(vsf_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let hash = compute_file_hash(&vsf_bytes)?;
    write_file_hash(vsf_bytes, &hash)
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
        VsfType::hb3(hash_bytes) | VsfType::hb4(hash_bytes) => {
            // pos now points AFTER the hash
            // Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected BLAKE3 hash type (hb3 or hb4)".to_string()),
    }
}

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
    Err("add_section_hash not yet implemented".to_string())
}

///
/// This function:
/// 1. Finds the specified section in the header
/// 2. Extracts the section data bytes `[d"name" (fields...)]`
/// 3. Signs those bytes with Ed25519
/// 4. Rebuilds the header with signature in label definition
/// 5. Recomputes file hash
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
/// * `section` - Name of the section to sign (e.g., "raw")
/// * `signing_key` - Ed25519 signing key bytes (must be valid SigningKey)
///
/// # Returns
/// Modified VSF bytes with section signature in label definition
///
/// # Example
/// ```ignore
/// use ed25519_dalek::SigningKey;
/// use rand::rngs::OsRng;
///
/// let signing_key = SigningKey::generate(&mut OsRng);
/// let bytes = sign_section(bytes, "raw", signing_key.as_bytes())?;
/// ```
pub fn sign_section(
    vsf_bytes: Vec<u8>,
    section_name: &str,
    signing_key: &[u8],
) -> Result<Vec<u8>, String> {
    use crate::crypto_algorithms::SIG_ED25519;
    use ed25519_dalek::{Signer, SigningKey};

    // Parse signing key
    let key_bytes: [u8; 32] = signing_key.try_into().map_err(|_| {
        format!(
            "Signing key must be exactly 32 bytes, got {}",
            signing_key.len()
        )
    })?;
    let signing_key = SigningKey::from_bytes(&key_bytes);

    // Parse complete header
    let header = parse_full_header(&vsf_bytes)?;

    // Find target section
    let section_label = header
        .labels
        .iter()
        .find(|l| l.name == section_name)
        .ok_or_else(|| format!("Section '{}' not found", section_name))?;

    let section_offset = section_label.offset_bytes;
    let section_size = section_label.size_bytes;

    // Extract and sign section bytes
    if section_offset + section_size > vsf_bytes.len() {
        return Err("Section exceeds file bounds".to_string());
    }
    let section_bytes = &vsf_bytes[section_offset..section_offset + section_size];
    let signature = signing_key.sign(section_bytes);

    // Create signature VsfType (Ed25519 signature is always 64 bytes)
    let sig_vsf = VsfType::ge3(signature.to_bytes().to_vec());

    // Update labels - add signature to target section
    let mut new_labels = header.labels.clone();
    for label in &mut new_labels {
        if label.name == section_name {
            label.signature = Some(sig_vsf);
            break;
        }
    }

    // Rebuild file with modified header
    rebuild_with_header(
        &vsf_bytes,
        new_labels,
        header.version,
        header.backward_compat,
        header.header_end,
    )
}

/// Add encryption metadata to a section's header label
///
/// This function:
/// 1. Finds the specified section in the header
/// 2. Adds encryption algorithm (v) and key (k) to the label
/// 3. Rebuilds the file with updated header
/// 4. Updates file hash
///
/// data BEFORE building the VSF file. This just adds metadata to the header.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
/// * `section_name` - Name of the section (e.g., "sensitive")
/// * `algorithm` - Encryption algorithm ID (e.g., b'c' for ChaCha20)
/// * `encryption_key` - Encryption key bytes
///
/// # Returns
/// Modified VSF bytes with encryption metadata in label
///
/// # Example
/// ```ignore
/// // 1. Encrypt data first
/// let encrypted_data = encrypt_with_chacha20(&plaintext, &key);
///
/// // 2. Build VSF with encrypted data
/// let vsf = VsfBuilder::new()
///     .add_section("sensitive", vec![("data", encrypted_data)])
///     .build()?;
///
/// // 3. Add encryption metadata to header
/// let vsf = add_encryption_metadata(vsf, "sensitive", b'c', &key)?;
/// ```
pub fn add_encryption_metadata(
    vsf_bytes: Vec<u8>,
    section_name: &str,
    algorithm: u8,
    encryption_key: &[u8],
) -> Result<Vec<u8>, String> {
    // Parse complete header
    let header = parse_full_header(&vsf_bytes)?;

    // Find target section and add encryption metadata
    let mut new_labels = header.labels.clone();
    let mut found = false;

    for label in &mut new_labels {
        if label.name == section_name {
            use crate::crypto_algorithms::{WRAP_AES256_GCM, WRAP_CHACHA20POLY1305};

            // Add wrapped/encrypted marker (v)
            label.wrap = Some(VsfType::v(algorithm, vec![])); // Empty vec, just marks as encrypted

            // Add encryption key based on algorithm
            let key_vsf = match algorithm {
                WRAP_CHACHA20POLY1305 => VsfType::kc3(encryption_key.to_vec()),
                WRAP_AES256_GCM => VsfType::ka3(encryption_key.to_vec()),
                _ => {
                    return Err(format!(
                        "Unsupported encryption algorithm: {}",
                        algorithm as char
                    ))
                }
            };
            label.key = Some(key_vsf);
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!("Section '{}' not found", section_name));
    }

    // Rebuild file with modified header
    rebuild_with_header(
        &vsf_bytes,
        new_labels,
        header.version,
        header.backward_compat,
        header.header_end,
    )
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
        VsfType::b(bits, _) => bits,
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
        VsfType::hb3(hash_bytes) | VsfType::hb4(hash_bytes) => {
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }
            hash_bytes
        }
        _ => return Err("Expected BLAKE3 hash type (hb3 or hb4) in header".to_string()),
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
