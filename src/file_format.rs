//! VSF file format with headers and hierarchical labels
//!
//! Binary structure (following basecalc pattern):
//! ```text
//! RÅ<                                    Magic + header start
//!   b[header_length_bytes]               Header length in BYTES
//!   z[version]                           Version number
//!   y[backward_compat]                   Backward compatibility version
//!   hb[256][hash]                        File integrity hash (BLAKE3)
//!   n[label_count]                       Number of label definitions
//!
//!   (d[label_name] h?[hash] g?[sig] k?[key] o[offset] b[size] n[count])  Label definition
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

use crate::encoding::traits::EncodeNumber;
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
    pub file_hash: Option<VsfType>, // Optional file-level hash (typically BLAKE3)
    pub labels: Vec<LabelDefinition>,
}

/// Label definition in header
#[derive(Debug, Clone)]
pub struct LabelDefinition {
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
    /// Create new header
    pub fn new(version: usize, backward_compat: usize) -> Self {
        Self {
            version,
            backward_compat,
            file_hash: None,
            labels: Vec::new(),
        }
    }

    /// Add a label definition
    pub fn add_label(&mut self, label: LabelDefinition) {
        self.labels.push(label);
    }

    /// Encode header to bytes (following basecalc pattern)
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut header = Vec::new();

        // Magic number
        header.extend_from_slice("RÅ".as_bytes());

        // Header start marker
        header.push(b'<');

        let header_length_placeholder = VsfType::b(0, true).flatten();
        header.extend_from_slice(&header_length_placeholder);

        // Version
        header.extend_from_slice(&VsfType::z(self.version).flatten());

        // Backward compatibility
        header.extend_from_slice(&VsfType::y(self.backward_compat).flatten());

        // File hash (optional)
        if let Some(ref hash) = self.file_hash {
            header.extend_from_slice(&hash.flatten());
        }

        // Label count
        header.extend_from_slice(&VsfType::n(self.labels.len()).flatten());

        // Label definitions
        for label in &self.labels {
            header.push(b'(');

            // Label name
            header.extend_from_slice(&VsfType::d(label.name.clone()).flatten());

            // Optional hash (VsfType::h with algorithm)
            if let Some(ref hash_type) = label.hash {
                header.extend_from_slice(&hash_type.flatten());
            }

            // Optional signature (VsfType::g with algorithm)
            if let Some(ref sig_type) = label.signature {
                header.extend_from_slice(&sig_type.flatten());
            }

            // Optional key (VsfType::k with algorithm)
            if let Some(ref key_type) = label.key {
                header.extend_from_slice(&key_type.flatten());
            }

            // Optional wrap (VsfType::v with algorithm)
            if let Some(ref wrap_type) = label.wrap {
                header.extend_from_slice(&wrap_type.flatten());
            }

            // Offset (in bytes)
            header.extend_from_slice(&VsfType::o(label.offset_bytes).flatten());

            // Size (in bytes)
            header.extend_from_slice(&VsfType::b(label.size_bytes, false).flatten());

            // Child count (omit if encrypted - implied to be n[0])
            if label.wrap.is_none() {
                header.extend_from_slice(&VsfType::n(label.child_count).flatten());
            }

            header.push(b')');
        }

        // Header end marker
        header.push(b'>');

        Ok(header)
    }

    /// Update header length field after knowing final size
    pub fn update_header_length(header_bytes: &mut Vec<u8>) -> Result<(), String> {
        // Find the position after "RÅ<" (4 bytes: R=1, Å=2, <=1)
        if header_bytes.len() < 5 {
            return Err("Header too short".to_string());
        }

        // Find placeholder size
        let placeholder_len = header_bytes
            .iter()
            .skip(4)
            .position(|&b| b == b'z')
            .ok_or("Could not find version marker")?;

        // Calculate what the header length will be AFTER we replace the placeholder
        // Current length - placeholder + new encoding
        // We need to iterate to find the right encoding size
        let mut header_length_bytes = header_bytes.len();
        let mut length_encoded = VsfType::b(header_length_bytes, true).flatten();

        // Iterate until stable (in case encoding size changes)
        loop {
            let new_total = header_bytes.len() - placeholder_len + length_encoded.len();
            if new_total == header_length_bytes {
                break; // Stable!
            }
            header_length_bytes = new_total;
            length_encoded = VsfType::b(header_length_bytes, true).flatten();
        }

        // Remove old placeholder
        header_bytes.drain(4..4 + placeholder_len);

        // Insert new length encoding
        for (i, byte) in length_encoded.iter().enumerate() {
            header_bytes.insert(4 + i, *byte);
        }

        Ok(())
    }
}

/// Section of structured data (has children)
#[derive(Debug, Clone)]
pub struct VsfSection {
    pub name: String,
    pub items: Vec<VsfItem>,
}

/// Single item in a section
#[derive(Debug, Clone)]
pub struct VsfItem {
    pub name: String,
    pub value: VsfType,
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
            items: Vec::new(),
        }
    }

    /// Add an item to the section with validated field name
    ///
    /// # Panics
    /// Panics if the field name contains invalid characters
    pub fn add_item(&mut self, name: impl Into<String>, value: VsfType) {
        let name_str = name.into();
        validate_name(&name_str).unwrap_or_else(|e| panic!("Invalid field name: {}", e));
        self.items.push(VsfItem {
            name: name_str,
            value,
        });
    }

    /// Encode section to bytes (no preamble - crypto moved to header labels)
    ///
    /// Format: [dsection_name(field:value)...]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Section start
        bytes.push(b'[');

        // Section name (namespace for all fields)
        bytes.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

        // Encode each item
        for item in &self.items {
            bytes.push(b'(');

            // Item name (simple identifier, no dots - namespace comes from section)
            bytes.extend_from_slice(&VsfType::d(item.name.clone()).flatten());

            // Separator
            bytes.push(b':');

            // Item value
            bytes.extend_from_slice(&item.value.flatten());

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
        header.add_label(LabelDefinition {
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
        section.add_item("width", VsfType::u(4096, false));
        section.add_item("height", VsfType::u(3072, false));

        let encoded = section.encode();

        // Verify no preamble (starts with '[')
        assert_eq!(encoded[0], b'[');
        assert_eq!(encoded[encoded.len() - 1], b']');

        // Verify parentheses for items
        assert!(encoded.contains(&b'('));
        assert!(encoded.contains(&b')'));
        assert!(encoded.contains(&b':')); // Separator
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
        section.add_item("ISO Speed", VsfType::f5(800.0)); // uppercase and space
    }
}
