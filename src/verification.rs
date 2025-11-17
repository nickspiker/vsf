//! VSF verification functions for hashing and signing
//!
//! This module provides standalone functions for adding cryptographic verification
//! to VSF files after they've been built. Two independent strategies are supported:
//!
//! - Single hash in header covering entire file
//! - Simple integrity check for archives
//! - Use `add_file_hash()` function
//!
//! - Hash/signature stored in header field definition
//! - Signs only specific sections (e.g., lock image data, allow metadata edits)
//! - Use `sign_section()` function
//!
//! # Example
//! ```ignore
//! use vsf::builders::RawImageBuilder;
//! use vsf::verification::sign_section;
//!
//! // Build the VSF
//! let bytes = raw.build()?;
//!
//! // Add verification as needed
//! ```

use crate::decoding::parse;
use crate::file_format::HeaderField;
use crate::types::VsfType;

/// Helper struct for complete header information
struct ParsedHeader {
    version: usize,
    backward_compat: usize,
    fields: Vec<HeaderField>,
    header_end: usize, // Byte position where header ends (after '>')
}

/// Parse complete VSF header including all header field crypto metadata
/// Robust, order-independent parser using existing VSF tools
fn parse_full_header(data: &[u8]) -> Result<ParsedHeader, String> {
    if data.len() < 4 {
        return Err("File too small".to_string());
    }
    if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid magic number".to_string());
    }

    let mut ptr = 4; // Skip "RÅ<"

    // Parse version FIRST (determines all encoding decisions)
    let version = match parse(data, &mut ptr).map_err(|e| format!("Failed to parse version: {}", e))? {
        VsfType::z(v) => v,
        _ => return Err("Expected z type for version".to_string()),
    };

    // Parse backward compat
    let backward_compat = match parse(data, &mut ptr).map_err(|e| format!("Failed to parse backward compat: {}", e))? {
        VsfType::y(v) => v,
        _ => return Err("Expected y type for backward compat".to_string()),
    };

    // Parse header length (now we know how to decode it!)
    let _ = parse(data, &mut ptr).map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Skip creation time (required in v4+)
    let _ = parse(data, &mut ptr).map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Parse provenance primitives in FIXED order (version determines format)
    // Always: hp (provenance hash) - may be replaced by ge (signature)
    // Optional: hb (rolling hash)

    // Parse hp or ge (required - one must be present)
    let prov_type = parse(data, &mut ptr).map_err(|e| format!("Failed to parse provenance hash/sig: {}", e))?;
    match prov_type {
        VsfType::hp(_) => {}, // Provenance hash
        VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => {}, // Signature (replaces hp)
        _ => return Err(format!("Expected hp or ge after creation time, got: {:?}", prov_type)),
    }

    // Optional: hb (rolling hash) - only if next byte is 'h'
    if ptr < data.len() && data[ptr] == b'h' {
        let _ = parse(data, &mut ptr).map_err(|e| format!("Failed to parse rolling hash: {}", e))?;
    }

    // Parse header field count
    let field_count = match parse(data, &mut ptr).map_err(|e| format!("Failed to parse field count: {}", e))? {
        VsfType::n(count) => count,
        _ => return Err("Expected n type for field count".to_string()),
    };

    // Parse each header field using helper function
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        fields.push(parse_header_field(data, &mut ptr)?);
    }

    // Find header end
    if data[ptr] != b'>' {
        return Err("Expected '>' at end of header".to_string());
    }
    ptr += 1;

    Ok(ParsedHeader {
        version,
        backward_compat,
        fields,
        header_end: ptr,
    })
}

/// Parse a single header field with validation
/// Uses existing tools and validation functions for robustness
fn parse_header_field(data: &[u8], ptr: &mut usize) -> Result<HeaderField, String> {
    use crate::file_format::validate_name;

    if data[*ptr] != b'(' {
        return Err("Expected '(' for header field".to_string());
    }
    *ptr += 1;

    // Parse name (required)
    let name = match parse(data, ptr).map_err(|e| format!("Failed to parse field name: {}", e))? {
        VsfType::d(n) => {
            validate_name(&n)?; // Use existing validation!
            n
        }
        _ => return Err("Expected d type for field name".to_string()),
    };

    // Parse optional crypto fields (order-independent!)
    // Keep parsing until we hit 'o' (offset marker)
    let mut hash = None;
    let mut signature = None;
    let mut key = None;
    let mut wrap = None;

    while *ptr < data.len() && data[*ptr] != b'o' {
        let field = parse(data, ptr).map_err(|e| format!("Failed to parse crypto field: {}", e))?;

        match field {
            VsfType::hb(_) | VsfType::hs(_) => hash = Some(field),
            VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => signature = Some(field),
            VsfType::ke(_) | VsfType::kx(_) | VsfType::kp(_)
            | VsfType::kc(_) | VsfType::ka(_) => key = Some(field),
            VsfType::v(_, _) => wrap = Some(field),
            _ => {
                // Forward compatibility: ignore unknown types
                // This allows future extensions without breaking old parsers
            }
        }
    }

    // Parse offset (required, marks start of positional fields)
    let offset_bytes = match parse(data, ptr).map_err(|e| format!("Failed to parse offset: {}", e))? {
        VsfType::o(bytes) => bytes,
        _ => return Err("Expected o type for offset".to_string()),
    };

    // Parse size (required)
    let size_bytes = match parse(data, ptr).map_err(|e| format!("Failed to parse size: {}", e))? {
        VsfType::b(bytes, _) => bytes,
        _ => return Err("Expected b type for size".to_string()),
    };

    // Parse child count (optional if encrypted)
    let child_count = if wrap.is_some() {
        // Encrypted sections have implied n[0]
        0
    } else {
        match parse(data, ptr).map_err(|e| format!("Failed to parse child count: {}", e))? {
            VsfType::n(count) => count,
            _ => return Err("Expected n type for child count".to_string()),
        }
    };

    if data[*ptr] != b')' {
        return Err("Expected ')' after header field".to_string());
    }
    *ptr += 1;

    Ok(HeaderField {
        name,
        hash,
        signature,
        key,
        wrap,
        offset_bytes,
        size_bytes,
        child_count,
    })
}

/// Rebuild VSF file with modified header fields
fn rebuild_with_header(
    old_data: &[u8],
    mut fields: Vec<HeaderField>,
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
        test_header.provenance_hash = VsfType::hp(vec![0u8; 32]);
        test_header.rolling_hash = Some(VsfType::hb(vec![0u8; 32]));
        for field in &fields {
            test_header.add_field(field.clone());
        }
        let mut test_encoded = test_header.encode()?;
        VsfHeader::update_header_length(&mut test_encoded)?;
        let new_header_size = test_encoded.len();

        // Check if converged
        if new_header_size == prev_header_size {
            // Build final header with these offsets
            let mut final_header = VsfHeader::new(version, backward_compat);
            final_header.provenance_hash = VsfType::hp(vec![0u8; 32]);
            final_header.rolling_hash = Some(VsfType::hb(vec![0u8; 32]));
            for field in fields {
                final_header.add_field(field);
            }
            let mut new_file = final_header.encode()?;
            VsfHeader::update_header_length(&mut new_file)?;

            // Append section data
            new_file.extend_from_slice(&old_data[old_header_end..]);

            // Compute and write provenance hash (hp)
            let hp_hash = compute_provenance_hash(&new_file)?;
            new_file = write_provenance_hash(new_file, &hp_hash)?;

            // Compute and write rolling hash (hb)
            let hb_hash = compute_file_hash(&new_file)?;
            return write_file_hash(new_file, &hb_hash);
        }

        // Adjust offsets for next iteration
        let offset_adjustment = new_header_size as isize - prev_header_size as isize;

        for field in &mut fields {
            field.offset_bytes = ((field.offset_bytes as isize) + offset_adjustment) as usize;
        }

        prev_header_size = new_header_size;
    }

    Err(format!(
        "Failed to stabilize header after {} iterations",
        MAX_ITERATIONS
    ))
}

/// Compute BLAKE3 provenance hash (hp) of VSF file
///
/// This computes the provenance hash with hp field as zeros.
/// The provenance hash is computed BEFORE any optional signature (ge),
/// so it represents the immutable content identity.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hp placeholder
///
/// # Returns
/// 32-byte BLAKE3 hash
///
pub fn compute_provenance_hash(vsf_bytes: &[u8]) -> Result<[u8; 32], String> {
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

    // Skip creation time (ef5 - always present in version 3+)
    let _creation_time = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Find hp hash placeholder
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() {
        return Err(format!("Pointer {} beyond file size {}", pointer, vsf_bytes.len()));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No provenance hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer,
            vsf_bytes[pointer],
            vsf_bytes[pointer] as char
        ));
    }

    // Parse hash to find position
    let hash_type =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse hash: {}", e))?;

    match hash_type {
        VsfType::hp(hash_bytes) => {
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }

            // Clone file and zero out the hash bytes
            let mut temp_bytes = vsf_bytes.to_vec();
            let hash_start = find_hp_value_position(&temp_bytes, hash_position)?;

            for i in 0..32 {
                temp_bytes[hash_start + i] = 0;
            }

            // Compute BLAKE3 hash of entire file
            let computed_hash = blake3::hash(&temp_bytes);
            Ok(*computed_hash.as_bytes())
        }
        _ => Err("Expected BLAKE3 provenance hash (hp)".to_string()),
    }
}

/// Write computed provenance hash (hp) into the placeholder
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hp placeholder
/// * `hash` - 32-byte BLAKE3 hash to write
///
/// # Returns
/// Modified VSF bytes with hp hash written
///
pub fn write_provenance_hash(mut vsf_bytes: Vec<u8>, hash: &[u8; 32]) -> Result<Vec<u8>, String> {
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

    // Skip creation time (ef5 - always present in version 3+)
    let _creation_time = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Find hash placeholder position
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() {
        return Err(format!("Pointer {} beyond file size {}", pointer, vsf_bytes.len()));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No provenance hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer,
            vsf_bytes[pointer],
            vsf_bytes[pointer] as char
        ));
    }

    // Find the hash value bytes position
    let hash_start = find_hp_value_position(&vsf_bytes, hash_position)?;

    // Write hash into the placeholder
    vsf_bytes[hash_start..hash_start + 32].copy_from_slice(hash);

    Ok(vsf_bytes)
}

/// Find the position of hp hash value bytes within the encoded hash type
fn find_hp_value_position(data: &[u8], hash_marker_pos: usize) -> Result<usize, String> {
    // Re-parse the hash to find where the value bytes start
    let mut pos = hash_marker_pos;

    // Parse the hash using the decode function
    let hash_type = parse(data, &mut pos).map_err(|e| {
        format!(
            "Failed to parse hp hash at position {}: {}",
            hash_marker_pos, e
        )
    })?;

    match hash_type {
        VsfType::hp(hash_bytes) => {
            // pos now points AFTER the hash
            // Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected BLAKE3 provenance hash type (hp)".to_string()),
    }
}

/// Compute BLAKE3 rolling hash (hb) of VSF file
///
/// This function computes the rolling hash with hb field as zeros.
/// It expects the file to already have a hash placeholder (hb[32][zeros]).
/// This is computed AFTER hp and optional ge, so it can catch changes.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hb placeholder
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

    // Skip creation time (ef5 - always present in version 3+)
    let _creation_time = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Skip hp (always present in version 3+)
    let _hp = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse provenance hash: {}", e))?;

    // Skip optional signature (ge)
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'g' {
        let _sig = parse(vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse signature: {}", e))?;
    }

    // Find rolling hash (hb) placeholder
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() {
        return Err(format!("Pointer {} beyond file size {}", pointer, vsf_bytes.len()));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No rolling hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer,
            vsf_bytes[pointer],
            vsf_bytes[pointer] as char
        ));
    }

    // Parse hash to find position
    let hash_type =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse hash: {}", e))?;

    match hash_type {
        VsfType::hb(hash_bytes) => {
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
        _ => Err("Expected BLAKE3 rolling hash (hb)".to_string()),
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

    // Skip creation time (ef5 - always present in version 3+)
    let _creation_time = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Skip hp (always present in version 3+)
    let _hp = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse provenance hash: {}", e))?;

    // Skip optional signature (ge)
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'g' {
        let _sig = parse(&vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse signature: {}", e))?;
    }

    // Find rolling hash (hb) placeholder position
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() {
        return Err(format!("Pointer {} beyond file size {}", pointer, vsf_bytes.len()));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No rolling hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer,
            vsf_bytes[pointer],
            vsf_bytes[pointer] as char
        ));
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
        VsfType::hb(hash_bytes) => {
            // pos now points AFTER the hash
            // Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected BLAKE3 hash type (hb)".to_string()),
    }
}

///
/// This function:
/// 0. Finds the specified section in the header
/// 1. Extracts the section data bytes `[d"name" (fields...)]`
/// 2. Signs those bytes with Ed25519
/// 3. Rebuilds the header with signature in header field definition
/// 4. Recomputes file hash
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
/// * `section` - Name of the section to sign (e.g., "raw")
/// * `signing_key` - Ed25519 signing key bytes (must be valid SigningKey)
///
/// # Returns
/// Modified VSF bytes with section signature in header field definition
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
    let section_field = header
        .fields
        .iter()
        .find(|f| f.name == section_name)
        .ok_or_else(|| format!("Section '{}' not found", section_name))?;

    let section_offset = section_field.offset_bytes;
    let section_size = section_field.size_bytes;

    // Extract and sign section bytes
    if section_offset + section_size > vsf_bytes.len() {
        return Err("Section exceeds file bounds".to_string());
    }
    let section_bytes = &vsf_bytes[section_offset..section_offset + section_size];
    let signature = signing_key.sign(section_bytes);

    // Create signature VsfType (Ed25519 signature is always 64 bytes)
    let sig_vsf = VsfType::ge(signature.to_bytes().to_vec());

    // Update header fields - add signature to target section
    let mut new_fields = header.fields.clone();
    for field in &mut new_fields {
        if field.name == section_name {
            field.signature = Some(sig_vsf);
            break;
        }
    }

    // Rebuild file with modified header
    rebuild_with_header(
        &vsf_bytes,
        new_fields,
        header.version,
        header.backward_compat,
        header.header_end,
    )
}

/// Add encryption metadata to a section's header field
///
/// This function:
/// 0. Finds the specified section in the header
/// 1. Adds encryption algorithm (v) and key (k) to the header field
/// 2. Rebuilds the file with updated header
/// 3. Updates file hash
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
/// Modified VSF bytes with encryption metadata in header field
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
    let mut new_fields = header.fields.clone();
    let mut found = false;

    for field in &mut new_fields {
        if field.name == section_name {
            use crate::crypto_algorithms::{WRAP_AES256_GCM, WRAP_CHACHA20POLY1305};

            // Add wrapped/encrypted marker (v)
            field.wrap = Some(VsfType::v(algorithm, vec![])); // Empty vec, just marks as encrypted

            // Add encryption key based on algorithm
            let key_vsf = match algorithm {
                WRAP_CHACHA20POLY1305 => VsfType::kc(encryption_key.to_vec()),
                WRAP_AES256_GCM => VsfType::ka(encryption_key.to_vec()),
                _ => {
                    return Err(format!(
                        "Unsupported encryption algorithm: {}",
                        algorithm as char
                    ))
                }
            };
            field.key = Some(key_vsf);
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
        new_fields,
        header.version,
        header.backward_compat,
        header.header_end,
    )
}

/// Verify the provenance hash (hp) in a VSF header
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns
/// `Ok(())` if hash is valid, `Err` with description if invalid or missing
pub fn verify_provenance_hash(vsf_bytes: &[u8]) -> Result<(), String> {
    let computed_hash = compute_provenance_hash(vsf_bytes)?;

    // Parse to get stored hash
    let mut pointer = 4; // Skip "RÅ<"

    let _header_length = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;
    let _version = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let _creation_time = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    let hash_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse provenance hash: {}", e))?;

    let stored_hash = match hash_type {
        VsfType::hp(hash_bytes) => {
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }
            hash_bytes
        }
        _ => return Err("Expected BLAKE3 provenance hash type (hp) in header".to_string()),
    };

    if computed_hash.as_slice() == stored_hash.as_slice() {
        Ok(())
    } else {
        Err("Provenance hash verification failed: computed hash does not match stored hash".to_string())
    }
}

/// Verify the rolling hash (hb) in a VSF header
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns
/// `Ok(())` if hash is valid, `Err` with description if invalid or missing
pub fn verify_file_hash(vsf_bytes: &[u8]) -> Result<(), String> {
    let computed_hash = compute_file_hash(vsf_bytes)?;

    // Parse to get stored hash
    let mut pointer = 4; // Skip "RÅ<"

    let _header_length = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;
    let _version = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let _creation_time = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;
    let _hp = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse provenance hash: {}", e))?;

    // Skip optional signature
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'g' {
        let _sig = parse(vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse signature: {}", e))?;
    }

    let hash_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse rolling hash: {}", e))?;

    let stored_hash = match hash_type {
        VsfType::hb(hash_bytes) => {
            if hash_bytes.len() != 32 {
                return Err(format!(
                    "Invalid hash size: expected 32 bytes, found {}",
                    hash_bytes.len()
                ));
            }
            hash_bytes
        }
        _ => return Err("Expected BLAKE3 rolling hash type (hb) in header".to_string()),
    };

    if computed_hash.as_slice() == stored_hash.as_slice() {
        Ok(())
    } else {
        Err("Rolling hash verification failed: computed hash does not match stored hash".to_string())
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
        section.add_field("value", VsfType::u(42, false));

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
