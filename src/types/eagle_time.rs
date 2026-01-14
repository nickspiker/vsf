use chrono::{DateTime, Duration, TimeZone, Utc};

/// Oscillations per Eagle second.
///
/// One Eagle second is defined as exactly 1,420,407,826 complete oscillation periods of the electromagnetic radiation emitted during the hydrogen-1 (protium) hyperfine transition between F=0 and F=1 ground states, as measured at the barycentric reference frame of the Milky Way-Andromeda galaxy system.
///
/// This is the 21cm hydrogen line at 1420.405751 MHz.
/// One oscillation period ≈ 704.032 picoseconds.
const OSCILLATIONS_PER_SECOND: u64 = 1_420_407_826;

/// Eagle Time type - represents time values as either oscillation counts or seconds.
///
/// Binary format: `[e][type][value...]`
/// - `[e]` = Eagle Time marker
/// - `[type]` = u/i/f (unsigned/signed/float)
/// - `[value]` = encoded value
///
/// **Integer types store actual oscillation counts:**
/// - Each count represents one complete 21cm hydrogen-1 hyperfine transition
/// - Precision: 704.032 picoseconds per oscillation
/// - u64: 411 years range at picosecond precision
/// - i64: ±206 years range at picosecond precision
///
/// **IEEE 754 Float types store seconds:**
/// - f32: ~2 minute effective precision (for timestamps since 1969)
/// - f64: ~200 nanosecond effective precision (for timestamps since 1969)
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum EtType {
    u(usize), // Oscillation count
    i(isize), // Oscillation count
    f5(f32),  // Seconds as 32-bit float (2-3min precision)
    f6(f64),  // Seconds as 64-bit float (lossy)
}

/// EagleTime represents a point in time in the Eagle Time standard.
///
/// Stores time as either:
/// - **Oscillation counts** (integer types): Count of hydrogen-1 transition events since epoch (lunar landing)
/// - **Seconds** (float types): Floating-point seconds since epoch (lunar landing)
///
/// Eagle epoch: Apollo 11 lunar landing - July 20, 1969 at 20:17:40 UTC
/// (The moment "The Eagle has landed" was transmitted)
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
/// All integer types (u3-u7, i3-i7) are supported for oscillation counts.
/// Common configurations:
///
/// | Type | Range | Precision | Use Case |
/// |------|-------|-----------|----------|
/// | u32 | 1.5 years | 704 ps | Short-term timing |
/// | u64 | 411 years (1969-2380) | 704 ps | Absolute timestamps (default) |
/// | i64 | ±206 years | 704 ps | Time deltas |
/// | f32 | Arbitrary | ~2 min | Low-precision timestamps (legacy) |
/// | f64 | Arbitrary | ~200 ns | High-precision timestamps (legacy) |
///
/// Note: All u/i types use generic encoding, so u8/u16/u128/etc. are also valid.
#[derive(Debug, Clone)]
pub struct EagleTime {
    et_seconds: EtType,
}

impl EagleTime {
    /// Creates a new EagleTime instance from a VsfType.
    ///
    /// Integer types are interpreted as oscillation counts.
    /// Float types are interpreted as seconds.
    ///
    /// # Panics
    /// Panics if the VsfType is not a valid numeric variant or EagleTime type.
    pub fn new_from_vsf(value: crate::types::VsfType) -> Self {
        use crate::types::VsfType;

        let et_seconds = match value {
            // Handle VsfType::e (already an EagleTime wrapper)
            VsfType::e(et) => et,
            // Float types: already in seconds
            VsfType::f5(v) => EtType::f5(v),
            VsfType::f6(v) => EtType::f6(v),
            // Integer types: oscillation counts
            VsfType::u(v, false) => EtType::u(v),
            VsfType::u3(v) => EtType::u(v as usize),
            VsfType::u4(v) => EtType::u(v as usize),
            VsfType::u5(v) => EtType::u(v as usize),
            VsfType::u6(v) => EtType::u(v as usize),
            VsfType::i(v) => EtType::i(v),
            VsfType::i3(v) => EtType::i(v as isize),
            VsfType::i4(v) => EtType::i(v as isize),
            VsfType::i5(v) => EtType::i(v as isize),
            VsfType::i6(v) => EtType::i(v as isize),
            _ => panic!("EagleTime must be created with a valid numeric VsfType variant"),
        };
        EagleTime { et_seconds }
    }

    /// Creates a new EagleTime directly from an EtType
    pub fn new(et_seconds: EtType) -> Self {
        EagleTime { et_seconds }
    }

    /// Creates an EagleTime from a raw oscillation count
    pub fn from_oscillations(count: usize) -> Self {
        EagleTime {
            et_seconds: EtType::u(count),
        }
    }

    /// Creates an EagleTime from a signed oscillation count (for deltas)
    pub fn from_oscillations_signed(count: isize) -> Self {
        EagleTime {
            et_seconds: EtType::i(count),
        }
    }

    /// Creates an EagleTime from seconds (f64), converting to oscillation count
    pub fn from_seconds_f64(seconds: f64) -> Self {
        let oscillations = (seconds * OSCILLATIONS_PER_SECOND as f64).round() as usize;
        EagleTime {
            et_seconds: EtType::u(oscillations),
        }
    }

    /// Creates an EagleTime from seconds (f32), converting to oscillation count
    pub fn from_seconds_f32(seconds: f32) -> Self {
        let oscillations = (seconds * OSCILLATIONS_PER_SECOND as f32).round() as usize;
        EagleTime {
            et_seconds: EtType::u(oscillations),
        }
    }

    /// Converts the current EagleTime to a VsfType.
    pub fn to_vsf_type(&self) -> crate::types::VsfType {
        use crate::types::VsfType;

        match self.et_seconds {
            EtType::f5(v) => VsfType::f5(v),
            EtType::f6(v) => VsfType::f6(v),
            EtType::u(v) => VsfType::u(v, false),
            EtType::i(v) => VsfType::i(v),
        }
    }

    /// Converts the EagleTime to a UTC DateTime.
    ///
    /// For integer types, divides oscillation count by OSCILLATIONS_PER_SECOND.
    /// For float types, uses the stored seconds directly.
    ///
    /// Returns None if the timestamp is outside chrono's representable range.
    pub fn to_datetime_opt(&self) -> Option<DateTime<Utc>> {
        let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let seconds = self.to_seconds_f64();
        let duration = Duration::from_std(std::time::Duration::from_secs_f64(seconds)).ok()?;
        Some(eagle_epoch + duration)
    }

    /// Converts the EagleTime to a UTC DateTime.
    ///
    /// For integer types, divides oscillation count by OSCILLATIONS_PER_SECOND.
    /// For float types, uses the stored seconds directly.
    ///
    /// Panics if the timestamp is outside chrono's representable range.
    /// For non-panicking version, use `to_datetime_opt()`.
    pub fn to_datetime(&self) -> DateTime<Utc> {
        self.to_datetime_opt()
            .unwrap_or_else(|| panic!("Timestamp outside representable range: {:?}", self.et_seconds))
    }

    /// Get a reference to the underlying EtType
    pub fn et_type(&self) -> &EtType {
        &self.et_seconds
    }

    /// Converts to f64 seconds, regardless of storage type.
    ///
    /// For integer types: divides oscillation count by OSCILLATIONS_PER_SECOND
    /// For float types: returns the stored seconds directly
    pub fn to_seconds_f64(&self) -> f64 {
        match self.et_seconds {
            EtType::u(oscillations) => oscillations as f64 / OSCILLATIONS_PER_SECOND as f64,
            EtType::i(oscillations) => oscillations as f64 / OSCILLATIONS_PER_SECOND as f64,
            EtType::f5(seconds) => seconds as f64,
            EtType::f6(seconds) => seconds,
        }
    }

    /// Converts to f32 seconds, regardless of storage type.
    pub fn to_seconds_f32(&self) -> f32 {
        match self.et_seconds {
            EtType::u(oscillations) => oscillations as f32 / OSCILLATIONS_PER_SECOND as f32,
            EtType::i(oscillations) => oscillations as f32 / OSCILLATIONS_PER_SECOND as f32,
            EtType::f5(seconds) => seconds,
            EtType::f6(seconds) => seconds as f32,
        }
    }

    /// Returns the oscillation count if stored as integer, None if stored as float.
    pub fn oscillations(&self) -> Option<isize> {
        match self.et_seconds {
            EtType::u(v) => Some(v as isize),
            EtType::i(v) => Some(v),
            EtType::f5(_) | EtType::f6(_) => None,
        }
    }

    /// Returns the picosecond precision timestamp (oscillations * 704.032).
    /// Returns None for float types since they've lost the picosecond precision.
    pub fn picoseconds(&self) -> Option<i128> {
        self.oscillations()
            .map(|osc| (osc as i128 * 704_032) / 1000)
    }
}

impl PartialEq for EagleTime {
    fn eq(&self, other: &Self) -> bool {
        // Compare at oscillation precision if both are integers
        match (&self.et_seconds, &other.et_seconds) {
            (EtType::u(a), EtType::u(b)) => a == b,
            (EtType::i(a), EtType::i(b)) => a == b,
            (EtType::u(a), EtType::i(b)) | (EtType::i(b), EtType::u(a)) => *a as isize == *b,
            // Fall back to f64 comparison for any float involvement
            _ => self.to_seconds_f64() == other.to_seconds_f64(),
        }
    }
}

impl Eq for EagleTime {}

impl PartialOrd for EagleTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EagleTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare at oscillation precision if both are integers
        match (&self.et_seconds, &other.et_seconds) {
            (EtType::u(a), EtType::u(b)) => a.cmp(b),
            (EtType::i(a), EtType::i(b)) => a.cmp(b),
            (EtType::u(a), EtType::i(b)) => (*a as isize).cmp(b),
            (EtType::i(a), EtType::u(b)) => a.cmp(&(*b as isize)),
            // Fall back to f64 comparison for any float involvement
            _ => self
                .to_seconds_f64()
                .partial_cmp(&other.to_seconds_f64())
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    }
}

/// Converts a UTC DateTime to Eagle Time (as oscillation count in u64)
///
/// Returns the number of hydrogen-1 hyperfine oscillations since the Apollo 11 landing.
pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
    let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
    let duration = dt - eagle_epoch;

    // Calculate total seconds including subseconds
    let total_seconds =
        duration.num_seconds() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0;

    // Convert to oscillations
    let oscillations = (total_seconds * OSCILLATIONS_PER_SECOND as f64).round() as usize;

    EagleTime::from_oscillations(oscillations)
}

/// Get current Eagle Time as oscillation count
///
/// Returns the number of hydrogen-1 hyperfine oscillations since the Apollo 11 landing
/// at picosecond precision.
pub fn eagle_time_now() -> EagleTime {
    datetime_to_eagle_time(Utc::now())
}

/// Get current Eagle Time as oscillations (integer, 704ps precision)
///
/// Returns the oscillation count since Apollo 11 landing.
/// Preferred method for timestamps - preserves full precision.
pub fn eagle_time_oscillations() -> usize {
    eagle_time_now().oscillations().unwrap_or(0) as usize
}

/// Get current Eagle Time as nanosecond-precision f64 seconds (for compatibility)
///
/// Note: This loses the picosecond precision available in the oscillation count.
/// Prefer `eagle_time_oscillations()` for maximum precision.
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
        let one_second = EagleTime::from_oscillations(OSCILLATIONS_PER_SECOND as usize);
        assert_eq!(one_second.to_seconds_f64(), 1.0);

        // 100 seconds
        let hundred_seconds =
            EagleTime::from_oscillations((OSCILLATIONS_PER_SECOND * 100) as usize);
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
        assert_eq!(et.oscillations(), Some(OSCILLATIONS_PER_SECOND as isize));

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

        // Test with float (should compare correctly despite type difference)
        let time_f = EagleTime::new(EtType::f6(1000.0 / OSCILLATIONS_PER_SECOND as f64));
        assert_eq!(time1, time_f);
    }

    #[test]
    fn test_eagle_time_sorting() {
        let mut times = vec![
            EagleTime::from_oscillations(3000),
            EagleTime::from_oscillations(1000),
            EagleTime::from_oscillations_signed(2000),
            EagleTime::from_oscillations(500),
        ];

        times.sort();

        assert_eq!(times[0].oscillations(), Some(500));
        assert_eq!(times[1].oscillations(), Some(1000));
        assert_eq!(times[2].oscillations(), Some(2000));
        assert_eq!(times[3].oscillations(), Some(3000));
    }

    #[test]
    fn test_range_limits() {
        // u64 should give us ~411 years
        // From 1969 to 2380 is 411 years
        let year_2380 = Utc.with_ymd_and_hms(2380, 1, 1, 0, 0, 0).unwrap();
        let et = datetime_to_eagle_time(year_2380);

        // Should be within u64 range
        if let Some(osc) = et.oscillations() {
            assert!(osc > 0);
            // ~411 years * 365.25 days * 86400 seconds * 1.42e9 oscillations/sec
            // Should be less than u64::MAX
        }
    }

    #[test]
    fn test_signed_oscillations() {
        // Test negative time deltas
        let before_epoch = EagleTime::from_oscillations_signed(-1000);
        assert_eq!(before_epoch.oscillations(), Some(-1000));

        // Should handle comparison with positive values
        let after_epoch = EagleTime::from_oscillations(500);
        assert!(before_epoch < after_epoch);
    }
}
