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
    rolling_hash: Option<VsfType>,
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
    let version =
        match parse(data, &mut ptr).map_err(|e| format!("Failed to parse version: {}", e))? {
            VsfType::z(v) => v,
            _ => return Err("Expected z type for version".to_string()),
        };

    // Parse backward compat
    let backward_compat = match parse(data, &mut ptr)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?
    {
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
    let prov_type =
        parse(data, &mut ptr).map_err(|e| format!("Failed to parse provenance hash/sig: {}", e))?;
    match prov_type {
        VsfType::hp(_) => {}                                   // Provenance hash
        VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => {} // Signature (replaces hp)
        _ => {
            return Err(format!(
                "Expected hp or ge after creation time, got: {:?}",
                prov_type
            ))
        }
    }

    // Optional: hb (rolling hash) - only if next byte is 'h'
    let rolling_hash = if ptr < data.len() && data[ptr] == b'h' {
        Some(parse(data, &mut ptr).map_err(|e| format!("Failed to parse rolling hash: {}", e))?)
    } else {
        None
    };

    // Parse header field count
    let field_count =
        match parse(data, &mut ptr).map_err(|e| format!("Failed to parse field count: {}", e))? {
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
        rolling_hash,
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
            VsfType::ke(_) | VsfType::kx(_) | VsfType::kp(_) | VsfType::kc(_) | VsfType::ka(_) => {
                key = Some(field)
            }
            VsfType::v(_, _) => wrap = Some(field),
            _ => {
                // Forward compatibility: ignore unknown types
                // This allows future extensions without breaking old parsers
            }
        }
    }

    // Parse offset (required, marks start of positional fields)
    let offset_bytes =
        match parse(data, ptr).map_err(|e| format!("Failed to parse offset: {}", e))? {
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
    include_rolling_hash: bool,
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
        test_header.rolling_hash = if include_rolling_hash {
            Some(VsfType::hb(vec![0u8; 32]))
        } else {
            None
        };
        for field in &fields {
            test_header.add_field(field.clone());
        }
        let mut test_encoded = test_header.encode()?;
        VsfHeader::update_header_length(&mut test_encoded)?;
        let new_header_size = test_encoded.len();

        // Check if converged
        if new_header_size == prev_header_size {
            // Extract signatures from fields BEFORE we consume them
            let field_signatures: Vec<Option<VsfType>> = fields.iter()
                .map(|f| {
                    if let Some(ref sig) = f.signature {
                        let sig_bytes = match sig {
                            VsfType::ge(bytes) => bytes,
                            VsfType::gp(bytes) => bytes,
                            VsfType::gr(bytes) => bytes,
                            _ => &vec![],
                        };
                        eprintln!("DEBUG rebuild_with_header: Extracting signature from field '{}', {} bytes, first 4: {:02X?}",
                            f.name, sig_bytes.len(), if sig_bytes.len() >= 4 { &sig_bytes[0..4] } else { &sig_bytes[..] });
                    }
                    f.signature.clone()
                })
                .collect();

            // Build final header with these offsets
            let mut final_header = VsfHeader::new(version, backward_compat);
            final_header.provenance_hash = VsfType::hp(vec![0u8; 32]);
            final_header.rolling_hash = if include_rolling_hash {
                Some(VsfType::hb(vec![0u8; 32]))
            } else {
                None
            };
            for field in fields {
                final_header.add_field(field);
            }
            let mut new_file = final_header.encode()?;
            VsfHeader::update_header_length(&mut new_file)?;

            // Append section data
            new_file.extend_from_slice(&old_data[old_header_end..]);

            // Compute and write provenance hash (hp) - this zeros signatures internally
            let hp_hash = compute_provenance_hash(&new_file)?;
            new_file = write_provenance_hash(new_file, &hp_hash)?;

            // Write all header field signatures (ge/gp/gr) into placeholders
            // This must come AFTER hp computation since hp is computed with signatures zeroed
            // But BEFORE hb computation since hb should include the actual signature bytes
            eprintln!("DEBUG rebuild_with_header: About to call write_header_field_signatures_from_list with {} signatures", field_signatures.len());
            new_file = write_header_field_signatures_from_list(new_file, field_signatures)?;

            // Compute and write rolling hash (hb) AFTER signatures are written (if requested)
            // Rolling hash is redundant when using signatures, so only include if explicitly requested
            if include_rolling_hash {
                let hb_hash = compute_file_hash(&new_file)?;
                new_file = write_file_hash(new_file, &hb_hash)?;
            }

            // DEBUG: Check if signature is in bytes
            if new_file.len() > 120 {
                eprintln!(
                    "DEBUG rebuild_with_header: Bytes 0x70-0x7F = {:02X?}",
                    &new_file[0x70..0x80]
                );
            }

            return Ok(new_file);
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

    // Parse version and backward compat FIRST (VSF v4+ format)
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse header length
    let _header_length_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Skip creation time (ef5 - always present in version 3+)
    let _creation_time = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse creation time: {}", e))?;

    // Find hp hash placeholder
    let hash_position = pointer;
    if pointer >= vsf_bytes.len() {
        return Err(format!(
            "Pointer {} beyond file size {}",
            pointer,
            vsf_bytes.len()
        ));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No provenance hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer, vsf_bytes[pointer], vsf_bytes[pointer] as char
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

            // Clone file and zero out all crypto fields: hp, hb (if present), ge (if present in header fields)
            let mut temp_bytes = vsf_bytes.to_vec();
            let hash_start = find_hp_value_position(&temp_bytes, hash_position)?;

            // Zero out hp field
            for i in 0..32 {
                temp_bytes[hash_start + i] = 0;
            }

            // Check for optional hb (rolling hash) after hp
            let mut ptr_after_hp = pointer;
            if ptr_after_hp < temp_bytes.len() && temp_bytes[ptr_after_hp] == b'h' {
                let hb_position = ptr_after_hp;
                let hb_type = parse(&temp_bytes, &mut ptr_after_hp)
                    .map_err(|e| format!("Failed to parse rolling hash: {}", e))?;
                if let VsfType::hb(hb_bytes) = hb_type {
                    if hb_bytes.len() == 32 {
                        let hb_start = find_hash_value_position(&temp_bytes, hb_position)?;
                        // Zero out hb field
                        for i in 0..32 {
                            temp_bytes[hb_start + i] = 0;
                        }
                    }
                }
            }

            // Now we need to find and zero out any ge/gp/gr signatures in header fields
            // This requires parsing the header fields which come after the field count
            // For now, we'll scan for signature fields and zero them
            zero_all_signatures(&mut temp_bytes)?;

            // Compute BLAKE3 hash of entire file
            let computed_hash = blake3::hash(&temp_bytes);
            Ok(*computed_hash.as_bytes())
        }
        _ => Err("Expected BLAKE3 provenance hash (hp)".to_string()),
    }
}

/// Zero out all signature fields (ge, gp, gr) in the VSF file
fn zero_all_signatures(vsf_bytes: &mut Vec<u8>) -> Result<(), String> {
    let mut ptr = 0;
    while ptr < vsf_bytes.len() - 1 {
        // Look for signature markers: ge, gp, gr
        if vsf_bytes[ptr] == b'g'
            && (vsf_bytes[ptr + 1] == b'e'
                || vsf_bytes[ptr + 1] == b'p'
                || vsf_bytes[ptr + 1] == b'r')
        {
            let sig_position = ptr;
            let sig_type = match parse(vsf_bytes, &mut ptr) {
                Ok(t) => t,
                Err(_) => {
                    ptr = sig_position + 1;
                    continue;
                }
            };

            match sig_type {
                VsfType::ge(sig_bytes) | VsfType::gp(sig_bytes) | VsfType::gr(sig_bytes) => {
                    let sig_len = sig_bytes.len();
                    if let Ok(sig_start) =
                        find_signature_value_position(vsf_bytes, sig_position, sig_len)
                    {
                        // Zero out signature
                        for i in 0..sig_len {
                            vsf_bytes[sig_start + i] = 0;
                        }
                    }
                }
                _ => {}
            }
        } else {
            ptr += 1;
        }
    }
    Ok(())
}

/// Find the position of signature value bytes within the encoded signature type
fn find_signature_value_position(
    data: &[u8],
    sig_marker_pos: usize,
    sig_len: usize,
) -> Result<usize, String> {
    let mut pos = sig_marker_pos;
    let sig_type =
        parse(data, &mut pos).map_err(|e| format!("Failed to parse signature: {}", e))?;

    match sig_type {
        VsfType::ge(bytes) | VsfType::gp(bytes) | VsfType::gr(bytes) => {
            // pos now points AFTER the signature
            // Calculate where the signature bytes started
            let sig_start = pos - bytes.len();
            Ok(sig_start)
        }
        _ => Err("Expected signature type (ge/gp/gr)".to_string()),
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
        return Err(format!(
            "Pointer {} beyond file size {}",
            pointer,
            vsf_bytes.len()
        ));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No provenance hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer, vsf_bytes[pointer], vsf_bytes[pointer] as char
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

    // Parse version and backward compat FIRST (VSF v4+ format)
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse header length
    let _header_length_type = parse(vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

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
        return Err(format!(
            "Pointer {} beyond file size {}",
            pointer,
            vsf_bytes.len()
        ));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No rolling hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer, vsf_bytes[pointer], vsf_bytes[pointer] as char
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
        return Err(format!(
            "Pointer {} beyond file size {}",
            pointer,
            vsf_bytes.len()
        ));
    }
    if vsf_bytes[pointer] != b'h' {
        return Err(format!(
            "No rolling hash placeholder found at position {}. Found byte: 0x{:02X} ('{}')",
            pointer, vsf_bytes[pointer], vsf_bytes[pointer] as char
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

/// Write all header field signatures from the parsed header into their placeholders
///
/// This function scans the header for all signature placeholders (ge/gp/gr with zeros)
/// and writes the actual signature bytes from the parsed header fields.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with signature placeholders (zeros)
///
/// # Returns
/// Modified VSF bytes with actual signature values written
/// Write header field signatures from a provided list (instead of parsing)
/// This is used when we already have the signature bytes extracted before flattening
fn write_header_field_signatures_from_list(
    mut vsf_bytes: Vec<u8>,
    field_signatures: Vec<Option<VsfType>>,
) -> Result<Vec<u8>, String> {
    eprintln!(
        "DEBUG write_from_list: Called with {} field signatures",
        field_signatures.len()
    );

    // Parse the header just to get header_end (don't extract signatures from it)
    let header = parse_full_header(&vsf_bytes)?;

    // Extract signature bytes from the provided list
    let mut signatures = Vec::new();
    for sig_opt in field_signatures {
        if let Some(sig_vsf) = sig_opt {
            let sig_bytes = match sig_vsf {
                VsfType::ge(bytes) => bytes,
                VsfType::gp(bytes) => bytes,
                VsfType::gr(bytes) => bytes,
                _ => continue,
            };
            eprintln!(
                "DEBUG write_from_list: Extracted signature {} bytes, first 4: {:02X?}",
                sig_bytes.len(),
                if sig_bytes.len() >= 4 {
                    &sig_bytes[0..4]
                } else {
                    &sig_bytes[..]
                }
            );
            signatures.push(sig_bytes);
        }
    }
    eprintln!(
        "DEBUG write_from_list: Total signatures to write: {}",
        signatures.len()
    );

    // Now scan header for signature placeholders and write them
    // We scan only up to header_end
    let header_end = header.header_end;
    eprintln!("DEBUG write_from_list: Header ends at byte {}", header_end);
    let mut sig_index = 0;

    let mut pos = 0;
    while pos < header_end - 1 && sig_index < signatures.len() {
        if vsf_bytes[pos] == b'g'
            && (vsf_bytes[pos + 1] == b'e'
                || vsf_bytes[pos + 1] == b'p'
                || vsf_bytes[pos + 1] == b'r')
        {
            // Found potential signature marker
            eprintln!(
                "DEBUG write_from_list: Found signature marker at pos {}",
                pos
            );
            let mut test_ptr = pos;
            if let Ok(sig_type) = parse(&vsf_bytes, &mut test_ptr) {
                match sig_type {
                    VsfType::ge(test_bytes) | VsfType::gp(test_bytes) | VsfType::gr(test_bytes) => {
                        eprintln!(
                            "DEBUG write_from_list: Parsed signature, {} bytes, all zeros: {}",
                            test_bytes.len(),
                            test_bytes.iter().all(|&b| b == 0)
                        );
                        // Check if this is all zeros (placeholder)
                        if test_bytes.iter().all(|&b| b == 0)
                            && test_bytes.len() == signatures[sig_index].len()
                        {
                            // Found a placeholder - write the signature
                            let sig_start = test_ptr - test_bytes.len();
                            eprintln!("DEBUG write_from_list: Writing signature at byte {}, first 4 bytes: {:02X?}", sig_start, &signatures[sig_index][0..4]);
                            vsf_bytes[sig_start..sig_start + signatures[sig_index].len()]
                                .copy_from_slice(&signatures[sig_index]);
                            eprintln!(
                                "DEBUG write_from_list: After write, bytes at sig_start: {:02X?}",
                                &vsf_bytes[sig_start..sig_start + 4]
                            );
                            sig_index += 1;
                        }
                        pos = test_ptr; // Continue after this signature
                    }
                    _ => {
                        pos += 1;
                    }
                }
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    eprintln!(
        "DEBUG write_from_list: Finished scanning, wrote {} signatures",
        sig_index
    );

    Ok(vsf_bytes)
}

fn write_header_field_signatures(mut vsf_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    // Parse the full header to get all fields with their signatures
    let header = parse_full_header(&vsf_bytes)?;

    // Get all signature bytes from header fields
    let mut signatures = Vec::new();
    for field in &header.fields {
        if let Some(ref sig_vsf) = field.signature {
            let sig_bytes = match sig_vsf {
                VsfType::ge(bytes) => bytes.clone(),
                VsfType::gp(bytes) => bytes.clone(),
                VsfType::gr(bytes) => bytes.clone(),
                _ => continue,
            };
            eprintln!(
                "DEBUG: Found signature in header field, {} bytes",
                sig_bytes.len()
            );
            signatures.push(sig_bytes);
        }
    }
    eprintln!("DEBUG: Total signatures to write: {}", signatures.len());

    // Now scan header for signature placeholders and write them
    // We scan only up to header_end
    let header_end = header.header_end;
    eprintln!("DEBUG: Header ends at byte {}", header_end);
    let mut sig_index = 0;

    let mut pos = 0;
    while pos < header_end - 1 && sig_index < signatures.len() {
        if vsf_bytes[pos] == b'g'
            && (vsf_bytes[pos + 1] == b'e'
                || vsf_bytes[pos + 1] == b'p'
                || vsf_bytes[pos + 1] == b'r')
        {
            // Found potential signature marker
            eprintln!("DEBUG: Found signature marker at pos {}", pos);
            let mut test_ptr = pos;
            if let Ok(sig_type) = parse(&vsf_bytes, &mut test_ptr) {
                match sig_type {
                    VsfType::ge(test_bytes) | VsfType::gp(test_bytes) | VsfType::gr(test_bytes) => {
                        eprintln!(
                            "DEBUG: Parsed signature, {} bytes, all zeros: {}",
                            test_bytes.len(),
                            test_bytes.iter().all(|&b| b == 0)
                        );
                        // Check if this is all zeros (placeholder)
                        if test_bytes.iter().all(|&b| b == 0)
                            && test_bytes.len() == signatures[sig_index].len()
                        {
                            // Found a placeholder - write the signature
                            let sig_start = test_ptr - test_bytes.len();
                            eprintln!(
                                "DEBUG: test_ptr={}, test_bytes.len()={}, sig_start={}",
                                test_ptr,
                                test_bytes.len(),
                                sig_start
                            );
                            eprintln!(
                                "DEBUG: Byte at sig_start (before write): 0x{:02X}",
                                vsf_bytes[sig_start]
                            );
                            eprintln!(
                                "DEBUG: First 4 signature bytes: {:02X?}",
                                &signatures[sig_index][0..4]
                            );
                            vsf_bytes[sig_start..sig_start + signatures[sig_index].len()]
                                .copy_from_slice(&signatures[sig_index]);
                            eprintln!(
                                "DEBUG: Byte at sig_start (after write): 0x{:02X}",
                                vsf_bytes[sig_start]
                            );
                            sig_index += 1;
                        }
                        pos = test_ptr; // Continue after this signature
                    }
                    _ => {
                        pos += 1;
                    }
                }
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    eprintln!("DEBUG: Finished scanning, wrote {} signatures", sig_index);

    Ok(vsf_bytes)
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
#[cfg(feature = "crypto")]
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
    let sig_bytes = signature.to_bytes().to_vec();
    eprintln!(
        "DEBUG sign_section: Generated signature, first 4 bytes: {:02X?}",
        &sig_bytes[0..4]
    );
    let sig_vsf = VsfType::ge(sig_bytes);

    // Update header fields - add signature to target section
    let mut new_fields = header.fields.clone();
    for field in &mut new_fields {
        if field.name == section_name {
            field.signature = Some(sig_vsf);
            break;
        }
    }

    // Rebuild file with modified header
    // Don't include rolling hash when signing - the signature provides cryptographic integrity
    rebuild_with_header(
        &vsf_bytes,
        new_fields,
        header.version,
        header.backward_compat,
        header.header_end,
        false, // Don't include rolling hash - signature is stronger
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
    // Preserve original rolling hash setting
    rebuild_with_header(
        &vsf_bytes,
        new_fields,
        header.version,
        header.backward_compat,
        header.header_end,
        header.rolling_hash.is_some(),
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
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
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
        Err(
            "Provenance hash verification failed: computed hash does not match stored hash"
                .to_string(),
        )
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
    let _version =
        parse(vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
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
        Err(
            "Rolling hash verification failed: computed hash does not match stored hash"
                .to_string(),
        )
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
