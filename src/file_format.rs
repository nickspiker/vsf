//! VSF file format with headers and hierarchical fields
//!
//! Binary structure (following basecalc pattern):
//! ```text
//! RÅ<                                    Magic + header start
//!   b[header_length_bytes]               Header length in BYTES
//!   z[version]                           Version number
//!   y[backward_compat]                   Backward compatibility version
//!   hb[256][hash]                        File integrity hash (BLAKE3)
//!   n[field_count]                       Number of header field definitions
//!
//!   (d[section_name] h?[hash] g?[sig] k?[key] o[offset] b[size] n[count])  Header field (section pointer)
//!   ...
//! >                                      Header end
//!
//! [                                      Section start (if n > 0)
//!   d[section_name]                      Section name
//!   (d[field_name]:[value])              Field definition (leaf)
//!   (d[field_name] o[offset] b[size] n[count])  Nested section (branch)
//!   ...
//! ]                                      Section end
//!
//! [raw_bytes...]                         Unboxed data (if n = 0)
//! ```

use crate::prelude::*;
use crate::types::VsfType;

/// Validate VSF section or field name
///
/// Rules:
/// - Must start with lowercase letter
/// - Can contain: lowercase letters, digits, underscores
/// - Dots allowed for hierarchy (each segment follows same rules)
/// - No trailing/leading dots, no consecutive dots
/// - No trailing/leading underscores, no consecutive underscores
/// - Regex: ^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$
///
/// # Examples
/// ```
/// use vsf::file_format::validate_name;
/// assert!(validate_name("camera").is_ok());
/// assert!(validate_name("camera_sensor").is_ok());
/// assert!(validate_name("camera.sensor").is_ok());
/// assert!(validate_name("iso_speed_100").is_ok());
/// assert!(validate_name("Camera").is_err());       // uppercase
/// assert!(validate_name("9camera").is_err());      // starts with digit
/// assert!(validate_name(".camera").is_err());      // starts with dot
/// assert!(validate_name("camera.").is_err());      // ends with dot
/// assert!(validate_name("camera..sensor").is_err()); // double dot
/// assert!(validate_name("camera__sensor").is_err()); // double underscore
/// ```
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    // Check for leading/trailing dots or underscores
    if name.starts_with('.') || name.ends_with('.') {
        return Err(format!(
            "Invalid name '{}' - cannot start or end with dot",
            name
        ));
    }
    if name.starts_with('_') || name.ends_with('_') {
        return Err(format!(
            "Invalid name '{}' - cannot start or end with underscore",
            name
        ));
    }

    // Check for consecutive dots or underscores
    if name.contains("..") {
        return Err(format!(
            "Invalid name '{}' - cannot contain consecutive dots",
            name
        ));
    }
    if name.contains("__") {
        return Err(format!(
            "Invalid name '{}' - cannot contain consecutive underscores",
            name
        ));
    }

    // Split by dots and validate each segment
    for segment in name.split('.') {
        if segment.is_empty() {
            return Err(format!("Invalid name '{}' - empty segment", name));
        }

        // First character must be lowercase letter
        let first = segment.chars().next().unwrap();
        if !first.is_ascii_lowercase() {
            return Err(format!(
                "Invalid name '{}' - segment '{}' must start with lowercase letter (found '{}')",
                name, segment, first
            ));
        }

        // Rest can be lowercase, digits, underscores
        for ch in segment.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
                return Err(format!(
                    "Invalid name '{}' - use lowercase letters, digits, and underscores only (found '{}')",
                    name, ch
                ));
            }
        }
    }

    Ok(())
}

/// VSF file header
#[derive(Debug, Clone)]
pub struct VsfHeader {
    pub version: usize,
    pub backward_compat: usize,
    pub file_length: usize, // Total file length in bytes (for TCP streaming)
    /// Creation timestamp.
    /// `None` when the device cannot know what time it is (no RTC, no network, etc) — in that case the wire-format header omits the `e` field entirely.
    /// `Some(VsfType::e(…))` carries the standard `eu6`/`ef5`/`ef6` value.
    pub creation_time: Option<VsfType>,
    pub provenance_hash: VsfType, // Required: BLAKE3 hash of immutable content (hp)
    pub rolling_hash: Option<VsfType>, // Optional: BLAKE3 hash of current state (hb) - OR signature
    pub signer_pubkey: Option<VsfType>, // Optional: Ed25519 public key (ke) - for signed files
    pub signature: Option<VsfType>, // Optional: Ed25519 signature (ge) - replaces rolling_hash
    pub fields: Vec<HeaderField>,
}

/// Header field definition (section pointer with positional values)
/// Format: (d[section_name] o[offset] b[size] n[count])
/// Note: Header fields use POSITIONAL values (no colons or commas)
/// For inline fields (no section body): (d[name]:value,value,...)
#[derive(Debug, Clone)]
pub struct HeaderField {
    pub name: String,
    pub hash: Option<VsfType>, // h: optional hash of section data (VsfType::h)
    pub signature: Option<VsfType>, // g: optional signature of section data (VsfType::g)
    pub key: Option<VsfType>,  // k: optional cryptographic key (VsfType::k)
    pub offset_bytes: usize,   // Offset in bytes (byte-aligned)
    pub size_bytes: usize,     // Size in bytes (byte-aligned)
    pub child_count: usize,    // 0 = unboxed blob, N = N structured children
    pub inline_values: Vec<VsfType>, // For header-only fields (no section body)
}

impl VsfHeader {
    /// Create a new header with no creation_time set.
    /// Callers that know what time it is should set `header.creation_time = Some(VsfType::e(EtType::e6(...)))` before encoding.
    /// Devices without a clock leave it as `None` and the encoded header omits the `e` field entirely.
    pub fn new(version: usize, backward_compat: usize) -> Self {
        Self {
            version,
            backward_compat,
            file_length: 0, // Placeholder, filled during build
            creation_time: None,
            provenance_hash: VsfType::hp(vec![0u8; 32]), // Placeholder, filled during build
            rolling_hash: None,
            signer_pubkey: None,
            signature: None,
            fields: Vec::new(),
        }
    }

    /// Add a header field definition (section pointer)
    pub fn add_field(&mut self, field: HeaderField) {
        self.fields.push(field);
    }

    /// Encode header to bytes (following basecalc pattern)
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut header = Vec::new();

        // Magic number
        header.extend_from_slice("RÅ".as_bytes());

        // Header start marker
        header.push(b'<');

        // Version (MUST come first to determine encoding)
        header.extend_from_slice(&VsfType::z(self.version).flatten());

        // Backward compatibility
        header.extend_from_slice(&VsfType::y(self.backward_compat).flatten());

        // Header length placeholder (after version/backward_compat)
        let header_length_placeholder = VsfType::b(0, true).flatten();
        header.extend_from_slice(&header_length_placeholder);

        // File length placeholder (for TCP streaming)
        let file_length_placeholder = VsfType::l(0, true).flatten();
        header.extend_from_slice(&file_length_placeholder);

        // Creation time — emitted only if the caller set it. Devices without a clock omit the field entirely.
        if let Some(ref et) = self.creation_time {
            header.extend_from_slice(&et.flatten());
        }

        // Provenance hash (always present)
        header.extend_from_slice(&self.provenance_hash.flatten());

        // Rolling hash OR signature (mutually exclusive)
        // If signature present, include ke (pubkey) + ge (signature) instead of hb
        if let Some(ref pubkey) = self.signer_pubkey {
            header.extend_from_slice(&pubkey.flatten());
        }
        if let Some(ref sig) = self.signature {
            header.extend_from_slice(&sig.flatten());
        } else if let Some(ref hash) = self.rolling_hash {
            header.extend_from_slice(&hash.flatten());
        }

        // Header field count (number of section pointers)
        header.extend_from_slice(&VsfType::n(self.fields.len()).flatten());

        // Header field definitions (section pointers with : and , separators)
        for field in &self.fields {
            header.push(b'(');

            // Section name
            header.extend_from_slice(&VsfType::d(field.name.clone()).flatten());

            // Separator after section name
            header.push(b':');

            // Optional hash (VsfType::h with algorithm)
            if let Some(ref hash_type) = field.hash {
                header.extend_from_slice(&hash_type.flatten());
                header.push(b',');
            }

            // Optional signature (VsfType::g with algorithm)
            if let Some(ref sig_type) = field.signature {
                header.extend_from_slice(&sig_type.flatten());
                header.push(b',');
            }

            // Optional key (VsfType::k with algorithm)
            if let Some(ref key_type) = field.key {
                header.extend_from_slice(&key_type.flatten());
                header.push(b',');
            }

            // Offset (in bytes)
            header.extend_from_slice(&VsfType::o(field.offset_bytes).flatten());
            header.push(b',');

            // Size (in bytes)
            header.extend_from_slice(&VsfType::b(field.size_bytes, false).flatten());
            header.push(b',');

            // Child count
            header.extend_from_slice(&VsfType::n(field.child_count).flatten());

            header.push(b')');
        }

        // Header end marker
        header.push(b'>');

        Ok(header)
    }

    /// Decode a VSF header from bytes
    ///
    /// Parses the binary header structure and returns a VsfHeader instance. Returns the parsed header and the number of bytes consumed.
    ///
    /// # Format
    /// ```text
    /// RÅ<                          Magic + header start
    ///   z[version]                 Version number
    ///   y[backward_compat]         Backward compatibility version
    ///   b[header_length_bytes]     Header length in BYTES
    ///   e[creation_time]           Creation timestamp (eu6 default, ef5/ef6 legacy)
    ///   hp[hash]                   Provenance hash (BLAKE3)
    ///   hb[hash]?                  Optional rolling hash (BLAKE3)
    ///   n[field_count]             Number of header fields
    ///   (...)                      Header fields
    /// >                            Header end
    /// ```
    pub fn decode(data: &[u8]) -> Result<(Self, usize), String> {
        use crate::decoding::parse::parse;

        // Check magic number "RÅ<" (R=0x52, Å=0xC3,0x85)
        if data.len() < 4 {
            return Err("Data too short for VSF header".to_string());
        }
        if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
            return Err(format!(
                "Invalid VSF magic number (expected 'RÅ<', found '{}{}{}')",
                data[0] as char, data[1] as char, data[2] as char
            ));
        }

        let mut ptr = 4; // After "RÅ<"

        // Parse version (z)
        let version_type =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse version: {}", e))?;
        let version = match version_type {
            VsfType::z(v) => v,
            _ => return Err(format!("Expected version (z), got {:?}", version_type)),
        };

        // Parse backward compatibility (y)
        let backward_compat_type =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse backward_compat: {}", e))?;
        let backward_compat = match backward_compat_type {
            VsfType::y(v) => v,
            _ => {
                return Err(format!(
                    "Expected backward_compat (y), got {:?}",
                    backward_compat_type
                ))
            }
        };

        // Parse header length (b) - we validate but don't use it for parsing
        let header_length_type =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse header_length: {}", e))?;
        let _header_length = match header_length_type {
            VsfType::b(len, _) => len,
            _ => {
                return Err(format!(
                    "Expected header_length (b), got {:?}",
                    header_length_type
                ))
            }
        };

        // Parse optional file length (l) - for TCP streaming (default on, but optional for compat)
        let file_length = if ptr < data.len() && data[ptr] == b'l' {
            match parse(data, &mut ptr) {
                Ok(VsfType::l(len, _)) => len,
                Ok(other) => {
                    return Err(format!(
                        "Expected l value after 'l' marker, got {:?}",
                        other
                    ))
                }
                Err(e) => return Err(format!("Failed to parse file_length: {}", e)),
            }
        } else {
            0 // No l field present - use 0 to indicate unknown
        };

        // Parse optional creation time (e). Devices without a clock omit the field entirely; peek the marker byte and only parse if it is `e`.
        let creation_time = if ptr < data.len() && data[ptr] == b'e' {
            let v = parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse creation_time: {}", e))?;
            if !matches!(v, VsfType::e(_)) {
                return Err(format!("Expected creation_time (e), got {:?}", v));
            }
            Some(v)
        } else {
            None
        };

        // Parse provenance hash (hp)
        let provenance_hash =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse provenance_hash: {}", e))?;
        if !matches!(provenance_hash, VsfType::hp(_)) {
            return Err(format!(
                "Expected provenance_hash (hp), got {:?}",
                provenance_hash
            ));
        }

        // Parse remaining header until '>' using VSF dispatch pattern
        // See byte → dispatch to parser → repeat
        let mut rolling_hash = None;
        let mut signer_pubkey = None;
        let mut signature = None;
        let mut fields = Vec::new();

        while ptr < data.len() && data[ptr] != b'>' {
            match data[ptr] {
                b'k' => {
                    // Key type (ke, kx, etc.)
                    signer_pubkey = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse signer_pubkey: {}", e))?,
                    );
                }
                b'g' => {
                    // Signature type (ge, gp, etc.)
                    signature = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse signature: {}", e))?,
                    );
                }
                b'h' => {
                    // Hash type (hb, hp, etc.)
                    rolling_hash = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse rolling_hash: {}", e))?,
                    );
                }
                b'n' => {
                    // Field count - just parse and discard (we count fields from '(' markers)
                    let _ = parse(data, &mut ptr)
                        .map_err(|e| format!("Failed to parse field_count: {}", e))?;
                }
                b'(' => {
                    // Header field
                    let field = VsfField::parse(data, &mut ptr)
                        .map_err(|e| format!("Failed to parse header field: {}", e))?;

                    // Extract typed values from the parsed field
                    let mut hash = None;
                    let mut field_signature = None;
                    let mut key = None;
                    let mut offset_bytes = 0;
                    let mut size_bytes = 0;
                    let mut child_count = 0;
                    let mut has_offset = false;
                    let mut inline_values = Vec::new();

                    for value in &field.values {
                        match value {
                            // Hash types
                            VsfType::hp(_) | VsfType::hb(_) | VsfType::hs(_) => {
                                hash = Some(value.clone());
                            }
                            // Signature types
                            VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => {
                                field_signature = Some(value.clone());
                            }
                            // Key types
                            VsfType::ke(_) | VsfType::kx(_) | VsfType::kc(_) | VsfType::ka(_) => {
                                key = Some(value.clone());
                            }
                            // Offset - indicates this field points to a section body
                            VsfType::o(o) => {
                                offset_bytes = *o;
                                has_offset = true;
                            }
                            // Size in bytes
                            VsfType::b(b, _) => {
                                size_bytes = *b;
                            }
                            // Child count
                            VsfType::n(n) => {
                                child_count = *n;
                            }
                            // Capture inline values (u, i, d, l, x, f, etc.) for header-only fields
                            _ => {
                                inline_values.push(value.clone());
                            }
                        }
                    }

                    // If no offset, this is a metadata-only field (no section body)
                    if !has_offset {
                        offset_bytes = 0;
                        size_bytes = 0;
                        child_count = 0;
                    }

                    fields.push(HeaderField {
                        name: field.name,
                        hash,
                        signature: field_signature,
                        key,
                        offset_bytes,
                        size_bytes,
                        child_count,
                        inline_values,
                    });
                }
                _ => {
                    // Unknown type - just parse and continue (VSF is extensible)
                    let _ = parse(data, &mut ptr).map_err(|e| {
                        format!("Failed to parse unknown type at byte {}: {}", ptr, e)
                    })?;
                }
            }
        }

        // Skip past '>'
        if ptr < data.len() && data[ptr] == b'>' {
            ptr += 1;
        }

        Ok((
            VsfHeader {
                version,
                backward_compat,
                file_length,
                creation_time,
                provenance_hash,
                rolling_hash,
                signer_pubkey,
                signature,
                fields,
            },
            ptr, // Return number of bytes consumed
        ))
    }

    /// Update header length field after knowing final size
    pub fn update_header_length(header_bytes: &mut Vec<u8>) -> Result<(), String> {
        // Find the position after "RÅ<" (4 bytes: R=1, Å=2, <=1)
        if header_bytes.len() < 5 {
            return Err("Header too short".to_string());
        }

        // Structure is now: RÅ< z y b ... (version, backward_compat, then header length)
        // Skip past z (version) and y (backward_compat) to find b (header length)
        let mut ptr = 4; // After "RÅ<"

        // Skip version (z) field
        if ptr >= header_bytes.len() || header_bytes[ptr] != b'z' {
            return Err("Expected 'z' (version) marker after header start".to_string());
        }
        ptr += 1;
        while ptr < header_bytes.len() && header_bytes[ptr] != b'y' {
            ptr += 1;
        }

        // Skip backward_compat (y) field
        if ptr >= header_bytes.len() || header_bytes[ptr] != b'y' {
            return Err("Expected 'y' (backward compat) marker after version".to_string());
        }
        ptr += 1;
        while ptr < header_bytes.len() && header_bytes[ptr] != b'b' {
            ptr += 1;
        }

        // Now at b (header length) field
        if ptr >= header_bytes.len() || header_bytes[ptr] != b'b' {
            return Err("Expected 'b' (header length) marker after backward compat".to_string());
        }

        let _b_start = ptr;
        ptr += 1; // Skip 'b'

        // Find end of b field (next field marker)
        let value_start = ptr;
        while ptr < header_bytes.len() && header_bytes[ptr] != b'e' && header_bytes[ptr] != b'h' {
            ptr += 1;
        }
        let placeholder_len = ptr - value_start;

        // Calculate what the header length will be AFTER we replace the placeholder
        let mut header_length_bytes = header_bytes.len();
        let mut length_encoded = VsfType::b(header_length_bytes, true).flatten();

        // Iterate until stable (in case encoding size changes)
        loop {
            let new_total = header_bytes.len() - placeholder_len + (length_encoded.len() - 1); // -1 for 'b' marker
            if new_total == header_length_bytes {
                break; // Stable!
            }
            header_length_bytes = new_total;
            length_encoded = VsfType::b(header_length_bytes, true).flatten();
        }

        // Remove old b field value (keep the 'b' marker)
        header_bytes.drain(value_start..value_start + placeholder_len);

        // Insert new length encoding value (skip first 'b' since it's already there)
        for (i, byte) in length_encoded.iter().skip(1).enumerate() {
            header_bytes.insert(value_start + i, *byte);
        }

        // Also update L (file length) if present - for header-only files, L = header length
        Self::update_file_length(header_bytes)?;

        Ok(())
    }

    /// Update the file length (L) field in a header to match actual size
    /// Called automatically by update_header_length() for header-only files
    pub fn update_file_length(header_bytes: &mut Vec<u8>) -> Result<(), String> {
        // Find L field (after b, before e or h)
        let mut ptr = 4; // After "RÅ<"

        // Skip z (version)
        while ptr < header_bytes.len() && header_bytes[ptr] != b'y' {
            ptr += 1;
        }
        // Skip y (backward compat)
        while ptr < header_bytes.len() && header_bytes[ptr] != b'b' {
            ptr += 1;
        }
        // Skip b (header length)
        if ptr >= header_bytes.len() || header_bytes[ptr] != b'b' {
            return Ok(()); // No b field, nothing to do
        }
        ptr += 1;
        while ptr < header_bytes.len()
            && header_bytes[ptr] != b'l'
            && header_bytes[ptr] != b'e'
            && header_bytes[ptr] != b'h'
        {
            ptr += 1;
        }

        // Check if l field exists
        if ptr >= header_bytes.len() || header_bytes[ptr] != b'l' {
            return Ok(()); // No l field, nothing to do
        }

        ptr += 1; // Skip 'l'

        // Find end of l field value
        let value_start = ptr;
        while ptr < header_bytes.len() && header_bytes[ptr] != b'e' && header_bytes[ptr] != b'h' {
            ptr += 1;
        }
        let placeholder_len = ptr - value_start;

        // Calculate file length with stabilization loop
        let mut file_length = header_bytes.len();
        let mut length_encoded = VsfType::l(file_length, true).flatten();

        loop {
            let new_total = header_bytes.len() - placeholder_len + (length_encoded.len() - 1);
            if new_total == file_length {
                break;
            }
            file_length = new_total;
            length_encoded = VsfType::l(file_length, true).flatten();
        }

        // Remove old l field value (keep 'l' marker)
        header_bytes.drain(value_start..value_start + placeholder_len);

        // Insert new length encoding (skip 'l' marker)
        for (i, byte) in length_encoded.iter().skip(1).enumerate() {
            header_bytes.insert(value_start + i, *byte);
        }

        Ok(())
    }
}

/// Section of structured data (has children)
#[derive(Debug, Clone)]
pub struct VsfSection {
    pub name: String,
    pub fields: Vec<VsfField>,
    /// Optional length hint from `b{}` prefix (for forensics/validation)
    /// Only present when section was encoded at offset > 1MB
    pub length_hint: Option<usize>,
    /// Optional field count hint from `n{}` prefix (for forensics/validation)
    /// Only present when section was encoded at offset > 1MB
    pub count_hint: Option<usize>,
}

/// Single field in a section
#[derive(Debug, Clone)]
pub struct VsfField {
    pub name: String,
    pub values: Vec<VsfType>, // Empty vec = flag, 1 elem = single value, N elems = multi-value
}

impl VsfField {
    /// Create a new field with the given name and no values
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }

    /// Create a field with name and values
    pub fn with_values(name: impl Into<String>, values: Vec<VsfType>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    /// Add a value to the field (builder pattern)
    pub fn with_value(mut self, value: VsfType) -> Self {
        self.values.push(value);
        self
    }

    /// Add a value to the field (mutable)
    pub fn add_value(&mut self, value: VsfType) -> &mut Self {
        self.values.push(value);
        self
    }

    /// Flatten field to bytes with automatic separator handling
    ///
    /// Format: (name:value1,value2,value3)
    /// - '(' starts the field
    /// - name is encoded as VsfType::d
    /// - ':' separates name from values (only if values present)
    /// - ',' separates values from each other
    /// - ')' ends the field
    pub fn flatten(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(b'(');
        bytes.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

        if !self.values.is_empty() {
            bytes.push(b':');

            for (i, value) in self.values.iter().enumerate() {
                if i > 0 {
                    bytes.push(b',');
                }
                bytes.extend_from_slice(&value.flatten());
            }
        }

        bytes.push(b')');
        bytes
    }

    /// Parse a field from bytes
    ///
    /// Expects format: (name:value1,value2,value3)
    /// Updates ptr to point after the closing ')'
    pub fn parse(data: &[u8], ptr: &mut usize) -> Result<Self, String> {
        // Expect '('
        if *ptr >= data.len() || data[*ptr] != b'(' {
            return Err(format!(
                "Expected '(' at position {}, found {:?}",
                ptr,
                data.get(*ptr)
            ));
        }
        *ptr += 1;

        // Parse field name
        let name = match crate::parse(data, ptr)
            .map_err(|e| format!("VsfField: Failed to parse name: {}", e))?
        {
            VsfType::d(s) => s,
            other => {
                return Err(format!(
                    "VsfField: Expected field name (d type), found {:?}",
                    other
                ))
            }
        };

        let mut values = Vec::new();

        // Check for ':' separator (values present)
        if *ptr < data.len() && data[*ptr] == b':' {
            *ptr += 1;

            // Parse values until ')'
            loop {
                if *ptr >= data.len() {
                    return Err("Unexpected end of data in field".to_string());
                }

                if data[*ptr] == b')' {
                    break;
                }

                // Skip comma separator
                if data[*ptr] == b',' {
                    *ptr += 1;
                    if *ptr >= data.len() {
                        return Err("Unexpected end of data after comma".to_string());
                    }
                }

                // Parse value
                let value = crate::parse(data, ptr)
                    .map_err(|e| format!("VsfField: Failed to parse value: {}", e))?;
                values.push(value);
            }
        }

        // Expect ')'
        if *ptr >= data.len() || data[*ptr] != b')' {
            return Err(format!(
                "Expected ')' at position {}, found {:?}",
                ptr,
                data.get(*ptr)
            ));
        }
        *ptr += 1;

        Ok(Self { name, values })
    }
}

impl VsfSection {
    /// Create new section with validated name
    ///
    /// # Panics
    /// Panics if the section name contains invalid characters
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        validate_name(&name_str).unwrap_or_else(|e| panic!("Invalid section name: {}", e));
        Self {
            name: name_str,
            fields: Vec::new(),
            length_hint: None,
            count_hint: None,
        }
    }

    /// Add a field to the section with validated field name (single value)
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    pub fn add_field(&mut self, name: impl Into<String>, value: VsfType) {
        let name_str = name.into();
        validate_name(&name_str).unwrap_or_else(|e| panic!("Invalid field name: {}", e));
        self.fields.push(VsfField {
            name: name_str,
            values: vec![value],
        });
    }

    /// Add a flag/marker field with no values
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    pub fn add_flag(&mut self, name: impl Into<String>) {
        let name_str = name.into();
        validate_name(&name_str).unwrap_or_else(|e| panic!("Invalid field name: {}", e));
        self.fields.push(VsfField {
            name: name_str,
            values: vec![],
        });
    }

    /// Add a field with multiple values
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    pub fn add_field_multi(&mut self, name: impl Into<String>, values: Vec<VsfType>) {
        let name_str = name.into();
        validate_name(&name_str).unwrap_or_else(|e| panic!("Invalid field name: {}", e));
        self.fields.push(VsfField {
            name: name_str,
            values,
        });
    }

    /// Add a field to the section (builder pattern)
    ///
    /// Returns self for method chaining
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    ///
    /// # Example
    /// ```ignore
    /// let section = VsfSection::new("metadata")
    ///     .field("width", VsfType::u(1920, false))
    ///     .field("height", VsfType::u(1080, false));
    /// ```
    pub fn field(mut self, name: impl Into<String>, value: VsfType) -> Self {
        self.add_field(name, value);
        self
    }

    /// Add an optional field to the section (builder pattern)
    ///
    /// Only adds the field if the Option is Some. Returns self for method chaining.
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    ///
    /// # Example
    /// ```ignore
    /// let section = VsfSection::new("metadata")
    ///     .field("width", VsfType::u(1920, false))
    ///     .field_opt("description", description_opt);  // Only added if Some
    /// ```
    pub fn field_opt(mut self, name: impl Into<String>, value: Option<VsfType>) -> Self {
        if let Some(v) = value {
            self.add_field(name, v);
        }
        self
    }

    /// Add multiple fields from a vector (builder pattern)
    ///
    /// Returns self for method chaining
    ///
    /// # Panics
    /// Panics if any field name contains invalid characters
    ///
    /// # Example
    /// ```ignore
    /// let fields = vec![
    ///     ("width".to_string(), VsfType::u(1920, false)),
    ///     ("height".to_string(), VsfType::u(1080, false)),
    /// ];
    /// let section = VsfSection::new("metadata").fields(fields);
    /// ```
    pub fn fields(mut self, fields: Vec<(String, VsfType)>) -> Self {
        for (name, value) in fields {
            self.add_field(name, value);
        }
        self
    }

    /// Encode section to bytes (no preamble - crypto moved to header labels)
    ///
    /// Format: [dsection_name(field:value)...]
    /// - Empty values: (dfield)
    /// - Single value: (dfield:value)
    /// - Multi-value: (dfield:v1,v2,v3)
    pub fn encode(&self) -> Vec<u8> {
        self.encode_at_offset(0)
    }

    /// Encode section for encryption (always includes n{count}b{length})
    ///
    /// Use this when encoding a section that will be encrypted and sent without a VSF header. The `n` and `b` fields allow validation after decryption.
    ///
    /// Format: [d{name}n{count}b{length}(field:value)...]
    /// Empty sections produce: [d{name}n{0}b{X}]
    pub fn encode_encrypted(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(b'[');

        // Section name
        let name_bytes = VsfType::d(self.name.clone()).flatten();
        bytes.extend_from_slice(&name_bytes);

        // Encode fields to know their size
        let mut fields_bytes = Vec::new();
        for field in &self.fields {
            fields_bytes.extend_from_slice(&field.flatten());
        }

        // n{count}
        let n_encoded = VsfType::n(self.fields.len()).flatten();

        // b{length} with inclusive encoding (includes its own size)
        let base_without_b = 1 + name_bytes.len() + n_encoded.len() + fields_bytes.len() + 1;
        let mut b_encoded = VsfType::b(base_without_b, true).flatten();

        loop {
            let total = base_without_b + b_encoded.len();
            let new_b = VsfType::b(total, true).flatten();
            if new_b.len() == b_encoded.len() {
                b_encoded = new_b;
                break;
            }
            b_encoded = new_b;
        }

        bytes.extend_from_slice(&n_encoded);
        bytes.extend_from_slice(&b_encoded);
        bytes.extend_from_slice(&fields_bytes);
        bytes.push(b']');
        bytes
    }

    /// Encode section with optional `n{count}b{length}` suffix for large files.
    ///
    /// When `file_offset > 1MB`, appends metadata AFTER the section name for fast seeking and forensic validation. Small files/packets get no suffix (zero overhead).
    ///
    /// # Format
    ///
    /// ```text
    /// Small files:  [d"section_name"(d"field":val)...]
    /// Large files:  [d"section_name"n{5}b{203}(d"field":val)...]
    ///                               ↑    ↑
    ///                               |    total section length (inclusive)
    ///                               field count
    /// ```
    ///
    /// # Inclusive Length Encoding
    ///
    /// The `b{}` value uses **inclusive encoding** - it includes its own size in the total. This avoids the "255 + overhead = 256 BOINK" problem where adding the length field's size pushes you into a larger encoding:
    ///
    /// ```text
    /// Content = 252 bytes, need to add b{} (3 bytes for values < 256)
    /// Naive:  252 + 3 = 255 → fits in u8 → b3{255} ✓
    /// But:    253 + 3 = 256 → needs u16 → b4{} is 4 bytes!
    ///         253 + 4 = 257 → still fits u16 ✓
    ///
    /// Inclusive encoding handles this automatically by including its own
    /// overhead in the encoded value, converging to the correct size.
    /// ```
    ///
    /// # Why name-first?
    ///
    /// Putting metadata after the name means you know WHAT you're looking at before you see HOW BIG it is. Reading `[d"attachments"n{12}b{1847}` tells you "this is attachments, 12 fields, 1847 bytes" in natural order.
    pub fn encode_at_offset(&self, file_offset: usize) -> Vec<u8> {
        // Empty sections have no body - the header already declares them
        if self.fields.is_empty() {
            return Vec::new();
        }

        let mut bytes = Vec::new();
        bytes.push(b'[');

        // Add n{count}b{length} suffix + section name for sections >1MB from file start
        // For sections <=1MB, section name is ONLY in the header TOC, not in the body
        const ONE_MB: usize = 1_048_576;
        if file_offset > ONE_MB {
            // Section name + metadata only for distant sections (>1MB from header)
            bytes.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

            let field_count = self.fields.len();

            // Encode fields to know their size
            let mut fields_bytes = Vec::new();
            for field in &self.fields {
                fields_bytes.extend_from_slice(&field.flatten());
            }

            // Use inclusive encoding for b{} - iterate until stable (adding b{} size might push us into a larger encoding class)
            // Total = '[' + name + n{} + b{} + fields + ']'
            let name_bytes = VsfType::d(self.name.clone()).flatten();
            let n_encoded = VsfType::n(field_count).flatten();

            // Start with base size, iterate until b{} encoding is stable
            let base_without_b = 1 + name_bytes.len() + n_encoded.len() + fields_bytes.len() + 1;
            let mut b_encoded = VsfType::b(base_without_b, true).flatten();

            loop {
                let total = base_without_b + b_encoded.len();
                let new_b = VsfType::b(total, true).flatten();
                if new_b.len() == b_encoded.len() {
                    b_encoded = new_b;
                    break;
                }
                b_encoded = new_b;
            }

            bytes.extend_from_slice(&n_encoded);
            bytes.extend_from_slice(&b_encoded);
            bytes.extend_from_slice(&fields_bytes);
        } else {
            // No metadata suffix for small files
            for field in &self.fields {
                bytes.extend_from_slice(&field.flatten());
            }
        }

        bytes.push(b']');
        bytes
    }

    /// Parse a section from bytes (low-level, schema-agnostic)
    ///
    /// Expects format: `[d"section_name"(d"field":value)...]`
    /// Updates ptr to point after the closing `]`
    ///
    /// This is the **low-level** parsing API that extracts raw data without validation. Each field is stored as a single `VsfField` with one value.
    ///
    /// For **schema-validated** parsing with type constraints and multi-value field support, use [`crate::schema::SectionBuilder::parse()`] instead. That API validates against a schema and returns a builder for the parse→modify→encode workflow.
    ///
    /// # Use Cases
    /// - Reading unknown/arbitrary VSF data
    /// - Debugging or inspecting files
    /// - Building tooling that handles any section type
    /// - When you don't have or need a schema
    pub fn parse(data: &[u8], ptr: &mut usize) -> Result<Self, String> {
        // Expect '['
        if *ptr >= data.len() || data[*ptr] != b'[' {
            return Err(format!(
                "Expected '[' at position {}, found {:?}",
                ptr,
                data.get(*ptr)
            ));
        }
        let section_start = *ptr;
        *ptr += 1;

        // For sections <1MB, no name is present - fields start immediately with '('
        // For sections >1MB, name is REQUIRED with n{count}b{length}: [d"name"n{count}b{length}(fields...)]
        let (name, length_hint, count_hint) = if *ptr < data.len() && data[*ptr] == b'(' {
            // No name - section within 1MB of header
            (String::from(""), None, None)
        } else {
            // Parse section name
            let section_name = match crate::parse(data, ptr)
                .map_err(|e| format!("VsfSection: Failed to parse name: {}", e))?
            {
                VsfType::d(s) => s,
                other => {
                    return Err(format!(
                        "VsfSection: Expected section name (d type), found {:?}",
                        other
                    ))
                }
            };

            // When section name is present, n{count} and b{length} are REQUIRED
            // Parse n{count}
            let count = match crate::parse(data, ptr) {
                Ok(VsfType::n(c)) => c,
                Ok(other) => {
                    return Err(format!(
                        "VsfSection: Expected n{{count}} after section name, found {:?}",
                        other
                    ))
                }
                Err(e) => return Err(format!("VsfSection: Failed to parse n{{count}}: {}", e)),
            };

            // Parse b{length}
            let length = match crate::parse(data, ptr) {
                Ok(VsfType::b(len, _)) => len,
                Ok(other) => {
                    return Err(format!(
                        "VsfSection: Expected b{{length}} after n{{count}}, found {:?}",
                        other
                    ))
                }
                Err(e) => return Err(format!("VsfSection: Failed to parse b{{length}}: {}", e)),
            };

            (section_name, Some(length), Some(count))
        };

        let mut fields = Vec::new();

        // Parse fields until ']'
        while *ptr < data.len() && data[*ptr] != b']' {
            if data[*ptr] == b'(' {
                let field = VsfField::parse(data, ptr)?;
                fields.push(field);
            } else {
                // Skip whitespace or unexpected bytes
                *ptr += 1;
            }
        }

        // Expect ']'
        if *ptr >= data.len() || data[*ptr] != b']' {
            return Err(format!(
                "Expected ']' at position {}, found {:?}",
                ptr,
                data.get(*ptr)
            ));
        }
        *ptr += 1;

        // Validate length hint if present (section_start to current position)
        let actual_length = *ptr - section_start;
        if let Some(expected) = length_hint {
            if actual_length != expected {
                // Don't fail - just log for forensics. The hints are informational. In debug builds this could warn, but we want to parse corrupted files.
            }
        }

        // Validate count hint if present
        if let Some(expected) = count_hint {
            if fields.len() != expected {
                // Same - informational, don't fail parse
            }
        }

        Ok(Self {
            name,
            fields,
            length_hint,
            count_hint,
        })
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&VsfField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get all fields with a given name (for repeated fields like "peer")
    pub fn get_fields(&self, name: &str) -> Vec<&VsfField> {
        self.fields.iter().filter(|f| f.name == name).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encoding() {
        let mut header = VsfHeader::new(1, 1);
        header.add_field(HeaderField {
            name: "test section".to_string(),
            hash: None,
            signature: None,
            key: None,
            offset_bytes: 512,
            size_bytes: 256,
            child_count: 3,
            inline_values: Vec::new(),
        });

        let encoded = header.encode().unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8: 0x52, 0xC3, 0x85)
        assert_eq!(&encoded[0..3], "RÅ".as_bytes());
        assert_eq!(encoded[3], b'<');

        // Should contain header markers
        assert!(encoded.contains(&b'z')); // Version
        assert!(encoded.contains(&b'y')); // Backward compat
        assert!(encoded.contains(&b'n')); // Count (was 'c', now 'n')
        assert!(encoded.contains(&b'>')); // Header end
    }

    #[test]
    fn test_section_encoding() {
        let mut section = VsfSection::new("test");
        section.add_field("width", VsfType::u(4096, false));
        section.add_field("height", VsfType::u(3072, false));

        let encoded = section.encode();

        // Verify no preamble (starts with '[')
        assert_eq!(encoded[0], b'[');
        assert_eq!(encoded[encoded.len() - 1], b']');

        // Verify parentheses for fields
        assert!(encoded.contains(&b'('));
        assert!(encoded.contains(&b')'));
        assert!(encoded.contains(&b':')); // Separator
    }

    #[test]
    fn test_field_syntax_variations() {
        let mut section = VsfSection::new("test");

        // Test flag/marker (no values, no colon)
        section.add_flag("enabled");

        // Test single value (colon + value)
        section.add_field("width", VsfType::u(1920, false));

        // Test multi-value (colon + comma-separated values)
        section.add_field_multi(
            "resolution",
            vec![VsfType::u(1920, false), VsfType::u(1080, false)],
        );

        let encoded = section.encode();
        let encoded_str = String::from_utf8_lossy(&encoded);

        // Flag should have no colon: (d"enabled")
        assert!(encoded_str.contains("enabled"));
        // Find the enabled field and verify no colon after it
        let enabled_pos = encoded_str.find("enabled").unwrap();
        let after_enabled = &encoded_str[enabled_pos + 7..enabled_pos + 8];
        assert_eq!(after_enabled, ")"); // Should close immediately, no colon

        // Single value should have colon
        assert!(encoded_str.contains("width"));

        // Multi-value should have comma
        assert!(encoded.contains(&b','));
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("camera").is_ok());
        assert!(validate_name("iso_speed").is_ok());
        assert!(validate_name("camera.sensor").is_ok());
        assert!(validate_name("lens_min_focal_m").is_ok());
        assert!(validate_name("shutter_time_s").is_ok());
        assert!(validate_name("test123").is_ok());
        assert!(validate_name("camera2").is_ok());
        assert!(validate_name("camera.sensor.temperature").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("a1").is_ok());
        assert!(validate_name("a_b_c").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        // Empty
        assert!(validate_name("").is_err());

        // Uppercase
        assert!(validate_name("Camera").is_err());
        assert!(validate_name("cameraA").is_err());

        // Invalid characters
        assert!(validate_name("iso speed").is_err()); // space
        assert!(validate_name("iso-speed").is_err()); // hyphen
        assert!(validate_name("camera(main)").is_err()); // paren
        assert!(validate_name("camera:sensor").is_err()); // colon
        assert!(validate_name("lens/model").is_err()); // slash

        // Invalid start
        assert!(validate_name("9camera").is_err()); // starts with digit
        assert!(validate_name("_camera").is_err()); // starts with underscore
        assert!(validate_name(".camera").is_err()); // starts with dot
        assert!(validate_name("1test").is_err()); // starts with digit

        // Invalid end
        assert!(validate_name("camera_").is_err()); // ends with underscore
        assert!(validate_name("camera.").is_err()); // ends with dot

        // Consecutive separators
        assert!(validate_name("camera..sensor").is_err()); // double dot
        assert!(validate_name("camera__sensor").is_err()); // double underscore

        // Invalid segment start in hierarchical names
        assert!(validate_name("camera.9sensor").is_err()); // segment starts with digit
        assert!(validate_name("camera._private").is_err()); // segment starts with underscore
    }

    #[test]
    #[should_panic(expected = "Invalid section name")]
    fn test_section_name_validation_panics() {
        VsfSection::new("Camera Sensor"); // uppercase and space
    }

    #[test]
    #[should_panic(expected = "Invalid field name")]
    fn test_field_name_validation_panics() {
        let mut section = VsfSection::new("camera");
        section.add_field("ISO Speed", VsfType::f5(800.0)); // uppercase and space
    }

    #[test]
    fn test_header_decode_basic() {
        // Create a simple header
        let mut header = VsfHeader::new(1, 1);
        header.add_field(HeaderField {
            name: "test_section".to_string(),
            hash: None,
            signature: None,
            key: None,
            offset_bytes: 256,
            size_bytes: 128,
            child_count: 2,
            inline_values: Vec::new(),
        });

        // Encode it
        let encoded = header.encode().unwrap();

        // Decode it
        let (decoded, bytes_consumed) = VsfHeader::decode(&encoded).unwrap();

        // Verify decoded matches original
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.backward_compat, 1);
        assert_eq!(decoded.fields.len(), 1);
        assert_eq!(decoded.fields[0].name, "test_section");
        assert_eq!(decoded.fields[0].offset_bytes, 256);
        assert_eq!(decoded.fields[0].size_bytes, 128);
        assert_eq!(decoded.fields[0].child_count, 2);
        assert_eq!(bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_decode_with_crypto() {
        // Create a header with crypto fields
        let mut header = VsfHeader::new(1, 1);
        header.add_field(HeaderField {
            name: "encrypted_section".to_string(),
            hash: Some(VsfType::hb(vec![0u8; 32])), // BLAKE3 rolling hash
            signature: Some(VsfType::ge(vec![0u8; 64])), // Ed25519 signature
            key: Some(VsfType::kx(vec![0u8; 32])),  // X25519 key
            offset_bytes: 512,
            size_bytes: 1024,
            child_count: 0,
            inline_values: Vec::new(),
        });

        // Encode it
        let encoded = header.encode().unwrap();

        // Decode it
        let (decoded, bytes_consumed) = VsfHeader::decode(&encoded).unwrap();

        // Verify decoded matches original
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.fields.len(), 1);
        assert_eq!(decoded.fields[0].name, "encrypted_section");
        assert!(decoded.fields[0].hash.is_some());
        assert!(decoded.fields[0].signature.is_some());
        assert!(decoded.fields[0].key.is_some());
        assert_eq!(decoded.fields[0].child_count, 0);
        assert_eq!(bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_decode_invalid_magic() {
        let invalid = b"WRONG<";
        let result = VsfHeader::decode(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid VSF magic number"));
    }

    #[test]
    fn test_header_decode_too_short() {
        let invalid = b"RA";
        let result = VsfHeader::decode(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Data too short"));
    }

    #[test]
    fn test_section_parse_roundtrip() {
        let mut section = VsfSection::new("test_section");
        section.add_field("width", VsfType::u(1920, false));
        section.add_field("height", VsfType::u(1080, false));
        section.add_field("key", VsfType::ke(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        // Encode to bytes
        let encoded = section.encode();

        // Parse back
        let mut ptr = 0;
        let parsed = VsfSection::parse(&encoded, &mut ptr).unwrap();

        // Verify roundtrip
        assert_eq!(parsed.name, "test_section");
        assert_eq!(parsed.fields.len(), 3);
        assert_eq!(parsed.fields[0].name, "width");
        assert_eq!(parsed.fields[1].name, "height");
        assert_eq!(parsed.fields[2].name, "key");

        // Test get_field helper
        let width = parsed.get_field("width").unwrap();
        assert_eq!(width.values.len(), 1);

        // Test get_fields for multiple
        let all_fields = parsed.get_fields("width");
        assert_eq!(all_fields.len(), 1);
    }

    #[test]
    fn test_section_encode_no_suffix_small_offset() {
        // Sections at small offsets should NOT have n{}b{} suffix
        let mut section = VsfSection::new("test");
        section.add_field("value", VsfType::u(42, false));

        let encoded = section.encode_at_offset(0);

        // Should start with '[d' (name first, no metadata)
        assert_eq!(encoded[0], b'[');
        assert_eq!(encoded[1], b'd'); // Section name starts immediately
    }

    #[test]
    fn test_section_encode_with_suffix_large_offset() {
        // Sections at >1MB offset SHOULD have n{}b{} suffix after name
        let mut section = VsfSection::new("test");
        section.add_field("value", VsfType::u(42, false));

        let offset_2mb = 2 * 1024 * 1024;
        let encoded = section.encode_at_offset(offset_2mb);

        // Should start with '[d' (name first)
        assert_eq!(encoded[0], b'[');
        assert_eq!(encoded[1], b'd'); // Name comes first

        // Parse it back and verify hints are captured
        let mut ptr = 0;
        let parsed = VsfSection::parse(&encoded, &mut ptr).unwrap();

        assert_eq!(parsed.name, "test");
        assert!(parsed.length_hint.is_some());
        assert!(parsed.count_hint.is_some());
        assert_eq!(parsed.length_hint.unwrap(), encoded.len());
        assert_eq!(parsed.count_hint.unwrap(), 1); // One field
    }

    #[test]
    fn test_section_parse_without_suffix() {
        // Parse section without suffix (old format / small files)
        let mut section = VsfSection::new("legacy");
        section.add_field("data", VsfType::u(123, false));

        let encoded = section.encode(); // No offset = no suffix

        let mut ptr = 0;
        let parsed = VsfSection::parse(&encoded, &mut ptr).unwrap();

        assert_eq!(parsed.name, "legacy");
        assert!(parsed.length_hint.is_none());
        assert!(parsed.count_hint.is_none());
    }

    #[test]
    fn test_section_suffix_length_accuracy() {
        // Verify that the length in b{} matches actual section length (inclusive)
        let mut section = VsfSection::new("data");
        section.add_field("field1", VsfType::u(100, false));
        section.add_field("field2", VsfType::u(200, false));
        section.add_field("field3", VsfType::a("hello".to_string()));

        let offset_5mb = 5 * 1024 * 1024;
        let encoded = section.encode_at_offset(offset_5mb);

        let mut ptr = 0;
        let parsed = VsfSection::parse(&encoded, &mut ptr).unwrap();

        // Length hint should match actual encoded length (inclusive encoding)
        assert_eq!(parsed.length_hint.unwrap(), encoded.len());
        // Count hint should match field count
        assert_eq!(parsed.count_hint.unwrap(), 3);
    }
}
