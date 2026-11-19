//! RAW video: one VSF document per clip, written while it is recorded (`idt/image.md`, "A RAW clip").
//!
//! A clip is generation 0, then its frames as they came, then a closing generation that indexes them:
//!
//! ```text
//! [ generation 0: visual, track.b0 (with the rate), capture, characterization, clip.b0 (the frames' plane and codec) ]
//! [ frame ][ frame ][ frame ] …   far-form sections named `frame`, back to back, each naming the one before it by hash
//! [ index.b0 ][ generation 1 header: every section of generation 0 and index.b0 ][ trailer ]
//! ```
//!
//! A frame's samples are `v('c')`: the CFA band code ([`super::cfa_band`]), the frame's bands back to back, each band's two bit streams one after the other, their lengths in the frame's `bands`.
//! The frames are not in any table of contents (a minute at 60 a second would be 3600 names); `index.b0` lists them by offset, and each frame reads on its own: its far form gives its name, field count and length.
//!
//! **A clip cut short** (power gone before the closing generation) is read up to its last whole frame without the index: the reader opens generation 0, then walks the frames from its end, each frame's `prev` naming the one before it (the first names generation 0), each frame's `hash` covering its own samples.
//! The first frame that does not parse, names another predecessor (a frame of an earlier recording in the same blocks) or fails its hash ends the clip.
//! [`Reader::repair`] then gives the closing generation to append there, so the clip is whole again.
//!
//! Identities: a band's digest is BLAKE3 of its two streams; a frame's `hash` is BLAKE3 of its bands' digests in order (so each worker that coded a band hashes it while it is in cache); a frame's identity is BLAKE3 of its section bytes before the samples and after them (they hold `hash`, so the identity covers everything), and the next frame's `prev` is it; the first frame's `prev` is BLAKE3 of generation 0's bytes.

use super::{bad, characterization_fields, first, push_text, text, Capture, VisualError};
use crate::container::{Appender, ByteSource, ContainerError, Document};
use crate::file_format::VsfSection;
use crate::prelude::*;
use crate::spectral_image::{self, ColourProfile};
use crate::types::{EtType, Tensor, VsfType};
use crate::vsf_builder::VsfBuilder;

/// VSF's code for the CFA band code in a wrapped value (`v('c')`): a mosaic's samples, lossless ([`super::cfa_band`]).
pub const CODEC_CFA_BAND: u8 = b'c';
/// The codec as `track.b0` and `clip.b0` name it.
pub const CODEC_NAME: &str = "cfa_band";
/// The frames' common description.
pub const CLIP_SECTION: &str = "clip.b0";
/// Every frame's section name.
pub const FRAME_SECTION: &str = "frame";
/// The closing index.
pub const INDEX_SECTION: &str = "index.b0";
/// The field that carries a frame's samples, last in its section.
pub const SAMPLES: &str = "samples";

/// How the band code finds a sample's same-colour neighbours: `bayer` (a 2 x 2 cell, neighbours 2 apart) or `quad` (2 x 2 blocks of one colour in a 4 x 4 cell).
/// Closed: a reader refuses a clip naming another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mosaic {
    Bayer,
    Quad,
}

impl Mosaic {
    pub fn as_str(self) -> &'static str {
        match self {
            Mosaic::Bayer => "bayer",
            Mosaic::Quad => "quad",
        }
    }
    pub fn parse(s: &str) -> Result<Mosaic, VisualError> {
        match s {
            "bayer" => Ok(Mosaic::Bayer),
            "quad" => Ok(Mosaic::Quad),
            other => Err(bad(format!("unknown band mosaic '{other}'"))),
        }
    }
    fn band(self) -> super::cfa_band::Cfa {
        match self {
            Mosaic::Bayer => super::cfa_band::Cfa::Bayer,
            Mosaic::Quad => super::cfa_band::Cfa::Quad,
        }
    }
}

/// What every frame of a clip shares, written once in generation 0.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipSpec {
    /// `visual` `title`: the clip's name as a person reads it.
    pub title: String,
    /// The frame rate asked for, frames a second as a reduced fraction (tukutahi's `rate_num`, `rate_den`); each frame's own `at` is when it was.
    pub rate_num: u32,
    pub rate_den: u32,
    /// The frame period the sensor was set to, in nanoseconds: the longer of 1 / rate and the shutter when the clip started.
    pub period_ns: u64,
    pub width: usize,
    pub height: usize,
    /// Channel index per cell of the colour filter, `[period, period]`, as a still's plane `cfa`.
    pub cfa: Tensor<u8>,
    pub mosaic: Mosaic,
    pub channel_names: Vec<String>,
    /// Per channel, raw counts.
    pub black: Vec<f32>,
    pub white: Vec<f32>,
    /// Bits a sample (10 for RAW10).
    pub depth: u8,
    /// EXIF 274 code, a hint; the samples are in sensor order.
    pub orientation: u16,
    /// Rows a band (the last band of a frame may be shorter).
    pub band_rows: usize,
    pub capture: Capture,
    pub characterization: Option<ColourProfile>,
    pub observer: String,
    /// When the clip started, Eagle oscillations: the document's creation time.
    pub at: Option<i64>,
}

impl ClipSpec {
    /// Bands a frame.
    pub fn bands(&self) -> usize {
        self.height.div_ceil(self.band_rows)
    }

    fn clip_fields(&self) -> Result<Vec<(String, VsfType)>, VisualError> {
        let k = self.channel_names.len();
        if self.black.len() != k || self.white.len() != k {
            return Err(bad(format!("clip: black/white length {}/{} != channel count {k}", self.black.len(), self.white.len())));
        }
        let names = crate::types::Ascii::new(self.channel_names.join("\n")).map_err(|e| bad(format!("channel_names: {e}")))?;
        let mut f = vec![
            ("role".to_string(), VsfType::a(crate::types::Ascii::lit("sensor"))),
            ("width".to_string(), VsfType::u(self.width, false)),
            ("height".to_string(), VsfType::u(self.height, false)),
            ("channel_count".to_string(), VsfType::u(k, false)),
            ("channel_names".to_string(), VsfType::a(names)),
            ("layout".to_string(), VsfType::a(crate::types::Ascii::lit("mosaic"))),
            ("cfa".to_string(), VsfType::t_u3(self.cfa.clone())),
            ("black".to_string(), VsfType::t_f5(Tensor::new(vec![k], self.black.clone()))),
            ("white".to_string(), VsfType::t_f5(Tensor::new(vec![k], self.white.clone()))),
            ("encoding".to_string(), VsfType::a(crate::types::Ascii::lit("linear"))),
            ("depth".to_string(), VsfType::u(self.depth as usize, false)),
            ("codec".to_string(), VsfType::a(crate::types::Ascii::lit(CODEC_NAME))),
            ("codec_format".to_string(), VsfType::u(super::cfa_band::FORMAT as usize, false)),
            ("mosaic".to_string(), VsfType::a(crate::types::Ascii::lit(self.mosaic.as_str()))),
            ("band_rows".to_string(), VsfType::u(self.band_rows, false)),
        ];
        if self.orientation != 1 && self.orientation != 0 {
            f.push(("orientation".to_string(), VsfType::u(self.orientation as usize, false)));
        }
        Ok(f)
    }

    fn track_fields(&self) -> Result<Vec<(String, VsfType)>, VisualError> {
        Ok(vec![
            ("kind".to_string(), VsfType::a(crate::types::Ascii::lit("beam"))),
            ("role".to_string(), VsfType::a(crate::types::Ascii::lit("capture"))),
            ("channels".to_string(), VsfType::u(1, false)),
            ("codec".to_string(), VsfType::a(crate::types::Ascii::lit(CODEC_NAME))),
            ("rate_num".to_string(), VsfType::u(self.rate_num as usize, false)),
            ("rate_den".to_string(), VsfType::u(self.rate_den as usize, false)),
            ("period_ns".to_string(), VsfType::u(self.period_ns as usize, false)),
        ])
    }

    /// Generation 0: a complete VSF document, every section with a content hash in its entry.
    pub fn head(&self) -> Result<Vec<u8>, VisualError> {
        if self.depth == 0 || self.depth > 16 || self.band_rows == 0 || self.band_rows & (self.cfa.shape.first().copied().unwrap_or(2) - 1) != 0 {
            return Err(bad("clip: depth 1 to 16 and band rows a multiple of the CFA period"));
        }
        let mut b = VsfBuilder::new().content_hashes(true);
        if let Some(at) = self.at {
            b = b.creation_time_oscillations(at);
        }
        let mut manifest = vec![("tracks".to_string(), VsfType::a(crate::types::Ascii::lit("b0")))];
        push_text(&mut manifest, "title", &self.title).map_err(bad)?;
        b = b.add_section("visual", manifest);
        b = b.add_section("track.b0", self.track_fields()?);
        let mut capture = self.capture.clone();
        if capture.mode.is_empty() {
            capture.mode = "video".into();
        }
        b = b.add_section("capture", capture.fields().map_err(bad)?);
        if let Some(p) = &self.characterization {
            b = b.add_section("characterization", characterization_fields(p, &self.observer, None).map_err(bad)?);
        }
        b = b.add_section(CLIP_SECTION, self.clip_fields()?);
        b.build().map_err(bad)
    }

    fn from_doc<S: ByteSource>(doc: &Document<S>) -> Result<ClipSpec, VisualError> {
        let section = |name: &str| -> Result<VsfSection, VisualError> {
            let e = doc.entry(name).ok_or_else(|| bad(format!("clip: no section '{name}'")))?;
            Ok(doc.section(e)?)
        };
        let c = section(CLIP_SECTION)?;
        let t = section("track.b0")?;
        let codec = text(&c, "codec");
        if codec != CODEC_NAME {
            return Err(bad(format!("clip: codec '{codec}' is not {CODEC_NAME}")));
        }
        let format = c.uint("codec_format").ok_or(VisualError::Missing("clip codec_format"))?;
        if format != super::cfa_band::FORMAT as u64 {
            return Err(bad(format!("clip: {CODEC_NAME} format {format}, this reader knows {}", super::cfa_band::FORMAT)));
        }
        let k = c.uint("channel_count").ok_or(VisualError::Missing("clip channel_count"))? as usize;
        let names = super::split_n(&text(&c, "channel_names"), k, "channel_names")?;
        let cfa = match first(&c, "cfa") {
            Some(VsfType::t_u3(t)) => t.clone(),
            _ => return Err(VisualError::Missing("clip cfa")),
        };
        let capture = match doc.entry("capture") {
            Some(e) => Capture::from_section(&doc.section(e)?),
            None => Capture::default(),
        };
        let (characterization, observer) = match doc.entry("characterization") {
            Some(e) => {
                let s = doc.section(e)?;
                (Some(spectral_image::profile_from_fields(&s.fields).map_err(|e| bad(e.to_string()))?), text(&s, "observer"))
            }
            None => (None, String::new()),
        };
        let title = match doc.entry("visual") {
            Some(e) => text(&doc.section(e)?, "title"),
            None => String::new(),
        };
        let spec = ClipSpec {
            title,
            rate_num: t.uint("rate_num").ok_or(VisualError::Missing("track rate_num"))? as u32,
            rate_den: t.uint("rate_den").ok_or(VisualError::Missing("track rate_den"))? as u32,
            period_ns: t.uint("period_ns").unwrap_or(0),
            width: c.uint("width").ok_or(VisualError::Missing("clip width"))? as usize,
            height: c.uint("height").ok_or(VisualError::Missing("clip height"))? as usize,
            cfa,
            mosaic: Mosaic::parse(&text(&c, "mosaic"))?,
            channel_names: names,
            black: super::f32_vec(&c, "black").ok_or(VisualError::Missing("clip black"))?,
            white: super::f32_vec(&c, "white").ok_or(VisualError::Missing("clip white"))?,
            depth: c.uint("depth").ok_or(VisualError::Missing("clip depth"))? as u8,
            orientation: c.uint("orientation").map(|o| o as u16).unwrap_or(1),
            band_rows: c.uint("band_rows").ok_or(VisualError::Missing("clip band_rows"))? as usize,
            capture,
            characterization,
            observer,
            at: doc.header().creation_time.as_ref().and_then(|v| match v {
                VsfType::e(EtType::e6(o)) => Some(*o),
                _ => None,
            }),
        };
        // WHY: the geometry is the file's; the decoder's rows are at most cfa_band::MAX_W wide and a band at least one row, or a crafted clip would panic every reader.
        if spec.width < super::cfa_band::MIN_W || spec.width > super::cfa_band::MAX_W || spec.width & 3 != 0 || spec.band_rows == 0 || spec.depth != super::cfa_band::BITS as u8 {
            return Err(bad(format!("clip: {} x {} at {} bits, bands of {} rows: outside what {CODEC_NAME} format {} codes", spec.width, spec.height, spec.depth, spec.band_rows, super::cfa_band::FORMAT)));
        }
        Ok(spec)
    }
}

/// One frame's own facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameMeta {
    /// The sensor's frame count from the clip's first frame (0); a step of more than one is a declared hole.
    pub index: u64,
    /// When its readout started, Eagle oscillations.
    pub at: i64,
    pub exposure_s: f32,
    /// Analogue times digital gain, 1 at the sensor's least.
    pub gain: f32,
    /// The lens actuator's position, the driver's code.
    pub focus: u32,
}

/// BLAKE3 of a band's two streams: what the worker that coded it hashes.
pub fn band_digest(a: &[u8], b: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(a);
    h.update(b);
    *h.finalize().as_bytes()
}

/// A frame's `hash`: BLAKE3 of its bands' digests in order.
pub fn frame_hash(digests: &[[u8; 32]]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    for d in digests {
        h.update(d);
    }
    *h.finalize().as_bytes()
}

fn identity(head: &[u8], tail: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(head);
    h.update(tail);
    *h.finalize().as_bytes()
}

/// A frame's place in its clip.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameEntry {
    pub meta: FrameMeta,
    /// Absolute offset of its section, and the section's bytes.
    pub offset: u64,
    pub size: u64,
    /// Its identity (the next frame's `prev`).
    pub id: [u8; 32],
}

/// The bytes of one frame around its samples: write `head`, the samples (`payload_len` bytes: the bands' streams in order), then `tail`.
#[derive(Debug, Clone)]
pub struct FramePieces {
    pub head: Vec<u8>,
    pub tail: Vec<u8>,
    pub payload_len: usize,
}

fn frame_section(meta: &FrameMeta, prev: &[u8; 32], bands: &[[u32; 2]], hash: &[u8; 32]) -> VsfSection {
    let mut s = VsfSection::new(FRAME_SECTION);
    s.add_field("index", VsfType::u(meta.index as usize, false));
    s.add_field("at", VsfType::e(EtType::e6(meta.at)));
    s.add_field("exposure_s", VsfType::f5(meta.exposure_s));
    s.add_field("gain", VsfType::f5(meta.gain));
    s.add_field("focus", VsfType::u(meta.focus as usize, false));
    s.add_field("prev", VsfType::hb(prev.to_vec()));
    s.add_field("bands", VsfType::t_u5(Tensor::new(vec![bands.len(), 2], bands.iter().flat_map(|b| [b[0], b[1]]).collect())));
    s.add_field("hash", VsfType::hb(hash.to_vec()));
    s
}

/// The clip being written: it holds generation 0 (for the closing generation) and the index so far, never a frame's samples.
pub struct Writer {
    head: Vec<u8>,
    bands: usize,
    end: u64,
    prev: [u8; 32],
    frames: Vec<FrameEntry>,
}

impl Writer {
    /// Start a clip: write `spec.head()` first, then each frame's pieces.
    pub fn new(spec: &ClipSpec) -> Result<(Writer, Vec<u8>), VisualError> {
        let head = spec.head()?;
        let w = Writer { prev: *blake3::hash(&head).as_bytes(), end: head.len() as u64, bands: spec.bands(), frames: Vec::new(), head: head.clone() };
        Ok((w, head))
    }

    /// Bytes written so far: where the next frame goes.
    pub fn end(&self) -> u64 {
        self.end
    }
    pub fn frames(&self) -> &[FrameEntry] {
        &self.frames
    }

    /// The next frame: its bands' stream lengths and digests (one each per band, in order).
    pub fn frame(&mut self, meta: &FrameMeta, bands: &[[u32; 2]], digests: &[[u8; 32]]) -> Result<FramePieces, VisualError> {
        if bands.len() != self.bands || digests.len() != self.bands {
            return Err(bad(format!("clip frame {}: {} bands and {} digests for {}", meta.index, bands.len(), digests.len(), self.bands)));
        }
        if self.frames.last().is_some_and(|f| meta.index <= f.meta.index) {
            return Err(bad(format!("clip frame {} after frame {}", meta.index, self.frames.last().map(|f| f.meta.index).unwrap_or(0))));
        }
        let payload_len = bands.iter().map(|b| b[0] as usize + b[1] as usize).sum::<usize>();
        let s = frame_section(meta, &self.prev, bands, &frame_hash(digests));
        let (head, tail) = s.encode_far_with_tail(SAMPLES, CODEC_CFA_BAND, payload_len);
        let id = identity(&head, &tail);
        let size = (head.len() + payload_len + tail.len()) as u64;
        self.frames.push(FrameEntry { meta: *meta, offset: self.end, size, id });
        self.end += size;
        self.prev = id;
        Ok(FramePieces { head, tail, payload_len })
    }

    /// The closing generation, to write at [`Self::end`]: `index.b0`, a header listing it and generation 0's sections, the trailer.
    /// Write it last; the clip is `end() + closing.len()` bytes.
    pub fn close(self, tool: &str, action: &str) -> Result<Vec<u8>, VisualError> {
        closing(&self.head, &self.frames, self.end, tool, action)
    }
}

fn index_fields(frames: &[FrameEntry], end: u64) -> Vec<(String, VsfType)> {
    let n = frames.len();
    vec![
        ("count".to_string(), VsfType::u(n, false)),
        ("end".to_string(), VsfType::u(end as usize, false)),
        ("offsets".to_string(), VsfType::t_u6(Tensor::new(vec![n], frames.iter().map(|f| f.offset).collect()))),
        ("sizes".to_string(), VsfType::t_u6(Tensor::new(vec![n], frames.iter().map(|f| f.size).collect()))),
        ("ids".to_string(), VsfType::t_u3(Tensor::new(vec![n, 32], frames.iter().flat_map(|f| f.id).collect()))),
        ("index".to_string(), VsfType::t_u6(Tensor::new(vec![n], frames.iter().map(|f| f.meta.index).collect()))),
        ("at".to_string(), VsfType::t_i6(Tensor::new(vec![n], frames.iter().map(|f| f.meta.at).collect()))),
        ("exposure_s".to_string(), VsfType::t_f5(Tensor::new(vec![n], frames.iter().map(|f| f.meta.exposure_s).collect()))),
        ("gain".to_string(), VsfType::t_f5(Tensor::new(vec![n], frames.iter().map(|f| f.meta.gain).collect()))),
        ("focus".to_string(), VsfType::t_u5(Tensor::new(vec![n], frames.iter().map(|f| f.meta.focus).collect()))),
    ]
}

fn closing(head: &[u8], frames: &[FrameEntry], end: u64, tool: &str, action: &str) -> Result<Vec<u8>, VisualError> {
    let doc = Document::open(head)?;
    let tool = crate::types::Ascii::new(tool).map_err(|e| bad(format!("tool: {e}")))?;
    let action = crate::types::Ascii::new(action).map_err(|e| bad(format!("action: {e}")))?;
    Ok(Appender::new(tool, action).add_section(INDEX_SECTION, index_fields(frames, end)).build_at(&doc, end)?)
}

/// A source seen only up to `len`.
struct Prefix<'a, S: ByteSource> {
    source: &'a S,
    len: u64,
}

impl<S: ByteSource> ByteSource for Prefix<'_, S> {
    fn len(&self) -> u64 {
        self.len
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), crate::container::SourceError> {
        // WHY/PROOF: offset and the length come from the file; past `len` is outside this view, and the sum can overflow.
        if offset.checked_add(buf.len() as u64).is_none_or(|e| e > self.len) {
            return Err(crate::container::SourceError::OutOfRange { offset, len: buf.len() as u64, size: self.len });
        }
        self.source.read_at(offset, buf)
    }
    fn slice(&self, offset: u64, len: u64) -> Option<&[u8]> {
        if offset.checked_add(len).is_none_or(|e| e > self.len) {
            return None;
        }
        self.source.slice(offset, len)
    }
}

/// Generation 0 of a clip in `source`, opened over its own bytes alone (a Document over the whole source would walk every frame looking for a torn generation).
fn open_head<S: ByteSource>(source: &S) -> Result<(Vec<u8>, ClipSpec), VisualError> {
    let size = source.len();
    let mut len = size.min(1 << 16);
    loop {
        let view = Prefix { source, len };
        let doc = Document::open(view)?;
        let end = doc.end();
        if end <= len {
            let spec = ClipSpec::from_doc(&doc)?;
            return Ok((source.read_vec(0, end).map_err(ContainerError::from)?, spec));
        }
        // WHY: the header names its own end; a header claiming more than the source holds is a truncated or forged clip.
        if end > size {
            return Err(bad(format!("clip: generation 0 ends at {end}, past the source's {size} bytes")));
        }
        len = end;
    }
}

/// A frame's section read up to its samples: its fields, where its samples are, and its identity.
struct ParsedFrame {
    meta: FrameMeta,
    prev: [u8; 32],
    hash: [u8; 32],
    bands: Vec<[u32; 2]>,
    samples_at: u64,
    samples_len: u64,
    size: u64,
    id: [u8; 32],
}

/// Bytes read to find a frame's fields; a frame's fields before its samples are a few hundred bytes (its bands' lengths are 8 bytes a band).
const FRAME_HEAD_READ: u64 = 1 << 14;

fn parse_frame<S: ByteSource>(source: &S, at: u64, bands_expected: usize) -> Result<ParsedFrame, VisualError> {
    use crate::container::skip::{field_name, skip_value_within, ValueKind};
    let size = source.len();
    let avail = size.saturating_sub(at).min(FRAME_HEAD_READ); // WHY/PROOF: `at` may be the source's end exactly (the walk's last step); saturating keeps an empty read instead of an underflow
    let data = source.read_vec(at, avail).map_err(ContainerError::from)?;
    let perr = |e: crate::decoding::traits::DecodeError| bad(format!("frame at {at}: {e}"));
    let mut p = 0usize;
    if data.first() != Some(&b'[') {
        return Err(bad(format!("frame at {at}: not a section")));
    }
    p += 1;
    let name = match crate::parse(&data, &mut p).map_err(perr)? {
        VsfType::d(s) => s.into_string(),
        _ => return Err(bad(format!("frame at {at}: no name"))),
    };
    if name != FRAME_SECTION {
        return Err(bad(format!("frame at {at}: a '{name}' section")));
    }
    let _count = crate::parse(&data, &mut p).map_err(perr)?;
    let len = match crate::parse(&data, &mut p).map_err(perr)? {
        VsfType::b(l, _) => l as u64,
        _ => return Err(bad(format!("frame at {at}: no length"))),
    };
    // WHY: the length is the file's; a frame running past the source is a torn write, the end of a recovered clip.
    if at.checked_add(len).is_none_or(|e| e > size) {
        return Err(bad(format!("frame at {at}: {len} bytes past the source's end")));
    }
    let mut s = VsfSection::new(FRAME_SECTION);
    let (samples_at, samples_len, head_len) = loop {
        match data.get(p) {
            Some(b'(') => {
                let start = p;
                p += 1;
                let fname = field_name(&data, &mut p).map_err(perr)?;
                if data.get(p) != Some(&b':') {
                    return Err(bad(format!("frame at {at}: field '{fname}' has no value")));
                }
                p += 1;
                if fname == SAMPLES {
                    match skip_value_within(&data, &mut p, len as usize).map_err(perr)? {
                        ValueKind::Wrapped { code: CODEC_CFA_BAND, data_start, data_len } => break (at + data_start as u64, data_len as u64, data_start),
                        _ => return Err(bad(format!("frame at {at}: samples not v('c')"))),
                    }
                }
                p = start;
                let f = crate::file_format::VsfField::parse(&data, &mut p).map_err(|e| bad(format!("frame at {at}: {e}")))?;
                s.fields.push(f);
            }
            _ => return Err(bad(format!("frame at {at}: no samples"))),
        }
    };
    // The samples close the section: `)` then `]`.
    if samples_at + samples_len + 2 != at + len {
        return Err(bad(format!("frame at {at}: samples do not end the section")));
    }
    let tail = source.read_vec(samples_at + samples_len, 2).map_err(ContainerError::from)?;
    if tail != [b')', b']'] {
        return Err(bad(format!("frame at {at}: not closed after its samples")));
    }
    let h32 = |name: &str| first(&s, name).and_then(super::hash32).ok_or_else(|| bad(format!("frame at {at}: no {name}")));
    let bands: Vec<[u32; 2]> = match first(&s, "bands") {
        Some(VsfType::t_u5(t)) if t.shape == [bands_expected, 2] => t.data.chunks(2).map(|c| [c[0], c[1]]).collect(),
        _ => return Err(bad(format!("frame at {at}: bands not [{bands_expected}, 2]"))),
    };
    if bands.iter().map(|b| b[0] as u64 + b[1] as u64).sum::<u64>() != samples_len {
        return Err(bad(format!("frame at {at}: bands do not add up to its samples")));
    }
    let meta = FrameMeta {
        index: s.uint("index").ok_or_else(|| bad(format!("frame at {at}: no index")))?,
        at: super::eagle(&s, "at").ok_or_else(|| bad(format!("frame at {at}: no at")))?,
        exposure_s: super::f32_field(&s, "exposure_s").unwrap_or(0.),
        gain: super::f32_field(&s, "gain").unwrap_or(0.),
        focus: s.uint("focus").unwrap_or(0) as u32,
    };
    Ok(ParsedFrame { meta, prev: h32("prev")?, hash: h32("hash")?, bands, samples_at, samples_len, size: len, id: identity(&data[..head_len], &tail) })
}

/// One frame read back: its facts, its bands' stream lengths and its samples (the code, not yet decoded).
#[derive(Debug, Clone)]
pub struct Frame {
    pub meta: FrameMeta,
    pub bands: Vec<[u32; 2]>,
    pub payload: Vec<u8>,
}

/// A clip open for reading.
pub struct Reader<S: ByteSource> {
    source: S,
    head: Vec<u8>,
    spec: ClipSpec,
    frames: Vec<FrameEntry>,
    end: u64,
    recovered: bool,
}

impl<S: ByteSource> Reader<S> {
    /// Open a clip: by its index when it was closed, else by walking its frames from generation 0 (a clip cut short; [`Self::recovered`] says which).
    /// `source` may be longer than the clip (the blocks it was recorded into): a closed clip is found by its index wherever its frames end only when `source` ends at its trailer, so pass a closed clip's exact length.
    pub fn open(source: S) -> Result<Reader<S>, VisualError> {
        let (head, spec) = open_head(&source)?;
        // Closed: the whole source is the clip, its trailer names the closing header, the header chains back to this generation 0 (an earlier recording's trailer left in the same blocks chains to its own), and index.b0 lists the frames from generation 0's end.
        let closed = {
            let doc = Document::open(&source)?;
            let ours = doc.generation() > 0 && doc.generations().is_ok_and(|g| g.first().is_some_and(|g0| g0.end == head.len() as u64));
            match doc.entry(INDEX_SECTION) {
                Some(e) if ours => Some(doc.section(e)?),
                _ => None,
            }
        };
        if let Some(ix) = closed {
            let frames = read_index(&ix)?;
            let end = ix.uint("end").ok_or(VisualError::Missing("index end"))?;
            // The index's first frame must be the one at generation 0's end, naming it: two recordings with equal generations 0 (equal settings started in the same nanosecond) still differ there.
            let first_ok = match frames.first() {
                Some(f0) => f0.offset == head.len() as u64 && parse_frame(&source, f0.offset, spec.bands()).is_ok_and(|p| p.id == f0.id && p.prev == *blake3::hash(&head).as_bytes()),
                None => end == head.len() as u64,
            };
            if first_ok {
                return Ok(Reader { source, head, spec, frames, end, recovered: false });
            }
        }
        let mut r = Reader { end: head.len() as u64, source, head, spec, frames: Vec::new(), recovered: true };
        r.walk();
        Ok(r)
    }

    /// Follow the chain from generation 0's end; stop at the first frame that does not prove itself.
    fn walk(&mut self) {
        let mut prev = *blake3::hash(&self.head).as_bytes();
        let mut at = self.head.len() as u64;
        let bands = self.spec.bands();
        while let Ok(f) = parse_frame(&self.source, at, bands) {
            if f.prev != prev || self.frames.last().is_some_and(|l| f.meta.index <= l.meta.index) {
                break;
            }
            let Ok(payload) = self.source.read_vec(f.samples_at, f.samples_len) else { break };
            if frame_hash(&digests(&payload, &f.bands)) != f.hash {
                break;
            }
            self.frames.push(FrameEntry { meta: f.meta, offset: at, size: f.size, id: f.id });
            prev = f.id;
            at += f.size;
        }
        self.end = at;
    }

    pub fn spec(&self) -> &ClipSpec {
        &self.spec
    }
    pub fn frames(&self) -> &[FrameEntry] {
        &self.frames
    }
    /// The clip was not closed: its frames were found by their chain.
    pub fn recovered(&self) -> bool {
        self.recovered
    }
    /// Where the frames end: a recovered clip's closing generation goes here.
    pub fn end(&self) -> u64 {
        self.end
    }
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Frame `k`, its samples checked against its `hash` and its place against the index.
    pub fn frame(&self, k: usize) -> Result<Frame, VisualError> {
        let e = &self.frames[k];
        let f = parse_frame(&self.source, e.offset, self.spec.bands())?;
        if f.id != e.id || f.size != e.size {
            return Err(bad(format!("frame {k}: not the frame the index names")));
        }
        let payload = self.source.read_vec(f.samples_at, f.samples_len).map_err(ContainerError::from)?;
        if frame_hash(&digests(&payload, &f.bands)) != f.hash {
            return Err(bad(format!("frame {k}: its samples fail its hash")));
        }
        Ok(Frame { meta: f.meta, bands: f.bands, payload })
    }

    /// Frame `k`'s samples, `width x height` in sensor order.
    pub fn decode(&self, k: usize) -> Result<Vec<u16>, VisualError> {
        let f = self.frame(k)?;
        decode_frame(&self.spec, &f)
    }

    /// The closing generation for a recovered clip, to append at [`Self::end`] (then the clip is `end() + bytes` long and reads by its index).
    pub fn repair(&self, tool: &str) -> Result<Vec<u8>, VisualError> {
        closing(&self.head, &self.frames, self.end, tool, "clip repair")
    }

    /// Frame `k` as a still's sensor plane (track `b0`, seq `clip`), bit-packed, for every reader of planes.
    pub fn frame_plane(&self, k: usize) -> Result<super::Plane, VisualError> {
        let f = self.frame(k)?;
        let px = decode_frame(&self.spec, &f)?;
        let sp = &self.spec;
        let samples = crate::types::BitPackedTensor::pack_u16(sp.depth, vec![sp.height, sp.width], &px);
        let channels = sp.channel_names.iter().map(|n| crate::spectral_image::SpectralChannel { name: n.clone(), curve: None }).collect();
        let mut p = super::Plane::sensor("b0", "clip", sp.width, sp.height, channels, crate::spectral_image::PlaneLayout::Mosaic { cfa: sp.cfa.clone() }, sp.black.clone(), sp.white.clone(), samples);
        p.orientation = sp.orientation;
        p.at = Some(f.meta.at);
        Ok(p)
    }
}

fn digests(payload: &[u8], bands: &[[u32; 2]]) -> Vec<[u8; 32]> {
    let mut at = 0usize;
    bands
        .iter()
        .map(|b| {
            let (la, lb) = (b[0] as usize, b[1] as usize);
            let d = band_digest(&payload[at..at + la], &payload[at + la..at + la + lb]);
            at += la + lb;
            d
        })
        .collect()
}

/// Decode a frame's bands into samples.
pub fn decode_frame(spec: &ClipSpec, f: &Frame) -> Result<Vec<u16>, VisualError> {
    let (w, h, br) = (spec.width, spec.height, spec.band_rows);
    let mut out = vec![0u16; w * h];
    let mut work = alloc::boxed::Box::new(super::cfa_band::Work::new());
    let mut at = 0usize;
    for (k, b) in f.bands.iter().enumerate() {
        let (la, lb) = (b[0] as usize, b[1] as usize);
        let y0 = k * br;
        let rows = br.min(h - y0);
        let used = super::cfa_band::decode_band(&f.payload[at..at + la], &f.payload[at + la..at + la + lb], w, rows, spec.mosaic.band(), &mut work, &mut out[y0 * w..], w);
        // A whole band uses exactly its two streams; anything else is a stream that does not match its lengths.
        if used != (la, lb) {
            return Err(bad(format!("band {k}: used {:?} of streams of {la} and {lb} bytes", used)));
        }
        at += la + lb;
    }
    Ok(out)
}

fn read_index(s: &VsfSection) -> Result<Vec<FrameEntry>, VisualError> {
    let n = s.uint("count").ok_or(VisualError::Missing("index count"))? as usize;
    let u6 = |name: &'static str| -> Result<Vec<u64>, VisualError> {
        match first(s, name) {
            Some(VsfType::t_u6(t)) if t.data.len() == n => Ok(t.data.clone()),
            _ => Err(VisualError::Missing(name)),
        }
    };
    let (offsets, sizes, index) = (u6("offsets")?, u6("sizes")?, u6("index")?);
    let ids = match first(s, "ids") {
        Some(VsfType::t_u3(t)) if t.data.len() == n * 32 => t.data.clone(),
        _ => return Err(VisualError::Missing("index ids")),
    };
    let at = match first(s, "at") {
        Some(VsfType::t_i6(t)) if t.data.len() == n => t.data.clone(),
        _ => return Err(VisualError::Missing("index at")),
    };
    let f5 = |name: &str| super::f32_vec(s, name).filter(|v| v.len() == n).unwrap_or_else(|| vec![0.; n]);
    let (exposure, gain) = (f5("exposure_s"), f5("gain"));
    let focus = super::u32_vec(s, "focus").filter(|v| v.len() == n).unwrap_or_else(|| vec![0; n]);
    Ok((0..n)
        .map(|k| FrameEntry {
            meta: FrameMeta { index: index[k], at: at[k], exposure_s: exposure[k], gain: gain[k], focus: focus[k] },
            offset: offsets[k],
            size: sizes[k],
            id: ids[k * 32..k * 32 + 32].try_into().expect("32-byte chunk"),
        })
        .collect())
}
