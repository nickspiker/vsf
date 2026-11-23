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
//! A clip has one codec, named in `clip.b0`:
//! - `cfa_band`, real-time video: a frame's samples are `v('c')`, the CFA band code ([`super::cfa_band`]), the frame's bands back to back, each band's two bit streams one after the other, their lengths in the frame's `bands`;
//! - `packed`, stacked exposures (a time-lapse, Average, Difference, Motion): a frame's samples are exact sums, a bit-packed `p [height, width]` as wide as that frame needs, with how many exposures it sums and its own `black` and `white`; a clip whose `clip.b0` says `motion` carries a second plane in every frame, `motion`, after the samples, with its own levels.
//!
//! The frames are not in any table of contents (a minute at 60 a second would be 3600 names); `index.b0` lists them by offset, and each frame reads on its own: its far form gives its name, field count and length.
//!
//! **A clip cut short** (power gone before the closing generation) is read up to its last whole frame without the index: the reader opens generation 0, then walks the frames from its end, each frame's `prev` naming the one before it (the first names generation 0), each frame's `hash` covering its own samples.
//! The first frame that does not parse, names another predecessor (a frame of an earlier recording in the same blocks) or fails its hash ends the clip.
//! [`Reader::repair`] then gives the closing generation to append there, so the clip is whole again.
//!
//! Identities: a band's digest is BLAKE3 of its two streams; a `cfa_band` frame's `hash` is BLAKE3 of its bands' digests in order (so each worker that coded a band hashes it while it is in cache), a `packed` frame's BLAKE3 of its samples' packed bytes then its motion plane's; a frame's identity is BLAKE3 of its section bytes other than its payloads (they hold `hash`, so the identity covers everything), and the next frame's `prev` is it; the first frame's `prev` is BLAKE3 of generation 0's bytes.

use super::{bad, characterization_fields, first, push_text, text, Capture, VisualError};
use crate::container::{Appender, ByteSource, ContainerError, Document};
use crate::file_format::VsfSection;
use crate::prelude::*;
use crate::spectral_image::{self, ColourProfile};
use crate::types::{BitPackedTensor, EtType, Tensor, VsfType};
use crate::vsf_builder::VsfBuilder;

/// VSF's code for the CFA band code in a wrapped value (`v('c')`): a mosaic's samples, lossless ([`super::cfa_band`]).
pub const CODEC_CFA_BAND: u8 = b'c';
/// The codec as `track.b0` and `clip.b0` name it.
pub const CODEC_NAME: &str = "cfa_band";
/// The exact sums' codec as `track.b0` and `clip.b0` name it: each frame's planes bit-packed (`p`) at the width they need.
pub const PACKED_NAME: &str = "packed";
/// The field that carries a `packed` frame's motion plane, after its samples.
pub const MOTION: &str = "motion";
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

/// How a clip's frames hold their samples: one codec a clip.
/// Closed: a reader refuses a clip naming another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipCodec {
    /// Real-time video, lossless 10-bit mosaics ([`super::cfa_band`]): how the code finds a sample's same-colour neighbours, and rows a band (the last band of a frame may be shorter).
    CfaBand { mosaic: Mosaic, band_rows: usize },
    /// Stacked exposures: each frame's exact sums bit-packed at the width they need, with their count and levels; `motion` when every frame carries a motion plane beside them.
    Packed { motion: bool },
}

impl ClipCodec {
    pub fn name(self) -> &'static str {
        match self {
            ClipCodec::CfaBand { .. } => CODEC_NAME,
            ClipCodec::Packed { .. } => PACKED_NAME,
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
    pub codec: ClipCodec,
    pub channel_names: Vec<String>,
    /// Per channel, raw counts of one exposure (a `packed` frame's own levels are in its section).
    pub black: Vec<f32>,
    pub white: Vec<f32>,
    /// Bits a sample of one exposure (10 for RAW10; a `packed` frame's own width is its tensor's).
    pub depth: u8,
    /// EXIF 274 code, a hint; the samples are in sensor order.
    pub orientation: u16,
    pub capture: Capture,
    pub characterization: Option<ColourProfile>,
    pub observer: String,
    /// When the clip started, Eagle oscillations: the document's creation time.
    pub at: Option<i64>,
}

impl ClipSpec {
    /// Bands a frame: 0 for a `packed` clip.
    pub fn bands(&self) -> usize {
        match self.codec {
            ClipCodec::CfaBand { band_rows, .. } => self.height.div_ceil(band_rows),
            ClipCodec::Packed { .. } => 0,
        }
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
            ("codec".to_string(), VsfType::a(crate::types::Ascii::lit(self.codec.name()))),
        ];
        match self.codec {
            ClipCodec::CfaBand { mosaic, band_rows } => {
                f.push(("codec_format".to_string(), VsfType::u(super::cfa_band::FORMAT as usize, false)));
                f.push(("mosaic".to_string(), VsfType::a(crate::types::Ascii::lit(mosaic.as_str()))));
                f.push(("band_rows".to_string(), VsfType::u(band_rows, false)));
            }
            ClipCodec::Packed { motion } => f.push((MOTION.to_string(), VsfType::u0(motion))),
        }
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
            ("codec".to_string(), VsfType::a(crate::types::Ascii::lit(self.codec.name()))),
            ("rate_num".to_string(), VsfType::u(self.rate_num as usize, false)),
            ("rate_den".to_string(), VsfType::u(self.rate_den as usize, false)),
            ("period_ns".to_string(), VsfType::u(self.period_ns as usize, false)),
        ])
    }

    /// Generation 0: a complete VSF document, every section with a content hash in its entry.
    pub fn head(&self) -> Result<Vec<u8>, VisualError> {
        let period = self.cfa.shape.first().copied().unwrap_or(2);
        if self.depth == 0 || self.depth > 16 || matches!(self.codec, ClipCodec::CfaBand { band_rows, .. } if band_rows == 0 || band_rows & (period - 1) != 0) {
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
        let codec = match text(&c, "codec").as_str() {
            CODEC_NAME => {
                let format = c.uint("codec_format").ok_or(VisualError::Missing("clip codec_format"))?;
                if format != super::cfa_band::FORMAT as u64 {
                    return Err(bad(format!("clip: {CODEC_NAME} format {format}, this reader knows {}", super::cfa_band::FORMAT)));
                }
                ClipCodec::CfaBand { mosaic: Mosaic::parse(&text(&c, "mosaic"))?, band_rows: c.uint("band_rows").ok_or(VisualError::Missing("clip band_rows"))? as usize }
            }
            PACKED_NAME => ClipCodec::Packed { motion: matches!(first(&c, MOTION), Some(VsfType::u0(true))) },
            other => return Err(bad(format!("clip: codec '{other}' is neither {CODEC_NAME} nor {PACKED_NAME}"))),
        };
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
            codec,
            channel_names: names,
            black: super::f32_vec(&c, "black").ok_or(VisualError::Missing("clip black"))?,
            white: super::f32_vec(&c, "white").ok_or(VisualError::Missing("clip white"))?,
            depth: c.uint("depth").ok_or(VisualError::Missing("clip depth"))? as u8,
            orientation: c.uint("orientation").map(|o| o as u16).unwrap_or(1),
            capture,
            characterization,
            observer,
            at: doc.header().creation_time.as_ref().and_then(|v| match v {
                VsfType::e(EtType::e6(o)) => Some(*o),
                _ => None,
            }),
        };
        // WHY: the geometry is the file's; the decoder's rows are at most cfa_band::MAX_W wide and a band at least one row, or a crafted clip would panic every reader. A packed frame's tensor is checked against the clip's width and height as it is read.
        if let ClipCodec::CfaBand { band_rows, .. } = spec.codec {
            if spec.width < super::cfa_band::MIN_W || spec.width > super::cfa_band::MAX_W || spec.width & 3 != 0 || band_rows == 0 || spec.depth != super::cfa_band::BITS as u8 {
                return Err(bad(format!("clip: {} x {} at {} bits, bands of {} rows: outside what {CODEC_NAME} format {} codes", spec.width, spec.height, spec.depth, band_rows, super::cfa_band::FORMAT)));
            }
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

/// A plane's levels, per channel, in its own scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Levels {
    pub black: Vec<f32>,
    pub white: Vec<f32>,
}

/// What a `packed` frame says about its sums: how many exposures each sample sums, and its planes' levels in their scale (a sum of n exposures: n times one exposure's black and white).
#[derive(Debug, Clone, PartialEq)]
pub struct Sum {
    /// Exposures summed into the frame, 1 for a single one.
    pub exposures: u32,
    /// The samples' levels.
    pub levels: Levels,
    /// The motion plane's levels, in a clip whose frames carry one.
    pub motion: Option<Levels>,
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

/// A `packed` frame's `hash`: BLAKE3 of its samples' packed bytes, then its motion plane's.
pub fn packed_hash(samples: &[u8], motion: Option<&[u8]>) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(samples);
    if let Some(m) = motion {
        h.update(m);
    }
    *h.finalize().as_bytes()
}

/// A frame's identity: BLAKE3 of its section's bytes other than its payloads, in order.
fn identity(pieces: &[&[u8]]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    for p in pieces {
        h.update(p);
    }
    *h.finalize().as_bytes()
}

/// A frame's place in its clip.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameEntry {
    pub meta: FrameMeta,
    /// Exposures the frame sums: 1 in a `cfa_band` clip.
    pub exposures: u32,
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

fn meta_fields(s: &mut VsfSection, meta: &FrameMeta, prev: &[u8; 32]) {
    s.add_field("index", VsfType::u(meta.index as usize, false));
    s.add_field("at", VsfType::e(EtType::e6(meta.at)));
    s.add_field("exposure_s", VsfType::f5(meta.exposure_s));
    s.add_field("gain", VsfType::f5(meta.gain));
    s.add_field("focus", VsfType::u(meta.focus as usize, false));
    s.add_field("prev", VsfType::hb(prev.to_vec()));
}

fn frame_section(meta: &FrameMeta, prev: &[u8; 32], bands: &[[u32; 2]], hash: &[u8; 32]) -> VsfSection {
    let mut s = VsfSection::new(FRAME_SECTION);
    meta_fields(&mut s, meta, prev);
    s.add_field("bands", VsfType::t_u5(Tensor::new(vec![bands.len(), 2], bands.iter().flat_map(|b| [b[0], b[1]]).collect())));
    s.add_field("hash", VsfType::hb(hash.to_vec()));
    s
}

fn levels_fields(s: &mut VsfSection, black: &str, white: &str, l: &Levels) {
    s.add_field(black, VsfType::t_f5(Tensor::new(vec![l.black.len()], l.black.clone())));
    s.add_field(white, VsfType::t_f5(Tensor::new(vec![l.white.len()], l.white.clone())));
}

fn packed_section(meta: &FrameMeta, prev: &[u8; 32], sum: &Sum, hash: &[u8; 32]) -> VsfSection {
    let mut s = VsfSection::new(FRAME_SECTION);
    meta_fields(&mut s, meta, prev);
    s.add_field("exposures", VsfType::u(sum.exposures as usize, false));
    levels_fields(&mut s, "black", "white", &sum.levels);
    if let Some(m) = &sum.motion {
        levels_fields(&mut s, "motion_black", "motion_white", m);
    }
    s.add_field("hash", VsfType::hb(hash.to_vec()));
    s
}

/// The clip being written: it holds generation 0 (for the closing generation) and the index so far, never a frame's samples.
pub struct Writer {
    head: Vec<u8>,
    codec: ClipCodec,
    bands: usize,
    width: usize,
    height: usize,
    channels: usize,
    end: u64,
    prev: [u8; 32],
    frames: Vec<FrameEntry>,
}

impl Writer {
    /// Start a clip: write `spec.head()` first, then each frame's pieces.
    pub fn new(spec: &ClipSpec) -> Result<(Writer, Vec<u8>), VisualError> {
        let head = spec.head()?;
        let w = Writer { prev: *blake3::hash(&head).as_bytes(), end: head.len() as u64, codec: spec.codec, bands: spec.bands(), width: spec.width, height: spec.height, channels: spec.channel_names.len(), frames: Vec::new(), head: head.clone() };
        Ok((w, head))
    }

    /// Bytes written so far: where the next frame goes.
    pub fn end(&self) -> u64 {
        self.end
    }
    pub fn frames(&self) -> &[FrameEntry] {
        &self.frames
    }

    fn after(&self, meta: &FrameMeta) -> Result<(), VisualError> {
        if self.frames.last().is_some_and(|f| meta.index <= f.meta.index) {
            return Err(bad(format!("clip frame {} after frame {}", meta.index, self.frames.last().map(|f| f.meta.index).unwrap_or(0))));
        }
        Ok(())
    }

    fn push(&mut self, meta: &FrameMeta, exposures: u32, size: u64, id: [u8; 32]) {
        self.frames.push(FrameEntry { meta: *meta, exposures, offset: self.end, size, id });
        self.end += size;
        self.prev = id;
    }

    /// The next frame of a `cfa_band` clip: its bands' stream lengths and digests (one each per band, in order).
    pub fn frame(&mut self, meta: &FrameMeta, bands: &[[u32; 2]], digests: &[[u8; 32]]) -> Result<FramePieces, VisualError> {
        if !matches!(self.codec, ClipCodec::CfaBand { .. }) {
            return Err(bad(format!("clip frame {}: a {} clip takes packed frames", meta.index, self.codec.name())));
        }
        if bands.len() != self.bands || digests.len() != self.bands {
            return Err(bad(format!("clip frame {}: {} bands and {} digests for {}", meta.index, bands.len(), digests.len(), self.bands)));
        }
        self.after(meta)?;
        let payload_len = bands.iter().map(|b| b[0] as usize + b[1] as usize).sum::<usize>();
        let s = frame_section(meta, &self.prev, bands, &frame_hash(digests));
        let (head, tail) = s.encode_far_with_tail(SAMPLES, CODEC_CFA_BAND, payload_len);
        let id = identity(&[&head, &tail]);
        self.push(meta, 1, (head.len() + payload_len + tail.len()) as u64, id);
        Ok(FramePieces { head, tail, payload_len })
    }

    /// The next frame of a `packed` clip: its sums `[height, width]` at whatever width they need, and its motion plane when the clip carries one (`sum.motion` its levels).
    /// Returns the pieces around the planes' packed bytes, which the caller writes itself: `pieces[0]`, `samples.data`, `pieces[1]`, and in a motion clip `motion.data`, `pieces[2]`.
    pub fn packed_frame(&mut self, meta: &FrameMeta, sum: &Sum, samples: &BitPackedTensor, motion: Option<&BitPackedTensor>) -> Result<Vec<Vec<u8>>, VisualError> {
        let ClipCodec::Packed { motion: carries } = self.codec else {
            return Err(bad(format!("clip frame {}: a {} clip takes coded bands", meta.index, self.codec.name())));
        };
        if carries != motion.is_some() || carries != sum.motion.is_some() {
            return Err(bad(format!("clip frame {}: a motion plane and its levels in every frame of a motion clip, in none of another", meta.index)));
        }
        let shape = [self.height, self.width];
        if samples.shape != shape || motion.is_some_and(|m| m.shape != shape) {
            return Err(bad(format!("clip frame {}: planes of {:?}, the clip's {}x{} wants [{}, {}]", meta.index, samples.shape, self.width, self.height, self.height, self.width)));
        }
        let k = self.channels;
        if [&sum.levels].into_iter().chain(&sum.motion).any(|l| l.black.len() != k || l.white.len() != k) {
            return Err(bad(format!("clip frame {}: levels for {k} channels", meta.index)));
        }
        self.after(meta)?;
        let hash = packed_hash(&samples.data, motion.map(|m| m.data.as_slice()));
        let s = packed_section(meta, &self.prev, sum, &hash);
        let mut tails = vec![(SAMPLES, samples.bit_depth, samples.shape.as_slice())];
        if let Some(m) = motion {
            tails.push((MOTION, m.bit_depth, m.shape.as_slice()));
        }
        let pieces = s.encode_far_with_packed_tails(&tails);
        let id = identity(&pieces.iter().map(|p| p.as_slice()).collect::<Vec<_>>());
        let size = pieces.iter().map(|p| p.len()).sum::<usize>() + samples.data.len() + motion.map_or(0, |m| m.data.len());
        self.push(meta, sum.exposures, size as u64, id);
        Ok(pieces)
    }

    /// The closing generation, to write at [`Self::end`]: `index.b0`, a header listing it and generation 0's sections, the trailer.
    /// Write it last; the clip is `end() + closing.len()` bytes.
    pub fn close(self, tool: &str, action: &str) -> Result<Vec<u8>, VisualError> {
        closing(&self.head, &self.frames, self.end, tool, action)
    }
}

fn index_fields(frames: &[FrameEntry], end: u64) -> Vec<(String, VsfType)> {
    let n = frames.len();
    let mut f = vec![
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
    ];
    // Absent when every frame is a single exposure.
    if frames.iter().any(|f| f.exposures != 1) {
        f.push(("exposures".to_string(), VsfType::t_u5(Tensor::new(vec![n], frames.iter().map(|f| f.exposures).collect()))));
    }
    f
}

fn closing(head: &[u8], frames: &[FrameEntry], end: u64, tool: &str, action: &str) -> Result<Vec<u8>, VisualError> {
    let doc = Document::open(head)?;
    let tool = crate::types::Ascii::new(tool).map_err(|e| bad(format!("tool: {e}")))?;
    let action = crate::types::Ascii::new(action).map_err(|e| bad(format!("action: {e}")))?;
    Ok(Appender::new(tool, action).add_section(INDEX_SECTION, index_fields(frames, end)).build_at(&doc, end)?)
}

/// A grade of a clip: a generation carrying a graph over the frames' plane (`clip.b0`) with `ops` after the IDT, and its view, as [`super::Visual::still_edit_generation`] gives a still.
/// Append it to the clip (`container::append_to_file`): nothing before it moves, the clip still reads by its index (the new table of contents lists `index.b0` as the last did), and undo is truncation.
pub fn edit_generation<S: ByteSource>(doc: &Document<S>, tool: &str, action: &str, ops: &[crate::spectral_image::ViewOp]) -> Result<Vec<u8>, VisualError> {
    if doc.entry(CLIP_SECTION).is_none() {
        return Err(VisualError::Missing("clip.b0"));
    }
    let vis = super::Visual::read_sections(doc)?;
    let n = doc.generation() + 1;
    let graph = super::Graph::default_still(&format!("g{n}"), CLIP_SECTION, super::Primaries::Vsf, vis.characterization.as_ref().and_then(|p| p.entries.first()).map(|e| e.class), ops);
    let view = super::View::default_of(&format!("v{n}"), &graph);
    let tracks = crate::types::Ascii::new(vis.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join("\n")).map_err(|e| bad(format!("tracks: {e}")))?;
    let default_view = crate::types::Ascii::new(view.section_name()).map_err(|e| bad(format!("default_view: {e}")))?;
    let mut manifest = vec![("tracks".to_string(), VsfType::a(tracks)), ("default_view".to_string(), VsfType::a(default_view))];
    push_text(&mut manifest, "title", &vis.title).map_err(bad)?;
    push_text(&mut manifest, "text", &vis.text).map_err(bad)?;
    let tool = crate::types::Ascii::new(tool).map_err(|e| bad(format!("tool: {e}")))?;
    let action = crate::types::Ascii::new(action).map_err(|e| bad(format!("action: {e}")))?;
    let mut a = Appender::new(tool, action).add_section("visual", manifest).add_section(graph.section_name(), graph.fields()).add_section(view.section_name(), view.fields());
    for e in doc.body_entries() {
        if e.name.starts_with("graph.") || e.name.starts_with("view.") {
            a = a.supersede(e.name.clone());
        }
    }
    Ok(a.build(doc)?)
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
        // A prefix holding the whole of a closed clip (one under 2^16 bytes) opens its latest generation: generation 0 is the first of its chain, opened alone.
        if doc.generation() > 0 {
            len = doc.generations()?.first().map_or(len, |g| g.end);
            continue;
        }
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

/// Where a frame's payloads are, as its section says.
enum Payloads {
    /// A `cfa_band` frame: its bands' stream lengths, its samples' place and length.
    Bands { bands: Vec<[u32; 2]>, at: u64, len: u64 },
    /// A `packed` frame: its sums, then each plane's bits a sample, place and length.
    Packed { sum: Sum, samples: (u8, u64, u64), motion: Option<(u8, u64, u64)> },
}

/// A frame's section read up to its samples: its fields, where its payloads are, and its identity.
struct ParsedFrame {
    meta: FrameMeta,
    prev: [u8; 32],
    hash: [u8; 32],
    payloads: Payloads,
    size: u64,
    id: [u8; 32],
}

impl ParsedFrame {
    fn exposures(&self) -> u32 {
        match &self.payloads {
            Payloads::Bands { .. } => 1,
            Payloads::Packed { sum, .. } => sum.exposures,
        }
    }
}

/// Bytes read to find a frame's fields; a frame's fields before its samples are a few hundred bytes (its bands' lengths are 8 bytes a band).
const FRAME_HEAD_READ: u64 = 1 << 14;
/// Bytes read after a packed frame's samples: their closing `)`, and a motion plane's field up to its packed bytes.
const MOTION_HEAD_READ: u64 = 1 << 8;

fn levels_of(s: &VsfSection, black: &str, white: &str, k: usize) -> Option<Levels> {
    let (black, white) = (super::f32_vec(s, black)?, super::f32_vec(s, white)?);
    (black.len() == k && white.len() == k).then_some(Levels { black, white })
}

fn parse_frame<S: ByteSource>(source: &S, at: u64, spec: &ClipSpec) -> Result<ParsedFrame, VisualError> {
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
    let shape = [spec.height, spec.width];
    let mut s = VsfSection::new(FRAME_SECTION);
    // The samples' value: a cfa_band clip's `v('c')`, a packed clip's `p` of the clip's shape.
    let (samples_at, samples_len, bits, head_len) = loop {
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
                    match (skip_value_within(&data, &mut p, len as usize).map_err(perr)?, spec.codec) {
                        (ValueKind::Wrapped { code: CODEC_CFA_BAND, data_start, data_len }, ClipCodec::CfaBand { .. }) => break (at + data_start as u64, data_len as u64, 0, data_start),
                        (ValueKind::Packed { bit_depth, shape: sh, data_start, data_len }, ClipCodec::Packed { .. }) if sh == shape => break (at + data_start as u64, data_len as u64, bit_depth, data_start),
                        _ => return Err(bad(format!("frame at {at}: samples not those of a {} clip of {}x{}", spec.codec.name(), spec.width, spec.height))),
                    }
                }
                p = start;
                let f = crate::file_format::VsfField::parse(&data, &mut p).map_err(|e| bad(format!("frame at {at}: {e}")))?;
                s.fields.push(f);
            }
            _ => return Err(bad(format!("frame at {at}: no samples"))),
        }
    };
    let end = at + len;
    let h32 = |name: &str| first(&s, name).and_then(super::hash32).ok_or_else(|| bad(format!("frame at {at}: no {name}")));
    let (prev, hash) = (h32("prev")?, h32("hash")?);
    let meta = FrameMeta {
        index: s.uint("index").ok_or_else(|| bad(format!("frame at {at}: no index")))?,
        at: super::eagle(&s, "at").ok_or_else(|| bad(format!("frame at {at}: no at")))?,
        exposure_s: super::f32_field(&s, "exposure_s").unwrap_or(0.),
        gain: super::f32_field(&s, "gain").unwrap_or(0.),
        focus: s.uint("focus").unwrap_or(0) as u32,
    };
    let samples_end = samples_at + samples_len;
    let (payloads, between) = match spec.codec {
        ClipCodec::CfaBand { .. } => {
            let bands: Vec<[u32; 2]> = match first(&s, "bands") {
                Some(VsfType::t_u5(t)) if t.shape == [spec.bands(), 2] => t.data.chunks(2).map(|c| [c[0], c[1]]).collect(),
                _ => return Err(bad(format!("frame at {at}: bands not [{}, 2]", spec.bands()))),
            };
            if bands.iter().map(|b| b[0] as u64 + b[1] as u64).sum::<u64>() != samples_len {
                return Err(bad(format!("frame at {at}: bands do not add up to its samples")));
            }
            (Payloads::Bands { bands, at: samples_at, len: samples_len }, Vec::new())
        }
        ClipCodec::Packed { motion: carries } => {
            let k = spec.channel_names.len();
            let sum = Sum {
                exposures: s.uint("exposures").ok_or_else(|| bad(format!("frame at {at}: no exposures")))? as u32,
                levels: levels_of(&s, "black", "white", k).ok_or_else(|| bad(format!("frame at {at}: no black and white for {k} channels")))?,
                motion: if carries { Some(levels_of(&s, "motion_black", "motion_white", k).ok_or_else(|| bad(format!("frame at {at}: no motion_black and motion_white for {k} channels")))?) } else { None },
            };
            let mut motion = None;
            let mut between = Vec::new();
            if carries {
                // The motion plane's field follows the samples: `)`, then `(motion:p…`, its head read here.
                // WHY: the read must stay inside the section (the source may end at it); skip_value_within held the samples inside it, so the subtraction cannot wrap.
                let avail = (end - samples_end).min(MOTION_HEAD_READ);
                let w = source.read_vec(samples_end, avail).map_err(ContainerError::from)?;
                let mut q = 0usize;
                if w.get(q) != Some(&b')') || w.get(q + 1) != Some(&b'(') {
                    return Err(bad(format!("frame at {at}: no motion plane after its samples")));
                }
                q += 2;
                if field_name(&w, &mut q).map_err(perr)? != MOTION || w.get(q) != Some(&b':') {
                    return Err(bad(format!("frame at {at}: no motion plane after its samples")));
                }
                q += 1;
                match skip_value_within(&w, &mut q, (end - samples_end) as usize).map_err(perr)? {
                    ValueKind::Packed { bit_depth, shape: sh, data_start, data_len } if sh == shape => {
                        between = w[..data_start].to_vec();
                        motion = Some((bit_depth, samples_end + data_start as u64, data_len as u64));
                    }
                    _ => return Err(bad(format!("frame at {at}: motion plane not p [{}, {}]", spec.height, spec.width))),
                }
            }
            (Payloads::Packed { sum, samples: (bits, samples_at, samples_len), motion }, between)
        }
    };
    // The last payload closes the section: `)` then `]`.
    let last_end = match &payloads {
        Payloads::Packed { motion: Some((_, m, l)), .. } => m + l,
        _ => samples_end,
    };
    if last_end + 2 != end {
        return Err(bad(format!("frame at {at}: its payloads do not end the section")));
    }
    let tail = source.read_vec(last_end, 2).map_err(ContainerError::from)?;
    if tail != [b')', b']'] {
        return Err(bad(format!("frame at {at}: not closed after its payloads")));
    }
    let id = identity(&[&data[..head_len], &between, &tail]);
    Ok(ParsedFrame { meta, prev, hash, payloads, size: len, id })
}

/// A frame's samples as read back.
#[derive(Debug, Clone)]
pub enum FrameSamples {
    /// A `cfa_band` frame: its bands' stream lengths and their streams, the code not yet decoded.
    Bands { bands: Vec<[u32; 2]>, payload: Vec<u8> },
    /// A `packed` frame: its sums' count and levels, its samples, and its motion plane in a motion clip.
    Packed { sum: Sum, samples: BitPackedTensor, motion: Option<BitPackedTensor> },
}

/// One frame read back: its facts and its samples, checked against its `hash`.
#[derive(Debug, Clone)]
pub struct Frame {
    pub meta: FrameMeta,
    pub samples: FrameSamples,
}

/// Read a frame's payloads and check them against its `hash`.
fn read_payloads<S: ByteSource>(source: &S, f: ParsedFrame, spec: &ClipSpec) -> Result<Frame, VisualError> {
    let samples = match f.payloads {
        Payloads::Bands { bands, at, len } => {
            let payload = source.read_vec(at, len).map_err(ContainerError::from)?;
            if frame_hash(&digests(&payload, &bands)) != f.hash {
                return Err(bad(format!("frame {}: its samples fail its hash", f.meta.index)));
            }
            FrameSamples::Bands { bands, payload }
        }
        Payloads::Packed { sum, samples: (bits, at, len), motion } => {
            let data = source.read_vec(at, len).map_err(ContainerError::from)?;
            let motion = match motion {
                Some((mb, mat, mlen)) => Some((mb, source.read_vec(mat, mlen).map_err(ContainerError::from)?)),
                None => None,
            };
            if packed_hash(&data, motion.as_ref().map(|m| m.1.as_slice())) != f.hash {
                return Err(bad(format!("frame {}: its samples fail its hash", f.meta.index)));
            }
            FrameSamples::Packed { sum, samples: BitPackedTensor { bit_depth: bits, shape: vec![spec.height, spec.width], data }, motion: motion.map(|(mb, data)| BitPackedTensor { bit_depth: mb, shape: vec![spec.height, spec.width], data }) }
        }
    };
    Ok(Frame { meta: f.meta, samples })
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
        Self::open_with(source, true)
    }

    /// Open a clip as [`Self::open`] does, but a clip cut short is walked by its frames' headers alone (each frame's fields, `prev` and closing bytes, a few blocks a frame), its samples not read: for a recovery that must not read and hash every byte of a long clip (2,525 frames, 18 GB, took 89 s on the Pro).
    /// Nothing about the samples is proven yet: check the clip's last frames with [`Self::check_tail`] (or each with [`Self::frame`]) before trusting where it ends.
    pub fn open_headers(source: S) -> Result<Reader<S>, VisualError> {
        Self::open_with(source, false)
    }

    /// [`Self::open_headers`] then [`Self::check_tail`]: a clip cut short ends before the earliest frame found failing its hash, with `n` frames right before that end proven.
    pub fn open_tail_checked(source: S, n: usize) -> Result<Reader<S>, VisualError> {
        let mut r = Self::open_headers(source)?;
        if r.recovered {
            r.check_tail(n);
        }
        Ok(r)
    }

    /// Check the frames' samples from the last one back: a frame failing its hash ends the clip before it (with every frame after it), until `n` frames in a row before the end pass; the frames found before them are taken as the chain lists them.
    /// Why the tail is enough: a recorder writes its frames in order, one after another, so a frame's header on the flash comes after every byte before it; a write cut short tears the last frames, never one in the middle. The `n` frames proven cover writes the flash took out of order before power went.
    /// Returns the frames checked.
    pub fn check_tail(&mut self, n: usize) -> usize {
        let mut cut = self.frames.len();
        let (mut good, mut checked, mut k) = (0, 0, self.frames.len());
        while k > 0 && good < n {
            k -= 1;
            checked += 1;
            if self.frame(k).is_ok() {
                good += 1;
            } else {
                cut = k;
                good = 0;
            }
        }
        self.truncate(cut);
        checked
    }

    /// Keep the first `k` frames only (a recovery that found frame `k` torn): the clip then ends where frame `k` began.
    pub fn truncate(&mut self, k: usize) {
        if k < self.frames.len() {
            self.end = self.frames[k].offset;
            self.frames.truncate(k);
        }
    }

    fn open_with(source: S, samples: bool) -> Result<Reader<S>, VisualError> {
        let (head, spec) = open_head(&source)?;
        // Closed: the whole source is the clip, its trailer names the closing header, the header chains back to this generation 0 (an earlier recording's trailer left in the same blocks chains to its own), and index.b0 lists the frames from generation 0's end.
        // Only a source that ends in a trailer is opened whole: without one, `Document::open` would look for a torn generation by reading every byte past generation 0 (a whole recording region, tens of GB, when a clip's writer died), where the frames' own chain is the way in.
        let trailer = {
            let size = source.len();
            size >= crate::container::TRAILER_LEN as u64 && source.read_vec(size - crate::container::TRAILER_LEN as u64, crate::container::TRAILER_LEN as u64).is_ok_and(|t| crate::container::append::parse_trailer(&t).is_some())
        };
        let closed = if !trailer {
            None
        } else {
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
                Some(f0) => f0.offset == head.len() as u64 && parse_frame(&source, f0.offset, &spec).is_ok_and(|p| p.id == f0.id && p.prev == *blake3::hash(&head).as_bytes()),
                None => end == head.len() as u64,
            };
            if first_ok {
                return Ok(Reader { source, head, spec, frames, end, recovered: false });
            }
        }
        let mut r = Reader { end: head.len() as u64, source, head, spec, frames: Vec::new(), recovered: true };
        r.walk(samples);
        Ok(r)
    }

    /// Follow the chain from generation 0's end; stop at the first frame that does not prove itself (with `samples` false, by its header and place in the chain alone).
    fn walk(&mut self, samples: bool) {
        let mut prev = *blake3::hash(&self.head).as_bytes();
        let mut at = self.head.len() as u64;
        while let Ok(f) = parse_frame(&self.source, at, &self.spec) {
            if f.prev != prev || self.frames.last().is_some_and(|l| f.meta.index <= l.meta.index) {
                break;
            }
            let (meta, exposures, size, id) = (f.meta, f.exposures(), f.size, f.id);
            if samples && read_payloads(&self.source, f, &self.spec).is_err() {
                break;
            }
            self.frames.push(FrameEntry { meta, exposures, offset: at, size, id });
            prev = id;
            at += size;
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

    /// The clip's document as a visual, from its latest generation: the characterization (one pasted after the recording included), the capture facts, the title and the views a grade appended ([`edit_generation`]).
    /// It has no planes: the frames are this reader's.
    pub fn visual(&self) -> Result<super::Visual, VisualError> {
        let doc = Document::open(&self.source)?;
        super::Visual::read_sections(&doc)
    }

    /// Frame `k`, its samples checked against its `hash` and its place against the index.
    pub fn frame(&self, k: usize) -> Result<Frame, VisualError> {
        let e = &self.frames[k];
        let f = parse_frame(&self.source, e.offset, &self.spec)?;
        if f.id != e.id || f.size != e.size {
            return Err(bad(format!("frame {k}: not the frame the index names")));
        }
        read_payloads(&self.source, f, &self.spec)
    }

    /// Frame `k`'s samples of a `cfa_band` clip, `width x height` in sensor order.
    pub fn decode(&self, k: usize) -> Result<Vec<u16>, VisualError> {
        let f = self.frame(k)?;
        decode_frame(&self.spec, &f)
    }

    /// Frame `k` of a `packed` clip: its sums' count and levels, its samples `[height, width]`, and its motion plane in a motion clip.
    pub fn packed(&self, k: usize) -> Result<(Sum, BitPackedTensor, Option<BitPackedTensor>), VisualError> {
        match self.frame(k)?.samples {
            FrameSamples::Packed { sum, samples, motion } => Ok((sum, samples, motion)),
            FrameSamples::Bands { .. } => Err(bad(format!("frame {k}: a {CODEC_NAME} frame, not packed"))),
        }
    }

    /// The closing generation for a recovered clip, to append at [`Self::end`] (then the clip is `end() + bytes` long and reads by its index).
    pub fn repair(&self, tool: &str) -> Result<Vec<u8>, VisualError> {
        closing(&self.head, &self.frames, self.end, tool, "clip repair")
    }

    /// Frame `k` as a still's sensor plane (track `b0`, seq `clip`), bit-packed, for every reader of planes; a `packed` frame's at its own width and levels.
    pub fn frame_plane(&self, k: usize) -> Result<super::Plane, VisualError> {
        Ok(self.frame_planes(k)?.0)
    }

    /// Frame `k` as a still's planes: its sensor plane, and a `packed` motion clip's motion plane beside it (role `motion`, as a still's).
    pub fn frame_planes(&self, k: usize) -> Result<(super::Plane, Option<super::Plane>), VisualError> {
        let f = self.frame(k)?;
        let sp = &self.spec;
        let channels: Vec<crate::spectral_image::SpectralChannel> = sp.channel_names.iter().map(|n| crate::spectral_image::SpectralChannel { name: n.clone(), curve: None }).collect();
        let layout = crate::spectral_image::PlaneLayout::Mosaic { cfa: sp.cfa.clone() };
        let plane = |samples: BitPackedTensor, l: Levels| {
            let mut p = super::Plane::sensor("b0", "clip", sp.width, sp.height, channels.clone(), layout.clone(), l.black, l.white, samples);
            p.orientation = sp.orientation;
            p.at = Some(f.meta.at);
            p
        };
        match &f.samples {
            FrameSamples::Bands { .. } => {
                let px = decode_frame(sp, &f)?;
                Ok((plane(BitPackedTensor::pack_u16(sp.depth, vec![sp.height, sp.width], &px), Levels { black: sp.black.clone(), white: sp.white.clone() }), None))
            }
            FrameSamples::Packed { sum, samples, motion } => {
                let motion = match (motion, &sum.motion) {
                    (Some(m), Some(l)) => {
                        let mut p = plane(m.clone(), l.clone());
                        p.name = MOTION.into();
                        p.role = super::PlaneRole::Motion;
                        Some(p)
                    }
                    _ => None,
                };
                Ok((plane(samples.clone(), sum.levels.clone()), motion))
            }
        }
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

/// Decode a `cfa_band` frame's bands into samples.
pub fn decode_frame(spec: &ClipSpec, f: &Frame) -> Result<Vec<u16>, VisualError> {
    let (ClipCodec::CfaBand { mosaic, band_rows: br }, FrameSamples::Bands { bands, payload }) = (spec.codec, &f.samples) else {
        return Err(bad(format!("frame {}: not a {CODEC_NAME} frame", f.meta.index)));
    };
    let (w, h) = (spec.width, spec.height);
    let mut out = vec![0u16; w * h];
    let mut work = alloc::boxed::Box::new(super::cfa_band::Work::new());
    let mut at = 0usize;
    for (k, b) in bands.iter().enumerate() {
        let (la, lb) = (b[0] as usize, b[1] as usize);
        let y0 = k * br;
        let rows = br.min(h - y0);
        let used = super::cfa_band::decode_band(&payload[at..at + la], &payload[at + la..at + la + lb], w, rows, mosaic.band(), &mut work, &mut out[y0 * w..], w);
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
    let u5 = |name: &str, or: u32| super::u32_vec(s, name).filter(|v| v.len() == n).unwrap_or_else(|| vec![or; n]);
    let (focus, exposures) = (u5("focus", 0), u5("exposures", 1));
    Ok((0..n)
        .map(|k| FrameEntry {
            meta: FrameMeta { index: index[k], at: at[k], exposure_s: exposure[k], gain: gain[k], focus: focus[k] },
            exposures: exposures[k],
            offset: offsets[k],
            size: sizes[k],
            id: ids[k * 32..k * 32 + 32].try_into().expect("32-byte chunk"),
        })
        .collect())
}
