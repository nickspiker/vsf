//! VSF-Image v0 — spectral-first raw image container. The DNG-shaped problem solved forward: an image is K channels of sensor counts, and each channel carries its own spectral sensitivity curve. A Bayer RAW is K=3 with camera curves; an LED multispectral composite is K=25 with LED×sensor product curves; RGB under CIE 1931 is a *rendering* resolved at read/export time, never the storage model. Store the question, not the answer.
//!
//! Sections written by [`write`] and read back by [`read`]:
//! - `spectral_image` — dims, layout (`mosaic`/`planar`), per-channel black/white, CFA tile (mosaic only), make/model, channel names, and the sensor counts as a [`BitPackedTensor`] at native bit depth (shape `[h, w]` for mosaic, `[k, h, w]` for planar).
//! - `spectral_response` — per-channel sensitivity curves, self-describing grid: `curve_start`/`curve_step`/`curve_counts` per channel plus concatenated `curve_values`. 1nm 350–1100 is the convention, not the law. A channel with count 0 is uncharacterized (present but unmeasured — legacy ingest lands here until a target scan characterizes the device). Section omitted entirely when no channel is characterized.
//! - `provenance` — identity ingredients per the ihi scheme: `handle` (omitted on the wire when empty — "" is the reserved omission value and what an absent field reads back as), `calibration_hash`, `camera_ihi`, `identity`. The composed identity `spaghettify(sensor_plane_hash || camera_calibration_hash || eagle_time || handle_proof || camera_ihi)` is computed by callers that link ihi; this crate just carries the bytes. The VSF header's own BLAKE3 provenance/rolling hashes cover file integrity as always.
//! - `colour_profile` — tiered camera→VSF-RGB characterization (parallel arrays, best entry first) plus the raw questions each answers: verbatim DNG matrices, magic-9 solve patches, calibration provenance. Elected under `spectral_response` (curves always win). Section omitted when uncharacterized. See [`ColourProfile`].
//! - `view_transform` — the translateration log: ordered, named, parameterized view ops (v0: `exposure`) with their IDT class, replayed by downstream tools. The image data is never touched. Section omitted when empty. See [`ViewTransform`].

use crate::decoding::parse::parse;
use crate::file_format::{VsfField, VsfHeader};
use crate::prelude::*;
use crate::types::tensor::{BitPackedTensor, Tensor};
use crate::types::VsfType;
use crate::vsf_builder::VsfBuilder;

/// A spectral sensitivity curve on a self-describing wavelength grid: value `i` is the channel's relative response at `start_nm + i * step_nm`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralCurve {
    pub start_nm: f32,
    pub step_nm: f32,
    pub values: Vec<f32>,
}

/// One measurement channel: a name for humans and, when the device has been characterized, the spectral response that gives its counts physical meaning. `curve: None` = uncharacterized.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralChannel {
    pub name: String,
    pub curve: Option<SpectralCurve>,
}

/// How the sensor counts map to channels.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaneLayout {
    /// One `[h, w]` plane; the CFA tile (shape `[tile_h, tile_w]`, values = channel indices) repeats across the sensor. Bayer RGGB is `[[0,1],[1,2]]`.
    Mosaic { cfa: Tensor<u8> },
    /// K full-resolution planes, shape `[k, h, w]`. Multispectral composites and already-demosaiced sources.
    Planar,
}

/// Identity ingredients for the ihi provenance scheme. All optional in v0; `handle == ""` means omitted (the reserved empty value — an absent wire field reads back as "").
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Provenance {
    pub handle: String,
    pub calibration_hash: Option<[u8; 32]>,
    pub camera_ihi: Option<[u8; 32]>,
    /// The composed ihi identity, computed by callers that link ihi (vsf itself does not).
    pub identity: Option<[u8; 32]>,
}

/// VERICHROME IDT taxonomy — how a colour transform relates the captured light to its output. `Absolute`: straight inversion, cast preserved (a DNG-matrix render). `Relative`: DSR spectral solve to a reference (a chameleon magic-9). `Creative`: deliberate shift (white balance, contrast, colour skew). `Technical`: hue-free mechanics (a scalar exposure). Stable four-value vocabulary — unknown values fail loud on read, per the "we control all clients" rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdtClass {
    Absolute,
    Relative,
    Creative,
    Technical,
}

impl IdtClass {
    pub fn as_str(self) -> &'static str {
        match self {
            IdtClass::Absolute => "absolute",
            IdtClass::Relative => "relative",
            IdtClass::Creative => "creative",
            IdtClass::Technical => "technical",
        }
    }
    fn parse(s: &str) -> Result<Self, SpectralImageError> {
        match s {
            "absolute" => Ok(IdtClass::Absolute),
            "relative" => Ok(IdtClass::Relative),
            "creative" => Ok(IdtClass::Creative),
            "technical" => Ok(IdtClass::Technical),
            other => Err(SpectralImageError::BadField(format!("unknown IDT class '{}'", other))),
        }
    }
}

/// Trust grade of a characterization matrix. `Unit`: measured on THIS camera (a magic-9 target scan). `Model`: factory per-model (a DNG ColorMatrix). `Assumed`: implied by the format convention alone (an sRGB JPEG). This is how "assumed profile, best guess" is first-class rather than a lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileGrade {
    Unit,
    Model,
    Assumed,
}

impl ProfileGrade {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileGrade::Unit => "unit",
            ProfileGrade::Model => "model",
            ProfileGrade::Assumed => "assumed",
        }
    }
    fn parse(s: &str) -> Result<Self, SpectralImageError> {
        match s {
            "unit" => Ok(ProfileGrade::Unit),
            "model" => Ok(ProfileGrade::Model),
            "assumed" => Ok(ProfileGrade::Assumed),
            other => Err(SpectralImageError::BadField(format!("unknown profile grade '{}'", other))),
        }
    }
}

/// The transfer the *samples* carry before this entry's matrix applies. All camera-raw paths are `Linear`; the rest exist so assumed-observer ingest (sRGB/Adobe JPEG) can record the encoding its code values arrived in, to be linearized before the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    Linear,
    Srgb,
    Gamma2,
    Gamma22,
}

impl Transfer {
    pub fn as_str(self) -> &'static str {
        match self {
            Transfer::Linear => "linear",
            Transfer::Srgb => "srgb",
            Transfer::Gamma2 => "gamma2",
            Transfer::Gamma22 => "gamma22",
        }
    }
    fn parse(s: &str) -> Result<Self, SpectralImageError> {
        match s {
            "linear" => Ok(Transfer::Linear),
            "srgb" => Ok(Transfer::Srgb),
            "gamma2" => Ok(Transfer::Gamma2),
            "gamma22" => Ok(Transfer::Gamma22),
            other => Err(SpectralImageError::BadField(format!("unknown transfer '{}'", other))),
        }
    }
}

/// One characterization: a camera→target matrix and the full account of where it came from and how far to trust it. `matrix` is row-major camera→VSF-RGB (linear in, linear out), UNSCALED — no display or illuminant normalization baked in, so the same entry serves any output space by concatenation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEntry {
    pub matrix: [f32; 9],
    /// Free-form derivation token (`magic9`, `dng_colormatrix1`, `assumed_srgb`, …) — open vocabulary; an unknown token still names a working matrix, it just reads opaque.
    pub source: String,
    pub class: IdtClass,
    pub grade: ProfileGrade,
    /// EXIF LightSource code of the characterization illuminant; 0 = unknown. For deriving a display exposure scalar only — never chromatic adaptation.
    pub illuminant: u16,
    pub transfer: Transfer,
}

/// magic-9 calibration provenance carried alongside a `unit`-grade entry: which target produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct CalProvenance {
    pub target_type: u32,
    pub target_serial: u64,
    pub timestamp: String,
}

/// Tiered characterization: the best available mappings from sensor counts to VSF RGB, best entry first, plus the raw questions they answer (verbatim DNG tags, solve patches, calibration provenance) so any reader can audit or re-derive. Elected under `spectral_response` curves, which always win when present. `target` is the destination space every matrix here maps to — v0 legal value `"vsf_rgb"`; a reader seeing any other target treats the file as uncharacterized.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourProfile {
    pub target: String,
    pub entries: Vec<ProfileEntry>,
    /// Verbatim DNG ColorMatrix1/2 (XYZ→camera) + CalibrationIlluminant codes, untouched — the question the derived entries answer.
    pub dng_colormatrix: [Option<([f32; 9], u16)>; 2],
    /// magic-9 solve inputs: (camera-space patch means, reference patch values in VSF RGB), each flat `[p*3]`.
    pub patches: Option<(Vec<f32>, Vec<f32>)>,
    pub cal: Option<CalProvenance>,
}

/// One transformation in the translateration log: a named op, its IDT class, and its parameters. Order in [`ViewTransform::ops`] is application order.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewOp {
    /// Op name — open vocabulary; v0 defines `exposure`. Reserved: `curve`, `contrast`, `skew_matrix`, `white_balance`, `dr_curve`. A reader that meets an unknown op MUST surface it, never silently drop it.
    pub name: String,
    pub class: IdtClass,
    pub params: Vec<f32>,
}

/// The translateration log: ordered view transformations applied after characterization, in `space`. The image DATA is never touched — this is interpretation metadata that downstream tools (oriel) replay. `space` v0 value `"vsf_rgb_linear"` (after the elected matrix, before display encode).
#[derive(Debug, Clone, PartialEq)]
pub struct ViewTransform {
    pub space: String,
    pub ops: Vec<ViewOp>,
}

/// A spectral-first raw image: K channels of sensor counts plus what those counts *mean*.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralImage {
    pub width: usize,
    pub height: usize,
    pub channels: Vec<SpectralChannel>,
    pub layout: PlaneLayout,
    /// Sensor counts, bitpacked at native depth. Mosaic: shape `[height, width]`. Planar: `[channels, height, width]`.
    pub samples: BitPackedTensor,
    /// Per-channel black level in raw counts.
    pub black: Vec<f32>,
    /// Per-channel white (saturation) level in raw counts.
    pub white: Vec<f32>,
    pub make: String,
    pub model: String,
    pub provenance: Provenance,
    /// Tiered camera→VSF-RGB characterization; `None` = uncharacterized (render raw-camera). See [`ColourProfile`].
    pub profile: Option<ColourProfile>,
    /// The translateration log; `None` = no view ops recorded. See [`ViewTransform`].
    pub view: Option<ViewTransform>,
}

#[derive(Debug)]
pub enum SpectralImageError {
    Parse(String),
    MissingSection(&'static str),
    MissingField(&'static str),
    BadField(String),
}

impl core::fmt::Display for SpectralImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpectralImageError::Parse(s) => write!(f, "VSF parse: {}", s),
            SpectralImageError::MissingSection(s) => write!(f, "no '{}' section in file", s),
            SpectralImageError::MissingField(s) => write!(f, "missing field '{}'", s),
            SpectralImageError::BadField(s) => write!(f, "bad field: {}", s),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SpectralImageError {}

impl SpectralImage {
    /// Total channel count K.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Native bit depth of the sensor counts.
    pub fn bit_depth(&self) -> u8 {
        self.samples.bit_depth
    }
}

/// Serialize to a complete VSF file (header + sections + BLAKE3 integrity, all handled by [`VsfBuilder`]).
pub fn write(img: &SpectralImage) -> Result<Vec<u8>, String> {
    let k = img.channels.len();
    if img.black.len() != k || img.white.len() != k {
        return Err(format!("black/white length {}/{} != channel count {}", img.black.len(), img.white.len(), k));
    }

    let mut fields: Vec<(String, VsfType)> = vec![
        ("width".to_string(), VsfType::u(img.width, false)),
        ("height".to_string(), VsfType::u(img.height, false)),
        ("channel_count".to_string(), VsfType::u(k, false)),
        ("black".to_string(), VsfType::t_f5(Tensor::new(vec![k], img.black.clone()))),
        ("white".to_string(), VsfType::t_f5(Tensor::new(vec![k], img.white.clone()))),
        ("channel_names".to_string(), VsfType::a(img.channels.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\n"))),
    ];
    match &img.layout {
        PlaneLayout::Mosaic { cfa } => {
            fields.push(("layout".to_string(), VsfType::a("mosaic".to_string())));
            fields.push(("cfa".to_string(), VsfType::t_u3(cfa.clone())));
        }
        PlaneLayout::Planar => fields.push(("layout".to_string(), VsfType::a("planar".to_string()))),
    }
    if !img.make.is_empty() {
        fields.push(("make".to_string(), VsfType::a(img.make.clone())));
    }
    if !img.model.is_empty() {
        fields.push(("model".to_string(), VsfType::a(img.model.clone())));
    }
    fields.push(("samples".to_string(), VsfType::p(img.samples.clone())));

    let mut builder = VsfBuilder::new().add_section("spectral_image", fields);

    // Spectral response — only when at least one channel is characterized. Uncharacterized channels in a mixed set carry count 0 and contribute nothing to curve_values.
    if img.channels.iter().any(|c| c.curve.is_some()) {
        let mut starts = Vec::with_capacity(k);
        let mut steps = Vec::with_capacity(k);
        let mut counts: Vec<u32> = Vec::with_capacity(k);
        let mut values: Vec<f32> = Vec::new();
        for c in &img.channels {
            match &c.curve {
                Some(curve) => {
                    starts.push(curve.start_nm);
                    steps.push(curve.step_nm);
                    counts.push(curve.values.len() as u32);
                    values.extend_from_slice(&curve.values);
                }
                None => {
                    starts.push(0.0);
                    steps.push(0.0);
                    counts.push(0);
                }
            }
        }
        let total = values.len();
        builder = builder.add_section(
            "spectral_response",
            vec![
                ("curve_start".to_string(), VsfType::t_f5(Tensor::new(vec![k], starts))),
                ("curve_step".to_string(), VsfType::t_f5(Tensor::new(vec![k], steps))),
                ("curve_counts".to_string(), VsfType::t_u5(Tensor::new(vec![k], counts))),
                ("curve_values".to_string(), VsfType::t_f5(Tensor::new(vec![total], values))),
            ],
        );
    }

    // Provenance — every field individually optional; the section itself is omitted when it would be empty. An empty handle is *not* written: "" is the reserved omission value and is exactly what read() yields for the absent field.
    let p = &img.provenance;
    let mut prov: Vec<(String, VsfType)> = Vec::new();
    if !p.handle.is_empty() {
        // Handles are Unicode and canonically Huffman-encoded (x) — that's what ihi hashes. Without the text feature compiled in, fall back to ASCII (a) and reject what it can't hold rather than silently mangling.
        #[cfg(any(feature = "text", feature = "text-encode"))]
        prov.push(("handle".to_string(), VsfType::x(p.handle.clone())));
        #[cfg(not(any(feature = "text", feature = "text-encode")))]
        {
            if !p.handle.is_ascii() {
                return Err("non-ASCII handle requires the 'text' or 'text-encode' feature".to_string());
            }
            prov.push(("handle".to_string(), VsfType::a(p.handle.clone())));
        }
    }
    if let Some(h) = &p.calibration_hash {
        prov.push(("calibration_hash".to_string(), VsfType::hp(h.to_vec())));
    }
    if let Some(h) = &p.camera_ihi {
        prov.push(("camera_ihi".to_string(), VsfType::hs(h.to_vec())));
    }
    if let Some(h) = &p.identity {
        prov.push(("identity".to_string(), VsfType::hs(h.to_vec())));
    }
    if !prov.is_empty() {
        builder = builder.add_section("provenance", prov);
    }

    // Colour profile — tiered characterization. Omitted entirely when uncharacterized. Entries become parallel arrays (best first); the raw-question fields ride alongside, each individually optional.
    if let Some(profile) = &img.profile {
        let n = profile.entries.len();
        let mut matrices: Vec<f32> = Vec::with_capacity(n * 9);
        let mut sources: Vec<&str> = Vec::with_capacity(n);
        let mut classes: Vec<&str> = Vec::with_capacity(n);
        let mut grades: Vec<&str> = Vec::with_capacity(n);
        let mut illuminants: Vec<u16> = Vec::with_capacity(n);
        let mut transfers: Vec<&str> = Vec::with_capacity(n);
        for e in &profile.entries {
            matrices.extend_from_slice(&e.matrix);
            sources.push(&e.source);
            classes.push(e.class.as_str());
            grades.push(e.grade.as_str());
            illuminants.push(e.illuminant);
            transfers.push(e.transfer.as_str());
        }
        let mut fields: Vec<(String, VsfType)> = vec![
            ("target".to_string(), VsfType::a(profile.target.clone())),
            ("count".to_string(), VsfType::u(n, false)),
            ("matrices".to_string(), VsfType::t_f5(Tensor::new(vec![n, 3, 3], matrices))),
            ("sources".to_string(), VsfType::a(sources.join("\n"))),
            ("classes".to_string(), VsfType::a(classes.join("\n"))),
            ("grades".to_string(), VsfType::a(grades.join("\n"))),
            ("illuminants".to_string(), VsfType::t_u4(Tensor::new(vec![n], illuminants))),
            ("transfers".to_string(), VsfType::a(transfers.join("\n"))),
        ];
        for (i, name) in ["dng_colormatrix1", "dng_colormatrix2"].iter().enumerate() {
            if let Some((m, code)) = &profile.dng_colormatrix[i] {
                fields.push((name.to_string(), VsfType::t_f5(Tensor::new(vec![3, 3], m.to_vec()))));
                let illum_name = if i == 0 { "dng_illuminant1" } else { "dng_illuminant2" };
                fields.push((illum_name.to_string(), VsfType::u(*code as usize, false)));
            }
        }
        if let Some((cam, reference)) = &profile.patches {
            let p = cam.len() / 3;
            fields.push(("patches_camera".to_string(), VsfType::t_f5(Tensor::new(vec![p, 3], cam.clone()))));
            fields.push(("patches_reference".to_string(), VsfType::t_f5(Tensor::new(vec![p, 3], reference.clone()))));
        }
        if let Some(cal) = &profile.cal {
            fields.push(("cal_target_type".to_string(), VsfType::u(cal.target_type as usize, false)));
            fields.push(("cal_target_serial".to_string(), VsfType::u(cal.target_serial as usize, false)));
            fields.push(("cal_timestamp".to_string(), VsfType::a(cal.timestamp.clone())));
        }
        builder = builder.add_section("colour_profile", fields);
    }

    // View transform — the translateration log. Omitted when empty. Ops become parallel arrays; params concatenate, sliced back by param_counts.
    if let Some(view) = &img.view {
        let m = view.ops.len();
        let mut names: Vec<&str> = Vec::with_capacity(m);
        let mut classes: Vec<&str> = Vec::with_capacity(m);
        let mut param_counts: Vec<u32> = Vec::with_capacity(m);
        let mut params: Vec<f32> = Vec::new();
        for op in &view.ops {
            names.push(&op.name);
            classes.push(op.class.as_str());
            param_counts.push(op.params.len() as u32);
            params.extend_from_slice(&op.params);
        }
        let total = params.len();
        builder = builder.add_section(
            "view_transform",
            vec![
                ("space".to_string(), VsfType::a(view.space.clone())),
                ("ops".to_string(), VsfType::a(names.join("\n"))),
                ("classes".to_string(), VsfType::a(classes.join("\n"))),
                ("param_counts".to_string(), VsfType::t_u5(Tensor::new(vec![m], param_counts))),
                ("params".to_string(), VsfType::t_f5(Tensor::new(vec![total], params))),
            ],
        );
    }

    builder.build()
}

/// Parse a VSF-Image file back into a [`SpectralImage`].
pub fn read(data: &[u8]) -> Result<SpectralImage, SpectralImageError> {
    let (header, _) = VsfHeader::decode(data).map_err(SpectralImageError::Parse)?;

    let main = section_fields(data, &header, "spectral_image")?.ok_or(SpectralImageError::MissingSection("spectral_image"))?;

    let width = take_usize(&main, "width")?;
    let height = take_usize(&main, "height")?;
    let k = take_usize(&main, "channel_count")?;
    let black = take_f32_vec(&main, "black")?;
    let white = take_f32_vec(&main, "white")?;
    let names_joined = take_string(&main, "channel_names")?;
    let layout_str = take_string(&main, "layout")?;
    let make = take_string_opt(&main, "make").unwrap_or_default();
    let model = take_string_opt(&main, "model").unwrap_or_default();

    let samples = match find(&main, "samples") {
        Some(VsfType::p(t)) => t.clone(),
        Some(_) => return Err(SpectralImageError::BadField("samples is not a BitPackedTensor".into())),
        None => return Err(SpectralImageError::MissingField("samples")),
    };

    let layout = match layout_str.as_str() {
        "mosaic" => match find(&main, "cfa") {
            Some(VsfType::t_u3(t)) => PlaneLayout::Mosaic { cfa: t.clone() },
            Some(_) => return Err(SpectralImageError::BadField("cfa is not a u8 tensor".into())),
            None => return Err(SpectralImageError::MissingField("cfa")),
        },
        "planar" => PlaneLayout::Planar,
        other => return Err(SpectralImageError::BadField(format!("unknown layout '{}'", other))),
    };

    // Channel names: '\n'-joined. write() emits exactly k names; a mismatch means the file is corrupt or hand-forged — fail loud, don't invent labels that would mask it.
    let names: Vec<String> = names_joined.split('\n').map(str::to_string).collect();
    if names.len() != k {
        return Err(SpectralImageError::BadField(format!("channel_names carries {} names for {} channels", names.len(), k)));
    }

    // Curves, if the file carries a spectral_response section.
    let mut curves: Vec<Option<SpectralCurve>> = vec![None; k];
    if let Some(resp) = section_fields(data, &header, "spectral_response")? {
        let starts = take_f32_vec(&resp, "curve_start")?;
        let steps = take_f32_vec(&resp, "curve_step")?;
        let counts = take_u32_vec(&resp, "curve_counts")?;
        let values = take_f32_vec(&resp, "curve_values")?;
        if starts.len() != k || steps.len() != k || counts.len() != k {
            return Err(SpectralImageError::BadField(format!("spectral_response arrays sized {}/{}/{} for {} channels", starts.len(), steps.len(), counts.len(), k)));
        }
        let mut cursor = 0usize;
        for i in 0..k {
            let n = counts[i] as usize;
            if n == 0 {
                continue;
            }
            if cursor + n > values.len() {
                return Err(SpectralImageError::BadField("curve_values shorter than curve_counts total".into()));
            }
            curves[i] = Some(SpectralCurve { start_nm: starts[i], step_nm: steps[i], values: values[cursor..cursor + n].to_vec() });
            cursor += n;
        }
    }

    let channels = names.into_iter().zip(curves).map(|(name, curve)| SpectralChannel { name, curve }).collect();

    // Provenance — absent section or absent fields read back as the defaults ("" handle = omitted, None hashes).
    let mut provenance = Provenance::default();
    if let Some(prov) = section_fields(data, &header, "provenance")? {
        if let Some(h) = take_string_opt(&prov, "handle") {
            provenance.handle = h;
        }
        provenance.calibration_hash = take_hash32(&prov, "calibration_hash");
        provenance.camera_ihi = take_hash32(&prov, "camera_ihi");
        provenance.identity = take_hash32(&prov, "identity");
    }

    if black.len() != k || white.len() != k {
        return Err(SpectralImageError::BadField(format!("black/white length {}/{} != channel count {}", black.len(), white.len(), k)));
    }

    let profile = read_colour_profile(data, &header)?;
    let view = read_view_transform(data, &header)?;

    Ok(SpectralImage { width, height, channels, layout, samples, black, white, make, model, provenance, profile, view })
}

/// Split a '\n'-joined field into exactly `n` parts, failing loud on a count mismatch — a corrupt or hand-forged parallel array must not silently desync from its siblings.
fn split_n(joined: &str, n: usize, what: &str) -> Result<Vec<String>, SpectralImageError> {
    let parts: Vec<String> = joined.split('\n').map(str::to_string).collect();
    if parts.len() != n {
        return Err(SpectralImageError::BadField(format!("{} carries {} entries for {} rows", what, parts.len(), n)));
    }
    Ok(parts)
}

/// Parse the optional `colour_profile` section into a [`ColourProfile`]. Absent section ⇒ `None` (uncharacterized). Parallel arrays are length-checked against `count`; the matrices tensor against `count*9`.
fn read_colour_profile(data: &[u8], header: &VsfHeader) -> Result<Option<ColourProfile>, SpectralImageError> {
    let Some(sec) = section_fields(data, header, "colour_profile")? else {
        return Ok(None);
    };
    let target = take_string(&sec, "target")?;
    let n = take_usize(&sec, "count")?;
    let matrices = take_f32_vec(&sec, "matrices")?;
    if matrices.len() != n * 9 {
        return Err(SpectralImageError::BadField(format!("colour_profile matrices carries {} values for {} entries", matrices.len(), n)));
    }
    let sources = split_n(&take_string(&sec, "sources")?, n, "colour_profile sources")?;
    let classes = split_n(&take_string(&sec, "classes")?, n, "colour_profile classes")?;
    let grades = split_n(&take_string(&sec, "grades")?, n, "colour_profile grades")?;
    let transfers = split_n(&take_string(&sec, "transfers")?, n, "colour_profile transfers")?;
    let illuminants = take_u32_vec(&sec, "illuminants")?;
    if illuminants.len() != n {
        return Err(SpectralImageError::BadField(format!("colour_profile illuminants carries {} values for {} entries", illuminants.len(), n)));
    }
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let mut matrix = [0f32; 9];
        matrix.copy_from_slice(&matrices[i * 9..i * 9 + 9]);
        entries.push(ProfileEntry {
            matrix,
            source: sources[i].clone(),
            class: IdtClass::parse(&classes[i])?,
            grade: ProfileGrade::parse(&grades[i])?,
            illuminant: illuminants[i] as u16,
            transfer: Transfer::parse(&transfers[i])?,
        });
    }

    let mut dng_colormatrix: [Option<([f32; 9], u16)>; 2] = [None, None];
    for (i, (mname, iname)) in [("dng_colormatrix1", "dng_illuminant1"), ("dng_colormatrix2", "dng_illuminant2")].iter().enumerate() {
        if let Some(f) = sec.iter().find(|f| f.name == *mname) {
            let flat = match f.values.first() {
                Some(VsfType::t_f5(t)) => t.data.clone(),
                Some(VsfType::v_f5(v)) => v.data.clone(),
                _ => return Err(SpectralImageError::BadField(format!("{} is not an f32 tensor", mname))),
            };
            if flat.len() != 9 {
                return Err(SpectralImageError::BadField(format!("{} carries {} values, need 9", mname, flat.len())));
            }
            let mut m = [0f32; 9];
            m.copy_from_slice(&flat);
            let code = take_usize(&sec, iname).unwrap_or(0) as u16;
            dng_colormatrix[i] = Some((m, code));
        }
    }

    let patches = match (sec.iter().any(|f| f.name == "patches_camera"), sec.iter().any(|f| f.name == "patches_reference")) {
        (true, true) => Some((take_f32_vec(&sec, "patches_camera")?, take_f32_vec(&sec, "patches_reference")?)),
        (false, false) => None,
        _ => return Err(SpectralImageError::BadField("colour_profile has one of patches_camera/patches_reference without the other".into())),
    };

    let cal = if sec.iter().any(|f| f.name == "cal_target_type") {
        Some(CalProvenance {
            target_type: take_usize(&sec, "cal_target_type")? as u32,
            target_serial: take_usize(&sec, "cal_target_serial")? as u64,
            timestamp: take_string(&sec, "cal_timestamp")?,
        })
    } else {
        None
    };

    Ok(Some(ColourProfile { target, entries, dng_colormatrix, patches, cal }))
}

/// Parse the optional `view_transform` section into a [`ViewTransform`]. Absent section ⇒ `None`. Params are sliced back by param_counts; a shortfall fails loud.
fn read_view_transform(data: &[u8], header: &VsfHeader) -> Result<Option<ViewTransform>, SpectralImageError> {
    let Some(sec) = section_fields(data, header, "view_transform")? else {
        return Ok(None);
    };
    let space = take_string(&sec, "space")?;
    let param_counts = take_u32_vec(&sec, "param_counts")?;
    let m = param_counts.len();
    let names = split_n(&take_string(&sec, "ops")?, m, "view_transform ops")?;
    let classes = split_n(&take_string(&sec, "classes")?, m, "view_transform classes")?;
    let params = take_f32_vec(&sec, "params")?;
    let mut ops = Vec::with_capacity(m);
    let mut cursor = 0usize;
    for i in 0..m {
        let count = param_counts[i] as usize;
        if cursor + count > params.len() {
            return Err(SpectralImageError::BadField("view_transform params shorter than param_counts total".into()));
        }
        ops.push(ViewOp {
            name: names[i].clone(),
            class: IdtClass::parse(&classes[i])?,
            params: params[cursor..cursor + count].to_vec(),
        });
        cursor += count;
    }
    Ok(Some(ViewTransform { space, ops }))
}

/// Walk a named section's body and return its fields, or Ok(None) when the section isn't in the file. Mirrors the section-body walk in [`crate::image::decode`].
fn section_fields(data: &[u8], header: &VsfHeader, name: &str) -> Result<Option<Vec<VsfField>>, SpectralImageError> {
    let Some(field) = header.fields.iter().find(|f| f.name == name) else {
        return Ok(None);
    };
    let mut p = field.offset_bytes;
    if p >= data.len() {
        return Err(SpectralImageError::Parse(format!("'{}' section offset {} beyond file length {}", name, p, data.len())));
    }
    if data[p] == b'>' {
        p += 1;
    }
    if p >= data.len() || data[p] != b'[' {
        return Err(SpectralImageError::Parse(format!("expected '[' at '{}' section start, got byte {:02x}", name, data.get(p).copied().unwrap_or(0))));
    }
    p += 1;
    if p < data.len() && data[p] != b'(' {
        parse(data, &mut p).map_err(|e| SpectralImageError::Parse(format!("section name: {:?}", e)))?;
        parse(data, &mut p).map_err(|e| SpectralImageError::Parse(format!("section n: {:?}", e)))?;
        parse(data, &mut p).map_err(|e| SpectralImageError::Parse(format!("section b: {:?}", e)))?;
    }
    let mut fields = Vec::with_capacity(field.child_count);
    for _ in 0..field.child_count {
        fields.push(VsfField::parse(data, &mut p).map_err(|e| SpectralImageError::Parse(format!("field parse: {}", e)))?);
    }
    Ok(Some(fields))
}

fn find<'a>(fields: &'a [VsfField], name: &str) -> Option<&'a VsfType> {
    fields.iter().find(|f| f.name == name).and_then(|f| f.values.first())
}

fn take_usize(fields: &[VsfField], name: &'static str) -> Result<usize, SpectralImageError> {
    match find(fields, name) {
        Some(VsfType::u(v, _)) => Ok(*v),
        Some(VsfType::u3(v)) => Ok(*v as usize),
        Some(VsfType::u4(v)) => Ok(*v as usize),
        Some(VsfType::u5(v)) => Ok(*v as usize),
        Some(VsfType::u6(v)) => Ok(*v as usize),
        Some(VsfType::n(v)) => Ok(*v),
        Some(_) => Err(SpectralImageError::BadField(format!("'{}' is not an unsigned integer", name))),
        None => Err(SpectralImageError::MissingField(name)),
    }
}

fn take_string(fields: &[VsfField], name: &'static str) -> Result<String, SpectralImageError> {
    take_string_opt(fields, name).ok_or(SpectralImageError::MissingField(name))
}

fn take_string_opt(fields: &[VsfField], name: &str) -> Option<String> {
    match find(fields, name) {
        Some(VsfType::a(s)) | Some(VsfType::x(s)) => Some(s.clone()),
        _ => None,
    }
}

fn take_f32_vec(fields: &[VsfField], name: &'static str) -> Result<Vec<f32>, SpectralImageError> {
    // 1D tensors take the compact `tn` wire form and parse back as Vector variants, so accept both shapes of the same data.
    match find(fields, name) {
        Some(VsfType::t_f5(t)) => Ok(t.data.clone()),
        Some(VsfType::v_f5(v)) => Ok(v.data.clone()),
        Some(_) => Err(SpectralImageError::BadField(format!("'{}' is not an f32 tensor", name))),
        None => Err(SpectralImageError::MissingField(name)),
    }
}

fn take_u32_vec(fields: &[VsfField], name: &'static str) -> Result<Vec<u32>, SpectralImageError> {
    // 1D unsigned tensors come back as Vector variants (compact `tn` wire form); accept tensor and vector forms at any width and widen to u32.
    match find(fields, name) {
        Some(VsfType::t_u3(t)) => Ok(t.data.iter().map(|&v| v as u32).collect()),
        Some(VsfType::t_u4(t)) => Ok(t.data.iter().map(|&v| v as u32).collect()),
        Some(VsfType::t_u5(t)) => Ok(t.data.clone()),
        Some(VsfType::v_u3(v)) => Ok(v.data.iter().map(|&v| v as u32).collect()),
        Some(VsfType::v_u4(v)) => Ok(v.data.iter().map(|&v| v as u32).collect()),
        Some(VsfType::v_u5(v)) => Ok(v.data.clone()),
        Some(_) => Err(SpectralImageError::BadField(format!("'{}' is not an unsigned tensor", name))),
        None => Err(SpectralImageError::MissingField(name)),
    }
}

fn take_hash32(fields: &[VsfField], name: &str) -> Option<[u8; 32]> {
    match find(fields, name) {
        Some(VsfType::hp(v)) | Some(VsfType::hs(v)) | Some(VsfType::hb(v)) => v.as_slice().try_into().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bayer_test_image() -> SpectralImage {
        // 4×4 RGGB mosaic, 12-bit counts.
        let counts: Vec<u16> = (0..16).map(|i| (i * 200) as u16).collect();
        SpectralImage {
            width: 4,
            height: 4,
            channels: vec![
                SpectralChannel { name: "R".into(), curve: None },
                SpectralChannel { name: "G".into(), curve: None },
                SpectralChannel { name: "B".into(), curve: None },
            ],
            layout: PlaneLayout::Mosaic { cfa: Tensor::new(vec![2, 2], vec![0, 1, 1, 2]) },
            samples: BitPackedTensor::pack(12, vec![4, 4], &counts),
            black: vec![512.0; 3],
            white: vec![4095.0; 3],
            make: "VERICHROME".into(),
            model: "TestCam".into(),
            provenance: Provenance::default(),
            profile: None,
            view: None,
        }
    }

    #[test]
    fn mosaic_round_trip() {
        let img = bayer_test_image();
        let bytes = write(&img).expect("write");
        let back = read(&bytes).expect("read");
        assert_eq!(back, img);
        assert_eq!(back.samples.unpack_u16(), img.samples.unpack_u16());
        assert_eq!(back.bit_depth(), 12);
    }

    #[test]
    fn planar_with_curves_round_trip() {
        // 2 channels × 2×2, one characterized on a 5nm grid and one uncharacterized — the mixed case.
        let img = SpectralImage {
            width: 2,
            height: 2,
            channels: vec![
                SpectralChannel { name: "LED_450".into(), curve: Some(SpectralCurve { start_nm: 400.0, step_nm: 5.0, values: vec![0.1, 0.9, 0.4] }) },
                SpectralChannel { name: "LED_650".into(), curve: None },
            ],
            layout: PlaneLayout::Planar,
            samples: BitPackedTensor::pack(16, vec![2, 2, 2], &[1u16, 2, 3, 4, 5, 6, 7, 8]),
            black: vec![0.0, 0.0],
            white: vec![65535.0, 65535.0],
            make: String::new(),
            model: String::new(),
            provenance: Provenance { handle: "nick".into(), calibration_hash: Some([7u8; 32]), camera_ihi: None, identity: Some([9u8; 32]) },
            profile: None,
            view: None,
        };
        let bytes = write(&img).expect("write");
        let back = read(&bytes).expect("read");
        assert_eq!(back, img);
    }

    #[test]
    fn colour_profile_and_view_round_trip() {
        // Two-entry profile (magic-9 unit + DNG model), verbatim DNG tags, solve patches, cal provenance, plus a one-op view log.
        let mut img = bayer_test_image();
        img.profile = Some(ColourProfile {
            target: "vsf_rgb".into(),
            entries: vec![
                ProfileEntry {
                    matrix: [1.1, 0.0, 0.0, 0.0, 0.9, 0.0, 0.0, 0.0, 1.2],
                    source: "magic9".into(),
                    class: IdtClass::Relative,
                    grade: ProfileGrade::Unit,
                    illuminant: 21,
                    transfer: Transfer::Linear,
                },
                ProfileEntry {
                    matrix: [0.8, 0.1, 0.05, 0.02, 0.95, 0.03, 0.01, 0.04, 1.1],
                    source: "dng_colormatrix1".into(),
                    class: IdtClass::Absolute,
                    grade: ProfileGrade::Model,
                    illuminant: 23,
                    transfer: Transfer::Linear,
                },
            ],
            dng_colormatrix: [Some(([1.0, 0.1, 0.0, 0.0, 1.0, 0.1, 0.0, 0.0, 1.0], 23)), None],
            patches: Some((vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], vec![0.11, 0.21, 0.31, 0.41, 0.51, 0.61])),
            cal: Some(CalProvenance { target_type: 9, target_serial: 12345, timestamp: "eagle".into() }),
        });
        img.view = Some(ViewTransform {
            space: "vsf_rgb_linear".into(),
            ops: vec![ViewOp { name: "exposure".into(), class: IdtClass::Technical, params: vec![1.5] }],
        });
        let bytes = write(&img).expect("write");
        let back = read(&bytes).expect("read");
        assert_eq!(back, img);
    }

    #[test]
    fn absent_profile_and_view_read_none() {
        let img = bayer_test_image();
        let back = read(&write(&img).expect("write")).expect("read");
        assert!(back.profile.is_none());
        assert!(back.view.is_none());
    }

    #[test]
    fn empty_handle_is_omitted_and_reads_back_empty() {
        let img = bayer_test_image();
        let bytes = write(&img).expect("write");
        assert!(!String::from_utf8_lossy(&bytes).contains("handle"));
        assert_eq!(read(&bytes).expect("read").provenance.handle, "");
    }
}
