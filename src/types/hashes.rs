//! One Rust type per `h` marker that is always 32 bytes, so an identity is never a bare `[u8; 32]`.
//!
//! - [`Hp`] (`hp`): a version's content identity, the BLAKE3 of these exact bytes (an icon, one build of a kist).
//! - [`Hb`] (`hb`): a file's rolling BLAKE3, its current state.
//! - [`Hi`] (`hi`): an idea, a kind or a meaning.
//! - [`Ht`] (`ht`): a thing, the same while its content changes (a kist across updates, a contact across avatars).
//! - [`HP`] (`hP`), [`HI`] (`hI`), [`HV`] (`hV`): Photon's handle proof, identity seed and vault seed.
//!
//! Each converts only to and from its own [`VsfType`] variant: reading an `hb` as an [`Hp`] is refused, and so is any width but 32 bytes.
//! The markers whose width varies (`hs`, `hm`, `hg`, `hc`, `hk`, and `hR`'s random material) have no type here.
//!
//! A reference to a thing is (kind as [`Hi`], the thing as [`Ht`]), optionally with its current content as [`Hp`] (owner, 2026-11-20).

use super::vsf_type::VsfType;
use crate::prelude::*;
use crate::schema::{FromVsfType, IntoVsfType, ValidationError, ValidationResult};

/// Why a [`VsfType`] is not the 32-byte hash asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrongHash {
    /// The value is some other type or another `h` marker.
    Marker { want: &'static str },
    /// The marker is right but the value is not 32 bytes.
    Length { want: &'static str, len: usize },
}

impl core::fmt::Display for WrongHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WrongHash::Marker { want } => write!(f, "expected {want}, found another type"),
            WrongHash::Length { want, len } => write!(f, "expected {want} of 32 bytes, found {len}"),
        }
    }
}

macro_rules! hash32 {
    ($(#[$doc:meta])* $name:ident, $variant:ident, $marker:literal) => {
        $(#[$doc])*
        #[allow(clippy::upper_case_acronyms)]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// The wire marker this type is read from and written as.
            pub const MARKER: &'static str = $marker;

            /// Names 32 bytes as this kind of hash; the caller says what they are.
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                $name(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            pub const fn to_bytes(self) -> [u8; 32] {
                self.0
            }

            /// This value as its own marker.
            pub fn to_vsf(self) -> VsfType {
                VsfType::$variant(self.0.to_vec())
            }

            /// Reads only its own marker, and only 32 bytes of it.
            pub fn from_vsf(value: &VsfType) -> Result<Self, WrongHash> {
                match value {
                    VsfType::$variant(b) => match <[u8; 32]>::try_from(b.as_slice()) {
                        Ok(bytes) => Ok($name(bytes)),
                        Err(_) => Err(WrongHash::Length { want: $marker, len: b.len() }),
                    },
                    _ => Err(WrongHash::Marker { want: $marker }),
                }
            }
        }

        impl From<$name> for VsfType {
            fn from(h: $name) -> VsfType {
                h.to_vsf()
            }
        }

        impl TryFrom<&VsfType> for $name {
            type Error = WrongHash;
            fn try_from(value: &VsfType) -> Result<Self, WrongHash> {
                $name::from_vsf(value)
            }
        }

        impl IntoVsfType for $name {
            fn into_vsf_type(self) -> VsfType {
                self.to_vsf()
            }
        }

        impl FromVsfType for $name {
            fn from_vsf_type(vsf: &VsfType) -> ValidationResult<Self> {
                $name::from_vsf(vsf).map_err(|e| ValidationError::Custom(format!("{e}")))
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str($marker)?;
                f.write_str(":")?;
                for b in self.0 {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
        }
    };
}

hash32!(
    /// A version's content identity (`hp`): the BLAKE3 of these exact bytes. For caches and dedup.
    Hp, hp, "hp"
);
hash32!(
    /// A file's rolling BLAKE3 (`hb`): its current state.
    Hb, hb, "hb"
);
hash32!(
    /// An idea (`hi`): a meaning or a kind, named by 32 bytes.
    Hi, hi, "hi"
);
hash32!(
    /// A thing (`ht`): what stays the same while its content changes, derived or minted once per thing.
    Ht, ht, "ht"
);
hash32!(
    /// Photon's handle proof (`hP`): the memory-hard proof of work over a handle.
    HP, hP, "hP"
);
hash32!(
    /// Photon's identity seed (`hI`): the BLAKE3-derived network and contacts root.
    HI, hI, "hI"
);
hash32!(
    /// Photon's vault seed (`hV`): the BLAKE3-derived local storage root.
    HV, hV, "hV"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoding::parse;

    /// Each type survives a flatten and parse as its own marker.
    #[test]
    fn round_trip_through_the_wire() {
        let t = Ht::from_bytes([0x5A; 32]);
        let flat = VsfType::from(t).flatten();
        assert_eq!(&flat[..2], b"ht");
        let mut ptr = 0;
        let back = Ht::from_vsf(&parse(&flat, &mut ptr).unwrap()).unwrap();
        assert_eq!(back, t);
        assert_eq!(ptr, flat.len());

        let p = Hp::from_bytes([0xA5; 32]);
        let mut ptr = 0;
        assert_eq!(Hp::try_from(&parse(&p.to_vsf().flatten(), &mut ptr).unwrap()), Ok(p));
    }

    /// A value under another marker is refused, even at 32 bytes; so is a wrong width under the right one.
    #[test]
    fn refuses_other_markers_and_widths() {
        let bytes = vec![7u8; 32];
        assert_eq!(Hp::from_vsf(&VsfType::hb(bytes.clone())), Err(WrongHash::Marker { want: "hp" }));
        assert_eq!(Ht::from_vsf(&VsfType::hp(bytes.clone())), Err(WrongHash::Marker { want: "ht" }));
        assert_eq!(Hi::from_vsf(&VsfType::ht(bytes.clone())), Err(WrongHash::Marker { want: "hi" }));
        assert_eq!(HI::from_vsf(&VsfType::hV(bytes.clone())), Err(WrongHash::Marker { want: "hI" }));
        assert_eq!(Ht::from_vsf(&VsfType::ke(bytes)), Err(WrongHash::Marker { want: "ht" }));
        assert_eq!(Ht::from_vsf(&VsfType::ht(vec![1; 16])), Err(WrongHash::Length { want: "ht", len: 16 }));
        assert!(Hp::from_vsf_type(&VsfType::hb(vec![0; 32])).is_err());
        assert!(Ht::from_vsf_type(&VsfType::ht(vec![0; 32])).is_ok());
    }

    /// The types are their bytes in memory, ordered as their bytes, and shown with their marker.
    #[test]
    fn layout_order_and_debug() {
        assert_eq!(core::mem::size_of::<Ht>(), 32);
        assert!(Ht::from_bytes([1; 32]) < Ht::from_bytes([2; 32]));
        let mut b = [0u8; 32];
        b[0] = 0xAB;
        let shown = format!("{:?}", Ht::from_bytes(b));
        assert!(shown.starts_with("ht:ab00"), "{shown}");
        assert_eq!(shown.len(), 3 + 64);
    }
}
