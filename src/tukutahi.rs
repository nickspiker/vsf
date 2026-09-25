//! Tukutahi (te reo Māori: simultaneous, in sync): how a video or audio stream declares its rate, how every frame and sample is named, and how those names are tied to true time — with Manawa (heartbeat), the cadence at which measurements are written down.
//!
//! The specification is photon's `docs/tukutahi.md` (v0.1, Nick Spiker, 2026-09-25; contributor-not-owner). Section numbers below are its sections.
//! Eagle Time is the timescale (epoch 1969-07-20T20:17:48 TAI, fixed forever). No float appears in a declaration, a name, a mark or an encoding; the one float-free rule holds everywhere here.
//!
//! What this module is: the declaration and its validation (§3), exact rational naming (§4, Appendix B), the Manawa marking grid with its hash chain (§5, §6), interpolation between marks (§5.3), capture geometry (§7), the lock enum (§8), timecode export (§12.2), the VSF field encoding (§13) and a validating reader (§14).
//! Mark signatures (§5.2 `sig`) are carried when present and never required: decryption is verification too, and the chain (`root`, `prev`) is rolled regardless.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::types::eagle_time::OSCILLATIONS_PER_SECOND;
use crate::VsfType;

/// Oscillations per Eagle second (§2), widened for exact rational arithmetic.
pub const OPS: i128 = OSCILLATIONS_PER_SECOND as i128;

/// The native video rates (§3): integers only — 25 and 50 stay for mains flicker; nothing ×1000/1001 is ever native.
pub const NATIVE_VIDEO_RATES: [u32; 10] = [24, 25, 30, 48, 50, 60, 96, 100, 120, 240];
/// The one native audio rate (§3, §10).
pub const NATIVE_AUDIO_RATE: u32 = 48_000;

/// Round-half-up integer division, correct for negatives (Appendix B).
fn div_round(n: i128, d: i128) -> i128 {
    let q = n.div_euclid(d);
    let r = n.rem_euclid(d);
    q + ((r * 2 >= d) as i128)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// What a stream carries (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Video = 0,
    Audio = 1,
}

/// What a device's clock is disciplined to (§8). `House` follows a non-Eagle genlock/timecode reference: names stay exact, alignment guarantees do not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Lock {
    Gnss = 0,
    Ptp = 1,
    Ntp = 2,
    Peer = 3,
    House = 4,
    Holdover = 5,
    Free = 6,
}

impl Lock {
    pub fn from_u8(v: u8) -> Option<Lock> {
        Some(match v {
            0 => Lock::Gnss,
            1 => Lock::Ptp,
            2 => Lock::Ntp,
            3 => Lock::Peer,
            4 => Lock::House,
            5 => Lock::Holdover,
            6 => Lock::Free,
            _ => return None,
        })
    }
}

/// What a device can guarantee about its stamps (§9). A reader never assumes Level 2 properties of a Level 1 stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Level {
    /// Commodity: software stamps thru a disciplined clock model; drift corrected after the fact.
    Commodity = 1,
    /// One disciplined oscillator sources sensor, audio and display; hardware frame-start stamps; interpolation exact.
    SingleClock = 2,
}

/// The header that fixes a stream (§3). Immutable for the stream's life: a change of any field is a new stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub kind: Kind,
    pub rate_num: u32,
    pub rate_den: u32,
    /// Offset of index 0 from the epoch, as a fraction of one period (§11). `0/1` unless declared.
    pub phase_num: u32,
    pub phase_den: u32,
    /// Marking cadence in whole periods (§5.4).
    pub manawa_period: u32,
    pub level: Level,
    /// The capturing device's identity (a photon device key).
    pub source: Vec<u8>,
    /// Video: rows read out per frame; `1` for global shutter or unknown.
    pub sensor_rows: u32,
    /// Captured under this declaration (`true`) or imported (`false`).
    pub native: bool,
}

/// Why a declaration is refused (§14 item 1 and the §3 constraints).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclError {
    ZeroRate,
    RateNotReduced,
    BadPhase,
    ZeroManawa,
    NativeRateNotAllowed,
    ZeroSensorRows,
}

impl Decl {
    /// A native 48 kHz audio stream, marked once a second (§10).
    pub fn audio(source: Vec<u8>) -> Decl {
        Decl { kind: Kind::Audio, rate_num: NATIVE_AUDIO_RATE, rate_den: 1, phase_num: 0, phase_den: 1, manawa_period: NATIVE_AUDIO_RATE, level: Level::Commodity, source, sensor_rows: 1, native: true }
    }

    /// A native video stream at an integer rate, marked on the first frame of each Eagle second.
    pub fn video(rate: u32, source: Vec<u8>, sensor_rows: u32) -> Decl {
        Decl { kind: Kind::Video, rate_num: rate, rate_den: 1, phase_num: 0, phase_den: 1, manawa_period: rate, level: Level::Commodity, source, sensor_rows, native: true }
    }

    /// An imported stream at any rational rate (e.g. `24000/1001`, `44100/1`), marked nearest each second boundary.
    pub fn imported(kind: Kind, rate_num: u32, rate_den: u32, source: Vec<u8>) -> Decl {
        let manawa = div_round(rate_num as i128, rate_den as i128).max(1) as u32; // WHY/PROOF: a rate under half a frame a second would round to a zero cadence, which §5.4 forbids — one period is the least cadence there is
        Decl { kind, rate_num, rate_den, phase_num: 0, phase_den: 1, manawa_period: manawa, level: Level::Commodity, source, sensor_rows: 1, native: false }
    }

    /// The §3 constraints: a reduced positive rational rate, a phase inside one period, a cadence of at least one period, and native streams on the native rates.
    pub fn validate(&self) -> Result<(), DeclError> {
        if self.rate_num == 0 || self.rate_den == 0 {
            return Err(DeclError::ZeroRate);
        }
        if gcd(self.rate_num as u64, self.rate_den as u64) != 1 {
            return Err(DeclError::RateNotReduced);
        }
        if self.phase_den == 0 || self.phase_num >= self.phase_den {
            return Err(DeclError::BadPhase);
        }
        if self.manawa_period == 0 {
            return Err(DeclError::ZeroManawa);
        }
        if self.sensor_rows == 0 {
            return Err(DeclError::ZeroSensorRows);
        }
        if self.native {
            let ok = self.rate_den == 1
                && match self.kind {
                    Kind::Video => NATIVE_VIDEO_RATES.contains(&self.rate_num),
                    Kind::Audio => self.rate_num == NATIVE_AUDIO_RATE,
                };
            if !ok {
                return Err(DeclError::NativeRateNotAllowed);
            }
        }
        Ok(())
    }

    /// Nominal Eagle count of index `n` (§4, Appendix B): `(n + phase) · period` since the epoch, rounded once, half up.
    pub fn eagle_of(&self, n: i64) -> i64 {
        let num = (n as i128 * self.phase_den as i128 + self.phase_num as i128) * self.rate_den as i128 * OPS;
        let den = self.rate_num as i128 * self.phase_den as i128;
        div_round(num, den) as i64
    }

    /// The nearest index to Eagle instant `e` (Appendix B). `index_of(eagle_of(n)) == n` for every n.
    pub fn index_of(&self, e: i64) -> i64 {
        let num = e as i128 * self.rate_num as i128 * self.phase_den as i128 - self.phase_num as i128 * self.rate_den as i128 * OPS;
        let den = self.rate_den as i128 * OPS * self.phase_den as i128;
        div_round(num, den) as i64
    }

    /// An integer rate with zero phase: marks fall on indices divisible by the cadence (§5.1). Anything else is the legacy placement.
    fn on_integer_grid(&self) -> bool {
        self.rate_den == 1 && self.phase_num == 0
    }

    /// Whether index `n` carries a mark (§5.1). Integer rates with zero phase: `n` divisible by the cadence. Legacy rational rates (and declared phases, where no frame lands on a boundary): the frame whose nominal instant is nearest each Eagle second boundary.
    pub fn is_mark(&self, n: i64) -> bool {
        if self.on_integer_grid() {
            return n.rem_euclid(self.manawa_period as i64) == 0;
        }
        let second = div_round(self.eagle_of(n) as i128, OPS);
        self.index_of((second * OPS) as i64) == n
    }
}

/// The instant row `r` of a frame is "of": its mid-exposure (§7.2, Appendix B). `stamp` is the frame's canonical point (middle row, mid-exposure); `readout` spans first-row start to last-row start (0 = global shutter).
pub fn row_center(stamp: i64, readout: i64, rows: u32, r: u32) -> i64 {
    if rows <= 1 {
        return stamp;
    }
    let num = readout as i128 * (2 * r as i128 - (rows as i128 - 1));
    stamp + div_round(num, 2 * (rows as i128 - 1)) as i64
}

/// Row `r`'s exposure start, from its centre (§7.2).
pub fn exposure_start(row_centre: i64, exposure: i64) -> i64 {
    row_centre - exposure / 2
}

/// Capture geometry (§7.2), re-declared on change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub exposure: i64,
    pub readout: i64,
}

/// One record written on the Manawa cadence (§5.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub index: i64,
    /// Measured instant of the frame's canonical point (§7.1), Eagle.
    pub stamp: i64,
    /// 1-sigma uncertainty of `stamp`, ns.
    pub unc_ns: u32,
    pub lock: Lock,
    /// Measured local clock rate error, informational.
    pub rate_ppm: i32,
    /// Merkle root of the per-frame hashes since the previous mark.
    pub root: [u8; 32],
    /// Hash of the previous mark (all zero bytes for a stream's first mark — there is no previous).
    pub prev: [u8; 32],
    /// Signature by the declaration's `source` over all of the above — optional; never required by this module.
    pub sig: Option<Vec<u8>>,
}

impl Mark {
    /// The mark's identity: BLAKE3 over its fields in declaration order, signature excluded (the signature signs this) — what the next mark's `prev` names.
    pub fn hash(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"tukutahi.mark.v0");
        h.update(&self.index.to_le_bytes());
        h.update(&self.stamp.to_le_bytes());
        h.update(&self.unc_ns.to_le_bytes());
        h.update(&[self.lock as u8]);
        h.update(&self.rate_ppm.to_le_bytes());
        h.update(&self.root);
        h.update(&self.prev);
        *h.finalize().as_bytes()
    }
}

/// The Merkle root of a run of frame hashes (§5.2 `root`): BLAKE3, leaves and inner nodes domain-separated (0x00 / 0x01) so no inner node can pose as a leaf; an odd node rises unpaired. An empty run's root is BLAKE3 of the leaf domain alone.
pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    let leaf = |h: &[u8; 32]| {
        let mut x = blake3::Hasher::new();
        x.update(&[0x00]);
        x.update(h);
        *x.finalize().as_bytes()
    };
    if leaves.is_empty() {
        return *blake3::hash(&[0x00]).as_bytes();
    }
    let mut level: Vec<[u8; 32]> = leaves.iter().map(leaf).collect();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|p| match p {
                [a, b] => {
                    let mut x = blake3::Hasher::new();
                    x.update(&[0x01]);
                    x.update(a);
                    x.update(b);
                    *x.finalize().as_bytes()
                }
                [a] => *a,
                _ => unreachable!("chunks(2) yields one or two"),
            })
            .collect();
    }
    level[0]
}

/// A frame (or audio packet) between marks (§6): the index step when it is not 1 (a DECLARED gap), and the payload hash that feeds the next mark's root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRecord {
    /// `index − previous index`; `None` = 1.
    pub delta: Option<i64>,
    pub hash: [u8; 32],
    pub geometry: Option<Geometry>,
}

/// The writer's error: a frame or mark out of the order §6 and §5.1 require.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// Index not after the previous one (`delta ≤ 0`, §14 item 6).
    NotAfter,
    /// A mark on an index the cadence does not place a mark on.
    OffCadence,
}

/// The stream writer — the Manawa marker. Frames come in index order; at each marked index the writer closes the span with a mark whose root covers every frame since the previous mark (the frames BEFORE this index), then records the marked frame itself as the first of the next span.
pub struct Marker {
    pub decl: Decl,
    last: Option<i64>,
    span: Vec<[u8; 32]>,
    prev: [u8; 32],
}

impl Marker {
    pub fn new(decl: Decl) -> Marker {
        Marker { decl, last: None, span: Vec::new(), prev: [0u8; 32] }
    }

    /// Close the span at marked index `index` (stamped as measured). Call BEFORE [`Self::frame`] for that index.
    pub fn mark(&mut self, index: i64, stamp: i64, unc_ns: u32, lock: Lock, rate_ppm: i32) -> Result<Mark, WriteError> {
        if !self.decl.is_mark(index) {
            return Err(WriteError::OffCadence);
        }
        if self.last.is_some_and(|l| index <= l) {
            return Err(WriteError::NotAfter);
        }
        let m = Mark { index, stamp, unc_ns, lock, rate_ppm, root: merkle_root(&self.span), prev: self.prev, sig: None };
        self.prev = m.hash();
        self.span.clear();
        Ok(m)
    }

    /// Record frame `index` with its payload hash. A skipped index is a declared gap (`delta`), never a silent one.
    pub fn frame(&mut self, index: i64, hash: [u8; 32], geometry: Option<Geometry>) -> Result<FrameRecord, WriteError> {
        let delta = match self.last {
            Some(l) if index <= l => return Err(WriteError::NotAfter),
            Some(l) => index - l,
            None => 1,
        };
        self.last = Some(index);
        self.span.push(hash);
        Ok(FrameRecord { delta: (delta != 1).then_some(delta), hash, geometry })
    }
}

/// What a reader flags (§14). From the flagged point the stream is broken; the reader says where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flag {
    /// §14 item 1 (and §3): the declaration itself.
    Declaration(DeclError),
    /// §14 item 3: a mark off the Manawa cadence.
    MarkOffCadence(i64),
    /// §14 item 4: the root does not match the frames since the previous mark (a tampered or missing frame).
    RootMismatch(i64),
    /// §14 item 4: `prev` does not name the previous mark.
    ChainBreak(i64),
    /// §14 item 5: the stamp implies arrival before capture, or older than the allowed age.
    ImpossibleArrival(i64),
    /// §14 item 6: a frame with `delta ≤ 0`.
    BadDelta,
    /// §14 item 7: two marks on one index.
    DuplicateMark(i64),
    /// §6: a mark whose index disagrees with the running frame count (an undeclared discontinuity).
    Discontinuity(i64),
}

/// The validating reader (§14). Feed it frames and marks in stream order; it names every frame, reports declared gaps, and flags the first breach.
pub struct Reader {
    pub decl: Decl,
    last: Option<i64>,
    span: Vec<[u8; 32]>,
    prev_mark: Option<Mark>,
    /// Declared gaps seen: (first missing index, how many).
    pub gaps: Vec<(i64, i64)>,
}

impl Reader {
    pub fn new(decl: Decl) -> Result<Reader, Flag> {
        decl.validate().map_err(Flag::Declaration)?;
        Ok(Reader { decl, last: None, span: Vec::new(), prev_mark: None, gaps: Vec::new() })
    }

    /// Name the next frame: its index, recovered from the running count plus its declared step. The very first frame takes its index from the mark before it.
    pub fn frame(&mut self, rec: &FrameRecord) -> Result<i64, Flag> {
        let step = rec.delta.unwrap_or(1);
        if step <= 0 {
            return Err(Flag::BadDelta);
        }
        let index = match (self.last, self.prev_mark.as_ref()) {
            (Some(l), _) => l + step,
            (None, Some(m)) => m.index,
            (None, None) => 0,
        };
        if step > 1 {
            self.gaps.push((index - step + 1, step - 1));
        }
        self.last = Some(index);
        self.span.push(rec.hash);
        Ok(index)
    }

    /// Check a mark. `arrival` (the reader's own Eagle now, if live) and `max_age` bound it per §14 item 5.
    pub fn mark(&mut self, m: &Mark, arrival: Option<i64>, max_age: i64) -> Result<(), Flag> {
        if !self.decl.is_mark(m.index) {
            return Err(Flag::MarkOffCadence(m.index));
        }
        if let Some(p) = self.prev_mark.as_ref() {
            if p.index == m.index {
                return Err(Flag::DuplicateMark(m.index));
            }
            if m.prev != p.hash() {
                return Err(Flag::ChainBreak(m.index));
            }
        }
        if let Some(l) = self.last {
            if m.index <= l {
                return Err(Flag::Discontinuity(m.index));
            }
        }
        if merkle_root(&self.span) != m.root {
            return Err(Flag::RootMismatch(m.index));
        }
        if let Some(now) = arrival {
            let unc = m.unc_ns as i128 * OPS / 1_000_000_000;
            if (now as i128) < m.stamp as i128 - unc || now as i128 - m.stamp as i128 > max_age as i128 {
                return Err(Flag::ImpossibleArrival(m.index));
            }
        }
        // The marked frame is the next to arrive; a gap up to it is declared by that frame's own delta.
        self.span.clear();
        self.prev_mark = Some(m.clone());
        Ok(())
    }
}

/// Where frame `n` was, between two marks (§5.3): linear between their stamps, uncertainty the larger mark's plus the rate error over the distance to the nearer mark. `(stamp, unc_ns)`.
pub fn interpolate(a: &Mark, b: &Mark, n: i64) -> (i64, u64) {
    let span = (b.index - a.index) as i128;
    let stamp = if span == 0 { a.stamp } else { (a.stamp as i128 + div_round((b.stamp - a.stamp) as i128 * (n - a.index) as i128, span)) as i64 };
    let near = if (n - a.index).abs() <= (b.index - n).abs() { a.stamp } else { b.stamp };
    let dist_ns = ((stamp - near) as i128).unsigned_abs() * 1_000_000_000 / OPS as u128;
    let ppm = a.rate_ppm.unsigned_abs().max(b.rate_ppm.unsigned_abs()) as u128;
    (stamp, a.unc_ns.max(b.unc_ns) as u64 + (dist_ns * ppm / 1_000_000) as u64)
}

/// Where frame `n` was, beyond the last mark (§5.3): the nominal period carries it, and the uncertainty grows by the rate error over the whole distance — a reader never pretends a coasting stream is as good as a marked one.
pub fn extrapolate(decl: &Decl, last: &Mark, n: i64) -> (i64, u64) {
    let stamp = last.stamp + (decl.eagle_of(n) - decl.eagle_of(last.index));
    let dist_ns = ((stamp - last.stamp) as i128).unsigned_abs() * 1_000_000_000 / OPS as u128;
    let ppm = (last.rate_ppm.unsigned_abs() as u128).max(1);
    (stamp, last.unc_ns as u64 + (dist_ns * ppm / 1_000_000) as u64)
}

/// SMPTE 12M timecode, rendered from a name — never stored (§12.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timecode {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub frames: u32,
    pub drop_frame: bool,
}

impl core::fmt::Display for Timecode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02}:{:02}:{:02}{}{:02}", self.hours, self.minutes, self.seconds, if self.drop_frame { ';' } else { ':' }, self.frames)
    }
}

/// Timecode for index `n` of a video stream (§12.2). The label rate is the rate rounded up to a whole frame count (24, 30, 60 for the 1000/1001 family); drop-frame applies ONLY to 1000/1001 rates whose label rate is a multiple of 30 — skip labels `:00` and `:01` (`:00`–`:03` at 60) at the start of each minute except every tenth. Hours wrap at 24. Counted from the index's own origin (the Eagle epoch), so the label of a given frame is the same on every device.
pub fn timecode(decl: &Decl, n: i64) -> Timecode {
    let fps = (decl.rate_num as u64).div_ceil(decl.rate_den as u64);
    let drop = decl.rate_den == 1001 && fps % 30 == 0;
    let mut f = n.rem_euclid(fps as i64 * 86_400 * 1_000) as u64; // WHY/PROOF: labels repeat daily; folding by a whole number of days first keeps the arithmetic small for an epoch-scale index, and rem_euclid keeps a pre-epoch index non-negative
    if drop {
        let skip = 2 * fps / 30;
        let per_10min = fps * 600 - 9 * skip;
        let per_min = fps * 60 - skip;
        let (d, m) = (f / per_10min, f % per_10min);
        f += 9 * skip * d + if m > skip { skip * ((m - skip) / per_min) } else { 0 };
    }
    Timecode {
        hours: ((f / (fps * 3600)) % 24) as u32,
        minutes: ((f / (fps * 60)) % 60) as u32,
        seconds: ((f / fps) % 60) as u32,
        frames: (f % fps) as u32,
        drop_frame: drop,
    }
}

// ---- §13 VSF encoding: plain integers for names, rates and phases; e6 for Eagle counts; no float type anywhere ----

/// The declaration's section name.
pub const DECL_SECTION: &str = "tukutahi_decl";
/// A mark's section name (one per mark).
pub const MARK_SECTION: &str = "tukutahi_mark";

fn uint(v: u64) -> VsfType {
    VsfType::u(v as usize, false)
}

/// The declaration as VSF fields (§13).
pub fn decl_fields(d: &Decl) -> Vec<(String, VsfType)> {
    alloc::vec![
        ("kind".to_string(), uint(d.kind as u64)),
        ("rate_num".to_string(), uint(d.rate_num as u64)),
        ("rate_den".to_string(), uint(d.rate_den as u64)),
        ("phase_num".to_string(), uint(d.phase_num as u64)),
        ("phase_den".to_string(), uint(d.phase_den as u64)),
        ("manawa_period".to_string(), uint(d.manawa_period as u64)),
        ("level".to_string(), uint(d.level as u64)),
        ("source".to_string(), VsfType::hR(d.source.clone())),
        ("sensor_rows".to_string(), uint(d.sensor_rows as u64)),
        ("native".to_string(), VsfType::u0(d.native)),
    ]
}

/// A mark as VSF fields (§13); the signature only when present.
pub fn mark_fields(m: &Mark) -> Vec<(String, VsfType)> {
    let mut f = alloc::vec![
        ("index".to_string(), VsfType::i(m.index as isize)),
        ("stamp".to_string(), VsfType::e(crate::types::EtType::e6(m.stamp))),
        ("unc_ns".to_string(), uint(m.unc_ns as u64)),
        ("lock".to_string(), uint(m.lock as u64)),
        ("rate_ppm".to_string(), VsfType::i(m.rate_ppm as isize)),
        ("root".to_string(), VsfType::hb(m.root.to_vec())),
        ("prev".to_string(), VsfType::hb(m.prev.to_vec())),
    ];
    if let Some(s) = m.sig.as_ref() {
        f.push(("sig".to_string(), VsfType::ge(s.clone())));
    }
    f
}

fn first<'a>(fields: &'a [(String, VsfType)], name: &str) -> Option<&'a VsfType> {
    fields.iter().find(|(n, _)| n == name).map(|(_, v)| v)
}

/// Read a declaration back from its fields — width-agnostic integer reads; refuses a float wherever one appears (§14 item 2) and validates (§14 item 1).
pub fn decl_from_fields(fields: &[(String, VsfType)]) -> Result<Decl, Flag> {
    let u = |name: &str| -> Option<u32> { first(fields, name)?.as_u64().and_then(|v| u32::try_from(v).ok()) };
    let bad = Flag::Declaration(DeclError::ZeroRate);
    let d = Decl {
        kind: match u("kind") {
            Some(0) => Kind::Video,
            Some(1) => Kind::Audio,
            _ => return Err(bad),
        },
        rate_num: u("rate_num").ok_or(bad.clone())?,
        rate_den: u("rate_den").ok_or(bad.clone())?,
        phase_num: u("phase_num").unwrap_or(0),
        phase_den: u("phase_den").unwrap_or(1),
        manawa_period: u("manawa_period").ok_or(bad.clone())?,
        level: match u("level") {
            Some(1) => Level::Commodity,
            Some(2) => Level::SingleClock,
            _ => return Err(bad),
        },
        source: match first(fields, "source") {
            Some(VsfType::hR(b)) => b.clone(),
            _ => Vec::new(),
        },
        sensor_rows: u("sensor_rows").unwrap_or(1),
        native: matches!(first(fields, "native"), Some(VsfType::u0(true))),
    };
    d.validate().map_err(Flag::Declaration)?;
    Ok(d)
}

/// Read a mark back from its fields.
pub fn mark_from_fields(fields: &[(String, VsfType)]) -> Option<Mark> {
    let h32 = |name: &str| -> Option<[u8; 32]> {
        match first(fields, name)? {
            VsfType::hb(b) => <[u8; 32]>::try_from(b.as_slice()).ok(),
            _ => None,
        }
    };
    Some(Mark {
        index: first(fields, "index")?.as_i64()?,
        stamp: match first(fields, "stamp")? {
            VsfType::e(crate::types::EtType::e6(o)) => *o,
            other => other.as_i64()?,
        },
        unc_ns: u32::try_from(first(fields, "unc_ns")?.as_u64()?).ok()?,
        lock: Lock::from_u8(u8::try_from(first(fields, "lock")?.as_u64()?).ok()?)?,
        rate_ppm: i32::try_from(first(fields, "rate_ppm")?.as_i64()?).ok()?,
        root: h32("root")?,
        prev: h32("prev")?,
        sig: match first(fields, "sig") {
            Some(VsfType::ge(s)) => Some(s.clone()),
            _ => None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); // the algorithm: an LCG wraps by definition
        *seed >> 11
    }

    fn all_decls() -> Vec<Decl> {
        let mut v: Vec<Decl> = NATIVE_VIDEO_RATES.iter().map(|&r| Decl::video(r, Vec::new(), 1)).collect();
        v.push(Decl::audio(Vec::new()));
        v.push(Decl::imported(Kind::Video, 24_000, 1001, Vec::new()));
        v.push(Decl::imported(Kind::Video, 30_000, 1001, Vec::new()));
        v.push(Decl::imported(Kind::Audio, 44_100, 1, Vec::new()));
        v
    }

    /// §15.1: `index(eagle(n)) == n` for 10^6 random n per declaration, at every native rate and the 1000/1001 family, with zero and a non-zero phase.
    #[test]
    fn names_round_trip_exactly() {
        let mut seed = 7u64;
        for base in all_decls() {
            for (pn, pd) in [(0u32, 1u32), (3, 8)] {
                let d = Decl { phase_num: pn, phase_den: pd, native: base.native && pn == 0, ..base.clone() };
                d.validate().unwrap();
                for _ in 0..1_000_000 {
                    // Names across ±200 years (EagleCount's own range is ±206, §2), at the rate's whole frames per second.
                    let span = 200 * 31_557_600 * (d.rate_num as i64 / d.rate_den as i64);
                    let n = (lcg(&mut seed) % (2 * span as u64)) as i64 - span;
                    assert_eq!(d.index_of(d.eagle_of(n)), n, "rate {}/{} phase {pn}/{pd} n {n}", d.rate_num, d.rate_den);
                }
            }
        }
    }

    /// §15.2: integer rate, zero phase — a name lies on an Eagle second boundary iff it is a whole multiple of the rate.
    #[test]
    fn second_boundaries_are_exactly_the_multiples_of_the_rate() {
        for d in all_decls().into_iter().filter(|d| d.rate_den == 1) {
            let r = d.rate_num as i64;
            for n in (-3 * r)..(3 * r) {
                assert_eq!((d.eagle_of(n) as i128).rem_euclid(OPS) == 0, n.rem_euclid(r) == 0, "rate {r} n {n}");
            }
        }
    }

    /// §15.3: audio and video share the epoch exactly — sample 800n at 48 kHz IS frame n at 60 fps.
    #[test]
    fn audio_and_video_bind_exactly() {
        let (a, v) = (Decl::audio(Vec::new()), Decl::video(60, Vec::new(), 1));
        let mut seed = 11u64;
        for _ in 0..100_000 {
            let n = (lcg(&mut seed) % 400_000_000_000) as i64 - 200_000_000_000;
            assert_eq!(a.eagle_of(800 * n), v.eagle_of(n));
        }
    }

    /// §15.4: a Level 1 camera at +80 ppm with one-second marks — every interpolated frame lands within 80 µs of where it really was.
    #[test]
    fn interpolation_holds_a_drifting_camera() {
        let d = Decl::video(60, Vec::new(), 1);
        let truth = |n: i64| -> i64 { (d.eagle_of(0) as f64 + (d.eagle_of(n) - d.eagle_of(0)) as f64 / (1.0 + 80e-6)).round() as i64 };
        let mut w = Marker::new(d.clone());
        let mut marks = Vec::new();
        for n in 0..(60 * 10) {
            if d.is_mark(n) {
                marks.push(w.mark(n, truth(n), 50_000, Lock::Ntp, 80).unwrap());
            }
            w.frame(n, [n as u8; 32], None).unwrap();
        }
        let tol = (80 * OPS / 1_000_000) as i64;
        for pair in marks.windows(2) {
            for n in pair[0].index..pair[1].index {
                let (s, _) = interpolate(&pair[0], &pair[1], n);
                assert!((s - truth(n)).abs() <= tol, "frame {n}: off by {} oscillations", s - truth(n));
            }
        }
    }

    /// §15.5: three frames removed — the names of the rest are preserved and the gap is reported at the right index.
    #[test]
    fn a_declared_gap_keeps_every_name() {
        let d = Decl::video(24, Vec::new(), 1);
        let mut w = Marker::new(d.clone());
        let mut r = Reader::new(d.clone()).unwrap();
        let mut named = Vec::new();
        for n in 0..48i64 {
            if d.is_mark(n) {
                let m = w.mark(n, d.eagle_of(n), 1_000, Lock::Ntp, 0).unwrap();
                r.mark(&m, None, i64::MAX).unwrap();
            }
            if (10..13).contains(&n) {
                continue; // dropped at capture
            }
            let rec = w.frame(n, [n as u8; 32], None).unwrap();
            named.push((n, r.frame(&rec).unwrap()));
        }
        assert!(named.iter().all(|(sent, read)| sent == read), "names survive the gap");
        assert_eq!(r.gaps, alloc::vec![(10, 3)]);
    }

    /// §15.6: altering one frame's payload fails validation at the next mark.
    #[test]
    fn a_tampered_frame_fails_at_the_next_mark() {
        let d = Decl::video(30, Vec::new(), 1);
        let mut w = Marker::new(d.clone());
        let mut stream: Vec<(Option<Mark>, FrameRecord)> = Vec::new();
        for n in 0..61i64 {
            let m = d.is_mark(n).then(|| w.mark(n, d.eagle_of(n), 1_000, Lock::Ntp, 0).unwrap());
            stream.push((m, w.frame(n, [n as u8; 32], None).unwrap()));
        }
        stream[17].1.hash[0] ^= 1;
        let mut r = Reader::new(d).unwrap();
        let mut failed_at = None;
        for (m, f) in &stream {
            if let Some(m) = m {
                if let Err(e) = r.mark(m, None, i64::MAX) {
                    failed_at = Some(e);
                    break;
                }
            }
            r.frame(f).unwrap();
        }
        assert_eq!(failed_at, Some(Flag::RootMismatch(30)), "the next mark after frame 17 is frame 30's");
    }

    /// The chain: a mark whose `prev` does not name the previous mark is flagged, and so is a duplicate index.
    #[test]
    fn the_mark_chain_is_checked() {
        let d = Decl::audio(Vec::new());
        let mut w = Marker::new(d.clone());
        let a = w.mark(0, d.eagle_of(0), 1, Lock::Gnss, 0).unwrap();
        let mut b = w.mark(48_000, d.eagle_of(48_000), 1, Lock::Gnss, 0).unwrap();
        let mut r = Reader::new(d).unwrap();
        r.mark(&a, None, i64::MAX).unwrap();
        b.prev[3] ^= 1;
        assert_eq!(r.mark(&b, None, i64::MAX), Err(Flag::ChainBreak(48_000)));
        assert_eq!(r.mark(&a, None, i64::MAX), Err(Flag::DuplicateMark(0)));
    }

    /// §14 items 1 and 3: a native stream off the native rates is refused, and a mark off the cadence is flagged; the writer refuses to write one.
    #[test]
    fn declarations_and_cadence_are_enforced() {
        assert_eq!(Decl { rate_num: 30_000, rate_den: 1001, ..Decl::video(30, Vec::new(), 1) }.validate(), Err(DeclError::NativeRateNotAllowed));
        assert_eq!(Decl { rate_num: 48, rate_den: 2, native: false, ..Decl::video(24, Vec::new(), 1) }.validate(), Err(DeclError::RateNotReduced));
        let d = Decl::video(25, Vec::new(), 1);
        assert!(d.is_mark(50) && !d.is_mark(51));
        assert_eq!(Marker::new(d.clone()).mark(51, 0, 0, Lock::Free, 0), Err(WriteError::OffCadence));
        let bad = Mark { index: 51, stamp: 0, unc_ns: 0, lock: Lock::Free, rate_ppm: 0, root: [0; 32], prev: [0; 32], sig: None };
        assert_eq!(Reader::new(d).unwrap().mark(&bad, None, i64::MAX), Err(Flag::MarkOffCadence(51)));
    }

    /// §5.1 legacy placement: at 30000/1001 exactly one frame per Eagle second carries the mark — the one nearest the boundary. Frames 0‥599 span 20.02 s, so they hold the boundaries of seconds 0 thru 20: twenty-one marks.
    #[test]
    fn legacy_rates_mark_the_frame_nearest_each_second() {
        let d = Decl::imported(Kind::Video, 30_000, 1001, Vec::new());
        let marked: Vec<i64> = (0..(30 * 20)).filter(|&n| d.is_mark(n)).collect();
        assert_eq!(marked.len(), 21);
        assert!(marked.windows(2).all(|w| w[1] - w[0] == 29 || w[1] - w[0] == 30), "one mark per second, 29 or 30 frames apart");
        for &n in &marked {
            let off = (d.eagle_of(n) as i128).rem_euclid(OPS);
            let off = off.min(OPS - off);
            assert!(off * 2 <= OPS * 1001 / 30_000, "frame {n} sits within half a period of its second");
        }
    }

    /// §12.2: drop-frame labels skip :00/:01 each minute but every tenth; non-drop counts plainly.
    #[test]
    fn timecode_renders_and_drops_frames() {
        let ndf = Decl::video(25, Vec::new(), 1);
        assert_eq!(timecode(&ndf, 25 * 3661 + 7).to_string(), "01:01:01:07");
        let df = Decl::imported(Kind::Video, 30_000, 1001, Vec::new());
        assert_eq!(timecode(&df, 1799).to_string(), "00:00:59;29");
        assert_eq!(timecode(&df, 1800).to_string(), "00:01:00;02", "labels ;00 and ;01 are skipped at minute one");
        assert_eq!(timecode(&df, 17_982).to_string(), "00:10:00;00", "the tenth minute keeps its labels");
    }

    /// LOCK's audio grid (vsf::grid, photon's wave frame names) IS a Tukutahi native audio stream: the same names at the same instants.
    #[test]
    fn the_lock_grid_is_the_audio_declaration() {
        let a = Decl::audio(Vec::new());
        let mut seed = 5u64;
        for _ in 0..100_000 {
            let k = (lcg(&mut seed) % 600_000_000_000_000) as i64 - 300_000_000_000_000;
            assert_eq!(a.eagle_of(k), crate::grid::sample_to_eagle(k));
            assert_eq!(a.index_of(a.eagle_of(k) + 7_000), crate::grid::eagle_to_sample(crate::grid::sample_to_eagle(k) + 7_000));
        }
    }

    /// §7.2 / Appendix B: the middle row IS the stamp, the edges sit half a readout either side.
    #[test]
    fn rolling_shutter_rows() {
        let (stamp, readout, rows) = (1_000_000i64, 16_000i64, 1081u32);
        assert_eq!(row_center(stamp, readout, rows, 540), stamp);
        assert_eq!(row_center(stamp, readout, rows, 0), stamp - readout / 2);
        assert_eq!(row_center(stamp, readout, rows, 1080), stamp + readout / 2);
        assert_eq!(row_center(stamp, 0, 1, 0), stamp);
        assert_eq!(exposure_start(stamp, 4_000), stamp - 2_000);
    }

    /// §13: the declaration and a mark round-trip thru their VSF fields, integers and e6 only.
    #[test]
    fn vsf_fields_round_trip() {
        let d = Decl::video(60, alloc::vec![7; 32], 1080);
        assert_eq!(decl_from_fields(&decl_fields(&d)).unwrap(), d);
        let mut w = Marker::new(d.clone());
        w.frame(-1, [1; 32], None).unwrap();
        let mut m = w.mark(0, d.eagle_of(0) + 12_345, 800_000, Lock::Holdover, -37).unwrap();
        assert_eq!(mark_from_fields(&mark_fields(&m)).unwrap(), m);
        m.sig = Some(alloc::vec![9; 64]);
        assert_eq!(mark_from_fields(&mark_fields(&m)).unwrap(), m);
        assert!(!decl_fields(&d).iter().chain(mark_fields(&m).iter()).any(|(_, v)| matches!(v, VsfType::f5(_) | VsfType::f6(_))), "no float anywhere (§14 item 2)");
    }
}
