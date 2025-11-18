use chrono::{DateTime, Duration, TimeZone, Utc};

/// Eagle Time type - represents time values that can be stored in various numeric formats
///
/// Binary format: `[e][type][value...]`
/// - `[e]` = Eagle Time marker
/// - `[type]` = u/i/f (unsigned/signed/float)
/// - `[value]` = encoded numeric value (Eagle seconds since Apollo 11 lunar landing)
///
/// One Eagle second = 1,420,407,826 hydrogen-1 hyperfine transition periods
/// (21cm line frequency measured at the Milky Way-Andromeda barycentric reference frame)
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum EtType {
    u(usize), // Auto-sized unsigned
    i(isize), // Auto-sized signed
    f5(f32),  // 32-bit float
    f6(f64),  // 64-bit float
}

/// EagleTime represents a point in time in the Eagle Time standard.
/// 
/// Stores Eagle seconds since the Apollo 11 lunar landing (July 20, 1969, 20:17:40 UTC).
/// 
/// One Eagle second is defined as 1,420,407,826 complete oscillation periods of the
/// electromagnetic radiation emitted during the hydrogen-1 (protium) hyperfine transition
/// between F=0 and F=1 ground states, as measured at the barycentric reference frame 
/// of the Milky Way-Andromeda galaxy system.
/// 
/// Eagle seconds have the same duration as SI seconds (and UTC seconds). The epoch differs:
/// Eagle Time uses the Apollo 11 landing, while Unix time uses January 1, 1970.
/// 
/// This definition:
/// - Uses the most abundant element in the universe (hydrogen-1)
/// - Is measurable with any 21cm radio receiver
/// - Accounts for gravitational time dilation in the frequency measurement
/// - Provides universal verifiability without trusted authorities
#[derive(Debug, Clone)]
pub struct EagleTime {
    et_seconds: EtType,
}

impl EagleTime {
    /// Creates a new EagleTime instance from a VsfType.
    ///
    /// # Panics
    /// Panics if the VsfType is not a valid numeric variant.
    pub fn new_from_vsf(value: crate::types::VsfType) -> Self {
        use crate::types::VsfType;

        let et_seconds = match value {
            VsfType::f5(v) => EtType::f5(v),
            VsfType::f6(v) => EtType::f6(v),
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
    /// Since Eagle seconds and SI seconds have the same duration, this is a simple
    /// epoch offset calculation. No relativistic corrections are needed for the 
    /// conversion itself.
    pub fn to_datetime(&self) -> DateTime<Utc> {
        let eagle_epoch = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
        let duration: Duration = match self.et_seconds {
            EtType::f5(v) => Duration::from_std(std::time::Duration::from_secs_f64(v as f64))
                .unwrap_or_else(|_| panic!("Invalid duration")),
            EtType::f6(v) => Duration::from_std(std::time::Duration::from_secs_f64(v))
                .unwrap_or_else(|_| panic!("Invalid duration")),
            EtType::u(v) => Duration::seconds(v as i64),
            EtType::i(v) => Duration::seconds(v as i64),
        };
        eagle_epoch + duration
    }

    /// Get a reference to the underlying EtType
    pub fn et_type(&self) -> &EtType {
        &self.et_seconds
    }

    /// Converts the EtType to f64 for comparison purposes
    fn to_f64(&self) -> f64 {
        match self.et_seconds {
            EtType::f5(v) => v as f64,
            EtType::f6(v) => v,
            EtType::u(v) => v as f64,
            EtType::i(v) => v as f64,
        }
    }
}

impl PartialEq for EagleTime {
    fn eq(&self, other: &Self) -> bool {
        self.to_f64() == other.to_f64()
    }
}

impl Eq for EagleTime {}

impl PartialOrd for EagleTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_f64().partial_cmp(&other.to_f64())
    }
}

impl Ord for EagleTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // For Ord, we need total ordering. f64 has NaN which breaks this,
        // but for time values we shouldn't have NaN, so we can unwrap.
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Converts a UTC DateTime to Eagle Time
/// 
/// Eagle seconds have the same duration as SI/UTC seconds, so this is simply
/// calculating the time elapsed since the Eagle epoch (Apollo 11 landing).
pub fn datetime_to_eagle_time(dt: DateTime<Utc>) -> EagleTime {
    // Eagle epoch: Apollo 11 lunar landing - July 20, 1969 at 20:17:40 UTC
    // (Moment when "The Eagle has landed" was transmitted)
    let eagle = Utc.with_ymd_and_hms(1969, 7, 20, 20, 17, 40).unwrap();
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

    #[test]
    fn test_eagle_time_comparison() {
        let time1 = EagleTime::new(EtType::u(1000));
        let time2 = EagleTime::new(EtType::u(2000));
        let time3 = EagleTime::new(EtType::f6(1000.0));

        // Test ordering
        assert!(time1 < time2);
        assert!(time2 > time1);

        // Test equality across different types (u vs f6)
        assert_eq!(time1, time3);

        // Test with mixed types
        let time_f5 = EagleTime::new(EtType::f5(1500.0));
        assert!(time1 < time_f5);
        assert!(time_f5 < time2);
    }

    #[test]
    fn test_eagle_time_sorting() {
        let mut times = vec![
            EagleTime::new(EtType::u(3000)),
            EagleTime::new(EtType::f6(1000.0)),
            EagleTime::new(EtType::i(2000)),
            EagleTime::new(EtType::f5(500.0)),
        ];

        times.sort();

        assert_eq!(times[0].to_f64(), 500.0);
        assert_eq!(times[1].to_f64(), 1000.0);
        assert_eq!(times[2].to_f64(), 2000.0);
        assert_eq!(times[3].to_f64(), 3000.0);
    }
}