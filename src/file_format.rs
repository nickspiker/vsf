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
    pub rolling_hash: Option<VsfType>, // Optional: BLAKE3 hash of current state (hb) - OR signature
    pub signer_pubkey: Option<VsfType>, // Optional: Ed25519 public key (ke) - for signed files
    pub signature: Option<VsfType>, // Optional: Ed25519 signature (ge) - replaces rolling_hash
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

        // Creation time (always present)
        header.extend_from_slice(&self.creation_time.flatten());

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

        // Check for optional signer_pubkey (ke), signature (ge), or rolling hash (hb)
        // These are mutually exclusive: either (ke + ge) OR (hb), but not both
        let mut rolling_hash = None;
        let mut signer_pubkey = None;
        let mut signature = None;

        // Parse optional crypto fields until we hit 'n' (field count)
        while ptr < data.len() && data[ptr] != b'n' {
            match data[ptr] {
                b'k' => {
                    // Signer pubkey (ke)
                    signer_pubkey = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse signer_pubkey: {}", e))?,
                    );
                }
                b'g' => {
                    // Signature (ge)
                    signature = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse signature: {}", e))?,
                    );
                }
                b'h' => {
                    // Rolling hash (hb) - only if no signature
                    rolling_hash = Some(
                        parse(data, &mut ptr)
                            .map_err(|e| format!("Failed to parse rolling_hash: {}", e))?,
                    );
                }
                _ => break, // Unknown field, stop parsing crypto
            }
        }

        let field_count_type = parse(data, &mut ptr)
            .map_err(|e| format!("Failed to parse field_count: {}", e))?;

        let field_count = match field_count_type {
            VsfType::n(count) => count,
            _ => {
                return Err(format!(
                    "Expected field_count (n), got {:?}",
                    field_count_type
                ))
            }
        };

        // Parse header fields using VsfField::parse() - same format as section fields
        // Format: (dname:value,value,value) or (dname) for empty
        let mut fields = Vec::with_capacity(field_count);
        for i in 0..field_count {
            let field = VsfField::parse(data, &mut ptr)
                .map_err(|e| format!("Failed to parse header field {}: {}", i, e))?;

            // Extract typed values from the parsed field
            let mut hash = None;
            let mut signature = None;
            let mut key = None;
            let mut wrap = None;
            let mut offset_bytes = 0;
            let mut size_bytes = 0;
            let mut child_count = 0;
            let mut has_offset = false;

            for value in &field.values {
                match value {
                    // Hash types
                    VsfType::hp(_) | VsfType::hb(_) | VsfType::hs(_) => {
                        hash = Some(value.clone());
                    }
                    // Signature types
                    VsfType::ge(_) | VsfType::gp(_) | VsfType::gr(_) => {
                        signature = Some(value.clone());
                    }
                    // Key types
                    VsfType::ke(_) | VsfType::kx(_) | VsfType::kc(_) | VsfType::ka(_) => {
                        key = Some(value.clone());
                    }
                    // Wrapped/encrypted marker
                    VsfType::v(_, _) => {
                        wrap = Some(value.clone());
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
                    _ => {} // Ignore other types
                }
            }

            // If no offset, this is a metadata-only field (no section body)
            // offset_bytes stays 0, size_bytes stays 0, child_count stays 0
            if !has_offset {
                offset_bytes = 0;
                size_bytes = 0;
                child_count = 0;
            }

            fields.push(HeaderField {
                name: field.name,
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
        let name = match crate::parse(data, ptr).map_err(|e| e.to_string())? {
            VsfType::d(s) => s,
            other => return Err(format!("Expected field name (d type), found {:?}", other)),
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
                let value = crate::parse(data, ptr).map_err(|e| e.to_string())?;
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
        // Empty sections have no body - the header already declares them
        // This saves bytes for things like ping/pong where the section name
        // in the header IS the message type identifier
        if self.fields.is_empty() {
            return Vec::new();
        }

        let mut bytes = Vec::new();

        // Section start
        bytes.push(b'[');

        // Section name (namespace for all fields)
        bytes.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

        // Encode each field using VsfField::flatten()
        for field in &self.fields {
            bytes.extend_from_slice(&field.flatten());
        }

        // Section end
        bytes.push(b']');

        bytes
    }

    /// Parse a section from bytes
    ///
    /// Expects format: [dsection_name(field:value)...]
    /// Updates ptr to point after the closing ']'
    pub fn parse(data: &[u8], ptr: &mut usize) -> Result<Self, String> {
        // Expect '['
        if *ptr >= data.len() || data[*ptr] != b'[' {
            return Err(format!(
                "Expected '[' at position {}, found {:?}",
                ptr,
                data.get(*ptr)
            ));
        }
        *ptr += 1;

        // Parse section name
        let name = match crate::parse(data, ptr).map_err(|e| e.to_string())? {
            VsfType::d(s) => s,
            other => return Err(format!("Expected section name (d type), found {:?}", other)),
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

        Ok(Self { name, fields })
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
}
