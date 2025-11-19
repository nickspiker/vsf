//! High-level builder for VSF files
//!
//! Uses the Vec<Vec<u8>> pattern with stabilization loop
//! to handle the chicken-and-egg problem of header size calculation.
//!
//! **Note:** Every VSF file requires and automatically includes a BLAKE3 hash in the header
//! for integrity verification. This is computed transparently during `build()`.
//! No manual hashing required - just call `builder.build()` and you're done!

use crate::encoding::hash_placeholder;
use crate::file_format::VsfSection;
use crate::types::VsfType;
use crate::{VSF_BACKWARD_COMPAT, VSF_VERSION};

/// Builder for complete VSF files with headers and sections
///
/// All VSF files automatically include a BLAKE3 hash for integrity verification.
pub struct VsfBuilder {
    version: usize,
    backward_compat: usize,
    creation_time: VsfType, // Creation timestamp (ef5 for ~2min precision)
    sections: Vec<VsfSection>,
    unboxed: Vec<(String, Vec<u8>)>,
    include_file_hash: bool, // Always true - BLAKE3 hash is mandatory for all VSF files
}

impl VsfBuilder {
    /// Create a new VSF file builder
    ///
    /// **Note:** Every VSF file automatically includes:
    /// - Creation timestamp (current time as ef5, ~2min precision)
    /// - BLAKE3 hash for integrity verification (computed during `build()`)
    pub fn new() -> Self {
        use crate::types::EtType;
        use chrono::Utc;

        // Get current time and convert to Eagle Time f32
        let now = Utc::now();
        let et = crate::datetime_to_eagle_time(now);
        let et_f32 = match et.et_type() {
            EtType::f6(v) => *v as f32,
            EtType::f5(v) => *v,
            EtType::u(v) => *v as f32,
            EtType::i(v) => *v as f32,
        };

        Self {
            version: VSF_VERSION,
            backward_compat: VSF_BACKWARD_COMPAT,
            creation_time: VsfType::e(EtType::f5(et_f32)),
            sections: Vec::new(),
            unboxed: Vec::new(),
            include_file_hash: true, // Always hash - required for all VSF files
        }
    }

    /// Set version numbers
    pub fn version(mut self, version: usize, backward_compat: usize) -> Self {
        self.version = version;
        self.backward_compat = backward_compat;
        self
    }

    /// Add a structured section with name and items
    pub fn add_section(mut self, name: impl Into<String>, fields: Vec<(String, VsfType)>) -> Self {
        let mut section = VsfSection::new(name);
        for (field_name, value) in fields {
            section.add_field(field_name, value);
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

        // Version (FIRST - determines all encoding decisions)
        header_index = vsf.len();
        vsf.push(VsfType::z(self.version).flatten());

        // Backward compat version
        vsf[header_index].extend_from_slice(&VsfType::y(self.backward_compat).flatten());

        // Placeholder for header length (now we know how to encode it!)
        let header_length_index = vsf.len();
        vsf.push(VsfType::b(0, true).flatten()); // Will be updated in loop

        // Creation time
        header_index = vsf.len();
        vsf.push(self.creation_time.flatten());

        // Provenance hash placeholder (required, always BLAKE3)
        vsf[header_index].extend_from_slice(&hash_placeholder(b'p', 32));

        // Rolling hash placeholder OR signature placeholder (optional, default enabled)
        if self.include_file_hash {
            vsf[header_index].extend_from_slice(&hash_placeholder(b'b', 32));
        }

        // If signed, signature would replace hp bytes here (ge replaces hp)

        // Header field count (number of section pointers)
        let total_fields = self.sections.len() + self.unboxed.len();
        vsf[header_index].extend_from_slice(&VsfType::n(total_fields).flatten());

        // Create header field definitions (section pointers) with placeholders
        let mut field_offset_indices = Vec::new();
        let mut field_size_indices = Vec::new();

        for (i, section) in self.sections.iter().enumerate() {
            vsf[header_index].push(b'(');
            vsf[header_index].extend_from_slice(&VsfType::d(section.name.clone()).flatten());

            // Offset placeholder (POSITIONAL - no colon)
            field_offset_indices.push((i, vsf.len()));
            vsf.push(VsfType::o(0).flatten());

            // Size placeholder (POSITIONAL - no colon, not inclusive - section size, not self-referential)
            field_size_indices.push((i, vsf.len()));
            vsf.push(VsfType::b(0, false).flatten());

            // Child count (POSITIONAL - no colon, actual value)
            header_index = vsf.len();
            vsf.push(VsfType::n(section.fields.len()).flatten());
            vsf[header_index].push(b')');
        }

        // Unboxed sections
        for (i, (name, _)) in self.unboxed.iter().enumerate() {
            vsf[header_index].push(b'(');
            vsf[header_index].extend_from_slice(&VsfType::d(name.clone()).flatten());

            // Offset placeholder (POSITIONAL - no colon)
            let unboxed_index = self.sections.len() + i;
            field_offset_indices.push((unboxed_index, vsf.len()));
            vsf.push(VsfType::o(0).flatten());

            // Size placeholder (POSITIONAL - no colon, not inclusive - section size, not self-referential)
            field_size_indices.push((unboxed_index, vsf.len()));
            vsf.push(VsfType::b(0, false).flatten());

            // Child count = 0 for unboxed (POSITIONAL - no colon)
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

        // Stabilization loop
        let mut prev_header_length = 0;
        let mut prev_offsets = vec![0; field_offset_indices.len()];
        let mut prev_sizes = vec![0; field_size_indices.len()];

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
                vsf[header_length_index] = VsfType::b(header_length, true).flatten();
                prev_header_length = header_length;
                changed = true;
            }

            // Calculate offsets and sizes for sections
            let mut current_offset = header_length;

            for (idx, (field_idx, vsf_idx)) in field_offset_indices.iter().enumerate() {
                let offset_bytes = current_offset;

                if offset_bytes != prev_offsets[idx] {
                    vsf[*vsf_idx] = VsfType::o(offset_bytes).flatten();
                    prev_offsets[idx] = offset_bytes;
                    changed = true;
                }

                // Calculate size
                let size_bytes = if *field_idx < self.sections.len() {
                    // Structured section
                    vsf[header_end_index + field_idx].len()
                } else {
                    // Unboxed section
                    let unboxed_idx = field_idx - self.sections.len();
                    self.unboxed[unboxed_idx].1.len()
                };

                if size_bytes != prev_sizes[idx] {
                    vsf[field_size_indices[idx].1] = VsfType::b(size_bytes, false).flatten();
                    prev_sizes[idx] = size_bytes;
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

        // Now structure is finalized - compute and write all crypto primitives
        use crate::verification::{compute_file_hash, write_file_hash};

        let hash = compute_file_hash(&result)?;
        result = write_file_hash(result, &hash)?;

        // If signed, signature would get computed and written here

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

        let bytes = result.expect("Failed to build VSF file");

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
