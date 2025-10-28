//! VSF file format with headers, preambles, and hierarchical labels
//!
//! Binary structure (following basecalc pattern):
//! ```text
//! RÅ<                                    Magic + header start
//!   b[header_length_bits]                Header length in BITS
//!   z[version]                           Version number
//!   y[backward_compat]                   Backward compatibility version
//!   n[label_count]                       Number of label definitions
//!
//!   (d[label_name] o[offset] b[size] n[count])  Label definition
//!   ...
//! >                                      Header end
//!
//! {n[count] b[size]}                     Preamble (metadata about label set)
//! [                                      Section start (if n > 0)
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
        return Err(format!("Invalid name '{}' - cannot start or end with dot", name));
    }
    if name.starts_with('_') || name.ends_with('_') {
        return Err(format!("Invalid name '{}' - cannot start or end with underscore", name));
    }

    // Check for consecutive dots or underscores
    if name.contains("..") {
        return Err(format!("Invalid name '{}' - cannot contain consecutive dots", name));
    }
    if name.contains("__") {
        return Err(format!("Invalid name '{}' - cannot contain consecutive underscores", name));
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
    pub labels: Vec<LabelDefinition>,
}

/// Label definition in header
#[derive(Debug, Clone)]
pub struct LabelDefinition {
    pub name: String,
    pub offset_bits: usize, // Offset in BITS (not bytes!)
    pub size_bits: usize,   // Size in BITS (not bytes!) - includes preamble
    pub child_count: usize, // 0 = unboxed blob, N = N structured children
}

/// Preamble metadata for a label set
///
/// Appears before every label set as: {n[count] b[size] h?[hash] g?[sig]}
/// Enables forensic recovery and integrity checking.
#[derive(Debug, Clone)]
pub struct Preamble {
    pub count: usize,               // n: number of labels in this set
    pub size_bits: usize,           // b: total size in BITS (includes preamble itself)
    pub hash: Option<Vec<u8>>,      // h: optional integrity hash
    pub signature: Option<Vec<u8>>, // g: optional authentication signature
}

impl Preamble {
    /// Create a new preamble with count and size (no hash/signature)
    pub fn new(count: usize, size_bits: usize) -> Self {
        Self {
            count,
            size_bits,
            hash: None,
            signature: None,
        }
    }

    /// Encode preamble to bytes: {n[count] b[size] h?[hash] g?[sig]}
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Opening brace
        bytes.push(b'{');

        // n: count
        bytes.push(b'n');
        bytes.extend_from_slice(&self.count.encode_number());

        // b: size in bits
        bytes.push(b'b');
        bytes.extend_from_slice(&self.size_bits.encode_number());

        // h: hash (optional)
        if let Some(ref hash) = self.hash {
            bytes.push(b'h');
            bytes.extend_from_slice(&hash.len().encode_number());
            bytes.extend_from_slice(hash);
        }

        // g: signature (optional)
        if let Some(ref sig) = self.signature {
            bytes.push(b'g');
            bytes.extend_from_slice(&sig.len().encode_number());
            bytes.extend_from_slice(sig);
        }

        // Closing brace
        bytes.push(b'}');

        bytes
    }
}

impl VsfHeader {
    /// Create new header
    pub fn new(version: usize, backward_compat: usize) -> Self {
        Self {
            version,
            backward_compat,
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

        // Header length placeholder (will be calculated later)
        let header_length_placeholder = VsfType::b(0).flatten();
        header.extend_from_slice(&header_length_placeholder);

        // Version
        header.extend_from_slice(&VsfType::z(self.version).flatten());

        // Backward compatibility
        header.extend_from_slice(&VsfType::y(self.backward_compat).flatten());

        // Label count
        header.extend_from_slice(&VsfType::n(self.labels.len()).flatten());

        // Label definitions
        for label in &self.labels {
            header.push(b'(');

            // Label name
            header.extend_from_slice(&VsfType::d(label.name.clone()).flatten());

            // Offset (in bits)
            header.extend_from_slice(&VsfType::o(label.offset_bits).flatten());

            // Size (in bits)
            header.extend_from_slice(&VsfType::b(label.size_bits).flatten());

            // Child count
            header.extend_from_slice(&VsfType::n(label.child_count).flatten());

            header.push(b')');
        }

        // Header end marker
        header.push(b'>');

        Ok(header)
    }

    /// Update header length field after knowing final size
    pub fn update_header_length(header_bytes: &mut Vec<u8>) -> Result<(), String> {
        // Find the position after "RÅ<" (3 bytes)
        if header_bytes.len() < 4 {
            return Err("Header too short".to_string());
        }

        // Calculate actual header length in bits
        let header_length_bits = header_bytes.len() * 8;

        // Encode the length
        let length_encoded = VsfType::b(header_length_bits).flatten();

        // Replace placeholder starting at position 3
        let placeholder_len = header_bytes
            .iter()
            .skip(3)
            .position(|&b| b == b'z')
            .ok_or("Could not find version marker")?;

        // Remove old placeholder
        header_bytes.drain(3..3 + placeholder_len);

        // Insert new length encoding
        for (i, byte) in length_encoded.iter().enumerate() {
            header_bytes.insert(3 + i, *byte);
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
        validate_name(&name_str)
            .unwrap_or_else(|e| panic!("Invalid section name: {}", e));
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
        validate_name(&name_str)
            .unwrap_or_else(|e| panic!("Invalid field name: {}", e));
        self.items.push(VsfItem {
            name: name_str,
            value,
        });
    }

    /// Encode section to bytes (with preamble and brackets)
    ///
    /// Format: {n[count] b[size]}[dsection_name(field:value)...]
    pub fn encode(&self) -> Vec<u8> {
        let mut section_body = Vec::new();

        // Section start
        section_body.push(b'[');

        // Section name (namespace for all fields)
        section_body.extend_from_slice(&VsfType::d(self.name.clone()).flatten());

        // Encode each item
        for item in &self.items {
            section_body.push(b'(');

            // Item name (simple identifier, no dots - namespace comes from section)
            section_body.extend_from_slice(&VsfType::d(item.name.clone()).flatten());

            // Separator
            section_body.push(b':');

            // Item value
            section_body.extend_from_slice(&item.value.flatten());

            section_body.push(b')');
        }

        // Section end
        section_body.push(b']');

        // Create preamble with placeholder size (will be correct after we know body size)
        let preamble = Preamble::new(self.items.len(), 0); // Placeholder size
        let preamble_bytes = preamble.encode();

        // Calculate actual total size including preamble
        let actual_total_size_bits = (preamble_bytes.len() + section_body.len()) * 8;

        // Re-encode preamble with correct size
        let preamble_final = Preamble::new(self.items.len(), actual_total_size_bits);
        let preamble_final_bytes = preamble_final.encode();

        // Combine: {n b}[...]
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&preamble_final_bytes);
        bytes.extend_from_slice(&section_body);

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
            offset_bits: 512,
            size_bits: 256,
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

        // Verify preamble (starts with '{')
        assert_eq!(encoded[0], b'{');
        assert!(encoded.contains(&b'n')); // count
        assert!(encoded.contains(&b'b')); // size
        assert!(encoded.contains(&b'}')); // closing brace

        // Verify brackets
        assert!(encoded.contains(&b'['));
        assert_eq!(encoded[encoded.len() - 1], b']');

        // Verify parentheses for items
        assert!(encoded.contains(&b'('));
        assert!(encoded.contains(&b')'));
        assert!(encoded.contains(&b':')); // Separator
    }

    #[test]
    fn test_preamble_encoding() {
        let preamble = Preamble::new(5, 847 * 8); // 5 items, 847 bytes = 6776 bits
        let bytes = preamble.encode();

        // Should start and end with braces
        assert_eq!(bytes[0], b'{');
        assert_eq!(bytes[bytes.len() - 1], b'}');

        // Should contain 'n' and 'b' markers
        assert!(bytes.contains(&b'n'));
        assert!(bytes.contains(&b'b'));

        // Should not contain hash or signature (None)
        assert!(!bytes.contains(&b'h'));
        assert!(!bytes.contains(&b'g'));
    }

    #[test]
    fn test_preamble_with_hash() {
        let mut preamble = Preamble::new(3, 500 * 8);
        preamble.hash = Some(vec![0xAB; 32]); // 32-byte hash

        let bytes = preamble.encode();

        // Should contain hash marker
        assert!(bytes.contains(&b'h'));
        assert!(bytes.windows(32).any(|w| w == &vec![0xAB; 32][..]));
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
