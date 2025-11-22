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

    /// Add a pre-built VsfSection directly
    /// Use this when you need fields with multiple values (Vec<VsfType>)
    pub fn add_section_direct(mut self, section: VsfSection) -> Self {
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

        // Create header field definitions (section pointers)
        // Note: Using VsfField::flatten() for cleaner separator handling
        let mut field_offset_indices = Vec::new();
        let mut field_size_indices = Vec::new();

        for (i, section) in self.sections.iter().enumerate() {
            // Create field with placeholder values
            let field = crate::file_format::VsfField::new(&section.name)
                .with_value(VsfType::o(0))        // offset placeholder
                .with_value(VsfType::b(0, false)) // size placeholder
                .with_value(VsfType::n(section.fields.len())); // child count

            let field_bytes = field.flatten();

            // Track indices for stabilization loop
            // We'll need to rebuild entire field when updating
            field_offset_indices.push((i, vsf.len()));
            field_size_indices.push((i, vsf.len()));

            header_index = vsf.len();
            vsf.push(field_bytes);
        }

        // Unboxed sections
        for (i, (name, _)) in self.unboxed.iter().enumerate() {
            let unboxed_index = self.sections.len() + i;

            let field = crate::file_format::VsfField::new(name)
                .with_value(VsfType::o(0))        // offset placeholder
                .with_value(VsfType::b(0, false)) // size placeholder
                .with_value(VsfType::n(0));       // no children for unboxed

            let field_bytes = field.flatten();

            field_offset_indices.push((unboxed_index, vsf.len()));
            field_size_indices.push((unboxed_index, vsf.len()));

            header_index = vsf.len();
            vsf.push(field_bytes);
        }

        // Close header with '>' as a separate chunk (so it's included in header_length)
        vsf.push(vec![b'>']);
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

                // Calculate size
                let size_bytes = if *field_idx < self.sections.len() {
                    // Structured section
                    vsf[header_end_index + field_idx].len()
                } else {
                    // Unboxed section
                    let unboxed_idx = field_idx - self.sections.len();
                    self.unboxed[unboxed_idx].1.len()
                };

                // Rebuild entire field if offset or size changed
                if offset_bytes != prev_offsets[idx] || size_bytes != prev_sizes[idx] {
                    // Get section name and child count
                    let (name, child_count) = if *field_idx < self.sections.len() {
                        (&self.sections[*field_idx].name, self.sections[*field_idx].fields.len())
                    } else {
                        let unboxed_idx = field_idx - self.sections.len();
                        (&self.unboxed[unboxed_idx].0, 0)
                    };

                    // Rebuild field with updated values using VsfField API
                    let field = crate::file_format::VsfField::new(name)
                        .with_value(VsfType::o(offset_bytes))
                        .with_value(VsfType::b(size_bytes, false))
                        .with_value(VsfType::n(child_count));

                    vsf[*vsf_idx] = field.flatten();

                    prev_offsets[idx] = offset_bytes;
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
        use crate::verification::{
            compute_provenance_hash, write_provenance_hash,
            compute_file_hash, write_file_hash
        };

        // 1. Compute and write provenance hash (hp) first
        let prov_hash = compute_provenance_hash(&result)?;
        result = write_provenance_hash(result, &prov_hash)?;

        // 2. Compute and write rolling hash (hb) after provenance hash is set
        let file_hash = compute_file_hash(&result)?;
        result = write_file_hash(result, &file_hash)?;

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

        // Verify file structure (instead of counting all '[' bytes which may appear in binary data)
        // Should have magic number
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Should have header
        assert_eq!(bytes[3], b'<');

        // Should find section names in the output
        let output_str = String::from_utf8_lossy(&bytes);
        assert!(output_str.contains("section1"));
        assert!(output_str.contains("section2"));
    }
}
