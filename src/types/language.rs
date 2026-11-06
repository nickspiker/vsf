//! A human language (`wl`, v11): an ISO 639-3 language, optionally narrowed by script (ISO 15924) and region (ISO 3166-1).
//!
//! Wire: `w l` then three big-endian u16, 8 bytes in all:
//! - language: the three ISO 639-3 letters packed base 26 (a = 0 … z = 25, the first letter most significant), 0..=17575; `spa` is 12558.
//! - script: the ISO 15924 numeric code, 0 when absent (`Latn` is 215).
//! - region: the ISO 3166-1 numeric code, 0 when absent (Spain is 724).
//!
//! A reader refuses a language past 17575 and a script or region past 999, the largest three-digit code.

use crate::decoding::traits::DecodeError;
use crate::prelude::*;

/// The largest packed language: `zzz`.
const LANGUAGE_MAX: u16 = 26 * 26 * 26 - 1;

/// The largest ISO 15924 or ISO 3166-1 numeric code (three digits).
const CODE_MAX: u16 = 999;

/// A human language: ISO 639-3, with optional script and region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    packed: u16,
    script: u16,
    region: u16,
}

impl Language {
    /// The language for three lowercase ISO 639-3 letters, with no script or region. Panics on anything but `a`–`z`: the code is the caller's literal.
    pub const fn new(code: [u8; 3]) -> Language {
        let mut packed = 0u16;
        let mut idx = 0;
        while idx < 3 {
            let c = code[idx];
            assert!(c.is_ascii_lowercase(), "ISO 639-3 codes are three lowercase letters");
            packed = packed * 26 + (c - b'a') as u16;
            idx += 1;
        }
        Language { packed, script: 0, region: 0 }
    }

    /// This language in an ISO 15924 script (numeric, 1..=999). Panics outside that range.
    pub const fn with_script(self, script: u16) -> Language {
        assert!(script >= 1 && script <= CODE_MAX, "ISO 15924 numeric codes are 1..=999");
        Language { script, ..self }
    }

    /// This language as used in an ISO 3166-1 region (numeric, 1..=999). Panics outside that range.
    pub const fn with_region(self, region: u16) -> Language {
        assert!(region >= 1 && region <= CODE_MAX, "ISO 3166-1 numeric codes are 1..=999");
        Language { region, ..self }
    }

    /// The three ISO 639-3 letters.
    pub fn code(&self) -> [u8; 3] {
        let p = self.packed;
        [b'a' + (p / 676) as u8, b'a' + (p / 26 % 26) as u8, b'a' + (p % 26) as u8]
    }

    /// The letters packed base 26, as the wire carries them.
    pub fn packed(&self) -> u16 {
        self.packed
    }

    /// The ISO 15924 numeric script, if one is set.
    pub fn script(&self) -> Option<u16> {
        (self.script != 0).then_some(self.script)
    }

    /// The ISO 3166-1 numeric region, if one is set.
    pub fn region(&self) -> Option<u16> {
        (self.region != 0).then_some(self.region)
    }

    /// The six bytes after the `wl` marker.
    pub fn to_wire(&self) -> [u8; 6] {
        let [a, b] = self.packed.to_be_bytes();
        let [c, d] = self.script.to_be_bytes();
        let [e, f] = self.region.to_be_bytes();
        [a, b, c, d, e, f]
    }

    /// The language from the six bytes after the `wl` marker; out-of-range fields are refused.
    pub fn from_wire(bytes: [u8; 6]) -> Result<Language, DecodeError> {
        let packed = u16::from_be_bytes([bytes[0], bytes[1]]);
        let script = u16::from_be_bytes([bytes[2], bytes[3]]);
        let region = u16::from_be_bytes([bytes[4], bytes[5]]);
        if packed > LANGUAGE_MAX {
            return Err(DecodeError::InvalidDataMsg(format!("wl language {packed} is past 17575 ('zzz')")));
        }
        if script > CODE_MAX {
            return Err(DecodeError::InvalidDataMsg(format!("wl script {script} is past 999")));
        }
        if region > CODE_MAX {
            return Err(DecodeError::InvalidDataMsg(format!("wl region {region} is past 999")));
        }
        Ok(Language { packed, script, region })
    }
}

impl core::fmt::Display for Language {
    /// `spa`, `spa script 215`, `spa script 215 region 724`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let c = self.code();
        write!(f, "{}{}{}", c[0] as char, c[1] as char, c[2] as char)?;
        if let Some(s) = self.script() {
            write!(f, " script {s}")?;
        }
        if let Some(r) = self.region() {
            write!(f, " region {r}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_base_26_first_letter_most_significant() {
        assert_eq!(Language::new(*b"aaa").packed(), 0);
        assert_eq!(Language::new(*b"spa").packed(), 12558);
        assert_eq!(Language::new(*b"zzz").packed(), 17575);
    }

    #[test]
    fn letters_survive_packing() {
        for code in [*b"eng", *b"spa", *b"fra", *b"por", *b"ind", *b"deu", *b"tur"] {
            assert_eq!(Language::new(code).code(), code);
        }
    }

    #[test]
    fn script_and_region_are_absent_until_set() {
        let spa = Language::new(*b"spa");
        assert_eq!((spa.script(), spa.region()), (None, None));
        let es = spa.with_script(215).with_region(724);
        assert_eq!((es.script(), es.region()), (Some(215), Some(724)));
        assert_eq!(es.to_string(), "spa script 215 region 724");
    }

    #[test]
    fn wire_refuses_out_of_range() {
        assert!(Language::from_wire([0x44, 0xA8, 0, 0, 0, 0]).is_err()); // 17576
        assert!(Language::from_wire([0, 0, 0x03, 0xE8, 0, 0]).is_err()); // script 1000
        assert!(Language::from_wire([0, 0, 0, 0, 0x03, 0xE8]).is_err()); // region 1000
        assert_eq!(Language::from_wire([0x44, 0xA7, 0x03, 0xE7, 0x03, 0xE7]).unwrap().code(), *b"zzz");
    }
}
