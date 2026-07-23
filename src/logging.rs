//! Structured VSF logging sink — the ONE durable, self-describing, adb-pullable log for the whole
//! VERICHROME stack (lumis/chameleon/opsin/limbus), replacing logcat and scattered `println!`.
//!
//! Each record is a COMPLETE VSF document — `{creation_time (Eagle), section "log" {lvl, msg, val*}}`
//! — appended to `<dir>/verichrome.log.vsf`. The message stores as a pure-text TEMPLATE and every
//! interpolated value rides as its own TYPED `val` field: numbers-binary-at-rest, so `vsflog` /
//! `vsfinfo` pick the display base at READ time and a number never stringifies into storage.
//!
//! Ported from photon's proven sink (2026-07). Gated behind the `logging` feature (which implies
//! `std`); compiles to nothing otherwise. Single-process design — the file opens lazily and RETRIES
//! until the platform data dir is known (Android sets it partway through JNI startup), buffering the
//! earliest records so nothing before the dir lands is lost.

use crate::prelude::*;
#[cfg(feature = "logging")]
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
#[cfg(feature = "logging")]
use std::sync::Mutex;

/// Severity of a structured log record. The discriminant IS the on-disk `lvl` value, so these
/// numbers are wire-stable — append new levels at the end, never renumber.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

/// One captured log value, typed — becomes a native VSF field in the record.
pub enum LogValue {
    U(u128),
    I(i128),
    F(f64),
    B(bool),
    T(String),
}

/// Capture wrapper for `logf!` args. Inherent impls (numerics, bools) outrank the [`CapDisplay`]
/// blanket at method resolution, so typed capture is automatic and everything else degrades to
/// text — the autoref-free inherent-priority specialization photon proved out.
pub struct Cap<T>(pub T);

/// Lowest-priority capture: anything Display becomes text (prose, hex labels, hashes). Implemented
/// on `&Cap<T>` so the typed inherent impls on `Cap<X>` win the method probe without ambiguity.
pub trait CapDisplay {
    fn cap(self) -> LogValue;
}
impl<T: core::fmt::Display> CapDisplay for &Cap<T> {
    fn cap(self) -> LogValue {
        LogValue::T(self.0.to_string())
    }
}

/// Primitive-typed capture, unified under one generic inherent impl on [`Cap`] so integer-literal
/// inference resolves (a per-type inherent zoo made `{integer}` ambiguous).
pub trait CapPrim: Copy {
    fn to_log(self) -> LogValue;
}
macro_rules! cap_prim {
    ($($t:ty => $variant:ident as $conv:ty),* $(,)?) => {
        $(impl CapPrim for $t {
            fn to_log(self) -> LogValue { LogValue::$variant(self as $conv) }
        })*
    };
}
cap_prim! {
    u8 => U as u128, u16 => U as u128, u32 => U as u128, u64 => U as u128, u128 => U as u128, usize => U as u128,
    i8 => I as i128, i16 => I as i128, i32 => I as i128, i64 => I as i128, i128 => I as i128, isize => I as i128,
    f32 => F as f64, f64 => F as f64,
}
impl CapPrim for bool {
    fn to_log(self) -> LogValue {
        LogValue::B(self)
    }
}
impl<T: CapPrim> Cap<&T> {
    pub fn cap(self) -> LogValue {
        (*self.0).to_log()
    }
}

/// Render a template + captured values for a terminal/console surface (`vsflog` shares this walk).
/// Slots `{}`/`{spec}` substitute values in order (spec is a rendering hint only). `{{`/`}}` are
/// literal braces.
pub fn render_log_line(template: &str, vals: &[LogValue]) -> String {
    let mut out = String::with_capacity(template.len() + vals.len() * 8);
    let mut chars = template.chars().peekable();
    let mut next = 0usize;
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                for s in chars.by_ref() {
                    if s == '}' {
                        break;
                    }
                }
                if let Some(v) = vals.get(next) {
                    match v {
                        LogValue::U(n) => out.push_str(&n.to_string()),
                        LogValue::I(n) => out.push_str(&n.to_string()),
                        LogValue::F(n) => out.push_str(&n.to_string()),
                        LogValue::B(b) => out.push_str(if *b { "true" } else { "false" }),
                        LogValue::T(s) => out.push_str(s),
                    }
                }
                next += 1;
            }
            c => out.push(c),
        }
    }
    out
}

/// One decoded log record — the shared decode shape for the `vsflog` bin and any in-app viewer.
#[derive(Clone, Debug)]
pub struct LogRecord {
    /// Record creation time, eagle oscillations (0 = the record carried none).
    pub osc: i64,
    /// Severity 0..=4 (Trace..Error); u64::MAX = the record carried none.
    pub level: u64,
    /// The rendered message: the stored template with its typed `val` fields substituted at READ
    /// time (numbers live binary in the record; this is the display edge).
    pub msg: String,
    /// The record's raw bytes — one complete VSF document, so a viewer can hand it to
    /// `inspect_vsf` for the coloured structural view. `vsflog` ignores it.
    pub raw: Vec<u8>,
}

/// Decode complete records from a `verichrome.log.vsf` byte stream. Returns the records plus the
/// byte offset of the last COMPLETE record boundary — a half-written trailing record (mid-append)
/// is left for the next pass instead of being mis-decoded. Shared by the `vsflog` bin and any
/// in-app viewer so the two surfaces can never drift. Available without the `logging` feature so
/// pure readers (desktop tooling) don't drag in the file sink.
pub fn parse_log_records(buf: &[u8]) -> (Vec<LogRecord>, usize) {
    use crate::file_format::{VsfHeader, VsfSection};
    use crate::types::EtType;
    use crate::VsfType;
    let mut records = Vec::new();
    let mut off = 0usize;
    while off < buf.len() {
        let rest = &buf[off..];
        let Ok((header, header_end)) = VsfHeader::decode(rest) else {
            break; // incomplete tail — stop, retry next pass
        };
        let mut ptr = 0usize;
        let Ok(section) = VsfSection::parse(&rest[header_end..], &mut ptr) else {
            break;
        };
        let rec = header_end + ptr;
        if rec == 0 {
            break;
        }
        let level = section
            .get_field("lvl")
            .and_then(|f| f.values.first())
            .and_then(|v| {
                use crate::schema::FromVsfType;
                u64::from_vsf_type(v).ok()
            })
            .unwrap_or(u64::MAX);
        let template = section
            .get_field("msg")
            .and_then(|f| f.values.first())
            .and_then(|v| match v {
                VsfType::a(s) | VsfType::x(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        // Numbers are stored auto-sized (VsfType::u/i6/f6) but decode back as the smallest fixed
        // width that fits (u3/u4/u5/u6, i3..i7, f5/f6). FromVsfType folds every width variant back
        // to one wide type, so try unsigned → signed → float → text in that order.
        use crate::schema::FromVsfType;
        let vals: Vec<LogValue> = section
            .get_fields("val")
            .iter()
            .filter_map(|f| f.values.first())
            .map(|v| match v {
                VsfType::a(s) | VsfType::x(s) => LogValue::T(s.clone()),
                _ => {
                    if let Ok(n) = u64::from_vsf_type(v) {
                        LogValue::U(n as u128)
                    } else if let Ok(n) = i64::from_vsf_type(v) {
                        LogValue::I(n as i128)
                    } else if let Ok(n) = f64::from_vsf_type(v) {
                        LogValue::F(n)
                    } else {
                        LogValue::T(format!("{v:?}"))
                    }
                }
            })
            .collect();
        let msg = if vals.is_empty() {
            template
        } else {
            render_log_line(&template, &vals)
        };
        let osc = match &header.creation_time {
            Some(VsfType::e(EtType::e6(o))) => *o,
            Some(VsfType::e(EtType::e5(o))) => *o as i64,
            Some(VsfType::e(EtType::e7(o))) => *o as i64,
            _ => 0,
        };
        records.push(LogRecord { osc, level, msg, raw: rest[..rec].to_vec() });
        off += rec;
    }
    (records, off)
}

// ─────────────────────────── file sink (feature = "logging") ───────────────────────────

/// The log filename. Logging is a dev-build feature, so adb-pull discoverability beats filename
/// privacy.
pub const LOG_FILENAME: &str = "verichrome.log.vsf";

#[cfg(feature = "logging")]
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

// Records that arrive before the sink can open (Android: everything logged before the JNI data dir
// lands) — held as already-built VSF record bytes so their creation stamps stay true, drained the
// moment the file opens. Bounded so a never-initializing process can't grow it unbounded; overflow
// drops the newest record (the earliest lines are the ones worth keeping).
#[cfg(feature = "logging")]
static LOG_PENDING: Mutex<Vec<u8>> = Mutex::new(Vec::new());
#[cfg(feature = "logging")]
const LOG_PENDING_CAP: usize = 64 << 10;

// Trim on EITHER cap: file past 16 MiB → drop oldest whole records back to ~8 MiB; oldest record
// past a jittered 24–48h → cut back to a jittered 12–24h keep window. Cuts land only on record
// boundaries (the file is a stream of complete VSF records), so the result stays fully decodable.
#[cfg(feature = "logging")]
const LOG_CAP_BYTES: u64 = 16 << 20;
#[cfg(feature = "logging")]
const LOG_TRIM_TO_BYTES: u64 = 8 << 20;
#[cfg(feature = "logging")]
static LOG_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "logging")]
const LOG_AGE_TRIGGER_BASE_OSC: i64 =
    2 * 24 * 60 * 60 * crate::OSCILLATIONS_PER_SECOND as i64; // jittered → 24–48h
#[cfg(feature = "logging")]
const LOG_AGE_KEEP_BASE_OSC: i64 = 24 * 60 * 60 * crate::OSCILLATIONS_PER_SECOND as i64; // → 12–24h
#[cfg(feature = "logging")]
static LOG_AGE_TRIGGER_OSC: AtomicI64 = AtomicI64::new(LOG_AGE_TRIGGER_BASE_OSC);
#[cfg(feature = "logging")]
static LOG_OLDEST_OSC: AtomicI64 = AtomicI64::new(i64::MAX);

// Explicit log-dir override, set once at startup. On Android, JNI passes the EXTERNAL files dir
// (adb-readable on a non-debuggable release dev APK where internal files/ is not); desktop passes
// its config/data dir.
#[cfg(feature = "logging")]
static LOG_DIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Set where the VSF log file goes. Call once at startup (desktop `main`, Android JNI); first call
/// wins. Until it's set, records buffer in memory (bounded) so nothing is lost.
#[cfg(feature = "logging")]
pub fn set_log_dir(dir: String) {
    if !dir.is_empty() {
        let _ = LOG_DIR.set(dir);
    }
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn set_log_dir(_dir: String) {}

// The log filename. Defaults to LOG_FILENAME; override per-process so multiple processes of one app
// (e.g. lumis UI + camera) never interleave appends into a single file. First call wins.
#[cfg(feature = "logging")]
static LOG_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Override the log filename (per-process, to avoid interleaved appends when several processes of
/// one app log at once). Call once at startup before the first log; first call wins.
#[cfg(feature = "logging")]
pub fn set_log_name(name: String) {
    if !name.is_empty() {
        let _ = LOG_NAME.set(name);
    }
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn set_log_name(_name: String) {}

#[cfg(feature = "logging")]
fn log_name() -> &'static str {
    LOG_NAME.get().map(String::as_str).unwrap_or(LOG_FILENAME)
}

#[cfg(feature = "logging")]
fn log_dir() -> Option<std::path::PathBuf> {
    LOG_DIR.get().map(std::path::PathBuf::from)
}

// Dependency-free jitter: a fresh xorshift seeded from the clock, scaling `base` by [0.5, 1.0].
// A fixed interval makes every subsystem trim on the same tick; jittering spreads it. Log timing
// never needs an exact deadline, so this cheap PRNG (no `rand` dep pulled into `logging`) suffices.
#[cfg(feature = "logging")]
fn jitter(base: i64) -> i64 {
    let mut x = crate::eagle_time_oscillations() as u64 ^ 0x9E3779B97F4A7C15;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let frac = 0.5 + (x >> 11) as f64 / (1u64 << 53) as f64 * 0.5;
    (base as f64 * frac) as i64
}

/// Build one VSF log record: `{creation_time (Eagle now), section "log" {lvl, msg, val*}}`.
#[cfg(feature = "logging")]
fn build_record(level: LogLevel, msg: &str, vals: &[LogValue]) -> Result<Vec<u8>, String> {
    let mut section = crate::VsfSection::new("log");
    section.add_field_multi("lvl", vec![crate::VsfType::u(level as usize, false)]);
    section.add_field_multi("msg", vec![crate::VsfType::a(msg.to_string())]);
    for v in vals {
        let t = match v {
            LogValue::U(n) => crate::VsfType::u(*n as usize, false),
            LogValue::I(n) => crate::VsfType::i6(*n as i64),
            LogValue::F(n) => crate::VsfType::f6(*n),
            LogValue::B(b) => crate::VsfType::u(*b as usize, false),
            LogValue::T(s) => crate::VsfType::a(s.clone()),
        };
        section.add_field_multi("val", vec![t]);
    }
    crate::VsfBuilder::new()
        .creation_time_oscillations(crate::eagle_time_oscillations())
        .provenance_only()
        .add_section_direct(section)
        .build()
}

#[cfg(feature = "logging")]
fn append_log_record(level: LogLevel, msg: &str, vals: &[LogValue]) {
    use std::io::Write;
    // Build first so a buffered record carries the stamp of when it was LOGGED, not when the sink
    // finally opened.
    let record = build_record(level, msg, vals);
    let Ok(mut guard) = LOG_FILE.lock() else {
        return;
    };
    if guard.is_none() {
        if let Some(dir) = log_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join(log_name());
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                // Drain the pre-dir buffer FIRST so the file stays chronological, then seed counters.
                if let Ok(mut pending) = LOG_PENDING.lock() {
                    if !pending.is_empty() {
                        let _ = f.write_all(&pending);
                        pending.clear();
                        pending.shrink_to_fit();
                    }
                }
                let sz = f.metadata().map(|m| m.len()).unwrap_or(0);
                LOG_BYTES.store(sz, Ordering::Relaxed);
                LOG_OLDEST_OSC.store(first_record_osc(&path).unwrap_or(i64::MAX), Ordering::Relaxed);
                LOG_AGE_TRIGGER_OSC.store(jitter(LOG_AGE_TRIGGER_BASE_OSC), Ordering::Relaxed);
                *guard = Some(f);
            }
        }
    }
    let Some(file) = guard.as_mut() else {
        // No sink yet (Android before the JNI data dir lands): hold the built record.
        if let (Ok(bytes), Ok(mut pending)) = (&record, LOG_PENDING.lock()) {
            if pending.len() + bytes.len() <= LOG_PENDING_CAP {
                pending.extend_from_slice(bytes);
            }
        }
        return;
    };
    if let Ok(bytes) = record {
        let _ = file.write_all(&bytes);
        let _ = file.flush();
        let total = LOG_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed) + bytes.len() as u64;
        let now = crate::eagle_time_oscillations();
        let oldest = LOG_OLDEST_OSC.load(Ordering::Relaxed);
        let trigger = LOG_AGE_TRIGGER_OSC.load(Ordering::Relaxed);
        let aged = oldest != i64::MAX && now.saturating_sub(oldest) > trigger;
        if total > LOG_CAP_BYTES || aged {
            if let Some((trimmed, new_size, new_oldest)) = trim_log_file(now) {
                *guard = Some(trimmed);
                LOG_BYTES.store(new_size, Ordering::Relaxed);
                LOG_OLDEST_OSC.store(new_oldest, Ordering::Relaxed);
                LOG_AGE_TRIGGER_OSC.store(jitter(LOG_AGE_TRIGGER_BASE_OSC), Ordering::Relaxed);
            }
        }
    }
}

// Drop the oldest whole records — enough to get under LOG_TRIM_TO_BYTES AND to drop anything older
// than the jittered keep window — then reopen the file for appending. Cuts only on record
// boundaries. Returns (reopened append handle, kept byte count, new oldest time); None if the file
// couldn't be read/rewritten (the cap check just retries next line).
#[cfg(feature = "logging")]
fn trim_log_file(now_osc: i64) -> Option<(std::fs::File, u64, i64)> {
    use std::io::Write;
    let path = log_dir()?.join(log_name());
    let bytes = std::fs::read(&path).ok()?;
    let age_cutoff = now_osc.saturating_sub(jitter(LOG_AGE_KEEP_BASE_OSC));
    let (keep, new_oldest) = log_keep_offset(&bytes, LOG_TRIM_TO_BYTES, age_cutoff);
    let kept = &bytes[keep.min(bytes.len())..];
    let mut w = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .ok()?;
    w.write_all(kept).ok()?;
    w.flush().ok()?;
    drop(w);
    let appender = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()?;
    Some((appender, kept.len() as u64, new_oldest))
}

// Pure boundary finder: the first whole-record boundary to keep so that `bytes[offset..]` is both
// within `trim_to_size` bytes AND free of records older than `age_cutoff_osc`. Records are appended
// in time order, so drop from the front while a record is EITHER before the size-drop point OR
// older than the cutoff, stopping at the first satisfying both. Returns (keep_offset,
// oldest_kept_time). Stops early on any decode error so a corrupt tail never causes a mid-record cut.
#[cfg(feature = "logging")]
fn log_keep_offset(bytes: &[u8], trim_to_size: u64, age_cutoff_osc: i64) -> (usize, i64) {
    let total = bytes.len();
    let size_drop = (total as u64).saturating_sub(trim_to_size) as usize;
    let mut offset = 0usize;
    while offset < total {
        let rest = &bytes[offset..];
        let (header, header_end) = match crate::file_format::VsfHeader::decode(rest) {
            Ok(h) => h,
            Err(_) => return (offset, i64::MAX),
        };
        let mut ptr = 0usize;
        if crate::file_format::VsfSection::parse(&rest[header_end..], &mut ptr).is_err() {
            return (offset, i64::MAX);
        }
        let rec = header_end + ptr;
        if rec == 0 {
            return (offset, i64::MAX);
        }
        let t = match &header.creation_time {
            Some(crate::VsfType::e(et)) => et_to_osc(et),
            _ => i64::MIN,
        };
        if offset >= size_drop && t >= age_cutoff_osc {
            return (offset, t);
        }
        offset += rec;
    }
    (total, i64::MAX)
}

#[cfg(feature = "logging")]
fn et_to_osc(et: &crate::types::EtType) -> i64 {
    use crate::types::EtType;
    match et {
        EtType::e5(o) => *o as i64,
        EtType::e6(o) => *o,
        EtType::e7(o) => *o as i64,
        _ => i64::MIN,
    }
}

#[cfg(feature = "logging")]
fn first_record_osc(path: &std::path::Path) -> Option<i64> {
    use std::io::Read;
    let mut buf = vec![0u8; 4096];
    let n = std::fs::File::open(path).ok()?.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    let (header, _) = crate::file_format::VsfHeader::decode(&buf[..n]).ok()?;
    match &header.creation_time {
        Some(crate::VsfType::e(et)) => Some(et_to_osc(et)),
        _ => None,
    }
}

/// Live size of the open VSF log — one atomic load, no I/O. Cheap "anything logged since?" probe.
#[cfg(feature = "logging")]
pub fn log_size_bytes() -> u64 {
    LOG_BYTES.load(Ordering::Relaxed)
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn log_size_bytes() -> u64 {
    0
}

/// The current on-disk log as raw bytes (for submission / "save log"). `None` if not yet opened or
/// unreadable.
#[cfg(feature = "logging")]
pub fn snapshot_log_bytes() -> Option<Vec<u8>> {
    let path = log_dir()?.join(log_name());
    std::fs::read(&path).ok().filter(|b| !b.is_empty())
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn snapshot_log_bytes() -> Option<Vec<u8>> {
    None
}

/// Read the on-disk log from byte `offset` to EOF — a viewer's tail-follow read. `None` = no log
/// yet or nothing past the offset; a shrunken file (rotation/clear) reads `None` and the caller
/// re-syncs from zero.
#[cfg(feature = "logging")]
pub fn read_log_from(offset: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let path = log_dir()?.join(log_name());
    let mut f = std::fs::File::open(&path).ok()?;
    let len = f.metadata().ok()?.len();
    if len <= offset {
        return None;
    }
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut out = Vec::with_capacity((len - offset) as usize);
    f.read_to_end(&mut out).ok()?;
    Some(out)
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn read_log_from(_offset: u64) -> Option<Vec<u8>> {
    None
}

/// Wipe the durable log (a "clear logs" action). Removes the file and drops the open handle so the
/// next write reopens a fresh, empty file.
#[cfg(feature = "logging")]
pub fn clear_log() {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(dir) = log_dir() {
            let _ = std::fs::remove_file(dir.join(log_name()));
        }
        *guard = None;
        LOG_BYTES.store(0, Ordering::Relaxed);
        LOG_OLDEST_OSC.store(i64::MAX, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn clear_log() {}

/// The structured logging entry point. `logf!`/`logf_at!` expand to this.
#[cfg(feature = "logging")]
pub fn log_structured(level: LogLevel, template: &str, vals: Vec<LogValue>) {
    append_log_record(level, template, &vals);
}
#[cfg(not(feature = "logging"))]
#[inline(always)]
pub fn log_structured(_level: LogLevel, _template: &str, _vals: Vec<LogValue>) {}

/// Plain-text convenience (Info). Prefer `logf!` when there are interpolated values.
pub fn log(msg: &str) {
    log_structured(LogLevel::Info, msg, Vec::new());
}

/// Plain-text convenience with an explicit level.
pub fn log_at(level: LogLevel, msg: &str) {
    log_structured(level, msg, Vec::new());
}

// ─────────────────────────── `log` crate bridge (feature = "log-bridge") ───────────────────────────

/// Route the `log` crate into the VSF sink so records from dependencies that use `log` macros
/// (fluor, the JNI platform layer, reqwest, …) land in the SAME durable file. Debug+ globally,
/// known-noisy crates only at Warn+. Call once at startup; a repeat call is a harmless no-op.
#[cfg(all(feature = "logging", feature = "log-bridge"))]
pub fn install_log_bridge() {
    struct VsfLogBridge;
    impl log::Log for VsfLogBridge {
        fn enabled(&self, meta: &log::Metadata) -> bool {
            const NOISY: &[&str] =
                &["cosmic_text", "reqwest", "naga", "wgpu", "rustls", "hyper", "h2"];
            let t = meta.target();
            let noisy = NOISY.iter().any(|p| t.starts_with(p));
            !noisy || meta.level() <= log::Level::Warn
        }
        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let lvl = match record.level() {
                log::Level::Error => LogLevel::Error,
                log::Level::Warn => LogLevel::Warn,
                log::Level::Info => LogLevel::Info,
                log::Level::Debug => LogLevel::Debug,
                log::Level::Trace => LogLevel::Trace,
            };
            append_log_record(lvl, &format!("{}: {}", record.target(), record.args()), &[]);
        }
        fn flush(&self) {}
    }
    static BRIDGE: VsfLogBridge = VsfLogBridge;
    if log::set_logger(&BRIDGE).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
}

/// format!-shaped structured log at Info: `logf!("scan took {} ms", n)` — the template stores as
/// pure text, `n` as a typed field.
#[macro_export]
macro_rules! logf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::logging::CapDisplay as _;
        $crate::logging::log_structured(
            $crate::logging::LogLevel::Info,
            $fmt,
            $crate::prelude::vec![$($crate::logging::Cap(&$arg).cap()),*],
        );
    }};
}

/// [`logf!`] with an explicit level.
#[macro_export]
macro_rules! logf_at {
    ($level:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::logging::CapDisplay as _;
        $crate::logging::log_structured(
            $level,
            $fmt,
            $crate::prelude::vec![$($crate::logging::Cap(&$arg).cap()),*],
        );
    }};
}

#[cfg(all(test, feature = "logging"))]
mod tests {
    use super::*;

    fn record_at(msg: &str, osc: i64) -> Vec<u8> {
        crate::VsfBuilder::new()
            .creation_time_oscillations(osc)
            .provenance_only()
            .add_section(
                "log",
                vec![
                    ("lvl".to_string(), crate::VsfType::u(2, false)),
                    ("msg".to_string(), crate::VsfType::a(msg.to_string())),
                ],
            )
            .build()
            .unwrap()
    }

    #[test]
    fn trim_cuts_on_a_record_boundary_and_stays_decodable() {
        let mut bytes = Vec::new();
        let mut starts = vec![0usize];
        for i in 0..50 {
            bytes.extend_from_slice(&record_at(&format!("message number {i}"), 1000 + i));
            starts.push(bytes.len());
        }
        let trim_to = (bytes.len() / 3) as u64;
        let (keep, _oldest) = log_keep_offset(&bytes, trim_to, i64::MIN);
        assert!(starts.contains(&keep), "cut at {keep} is not a record boundary");
        assert!(keep > 0);
        let kept = &bytes[keep..];
        assert!(kept.len() as u64 <= trim_to && !kept.is_empty());
        let mut off = 0usize;
        while off < kept.len() {
            let (_, he) = crate::file_format::VsfHeader::decode(&kept[off..]).unwrap();
            let mut p = 0usize;
            crate::file_format::VsfSection::parse(&kept[off + he..], &mut p).unwrap();
            off += he + p;
        }
        assert_eq!(off, kept.len());
    }

    #[test]
    fn age_cap_drops_records_older_than_cutoff() {
        let mut bytes = Vec::new();
        for i in 0..10 {
            bytes.extend_from_slice(&record_at(&format!("old {i}"), 1000));
        }
        let new_start = bytes.len();
        for i in 0..10 {
            bytes.extend_from_slice(&record_at(&format!("new {i}"), 9000));
        }
        let (keep, oldest) = log_keep_offset(&bytes, 64 << 20, 5000);
        assert_eq!(keep, new_start);
        assert_eq!(oldest, 9000);
    }

    #[test]
    fn parse_roundtrips_template_and_typed_vals() {
        let mut section = crate::VsfSection::new("log");
        section.add_field_multi("lvl", vec![crate::VsfType::u(3, false)]);
        section.add_field_multi("msg", vec![crate::VsfType::a("scan took {} ms".to_string())]);
        section.add_field_multi("val", vec![crate::VsfType::u(8109, false)]);
        let rec = crate::VsfBuilder::new()
            .creation_time_oscillations(42)
            .provenance_only()
            .add_section_direct(section)
            .build()
            .unwrap();
        let (records, off) = parse_log_records(&rec);
        assert_eq!(off, rec.len());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, 3);
        assert_eq!(records[0].osc, 42);
        assert_eq!(records[0].msg, "scan took 8109 ms");
    }
}
