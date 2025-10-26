//! VSF file format with headers and hierarchical labels
//!
//! Binary structure (following basecalc pattern):
//! ```text
//! RÅ<                                    Magic + header start
//!   b[header_length_bits]                Header length in BITS
//!   z[version]                           Version number
//!   y[backward_compat]                   Backward compatibility version
//!   c[label_count]                       Number of label definitions
//!
//!   (d[label_name] o[offset] b[size] c[count])  Label definition
//!   ...
//! >                                      Header end
//!
//! [                                      Section start (if c > 0)
//!   (d[field_name]:[value])              Field definition
//!   ...
//! ]                                      Section end
//!
//! [raw_bytes...]                         Unboxed data (if c = 0)
//! ```

use crate::types::VsfType;

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
    pub offset_bits: usize,    // Offset in BITS (not bytes!)
    pub size_bits: usize,      // Size in BITS (not bytes!)
    pub child_count: usize,    // 0 = unboxed blob, N = N structured children
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
        let placeholder_len = header_bytes.iter()
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
    /// Create new section
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            items: Vec::new(),
        }
    }

    /// Add an item to the section
    pub fn add_item(&mut self, name: impl Into<String>, value: VsfType) {
        self.items.push(VsfItem {
            name: name.into(),
            value,
        });
    }

    /// Encode section to bytes (with brackets)
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Section start
        bytes.push(b'[');

        // Encode each item
        for item in &self.items {
            bytes.push(b'(');

            // Item name
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

        // Verify brackets
        assert_eq!(encoded[0], b'[');
        assert_eq!(encoded[encoded.len() - 1], b']');

        // Verify parentheses for items
        assert!(encoded.contains(&b'('));
        assert!(encoded.contains(&b')'));
        assert!(encoded.contains(&b':')); // Separator
    }
}
