//! High-level builder for VSF files
//!
//! Uses the Vec<Vec<u8>> pattern from basecalc with stabilization loop
//! to handle the chicken-and-egg problem of header size calculation.

use crate::file_format::VsfSection;
use crate::types::VsfType;
use crate::{VSF_BACKWARD_COMPAT, VSF_VERSION};

/// Builder for complete VSF files with headers and sections
pub struct VsfBuilder {
    version: usize,
    backward_compat: usize,
    sections: Vec<VsfSection>,
    unboxed: Vec<(String, Vec<u8>)>,
}

impl VsfBuilder {
    /// Create a new VSF file builder
    pub fn new() -> Self {
        Self {
            version: VSF_VERSION,
            backward_compat: VSF_BACKWARD_COMPAT,
            sections: Vec::new(),
            unboxed: Vec::new(),
        }
    }

    /// Set version numbers
    pub fn version(mut self, version: usize, backward_compat: usize) -> Self {
        self.version = version;
        self.backward_compat = backward_compat;
        self
    }

    /// Add a structured section with name and items
    pub fn add_section(mut self, name: impl Into<String>, items: Vec<(String, VsfType)>) -> Self {
        let mut section = VsfSection::new(name);
        for (item_name, value) in items {
            section.add_item(item_name, value);
        }
        self.sections.push(section);
        self
    }

    /// Add an unboxed data blob (zero-copy section)
    pub fn add_unboxed(mut self, name: impl Into<String>, data: Vec<u8>) -> Self {
        self.unboxed.push((name.into(), data));
        self
    }

    /// Build complete VSF file using Vec<Vec<u8>> pattern with stabilization loop
    pub fn build(self) -> Result<Vec<u8>, String> {
        // Pre-encode all sections to know their sizes
        let mut section_data: Vec<Vec<u8>> = Vec::new();
        for section in &self.sections {
            section_data.push(section.encode());
        }

        // Initialize vsf as Vec<Vec<u8>> like basecalc
        let mut vsf: Vec<Vec<u8>> = Vec::new();

        // Magic number
        vsf.push("RÅ".as_bytes().to_vec());

        // Header start
        let mut header_index = 0;
        vsf[header_index].push(b'<');

        // Placeholder for header length
        let header_length_index = vsf.len();
        vsf.push(VsfType::b(0).flatten()); // Will be updated in loop

        // Version and backward compat
        header_index = vsf.len();
        vsf.push(VsfType::z(self.version).flatten());
        vsf[header_index].extend_from_slice(&VsfType::y(self.backward_compat).flatten());

        // Label count
        let total_labels = self.sections.len() + self.unboxed.len();
        vsf[header_index].extend_from_slice(&VsfType::n(total_labels).flatten());

        // Create label definitions with placeholders
        let mut label_offset_indices = Vec::new();
        let mut label_size_indices = Vec::new();

        for (i, section) in self.sections.iter().enumerate() {
            vsf[header_index].push(b'(');
            vsf[header_index].extend_from_slice(&VsfType::d(section.name.clone()).flatten());

            // Offset placeholder
            label_offset_indices.push((i, vsf.len()));
            vsf.push(VsfType::o(0).flatten());

            // Size placeholder
            label_size_indices.push((i, vsf.len()));
            vsf.push(VsfType::b(0).flatten());

            // Child count (actual value)
            header_index = vsf.len();
            vsf.push(VsfType::n(section.items.len()).flatten());
            vsf[header_index].push(b')');
        }

        // Unboxed sections
        for (i, (name, _)) in self.unboxed.iter().enumerate() {
            vsf[header_index].push(b'(');
            vsf[header_index].extend_from_slice(&VsfType::d(name.clone()).flatten());

            // Offset placeholder
            let unboxed_index = self.sections.len() + i;
            label_offset_indices.push((unboxed_index, vsf.len()));
            vsf.push(VsfType::o(0).flatten());

            // Size placeholder
            label_size_indices.push((unboxed_index, vsf.len()));
            vsf.push(VsfType::b(0).flatten());

            // Child count = 0 for unboxed
            header_index = vsf.len();
            vsf.push(VsfType::n(0).flatten());
            vsf[header_index].push(b')');
        }

        // Close header
        vsf[header_index].push(b'>');
        let header_end_index = vsf.len();

        // Add section data
        for section_bytes in section_data {
            vsf.push(section_bytes);
        }

        // Stabilization loop (like basecalc)
        let mut prev_header_length = 0;
        let mut prev_offsets = vec![0; label_offset_indices.len()];
        let mut prev_sizes = vec![0; label_size_indices.len()];

        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 10;

        while iteration < MAX_ITERATIONS {
            let mut changed = false;

            // Calculate header length
            let mut header_length = 0;
            for i in 0..header_end_index {
                header_length += vsf[i].len();
            }

            if header_length != prev_header_length {
                vsf[header_length_index] = VsfType::b(header_length * 8).flatten();
                prev_header_length = header_length;
                changed = true;
            }

            // Calculate offsets and sizes for sections
            let mut current_offset = header_length;

            for (idx, (label_idx, vsf_idx)) in label_offset_indices.iter().enumerate() {
                let offset_bits = current_offset * 8;

                if offset_bits != prev_offsets[idx] {
                    vsf[*vsf_idx] = VsfType::o(offset_bits).flatten();
                    prev_offsets[idx] = offset_bits;
                    changed = true;
                }

                // Calculate size
                let size_bytes = if *label_idx < self.sections.len() {
                    // Structured section
                    vsf[header_end_index + label_idx].len()
                } else {
                    // Unboxed section
                    let unboxed_idx = label_idx - self.sections.len();
                    self.unboxed[unboxed_idx].1.len()
                };

                let size_bits = size_bytes * 8;

                if size_bits != prev_sizes[idx] {
                    vsf[label_size_indices[idx].1] = VsfType::b(size_bits).flatten();
                    prev_sizes[idx] = size_bits;
                    changed = true;
                }

                current_offset += size_bytes;
            }

            if !changed {
                break; // Stabilized
            }

            iteration += 1;
        }

        if iteration >= MAX_ITERATIONS {
            return Err("Failed to stabilize header after 10 iterations".to_string());
        }

        // Flatten vsf
        let mut result: Vec<u8> = vsf.into_iter().flatten().collect();

        // Append unboxed data
        for (_, data) in self.unboxed {
            result.extend_from_slice(&data);
        }

        Ok(result)
    }
}

impl Default for VsfBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_file() {
        let result = VsfBuilder::new()
            .add_section(
                "metadata",
                vec![
                    ("width".to_string(), VsfType::u(1920, false)),
                    ("height".to_string(), VsfType::u(1080, false)),
                ],
            )
            .build();

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());
        assert_eq!(bytes[3], b'<');

        // Should end with section bracket
        let last_bracket_pos = bytes.iter().rposition(|&b| b == b']');
        assert!(last_bracket_pos.is_some());
    }

    #[test]
    fn test_with_unboxed() {
        let pixel_data = vec![0xFF; 1024];

        let result = VsfBuilder::new()
            .add_section(
                "metadata",
                vec![
                    ("width".to_string(), VsfType::u(32, false)),
                    ("height".to_string(), VsfType::u(32, false)),
                ],
            )
            .add_unboxed("pixels", pixel_data.clone())
            .build();

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Verify magic number (RÅ is 3 bytes in UTF-8)
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Last 1024 bytes should be pixel data
        let len = bytes.len();
        assert_eq!(&bytes[len - 1024..], &pixel_data[..]);
    }

    #[test]
    fn test_multiple_sections() {
        let result = VsfBuilder::new()
            .add_section(
                "section1",
                vec![("field1".to_string(), VsfType::u(100, false))],
            )
            .add_section(
                "section2",
                vec![("field2".to_string(), VsfType::u(200, false))],
            )
            .build();

        assert!(result.is_ok());
        let bytes = result.unwrap();

        // Should have two bracketed sections
        let bracket_count = bytes.iter().filter(|&&b| b == b'[').count();
        assert_eq!(bracket_count, 2);
    }
}
