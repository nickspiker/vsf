//! VSF verification functions for hashing and signing
//!
//! This module provides standalone functions for adding cryptographic verification to VSF files after they've been built. Two independent strategies are supported:
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
//! use vsf::builders::RawImageBuilder; use vsf::verification::sign_section;
//!
//! // Build the VSF let bytes = raw.build()?;
//!
//! // Add verification as needed
//! ```

use crate::prelude::*;
use crate::decoding::parse;

/// Skip the optional creation_time field in a VSF header at `pointer`. The field is identified by the `e` marker byte; if not present, the header simply omits it (callers without a clock).
fn skip_optional_creation_time(bytes: &[u8], pointer: &mut usize) -> Result<(), String> {
    if *pointer < bytes.len() && bytes[*pointer] == b'e' {
        let _ = parse(bytes, pointer).map_err(|e| format!("creation_time: {}", e))?;
    }
    Ok(())
}
use crate::file_format::HeaderField;
use crate::types::VsfType;

/// Helper struct for complete header information
pub struct ParsedHeader {
    pub version: usize,
    pub backward_compat: usize,
    pub file_length: usize, // Total file size from l field
    /// Creation time field if the source file emitted one. Clockless devices omit the `e` field entirely; this stays `None` in that case so a rebuild can preserve the omission.
    pub creation_time: Option<VsfType>,
    pub rolling_hash: Option<VsfType>,
    pub fields: Vec<HeaderField>,
    pub header_end: usize, // Byte position where header ends (after '>')
}

/// Parse complete VSF header including all header field crypto metadata Robust, order-independent parser using existing VSF tools
pub fn parse_full_header(data: &[u8]) -> Result<ParsedHeader, String> {
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

    // Reject files that require a newer implementation than we are
    if backward_compat > crate::VSF_VERSION {
        return Err(format!(
            "File requires VSF v{} but this implementation is v{}",
            backward_compat,
            crate::VSF_VERSION
        ));
    }

    // Reject files OLDER than our backward-compat floor. Without this check a v7 file sails into a v8 reader and its x fields Huffman-decode under the wrong codebook — garbage text or a misleading UTF-8 error instead of a version error. Same no-silent-misinterpretation rule as everywhere else: if we can't read it correctly, say so loudly and say why. The floor is build-time chosen (see VSF_BACKWARD_COMPAT): compat-v* features lower it and compile in the matching era's decode paths, dispatched on `version` from this point down.
    if version < crate::VSF_BACKWARD_COMPAT {
        return Err(format!(
            "File is VSF v{} but this build reads v{}+ — a compat-v{} feature of vsf is required to read this archive",
            version,
            crate::VSF_BACKWARD_COMPAT,
            version
        ));
    }

    // Parse header length (now we know how to decode it!)
    let _ = parse(data, &mut ptr).map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Parse optional file length (l field for TCP streaming)
    let file_length = if ptr < data.len() && data[ptr] == b'l' {
        match parse(data, &mut ptr) {
            Ok(VsfType::l(len, _)) => len,
            Ok(other) => return Err(crate::type_mismatch_err!("Expected l value", other)),
            Err(e) => return Err(format!("Failed to parse file length: {}", e)),
        }
    } else {
        0 // No l field - use 0 to indicate unknown
    };

    // Optional creation time — devices without a clock omit the `e` field. Preserve the value so callers (e.g. rebuild_with_header) can re-emit it.
    let creation_time = if ptr < data.len() && data[ptr] == b'e' {
        Some(parse(data, &mut ptr).map_err(|e| format!("Failed to parse creation time: {}", e))?)
    } else {
        None
    };

    // Parse provenance primitives in FIXED order (version determines format) Required: hp (provenance hash) Optional: ke (signer pubkey) + ge (signature) OR hb (rolling hash)

    // Parse hp (required)
    let prov_type =
        parse(data, &mut ptr).map_err(|e| format!("Failed to parse provenance hash: {}", e))?;
    match prov_type {
        VsfType::hp(_) => {} // Provenance hash
        _ => {
            return Err(crate::type_mismatch_err!(
                "Expected hp after creation time",
                prov_type
            ))
        }
    }

    // Optional: ke (signer pubkey) + ge (signature) OR hb (rolling hash)
    let mut rolling_hash = None;

    // Check for ke (signer pubkey) - starts with 'k'
    if ptr < data.len() && data[ptr] == b'k' {
        let _ =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse signer pubkey: {}", e))?;

        // ke must be followed by ge (signature)
        if ptr < data.len() && data[ptr] == b'g' {
            let _ =
                parse(data, &mut ptr).map_err(|e| format!("Failed to parse signature: {}", e))?;
        }
    }
    // Check for hb (rolling hash) - starts with 'h'
    else if ptr < data.len() && data[ptr] == b'h' {
        rolling_hash = Some(
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse rolling hash: {}", e))?,
        );
    }

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
        file_length,
        creation_time,
        rolling_hash,
        fields,
        header_end: ptr,
    })
}

/// Parse a single header field with validation Uses VsfField::parse() for generic parsing, then extracts and validates values
fn parse_header_field(data: &[u8], ptr: &mut usize) -> Result<HeaderField, String> {
    use crate::file_format::{validate_name, VsfField};

    // Use generic field parser
    let field =
        VsfField::parse(data, ptr).map_err(|e| format!("Failed to parse header field: {}", e))?;

    // Validate field name
    validate_name(&field.name)?;

    // Extract values by type
    let mut hash = None;
    let mut signature = None;
    let mut key = None;
    let mut offset_bytes = None;
    let mut size_bytes = None;
    let mut child_count = None;

    for value in field.values {
        match value {
            // Crypto fields
            VsfType::hb(_) | VsfType::hs(_) | VsfType::hp(_) => hash = Some(value),
            VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => signature = Some(value),
            VsfType::ke(_) | VsfType::kx(_) | VsfType::kp(_) | VsfType::kc(_) | VsfType::ka(_) => {
                key = Some(value)
            }
            // Positional fields
            VsfType::o(bytes) => offset_bytes = Some(bytes),
            VsfType::b(bytes, _) => size_bytes = Some(bytes),
            VsfType::n(count) => child_count = Some(count),
            // Forward compatibility: ignore unknown types (including legacy wrap markers)
            _ => {}
        }
    }

    // Metadata-only fields have no positional data (no o/b/n) This is valid for empty sections or header metadata like avatar_id, ping, etc.
    let is_metadata_only = offset_bytes.is_none() && size_bytes.is_none();

    if is_metadata_only {
        // Metadata-only field - no section body
        Ok(HeaderField {
            name: field.name,
            hash,
            signature,
            key,
            offset_bytes: 0,
            size_bytes: 0,
            child_count: 0,
            inline_values: Vec::new(),
        })
    } else {
        // Section field - require all positional data
        let offset_bytes = offset_bytes.ok_or("Missing required offset (o) field")?;
        let size_bytes = size_bytes.ok_or("Missing required size (b) field")?;
        let child_count = child_count.ok_or("Missing required child count (n) field")?;

        Ok(HeaderField {
            name: field.name,
            hash,
            signature,
            key,
            offset_bytes,
            size_bytes,
            child_count,
            inline_values: Vec::new(),
        })
    }
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

    // Preserve creation_time from the source so the rebuilt header keeps its timestamp. Clockless files (no `e` field) round-trip as `None`.
    let preserved_creation_time = parse_full_header(old_data)?.creation_time;

    // Stabilization loop - iterate until header size and offsets converge
    const MAX_ITERATIONS: usize = 10;
    let mut prev_header_size = old_header_size;

    for _iteration in 0..MAX_ITERATIONS {
        // Calculate what the new header size will be
        let mut test_header = VsfHeader::new(version, backward_compat);
        test_header.creation_time = preserved_creation_time.clone();
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
            let field_signatures: Vec<Option<VsfType>> =
                fields.iter().map(|f| f.signature.clone()).collect();

            // Build final header with these offsets
            let mut final_header = VsfHeader::new(version, backward_compat);
            final_header.creation_time = preserved_creation_time.clone();
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

            // Write all header field signatures (ge/gp/gr) into placeholders This must come AFTER hp computation since hp is computed with signatures zeroed But BEFORE hb computation since hb should include the actual signature bytes
            new_file = write_header_field_signatures_from_list(new_file, field_signatures)?;

            // Compute and write rolling hash (hb) AFTER signatures are written (if requested) Rolling hash is redundant when using signatures, so only include if explicitly requested
            if include_rolling_hash {
                let hb_hash = compute_file_hash(&new_file)?;
                new_file = write_file_hash(new_file, &hb_hash)?;
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
/// This computes the provenance hash with hp field as zeros. The provenance hash is computed BEFORE any optional signature (ge), so it represents the immutable content identity.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hp placeholder
///
/// # Returns 32-byte BLAKE3 hash
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

    // Skip optional file length (l field) if present
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse file length: {}", e))?;
    }

    // Skip creation time (eu6 default, ef5/ef6 legacy - always present in version 3+)
    skip_optional_creation_time(vsf_bytes, &mut pointer)?;

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

            // Now we need to find and zero out any ge/gp/gr signatures in header fields This requires parsing the header fields which come after the field count For now, we'll scan for signature fields and zero them
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
                    if let Ok(sig_start) = find_signature_value_position(vsf_bytes, sig_position) {
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
fn find_signature_value_position(data: &[u8], sig_marker_pos: usize) -> Result<usize, String> {
    let mut pos = sig_marker_pos;
    let sig_type =
        parse(data, &mut pos).map_err(|e| format!("Failed to parse signature: {}", e))?;

    match sig_type {
        VsfType::ge(bytes) | VsfType::gp(bytes) | VsfType::gr(bytes) => {
            // pos now points AFTER the signature Calculate where the signature bytes started
            let sig_start = pos - bytes.len();
            Ok(sig_start)
        }
        _ => Err("Expected signature type (ge/gp/gr)".to_string()),
    }
}

/// Write computed provenance hash (hp) into the placeholder
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hp placeholder * `hash` - 32-byte BLAKE3 hash to write
///
/// # Returns
/// Modified VSF bytes with hp hash written
///
pub fn write_provenance_hash(mut vsf_bytes: Vec<u8>, hash: &[u8; 32]) -> Result<Vec<u8>, String> {
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse version and backward compat
    let _version =
        parse(&vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse header length
    let _header_length_type = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Skip optional file length (l field) if present
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(&vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse file length: {}", e))?;
    }

    // Skip creation time (ef5 - always present in version 3+)
    skip_optional_creation_time(&vsf_bytes, &mut pointer)?;

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
            // pos now points AFTER the hash Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected BLAKE3 provenance hash type (hp)".to_string()),
    }
}

/// Fill the provenance hash (hp) placeholder with computed hash bytes
///
/// # Arguments
/// * `vsf_bytes` - Mutable VSF file bytes with hp placeholder (32 zeros) * `hash` - The computed 32-byte BLAKE3 hash
///
/// # Returns
/// Ok(()) on success
///
pub fn fill_provenance_hash(vsf_bytes: &mut [u8], hash: &[u8; 32]) -> Result<(), String> {
    // Find hp position
    let mut pointer = 4; // Skip "RÅ<"

    // Skip version, backward compat, header length
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("version: {}", e))?;
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("backward: {}", e))?;
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("header_len: {}", e))?;

    // Skip optional file length (L)
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("file_len: {}", e))?;
    }

    // Skip creation time
    skip_optional_creation_time(vsf_bytes, &mut pointer)?;

    // Should be at hp now
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'h' {
        return Err("No hp field found".to_string());
    }

    let hash_start = find_hp_value_position(vsf_bytes, pointer)?;
    vsf_bytes[hash_start..hash_start + 32].copy_from_slice(hash);

    Ok(())
}

/// Fill the signature (ge) placeholder with signature bytes
///
/// # Arguments
/// * `vsf_bytes` - Mutable VSF file bytes with ge placeholder (64 zeros) * `signature` - The 64-byte Ed25519 signature
///
/// # Returns
/// Ok(()) on success
///
pub fn fill_signature(vsf_bytes: &mut [u8], signature: &[u8]) -> Result<(), String> {
    if signature.len() != 64 {
        return Err(format!(
            "Signature must be 64 bytes, got {}",
            signature.len()
        ));
    }

    // Find ge position (after hp, optionally after ke)
    let mut pointer = 4; // Skip "RÅ<"

    // Skip version, backward compat, header length
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("version: {}", e))?;
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("backward: {}", e))?;
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("header_len: {}", e))?;

    // Skip optional file length (L)
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("file_len: {}", e))?;
    }

    // Skip creation time
    skip_optional_creation_time(vsf_bytes, &mut pointer)?;

    // Skip hp (provenance hash)
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'h' {
        return Err("No hp field found".to_string());
    }
    let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("hp: {}", e))?;

    // Skip optional ke (signer pubkey) if present
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'k' {
        let _ = parse(vsf_bytes, &mut pointer).map_err(|e| format!("ke: {}", e))?;
    }

    // Should be at ge now
    if pointer >= vsf_bytes.len() || vsf_bytes[pointer] != b'g' {
        return Err("No ge field found".to_string());
    }

    // Parse ge to find value position
    let ge_marker = pointer;
    let ge_type = parse(vsf_bytes, &mut pointer).map_err(|e| format!("ge: {}", e))?;

    match ge_type {
        VsfType::ge(sig_bytes) => {
            let sig_start = pointer - sig_bytes.len();
            vsf_bytes[sig_start..sig_start + 64].copy_from_slice(signature);
            Ok(())
        }
        _ => Err(format!("Expected ge at position {}", ge_marker)),
    }
}

/// Compute BLAKE3 rolling hash (hb) of VSF file
///
/// This function computes the rolling hash with hb field as zeros. It expects the file to already have a hash placeholder (hb[32][zeros]). This is computed AFTER hp and optional ge, so it can catch changes.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with hb placeholder
///
/// # Returns 32-byte BLAKE3 hash
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

    // Skip optional file length (l field) if present
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse file length: {}", e))?;
    }

    // Skip creation time (eu6 default, ef5/ef6 legacy - always present in version 3+)
    skip_optional_creation_time(vsf_bytes, &mut pointer)?;

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
/// * `vsf_bytes` - Complete VSF file bytes with hash placeholder * `hash` - 32-byte BLAKE3 hash to write
///
/// # Returns
/// Modified VSF bytes with hash written
///
pub fn write_file_hash(mut vsf_bytes: Vec<u8>, hash: &[u8; 32]) -> Result<Vec<u8>, String> {
    if vsf_bytes.len() < 4 {
        return Err("File too small to be valid VSF".to_string());
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Parse version and backward compat
    let _version =
        parse(&vsf_bytes, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _backward = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse header length
    let _header_length_type = parse(&vsf_bytes, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;

    // Skip optional file length (l field) if present
    if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b'l' {
        let _ = parse(&vsf_bytes, &mut pointer)
            .map_err(|e| format!("Failed to parse file length: {}", e))?;
    }

    // Skip creation time (ef5 - always present in version 3+)
    skip_optional_creation_time(&vsf_bytes, &mut pointer)?;

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
/// This function combines compute_file_hash and write_file_hash. New code should use the separate functions instead.
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
            // pos now points AFTER the hash Calculate where the hash bytes started
            let hash_start = pos - hash_bytes.len();
            Ok(hash_start)
        }
        _ => Err("Expected BLAKE3 hash type (hb)".to_string()),
    }
}

/// Write all header field signatures from the parsed header into their placeholders
///
/// This function scans the header for all signature placeholders (ge/gp/gr with zeros) and writes the actual signature bytes from the parsed header fields.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes with signature placeholders (zeros)
///
/// # Returns
/// Modified VSF bytes with actual signature values written Write header field signatures from a provided list (instead of parsing) This is used when we already have the signature bytes extracted before flattening
fn write_header_field_signatures_from_list(
    mut vsf_bytes: Vec<u8>,
    field_signatures: Vec<Option<VsfType>>,
) -> Result<Vec<u8>, String> {
    // Parse the header just to get header_end (don't extract signatures from it)
    let header = parse_full_header(&vsf_bytes)?;

    // Extract signature bytes from the provided list
    let mut signatures = Vec::new();
    for sig_vsf in field_signatures.into_iter().flatten() {
        let sig_bytes = match sig_vsf {
            VsfType::ge(bytes) => bytes,
            VsfType::gp(bytes) => bytes,
            VsfType::gr(bytes) => bytes,
            _ => continue,
        };
        signatures.push(sig_bytes);
    }

    // Now scan header for signature placeholders and write them We scan only up to header_end
    let header_end = header.header_end;
    let mut sig_index = 0;

    let mut pos = 0;
    while pos < header_end - 1 && sig_index < signatures.len() {
        if vsf_bytes[pos] == b'g'
            && (vsf_bytes[pos + 1] == b'e'
                || vsf_bytes[pos + 1] == b'p'
                || vsf_bytes[pos + 1] == b'r')
        {
            // Found potential signature marker
            let mut test_ptr = pos;
            if let Ok(sig_type) = parse(&vsf_bytes, &mut test_ptr) {
                match sig_type {
                    VsfType::ge(test_bytes) | VsfType::gp(test_bytes) | VsfType::gr(test_bytes) => {
                        // Check if this is all zeros (placeholder)
                        if test_bytes.iter().all(|&b| b == 0)
                            && test_bytes.len() == signatures[sig_index].len()
                        {
                            // Found a placeholder - write the signature
                            let sig_start = test_ptr - test_bytes.len();
                            vsf_bytes[sig_start..sig_start + signatures[sig_index].len()]
                                .copy_from_slice(&signatures[sig_index]);
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
/// * `vsf_bytes` - Complete VSF file bytes * `section` - Name of the section to sign (e.g., "raw") * `signing_key` - Ed25519 signing key bytes (must be valid SigningKey)
///
/// # Returns
/// Modified VSF bytes with section signature in header field definition
///
/// # Example
/// ```ignore
/// use ed25519_dalek::SigningKey; use rand::rngs::OsRng;
///
/// let signing_key = SigningKey::generate(&mut OsRng); let bytes = sign_section(bytes, "raw", signing_key.as_bytes())?;
/// ```
/// Sign a VSF file and add the signature to the header.
///
/// # VSF Signature Scheme
///
/// VSF uses a two-stage hash scheme for signatures:
///
/// 1. **Provenance hash (hp)**: Computed over the entire file with all crypto fields zeroed (hp=0, sig=0). This is the immutable content identity.
///
/// 2. **Signature**: Signs the hash of the entire file WITH provenance filled in but signature still zeroed. This binds the signer to both the content AND its provenance.
///
/// ```text
/// Creation: file[hp=0, sig=0] → BLAKE3 → hp file[hp=✓, sig=0] → BLAKE3 → sign() → sig
///
/// Update (re-signing): file[hp=✓, sig=0] → BLAKE3 → sign() → new_sig (hp stays the same - it's the original content identity)
/// ```
///
/// # Signature vs Rolling Hash
///
/// VSF supports TWO mutually exclusive integrity mechanisms:
/// - **hb (rolling hash)**: Anonymous integrity - just verifies content hasn't changed
/// - **ge (signature)**: Authenticated integrity - proves WHO created/signed it
///
/// When signing, we use signature (ge) instead of rolling hash (hb). Both cover file integrity; signature additionally proves authorship.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file with hp already computed * `section_name` - Section to associate signature with in header * `signing_key` - Ed25519 signing key (32 bytes)
///
/// # Returns
/// VSF file bytes with signature added to header
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

    // Parse complete header to verify section exists
    let header = parse_full_header(&vsf_bytes)?;
    let _section_field = header
        .fields
        .iter()
        .find(|f| f.name == section_name)
        .ok_or_else(|| format!("Section '{}' not found", section_name))?;

    // Add signature placeholder to header, then compute hash of file-with-provenance The signature placeholder ensures header size is stable before we hash
    let mut new_fields = header.fields.clone();
    for field in &mut new_fields {
        if field.name == section_name {
            // 64-byte Ed25519 signature placeholder (zeros)
            field.signature = Some(VsfType::ge(vec![0u8; 64]));
            break;
        }
    }

    // Rebuild file with signature placeholder (no rolling hash - signature replaces it)
    let file_with_placeholder = rebuild_with_header(
        &vsf_bytes,
        new_fields.clone(),
        header.version,
        header.backward_compat,
        header.header_end,
        false, // No rolling hash - signature provides integrity + authentication
    )?;

    // Compute hash of file with provenance filled, signature zeroed This is what we sign - binding signer to content AND its provenance
    let hash_to_sign = compute_signature_hash(&file_with_placeholder, section_name)?;

    // Sign the hash
    let signature = signing_key.sign(&hash_to_sign);
    let sig_bytes = signature.to_bytes().to_vec();

    // Write actual signature into the placeholder
    write_section_signature(file_with_placeholder, section_name, &sig_bytes)
}

/// Compute the hash that gets signed for a section.
///
/// This hashes the entire file with:
/// - Provenance hash (hp) filled in (binds signature to content identity)
/// - Signature field zeroed (the thing we're computing)
///
/// The result is what the signer signs, proving they attest to this specific content with this specific provenance.
#[cfg(feature = "crypto")]
fn compute_signature_hash(vsf_bytes: &[u8], section_name: &str) -> Result<[u8; 32], String> {
    // Find and zero the signature bytes for this section
    let header = parse_full_header(vsf_bytes)?;

    let section_field = header
        .fields
        .iter()
        .find(|f| f.name == section_name)
        .ok_or_else(|| format!("Section '{}' not found", section_name))?;

    // Find signature position in the header
    let sig_position = match &section_field.signature {
        Some(VsfType::ge(_)) => {
            // Need to find where this signature is in the raw bytes
            find_section_signature_position(vsf_bytes, section_name)?
        }
        _ => return Err("Section has no signature placeholder".to_string()),
    };

    // Clone and zero the signature bytes
    let mut temp_bytes = vsf_bytes.to_vec();
    for i in 0..64 {
        temp_bytes[sig_position + i] = 0;
    }

    // Hash the file with signature zeroed, provenance intact
    Ok(*blake3::hash(&temp_bytes).as_bytes())
}

/// Find the byte position of a section's signature value in the header.
#[cfg(feature = "crypto")]
fn find_section_signature_position(vsf_bytes: &[u8], section_name: &str) -> Result<usize, String> {
    // Parse thru header to find the section's signature field
    let mut pointer = 4; // Skip magic "RÅ<"

    // VSF design: dispatch on byte until we hit '>' See byte → parse it → repeat
    while pointer < vsf_bytes.len() && vsf_bytes[pointer] != b'>' {
        let byte = vsf_bytes[pointer];

        if byte == b'(' {
            // Header field - parse it and look for our section's signature
            pointer += 1; // skip '('

            // Parse field name
            let field_name = match parse(vsf_bytes, &mut pointer).map_err(|e| e.to_string())? {
                VsfType::d(name) => name,
                _ => return Err("Expected field name".to_string()),
            };

            // Skip colon separator if present
            if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b':' {
                pointer += 1;
            }

            // Parse field values until ')'
            let mut found_sig_position = None;
            while pointer < vsf_bytes.len() && vsf_bytes[pointer] != b')' {
                // Skip comma separators
                if vsf_bytes[pointer] == b',' {
                    pointer += 1;
                    continue;
                }

                let value = parse(vsf_bytes, &mut pointer).map_err(|e| e.to_string())?;

                // If this is a signature and we're in the right section
                if field_name == section_name {
                    if let VsfType::ge(sig_bytes) = value {
                        found_sig_position = Some(pointer - sig_bytes.len());
                    }
                }
            }

            if pointer < vsf_bytes.len() && vsf_bytes[pointer] == b')' {
                pointer += 1; // skip ')'
            }

            // Check if this was our target section
            if field_name == section_name {
                return found_sig_position
                    .ok_or_else(|| format!("Section '{}' has no signature field", section_name));
            }
        } else {
            // Any other type marker - just parse and continue
            let _ = parse(vsf_bytes, &mut pointer)
                .map_err(|e| format!("Header parse error at byte {}: {}", pointer, e))?;
        }
    }

    Err(format!("Section '{}' not found in header", section_name))
}

/// Write signature bytes into a section's signature placeholder.
#[cfg(feature = "crypto")]
fn write_section_signature(
    mut vsf_bytes: Vec<u8>,
    section_name: &str,
    signature: &[u8],
) -> Result<Vec<u8>, String> {
    if signature.len() != 64 {
        return Err(format!(
            "Signature must be 64 bytes, got {}",
            signature.len()
        ));
    }

    let sig_position = find_section_signature_position(&vsf_bytes, section_name)?;
    vsf_bytes[sig_position..sig_position + 64].copy_from_slice(signature);

    Ok(vsf_bytes)
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
/// * `vsf_bytes` - Complete VSF file bytes * `section_name` - Name of the section (e.g., "sensitive") * `algorithm` - Encryption algorithm ID (e.g., b'c' for ChaCha20) * `encryption_key` - Encryption key bytes
///
/// # Returns
/// Modified VSF bytes with encryption metadata in header field
///
/// # Example
/// ```ignore
/// // 1. Encrypt data first let encrypted_data = encrypt_with_chacha20(&plaintext, &key);
///
/// // 2. Build VSF with encrypted data let vsf = VsfBuilder::new() .add_section("sensitive", vec![("data", encrypted_data)]) .build()?;
///
/// // 3. Add encryption metadata to header let vsf = add_encryption_metadata(vsf, "sensitive", b'c', &key)?;
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

    // Rebuild file with modified header Preserve original rolling hash setting
    rebuild_with_header(
        &vsf_bytes,
        new_fields,
        header.version,
        header.backward_compat,
        header.header_end,
        header.rolling_hash.is_some(),
    )
}

/// Check if a VSF file is an unmodified original by verifying the provenance hash (hp).
///
/// The provenance hash identifies the original content. This function verifies that the file content matches its claimed identity - i.e., it hasn't been modified since creation.
///
/// Note: This does NOT verify integrity of signed/updated files. For files that have been signed or updated, use signature verification instead. The provenance hash is computed with crypto fields zeroed, so it represents the original content identity regardless of later signatures.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns `Ok(())` if file is an unmodified original, `Err` if modified or invalid
pub fn is_original(vsf_bytes: &[u8]) -> Result<(), String> {
    use crate::file_format::VsfHeader;

    let computed_hash = compute_provenance_hash(vsf_bytes)?;

    let (header, _) = VsfHeader::decode(vsf_bytes)?;

    let stored_hash = match &header.provenance_hash {
        VsfType::hp(hash_bytes) if hash_bytes.len() == 32 => hash_bytes,
        VsfType::hp(hash_bytes) => {
            return Err(format!(
                "Invalid provenance hash size: expected 32 bytes, found {}",
                hash_bytes.len()
            ))
        }
        _ => return Err("Missing provenance hash (hp) in header".to_string()),
    };

    if computed_hash.as_slice() == stored_hash.as_slice() {
        Ok(())
    } else {
        Err("File has been modified - provenance hash does not match content".to_string())
    }
}

/// Verify the rolling hash (hb) in a VSF header.
///
/// The rolling hash covers the entire file including any modifications/signatures, providing integrity verification for the current file state.
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file bytes
///
/// # Returns `Ok(())` if hash is valid, `Err` with description if invalid or missing
pub fn verify_file_hash(vsf_bytes: &[u8]) -> Result<(), String> {
    use crate::file_format::VsfHeader;

    let computed_hash = compute_file_hash(vsf_bytes)?;

    let (header, _) = VsfHeader::decode(vsf_bytes)?;

    let stored_hash = match &header.rolling_hash {
        Some(VsfType::hb(hash_bytes)) if hash_bytes.len() == 32 => hash_bytes,
        Some(VsfType::hb(hash_bytes)) => {
            return Err(format!(
                "Invalid rolling hash size: expected 32 bytes, found {}",
                hash_bytes.len()
            ))
        }
        _ => return Err("Missing rolling hash (hb) in header".to_string()),
    };

    if computed_hash.as_slice() == stored_hash.as_slice() {
        Ok(())
    } else {
        Err("Rolling hash verification failed: file has been modified".to_string())
    }
}

/// Sign entire VSF file with Ed25519 (header-level signature)
///
/// This function signs the FILE HASH (not the provenance hash directly). The file must already be built with:
/// - `ke` (Ed25519 pubkey) in header
/// - `ge` placeholder (64 zeros) in header
/// - `hp` either as placeholder (auto-computed) or explicit value
///
/// # Flow:
/// 1. If hp is zeros, compute provenance hash and patch it
/// 2. Compute file hash = BLAKE3(file with hp filled, ge still zeros)
/// 3. Sign the file hash with Ed25519
/// 4. Patch ge with signature
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file with ke and ge placeholder * `signing_key` - Ed25519 signing key (32 bytes)
///
/// # Returns
/// VSF file bytes with hp and ge filled in
///
/// # Security
/// Signs the FILE HASH, not the provenance hash directly. This prevents extension attacks where content could be added after the signed portion.
#[cfg(feature = "crypto")]
pub fn sign_file(mut vsf_bytes: Vec<u8>, signing_key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use ed25519_dalek::{Signer, SigningKey};

    // Parse signing key
    let signing_key = SigningKey::from_bytes(signing_key);

    // Check if hp needs to be computed (all zeros = placeholder)
    let hp_info = find_header_hp(&vsf_bytes)?;
    if hp_info.is_placeholder {
        // Compute and write provenance hash
        let hp_hash = compute_provenance_hash(&vsf_bytes)?;
        vsf_bytes[hp_info.value_start..hp_info.value_start + 32].copy_from_slice(&hp_hash);
    }

    // Find ge signature placeholder
    let ge_info = find_header_ge(&vsf_bytes)?;
    if !ge_info.is_placeholder {
        return Err("ge is already filled - file may already be signed".to_string());
    }

    // Compute file hash with hp filled, ge still zeros This is what we sign
    let file_hash = blake3::hash(&vsf_bytes);

    // Sign the file hash
    let signature = signing_key.sign(file_hash.as_bytes());
    let sig_bytes = signature.to_bytes();

    // Patch ge with signature
    vsf_bytes[ge_info.value_start..ge_info.value_start + 64].copy_from_slice(&sig_bytes);

    Ok(vsf_bytes)
}

/// Verify header-level Ed25519 signature
///
/// # Flow:
/// 1. Extract ke (pubkey) and ge (signature) from header
/// 2. Zero out ge bytes to get signing input
/// 3. Compute file hash of that
/// 4. Verify signature against file hash using ke
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file with ke and ge in header
///
/// # Returns `Ok(true)` if signature is valid `Ok(false)` if signature is invalid `Err` if file format is wrong or missing ke/ge
#[cfg(feature = "crypto")]
pub fn verify_file_signature(vsf_bytes: &[u8]) -> Result<bool, String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Find ke (public key)
    let ke_info = find_header_ke(vsf_bytes)?;
    if ke_info.value_len != 32 {
        return Err(format!(
            "Expected 32-byte Ed25519 pubkey, got {}",
            ke_info.value_len
        ));
    }
    let pubkey_bytes: [u8; 32] = vsf_bytes[ke_info.value_start..ke_info.value_start + 32]
        .try_into()
        .map_err(|_| "Failed to extract pubkey bytes")?;

    // Find ge (signature)
    let ge_info = find_header_ge(vsf_bytes)?;
    if ge_info.value_len != 64 {
        return Err(format!(
            "Expected 64-byte Ed25519 signature, got {}",
            ge_info.value_len
        ));
    }
    let sig_bytes: [u8; 64] = vsf_bytes[ge_info.value_start..ge_info.value_start + 64]
        .try_into()
        .map_err(|_| "Failed to extract signature bytes")?;

    // Check if signature is all zeros (not signed)
    if sig_bytes.iter().all(|&b| b == 0) {
        return Err("Signature is all zeros - file not signed".to_string());
    }

    // Zero out ge to compute what was signed
    let mut temp_bytes = vsf_bytes.to_vec();
    for i in 0..64 {
        temp_bytes[ge_info.value_start + i] = 0;
    }

    // Compute file hash
    let file_hash = blake3::hash(&temp_bytes);

    // Verify signature
    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_bytes).map_err(|e| format!("Invalid pubkey: {}", e))?;
    let signature = Signature::from_bytes(&sig_bytes);

    match verifying_key.verify(file_hash.as_bytes(), &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Extract the signer's public key (ke) from a signed VSF file
///
/// # Arguments
/// * `vsf_bytes` - Complete VSF file with ke in header
///
/// # Returns 32-byte Ed25519 public key
#[cfg(feature = "crypto")]
pub fn extract_signer_pubkey(vsf_bytes: &[u8]) -> Result<[u8; 32], String> {
    let ke_info = find_header_ke(vsf_bytes)?;
    if ke_info.value_len != 32 {
        return Err(format!(
            "Expected 32-byte Ed25519 pubkey, got {}",
            ke_info.value_len
        ));
    }
    vsf_bytes[ke_info.value_start..ke_info.value_start + 32]
        .try_into()
        .map_err(|_| "Failed to extract pubkey bytes".to_string())
}

/// Info about a header field location
#[allow(dead_code)]
struct HeaderFieldInfo {
    #[allow(dead_code)]
    marker_pos: usize, // Position of type marker (e.g., 'h' for hp)
    value_start: usize,   // Position where value bytes start
    value_len: usize,     // Length of value bytes
    is_placeholder: bool, // True if all value bytes are zero
}

/// Find hp (provenance hash) in header
#[allow(dead_code)]
fn find_header_hp(data: &[u8]) -> Result<HeaderFieldInfo, String> {
    if data.len() < 4 || &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic".to_string());
    }

    let mut ptr = 4;

    // Skip version, backward compat, header length
    let _ = parse(data, &mut ptr).map_err(|e| format!("version: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("backward: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("header_len: {}", e))?;

    // Skip optional file length (L) if present
    if ptr < data.len() && data[ptr] == b'l' {
        let _ = parse(data, &mut ptr).map_err(|e| format!("file_len: {}", e))?;
    }

    // Skip creation time
    skip_optional_creation_time(data, &mut ptr)?;

    // Now should be at hp
    let marker_pos = ptr;
    if ptr >= data.len() || data[ptr] != b'h' {
        return Err(format!("Expected hp at position {}", ptr));
    }

    let hp_type = parse(data, &mut ptr).map_err(|e| format!("hp: {}", e))?;
    match hp_type {
        VsfType::hp(bytes) => {
            let value_start = ptr - bytes.len();
            let is_placeholder = bytes.iter().all(|&b| b == 0);
            Ok(HeaderFieldInfo {
                marker_pos,
                value_start,
                value_len: bytes.len(),
                is_placeholder,
            })
        }
        _ => Err("Expected hp type".to_string()),
    }
}

/// Find ke (Ed25519 pubkey) in header
#[allow(dead_code)]
fn find_header_ke(data: &[u8]) -> Result<HeaderFieldInfo, String> {
    if data.len() < 4 || &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic".to_string());
    }

    let mut ptr = 4;

    // Skip version, backward compat, header length
    let _ = parse(data, &mut ptr).map_err(|e| format!("version: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("backward: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("header_len: {}", e))?;

    // Skip optional file length (L) if present
    if ptr < data.len() && data[ptr] == b'l' {
        let _ = parse(data, &mut ptr).map_err(|e| format!("file_len: {}", e))?;
    }

    // Skip creation time
    skip_optional_creation_time(data, &mut ptr)?;

    // Skip hp
    let _ = parse(data, &mut ptr).map_err(|e| format!("hp: {}", e))?;

    // Now should be at ke (if present)
    let marker_pos = ptr;
    if ptr >= data.len() || data[ptr] != b'k' {
        return Err("No ke found in header".to_string());
    }

    let ke_type = parse(data, &mut ptr).map_err(|e| format!("ke: {}", e))?;
    match ke_type {
        VsfType::ke(bytes) => {
            let value_start = ptr - bytes.len();
            let is_placeholder = bytes.iter().all(|&b| b == 0);
            Ok(HeaderFieldInfo {
                marker_pos,
                value_start,
                value_len: bytes.len(),
                is_placeholder,
            })
        }
        _ => Err(crate::type_mismatch_err!("Expected ke type", ke_type)),
    }
}

/// Find ge (Ed25519 signature) in header
#[allow(dead_code)]
fn find_header_ge(data: &[u8]) -> Result<HeaderFieldInfo, String> {
    if data.len() < 4 || &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic".to_string());
    }

    let mut ptr = 4;

    // Skip version, backward compat, header length
    let _ = parse(data, &mut ptr).map_err(|e| format!("version: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("backward: {}", e))?;
    let _ = parse(data, &mut ptr).map_err(|e| format!("header_len: {}", e))?;

    // Skip optional file length (L) if present
    if ptr < data.len() && data[ptr] == b'l' {
        let _ = parse(data, &mut ptr).map_err(|e| format!("file_len: {}", e))?;
    }

    // Skip creation time
    skip_optional_creation_time(data, &mut ptr)?;

    // Skip hp
    let _ = parse(data, &mut ptr).map_err(|e| format!("hp: {}", e))?;

    // Skip ke (must be present before ge)
    if ptr >= data.len() || data[ptr] != b'k' {
        return Err("No ke found before ge".to_string());
    }
    let _ = parse(data, &mut ptr).map_err(|e| format!("ke: {}", e))?;

    // Now should be at ge
    let marker_pos = ptr;
    if ptr >= data.len() || data[ptr] != b'g' {
        return Err("No ge found in header".to_string());
    }

    let ge_type = parse(data, &mut ptr).map_err(|e| format!("ge: {}", e))?;
    match ge_type {
        VsfType::ge(bytes) => {
            let value_start = ptr - bytes.len();
            let is_placeholder = bytes.iter().all(|&b| b == 0);
            Ok(HeaderFieldInfo {
                marker_pos,
                value_start,
                value_len: bytes.len(),
                is_placeholder,
            })
        }
        _ => Err(crate::type_mismatch_err!("Expected ge type", ge_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::RawImageBuilder;
    use crate::types::BitPackedTensor;

    #[cfg(feature = "crypto")]
    #[test]
    fn test_sign_section_with_metadata() {
        use crate::vsf_builder::{SectionMeta, VsfBuilder};

        // Build a simple VSF file similar to what Photon does
        let meta = SectionMeta::new(
            VsfType::ke(vec![0u8; 32]), // Ed25519 device public key
        );

        let encrypted = vec![0u8; 100]; // Fake encrypted data

        let unsigned_bytes = VsfBuilder::new()
            .add_section_with_meta(
                "announce",
                vec![("payload".to_string(), VsfType::v(b'e', encrypted))],
                meta,
            )
            .build()
            .expect("Failed to build VSF");

        println!("Built unsigned VSF: {} bytes", unsigned_bytes.len());

        // Print hex dump for debugging
        println!("Hex dump:");
        for (i, b) in unsigned_bytes.iter().enumerate() {
            if i % 32 == 0 {
                println!();
                print!("{:04x}: ", i);
            }
            print!("{:02x} ", b);
        }
        println!();

        // Try to sign it
        let signing_key = [1u8; 32]; // Non-zero key for valid Ed25519
        match sign_section(unsigned_bytes.clone(), "announce", &signing_key) {
            Ok(signed) => {
                println!("\nSigned VSF: {} bytes", signed.len());
                assert!(
                    signed.len() > unsigned_bytes.len(),
                    "Signed should be larger"
                );
            }
            Err(e) => {
                panic!("Sign error: {}", e);
            }
        }
    }

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

    #[cfg(feature = "crypto")]
    #[test]
    fn test_sign_and_verify_file() {
        use crate::vsf_builder::VsfBuilder;
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        // Generate a keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let pubkey = signing_key.verifying_key().to_bytes();

        // Build VSF with ke + ge placeholder
        let unsigned = VsfBuilder::new()
            .signature_ed25519(pubkey, [0u8; 64]) // ke + ge placeholder
            .add_section("test", vec![("value".to_string(), VsfType::u(42, false))])
            .build()
            .expect("Failed to build unsigned VSF");

        println!("Unsigned VSF: {} bytes", unsigned.len());

        // Sign the file
        let signed =
            sign_file(unsigned.clone(), signing_key.as_bytes()).expect("Failed to sign file");

        println!("Signed VSF: {} bytes", signed.len());

        // Verify the signature
        let valid = verify_file_signature(&signed).expect("Failed to verify signature");
        assert!(valid, "Signature should be valid");

        // Corrupt a byte and verify it fails
        let mut corrupted = signed.clone();
        corrupted[signed.len() - 10] ^= 0xFF;
        let valid = verify_file_signature(&corrupted).expect("Should parse corrupted file");
        assert!(!valid, "Corrupted file should fail verification");

        // Extract signer pubkey
        let extracted = extract_signer_pubkey(&signed).expect("Failed to extract pubkey");
        assert_eq!(extracted, pubkey, "Extracted pubkey should match");
    }

    #[cfg(feature = "crypto")]
    #[test]
    fn test_sign_file_with_custom_provenance() {
        use crate::vsf_builder::VsfBuilder;
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        // Generate a keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let pubkey = signing_key.verifying_key().to_bytes();

        // Custom provenance (e.g., ceremony session ID)
        let ceremony_hp = *blake3::hash(b"test_ceremony").as_bytes();

        // Build VSF with custom provenance + ke + ge placeholder
        let unsigned = VsfBuilder::new()
            .provenance_hash(ceremony_hp)
            .signature_ed25519(pubkey, [0u8; 64])
            .add_section(
                "clutch_full_offer",
                vec![
                    ("lower".to_string(), VsfType::hb(vec![1u8; 32])),
                    ("higher".to_string(), VsfType::hb(vec![2u8; 32])),
                ],
            )
            .build()
            .expect("Failed to build unsigned VSF");

        // Sign the file (should NOT recompute hp since it's not all zeros)
        let signed = sign_file(unsigned, signing_key.as_bytes()).expect("Failed to sign file");

        // Verify
        let valid = verify_file_signature(&signed).expect("Failed to verify signature");
        assert!(valid, "Signature should be valid with custom provenance");
    }
}
