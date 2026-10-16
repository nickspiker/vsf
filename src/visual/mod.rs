//! VSF visual data — one container for stills, video, audio, layers and provenance. This module is the still-image slice of it (the plan's M1): tracks, spans, planes, the capture facts, the characterization, a graph and a view, and the foreign metadata a conversion must not lose.
//!
//! A still is the one-frame case of a beam track: `visual` + `track.b0` + `span.b0.s0` (one frame, naming a plane) + `plane.b0.s0.sensor` + `capture` + `characterization` + `graph.g0` + `view.v0`. Video adds a rate declaration and spans whose frames are inline; nothing about a still changes when it does. Every section is written with a content hash in its table-of-contents entry, so each can be checked — and later signed — on its own.
//!
//! The normative field tables live in `idt/image.md`; this module's doc comments are the implementation's own account. What a reader does with absence: no `characterization` ⇒ the samples are VSF RGB; no `graph` ⇒ render the sensor plane through `black_white → idt(0)`; a `plane` with no `encoding` ⇒ `gamma2` (the integer-plane rule of `colour.md`) — which is why every writer here emits `encoding`, and why the legacy `spectral_image` section reads as `linear`: its only writer ever was opsin, writing linear counts.
//!
//! [`Visual::to_spectral_image`] / [`Visual::from_spectral_image`] bridge to the [`crate::spectral_image::SpectralImage`] in-memory model opsin renders from, so a consumer can adopt the container without rewriting its pipeline.

use crate::container::{ByteSource, ContainerError, Document};
use crate::file_format::VsfSection;
use crate::prelude::*;
use crate::spectral_image::{self, ColourProfile, IdtClass, PlaneLayout, Provenance, SpectralChannel, SpectralCurve, SpectralImage, ViewOp, ViewTransform};
use crate::types::{BitPackedTensor, EtType, Tensor, VsfType, WorldCoord};
use crate::vsf_builder::VsfBuilder;

#[derive(Debug)]
pub enum VisualError {
    Container(ContainerError),
    Missing(&'static str),
    Bad(String),
}

impl core::fmt::Display for VisualError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VisualError::Container(e) => write!(f, "{e}"),
            VisualError::Missing(s) => write!(f, "missing '{s}'"),
            VisualError::Bad(s) => write!(f, "{s}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VisualError {}

impl From<ContainerError> for VisualError {
    fn from(e: ContainerError) -> Self {
        VisualError::Container(e)
    }
}

fn bad(s: impl Into<String>) -> VisualError {
    VisualError::Bad(s.into())
}

/// Closed vocabularies — a reader fails loud on a value it does not know.
macro_rules! closed_vocab {
    ($name:ident { $($variant:ident => $s:literal),+ $(,)? }, $what:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }
            pub fn parse(s: &str) -> Result<Self, VisualError> {
                match s { $($s => Ok($name::$variant),)+ other => Err(bad(format!("unknown {} '{}'", $what, other))) }
            }
        }
    };
}

closed_vocab!(TrackKind { Beam => "beam", Wave => "wave", Timecode => "timecode", Annotation => "annotation", Data => "data" }, "track kind");
closed_vocab!(TrackRole { Capture => "capture", Derived => "derived", Display => "display", Preview => "preview", Calibration => "calibration" }, "track role");
closed_vocab!(PlaneRole { Sensor => "sensor", Display => "display", Preview => "preview", Depth => "depth", Ir => "ir", Uv => "uv", Confidence => "confidence", Motion => "motion", Matte => "matte", Mask => "mask", Alpha => "alpha", Dark => "dark", Flat => "flat", Bias => "bias", Derived => "derived" }, "plane role");
closed_vocab!(Encoding { Linear => "linear", Gamma2 => "gamma2", Gamma2Tanh => "gamma2tanh" }, "encoding");
closed_vocab!(Purpose { Default => "default", Preview => "preview", Print => "print", Technical => "technical" }, "view purpose");

/// A track: one stream of frames (beam), samples (wave), marks (timecode), notes (annotation) or sensor data (data).
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// `b0`, `w0`, … — a letter first, because a section name cannot start with a digit.
    pub id: String,
    pub kind: TrackKind,
    pub role: TrackRole,
    pub title: String,
    /// Wave channel count; beam plane count.
    pub channels: usize,
    /// Declared codec of span payloads, open: `p`, `av1`, `opus`, …; empty = packed.
    pub codec: String,
}

/// A frame's payload inside a span.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameData {
    /// Names a `plane.*` section — a still, or a burst kept as planes.
    PlaneRef(String),
    Packed(BitPackedTensor),
    Wrapped(u8, Vec<u8>),
    /// A declared hole: the frame is missing and says so.
    Hole,
}

/// A contiguous run of frames of one track with no holes inside — photon's "row".
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub track: String,
    pub seq: String,
    pub first: i64,
    pub data: Vec<FrameData>,
}

impl Span {
    pub fn count(&self) -> usize {
        self.data.len()
    }
}

/// The samples of a plane: bit-packed (lossless), or wrapped under a codec code.
#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    Packed(BitPackedTensor),
    Wrapped(u8, Vec<u8>),
}

/// One 2-D (or `[k, h, w]` planar) sample array with its own content hash in the table of contents — the unit of lineage.
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    pub track: String,
    pub seq: String,
    pub name: String,
    pub role: PlaneRole,
    pub width: usize,
    pub height: usize,
    /// Channel names and, when characterized, spectral sensitivity curves.
    pub channels: Vec<SpectralChannel>,
    pub layout: PlaneLayout,
    /// Per channel, raw counts.
    pub black: Vec<f32>,
    pub white: Vec<f32>,
    pub encoding: Encoding,
    /// Bits per sample, 1–128.
    pub depth: u8,
    pub samples: Samples,
    /// The maker's recommended crop `[x, y, w, h]` in this plane's pixels — a suggestion, never applied.
    pub maker_crop: Option<[u32; 4]>,
    /// EXIF 274 code, a hint for the `orientation` node; the plane itself is sensor-order.
    pub orientation: u16,
    /// Eagle instant of the capture (stills without a rate declaration).
    pub at: Option<i64>,
    /// Lineage: the parent document's `hp` and the parent plane's content hash.
    pub parent: Option<([u8; 32], [u8; 32])>,
    /// Name of the `graph.*` whose root produced this plane, when derived.
    pub program: String,
}

impl Plane {
    /// A capture plane holding `samples` at the given geometry, encoding linear, no crop, upright.
    pub fn sensor(track: &str, seq: &str, width: usize, height: usize, channels: Vec<SpectralChannel>, layout: PlaneLayout, black: Vec<f32>, white: Vec<f32>, samples: BitPackedTensor) -> Self {
        let depth = samples.bit_depth;
        Plane { track: track.into(), seq: seq.into(), name: "sensor".into(), role: PlaneRole::Sensor, width, height, channels, layout, black, white, encoding: Encoding::Linear, depth, samples: Samples::Packed(samples), maker_crop: None, orientation: 1, at: None, parent: None, program: String::new() }
    }
    pub fn section_name(&self) -> String {
        format!("plane.{}.{}.{}", self.track, self.seq, self.name)
    }
}

/// The capture facts: the exposure triangle, the optics, when and where, the body and lens, the mode and the stack recipe — what EXIF and DNG carry today, with Eagle time and a `WorldCoord` where a string and a sexagesimal triple used to be.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Capture {
    pub exposure_s: Option<f32>,
    pub f_number: Option<f32>,
    pub t_stop: Option<f32>,
    pub gain: Option<f32>,
    pub iso: Option<f32>,
    pub focal_mm: Option<f32>,
    pub focal_35mm: Option<f32>,
    pub subject_m: Option<f32>,
    /// DateTimeOriginal as an Eagle instant.
    pub at: Option<i64>,
    /// The source's own date-time text, verbatim (no zone, no subseconds — kept because it is what the file said).
    pub at_text: String,
    pub location: Option<WorldCoord>,
    pub altitude_m: Option<f32>,
    pub make: String,
    pub model: String,
    pub sensor_module: String,
    pub lens: String,
    /// Open: `single` | `burst` | `bracket` | `average` | `slitscan` | `video`.
    pub mode: String,
    /// DNG `BaselineExposure`: the file's declared opening gain in stops — a Technical scalar, applied by the reader, never folded into an operator's exposure.
    pub baseline: f32,
    /// The `graph.*` whose program is the stack recipe, when the file is a stack.
    pub stack: String,
    /// Names of the dark/flat/bias planes applied, if any.
    pub cal_applied: Vec<String>,
    pub shading: String,
}

/// One op of a graph: a node of the layer DAG in its Stack Notation (postfix) order.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphOp {
    pub name: String,
    pub class: IdtClass,
    /// Operands popped.
    pub arity: u8,
    pub params: Vec<f32>,
    /// A section-name operand (`plane.b0.s0.sensor`, `span.b0.s3`), or empty.
    pub reference: String,
}

/// A layer graph as a canonical postfix program; `root` is the op whose result is the output.
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub id: String,
    pub ops: Vec<GraphOp>,
    pub root: usize,
}

impl Graph {
    pub fn section_name(&self) -> String {
        format!("graph.{}", self.id)
    }
    /// BLAKE3 of the canonical program bytes — lineage names programs by this.
    pub fn hash(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"vsf/graph/v1");
        for op in &self.ops {
            h.update(op.name.as_bytes());
            h.update(&[0, op.class.as_str().as_bytes()[0], op.arity]);
            for p in &op.params {
                h.update(&p.to_bits().to_le_bytes());
            }
            h.update(op.reference.as_bytes());
            h.update(&[0]);
        }
        h.update(&(self.root as u64).to_le_bytes());
        *h.finalize().as_bytes()
    }
}

/// A named output of a graph.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub id: String,
    pub graph: String,
    pub root: usize,
    pub space: String,
    pub encoding: Encoding,
    pub bits: u8,
    pub purpose: Purpose,
}

/// Foreign metadata carried verbatim: a source file's XMP packet and ICC profile. Renderings (a JPEG preview) are not — they are re-rendered.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Foreign {
    pub xmp: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
    /// The source's profile name, if it had one (DNG `ProfileName`).
    pub profile_name: String,
    /// The source's calibration pairing string (DNG `CameraCalibrationSignature`).
    pub calibration_signature: String,
}

/// The whole visual document, in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Visual {
    pub title: String,
    /// Free text — ImageDescription, a comment.
    pub text: String,
    pub default_view: String,
    pub tracks: Vec<Track>,
    pub spans: Vec<Span>,
    pub planes: Vec<Plane>,
    pub capture: Option<Capture>,
    pub characterization: Option<ColourProfile>,
    /// The observer the characterization's matrices target (open: `cie1931`, `cvrl2006`); empty when unstated.
    pub observer: String,
    /// The camera's spectral response under the calibration light — the Relative IDT's stored half; the characterization's matrices are its cache (`idt/relative.md`, "The representation").
    pub response: Option<SpectralResponse>,
    pub graphs: Vec<Graph>,
    pub views: Vec<View>,
    pub foreign: Foreign,
    pub provenance: Provenance,
}

impl Default for Visual {
    fn default() -> Self {
        Visual { title: String::new(), text: String::new(), default_view: String::new(), tracks: Vec::new(), spans: Vec::new(), planes: Vec::new(), capture: None, characterization: None, observer: String::new(), response: None, graphs: Vec::new(), views: Vec::new(), foreign: Foreign::default(), provenance: Provenance::default() }
    }
}

/// A camera's spectral response under its calibration light, `P₀_c(λ) = S_c(λ)·E_ref(λ)`: one value per band per camera channel on a linear grid, **one common scale** across channels (their relative heights are information). Stored in `characterization` as `response_start`, `response_step`, `response_count` (bands), `response_channels`, `response_values` (`[channels × bands]`, channel-major), `response_prior` (how the curves were regularised — open, `shortest` today) and `response_light` (BLAKE3 identity of the calibration scan; two responses with the same light compare directly, and a shot's illuminant ratio is relative to it).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpectralResponse {
    pub start_nm: f32,
    pub step_nm: f32,
    pub channels: Vec<Vec<f32>>,
    pub prior: String,
    pub light: Option<[u8; 32]>,
}

impl SpectralResponse {
    pub fn fields(&self) -> Vec<(String, VsfType)> {
        let n = self.channels.first().map_or(0, Vec::len);
        let mut f = vec![
            ("response_start".to_string(), VsfType::f5(self.start_nm)),
            ("response_step".to_string(), VsfType::f5(self.step_nm)),
            ("response_count".to_string(), VsfType::u(n, false)),
            ("response_channels".to_string(), VsfType::u(self.channels.len(), false)),
            ("response_values".to_string(), VsfType::t_f5(Tensor::new(vec![self.channels.len() * n], self.channels.iter().flat_map(|c| c.iter().copied()).collect()))),
        ];
        push_text(&mut f, "response_prior", &self.prior);
        if let Some(l) = &self.light {
            f.push(("response_light".to_string(), VsfType::hb(l.to_vec())));
        }
        f
    }

    /// From a characterization section; `None` when it carries no response.
    pub fn from_section(s: &VsfSection) -> Result<Option<SpectralResponse>, VisualError> {
        let Some(values) = f32_vec(s, "response_values") else { return Ok(None) };
        let scalar = |name: &str| match s.get_field(name).and_then(|f| f.values.first()) {
            Some(VsfType::f5(v)) => Some(*v),
            Some(VsfType::f6(v)) => Some(*v as f32),
            _ => None,
        };
        let uint = |name: &str| s.get_field(name).and_then(|f| f.values.first()).and_then(u64_of).map(|v| v as usize);
        let count = uint("response_count").ok_or(VisualError::Missing("response_count"))?;
        let k = uint("response_channels").ok_or(VisualError::Missing("response_channels"))?;
        if count == 0 || k == 0 || values.len() != count * k {
            return Err(bad(format!("response_values holds {} values for {k} channels × {count} bands", values.len())));
        }
        Ok(Some(SpectralResponse {
            start_nm: scalar("response_start").ok_or(VisualError::Missing("response_start"))?,
            step_nm: scalar("response_step").ok_or(VisualError::Missing("response_step"))?,
            channels: values.chunks(count).map(|c| c.to_vec()).collect(),
            prior: text(s, "response_prior"),
            light: s.get_field("response_light").and_then(|f| f.values.first()).and_then(hash32),
        }))
    }
}

// ───────────────────────── graph ↔ view_transform ─────────────────────────

/// The reserved op vocabulary a still's default graph uses. Open set: a reader surfaces an op it does not know, never drops it.
pub const OP_PLANE: &str = "plane";
pub const OP_BLACK_WHITE: &str = "black_white";
pub const OP_IDT: &str = "idt";
pub const OP_ORIENTATION: &str = "orientation";
pub const OP_CROP: &str = "crop";
pub const OP_BASELINE: &str = "baseline";
pub const OP_EXPOSURE: &str = "exposure";
pub const OP_DR_CURVE: &str = "dr_curve";
pub const OP_ENCODE: &str = "encode";

impl Graph {
    /// The default graph of a still: the sensor plane, normalised, through the first characterization entry, then the view ops in order. Every op after `idt` is arity 1, so a reader can strip the Creative ones and the program stays well-formed.
    pub fn default_still(id: &str, plane: &str, idt_class: Option<IdtClass>, ops: &[ViewOp]) -> Graph {
        let mut g = vec![
            GraphOp { name: OP_PLANE.into(), class: IdtClass::Technical, arity: 0, params: vec![], reference: plane.into() },
            GraphOp { name: OP_BLACK_WHITE.into(), class: IdtClass::Technical, arity: 1, params: vec![], reference: String::new() },
        ];
        // The `idt` op carries the class of the characterization entry it indexes — an Absolute matrix makes an Absolute op.
        if let Some(class) = idt_class {
            g.push(GraphOp { name: OP_IDT.into(), class, arity: 1, params: vec![0.0], reference: String::new() });
        }
        for op in ops {
            g.push(GraphOp { name: op.name.clone(), class: op.class, arity: 1, params: op.params.clone(), reference: String::new() });
        }
        let root = g.len() - 1;
        Graph { id: id.into(), ops: g, root }
    }

    /// The view ops of a still's default graph: everything after the source/normalise/idt prologue, as a [`ViewTransform`] — what opsin replays.
    pub fn to_view_transform(&self) -> ViewTransform {
        let ops = self
            .ops
            .iter()
            .filter(|op| !matches!(op.name.as_str(), OP_PLANE | OP_BLACK_WHITE | OP_IDT | OP_ENCODE))
            .map(|op| ViewOp { name: op.name.clone(), class: op.class, params: op.params.clone() })
            .collect();
        ViewTransform { space: "vsf_rgb_linear".into(), ops }
    }
}

// ───────────────────────── helpers ─────────────────────────

fn f32_of(v: &VsfType) -> Option<f32> {
    match v {
        VsfType::f5(x) => Some(*x),
        VsfType::f6(x) => Some(*x as f32),
        _ => None,
    }
}

fn u64_of(v: &VsfType) -> Option<u64> {
    use crate::schema::FromVsfType;
    u64::from_vsf_type(v).ok()
}

fn i64_of(v: &VsfType) -> Option<i64> {
    use crate::schema::FromVsfType;
    i64::from_vsf_type(v).ok().or_else(|| u64_of(v).map(|u| u as i64))
}

fn first<'a>(s: &'a VsfSection, name: &str) -> Option<&'a VsfType> {
    s.get_field(name).and_then(|f| f.values.first())
}

fn f32_field(s: &VsfSection, name: &str) -> Option<f32> {
    first(s, name).and_then(f32_of)
}

fn f32_vec(s: &VsfSection, name: &str) -> Option<Vec<f32>> {
    match first(s, name)? {
        VsfType::t_f5(t) => Some(t.data.clone()),
        _ => None,
    }
}

fn u32_vec(s: &VsfSection, name: &str) -> Option<Vec<u32>> {
    match first(s, name)? {
        VsfType::t_u5(t) => Some(t.data.clone()),
        VsfType::t_u4(t) => Some(t.data.iter().map(|&x| x as u32).collect()),
        VsfType::t_u3(t) => Some(t.data.iter().map(|&x| x as u32).collect()),
        _ => None,
    }
}

fn text(s: &VsfSection, name: &str) -> String {
    s.text(name).unwrap_or_default().to_string()
}

fn eagle(s: &VsfSection, name: &str) -> Option<i64> {
    match first(s, name)? {
        VsfType::e(EtType::e6(o)) => Some(*o),
        VsfType::e(EtType::e5(o)) => Some(*o as i64),
        #[allow(deprecated)]
        VsfType::e(EtType::e7(o)) => Some(*o as i64),
        _ => None,
    }
}

fn hash32(v: &VsfType) -> Option<[u8; 32]> {
    match v {
        VsfType::hp(b) | VsfType::hb(b) | VsfType::hs(b) if b.len() == 32 => b.as_slice().try_into().ok(),
        _ => None,
    }
}

fn split_n(joined: &str, n: usize, what: &str) -> Result<Vec<String>, VisualError> {
    if n == 0 && joined.is_empty() {
        return Ok(Vec::new());
    }
    let parts: Vec<String> = joined.split('\n').map(str::to_string).collect();
    if parts.len() != n {
        return Err(bad(format!("{what} carries {} entries for {n} rows", parts.len())));
    }
    Ok(parts)
}

fn push_f32(fields: &mut Vec<(String, VsfType)>, name: &str, v: Option<f32>) {
    if let Some(x) = v {
        fields.push((name.to_string(), VsfType::f5(x)));
    }
}

fn push_text(fields: &mut Vec<(String, VsfType)>, name: &str, v: &str) {
    if !v.is_empty() {
        fields.push((name.to_string(), VsfType::a(v.to_string())));
    }
}

// ───────────────────────── section codecs ─────────────────────────

impl Track {
    pub fn section_name(&self) -> String {
        format!("track.{}", self.id)
    }
    fn fields(&self) -> Vec<(String, VsfType)> {
        let mut f = vec![("kind".to_string(), VsfType::a(self.kind.as_str().into())), ("role".to_string(), VsfType::a(self.role.as_str().into())), ("channels".to_string(), VsfType::u(self.channels, false))];
        push_text(&mut f, "title", &self.title);
        push_text(&mut f, "codec", &self.codec);
        f
    }
    fn from_section(id: &str, s: &VsfSection) -> Result<Track, VisualError> {
        Ok(Track { id: id.into(), kind: TrackKind::parse(&text(s, "kind"))?, role: TrackRole::parse(&text(s, "role"))?, title: text(s, "title"), channels: s.uint("channels").unwrap_or(0) as usize, codec: text(s, "codec") })
    }
}

impl Span {
    pub fn section_name(&self) -> String {
        format!("span.{}.{}", self.track, self.seq)
    }
    fn section(&self) -> VsfSection {
        let mut s = VsfSection::new(self.section_name());
        s.add_field("track", VsfType::a(self.track.clone()));
        s.add_field("first", VsfType::i(self.first as isize));
        s.add_field("count", VsfType::u(self.data.len(), false));
        for d in &self.data {
            match d {
                FrameData::PlaneRef(name) => s.add_field("data", VsfType::a(name.clone())),
                FrameData::Packed(t) => s.add_field("data", VsfType::p(t.clone())),
                FrameData::Wrapped(code, bytes) => s.add_field("data", VsfType::v(*code, bytes.clone())),
                FrameData::Hole => s.add_field("data", VsfType::v(b'b', Vec::new())),
            }
        }
        s
    }
    fn from_section(track: &str, seq: &str, s: &VsfSection) -> Result<Span, VisualError> {
        let first_frame = first(s, "first").and_then(i64_of).unwrap_or(0);
        let data = s
            .get_fields("data")
            .iter()
            .filter_map(|f| f.values.first())
            .map(|v| match v {
                VsfType::a(n) | VsfType::x(n) => FrameData::PlaneRef(n.clone()),
                VsfType::p(t) => FrameData::Packed(t.clone()),
                VsfType::v(c, b) if b.is_empty() => {
                    let _ = c;
                    FrameData::Hole
                }
                VsfType::v(c, b) => FrameData::Wrapped(*c, b.clone()),
                _ => FrameData::Hole,
            })
            .collect();
        Ok(Span { track: track.into(), seq: seq.into(), first: first_frame, data })
    }
}

impl Plane {
    fn section(&self) -> Result<VsfSection, VisualError> {
        let k = self.channels.len();
        if self.black.len() != k || self.white.len() != k {
            return Err(bad(format!("plane {}: black/white length {}/{} != channel count {k}", self.section_name(), self.black.len(), self.white.len())));
        }
        let mut s = VsfSection::new(self.section_name());
        s.add_field("role", VsfType::a(self.role.as_str().into()));
        s.add_field("width", VsfType::u(self.width, false));
        s.add_field("height", VsfType::u(self.height, false));
        s.add_field("channel_count", VsfType::u(k, false));
        s.add_field("channel_names", VsfType::a(self.channels.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\n")));
        match &self.layout {
            PlaneLayout::Mosaic { cfa } => {
                s.add_field("layout", VsfType::a("mosaic".into()));
                s.add_field("cfa", VsfType::t_u3(cfa.clone()));
            }
            PlaneLayout::Planar => s.add_field("layout", VsfType::a("planar".into())),
        }
        s.add_field("black", VsfType::t_f5(Tensor::new(vec![k], self.black.clone())));
        s.add_field("white", VsfType::t_f5(Tensor::new(vec![k], self.white.clone())));
        s.add_field("encoding", VsfType::a(self.encoding.as_str().into()));
        s.add_field("depth", VsfType::u(self.depth as usize, false));
        if let Some(c) = self.maker_crop {
            s.add_field("maker_crop", VsfType::t_u5(Tensor::new(vec![4], c.to_vec())));
        }
        if self.orientation != 1 && self.orientation != 0 {
            s.add_field("orientation", VsfType::u(self.orientation as usize, false));
        }
        if let Some(at) = self.at {
            s.add_field("at", VsfType::e(EtType::e6(at)));
        }
        if let Some((hp, h)) = &self.parent {
            s.add_field("parent", VsfType::hp(hp.to_vec()));
            s.add_field("parent_plane", VsfType::hb(h.to_vec()));
        }
        if !self.program.is_empty() {
            s.add_field("program", VsfType::a(self.program.clone()));
        }
        // Spectral sensitivity, when any channel is characterized — the spectral_response codec, on the plane.
        if self.channels.iter().any(|c| c.curve.is_some()) {
            let (mut starts, mut steps, mut counts, mut values): (Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>) = (vec![], vec![], vec![], vec![]);
            for c in &self.channels {
                match &c.curve {
                    Some(cv) => {
                        starts.push(cv.start_nm);
                        steps.push(cv.step_nm);
                        counts.push(cv.values.len() as u32);
                        values.extend_from_slice(&cv.values);
                    }
                    None => {
                        starts.push(0.);
                        steps.push(0.);
                        counts.push(0);
                    }
                }
            }
            let total = values.len();
            s.add_field("curve_start", VsfType::t_f5(Tensor::new(vec![k], starts)));
            s.add_field("curve_step", VsfType::t_f5(Tensor::new(vec![k], steps)));
            s.add_field("curve_counts", VsfType::t_u5(Tensor::new(vec![k], counts)));
            s.add_field("curve_values", VsfType::t_f5(Tensor::new(vec![total], values)));
        }
        // Samples last: the bulk, located by a lazy reader from the header before it.
        match &self.samples {
            Samples::Packed(t) => s.add_field("samples", VsfType::p(t.clone())),
            Samples::Wrapped(code, b) => s.add_field("samples", VsfType::v(*code, b.clone())),
        }
        Ok(s)
    }

    fn from_section(track: &str, seq: &str, name: &str, s: &VsfSection) -> Result<Plane, VisualError> {
        let k = s.uint("channel_count").ok_or(VisualError::Missing("plane channel_count"))? as usize;
        let names = split_n(&text(s, "channel_names"), k, "channel_names")?;
        let mut curves: Vec<Option<SpectralCurve>> = vec![None; k];
        if let (Some(starts), Some(steps), Some(counts), Some(values)) = (f32_vec(s, "curve_start"), f32_vec(s, "curve_step"), u32_vec(s, "curve_counts"), f32_vec(s, "curve_values")) {
            if starts.len() != k || steps.len() != k || counts.len() != k {
                return Err(bad("plane curve arrays do not match channel_count"));
            }
            let mut cursor = 0usize;
            for i in 0..k {
                let n = counts[i] as usize;
                if n == 0 {
                    continue;
                }
                if cursor + n > values.len() {
                    return Err(bad("plane curve_values shorter than curve_counts total"));
                }
                curves[i] = Some(SpectralCurve { start_nm: starts[i], step_nm: steps[i], values: values[cursor..cursor + n].to_vec() });
                cursor += n;
            }
        }
        let channels = names.into_iter().zip(curves).map(|(name, curve)| SpectralChannel { name, curve }).collect();
        let layout = match text(s, "layout").as_str() {
            "mosaic" => match first(s, "cfa") {
                Some(VsfType::t_u3(t)) => PlaneLayout::Mosaic { cfa: t.clone() },
                _ => return Err(VisualError::Missing("plane cfa")),
            },
            "planar" => PlaneLayout::Planar,
            other => return Err(bad(format!("unknown layout '{other}'"))),
        };
        let samples = match first(s, "samples") {
            Some(VsfType::p(t)) => Samples::Packed(t.clone()),
            Some(VsfType::v(c, b)) => Samples::Wrapped(*c, b.clone()),
            _ => return Err(VisualError::Missing("plane samples")),
        };
        let depth = s.uint("depth").map(|d| d as u8).unwrap_or(match &samples {
            Samples::Packed(t) => t.bit_depth,
            Samples::Wrapped(..) => 0,
        });
        let maker_crop = u32_vec(s, "maker_crop").and_then(|v| <[u32; 4]>::try_from(v).ok());
        let parent = match (first(s, "parent").and_then(hash32), first(s, "parent_plane").and_then(hash32)) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        };
        Ok(Plane {
            track: track.into(),
            seq: seq.into(),
            name: name.into(),
            role: PlaneRole::parse(&s.text("role").unwrap_or("sensor"))?,
            width: s.uint("width").ok_or(VisualError::Missing("plane width"))? as usize,
            height: s.uint("height").ok_or(VisualError::Missing("plane height"))? as usize,
            channels,
            layout,
            black: f32_vec(s, "black").ok_or(VisualError::Missing("plane black"))?,
            white: f32_vec(s, "white").ok_or(VisualError::Missing("plane white"))?,
            // The integer-plane rule: absent means gamma 2. Writers here always say.
            encoding: match s.text("encoding") {
                Some(e) => Encoding::parse(e)?,
                None => Encoding::Gamma2,
            },
            depth,
            samples,
            maker_crop,
            orientation: s.uint("orientation").map(|o| o as u16).unwrap_or(1),
            at: eagle(s, "at"),
            parent,
            program: text(s, "program"),
        })
    }
}

impl Capture {
    fn fields(&self) -> Vec<(String, VsfType)> {
        let mut f = Vec::new();
        push_f32(&mut f, "exposure_s", self.exposure_s);
        push_f32(&mut f, "f_number", self.f_number);
        push_f32(&mut f, "t_stop", self.t_stop);
        push_f32(&mut f, "gain", self.gain);
        push_f32(&mut f, "iso", self.iso);
        push_f32(&mut f, "focal_mm", self.focal_mm);
        push_f32(&mut f, "focal_35mm", self.focal_35mm);
        push_f32(&mut f, "subject_m", self.subject_m);
        if let Some(at) = self.at {
            f.push(("at".into(), VsfType::e(EtType::e6(at))));
        }
        push_text(&mut f, "at_text", &self.at_text);
        if let Some(loc) = &self.location {
            f.push(("location".into(), VsfType::w(*loc)));
        }
        push_f32(&mut f, "altitude_m", self.altitude_m);
        push_text(&mut f, "make", &self.make);
        push_text(&mut f, "model", &self.model);
        push_text(&mut f, "sensor_module", &self.sensor_module);
        push_text(&mut f, "lens", &self.lens);
        push_text(&mut f, "mode", &self.mode);
        if self.baseline != 0.0 {
            f.push(("baseline".into(), VsfType::f5(self.baseline)));
        }
        push_text(&mut f, "stack", &self.stack);
        if !self.cal_applied.is_empty() {
            f.push(("cal_applied".into(), VsfType::a(self.cal_applied.join("\n"))));
        }
        push_text(&mut f, "shading", &self.shading);
        f
    }
    fn from_section(s: &VsfSection) -> Capture {
        Capture {
            exposure_s: f32_field(s, "exposure_s"),
            f_number: f32_field(s, "f_number"),
            t_stop: f32_field(s, "t_stop"),
            gain: f32_field(s, "gain"),
            iso: f32_field(s, "iso"),
            focal_mm: f32_field(s, "focal_mm"),
            focal_35mm: f32_field(s, "focal_35mm"),
            subject_m: f32_field(s, "subject_m"),
            at: eagle(s, "at"),
            at_text: text(s, "at_text"),
            location: match first(s, "location") {
                Some(VsfType::w(c)) => Some(*c),
                _ => None,
            },
            altitude_m: f32_field(s, "altitude_m"),
            make: text(s, "make"),
            model: text(s, "model"),
            sensor_module: text(s, "sensor_module"),
            lens: text(s, "lens"),
            mode: text(s, "mode"),
            baseline: f32_field(s, "baseline").unwrap_or(0.0),
            stack: text(s, "stack"),
            cal_applied: s.text("cal_applied").map(|t| t.split('\n').map(str::to_string).collect()).unwrap_or_default(),
            shading: text(s, "shading"),
        }
    }
    pub fn is_empty(&self) -> bool {
        *self == Capture::default()
    }
}

impl Graph {
    pub fn fields(&self) -> Vec<(String, VsfType)> {
        let m = self.ops.len();
        let mut names = Vec::with_capacity(m);
        let mut classes = Vec::with_capacity(m);
        let mut arity: Vec<u8> = Vec::with_capacity(m);
        let mut counts: Vec<u32> = Vec::with_capacity(m);
        let mut params: Vec<f32> = Vec::new();
        let mut refs = Vec::with_capacity(m);
        for op in &self.ops {
            names.push(op.name.as_str());
            classes.push(op.class.as_str());
            arity.push(op.arity);
            counts.push(op.params.len() as u32);
            params.extend_from_slice(&op.params);
            refs.push(if op.reference.is_empty() { "-" } else { op.reference.as_str() });
        }
        let total = params.len();
        vec![
            ("ops".into(), VsfType::a(names.join("\n"))),
            ("classes".into(), VsfType::a(classes.join("\n"))),
            ("arity".into(), VsfType::t_u3(Tensor::new(vec![m], arity))),
            ("param_counts".into(), VsfType::t_u5(Tensor::new(vec![m], counts))),
            ("params".into(), VsfType::t_f5(Tensor::new(vec![total], params))),
            ("refs".into(), VsfType::a(refs.join("\n"))),
            ("root".into(), VsfType::u(self.root, false)),
            ("hash".into(), VsfType::hb(self.hash().to_vec())),
        ]
    }
    fn from_section(id: &str, s: &VsfSection) -> Result<Graph, VisualError> {
        let counts = u32_vec(s, "param_counts").ok_or(VisualError::Missing("graph param_counts"))?;
        let m = counts.len();
        let names = split_n(&text(s, "ops"), m, "graph ops")?;
        let classes = split_n(&text(s, "classes"), m, "graph classes")?;
        let refs = split_n(&text(s, "refs"), m, "graph refs")?;
        let arity = u32_vec(s, "arity").ok_or(VisualError::Missing("graph arity"))?;
        if arity.len() != m {
            return Err(bad("graph arity does not match ops"));
        }
        let params = f32_vec(s, "params").unwrap_or_default();
        let mut ops = Vec::with_capacity(m);
        let mut cursor = 0usize;
        for i in 0..m {
            let n = counts[i] as usize;
            if cursor + n > params.len() {
                return Err(bad("graph params shorter than param_counts total"));
            }
            ops.push(GraphOp { name: names[i].clone(), class: IdtClass::parse(&classes[i]).map_err(|e| bad(e.to_string()))?, arity: arity[i] as u8, params: params[cursor..cursor + n].to_vec(), reference: if refs[i] == "-" { String::new() } else { refs[i].clone() } });
            cursor += n;
        }
        let root = s.uint("root").map(|r| r as usize).unwrap_or(m.saturating_sub(1));
        if root >= m && m > 0 {
            return Err(bad(format!("graph root {root} outside {m} ops")));
        }
        Ok(Graph { id: id.into(), ops, root })
    }
}

impl View {
    pub fn section_name(&self) -> String {
        format!("view.{}", self.id)
    }
    /// The default view of a still: `graph`'s root, in VSF RGB linear.
    pub fn default_of(id: &str, graph: &Graph) -> View {
        View { id: id.into(), graph: graph.id.clone(), root: graph.root, space: "vsf_rgb_linear".into(), encoding: Encoding::Linear, bits: 16, purpose: Purpose::Default }
    }
    pub fn fields(&self) -> Vec<(String, VsfType)> {
        vec![("graph".into(), VsfType::a(self.graph.clone())), ("root".into(), VsfType::u(self.root, false)), ("space".into(), VsfType::a(self.space.clone())), ("encoding".into(), VsfType::a(self.encoding.as_str().into())), ("bits".into(), VsfType::u(self.bits as usize, false)), ("purpose".into(), VsfType::a(self.purpose.as_str().into()))]
    }
    fn from_section(id: &str, s: &VsfSection) -> Result<View, VisualError> {
        Ok(View { id: id.into(), graph: text(s, "graph"), root: s.uint("root").unwrap_or(0) as usize, space: text(s, "space"), encoding: Encoding::parse(s.text("encoding").unwrap_or("linear"))?, bits: s.uint("bits").unwrap_or(16) as u8, purpose: Purpose::parse(s.text("purpose").unwrap_or("default"))? })
    }
}

impl Foreign {
    pub fn fields(&self) -> Vec<(String, VsfType)> {
        let mut f = Vec::new();
        if let Some(x) = &self.xmp {
            f.push(("xmp".into(), VsfType::v(b'b', x.clone())));
        }
        if let Some(i) = &self.icc {
            f.push(("icc".into(), VsfType::v(b'b', i.clone())));
        }
        push_text(&mut f, "profile_name", &self.profile_name);
        push_text(&mut f, "calibration_signature", &self.calibration_signature);
        f
    }
    fn from_section(s: &VsfSection) -> Foreign {
        let blob = |name: &str| match first(s, name) {
            Some(VsfType::v(_, b)) => Some(b.clone()),
            _ => None,
        };
        Foreign { xmp: blob("xmp"), icc: blob("icc"), profile_name: text(s, "profile_name"), calibration_signature: text(s, "calibration_signature") }
    }
    pub fn is_empty(&self) -> bool {
        *self == Foreign::default()
    }
}

/// The characterization section: today's `colour_profile` fields (reused through `spectral_image`'s own codec) plus `observer`.
pub fn characterization_fields(p: &ColourProfile, observer: &str, response: Option<&SpectralResponse>) -> Vec<(String, VsfType)> {
    let mut f = spectral_image::profile_fields(p);
    push_text(&mut f, "observer", observer);
    if let Some(r) = response {
        f.extend(r.fields());
    }
    f
}

// ───────────────────────── write / read ─────────────────────────

impl Visual {
    /// A still from its parts: one beam track `b0`, one span `s0` of one frame naming the plane, the plane as `plane.b0.s0.sensor`, the default graph and view.
    pub fn still(plane: Plane, capture: Option<Capture>, characterization: Option<ColourProfile>, ops: &[ViewOp]) -> Visual {
        let mut plane = plane;
        plane.track = "b0".into();
        plane.seq = "s0".into();
        let plane_name = plane.section_name();
        let graph = Graph::default_still("g0", &plane_name, characterization.as_ref().and_then(|p| p.entries.first()).map(|e| e.class), ops);
        let view = View::default_of("v0", &graph);
        Visual {
            default_view: view.section_name(),
            tracks: vec![Track { id: "b0".into(), kind: TrackKind::Beam, role: TrackRole::Capture, title: String::new(), channels: 1, codec: String::new() }],
            spans: vec![Span { track: "b0".into(), seq: "s0".into(), first: 0, data: vec![FrameData::PlaneRef(plane_name)] }],
            planes: vec![plane],
            capture,
            characterization,
            graphs: vec![graph],
            views: vec![view],
            ..Visual::default()
        }
    }

    /// The primary plane: the first `sensor` plane, else the first plane.
    pub fn primary_plane(&self) -> Option<&Plane> {
        self.planes.iter().find(|p| p.role == PlaneRole::Sensor).or(self.planes.first())
    }

    /// The default view's graph.
    pub fn default_graph(&self) -> Option<&Graph> {
        let view = self.views.iter().find(|v| v.section_name() == self.default_view).or(self.views.first())?;
        self.graphs.iter().find(|g| g.id == view.graph).or(self.graphs.first())
    }

    /// Serialise as a generation-0 document, every section with a content hash in its entry.
    pub fn write(&self) -> Result<Vec<u8>, String> {
        let mut b = VsfBuilder::new().content_hashes(true);
        if let Some(at) = self.capture.as_ref().and_then(|c| c.at) {
            b = b.creation_time_oscillations(at);
        }
        let mut manifest = vec![("tracks".to_string(), VsfType::a(self.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join("\n"))), ("default_view".to_string(), VsfType::a(self.default_view.clone()))];
        push_text(&mut manifest, "title", &self.title);
        push_text(&mut manifest, "text", &self.text);
        b = b.add_section("visual", manifest);
        for t in &self.tracks {
            b = b.add_section(t.section_name(), t.fields());
        }
        for s in &self.spans {
            b = b.add_section_direct(s.section());
        }
        if let Some(c) = &self.capture {
            if !c.is_empty() {
                b = b.add_section("capture", c.fields());
            }
        }
        if let Some(p) = &self.characterization {
            b = b.add_section("characterization", characterization_fields(p, &self.observer, self.response.as_ref()));
        }
        for g in &self.graphs {
            b = b.add_section(g.section_name(), g.fields());
        }
        for v in &self.views {
            b = b.add_section(v.section_name(), v.fields());
        }
        if !self.foreign.is_empty() {
            b = b.add_section("foreign", self.foreign.fields());
        }
        let prov = spectral_image::provenance_fields(&self.provenance)?;
        if !prov.is_empty() {
            b = b.add_section("provenance", prov);
        }
        // Planes last: the bulk sits after every small section, so a lazy reader's header-and-metadata pass is short.
        for p in &self.planes {
            b = b.add_section_direct(p.section().map_err(|e| e.to_string())?);
        }
        b.build()
    }

    /// Read from a document. A document with a `spectral_image` section (the legacy five-section form) is read as a one-track visual first; then every live section overlays it — so a generation appended to a legacy file (a pasted characterization, a new view) is seen, and the file never needs rewriting to be edited.
    pub fn read_document<S: ByteSource>(doc: &Document<S>) -> Result<Visual, VisualError> {
        let legacy = doc.entry("spectral_image").is_some();
        let mut v = if legacy { Self::read_legacy_base(doc)? } else { Visual::default() };
        let mut seen_visual = false;
        for e in doc.body_entries() {
            let name = e.name.clone();
            let parts: Vec<&str> = name.split('.').collect();
            if legacy && matches!(parts[0], "spectral_image" | "spectral_response" | "view_transform") {
                continue;
            }
            let s = doc.section(e)?;
            match (parts[0], parts.len()) {
                ("visual", 1) => {
                    seen_visual = true;
                    v.title = text(&s, "title");
                    v.text = text(&s, "text");
                    v.default_view = text(&s, "default_view");
                }
                ("track", 2) => {
                    let t = Track::from_section(parts[1], &s)?;
                    v.tracks.retain(|x| x.id != t.id);
                    v.tracks.push(t);
                }
                ("span", 3) => {
                    let sp = Span::from_section(parts[1], parts[2], &s)?;
                    v.spans.retain(|x| !(x.track == sp.track && x.seq == sp.seq));
                    v.spans.push(sp);
                }
                ("plane", 4) => {
                    let p = Plane::from_section(parts[1], parts[2], parts[3], &s)?;
                    v.planes.retain(|x| x.section_name() != p.section_name());
                    v.planes.push(p);
                }
                ("capture", 1) => v.capture = Some(Capture::from_section(&s)),
                ("characterization", 1) | ("colour_profile", 1) => {
                    v.characterization = Some(spectral_image::profile_from_fields(&s.fields).map_err(|e| bad(e.to_string()))?);
                    v.observer = text(&s, "observer");
                    v.response = SpectralResponse::from_section(&s)?;
                }
                ("graph", 2) => {
                    let g = Graph::from_section(parts[1], &s)?;
                    v.graphs.retain(|x| x.id != g.id);
                    v.graphs.push(g);
                }
                ("view", 2) => {
                    let vw = View::from_section(parts[1], &s)?;
                    v.views.retain(|x| x.id != vw.id);
                    v.views.push(vw);
                }
                ("foreign", 1) => v.foreign = Foreign::from_section(&s),
                ("provenance", 1) => v.provenance = spectral_image::provenance_from_fields(&s.fields),
                _ => {} // Unknown sections are kept by the container, ignored by this reader.
            }
        }
        if !seen_visual && !legacy {
            if doc.entry("image").is_some() {
                return Err(bad("an `image` document (icon/preview) is not a visual; it has no plane"));
            }
            return Err(VisualError::Missing("visual"));
        }
        if v.planes.is_empty() {
            return Err(VisualError::Missing("plane"));
        }
        Ok(v)
    }

    /// Read from bytes.
    pub fn read(bytes: &[u8]) -> Result<Visual, VisualError> {
        let doc = Document::open(bytes)?;
        Self::read_document(&doc)
    }

    /// A legacy five-section `spectral_image` document as a visual: one track, one span, the plane with `encoding = linear` (its only writer, opsin, wrote linear counts), the profile as the characterization, the view log as the graph. The graph's source op references the `spectral_image` section itself, since that is where the plane lives.
    fn read_legacy_base<S: ByteSource>(doc: &Document<S>) -> Result<Visual, VisualError> {
        let gens = doc.generations()?;
        let end = gens.first().map(|g| g.end).unwrap_or(doc.end());
        let bytes = doc.source().read_vec(0, end).map_err(ContainerError::from)?;
        let img = spectral_image::read(&bytes).map_err(|e| bad(e.to_string()))?;
        let mut v = Visual::from_spectral_image(&img, None, Foreign::default());
        for g in &mut v.graphs {
            for op in &mut g.ops {
                if op.name == OP_PLANE {
                    op.reference = "spectral_image".into();
                }
            }
        }
        Ok(v)
    }

    /// Read a file. On unix the `Document` reads the file positionally (no copy); elsewhere — Windows, and a wasm32 build with `std` such as the fgtw worker, where `File` has no positional reads and so no `ByteSource` impl — the file is read whole and the `Document` opens over the bytes (2026-10-10: the unix-only gate this replaced broke limbus on Windows).
    #[cfg(feature = "std")]
    pub fn read_file(path: &std::path::Path) -> Result<Visual, VisualError> {
        #[cfg(unix)]
        {
            let f = std::fs::File::open(path).map_err(|e| bad(format!("{}: {e}", path.display())))?;
            let doc = Document::open(f)?;
            Self::read_document(&doc)
        }
        #[cfg(not(unix))]
        {
            let bytes = std::fs::read(path).map_err(|e| bad(format!("{}: {e}", path.display())))?;
            let doc = Document::open(bytes)?;
            Self::read_document(&doc)
        }
    }

    /// The bytes that append an EDIT generation to `doc`: a new graph and view holding `ops` (the default still graph: source, normalise, idt, then the ops), a `visual` manifest pointing the default view at it, the previous graphs and views superseded in the table of contents (their bytes stay; history has them). The caller appends them (`container::append_to_file`).
    pub fn still_edit_generation<S: ByteSource>(doc: &Document<S>, tool: &str, action: &str, ops: &[ViewOp]) -> Result<Vec<u8>, VisualError> {
        let vis = Self::read_document(doc)?;
        let plane_ref = vis.default_graph().and_then(|g| g.ops.iter().find(|o| o.name == OP_PLANE)).map(|o| o.reference.clone()).or_else(|| vis.primary_plane().map(|p| p.section_name())).ok_or(VisualError::Missing("plane"))?;
        let n = doc.generation() + 1;
        let graph = Graph::default_still(&format!("g{n}"), &plane_ref, vis.characterization.as_ref().and_then(|p| p.entries.first()).map(|e| e.class), ops);
        let view = View::default_of(&format!("v{n}"), &graph);
        let mut manifest = vec![("tracks".to_string(), VsfType::a(vis.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join("\n"))), ("default_view".to_string(), VsfType::a(view.section_name()))];
        push_text(&mut manifest, "title", &vis.title);
        push_text(&mut manifest, "text", &vis.text);
        let mut a = crate::container::Appender::new(tool, action).add_section("visual", manifest).add_section(graph.section_name(), graph.fields()).add_section(view.section_name(), view.fields());
        for e in doc.body_entries() {
            if e.name.starts_with("graph.") || e.name.starts_with("view.") {
                a = a.supersede(e.name.clone());
            }
        }
        Ok(a.build(doc)?)
    }

    /// Bridge from the in-memory model: the image's plane, profile and view log, plus the capture facts and foreign metadata a `SpectralImage` has no field for.
    pub fn from_spectral_image(img: &SpectralImage, capture: Option<Capture>, foreign: Foreign) -> Visual {
        let mut capture = capture.unwrap_or_default();
        if capture.make.is_empty() {
            capture.make = img.make.clone();
        }
        if capture.model.is_empty() {
            capture.model = img.model.clone();
        }
        let plane = Plane::sensor("b0", "s0", img.width, img.height, img.channels.clone(), img.layout.clone(), img.black.clone(), img.white.clone(), img.samples.clone());
        let ops: Vec<ViewOp> = img.view.as_ref().map(|v| v.ops.clone()).unwrap_or_default();
        let mut v = Visual::still(plane, if capture.is_empty() { None } else { Some(capture) }, img.profile.clone(), &ops);
        v.foreign = foreign;
        v.provenance = img.provenance.clone();
        v
    }

    /// Bridge to the in-memory model: the primary plane as the image, the characterization as its profile, the default graph's view ops as its view log. Extra planes, the capture facts and foreign metadata stay on the `Visual`.
    pub fn to_spectral_image(&self) -> Result<SpectralImage, VisualError> {
        let p = self.primary_plane().ok_or(VisualError::Missing("plane"))?;
        let samples = match &p.samples {
            Samples::Packed(t) => t.clone(),
            Samples::Wrapped(code, _) => return Err(bad(format!("primary plane is wrapped under codec '{}' — not a bit-packed plane", *code as char))),
        };
        let (make, model) = match &self.capture {
            Some(c) => (c.make.clone(), c.model.clone()),
            None => (String::new(), String::new()),
        };
        let view = self.default_graph().map(|g| g.to_view_transform()).filter(|v| !v.ops.is_empty());
        Ok(SpectralImage { width: p.width, height: p.height, channels: p.channels.clone(), layout: p.layout.clone(), samples, black: p.black.clone(), white: p.white.clone(), make, model, provenance: self.provenance.clone(), profile: self.characterization.clone(), view })
    }
}
