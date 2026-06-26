use chrono::{DateTime, Duration, TimeZone, Utc};

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
/// Eagle epoch: Apollo 11 lunar landing - July 20, 1969 at 20:17:40 UTC (The moment "The Eagle has landed" was transmitted)
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
        let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let seconds = self.to_seconds_f64();
        // Build Duration from i64 seconds + i64 nanoseconds so we don't need std::time::Duration::from_secs_f64. Splitting also avoids the f64×1e9 overflow that would happen for values >~9.2e9 s.
        let abs = seconds.abs();
        let int_secs = abs.trunc() as i64;
        let frac_nanos = (abs.fract() * 1_000_000_000.0).round() as i64;
        let duration = Duration::seconds(int_secs).checked_add(&Duration::nanoseconds(frac_nanos))?;
        if seconds >= 0.0 {
            Some(eagle_epoch + duration)
        } else {
            Some(eagle_epoch - duration)
        }
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
        self.oscillations_i128()
            .map(|osc| (osc * 704_032) / 1000)
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
            (a, b) if matches!(a, EtType::e5(_) | EtType::e6(_) | EtType::e7(_))
                   && matches!(b, EtType::e5(_) | EtType::e6(_) | EtType::e7(_)) => {
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

/// Converts a UTC DateTime to Eagle Time (as oscillation count in i64)
///
/// Returns the number of hydrogen-1 hyperfine oscillations since the Apollo 11 landing. Negative values represent times before the landing.
pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
    let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
    let duration = dt - eagle_epoch;

    // Calculate total seconds including subseconds
    let total_seconds =
        duration.num_seconds() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0;

    // Convert to oscillations
    let oscillations = (total_seconds * OSCILLATIONS_PER_SECOND as f64).round() as i64;

    EagleTime::from_oscillations(oscillations)
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
        let hundred_seconds =
            EagleTime::from_oscillations(OSCILLATIONS_PER_SECOND as i64 * 100);
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
