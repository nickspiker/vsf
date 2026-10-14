//! Lazy access to a VSF document: the header first, then exactly the bytes a caller asks for — and, when the document has been appended to, its generations.
//!
//! [`crate::VsfHeader::decode`] and [`crate::VsfSection::parse`] want the whole document in memory and copy every value they meet, which is right for a 2 KB message and wrong for a 50-megapixel plane or a 4 GB recording. A [`Document`] opens a [`ByteSource`] — a slice, a `Vec`, a `File` — by reading its header (and nothing past it), hands out the table of contents, and fetches a section's bytes only when asked. For the bulk values, the bit-packed planes and wrapped payloads, [`Document::packed`] and [`Document::wrapped`] locate the payload without copying it, and [`Document::packed_rows_u16`] reads just the rows a viewer needs.
//!
//! A document edited by appending (see [`append`]) ends in a trailer naming its latest header; `open` follows it, and [`Document::generations`] walks the chain back to the original. Offsets in a table of contents are measured from the start of the file, so a section is found by its entry wherever it was written, never by scanning.

pub mod append;
pub mod skip;
pub mod source;

pub use append::{Appender, TRAILER_LEN};
#[cfg(feature = "std")]
pub use append::{append_to_file, truncate_file};
pub use skip::{FieldSpan, ValueKind, ValueSpan};
pub use source::{ByteSource, SourceError};

use crate::file_format::{HeaderField, VsfHeader, VsfSection};
use crate::prelude::*;
use crate::types::packed_view::BitPackedView;
use crate::types::VsfType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    Source(SourceError),
    /// The header did not decode.
    Header(String),
    /// A section or value did not parse.
    Parse(String),
    /// The table of contents names bytes the source does not have.
    Truncated { entry: String, offset: u64, len: u64, size: u64 },
    /// No entry by that name.
    Missing(String),
    /// The generation chain does not verify: a `prev` link's hash does not match the header it names.
    Chain { generation: u32, header_offset: u64 },
}

impl core::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ContainerError::Source(e) => write!(f, "{e}"),
            ContainerError::Header(s) => write!(f, "header: {s}"),
            ContainerError::Parse(s) => write!(f, "parse: {s}"),
            ContainerError::Truncated { entry, offset, len, size } => write!(f, "entry '{entry}' claims {len} bytes at {offset} but the source is {size} bytes"),
            ContainerError::Missing(s) => write!(f, "no entry named '{s}'"),
            ContainerError::Chain { generation, header_offset } => write!(f, "generation {generation}: the header at {header_offset} does not match the hash its successor recorded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ContainerError {}

impl From<SourceError> for ContainerError {
    fn from(e: SourceError) -> Self {
        ContainerError::Source(e)
    }
}

/// A located bit-packed tensor: header decoded, samples still in the source at `data_offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedSpan {
    pub bit_depth: u8,
    pub shape: Vec<usize>,
    /// Absolute offset of the first sample byte.
    pub data_offset: u64,
    pub data_len: u64,
}

/// A located wrapped payload (`v`): its code and the bytes, still in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedSpan {
    pub code: u8,
    pub data_offset: u64,
    pub data_len: u64,
}

/// One generation of a document: where its bytes are and what wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    pub index: u32,
    /// First byte this generation wrote (0 for the original).
    pub start: u64,
    /// One past its last byte — the length to truncate to for its state.
    pub end: u64,
    pub header_offset: u64,
    pub header_len: u64,
    pub tool: String,
    pub action: String,
}

/// A document open on a source. Holds the decoded header of its latest generation; everything else is fetched on demand.
pub struct Document<S: ByteSource> {
    source: S,
    header: VsfHeader,
    header_offset: u64,
    header_len: u64,
    header_end: u64,
    generation: u32,
    orphan: u64,
}

fn inline_u64(h: &VsfHeader, name: &str) -> Option<u64> {
    use crate::schema::FromVsfType;
    h.fields.iter().find(|f| f.name == name).and_then(|f| f.inline_values.first()).and_then(|v| u64::from_vsf_type(v).ok())
}

fn inline_text(h: &VsfHeader, name: &str) -> String {
    h.fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| f.inline_values.first())
        .and_then(|v| match v {
            VsfType::a(s) | VsfType::x(s) | VsfType::d(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn inline_hash(h: &VsfHeader, name: &str) -> Option<[u8; 32]> {
    h.fields.iter().find(|f| f.name == name).and_then(|f| match &f.hash {
        Some(VsfType::hb(b)) | Some(VsfType::hp(b)) | Some(VsfType::hs(b)) if b.len() == 32 => b.as_slice().try_into().ok(),
        _ => None,
    })
}

impl<S: ByteSource> Document<S> {
    /// Open by reading the header only. A document with generations ends in a trailer naming its latest header; that header is read if it checks out (its `l` must equal the source length), else the original header at offset 0 is read and the unresolved tail reported as orphaned — a torn append never hides the last good state.
    pub fn open(source: S) -> Result<Self, ContainerError> {
        let size = source.len();
        if size >= (TRAILER_LEN as u64) + 8 {
            let tail = source.read_vec(size - TRAILER_LEN as u64, TRAILER_LEN as u64)?;
            if let Some(off) = append::parse_trailer(&tail) {
                if off + 8 < size {
                    if let Ok((header, hlen, hend)) = read_header_at(&source, off) {
                        if header.file_length as u64 == size {
                            let generation = inline_u64(&header, append::FIELD_GEN).unwrap_or(0) as u32;
                            return Ok(Self { source, header, header_offset: off, header_len: hlen, header_end: hend, generation, orphan: 0 });
                        }
                    }
                }
            }
        }
        let (header, hlen, hend) = read_header_at(&source, 0)?;
        let mut end = gen0_end(&header, hend);
        let mut doc = Self { source, header, header_offset: 0, header_len: hlen, header_end: hend, generation: 0, orphan: 0 };
        // No usable trailer, yet bytes past the original: a torn append. Every complete generation before the tear left its header and trailer in place, so walk forward from the last known end to the furthest one that proves itself.
        while end != 0 && size > end {
            match find_generation_after(&doc.source, end, size)? {
                Some((header, off, hlen, hend)) => {
                    end = header.file_length as u64;
                    doc.generation = inline_u64(&header, append::FIELD_GEN).unwrap_or(0) as u32;
                    doc.header = header;
                    doc.header_offset = off;
                    doc.header_len = hlen;
                    doc.header_end = hend;
                }
                None => break,
            }
        }
        doc.orphan = if end != 0 && size > end { size - end } else { 0 };
        Ok(doc)
    }

    pub fn source(&self) -> &S {
        &self.source
    }
    pub fn header(&self) -> &VsfHeader {
        &self.header
    }
    /// Where the current header starts (0 unless the document has generations).
    pub fn header_offset(&self) -> u64 {
        self.header_offset
    }
    pub fn header_len(&self) -> u64 {
        self.header_len
    }
    /// Where the current header ends — the first section of a legacy document whose entries carry no offset starts here.
    pub fn header_end(&self) -> u64 {
        self.header_end
    }
    /// The current generation index: 0 for a document never appended to.
    pub fn generation(&self) -> u32 {
        self.generation
    }
    /// Bytes past the last header that could be proven: a torn append, or foreign bytes. 0 for a whole document.
    pub fn orphaned_bytes(&self) -> u64 {
        self.orphan
    }
    /// The length of the current generation's state: `l` of the current header (a generation header's `l` is exact; an original's is cross-checked against its last entry, because builders before 2026-10-09 wrote `l` without their unboxed blobs).
    pub fn end(&self) -> u64 {
        if self.generation == 0 {
            gen0_end(&self.header, self.header_end)
        } else {
            self.header.file_length as u64
        }
    }

    /// Every table-of-contents entry, in order.
    pub fn entries(&self) -> &[HeaderField] {
        &self.header.fields
    }
    /// The first entry named `name`.
    pub fn entry(&self, name: &str) -> Option<&HeaderField> {
        self.header.fields.iter().find(|e| e.name == name)
    }
    /// Every entry named `name`, in order — repeated same-name sections are first-class.
    pub fn entries_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a HeaderField> + 'a {
        self.header.fields.iter().filter(move |e| e.name == name)
    }
    /// Entries that point at a body (sections and blobs), excluding header-inline fields.
    pub fn body_entries(&self) -> impl Iterator<Item = &HeaderField> + '_ {
        self.header.fields.iter().filter(|e| !(e.size_bytes == 0 && e.child_count == 0 && e.offset_bytes == 0 && (!e.inline_values.is_empty() || e.hash.is_some())))
    }

    /// The absolute byte range an entry's body occupies: `(offset, len)`. A header-only entry (no body) is `(0, 0)`.
    pub fn span(&self, entry: &HeaderField) -> (u64, u64) {
        if entry.size_bytes == 0 && entry.child_count == 0 {
            return (0, 0);
        }
        let off = if entry.offset_bytes != 0 { entry.offset_bytes as u64 } else { self.header_end };
        (off, entry.size_bytes as u64)
    }

    fn checked_span(&self, entry: &HeaderField) -> Result<(u64, u64), ContainerError> {
        let (off, len) = self.span(entry);
        let size = self.source.len();
        if off.checked_add(len).map_or(true, |e| e > size) {
            return Err(ContainerError::Truncated { entry: entry.name.clone(), offset: off, len, size });
        }
        Ok((off, len))
    }

    /// An entry's body bytes, verbatim.
    pub fn raw(&self, entry: &HeaderField) -> Result<Vec<u8>, ContainerError> {
        let (off, len) = self.checked_span(entry)?;
        Ok(self.source.read_vec(off, len)?)
    }

    /// An entry's body, parsed — every value materialised, as [`VsfSection::parse`] does. Header-only entries come back as zero-field sections. Near-form bodies take their name from the entry.
    pub fn section(&self, entry: &HeaderField) -> Result<VsfSection, ContainerError> {
        let (off, len) = self.checked_span(entry)?;
        if len == 0 {
            return Ok(VsfSection::new(entry.name.clone()));
        }
        let bytes = self.source.read_vec(off, len)?;
        let mut ptr = 0;
        let mut s = VsfSection::parse(&bytes, &mut ptr).map_err(ContainerError::Parse)?;
        if s.name.is_empty() {
            s.name = entry.name.clone();
        }
        Ok(s)
    }

    /// The entry's fields with every value's absolute span, bulk payloads located but not copied.
    pub fn fields(&self, entry: &HeaderField) -> Result<Vec<FieldSpan>, ContainerError> {
        let (off, len) = self.checked_span(entry)?;
        if len == 0 {
            return Ok(Vec::new());
        }
        let bytes = self.source.read_vec(off, len)?;
        let mut ptr = 0;
        let (_, mut fields) = skip::skip_section(&bytes, &mut ptr).map_err(|e| ContainerError::Parse(e.to_string()))?;
        let base = off as usize;
        for f in &mut fields {
            for v in &mut f.values {
                v.start += base;
                v.end += base;
                match &mut v.kind {
                    ValueKind::Packed { data_start, .. } | ValueKind::Wrapped { data_start, .. } => *data_start += base,
                    ValueKind::Other(_) => {}
                }
            }
        }
        Ok(fields)
    }

    /// The first bit-packed value of field `name` in `entry`, located — `None` when the field is absent or not packed.
    pub fn packed(&self, entry: &HeaderField, name: &str) -> Result<Option<PackedSpan>, ContainerError> {
        for f in self.fields(entry)? {
            if f.name != name {
                continue;
            }
            if let Some(ValueSpan { kind: ValueKind::Packed { bit_depth, shape, data_start, data_len }, .. }) = f.values.first() {
                return Ok(Some(PackedSpan { bit_depth: *bit_depth, shape: shape.clone(), data_offset: *data_start as u64, data_len: *data_len as u64 }));
            }
            return Ok(None);
        }
        Ok(None)
    }

    /// The first wrapped value of field `name` in `entry`, located.
    pub fn wrapped(&self, entry: &HeaderField, name: &str) -> Result<Option<WrappedSpan>, ContainerError> {
        for f in self.fields(entry)? {
            if f.name != name {
                continue;
            }
            if let Some(ValueSpan { kind: ValueKind::Wrapped { code, data_start, data_len }, .. }) = f.values.first() {
                return Ok(Some(WrappedSpan { code: *code, data_offset: *data_start as u64, data_len: *data_len as u64 }));
            }
            return Ok(None);
        }
        Ok(None)
    }

    /// Rows `y0..y1` (slabs along the first dimension) of a located plane, reading only the bytes they occupy. Over an in-memory source nothing is copied but the output.
    pub fn packed_rows_u16(&self, span: &PackedSpan, y0: usize, y1: usize) -> Result<Vec<u16>, ContainerError> {
        let per: usize = span.shape.get(1..).unwrap_or(&[]).iter().product();
        if y1 < y0 {
            return Err(ContainerError::Parse(format!("rows {y0}..{y1} reversed")));
        }
        self.packed_range_u16(span, y0 * per, (y1 - y0) * per)
    }

    /// Samples `first..first+count` of a located plane, reading only the bytes they occupy.
    pub fn packed_range_u16(&self, span: &PackedSpan, first: usize, count: usize) -> Result<Vec<u16>, ContainerError> {
        let bits = span.bit_depth as usize;
        let n: usize = span.shape.iter().product();
        let end = first.checked_add(count).filter(|&e| e <= n).ok_or_else(|| ContainerError::Parse(format!("samples {first}..{first}+{count} outside 0..{n}")))?;
        let b0 = first * bits;
        let b1 = end * bits;
        let byte0 = (b0 >> 3) as u64;
        let byte1 = b1.div_ceil(8) as u64;
        if byte1 > span.data_len {
            return Err(ContainerError::Truncated { entry: "plane".into(), offset: span.data_offset, len: span.data_len, size: self.source.len() });
        }
        let intra = b0 & 7;
        let shape = [count];
        if let Some(s) = self.source.slice(span.data_offset + byte0, byte1 - byte0) {
            let v = BitPackedView::with_bit_base(s, intra, span.bit_depth, &shape).map_err(ContainerError::Parse)?;
            return v.unpack_range_u16(0, count).map_err(ContainerError::Parse);
        }
        let bytes = self.source.read_vec(span.data_offset + byte0, byte1 - byte0)?;
        let v = BitPackedView::with_bit_base(&bytes, intra, span.bit_depth, &shape).map_err(ContainerError::Parse)?;
        v.unpack_range_u16(0, count).map_err(ContainerError::Parse)
    }

    /// Whether an entry's body matches the content hash its table-of-contents entry carries: `None` when the entry carries none, `Some(bool)` otherwise.
    pub fn content_hash_ok(&self, entry: &HeaderField) -> Result<Option<bool>, ContainerError> {
        let expected = match &entry.hash {
            Some(VsfType::hb(h)) | Some(VsfType::hs(h)) | Some(VsfType::hp(h)) if h.len() == 32 => h.clone(),
            _ => return Ok(None),
        };
        let bytes = self.raw(entry)?;
        Ok(Some(blake3::hash(&bytes).as_bytes()[..] == expected[..]))
    }

    /// Every generation, oldest first, by walking `prev` links from the current header back to the original — each link's hash checked against the header it names, so a forged or torn chain fails here.
    pub fn generations(&self) -> Result<Vec<Generation>, ContainerError> {
        let mut out = Vec::new();
        let mut header = self.header.clone();
        let mut offset = self.header_offset;
        let mut hlen = self.header_len;
        loop {
            let index = inline_u64(&header, append::FIELD_GEN).unwrap_or(0) as u32;
            let start = if index == 0 { 0 } else { inline_u64(&header, append::FIELD_START).unwrap_or(0) };
            let end = if index == 0 { gen0_end(&header, offset + hlen) } else { header.file_length as u64 };
            out.push(Generation { index, start, end, header_offset: offset, header_len: hlen, tool: inline_text(&header, append::FIELD_TOOL), action: inline_text(&header, append::FIELD_ACTION) });
            if index == 0 {
                break;
            }
            let prev_off = inline_u64(&header, append::FIELD_PREV).ok_or(ContainerError::Chain { generation: index, header_offset: offset })?;
            let prev_hash = inline_hash(&header, append::FIELD_PREV).ok_or(ContainerError::Chain { generation: index, header_offset: offset })?;
            let (prev_header, prev_len, _) = read_header_at(&self.source, prev_off).map_err(|_| ContainerError::Chain { generation: index - 1, header_offset: prev_off })?;
            let bytes = self.source.read_vec(prev_off, prev_len)?;
            if blake3::hash(&bytes).as_bytes()[..] != prev_hash[..] {
                return Err(ContainerError::Chain { generation: index - 1, header_offset: prev_off });
            }
            header = prev_header;
            offset = prev_off;
            hlen = prev_len;
            if out.len() > 1_000_000 {
                return Err(ContainerError::Chain { generation: index, header_offset: offset });
            }
        }
        out.reverse();
        Ok(out)
    }

    /// The document exactly as it stood at generation `n`: its first `end` bytes — byte for byte, because no later generation wrote inside them.
    pub fn checkout(&self, n: u32) -> Result<Vec<u8>, ContainerError> {
        let gens = self.generations()?;
        let g = gens.iter().find(|g| g.index == n).ok_or(ContainerError::Missing(format!("generation {n}")))?;
        Ok(self.source.read_vec(0, g.end)?)
    }

    /// The length to truncate to for undoing the latest generation — `None` for the original.
    pub fn undo_len(&self) -> Result<Option<u64>, ContainerError> {
        if self.generation == 0 {
            return Ok(None);
        }
        let gens = self.generations()?;
        Ok(gens.iter().rev().nth(1).map(|g| g.end))
    }

    /// Whether a generation's content identity (`hp`) matches its bytes. Generation 0 is hashed the way its builder hashed it ([`crate::verification::compute_provenance_hash`] over the original document); a later generation is `blake3("vsf/gen/v1" ‖ bodies ‖ header)` with `hp` and `hb` zeroed, as [`append::Appender::build`] computed it. `None` when the header carries no 32-byte `hp`.
    pub fn generation_hash_ok(&self, g: &Generation) -> Result<Option<bool>, ContainerError> {
        let (header, _, _) = read_header_at(&self.source, g.header_offset)?;
        let stored = match &header.provenance_hash {
            VsfType::hp(h) if h.len() == 32 => h.clone(),
            _ => return Ok(None),
        };
        if g.index == 0 {
            // Builders before 2026-10-09 hashed what they wrote, which may stop at `l` before an unboxed blob: try both the declared length and the furthest byte.
            let mut ends = vec![g.end];
            if (header.file_length as u64) != g.end {
                ends.push(header.file_length as u64);
            }
            for end in ends {
                let bytes = self.source.read_vec(0, end)?;
                if let Ok(h) = crate::verification::compute_provenance_hash(&bytes) {
                    if h[..] == stored[..] {
                        return Ok(Some(true));
                    }
                }
            }
            return Ok(Some(false));
        }
        let bodies = self.source.read_vec(g.start, g.header_offset - g.start)?;
        let mut head = self.source.read_vec(g.header_offset, g.header_len)?;
        crate::verification::fill_provenance_hash(&mut head, &[0u8; 32]).map_err(ContainerError::Header)?;
        let head = crate::verification::write_file_hash(head, &[0u8; 32]).map_err(ContainerError::Header)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"vsf/gen/v1");
        hasher.update(&bodies);
        hasher.update(&head);
        Ok(Some(hasher.finalize().as_bytes()[..] == stored[..]))
    }
}

/// Where an original (generation-0) document ends: its `l`, or the end of its furthest entry if that lies past `l` — builders before 2026-10-09 wrote `l` without their unboxed blobs, and such files are in the wild.
fn gen0_end(header: &VsfHeader, header_end: u64) -> u64 {
    let declared = header.file_length as u64;
    let furthest = header
        .fields
        .iter()
        .filter(|e| !(e.size_bytes == 0 && e.child_count == 0))
        .map(|e| {
            let off = if e.offset_bytes != 0 { e.offset_bytes as u64 } else { header_end };
            off + e.size_bytes as u64
        })
        .max()
        .unwrap_or(0);
    declared.max(furthest)
}

/// The first complete generation whose bodies begin at `from`: a header past `from` whose `start` is `from`, whose `l` is the end of its own trailer within `size`, and whose trailer names it. Bodies are scanned for the header magic in windows; a false magic inside a body fails one of those checks and is passed over.
fn find_generation_after<S: ByteSource>(source: &S, from: u64, size: u64) -> Result<Option<(VsfHeader, u64, u64, u64)>, ContainerError> {
    const WINDOW: u64 = 1 << 20;
    let magic = "RÅ<".as_bytes();
    let mut at = from;
    while at < size {
        let len = (size - at).min(WINDOW + magic.len() as u64 - 1);
        let chunk = source.read_vec(at, len)?;
        for i in 0..chunk.len().saturating_sub(magic.len() - 1) {
            if &chunk[i..i + magic.len()] != magic {
                continue;
            }
            let off = at + i as u64;
            let Ok((header, hlen, hend)) = read_header_at(source, off) else { continue };
            if inline_u64(&header, append::FIELD_START) != Some(from) {
                continue;
            }
            let trailer_at = off + hlen;
            if header.file_length as u64 != trailer_at + TRAILER_LEN as u64 || header.file_length as u64 > size {
                continue;
            }
            let tail = source.read_vec(trailer_at, TRAILER_LEN as u64)?;
            if append::parse_trailer(&tail) != Some(off) {
                continue;
            }
            return Ok(Some((header, off, hlen, hend)));
        }
        at += WINDOW;
    }
    Ok(None)
}

/// Read and decode the header at `offset`: `(header, header_len, header_end)`. The header's own `b` says how long it is, so exactly that many bytes are read.
fn read_header_at<S: ByteSource>(source: &S, offset: u64) -> Result<(VsfHeader, u64, u64), ContainerError> {
    let size = source.len();
    if offset >= size {
        return Err(ContainerError::Header(format!("header offset {offset} outside a {size}-byte source")));
    }
    let probe_len = (size - offset).min(64);
    let probe = source.read_vec(offset, probe_len)?;
    if probe.len() < 4 || &probe[0..3] != "RÅ".as_bytes() || probe[3] != b'<' {
        return Err(ContainerError::Header("not a VSF document (no RÅ< magic)".into()));
    }
    let mut ptr = 4;
    let z = crate::parse(&probe, &mut ptr).map_err(|e| ContainerError::Header(format!("version: {e}")))?;
    if !matches!(z, VsfType::z(_)) {
        return Err(ContainerError::Header("expected the version (z) after the magic".into()));
    }
    let y = crate::parse(&probe, &mut ptr).map_err(|e| ContainerError::Header(format!("backward_compat: {e}")))?;
    if !matches!(y, VsfType::y(_)) {
        return Err(ContainerError::Header("expected backward_compat (y)".into()));
    }
    let header_len = match crate::parse(&probe, &mut ptr).map_err(|e| ContainerError::Header(format!("header length: {e}")))? {
        VsfType::b(len, _) => len as u64,
        other => return Err(ContainerError::Header(format!("expected the header length (b), found {other:?}"))),
    };
    if header_len < 8 || offset + header_len > size {
        return Err(ContainerError::Header(format!("header length {header_len} at {offset} is impossible for a {size}-byte source")));
    }
    let header_bytes = source.read_vec(offset, header_len)?;
    let (header, consumed) = VsfHeader::decode(&header_bytes).map_err(ContainerError::Header)?;
    Ok((header, header_len, offset + consumed as u64))
}
