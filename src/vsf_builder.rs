//! High-level builder for VSF files
//!
//! Uses the Vec<Vec<u8>> pattern with stabilization loop to handle the chicken-and-egg problem of header size calculation.
//!
//! **Note:** Every VSF file requires and automatically includes a BLAKE3 hash in the header for integrity verification. This is computed transparently during `build()`. No manual hashing required - just call `builder.build()` and you're done!

use crate::encoding::hash_placeholder;
use crate::file_format::VsfSection;
use crate::prelude::*;
use crate::types::VsfType;
use crate::{VSF_BACKWARD_COMPAT, VSF_VERSION};

/// Per-section cryptographic metadata for header fields
///
/// This allows setting a key (k) field on individual sections. Used for sections where the header needs to indicate the cryptographic key.
#[derive(Clone, Default)]
pub struct SectionMeta {
    /// Cryptographic key associated with this section (ke for Ed25519, kx for X25519)
    pub key: Option<VsfType>,
}

impl SectionMeta {
    /// Create new section metadata with a key
    pub fn new(key: VsfType) -> Self {
        Self { key: Some(key) }
    }
}

/// Builder for complete VSF files with headers and sections
///
/// All VSF files automatically include a BLAKE3 hash for integrity verification.
pub struct VsfBuilder {
    version: usize,
    backward_compat: usize,
    /// Creation timestamp. `None` means the device cannot know the current time (no clock, no network) and the header omits the `e` field entirely rather than emitting a fake/zero value — see the README and `pipe_message` schema docs. `Some(VsfType::e(…))` writes a standard `eu6` / `ef5` / `ef6` field.
    creation_time: Option<VsfType>,
    sections: Vec<(VsfSection, SectionMeta)>, // Section with optional crypto metadata
    unboxed: Vec<(String, Vec<u8>, SectionMeta)>, // Name, data, and optional crypto metadata
    inline_fields: Vec<(String, Vec<VsfType>)>, // Metadata-only fields: (name, inline values)
    include_file_hash: bool,                  // True for rolling hash, false if signed
    custom_provenance: Option<[u8; 32]>,      // Custom provenance hash (immutable identity)
    signer_pubkey: Option<VsfType>, // Signer's Ed25519 pubkey (ke) - for signature verification
    signature: Option<(VsfType, [u8; 64])>, // (signature type, signature bytes) - replaces hb
    avatar_hash: Option<[u8; 32]>,  // Optional avatar provenance hash as header field
}

impl VsfBuilder {
    /// Create a new VSF file builder.
    ///
    /// Creation time defaults to **absent** — callers should set it explicitly with `creation_time_oscillations(...)` (or `creation_time_nanos(...)` for the legacy float form) before `build()` if they know what time it is. Devices without a clock (no RTC, no network, pre-handshake) simply leave it unset; the resulting VSF document omits the `e` field rather than emitting a fake timestamp. Every VSF file always includes a BLAKE3 hash for integrity verification (computed during `build()`).
    pub fn new() -> Self {
        Self {
            version: VSF_VERSION,
            backward_compat: VSF_BACKWARD_COMPAT,
            creation_time: None,
            sections: Vec::new(),
            unboxed: Vec::new(),
            inline_fields: Vec::new(),
            include_file_hash: true, // True for rolling hash, false if signed
            custom_provenance: None,
            signer_pubkey: None,
            signature: None,
            avatar_hash: None,
        }
    }

    /// Set creation time with integer oscillations (e6 wire format). 1,420,407,826 oscillations per second (21cm hydrogen line) = 704ps precision.
    pub fn creation_time_oscillations(mut self, oscillations: i64) -> Self {
        use crate::types::EtType;
        self.creation_time = Some(VsfType::e(EtType::e6(oscillations)));
        self
    }

    /// Set creation time with nanosecond precision (ef6) — DEPRECATED. Use `creation_time_oscillations()` for integer timestamps instead.
    #[deprecated(note = "Use creation_time_oscillations() for integer timestamps")]
    pub fn creation_time_nanos(mut self, eagle_time: f64) -> Self {
        use crate::types::EtType;
        #[allow(deprecated)]
        {
            self.creation_time = Some(VsfType::e(EtType::f6(eagle_time)));
        }
        self
    }

    /// Set a custom provenance hash (immutable content identity)
    ///
    /// Use this when the provenance hash has specific meaning in your protocol, such as linking a response to a challenge. The provenance hash will NOT be recomputed during build() - it stays exactly as you set it.
    pub fn provenance_hash(mut self, hash: [u8; 32]) -> Self {
        self.custom_provenance = Some(hash);
        self
    }

    /// Add an Ed25519 signature to the header (replaces rolling hash)
    ///
    /// The signature should sign the provenance hash. When a signature is present, no rolling hash (hb) is included - the signature provides integrity verification.
    ///
    /// Both pubkey (ke) and signature (ge) are included in the header for verification.
    pub fn signature_ed25519(mut self, pubkey: [u8; 32], signature: [u8; 64]) -> Self {
        self.signer_pubkey = Some(VsfType::ke(pubkey.to_vec()));
        self.signature = Some((VsfType::ge(signature.to_vec()), signature));
        self.include_file_hash = false; // Signature replaces rolling hash
        self
    }

    /// Add an avatar provenance hash as a header field (davatar_id:hp[hash])
    ///
    /// This is the BLAKE3 provenance hash of the sender's avatar VSF file. Used in ping/pong messages to sync avatar state between peers. Stored as a metadata-only header field with name "avatar_id".
    pub fn avatar_hash(mut self, hash: [u8; 32]) -> Self {
        self.avatar_hash = Some(hash);
        self
    }

    /// Disable rolling hash - use provenance hash only
    ///
    /// Use this for files that don't need mutable state tracking or signatures. The provenance hash (hp) will still be computed for content identity.
    pub fn provenance_only(mut self) -> Self {
        self.include_file_hash = false;
        self
    }

    /// Set signer pubkey and include signature placeholder
    ///
    /// Creates a header with ke (signer pubkey) and ge (signature placeholder). The signature must be filled in externally after computing provenance hash. This mode is for protocols that need async signing (e.g., WebCrypto in Workers).
    ///
    /// # Arguments
    /// * `pubkey` - VsfType::ke with the Ed25519 public key bytes
    ///
    /// # Example
    /// ```ignore
    /// let unsigned = VsfBuilder::new() .signed_only(VsfType::ke(pubkey.to_vec())) .build()?; let hash = verification::compute_provenance_hash(&unsigned)?; let signature = sign_async(&hash, &secret_key).await; verification::fill_provenance_hash(&mut unsigned, &hash)?; verification::fill_signature(&mut unsigned, &signature)?;
    /// ```
    pub fn signed_only(mut self, pubkey: VsfType) -> Self {
        self.include_file_hash = false;
        self.signer_pubkey = Some(pubkey);
        // Signature will be a 64-byte placeholder (zeros) that must be filled externally
        self.signature = Some((VsfType::ge(vec![0u8; 64]), [0u8; 64]));
        self
    }

    /// Add an inline metadata field (header-only, no section body)
    ///
    /// Creates a field like `(d#{name}:value1,value2,...)` in the header. Unlike sections, inline fields have no offset/size/count - just name + values.
    ///
    /// Use for lightweight control packets (e.g., PT acks) where provenance hash provides integrity and inline values carry the metadata.
    ///
    /// # Example
    /// ```ignore
    /// VsfBuilder::new() .provenance_hash(chunk_hash) .provenance_only() .add_inline_field("pt_ack", vec![VsfType::u3(seq), VsfType::u3(buf)]) .build()
    /// ```
    pub fn add_inline_field(mut self, name: impl Into<String>, values: Vec<VsfType>) -> Self {
        self.inline_fields.push((name.into(), values));
        self
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
        self.sections.push((section, SectionMeta::default()));
        self
    }

    /// Add a structured section with cryptographic metadata (key)
    ///
    /// Use this for sections where the header field needs to indicate:
    /// - `key`: The cryptographic key (ke for Ed25519, kx for X25519)
    ///
    /// # Example
    /// ```ignore
    /// let builder = VsfBuilder::new() .add_section_with_meta( "encrypted_data", vec![("payload".to_string(), VsfType::v(b'e', encrypted_bytes))], SectionMeta::new(VsfType::ke(pubkey.to_vec())), );
    /// ```
    pub fn add_section_with_meta(
        mut self,
        name: impl Into<String>,
        fields: Vec<(String, VsfType)>,
        meta: SectionMeta,
    ) -> Self {
        let mut section = VsfSection::new(name);
        for (field_name, value) in fields {
            section.add_field(field_name, value);
        }
        self.sections.push((section, meta));
        self
    }

    /// Add a pre-built VsfSection directly Use this when you need fields with multiple values (Vec<VsfType>)
    pub fn add_section_direct(mut self, section: VsfSection) -> Self {
        self.sections.push((section, SectionMeta::default()));
        self
    }

    /// Add a pre-built VsfSection with cryptographic metadata
    pub fn add_section_direct_with_meta(mut self, section: VsfSection, meta: SectionMeta) -> Self {
        self.sections.push((section, meta));
        self
    }

    /// Add an unboxed data blob (zero-copy section)
    pub fn add_unboxed(mut self, name: impl Into<String>, data: Vec<u8>) -> Self {
        self.unboxed
            .push((name.into(), data, SectionMeta::default()));
        self
    }

    /// Add an unboxed data blob with cryptographic metadata
    ///
    /// Use this for encrypted sections where the header field needs to indicate the encryption key and wrapper type, but the section body is raw bytes.
    pub fn add_unboxed_with_meta(
        mut self,
        name: impl Into<String>,
        data: Vec<u8>,
        meta: SectionMeta,
    ) -> Self {
        self.unboxed.push((name.into(), data, meta));
        self
    }

    /// Build complete VSF file using Vec<Vec<u8>> pattern with stabilization loop
    pub fn build(self) -> Result<Vec<u8>, String> {
        // Pre-encode all sections to know their sizes
        let mut section_data: Vec<Vec<u8>> = Vec::new();
        for (section, _meta) in &self.sections {
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

        // Placeholder for file length (for TCP streaming)
        let file_length_index = vsf.len();
        vsf.push(VsfType::l(0, true).flatten()); // Will be updated in loop

        // Creation time — only emit the `e` field if the caller set one. Devices without a clock leave it absent.
        header_index = vsf.len();
        if let Some(et) = &self.creation_time {
            vsf.push(et.flatten());
        } else {
            vsf.push(Vec::new());
        }

        // Provenance hash - use custom if set, otherwise placeholder for auto-compute
        if let Some(custom_hp) = &self.custom_provenance {
            vsf[header_index].extend_from_slice(&VsfType::hp(custom_hp.to_vec()).flatten());
        } else {
            vsf[header_index].extend_from_slice(&hash_placeholder(b'p', 32));
        }

        // Signature (ke + ge) OR rolling hash (hb) - mutually exclusive
        if let Some(ref pubkey) = &self.signer_pubkey {
            // Include signer's public key before signature
            vsf[header_index].extend_from_slice(&pubkey.flatten());
        }
        if let Some((sig_type, _)) = &self.signature {
            // Signature replaces rolling hash
            vsf[header_index].extend_from_slice(&sig_type.flatten());
        } else if self.include_file_hash {
            // Rolling hash placeholder for auto-compute
            vsf[header_index].extend_from_slice(&hash_placeholder(b'b', 32));
        }

        // Header field count (sections + unboxed + inline_fields + optional avatar_id)
        let avatar_field_count = if self.avatar_hash.is_some() { 1 } else { 0 };
        let total_fields = self.sections.len()
            + self.unboxed.len()
            + self.inline_fields.len()
            + avatar_field_count;
        vsf[header_index].extend_from_slice(&VsfType::n(total_fields).flatten());

        // Create header field definitions (section pointers) Note: Using VsfField::flatten() for cleaner separator handling
        let mut field_offset_indices = Vec::new();
        let mut field_size_indices = Vec::new();

        for (i, (section, meta)) in self.sections.iter().enumerate() {
            // Empty sections have no body - just the name in header
            if section.fields.is_empty() {
                let mut field = crate::file_format::VsfField::new(&section.name);
                // Add key metadata if present
                if let Some(ref key) = meta.key {
                    field = field.with_value(key.clone());
                }
                vsf.push(field.flatten());
                continue;
            }

            // Create field with placeholder values
            let mut field = crate::file_format::VsfField::new(&section.name);
            // Add key metadata if present (before offset/size/n)
            if let Some(ref key) = meta.key {
                field = field.with_value(key.clone());
            }
            field = field
                .with_value(VsfType::o(0)) // offset placeholder
                .with_value(VsfType::b(0, false)) // size placeholder
                .with_value(VsfType::n(section.fields.len())); // child count

            let field_bytes = field.flatten();

            // Track indices for stabilization loop We'll need to rebuild entire field when updating
            field_offset_indices.push((i, vsf.len()));
            field_size_indices.push((i, vsf.len()));

            vsf.push(field_bytes);
        }

        // Unboxed sections
        for (i, (name, _, meta)) in self.unboxed.iter().enumerate() {
            let unboxed_index = self.sections.len() + i;

            let mut field = crate::file_format::VsfField::new(name);
            // Add key metadata if present (before offset/size/n)
            if let Some(ref key) = meta.key {
                field = field.with_value(key.clone());
            }
            field = field
                .with_value(VsfType::o(0)) // offset placeholder
                .with_value(VsfType::b(0, false)) // size placeholder
                .with_value(VsfType::n(0)); // no children for unboxed

            let field_bytes = field.flatten();

            field_offset_indices.push((unboxed_index, vsf.len()));
            field_size_indices.push((unboxed_index, vsf.len()));

            vsf.push(field_bytes);
        }

        // Inline metadata fields (no section body, just name + values in header) Format: (d#{name}:value1,value2,...) - no offset/size/count
        for (name, values) in &self.inline_fields {
            let mut field = crate::file_format::VsfField::new(name);
            for value in values {
                field = field.with_value(value.clone());
            }
            vsf.push(field.flatten());
        }

        // Avatar ID metadata-only field (if set) Format: (davatar_id:hp[hash]) - no offset/size/count, just name + provenance hash
        if let Some(hash) = &self.avatar_hash {
            let field = crate::file_format::VsfField::new("avatar_id")
                .with_value(VsfType::hp(hash.to_vec()));
            vsf.push(field.flatten());
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
        let mut prev_file_length = 0;
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

            // Calculate total file length
            let file_length: usize = vsf.iter().map(|chunk| chunk.len()).sum();

            if file_length != prev_file_length {
                vsf[file_length_index] = VsfType::l(file_length, true).flatten();
                prev_file_length = file_length;
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
                    // Get section name, child count, and metadata
                    let (name, child_count, meta): (&String, usize, Option<&SectionMeta>) =
                        if *field_idx < self.sections.len() {
                            let (section, meta) = &self.sections[*field_idx];
                            (&section.name, section.fields.len(), Some(meta))
                        } else {
                            let unboxed_idx = field_idx - self.sections.len();
                            let (name, _, meta) = &self.unboxed[unboxed_idx];
                            (name, 0, Some(meta))
                        };

                    // Rebuild field with updated values using VsfField API
                    let mut field = crate::file_format::VsfField::new(name);
                    // Add key metadata if present (before offset/size/n)
                    if let Some(meta) = meta {
                        if let Some(ref key) = meta.key {
                            field = field.with_value(key.clone());
                        }
                    }
                    field = field
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
        for (_, data, _) in self.unboxed {
            result.extend_from_slice(&data);
        }

        // Now structure is finalized - compute and write crypto primitives Skip if custom values were provided (they're already in the header)

        if self.custom_provenance.is_none() {
            // Auto-compute provenance hash
            use crate::verification::{compute_provenance_hash, write_provenance_hash};
            let prov_hash = compute_provenance_hash(&result)?;
            result = write_provenance_hash(result, &prov_hash)?;
        }

        if self.signature.is_none() && self.include_file_hash {
            // Auto-compute rolling hash (only if no signature)
            use crate::verification::{compute_file_hash, write_file_hash};
            let file_hash = compute_file_hash(&result)?;
            result = write_file_hash(result, &file_hash)?;
        }
        // If signature was provided, it's already written in the header

        Ok(result)
    }

    /// Build the document and Ed25519-sign the named section in one step.
    ///
    /// Convenience over `build()` followed by `verification::sign_section`: the signature replaces the rolling hash as this section's integrity mechanism, additionally proving authorship.
    /// `section_name` must be one of the sections added to this builder.
    #[cfg(feature = "crypto")]
    pub fn build_signed(
        self,
        section_name: &str,
        signing_key: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let bytes = self.build()?;
        crate::verification::sign_section(bytes, section_name, signing_key)
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

        // Verify file structure (instead of counting all '[' bytes which may appear in binary data) Should have magic number
        assert_eq!(&bytes[0..3], "RÅ".as_bytes());

        // Should have header
        assert_eq!(bytes[3], b'<');

        // Should find section names in the output
        let output_str = String::from_utf8_lossy(&bytes);
        assert!(output_str.contains("section1"));
        assert!(output_str.contains("section2"));
    }
}
