//! VSF visual data — one container for stills, video, audio, layers and provenance. This module is the still-image slice of it (the plan's M1): tracks, spans, planes, the capture facts, the characterization, a graph and a view, and the foreign metadata a conversion must not lose.
//!
//! A still is the one-frame case of a beam track: `visual` + `track.b0` + `span.b0.s0` (one frame, naming a plane) + `plane.b0.s0.sensor` + `capture` + `characterization` + `graph.g0` + `view.v0`. Video adds a rate declaration and spans whose frames are inline; nothing about a still changes when it does. Every section is written with a content hash in its table-of-contents entry, so each can be checked — and later signed — on its own.
//!
//! The normative field tables live in `idt/image.md`; this module's doc comments are the implementation's own account. What a reader does with absence: no `characterization` and no `primaries` ⇒ the samples are VSF RGB; no `prior` ⇒ nothing was done to the samples before the file; no `graph` ⇒ render the sensor plane through `black_white → idt(0)`; a `plane` or `view` with no `encoding` ⇒ `gamma2`, always (the integer-plane rule of `colour.md`) — which is why every writer here emits `encoding`, and why the legacy `spectral_image` section reads as `linear`: its only writer ever was opsin, writing linear counts.
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
/// A closed vocabulary: each variant writes its first spelling and reads that or any alias after `|`.
macro_rules! closed_vocab {
    ($name:ident { $($variant:ident => $s:literal $(| $alias:literal)*),+ $(,)? }, $what:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }
            pub fn parse(s: &str) -> Result<Self, VisualError> {
                match s { $($s $(| $alias)* => Ok($name::$variant),)+ other => Err(bad(format!("unknown {} '{}'", $what, other))) }
            }
        }
    };
}

closed_vocab!(TrackKind { Beam => "beam", Wave => "wave", Timecode => "timecode", Annotation => "annotation", Data => "data" }, "track kind");
closed_vocab!(TrackRole { Capture => "capture", Derived => "derived", Display => "display", Preview => "preview", Calibration => "calibration" }, "track role");
closed_vocab!(PlaneRole { Sensor => "sensor", Display => "display", Preview => "preview", Depth => "depth", Ir => "ir", Uv => "uv", Confidence => "confidence", Motion => "motion", Matte => "matte", Mask => "mask", Alpha => "alpha", Dark => "dark", Flat => "flat", Bias => "bias", Derived => "derived" }, "plane role");
closed_vocab!(Encoding {
    Linear => "linear", Gamma2 => "gamma2", Gamma2Tanh => "g2t" | "gamma2tanh",
    Srgb => "srgb", Bt1886 => "bt1886", AdobeRgb => "adobe_rgb", Prophoto => "prophoto", Pq => "pq", Hlg => "hlg",
    Gamma18 => "gamma18", Gamma22 => "gamma22", Gamma26 => "gamma26", Bt709 => "bt709", Smpte240m => "smpte240m", Dcdm => "dcdm",
    Slog => "slog", Slog2 => "slog2", Slog3 => "slog3", Logc3 => "logc3", Logc4 => "logc4", Vlog => "vlog",
    Clog => "clog", Clog2 => "clog2", Clog3 => "clog3", Flog => "flog", Flog2 => "flog2", Nlog => "nlog", Dlog => "dlog", Llog => "llog",
    AppleLog => "apple_log", MiLog => "mi_log", Log3g10 => "log3g10", Log3g12 => "log3g12", Acescc => "acescc", Acescct => "acescct",
    Cineon => "cineon", Protune => "protune", Tlog => "tlog", DavinciIntermediate => "davinci_intermediate", BmdFilmGen5 => "bmd_film_gen5",
}, "encoding");
closed_vocab!(Primaries {
    Vsf => "vsf", Srgb => "srgb", DisplayP3 => "display_p3", DciP3 => "dci_p3", AdobeRgb => "adobe_rgb", AdobeWideGamut => "adobe_wide_gamut",
    Prophoto => "prophoto", Rec2020 => "rec2020" | "f_gamut" | "n_gamut", Bt601Pal => "bt601_625", Bt601Ntsc => "bt601_525", Ntsc1953 => "ntsc1953",
    AcesAp0 => "aces_ap0", AcesAp1 => "aces_ap1",
    SGamut3 => "s_gamut3" | "s_gamut", SGamut3Cine => "s_gamut3_cine", VeniceSGamut3 => "venice_s_gamut3", VeniceSGamut3Cine => "venice_s_gamut3_cine",
    ArriWideGamut3 => "arri_wide_gamut3", ArriWideGamut4 => "arri_wide_gamut4", VGamut => "v_gamut", CinemaGamut => "cinema_gamut",
    RedWideGamut => "red_wide_gamut", DavinciWideGamut => "davinci_wide_gamut", BlackmagicWideGamut => "blackmagic_wide_gamut",
    EGamut => "e_gamut", EGamut2 => "e_gamut2", DGamut => "d_gamut", ProtuneNative => "protune_native",
    AppleRgb => "apple_rgb", ColormatchRgb => "colormatch_rgb", EciRgbV2 => "eci_rgb_v2", CieRgb => "cie_rgb", Xyz => "xyz",
}, "primaries");
closed_vocab!(Purpose { Default => "default", Preview => "preview", Print => "print", Technical => "technical" }, "view purpose");

/// What light a code holds. VSF's own encodings (`linear`, `gamma2`, `g2t`) quantise ×2ⁿ with a code at the bottom of its bucket. The rest are a source's own transfer, kept so a plane can hold a JPEG, a TIFF, an AVIF, a film scan or camera log footage exactly as it was written, each named for the specification that defines it:
/// - stills and displays: `srgb` (IEC 61966-2-1, also Display P3's), `bt1886` (BT.709/BT.2020 SDR reference display), `adobe_rgb` (563/256), `prophoto` (ROMM RGB), `gamma18`, `gamma22`, `gamma26`, `smpte240m`, `dcdm` (cinema X'Y'Z', white at 48 cd/m²);
/// - HDR: `pq`, `hlg` (BT.2100, diffuse white per BT.2408);
/// - cameras: `bt709` (the BT.709/601 camera curve, scene light), `slog`, `slog2`, `slog3`, `logc3` (EI 800), `logc4`, `vlog`, `clog`, `clog2`, `clog3`, `flog`, `flog2`, `nlog`, `dlog`, `llog`, `apple_log`, `mi_log`, `log3g10`, `log3g12`, `protune`, `tlog`, `bmd_film_gen5`;
/// - grading and film: `acescc`, `acescct`, `davinci_intermediate`, `cineon`.
///
/// Every one decodes to light with 1.0 at diffuse white (for a camera log, a perfect diffuse reflector), so light above white is kept. A source encoding's signal is its code normalised by the plane's `black` and `white` (`Plane::encoded` sets 0 and the top code); the logs expect the normalised code value their specification defines, so an importer leaves black 0 and white at the top code for them too.
impl Encoding {
    /// One of VSF's own encodings, quantised ×2ⁿ, bucket bottom.
    pub fn is_vsf(self) -> bool {
        matches!(self, Encoding::Linear | Encoding::Gamma2 | Encoding::Gamma2Tanh)
    }

    /// Light (diffuse white = 1.0) of a signal 0 to 1 in this encoding. `linear` is the signal itself; `g2t`, `pq` and `hlg` return light above 1 for their highlights. A `g2t` signal of 1 would be infinite light and is taken as the largest finite one.
    #[allow(deprecated)]
    pub fn light_f32(self, signal: f32) -> f32 {
        use crate::colour::{convert, legacy::transfer as legacy, transfer};
        match self {
            Encoding::Linear => signal.max(0.),
            Encoding::Gamma2 => {
                let e = signal.clamp(0., 1.);
                e * e
            }
            Encoding::Gamma2Tanh => convert::linearize_gamma2tanh_f32(signal.clamp(0., 1. - f32::EPSILON)),
            Encoding::Srgb => legacy::srgb_eotf(signal.clamp(0., 1.)),
            Encoding::Bt1886 => transfer::bt1886_light(signal),
            Encoding::AdobeRgb => legacy::adobe_rgb_eotf(signal.clamp(0., 1.)),
            Encoding::Prophoto => transfer::prophoto_light(signal),
            Encoding::Pq => transfer::pq_light(signal),
            Encoding::Hlg => transfer::hlg_light(signal),
            Encoding::Gamma18 => transfer::gamma_light(signal, 1.8),
            Encoding::Gamma22 => transfer::gamma_light(signal, 2.2),
            Encoding::Gamma26 => transfer::gamma_light(signal, 2.6),
            Encoding::Bt709 => transfer::bt709_scene_light(signal),
            Encoding::Smpte240m => transfer::smpte240m_light(signal),
            Encoding::Dcdm => transfer::dcdm_light(signal),
            Encoding::Slog => transfer::slog_light(signal),
            Encoding::Slog2 => transfer::slog2_light(signal),
            Encoding::Slog3 => transfer::slog3_light(signal),
            Encoding::Logc3 => transfer::logc3_light(signal),
            Encoding::Logc4 => transfer::logc4_light(signal),
            Encoding::Vlog => transfer::vlog_light(signal),
            Encoding::Clog => transfer::clog_light(signal),
            Encoding::Clog2 => transfer::clog2_light(signal),
            Encoding::Clog3 => transfer::clog3_light(signal),
            Encoding::Flog => transfer::flog_light(signal),
            Encoding::Flog2 => transfer::flog2_light(signal),
            Encoding::Nlog => transfer::nlog_light(signal),
            Encoding::Dlog => transfer::dlog_light(signal),
            Encoding::Llog => transfer::llog_light(signal),
            Encoding::AppleLog => transfer::apple_log_light(signal),
            Encoding::MiLog => transfer::mi_log_light(signal),
            Encoding::Log3g10 => transfer::log3g10_light(signal),
            Encoding::Log3g12 => transfer::log3g12_light(signal),
            Encoding::Acescc => transfer::acescc_light(signal),
            Encoding::Acescct => transfer::acescct_light(signal),
            Encoding::Cineon => transfer::cineon_light(signal),
            Encoding::Protune => transfer::protune_light(signal),
            Encoding::Tlog => transfer::tlog_light(signal),
            Encoding::DavinciIntermediate => transfer::davinci_intermediate_light(signal),
            Encoding::BmdFilmGen5 => transfer::bmd_film_gen5_light(signal),
        }
    }

    /// Light (diffuse white = 1.0) of an integer `code` at `bits` depth: a VSF encoding ×2ⁿ, bucket bottom; a source's encoding `code / (2ⁿ − 1)`, white at the top code.
    pub fn light(self, code: u32, bits: u32) -> f32 {
        let bits = bits.clamp(1, 31);
        match self {
            Encoding::Gamma2Tanh => crate::colour::convert::linearize_gamma2tanh_bits_f32(code, bits),
            e if e.is_vsf() => e.light_f32(code as f32 / (1u32 << bits) as f32),
            e => e.light_f32(code as f32 / ((1u32 << bits) - 1) as f32),
        }
    }
}

/// The primaries a plane's samples are in, each with the white its specification states. `vsf` is VSF RGB, and also camera counts that a characterization maps (the characterization says what they are). Every other value marks a planar RGB plane kept exactly as a source wrote it:
/// - stills, displays and print: `srgb` (BT.709 primaries, D65), `display_p3`, `dci_p3` (DCI white), `adobe_rgb`, `adobe_wide_gamut` (D50), `prophoto` (ROMM RGB, D50), `apple_rgb`, `colormatch_rgb` (D50), `eci_rgb_v2` (D50), `cie_rgb` (E), `xyz` (CIE XYZ, E);
/// - broadcast: `rec2020` (BT.2020/2100, from its wavelengths; also spelled `f_gamut` and `n_gamut`, which are BT.2020), `bt601_625`, `bt601_525`, `ntsc1953` (Illuminant C);
/// - ACES: `aces_ap0`, `aces_ap1` (ACES white);
/// - cameras and grading: `s_gamut3` (also `s_gamut`, the same primaries), `s_gamut3_cine`, `venice_s_gamut3`, `venice_s_gamut3_cine`, `arri_wide_gamut3`, `arri_wide_gamut4`, `v_gamut`, `cinema_gamut`, `red_wide_gamut`, `davinci_wide_gamut`, `blackmagic_wide_gamut`, `e_gamut`, `e_gamut2`, `d_gamut`, `protune_native`.
///
/// They are encodings, not characterizations: a definition, exact by its specification, with no class and no tier (`idt/README.md`, "Encodings are not transforms"). Most camera gamuts put primaries outside the spectral locus, so their real colours can land outside VSF RGB.
impl Primaries {
    /// Linear RGB in these primaries → linear VSF RGB, column-major for [`crate::colour::convert::apply_matrix_3x3_f32`]; `None` for `vsf`. Colorimetric, with no chromatic adaptation: the encoding's white is the light its specification names (D65, or D50 for ProPhoto) and lands where that light sits in VSF RGB, off the R = G = B axis; neutralising it is white balance, a Creative op. xy-defined spaces go thru CIE 1931 XYZ, the only observer an xy pair supports; Rec.2020 comes from its wavelengths under the Stockman & Sharpe 2000 10° fundamentals, 1931 not involved (`tools/generate_constants.rs`).
    pub fn to_vsf_rgb(self) -> Option<[f32; 9]> {
        use crate::colour::legacy as c;
        match self {
            Primaries::Vsf => None,
            Primaries::Srgb => Some(c::SRGB2VSF_RGB),
            Primaries::DisplayP3 => Some(c::DISPLAY_P3_2VSF_RGB),
            Primaries::AdobeRgb => Some(c::ADOBE_RGB2VSF_RGB),
            Primaries::Prophoto => Some(c::PROPHOTO2VSF_RGB),
            Primaries::Rec2020 => Some(crate::colour::rec2020::REC2020_2VSF_RGB),
            Primaries::DciP3 => Some(c::DCI_P32VSF_RGB),
            Primaries::AdobeWideGamut => Some(c::ADOBE_WIDE_GAMUT2VSF_RGB),
            Primaries::Bt601Pal => Some(c::BT601_6252VSF_RGB),
            Primaries::Bt601Ntsc => Some(c::BT601_5252VSF_RGB),
            Primaries::Ntsc1953 => Some(c::NTSC19532VSF_RGB),
            Primaries::AcesAp0 => Some(c::ACES_AP02VSF_RGB),
            Primaries::AcesAp1 => Some(c::ACES_AP12VSF_RGB),
            Primaries::SGamut3 => Some(c::S_GAMUT32VSF_RGB),
            Primaries::SGamut3Cine => Some(c::S_GAMUT3_CINE2VSF_RGB),
            Primaries::VeniceSGamut3 => Some(c::VENICE_S_GAMUT32VSF_RGB),
            Primaries::VeniceSGamut3Cine => Some(c::VENICE_S_GAMUT3_CINE2VSF_RGB),
            Primaries::ArriWideGamut3 => Some(c::ARRI_WIDE_GAMUT32VSF_RGB),
            Primaries::ArriWideGamut4 => Some(c::ARRI_WIDE_GAMUT42VSF_RGB),
            Primaries::VGamut => Some(c::V_GAMUT2VSF_RGB),
            Primaries::CinemaGamut => Some(c::CINEMA_GAMUT2VSF_RGB),
            Primaries::RedWideGamut => Some(c::RED_WIDE_GAMUT2VSF_RGB),
            Primaries::DavinciWideGamut => Some(c::DAVINCI_WIDE_GAMUT2VSF_RGB),
            Primaries::BlackmagicWideGamut => Some(c::BLACKMAGIC_WIDE_GAMUT2VSF_RGB),
            Primaries::EGamut => Some(c::E_GAMUT2VSF_RGB),
            Primaries::EGamut2 => Some(c::E_GAMUT22VSF_RGB),
            Primaries::DGamut => Some(c::D_GAMUT2VSF_RGB),
            Primaries::ProtuneNative => Some(c::PROTUNE_NATIVE2VSF_RGB),
            Primaries::AppleRgb => Some(c::APPLE_RGB2VSF_RGB),
            Primaries::ColormatchRgb => Some(c::COLORMATCH_RGB2VSF_RGB),
            Primaries::EciRgbV2 => Some(c::ECI_RGB_V22VSF_RGB),
            Primaries::CieRgb => Some(c::CIE_RGB2VSF_RGB),
            Primaries::Xyz => Some(c::XYZ2VSF_RGB),
        }
    }
}

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
    /// The primaries the samples are in. `vsf` (absent on the wire) for VSF RGB and for camera counts a characterization maps; any other value for a planar RGB plane kept exactly as a source wrote it, which the graph's `decode` op takes into VSF RGB.
    pub primaries: Primaries,
    /// What was done to these samples before this file existed and is not recorded in it, as an IDT class. `None` (absent on the wire): nothing — raw counts, a drawing's own values. `technical`: mechanics only, such as a demosaic or a frame merge in the camera. `creative`: a look was applied — white balance, a phone's tone mapping — which is every camera JPEG. A writer that cannot tell writes `creative`: absence claims nothing happened, and the weaker claim must win. Only the class lives here; the device is in `capture`, and the event with its evidence belongs to provenance (`idt/fields.md`, "Prior").
    pub prior: Option<IdtClass>,
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
        Plane { track: track.into(), seq: seq.into(), name: "sensor".into(), role: PlaneRole::Sensor, width, height, channels, layout, black, white, encoding: Encoding::Linear, primaries: Primaries::Vsf, prior: None, depth, samples: Samples::Packed(samples), maker_crop: None, orientation: 1, at: None, parent: None, program: String::new() }
    }

    /// A planar RGB plane kept exactly as a source encoded it — an sRGB or Display P3 JPEG, an Adobe RGB TIFF, a PQ AVIF. `samples` is `[3, height, width]`; `primaries` and `encoding` say what its codes mean; `black` is 0 and `white` the top code (`idt/layers.md`, the import rule); role `display`, name `main`. `prior` starts as `creative`, the weaker claim, since a picture in a display encoding has almost always been rendered by whatever made it; a writer that knows better (a drawing, a render with no tone map) clears it.
    pub fn encoded(track: &str, seq: &str, width: usize, height: usize, primaries: Primaries, encoding: Encoding, samples: BitPackedTensor) -> Self {
        let depth = samples.bit_depth;
        let top = ((1u64 << depth.min(63)) - 1) as f32;
        let channels = ["R", "G", "B"].iter().map(|n| SpectralChannel { name: (*n).into(), curve: None }).collect();
        Plane { track: track.into(), seq: seq.into(), name: "main".into(), role: PlaneRole::Display, width, height, channels, layout: PlaneLayout::Planar, black: vec![0.; 3], white: vec![top; 3], encoding, primaries, prior: Some(IdtClass::Creative), depth, samples: Samples::Packed(samples), maker_crop: None, orientation: 1, at: None, parent: None, program: String::new() }
    }
    pub fn section_name(&self) -> String {
        format!("plane.{}.{}.{}", self.track, self.seq, self.name)
    }

    fn packed(&self) -> Result<&BitPackedTensor, VisualError> {
        match &self.samples {
            Samples::Packed(t) => Ok(t),
            Samples::Wrapped(code, _) => Err(bad(format!("plane is wrapped under codec '{}' — not a bit-packed plane", *code as char))),
        }
    }

    /// Light of one sample: VSF's own encodings by their ×2ⁿ rule ([`Encoding::light`]); a source's encoding from its code normalised by this plane's `black` and `white` for that channel, so a narrow-range source (10-bit 64 to 940) decodes as it was written.
    fn sample_light(&self, ch: usize, code: u32, bits: u32) -> f32 {
        if self.encoding.is_vsf() {
            return self.encoding.light(code, bits);
        }
        let (b, w) = (self.black.get(ch).copied().unwrap_or(0.), self.white.get(ch).copied().unwrap_or(((1u64 << bits) - 1) as f32));
        self.encoding.light_f32((code as f32 - b) / (w - b))
    }

    /// The channel of sample `i`: by plane for a planar `[k, h, w]` layout, channel 0 otherwise (a source's encoding on a mosaic plane is decoded with its first channel's levels).
    fn channel_of(&self, i: usize) -> usize {
        let n = self.width * self.height;
        if self.layout == PlaneLayout::Planar && n > 0 { (i / n).min(self.black.len().saturating_sub(1)) } else { 0 }
    }

    /// A plane in foreign primaries must be planar RGB: the primaries are a 3 × 3 matrix's worth of meaning.
    fn check_primaries(&self) -> Result<(), VisualError> {
        if self.primaries != Primaries::Vsf && (self.channels.len() != 3 || self.layout != PlaneLayout::Planar) {
            return Err(bad(format!("plane {}: primaries '{}' need a planar plane of 3 channels (it has {}, {})", self.section_name(), self.primaries.as_str(), self.channels.len(), if self.layout == PlaneLayout::Planar { "planar" } else { "mosaic" })));
        }
        Ok(())
    }

    /// The plane as LINEAR counts in its own primaries: `(samples, black, white)`. A linear plane is returned as stored. An encoded plane is decoded code by code by its encoding ([`Encoding::light`]: ×2ⁿ, bucket bottom for VSF's own; white at the top code for a source's) and re-packed at 16 bits with black 0 and white placed so its brightest light still fits: `white = 65535 / max(1, peak)`, so diffuse white stays at `white` and the highlights a `g2t`, PQ or HLG plane holds sit above it rather than clipping. Readers that want counts (opsin's bridge, limbus's raw reader) go thru here and never see an encoded plane as if it were linear. The primaries are untouched; [`Self::vsf_linear_samples`] also takes them into VSF RGB.
    pub fn linear_samples(&self) -> Result<(BitPackedTensor, Vec<f32>, Vec<f32>), VisualError> {
        let t = self.packed()?;
        if self.encoding == Encoding::Linear {
            return Ok((t.clone(), self.black.clone(), self.white.clone()));
        }
        if t.bit_depth > 16 {
            return Err(bad(format!("{}-bit {} plane: encoded planes are 1 to 16 bits", t.bit_depth, self.encoding.as_str())));
        }
        let bits = t.bit_depth as u32;
        let light: Vec<f32> = t.unpack_u16().into_iter().enumerate().map(|(i, c)| self.sample_light(self.channel_of(i), c as u32, bits)).collect();
        let (counts, white) = pack_light(&light);
        let k = self.white.len().max(1);
        Ok((BitPackedTensor::pack(16, t.shape.clone(), &counts), vec![0.; k], vec![white; k]))
    }

    /// The plane as LINEAR counts in VSF RGB, ready for a model with no notion of primaries (opsin's bridge). A `vsf` plane is [`Self::linear_samples`]. A plane in foreign primaries is decoded to light by its encoding (a linear one by its black and white), taken thru [`Primaries::to_vsf_rgb`] pixel by pixel — the graph's `decode` op, done once — and re-packed by the same rule. A colour outside VSF RGB, reachable only from ProPhoto's imaginary primaries, comes out negative and is clipped at 0 here; the file keeps it.
    pub fn vsf_linear_samples(&self) -> Result<(BitPackedTensor, Vec<f32>, Vec<f32>), VisualError> {
        let Some(m) = self.primaries.to_vsf_rgb() else { return self.linear_samples() };
        self.check_primaries()?;
        let t = self.packed()?;
        if t.bit_depth > 16 {
            return Err(bad(format!("{}-bit plane in '{}' primaries: a source's encoding is 1 to 16 bits", t.bit_depth, self.primaries.as_str())));
        }
        let n = self.width * self.height;
        let codes = t.unpack_u16();
        if codes.len() != 3 * n {
            return Err(bad(format!("plane {}: {} samples for 3 × {} × {}", self.section_name(), codes.len(), self.width, self.height)));
        }
        let bits = t.bit_depth as u32;
        let light = |ch: usize, c: u16| match self.encoding {
            Encoding::Linear => (c as f32 - self.black[ch]) / (self.white[ch] - self.black[ch]),
            _ => self.sample_light(ch, c as u32, bits),
        };
        let mut out = vec![0f32; 3 * n];
        for i in 0..n {
            let rgb = [light(0, codes[i]), light(1, codes[n + i]), light(2, codes[2 * n + i])];
            let v = crate::colour::convert::apply_matrix_3x3_f32(&m, &rgb);
            for ch in 0..3 {
                out[ch * n + i] = v[ch].max(0.);
            }
        }
        let (counts, white) = pack_light(&out);
        Ok((BitPackedTensor::pack(16, vec![3, self.height, self.width], &counts), vec![0.; 3], vec![white; 3]))
    }
}

/// Light (diffuse white = 1.0) → 16-bit counts with white at `65535 / max(1, peak)`, so the brightest light takes the top count and nothing above white clips.
fn pack_light(light: &[f32]) -> (Vec<u16>, f32) {
    let peak = light.iter().copied().fold(1f32, f32::max);
    let white = 65535. / peak;
    (light.iter().map(|&l| (l * white + 0.5).min(65535.) as u16).collect(), white)
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
/// Technical, arity 1, no params: its operand, a plane in foreign primaries, decoded to light by the plane's encoding and taken into VSF RGB by its primaries' matrix ([`Primaries::to_vsf_rgb`]). The plane says which; the op says where in the program it happens, so the two cannot disagree.
pub const OP_DECODE: &str = "decode";

impl Graph {
    /// The default graph of a still: the plane, normalised, decoded into VSF RGB when it is in foreign `primaries`, through the first characterization entry, then the view ops in order. Every op after `idt` is arity 1, so a reader can strip the Creative ones and the program stays well-formed.
    pub fn default_still(id: &str, plane: &str, primaries: Primaries, idt_class: Option<IdtClass>, ops: &[ViewOp]) -> Graph {
        let mut g = vec![
            GraphOp { name: OP_PLANE.into(), class: IdtClass::Technical, arity: 0, params: vec![], reference: plane.into() },
            GraphOp { name: OP_BLACK_WHITE.into(), class: IdtClass::Technical, arity: 1, params: vec![], reference: String::new() },
        ];
        if primaries != Primaries::Vsf {
            g.push(GraphOp { name: OP_DECODE.into(), class: IdtClass::Technical, arity: 1, params: vec![], reference: String::new() });
        }
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

    /// The view ops of a still's default graph: everything after the source/normalise/decode/idt prologue, as a [`ViewTransform`] — what opsin replays.
    pub fn to_view_transform(&self) -> ViewTransform {
        let ops = self
            .ops
            .iter()
            .filter(|op| !matches!(op.name.as_str(), OP_PLANE | OP_BLACK_WHITE | OP_DECODE | OP_IDT | OP_ENCODE))
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
        // Absent ⇒ VSF RGB and nothing before the file: both written only when they say something, so a plane that needs neither keeps its bytes.
        self.check_primaries()?;
        if self.primaries != Primaries::Vsf {
            s.add_field("primaries", VsfType::a(self.primaries.as_str().into()));
        }
        if let Some(c) = self.prior {
            s.add_field("prior", VsfType::a(c.as_str().into()));
        }
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
        let plane = Plane {
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
            primaries: match s.text("primaries") {
                Some(p) => Primaries::parse(p)?,
                None => Primaries::Vsf,
            },
            prior: match s.text("prior") {
                Some(c) => Some(IdtClass::parse(c).map_err(|e| bad(format!("plane prior: {e}")))?),
                None => None,
            },
            depth,
            samples,
            maker_crop,
            orientation: s.uint("orientation").map(|o| o as u16).unwrap_or(1),
            at: eagle(s, "at"),
            parent,
            program: text(s, "program"),
        };
        plane.check_primaries()?;
        Ok(plane)
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
        Ok(View { id: id.into(), graph: text(s, "graph"), root: s.uint("root").unwrap_or(0) as usize, space: text(s, "space"), encoding: match s.text("encoding") { Some(e) => Encoding::parse(e)?, None => Encoding::Gamma2 }, bits: s.uint("bits").unwrap_or(16) as u8, purpose: Purpose::parse(s.text("purpose").unwrap_or("default"))? })
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
    /// A still from its parts: one beam track `b0`, one span `s0` of one frame naming the plane, the plane as `plane.b0.s0.<name>` (`sensor` for a capture, `main` for an encoded source), the default graph and view.
    pub fn still(plane: Plane, capture: Option<Capture>, characterization: Option<ColourProfile>, ops: &[ViewOp]) -> Visual {
        let mut plane = plane;
        plane.track = "b0".into();
        plane.seq = "s0".into();
        let plane_name = plane.section_name();
        let graph = Graph::default_still("g0", &plane_name, plane.primaries, characterization.as_ref().and_then(|p| p.entries.first()).map(|e| e.class), ops);
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
        let primaries = vis.planes.iter().find(|p| p.section_name() == plane_ref).map_or(Primaries::Vsf, |p| p.primaries);
        let graph = Graph::default_still(&format!("g{n}"), &plane_ref, primaries, vis.characterization.as_ref().and_then(|p| p.entries.first()).map(|e| e.class), ops);
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
        // The in-memory model holds linear VSF RGB counts (or camera counts its profile maps), so an encoded plane is linearized and a plane in foreign primaries decoded on the way (`Plane::vsf_linear_samples`).
        let (samples, black, white) = p.vsf_linear_samples()?;
        let (make, model) = match &self.capture {
            Some(c) => (c.make.clone(), c.model.clone()),
            None => (String::new(), String::new()),
        };
        let view = self.default_graph().map(|g| g.to_view_transform()).filter(|v| !v.ops.is_empty());
        Ok(SpectralImage { width: p.width, height: p.height, channels: p.channels.clone(), layout: p.layout.clone(), samples, black, white, make, model, provenance: self.provenance.clone(), profile: self.characterization.clone(), view })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Untagged is always VSF RGB gamma 2: a view (like a plane) that omits `encoding` reads as `gamma2`, never `linear`.
    #[test]
    fn a_view_without_encoding_is_gamma2() {
        let mut s = VsfSection::new("view.v0");
        s.add_field("graph", VsfType::a("g0".into()));
        s.add_field("root", VsfType::u(0, false));
        let v = View::from_section("v0", &s).unwrap();
        assert_eq!(v.encoding, Encoding::Gamma2);
        s.add_field("encoding", VsfType::a("gamma2tanh".into()));
        assert_eq!(View::from_section("v0", &s).unwrap().encoding, Encoding::Gamma2Tanh);
    }

    /// An encoded plane reaches the in-memory model as linear counts: gamma 2 squares, `g2t` keeps its highlights above white.
    #[test]
    fn encoded_planes_linearize_with_room_above_white() {
        use crate::spectral_image::PlaneLayout;
        let codes: Vec<u16> = vec![0, 128, 194, 255];
        let mut p = Plane::sensor("b0", "s0", 4, 1, vec![], PlaneLayout::Planar, vec![0.], vec![255.], BitPackedTensor::pack(8, vec![1, 4], &codes));
        p.encoding = Encoding::Gamma2;
        let (t, black, white) = p.linear_samples().unwrap();
        assert_eq!((black[0], white[0], t.bit_depth), (0., 65535., 16));
        assert_eq!(t.unpack_u16()[1], 16384);
        p.encoding = Encoding::Gamma2Tanh;
        let (t, _, white) = p.linear_samples().unwrap();
        let counts = t.unpack_u16();
        assert_eq!(counts[3], 65535, "the brightest code takes the top count");
        assert!(white[0] < 65535. / 9.);
        let diffuse = counts[2] as f32 / white[0];
        assert!((diffuse - 1.).abs() < 0.02, "code 194 is diffuse white: {diffuse}");
    }

    /// `g2t` is the short spelling of `gamma2tanh`: both read as the same encoding, and writers emit `g2t`.
    #[test]
    fn g2t_reads_as_gamma2tanh() {
        assert_eq!(Encoding::parse("g2t").unwrap(), Encoding::Gamma2Tanh);
        assert_eq!(Encoding::parse("gamma2tanh").unwrap(), Encoding::Gamma2Tanh);
        assert_eq!(Encoding::Gamma2Tanh.as_str(), "g2t");
        assert!(Encoding::parse("G2T").is_err());
    }
}
