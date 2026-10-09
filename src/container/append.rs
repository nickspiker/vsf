//! Append-only generations: edit a document by adding to its end, never by rewriting a byte of it.
//!
//! EWE integers are minimal-width on read (`crate::ewe`), so no pointer in an existing header can be patched in place once it needs another byte. An edit therefore appends a **generation**: the new section bodies (far form — self-describing, readable where they land), then a complete new header whose table of contents lists every live section by absolute offset (old ones where they always were, new ones just written), then a fixed 16-byte trailer naming that header. Readers find the latest header from the trailer; a reader that predates generations reads the gen-0 header at offset 0 and sees the original capture, hashes intact, edits invisible.
//!
//! ```text
//! [ gen-0 document, exactly as VsfBuilder::build wrote it … ]
//! [ gen-1 bodies (far form) ][ gen-1 header: live TOC + (gen)(start)(prev)(tool)(action) ][ trailer → gen-1 header ]
//! [ gen-2 bodies ][ gen-2 header … (prev → gen-1 header) ][ trailer ]
//! ```
//!
//! A generation header carries its own `hp` (BLAKE3 of its bodies and header with crypto fields zeroed) and `prev`: the previous header's offset and the BLAKE3 of its bytes — the history chain. Undo is truncation to the previous generation's end, which is byte-exact because nothing earlier was ever touched ([`crate::container::Document::checkout`]). A crash mid-append leaves a trailer that does not resolve; the reader falls back to the last header it can prove and reports the orphaned tail.

use crate::container::{ByteSource, ContainerError, Document};
use crate::encoding::hash_placeholder;
use crate::file_format::{HeaderField, VsfField, VsfSection};
use crate::prelude::*;
use crate::types::VsfType;
use crate::vsf_builder::SectionMeta;

/// The last 16 bytes of a document with generations: `">\xC3\x85R\0VSF\n"` then the latest header's offset, big-endian u64 — fixed width on purpose, because the format's own integers are not.
pub const TRAILER_LEN: usize = 16;
pub const TRAILER_MAGIC: [u8; 8] = [b'>', 0xC3, 0x85, b'R', 0, b'V', b'S', b'F'];

/// Encode the trailer naming a header at `header_offset`.
pub fn trailer(header_offset: u64) -> [u8; TRAILER_LEN] {
    let mut t = [0u8; TRAILER_LEN];
    t[..8].copy_from_slice(&TRAILER_MAGIC);
    t[8..].copy_from_slice(&header_offset.to_be_bytes());
    t
}

/// The header offset a trailer names, if `tail` is one.
pub fn parse_trailer(tail: &[u8]) -> Option<u64> {
    if tail.len() != TRAILER_LEN || tail[..8] != TRAILER_MAGIC {
        return None;
    }
    Some(u64::from_be_bytes(tail[8..].try_into().ok()?))
}

/// Inline header-field names a generation header carries.
pub const FIELD_GEN: &str = "gen";
pub const FIELD_START: &str = "start";
pub const FIELD_PREV: &str = "prev";
pub const FIELD_TOOL: &str = "tool";
pub const FIELD_ACTION: &str = "action";

/// Build the bytes that turn a document into its next generation. Everything the document has stays live unless superseded or re-added under the same name; the new sections and blobs are appended with content hashes in their entries.
pub struct Appender {
    sections: Vec<(VsfSection, SectionMeta)>,
    unboxed: Vec<(String, Vec<u8>, SectionMeta)>,
    supersede: Vec<String>,
    tool: String,
    action: String,
    creation_time: Option<VsfType>,
}

impl Appender {
    /// `tool` names the writer, `action` what it did (free text, e.g. "opsin", "paste idt") — both ride in the generation header.
    pub fn new(tool: impl Into<String>, action: impl Into<String>) -> Self {
        Self { sections: Vec::new(), unboxed: Vec::new(), supersede: Vec::new(), tool: tool.into(), action: action.into(), creation_time: None }
    }

    pub fn creation_time_oscillations(mut self, oscillations: i64) -> Self {
        self.creation_time = Some(VsfType::e(crate::types::EtType::e6(oscillations)));
        self
    }

    pub fn add_section(mut self, name: impl Into<String>, fields: Vec<(String, VsfType)>) -> Self {
        let mut section = VsfSection::new(name);
        for (n, v) in fields {
            section.add_field(n, v);
        }
        self.sections.push((section, SectionMeta::default()));
        self
    }

    pub fn add_section_direct(mut self, section: VsfSection) -> Self {
        self.sections.push((section, SectionMeta::default()));
        self
    }

    pub fn add_section_direct_with_meta(mut self, section: VsfSection, meta: SectionMeta) -> Self {
        self.sections.push((section, meta));
        self
    }

    pub fn add_unboxed(mut self, name: impl Into<String>, data: Vec<u8>) -> Self {
        self.unboxed.push((name.into(), data, SectionMeta::default()));
        self
    }

    /// Drop a live section from the new table of contents. Its bytes stay — a tombstone in the TOC, never a deletion — and every earlier generation still lists it.
    pub fn supersede(mut self, name: impl Into<String>) -> Self {
        self.supersede.push(name.into());
        self
    }

    /// The bytes to append after `doc`'s current end: bodies, header, trailer. The caller writes them (see [`append_to_file`]) — bodies first, header, then the trailer, each synced, so a crash never leaves a trailer naming a header that is not there.
    pub fn build<S: ByteSource>(self, doc: &Document<S>) -> Result<Vec<u8>, ContainerError> {
        let base = doc.source().len();
        // The previous header, by offset and by hash — the link in the chain.
        let prev_offset = doc.header_offset();
        let prev_bytes = doc.source().read_vec(prev_offset, doc.header_len())?;
        let prev_hash = *blake3::hash(&prev_bytes).as_bytes();
        let gen = doc.generation() + 1;

        // New bodies, far form, laid end to end from `base`.
        let mut bodies: Vec<u8> = Vec::new();
        let mut new_entries: Vec<HeaderField> = Vec::new();
        let mut names_added: Vec<String> = Vec::new();
        for (section, meta) in &self.sections {
            let bytes = section.encode_far();
            let off = base + bodies.len() as u64;
            let hash = *blake3::hash(&bytes).as_bytes();
            new_entries.push(HeaderField {
                name: section.name.clone(),
                hash: Some(VsfType::hb(hash.to_vec())),
                signature: None,
                key: meta.key.clone(),
                offset_bytes: off as usize,
                size_bytes: bytes.len(),
                child_count: section.fields.len(),
                inline_values: Vec::new(),
            });
            names_added.push(section.name.clone());
            bodies.extend_from_slice(&bytes);
        }
        for (name, data, meta) in &self.unboxed {
            let off = base + bodies.len() as u64;
            let hash = *blake3::hash(data).as_bytes();
            new_entries.push(HeaderField { name: name.clone(), hash: Some(VsfType::hb(hash.to_vec())), signature: None, key: meta.key.clone(), offset_bytes: off as usize, size_bytes: data.len(), child_count: 0, inline_values: Vec::new() });
            names_added.push(name.clone());
            bodies.extend_from_slice(data);
        }

        // The live table of contents: what the document has, minus tombstones and re-added names, with every offset made absolute; then the new entries. Per-generation inline fields are not carried; other inline fields (state) are.
        let per_gen = [FIELD_GEN, FIELD_START, FIELD_PREV, FIELD_TOOL, FIELD_ACTION];
        let mut carried: Vec<HeaderField> = Vec::new();
        for e in doc.entries() {
            let is_inline = e.size_bytes == 0 && e.child_count == 0 && e.offset_bytes == 0 && (!e.inline_values.is_empty() || e.hash.is_some());
            if is_inline && per_gen.contains(&e.name.as_str()) {
                continue;
            }
            if self.supersede.contains(&e.name) || names_added.contains(&e.name) {
                continue;
            }
            let mut c = e.clone();
            if !is_inline {
                let (off, _) = doc.span(e);
                c.offset_bytes = off as usize;
            }
            carried.push(c);
        }

        // Header: the builder's byte order, with placeholders for hp and hb, stabilised on its own length and the file length.
        let header_offset = base + bodies.len() as u64;
        let mut header = Vec::new();
        for guess in 0..8u32 {
            let _ = guess;
            let total = header_offset + header.len() as u64 + TRAILER_LEN as u64;
            let candidate = encode_header(self.creation_time.as_ref(), gen, base, prev_offset, &prev_hash, &self.tool, &self.action, &carried, &new_entries, header.len(), total as usize);
            if candidate.len() == header.len() {
                header = candidate;
                break;
            }
            header = candidate;
        }
        // Content identity of this generation: its bodies and its header with the crypto fields zeroed (they are zero now).
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"vsf/gen/v1");
        hasher.update(&bodies);
        hasher.update(&header);
        let hp = *hasher.finalize().as_bytes();
        crate::verification::fill_provenance_hash(&mut header, &hp).map_err(ContainerError::Header)?;
        // Rolling hash: the header as it stands with hp filled and hb zero.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&bodies);
        hasher.update(&header);
        let hb = *hasher.finalize().as_bytes();
        header = crate::verification::write_file_hash(header, &hb).map_err(ContainerError::Header)?;

        let mut out = bodies;
        out.extend_from_slice(&header);
        out.extend_from_slice(&trailer(header_offset));
        Ok(out)
    }
}

/// One generation header, the builder's byte order. `header_len`/`file_len` are the values to encode for `b` and `l`; the caller iterates until the encoded length stops changing.
#[allow(clippy::too_many_arguments)]
fn encode_header(creation_time: Option<&VsfType>, gen: u32, start: u64, prev_offset: u64, prev_hash: &[u8; 32], tool: &str, action: &str, carried: &[HeaderField], new_entries: &[HeaderField], header_len: usize, file_len: usize) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice("RÅ".as_bytes());
    h.push(b'<');
    h.extend_from_slice(&VsfType::z(crate::VSF_VERSION).flatten());
    h.extend_from_slice(&VsfType::y(crate::VSF_BACKWARD_COMPAT).flatten());
    h.extend_from_slice(&VsfType::b(header_len, true).flatten());
    h.extend_from_slice(&VsfType::l(file_len, true).flatten());
    if let Some(e) = creation_time {
        h.extend_from_slice(&e.flatten());
    }
    h.extend_from_slice(&hash_placeholder(b'p', 32));
    h.extend_from_slice(&hash_placeholder(b'b', 32));
    let inline = [
        VsfField::new(FIELD_GEN).with_value(VsfType::u(gen as usize, false)),
        VsfField::new(FIELD_START).with_value(VsfType::u(start as usize, false)),
        VsfField::new(FIELD_PREV).with_value(VsfType::u(prev_offset as usize, false)).with_value(VsfType::hb(prev_hash.to_vec())),
        VsfField::new(FIELD_TOOL).with_value(VsfType::a(tool.to_string())),
        VsfField::new(FIELD_ACTION).with_value(VsfType::a(action.to_string())),
    ];
    h.extend_from_slice(&VsfType::n(inline.len() + carried.len() + new_entries.len()).flatten());
    for f in &inline {
        h.extend_from_slice(&f.flatten());
    }
    for e in carried.iter().chain(new_entries) {
        h.extend_from_slice(&entry_field(e).flatten());
    }
    h.push(b'>');
    h
}

/// A table-of-contents entry as the builder writes one: hash, key, offset, size, children — or, for an inline field, hash then its values.
fn entry_field(e: &HeaderField) -> VsfField {
    let mut f = VsfField::new(&e.name);
    if let Some(h) = &e.hash {
        f = f.with_value(h.clone());
    }
    let is_inline = e.size_bytes == 0 && e.child_count == 0 && e.offset_bytes == 0 && (!e.inline_values.is_empty() || e.hash.is_some());
    if is_inline {
        for v in &e.inline_values {
            f = f.with_value(v.clone());
        }
        return f;
    }
    if let Some(k) = &e.key {
        f = f.with_value(k.clone());
    }
    if e.size_bytes == 0 && e.child_count == 0 {
        // Header-only section: a name and nothing else.
        return f;
    }
    f.with_value(VsfType::o(e.offset_bytes)).with_value(VsfType::b(e.size_bytes, false)).with_value(VsfType::n(e.child_count))
}

/// Append generation bytes to a file in the order that keeps every intermediate state readable: bodies, sync, header, sync, trailer, sync. A crash before the trailer leaves the previous generation current with an orphaned tail the reader reports.
#[cfg(feature = "std")]
pub fn append_to_file(path: &std::path::Path, bytes: &[u8]) -> Result<(), ContainerError> {
    use std::io::Write;
    let io = |e: std::io::Error| ContainerError::Source(crate::container::SourceError::Io(e.to_string()));
    let mut f = std::fs::OpenOptions::new().append(true).open(path).map_err(io)?;
    if bytes.len() < TRAILER_LEN {
        return Err(ContainerError::Parse("generation bytes shorter than a trailer".into()));
    }
    let (body, tail) = bytes.split_at(bytes.len() - TRAILER_LEN);
    f.write_all(body).map_err(io)?;
    f.sync_data().map_err(io)?;
    f.write_all(tail).map_err(io)?;
    f.sync_data().map_err(io)?;
    Ok(())
}

/// Truncate a file to `len` bytes — the undo of an append, byte-exact because nothing before `len` was ever written by a later generation.
#[cfg(feature = "std")]
pub fn truncate_file(path: &std::path::Path, len: u64) -> Result<(), ContainerError> {
    let io = |e: std::io::Error| ContainerError::Source(crate::container::SourceError::Io(e.to_string()));
    let f = std::fs::OpenOptions::new().write(true).open(path).map_err(io)?;
    f.set_len(len).map_err(io)?;
    f.sync_data().map_err(io)?;
    Ok(())
}
