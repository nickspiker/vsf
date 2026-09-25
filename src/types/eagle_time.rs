use chrono::{DateTime, TimeZone, Utc};
#[cfg(not(feature = "std"))]
use num_traits::Float;

/// Oscillations per Eagle second.
///
/// One Eagle second is defined as exactly 1,420,407,826 complete oscillation periods of the electromagnetic radiation emitted during the hydrogen-1 (protium) hyperfine transition between F=0 and F=1 ground states, as measured at the barycentric reference frame of the Milky Way-Andromeda galaxy system.
///
/// This is the 21cm hydrogen line at 1420.405751 MHz. One oscillation period ≈ 704.032 picoseconds.
pub const OSCILLATIONS_PER_SECOND: u64 = 1_420_407_826;

/// Eagle Time type - represents time values as oscillation counts.
///
/// Binary format: `[e][size][value...]`
/// - `[e]` = Eagle Time marker
/// - `[size]` = 5/6/7 (base36 bit-width digit: 2^5=32, 2^6=64, 2^7=128 bits)
/// - `[value]` = fixed-width big-endian integer
///
/// **Oscillation counts (canonical):**
/// - Each count = one complete 21cm hydrogen-1 hyperfine transition
/// - Precision: 704.032 picoseconds per oscillation
/// - e5 (i32): ±1.5 years range — short intervals, packed structs
/// - e6 (i64): ±206 years range — standard timestamps (covers 1763–2175)
/// - e7 (i128): astronomical/geological range — future use
/// - Positive = after epoch, negative = before epoch
///
/// **Deprecated float variants (legacy, readers accept, writers must not emit):**
/// - ef5 (f32): ~2 minute effective precision — never use in new code
/// - ef6 (f64): ~200 nanosecond effective precision — never use in new code
#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum EtType {
    /// 32-bit oscillation count. Wire: `e5` + 4 bytes i32 BE.
    e5(i32),
    /// 64-bit oscillation count. Wire: `e6` + 8 bytes i64 BE. Standard form.
    e6(i64),
    /// 128-bit oscillation count. Wire: `e7` + 16 bytes i128 BE.
    e7(i128),
    /// Deprecated: seconds as f32. Wire: `ef5` + 4 bytes. Use e6 instead.
    #[deprecated(since = "0.3.5", note = "Use EtType::e6 (oscillation count)")]
    f5(f32),
    /// Deprecated: seconds as f64. Wire: `ef6` + 8 bytes. Use e6 instead.
    #[deprecated(since = "0.3.5", note = "Use EtType::e6 (oscillation count)")]
    f6(f64),
}

/// EagleTime represents a point in time in the Eagle Time standard.
///
/// Stores time as oscillation counts of the 21cm hydrogen-1 hyperfine transition.
///
/// Eagle epoch: the Apollo 11 lunar TOUCHDOWN (≈1969-07-20T20:17:47.575 TAI), pinned by fiat to the integer TAI second 20:17:48 so Eagle second boundaries coincide with TAI, GPS and UTC second boundaries — see [`EAGLE_EPOCH_TAI_SECS`]. Stamps minted by `eagle_time_now` today still count POSIX seconds from the legacy label 20:17:40 UTC ([`EAGLE_EPOCH_UNIX_SECS`]), which runs exactly [`LOCK_MINUS_LEGACY_SECS`] behind the TAI definition for any instant since 2017.
///
/// This definition:
/// - Uses the most abundant element in the universe (hydrogen-1)
/// - Is measurable with any 21cm radio receiver
/// - Accounts for gravitational time dilation in the frequency measurement
/// - Provides universal verifiability without trusted authorities
/// - Achieves picosecond precision with standard integer types
///
/// # Precision Characteristics
///
/// | Type | Range           | Precision | Use Case                           |
/// |------|-----------------|-----------|------------------------------------|
/// | e5   | ±1.5 years      | 704 ps    | Short intervals, packed structs    |
/// | e6   | ±206 years      | 704 ps    | Absolute timestamps (default)      |
/// | e7   | astronomical    | 704 ps    | Geological/astronomical timestamps |
#[derive(Debug, Clone)]
pub struct EagleTime {
    et_seconds: EtType,
}

impl EagleTime {
    /// Creates a new EagleTime instance from a VsfType.
    ///
    /// Integer types are interpreted as oscillation counts. Float types are interpreted as seconds.
    ///
    /// # Panics
    /// Panics if the VsfType is not a valid numeric variant or EagleTime type.
    pub fn new_from_vsf(value: crate::types::VsfType) -> Self {
        use crate::types::VsfType;

        #[allow(deprecated)]
        let et_seconds = match value {
            // Handle VsfType::e (already an EagleTime wrapper)
            VsfType::e(et) => et,
            // Float types: already in seconds (deprecated, preserved for compat)
            VsfType::f5(v) => EtType::f5(v),
            VsfType::f6(v) => EtType::f6(v),
            // Unsigned integer types: convert to i64 oscillation counts
            VsfType::u(v, false) => EtType::e6(v as i64),
            VsfType::u3(v) => EtType::e6(v as i64),
            VsfType::u4(v) => EtType::e6(v as i64),
            VsfType::u5(v) => EtType::e6(v as i64),
            VsfType::u6(v) => EtType::e6(v as i64),
            // Signed integer types: direct i64
            VsfType::i(v) => EtType::e6(v as i64),
            VsfType::i3(v) => EtType::e6(v as i64),
            VsfType::i4(v) => EtType::e6(v as i64),
            VsfType::i5(v) => EtType::e6(v as i64),
            VsfType::i6(v) => EtType::e6(v as i64),
            _ => panic!("EagleTime must be created with a valid numeric VsfType variant"),
        };
        EagleTime { et_seconds }
    }

    /// Creates a new EagleTime directly from an EtType
    pub fn new(et_seconds: EtType) -> Self {
        EagleTime { et_seconds }
    }

    /// Creates an EagleTime from an oscillation count (i64)
    pub fn from_oscillations(count: i64) -> Self {
        EagleTime {
            et_seconds: EtType::e6(count),
        }
    }

    /// Creates an EagleTime from seconds (f64), converting to oscillation count
    pub fn from_seconds_f64(seconds: f64) -> Self {
        let oscillations = (seconds * OSCILLATIONS_PER_SECOND as f64).round() as i64;
        EagleTime {
            et_seconds: EtType::e6(oscillations),
        }
    }

    /// Creates an EagleTime from seconds (f32), converting to oscillation count
    pub fn from_seconds_f32(seconds: f32) -> Self {
        let oscillations = (seconds * OSCILLATIONS_PER_SECOND as f32).round() as i64;
        EagleTime {
            et_seconds: EtType::e6(oscillations),
        }
    }

    /// Converts the current EagleTime to a VsfType.
    pub fn to_vsf_type(&self) -> crate::types::VsfType {
        use crate::types::VsfType;

        #[allow(deprecated)]
        match self.et_seconds {
            EtType::e5(v) => VsfType::i5(v),
            EtType::e6(v) => VsfType::i6(v),
            EtType::e7(v) => VsfType::i6(v as i64), // lossy, best effort
            EtType::f5(v) => VsfType::f5(v),
            EtType::f6(v) => VsfType::f6(v),
        }
    }

    /// Converts the EagleTime to a UTC DateTime.
    ///
    /// For integer types, divides oscillation count by OSCILLATIONS_PER_SECOND. For float types, uses the stored seconds directly.
    ///
    /// Returns None if the timestamp is outside chrono's representable range.
    pub fn to_datetime_opt(&self) -> Option<DateTime<Utc>> {
        #[allow(deprecated)]
        let osc = match self.et_seconds {
            EtType::e5(_) | EtType::e6(_) | EtType::e7(_) => i64::try_from(self.oscillations_i128()?).ok()?,
            // A deprecated float stamp is seconds already; it takes the float path it was written with.
            EtType::f5(_) | EtType::f6(_) => {
                let s = self.to_seconds_f64();
                (s * OSCILLATIONS_PER_SECOND as f64).round() as i64
            }
        };
        let (secs, nanos) = to_unix_ns(osc);
        Utc.timestamp_opt(secs, nanos).single()
    }

    /// Converts the EagleTime to a UTC DateTime.
    ///
    /// Panics if the timestamp is outside chrono's representable range. For non-panicking version, use `to_datetime_opt()`.
    pub fn to_datetime(&self) -> DateTime<Utc> {
        self.to_datetime_opt().unwrap_or_else(|| {
            panic!(
                "Timestamp outside representable range: {:?}",
                self.et_seconds
            )
        })
    }

    /// Get a reference to the underlying EtType
    pub fn et_type(&self) -> &EtType {
        &self.et_seconds
    }

    /// Converts to f64 seconds, regardless of storage type.
    ///
    /// For integer types: divides oscillation count by OSCILLATIONS_PER_SECOND. For float types (deprecated): returns the stored seconds directly.
    pub fn to_seconds_f64(&self) -> f64 {
        #[allow(deprecated)]
        match self.et_seconds {
            EtType::e5(oscillations) => oscillations as f64 / OSCILLATIONS_PER_SECOND as f64,
            EtType::e6(oscillations) => oscillations as f64 / OSCILLATIONS_PER_SECOND as f64,
            EtType::e7(oscillations) => oscillations as f64 / OSCILLATIONS_PER_SECOND as f64,
            EtType::f5(seconds) => seconds as f64,
            EtType::f6(seconds) => seconds,
        }
    }

    /// Converts to f32 seconds, regardless of storage type.
    pub fn to_seconds_f32(&self) -> f32 {
        #[allow(deprecated)]
        match self.et_seconds {
            EtType::e5(oscillations) => oscillations as f32 / OSCILLATIONS_PER_SECOND as f32,
            EtType::e6(oscillations) => oscillations as f32 / OSCILLATIONS_PER_SECOND as f32,
            EtType::e7(oscillations) => oscillations as f32 / OSCILLATIONS_PER_SECOND as f32,
            EtType::f5(seconds) => seconds,
            EtType::f6(seconds) => seconds as f32,
        }
    }

    /// Returns the oscillation count as i64 if stored as an integer type, None for deprecated float types.
    pub fn oscillations(&self) -> Option<i64> {
        #[allow(deprecated)]
        match self.et_seconds {
            EtType::e5(v) => Some(v as i64),
            EtType::e6(v) => Some(v),
            EtType::e7(v) => Some(v as i64), // truncates for very large values
            EtType::f5(_) | EtType::f6(_) => None,
        }
    }

    /// Returns the full i128 oscillation count. Covers e5/e6/e7 without truncation.
    pub fn oscillations_i128(&self) -> Option<i128> {
        #[allow(deprecated)]
        match self.et_seconds {
            EtType::e5(v) => Some(v as i128),
            EtType::e6(v) => Some(v as i128),
            EtType::e7(v) => Some(v),
            EtType::f5(_) | EtType::f6(_) => None,
        }
    }

    /// Returns the picosecond precision timestamp (oscillations × 704.032 ps). Returns None for deprecated float types.
    pub fn picoseconds(&self) -> Option<i128> {
        self.oscillations_i128().map(|osc| (osc * 704_032) / 1000)
    }
}

impl PartialEq for EagleTime {
    fn eq(&self, other: &Self) -> bool {
        #[allow(deprecated)]
        match (&self.et_seconds, &other.et_seconds) {
            (EtType::e5(a), EtType::e5(b)) => a == b,
            (EtType::e6(a), EtType::e6(b)) => a == b,
            (EtType::e7(a), EtType::e7(b)) => a == b,
            // Cross-width integer comparison via i128
            (a, b)
                if matches!(a, EtType::e5(_) | EtType::e6(_) | EtType::e7(_))
                    && matches!(b, EtType::e5(_) | EtType::e6(_) | EtType::e7(_)) =>
            {
                self.oscillations_i128() == other.oscillations_i128()
            }
            // Float involvement: fall back to f64
            _ => self.to_seconds_f64() == other.to_seconds_f64(),
        }
    }
}

impl Eq for EagleTime {}

impl PartialOrd for EagleTime {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EagleTime {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        #[allow(deprecated)]
        match (&self.et_seconds, &other.et_seconds) {
            (EtType::e6(a), EtType::e6(b)) => a.cmp(b),
            // Cross-width integer comparison via i128
            _ if self.oscillations_i128().is_some() && other.oscillations_i128().is_some() => {
                self.oscillations_i128().cmp(&other.oscillations_i128())
            }
            // Float involvement: fall back to f64
            _ => self
                .to_seconds_f64()
                .partial_cmp(&other.to_seconds_f64())
                .unwrap_or(core::cmp::Ordering::Equal),
        }
    }
}

// ---------------------------------------------------------------------------
// INTEGER ABSOLUTE CONVERSIONS (LOCK spec §2, 2026-09-24). No f64 anywhere on an absolute path: an f64 cannot hold a current oscillation count (~1.2e18) exactly, so the old `(secs + nanos/1e9) * OPS` product was only good to ~256 oscillations (~180 ns). Everything below is i128, round half up, div_euclid/rem_euclid so negative instants round the same way.
// ---------------------------------------------------------------------------

/// The LEGACY epoch label: 1969-07-20T20:17:40 UTC as signed Unix seconds. `eagle_time_now` counts POSIX seconds from here — the system clock's UTC, leap seconds folded away as POSIX folds them.
pub const EAGLE_EPOCH_UNIX_SECS: i64 = -14_182_940;

/// The LOCK epoch: 1969-07-20T20:17:48 TAI as signed seconds since 1970-01-01T00:00:00 TAI. An integer TAI second by fiat, 0.427598 s after the touchdown (TAI−UTC was 7.572402 s that day under the 1968–72 rubber-second rule), so every Eagle second boundary is a TAI/GPS/UTC second boundary.
pub const EAGLE_EPOCH_TAI_SECS: i64 = -14_182_932;

/// How far the LOCK (TAI) count runs ahead of the legacy (POSIX) count for any instant since 2017-01-01: TAI−UTC (37 s) − 7.572402 s − 0.427598 s = exactly 29 s. The chosen epoch makes the difference an integer on purpose. For an older instant it is `tai_minus_utc(t) − 8`.
pub const LOCK_MINUS_LEGACY_SECS: i64 = 29;

const NS: i128 = 1_000_000_000;
const OPS_I128: i128 = OSCILLATIONS_PER_SECOND as i128;

fn div_round_half_up(n: i128, d: i128) -> i128 {
    n.div_euclid(d) + ((n.rem_euclid(d) * 2 >= d) as i128)
}

fn ns_to_osc(ns: i128) -> i64 {
    div_round_half_up(ns * OPS_I128, NS) as i64
}

fn osc_to_ns(osc: i64) -> i128 {
    div_round_half_up(osc as i128 * NS, OPS_I128)
}

/// TAI instant (seconds since 1970-01-01 TAI, nanoseconds) → LOCK Eagle oscillations. Exact: every nanosecond maps to a distinct count (1.42 counts per ns) and `to_tai_ns` recovers it.
pub fn from_tai_ns(secs: i64, nanos: u32) -> i64 {
    ns_to_osc((secs as i128 - EAGLE_EPOCH_TAI_SECS as i128) * NS + nanos as i128)
}

/// LOCK Eagle oscillations → TAI instant (seconds since 1970-01-01 TAI, nanoseconds).
pub fn to_tai_ns(osc: i64) -> (i64, u32) {
    let ns = osc_to_ns(osc);
    ((ns.div_euclid(NS) + EAGLE_EPOCH_TAI_SECS as i128) as i64, ns.rem_euclid(NS) as u32)
}

/// Unix instant (POSIX seconds, nanoseconds) → LEGACY Eagle oscillations — what every stamp minted today is.
pub fn from_unix_ns(secs: i64, nanos: u32) -> i64 {
    ns_to_osc((secs as i128 - EAGLE_EPOCH_UNIX_SECS as i128) * NS + nanos as i128)
}

/// LEGACY Eagle oscillations → Unix instant (POSIX seconds, nanoseconds).
pub fn to_unix_ns(osc: i64) -> (i64, u32) {
    let ns = osc_to_ns(osc);
    ((ns.div_euclid(NS) + EAGLE_EPOCH_UNIX_SECS as i128) as i64, ns.rem_euclid(NS) as u32)
}

/// TAI − UTC in whole seconds, as data: (Unix second at which the offset takes effect, the offset). Leap seconds are announced six months ahead by IERS Bulletin C — a new row here is the whole update, and `set_leap_table` replaces the table at runtime without a code change. Before 1972 the offset was fractional (rubber seconds); the table starts at the integer era.
pub const LEAP_TABLE: &[(i64, i32)] = &[
    (63072000, 10), // 1972-01-01
    (78796800, 11), // 1972-07-01
    (94694400, 12), // 1973-01-01
    (126230400, 13), // 1974-01-01
    (157766400, 14), // 1975-01-01
    (189302400, 15), // 1976-01-01
    (220924800, 16), // 1977-01-01
    (252460800, 17), // 1978-01-01
    (283996800, 18), // 1979-01-01
    (315532800, 19), // 1980-01-01
    (362793600, 20), // 1981-07-01
    (394329600, 21), // 1982-07-01
    (425865600, 22), // 1983-07-01
    (489024000, 23), // 1985-07-01
    (567993600, 24), // 1988-01-01
    (631152000, 25), // 1990-01-01
    (662688000, 26), // 1991-01-01
    (709948800, 27), // 1992-07-01
    (741484800, 28), // 1993-07-01
    (773020800, 29), // 1994-07-01
    (820454400, 30), // 1996-01-01
    (867715200, 31), // 1997-07-01
    (915148800, 32), // 1999-01-01
    (1136073600, 33), // 2006-01-01
    (1230768000, 34), // 2009-01-01
    (1341100800, 35), // 2012-07-01
    (1435708800, 36), // 2015-07-01
    (1483228800, 37), // 2017-01-01
];

#[cfg(feature = "std")]
static LEAP_OVERRIDE: std::sync::RwLock<Option<alloc::vec::Vec<(i64, i32)>>> = std::sync::RwLock::new(None);

/// Replace the leap table at runtime (a fresher table from IERS, shipped as data). Rows must be ascending by effective second.
#[cfg(feature = "std")]
pub fn set_leap_table(table: alloc::vec::Vec<(i64, i32)>) {
    if let Ok(mut t) = LEAP_OVERRIDE.write() {
        *t = Some(table);
    }
}

fn offset_in(table: &[(i64, i32)], unix_secs: i64) -> i32 {
    table.iter().rev().find(|(at, _)| unix_secs >= *at).map_or(10, |(_, o)| *o)
}

/// TAI − UTC at a Unix second, in whole seconds (10 before 1972: the integer era's first value).
pub fn tai_minus_utc(unix_secs: i64) -> i32 {
    #[cfg(feature = "std")]
    if let Ok(t) = LEAP_OVERRIDE.read() {
        if let Some(table) = t.as_ref() {
            return offset_in(table, unix_secs);
        }
    }
    offset_in(LEAP_TABLE, unix_secs)
}

/// A LEGACY stamp's LOCK equivalent: the same instant counted from the TAI epoch. `legacy + (TAI−UTC then − 8 s)` — 29 s for anything since 2017.
pub fn legacy_to_lock(legacy_osc: i64) -> i64 {
    let (secs, _) = to_unix_ns(legacy_osc);
    legacy_osc + (tai_minus_utc(secs) as i64 - 8) * OSCILLATIONS_PER_SECOND as i64
}

/// Where TAI comes from. `eagle_time_now` never reads one of these yet (the epoch flip is its own decision); LOCK's TrueClock will.
pub trait TaiSource {
    /// Now as (seconds since 1970-01-01 TAI, nanoseconds), or None when this source has nothing.
    fn tai_now(&self) -> Option<(i64, u32)>;
}

/// The system clock (NTP-disciplined UTC) lifted to TAI through the leap table.
#[cfg(feature = "std")]
pub struct NtpTai;

#[cfg(feature = "std")]
impl TaiSource for NtpTai {
    fn tai_now(&self) -> Option<(i64, u32)> {
        let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?;
        let secs = d.as_secs() as i64;
        Some((secs + tai_minus_utc(secs) as i64, d.subsec_nanos()))
    }
}

/// GPS time (seconds since 1970-01-01 on the GPS scale, which never took a leap second after 1980) → TAI = GPS + 19 s. The reader is the platform's (Android `GnssClock`).
pub struct GpsTai<F: Fn() -> Option<(i64, u32)>>(pub F);

impl<F: Fn() -> Option<(i64, u32)>> TaiSource for GpsTai<F> {
    fn tai_now(&self) -> Option<(i64, u32)> {
        (self.0)().map(|(s, n)| (s + 19, n))
    }
}

/// PTP already speaks TAI: a passthrough over the platform's reader.
pub struct PtpTai<F: Fn() -> Option<(i64, u32)>>(pub F);

impl<F: Fn() -> Option<(i64, u32)>> TaiSource for PtpTai<F> {
    fn tai_now(&self) -> Option<(i64, u32)> {
        (self.0)()
    }
}

/// Now on the LOCK (TAI-epoch) scale from any TAI source.
pub fn lock_eagle_now(source: &dyn TaiSource) -> Option<i64> {
    source.tai_now().map(|(s, n)| from_tai_ns(s, n))
}

/// Converts a UTC DateTime to Eagle Time (as oscillation count in i64)
///
/// Returns the number of hydrogen-1 hyperfine oscillations since the Apollo 11 landing. Negative values represent times before the landing.
pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
    EagleTime::from_oscillations(from_unix_ns(dt.timestamp(), dt.timestamp_subsec_nanos()))
}

/// Get current Eagle Time as oscillation count
///
/// Returns the number of hydrogen-1 hyperfine oscillations since the Apollo 11 landing at picosecond precision. Only available with the `std` feature, because chrono's `Utc::now()` requires `std::time::SystemTime`. `no_std` callers must read their own clock (QTIMER, nunc-time, etc) and call `EagleTime::from_oscillations`.
#[cfg(feature = "std")]
pub fn eagle_time_now() -> EagleTime {
    datetime_to_eagle_time(Utc::now())
}

/// Get current Eagle Time as i64 oscillations (704ps precision)
///
/// Returns the oscillation count since Apollo 11 landing. Preferred method for timestamps - preserves full precision. `std`-only because the underlying clock comes from `chrono::Utc::now()`.
#[cfg(feature = "std")]
pub fn eagle_time_oscillations() -> i64 {
    eagle_time_now().oscillations().unwrap_or(0)
}

/// Get current Eagle Time as nanosecond-precision f64 seconds (for compatibility)
///
/// Note: This loses the picosecond precision available in the oscillation count. Prefer `eagle_time_oscillations()` for integer timestamps.
#[cfg(feature = "std")]
#[deprecated(note = "Use eagle_time_oscillations() for integer timestamps")]
pub fn eagle_time_nanos() -> f64 {
    eagle_time_now().to_seconds_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LOCK §2.6: the TAI epoch is count zero, one second is exactly OPS, and a second boundary on either scale is a multiple of OPS on the other — both directions.
    #[test]
    fn lock_epoch_second_and_boundary_vectors() {
        assert_eq!(from_tai_ns(EAGLE_EPOCH_TAI_SECS, 0), 0);
        assert_eq!(to_tai_ns(0), (EAGLE_EPOCH_TAI_SECS, 0));
        assert_eq!(from_tai_ns(EAGLE_EPOCH_TAI_SECS + 1, 0), OSCILLATIONS_PER_SECOND as i64);
        let ops = OSCILLATIONS_PER_SECOND as i64;
        for s in [-3_000_000_000i64, -1, 0, 1, 1_790_000_000, 3_000_000_000] {
            assert_eq!(from_tai_ns(s, 0) % ops, 0, "a TAI second boundary is an Eagle second boundary");
            assert_ne!(from_tai_ns(s, 1) % ops, 0);
            assert_ne!(from_tai_ns(s, 999_999_999) % ops, 0);
        }
        for k in [-7i64, 0, 1, 1_000_000_000] {
            assert_eq!(to_tai_ns(k * ops).1, 0, "an Eagle second boundary is a TAI second boundary");
            assert_ne!(to_tai_ns(k * ops + 1).1, 0);
        }
    }

    /// LOCK §2.6: a million pseudo-random instants round-trip EXACTLY through the integer path, on both scales, negative instants included.
    #[test]
    fn lock_round_trip_is_exact_for_a_million_instants() {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..1_000_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let secs = (x % 12_000_000_000) as i64 - 6_000_000_000;
            let nanos = (x >> 33) as u32 % 1_000_000_000;
            assert_eq!(to_tai_ns(from_tai_ns(secs, nanos)), (secs, nanos));
            assert_eq!(to_unix_ns(from_unix_ns(secs, nanos)), (secs, nanos));
        }
    }

    /// The two scales, pinned: the legacy epoch label sits 0.427598 s before the TAI epoch in the 1969 rubber-second era, and any instant since 2017 counts exactly 29 s more on LOCK than on the legacy scale.
    #[test]
    fn legacy_and_lock_scales_differ_by_exactly_29_seconds_since_2017() {
        let ops = OSCILLATIONS_PER_SECOND as i64;
        for unix in [1_483_228_800i64, 1_790_000_000, 1_790_000_000 + 123] {
            let legacy = from_unix_ns(unix, 250_000_000);
            let lock = from_tai_ns(unix + tai_minus_utc(unix) as i64, 250_000_000);
            assert_eq!(lock - legacy, LOCK_MINUS_LEGACY_SECS * ops);
            assert_eq!(legacy_to_lock(legacy), lock);
        }
        assert_eq!(tai_minus_utc(1_483_228_799), 36);
        assert_eq!(tai_minus_utc(1_483_228_800), 37);
    }

    /// The legacy "now" path is integer now: a DateTime round-trips to the nanosecond, where the old f64 product drifted by hundreds of oscillations.
    #[test]
    fn datetime_path_is_integer_and_exact() {
        let dt = Utc.timestamp_opt(1_790_000_000, 123_456_789).single().unwrap();
        let et = datetime_to_eagle_time(dt);
        assert_eq!(et.oscillations(), Some(from_unix_ns(1_790_000_000, 123_456_789)));
        assert_eq!(et.to_datetime(), dt);
    }

    /// Sources lift to TAI the way the spec says: GPS + 19 s, PTP as-is.
    #[test]
    fn tai_sources_lift_correctly() {
        assert_eq!(GpsTai(|| Some((100, 5))).tai_now(), Some((119, 5)));
        assert_eq!(PtpTai(|| Some((100, 5))).tai_now(), Some((100, 5)));
        assert_eq!(lock_eagle_now(&PtpTai(|| Some((EAGLE_EPOCH_TAI_SECS, 0)))), Some(0));
    }

    #[test]
    fn test_oscillations_per_second_constant() {
        // Verify the hydrogen-1 21cm line frequency
        assert_eq!(OSCILLATIONS_PER_SECOND, 1_420_407_826);
    }

    #[test]
    fn test_eagle_epoch() {
        let epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let et = datetime_to_eagle_time(epoch);

        // At epoch, oscillation count should be zero
        assert_eq!(et.oscillations(), Some(0));

        let back = et.to_datetime();
        assert_eq!(epoch, back);
    }

    #[test]
    fn test_oscillation_counting() {
        // One second should be exactly OSCILLATIONS_PER_SECOND oscillations
        let one_second = EagleTime::from_oscillations(OSCILLATIONS_PER_SECOND as i64);
        assert_eq!(one_second.to_seconds_f64(), 1.0);

        // 100 seconds
        let hundred_seconds = EagleTime::from_oscillations(OSCILLATIONS_PER_SECOND as i64 * 100);
        assert_eq!(hundred_seconds.to_seconds_f64(), 100.0);
    }

    #[test]
    fn test_picosecond_precision() {
        // One oscillation ≈ 704.032 picoseconds
        let one_osc = EagleTime::from_oscillations(1);
        let ps = one_osc.picoseconds().unwrap();
        assert_eq!(ps, 704); // 704.032 truncated

        // Ten thousand oscillations
        let ten_k = EagleTime::from_oscillations(10_000);
        let ps = ten_k.picoseconds().unwrap();
        assert_eq!(ps, 7_040_320);
    }

    #[test]
    fn test_float_to_oscillation_conversion() {
        // Converting from seconds should preserve precision at oscillation level
        let et = EagleTime::from_seconds_f64(1.0);
        assert_eq!(et.oscillations(), Some(OSCILLATIONS_PER_SECOND as i64));

        // Round trip
        let seconds = et.to_seconds_f64();
        assert!((seconds - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_eagle_time_positive() {
        let future = Utc.with_ymd_and_hms(2025, 10, 25, 0, 0, 0).unwrap();
        let et = datetime_to_eagle_time(future);
        let back = et.to_datetime();

        // Should be exact at second precision
        assert_eq!((future - back).num_seconds().abs(), 0);
    }

    #[test]
    fn test_eagle_time_comparison() {
        let time1 = EagleTime::from_oscillations(1000);
        let time2 = EagleTime::from_oscillations(2000);
        let time3 = EagleTime::from_oscillations(1000);

        // Test ordering
        assert!(time1 < time2);
        assert!(time2 > time1);

        // Test equality at oscillation level
        assert_eq!(time1, time3);

        // Test with float (deprecated, should compare correctly despite type difference)
        #[allow(deprecated)]
        let time_f = EagleTime::new(EtType::f6(1000.0 / OSCILLATIONS_PER_SECOND as f64));
        assert_eq!(time1, time_f);
    }

    #[test]
    fn test_eagle_time_sorting() {
        let mut times = vec![
            EagleTime::from_oscillations(3000),
            EagleTime::from_oscillations(1000),
            EagleTime::from_oscillations(2000),
            EagleTime::from_oscillations(500),
        ];

        times.sort();

        assert_eq!(times[0].oscillations(), Some(500));
        assert_eq!(times[1].oscillations(), Some(1000));
        assert_eq!(times[2].oscillations(), Some(2000));
        assert_eq!(times[3].oscillations(), Some(3000));
    }

    #[test]
    fn test_negative_oscillations() {
        // Before epoch
        let before = EagleTime::from_oscillations(-1000);
        assert_eq!(before.oscillations(), Some(-1000));

        // Should order correctly vs positive
        let after = EagleTime::from_oscillations(500);
        assert!(before < after);
    }

    #[test]
    fn test_pre_epoch_datetime() {
        // 1960 is before the eagle epoch (1969)
        let pre_epoch = Utc.with_ymd_and_hms(1960, 1, 1, 0, 0, 0).unwrap();
        let et = datetime_to_eagle_time(pre_epoch);

        // Should be negative
        assert!(et.oscillations().unwrap() < 0);

        // Round trip
        let back = et.to_datetime();
        assert_eq!((pre_epoch - back).num_seconds().abs(), 0);
    }
}
