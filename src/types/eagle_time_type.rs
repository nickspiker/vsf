use chrono::{DateTime, Duration, TimeZone, Utc};

/// Eagle Time type - represents time values that can be stored in various numeric formats
///
/// Binary format: `[e][type][value...]`
/// - `[e]` = Eagle Time marker
/// - `[type]` = u/i/f (unsigned/signed/float)
/// - `[value]` = encoded numeric value (seconds since Eagle lunar landing)
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum EtType {
    u(usize), // Auto-sized unsigned
    u5(u32),  // 32-bit unsigned
    u6(u64),  // 64-bit unsigned
    u7(u128), // 128-bit unsigned
    i(isize), // Auto-sized signed
    i5(i32),  // 32-bit signed
    i6(i64),  // 64-bit signed
    i7(i128), // 128-bit signed
    f5(f32),  // 32-bit float
    f6(f64),  // 64-bit float
}

/// EagleTime represents a point in time in the Eagle Time standard.
/// It stores the number of seconds since the Eagle lunar landing
/// (July 20, 1969, 20:17:40 UTC).
#[derive(Debug, Clone)]
pub struct EagleTime {
    et_seconds: EtType,
}

impl EagleTime {
    /// Creates a new EagleTime instance from a VsfType.
    ///
    /// # Panics
    /// Panics if the VsfType is not a valid EtType variant.
    pub fn new_from_vsf(value: crate::types::VsfType) -> Self {
        use crate::types::VsfType;

        let et_seconds = match value {
            VsfType::f6(v) => EtType::f6(v),
            VsfType::u(v, false) => EtType::u(v),
            VsfType::u5(v) => EtType::u5(v),
            VsfType::u6(v) => EtType::u6(v),
            VsfType::u7(v) => EtType::u7(v),
            VsfType::i(v) => EtType::i(v),
            VsfType::i5(v) => EtType::i5(v),
            VsfType::i6(v) => EtType::i6(v),
            VsfType::i7(v) => EtType::i7(v),
            _ => panic!("EagleTime must be created with a valid numeric VsfType variant"),
        };
        EagleTime { et_seconds }
    }

    /// Creates a new EagleTime directly from an EtType
    pub fn new(et_seconds: EtType) -> Self {
        EagleTime { et_seconds }
    }

    /// Converts the current EagleTime to a VsfType.
    pub fn to_vsf_type(&self) -> crate::types::VsfType {
        use crate::types::VsfType;

        match self.et_seconds {
            EtType::f6(v) => VsfType::f6(v),
            EtType::u(v) => VsfType::u(v, false),
            EtType::u5(v) => VsfType::u5(v),
            EtType::u6(v) => VsfType::u6(v),
            EtType::u7(v) => VsfType::u7(v),
            EtType::i(v) => VsfType::i(v),
            EtType::i5(v) => VsfType::i5(v),
            EtType::i6(v) => VsfType::i6(v),
            EtType::i7(v) => VsfType::i7(v),
            _ => panic!("Unexpected EtType variant"),
        }
    }

    /// Converts the EagleTime to a UTC DateTime.
    pub fn to_datetime(&self) -> DateTime<Utc> {
        let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let duration: Duration = match self.et_seconds {
            EtType::f6(v) => Duration::from_std(std::time::Duration::from_secs_f64(v))
                .unwrap_or_else(|_| panic!("Invalid duration")),
            EtType::u(v) => Duration::seconds(v as i64),
            EtType::u5(v) => Duration::seconds(v as i64),
            EtType::u6(v) => Duration::seconds(v as i64),
            EtType::u7(v) => Duration::seconds(v as i64),
            EtType::i(v) => Duration::seconds(v as i64),
            EtType::i5(v) => Duration::seconds(v as i64),
            EtType::i6(v) => Duration::seconds(v),
            EtType::i7(v) => Duration::seconds(v as i64),
            _ => panic!("Unexpected EtType variant"),
        };
        eagle_epoch + duration
    }

    /// Get a reference to the underlying EtType
    pub fn et_type(&self) -> &EtType {
        &self.et_seconds
    }
}

/// Converts a UTC DateTime to Eagle Time
pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
    let eagle = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap(); // Lunar landing
    let seconds_since_landing = dt - eagle;
    let et_seconds = seconds_since_landing.num_seconds() as f64;
    EagleTime::new(EtType::f6(et_seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eagle_epoch() {
        let epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let et = datetime_to_eagle_time(epoch);
        let back = et.to_datetime();
        assert_eq!(epoch, back);
    }

    #[test]
    fn test_eagle_time_positive() {
        let future = Utc.with_ymd_and_hms(2025, 10, 25, 0, 0, 0).unwrap();
        let et = datetime_to_eagle_time(future);
        let back = et.to_datetime();
        // Allow small floating point error
        assert!((future - back).num_seconds().abs() < 2);
    }
}
