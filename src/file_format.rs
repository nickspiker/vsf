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
    pub creation_time: VsfType, // Creation timestamp (ef5 for ~2min precision)
    pub provenance_hash: VsfType, // Required: BLAKE3 hash of immutable content (hp)
    pub rolling_hash: Option<VsfType>, // Optional: BLAKE3 hash of current state (hb)
    pub fields: Vec<HeaderField>,
}

/// Header field definition (section pointer with positional values)
/// Format: (d[section_name] o[offset] b[size] n[count])
/// Note: Header fields use POSITIONAL values (no colons or commas)
#[derive(Debug, Clone)]
pub struct HeaderField {
    pub name: String,
    pub hash: Option<VsfType>, // h: optional hash of section data (VsfType::h)
    pub signature: Option<VsfType>, // g: optional signature of section data (VsfType::g)
    pub key: Option<VsfType>,  // k: optional cryptographic key (VsfType::k)
    pub wrap: Option<VsfType>, // v: optional wrapped/encrypted marker (VsfType::v)
    pub offset_bytes: usize,   // Offset in bytes (byte-aligned)
    pub size_bytes: usize,     // Size in bytes (byte-aligned)
    pub child_count: usize,    // 0 = unboxed blob, N = N structured children
}

impl VsfHeader {
    /// Create new header with current timestamp
    pub fn new(version: usize, backward_compat: usize) -> Self {
        use chrono::Utc;

        // Get current time and convert to Eagle Time with full precision
        let now = Utc::now();
        let et = crate::datetime_to_eagle_time(now);

        // Preserve the original precision from datetime_to_eagle_time
        // This uses f6 (f64) for subsecond precision
        let creation_time = VsfType::e(et.et_type().clone());

        Self {
            version,
            backward_compat,
            creation_time,
            provenance_hash: VsfType::hp(vec![0u8; 32]), // Placeholder, filled during build
            rolling_hash: None,
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

        // Creation time (always present)
        header.extend_from_slice(&self.creation_time.flatten());

        // Provenance hash (always present)
        header.extend_from_slice(&self.provenance_hash.flatten());

        // Rolling hash (optional)
        if let Some(ref hash) = self.rolling_hash {
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

            // Optional wrap (VsfType::v with algorithm)
            if let Some(ref wrap_type) = field.wrap {
                header.extend_from_slice(&wrap_type.flatten());
                header.push(b',');
            }

            // Offset (in bytes)
            header.extend_from_slice(&VsfType::o(field.offset_bytes).flatten());
            header.push(b',');

            // Size (in bytes)
            header.extend_from_slice(&VsfType::b(field.size_bytes, false).flatten());

            // Child count - omit if encrypted (implied to be n[0])
            if field.wrap.is_none() {
                header.push(b',');
                header.extend_from_slice(&VsfType::n(field.child_count).flatten());
            }

            header.push(b')');
        }

        // Header end marker
        header.push(b'>');

        Ok(header)
    }

    /// Decode a VSF header from bytes
    ///
    /// Parses the binary header structure and returns a VsfHeader instance.
    /// Returns the parsed header and the number of bytes consumed.
    ///
    /// # Format
    /// ```text
    /// RÅ<                          Magic + header start
    ///   z[version]                 Version number
    ///   y[backward_compat]         Backward compatibility version
    ///   b[header_length_bytes]     Header length in BYTES
    ///   e[creation_time]           Creation timestamp (ef5/ef6)
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

        // Parse creation time (e)
        let creation_time =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse creation_time: {}", e))?;
        if !matches!(creation_time, VsfType::e(_)) {
            return Err(format!(
                "Expected creation_time (e), got {:?}",
                creation_time
            ));
        }

        // Parse provenance hash (hp)
        let provenance_hash =
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse provenance_hash: {}", e))?;
        if !matches!(provenance_hash, VsfType::hp(_)) {
            return Err(format!(
                "Expected provenance_hash (hp), got {:?}",
                provenance_hash
            ));
        }

        // Check for optional rolling hash (hb) or field count (n)
        let mut rolling_hash = None;
        let field_count_type = if ptr < data.len() && data[ptr] == b'h' {
            // Optional rolling hash present
            rolling_hash = Some(
                parse(data, &mut ptr)
                    .map_err(|e| format!("Failed to parse rolling_hash: {}", e))?,
            );

            // Now parse field count
            parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse field_count after rolling_hash: {}", e))?
        } else {
            // No rolling hash, parse field count directly
            parse(data, &mut ptr).map_err(|e| format!("Failed to parse field_count: {}", e))?
        };

        let field_count = match field_count_type {
            VsfType::n(count) => count,
            _ => {
                return Err(format!(
                    "Expected field_count (n), got {:?}",
                    field_count_type
                ))
            }
        };

        // Parse header fields (section pointers)
        let mut fields = Vec::with_capacity(field_count);
        for i in 0..field_count {
            // Expect opening '('
            if ptr >= data.len() || data[ptr] != b'(' {
                return Err(format!(
                    "Expected '(' for header field {}, found {:?}",
                    i,
                    data.get(ptr)
                ));
            }
            ptr += 1;

            // Parse section name (d)
            let name_type = parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse field {} name: {}", i, e))?;
            let name = match name_type {
                VsfType::d(n) => n,
                _ => {
                    return Err(format!(
                        "Expected section name (d) for field {}, got {:?}",
                        i, name_type
                    ))
                }
            };

            // Expect ':' separator after section name
            if ptr >= data.len() || data[ptr] != b':' {
                return Err(format!(
                    "Expected ':' after section name for field {}, found {:?}",
                    i,
                    data.get(ptr)
                ));
            }
            ptr += 1;

            // Parse optional hash, signature, key, wrap
            let mut hash = None;
            let mut signature = None;
            let mut key = None;
            let mut wrap = None;

            // Parse optional crypto fields until we hit 'o' (offset)
            while ptr < data.len() && data[ptr] != b'o' {
                match data[ptr] {
                    b'h' => {
                        hash =
                            Some(parse(data, &mut ptr).map_err(|e| {
                                format!("Failed to parse hash for field {}: {}", i, e)
                            })?);
                        // Skip ',' separator
                        if ptr < data.len() && data[ptr] == b',' {
                            ptr += 1;
                        }
                    }
                    b'g' => {
                        signature = Some(parse(data, &mut ptr).map_err(|e| {
                            format!("Failed to parse signature for field {}: {}", i, e)
                        })?);
                        // Skip ',' separator
                        if ptr < data.len() && data[ptr] == b',' {
                            ptr += 1;
                        }
                    }
                    b'k' => {
                        key =
                            Some(parse(data, &mut ptr).map_err(|e| {
                                format!("Failed to parse key for field {}: {}", i, e)
                            })?);
                        // Skip ',' separator
                        if ptr < data.len() && data[ptr] == b',' {
                            ptr += 1;
                        }
                    }
                    b'v' => {
                        wrap =
                            Some(parse(data, &mut ptr).map_err(|e| {
                                format!("Failed to parse wrap for field {}: {}", i, e)
                            })?);
                        // Skip ',' separator
                        if ptr < data.len() && data[ptr] == b',' {
                            ptr += 1;
                        }
                    }
                    b')' => break, // End of field
                    _ => {
                        return Err(format!(
                            "Unexpected byte '{}' in header field {}",
                            data[ptr] as char, i
                        ))
                    }
                }
            }

            // Parse offset (o)
            let offset_type = parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse offset for field {}: {}", i, e))?;
            let offset_bytes = match offset_type {
                VsfType::o(offset) => offset,
                _ => {
                    return Err(format!(
                        "Expected offset (o) for field {}, got {:?}",
                        i, offset_type
                    ))
                }
            };

            // Skip ',' separator after offset
            if ptr < data.len() && data[ptr] == b',' {
                ptr += 1;
            }

            // Parse size (b)
            let size_type = parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse size for field {}: {}", i, e))?;
            let size_bytes = match size_type {
                VsfType::b(size, _) => size,
                _ => {
                    return Err(format!(
                        "Expected size (b) for field {}, got {:?}",
                        i, size_type
                    ))
                }
            };

            // Parse child count (n) - optional if encrypted (wrap present)
            let child_count = if wrap.is_some() {
                0 // Encrypted sections have implicit n[0]
            } else {
                // Skip ',' separator before count
                if ptr < data.len() && data[ptr] == b',' {
                    ptr += 1;
                }
                let count_type = parse(data, &mut ptr)
                    .map_err(|e| format!("Failed to parse count for field {}: {}", i, e))?;
                match count_type {
                    VsfType::n(count) => count,
                    _ => {
                        return Err(format!(
                            "Expected count (n) for field {}, got {:?}",
                            i, count_type
                        ))
                    }
                }
            };

            // Expect closing ')'
            if ptr >= data.len() || data[ptr] != b')' {
                return Err(format!(
                    "Expected ')' for header field {}, found {:?}",
                    i,
                    data.get(ptr)
                ));
            }
            ptr += 1;

            fields.push(HeaderField {
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

        // Expect closing '>'
        if ptr >= data.len() || data[ptr] != b'>' {
            return Err(format!(
                "Expected '>' to close header, found {:?}",
                data.get(ptr)
            ));
        }
        ptr += 1;

        Ok((
            VsfHeader {
                version,
                backward_compat,
                creation_time,
                provenance_hash,
                rolling_hash,
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

        Ok(())
    }
}

/// Section of structured data (has children)
#[derive(Debug, Clone)]
pub struct VsfSection {
    pub name: String,
    pub fields: Vec<VsfField>,
}

/// Single field in a section
#[derive(Debug, Clone)]
pub struct VsfField {
    pub name: String,
    pub values: Vec<VsfType>, // Empty vec = flag, 1 elem = single value, N elems = multi-value
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
        let mut bytes = Vec::new();

        // Section start
        bytes.push(b'[');

        // Section name (namespace for all fields)
        bytes.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

        // Encode each field
        for field in &self.fields {
            bytes.push(b'(');

            // Field name (simple identifier, no dots - namespace comes from section)
            bytes.extend_from_slice(&VsfType::d(field.name.clone()).flatten());

            // Handle values based on count
            if !field.values.is_empty() {
                // Add separator only if there are values
                bytes.push(b':');

                // Encode values with comma separators
                for (i, value) in field.values.iter().enumerate() {
                    if i > 0 {
                        bytes.push(b',');
                    }
                    bytes.extend_from_slice(&value.flatten());
                }
            }

            bytes.push(b')');
        }

        // Section end
        bytes.push(b']');

        bytes
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
            wrap: None,
            offset_bytes: 512,
            size_bytes: 256,
            child_count: 3,
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
            wrap: None,
            offset_bytes: 256,
            size_bytes: 128,
            child_count: 2,
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
            wrap: Some(VsfType::v(0, vec![0u8; 0])), // Encrypted marker
            offset_bytes: 512,
            size_bytes: 1024,
            child_count: 0, // Encrypted sections have n[0]
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
        assert!(decoded.fields[0].wrap.is_some());
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
}
