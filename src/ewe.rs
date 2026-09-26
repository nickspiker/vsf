//! EWE integer reader and writer: the one place a bare size-marked integer (`3`..`7` marker + big-endian bytes) crosses the wire.
//!
//! The Rust type is the caller's choice and the byte width is the format's, and the two never mix: [`write_uint`]/[`write_int`] always emit the narrowest marker that holds the value (`5u32` and `5u128` both write `3 05`), and [`read_uint`]/[`read_int`] accept any marker but reject a value a narrower marker would have held, then narrow into the requested type or return [`DecodeError::ValueOutOfRange`].
//!
//! Canonical form is what makes byte-exact matching and signatures meaningful: one value has exactly one valid byte form, so two distinct byte strings can never decode to the same number.
//! Every marker is exactly twice the width of the one below it, so "a narrower marker would hold this" is just "the upper half of the bytes is empty" (unsigned) or "the upper half is only the sign extension of the lower half" (signed).
//!
//! Markers above `7` are valid EWE (the spec runs to `Z`) but exceed every native Rust integer, so these readers refuse them; a big-integer reader belongs beside these if something ever needs one.

use crate::decoding::traits::DecodeError;
use crate::prelude::*;

/// A native unsigned integer that EWE can read into and write from.
pub trait EweUint: Copy {
    /// Widen to `u128` (always exact).
    fn to_u128(self) -> u128;
    /// Narrow from `u128`, `None` if the value doesn't fit.
    fn from_u128(v: u128) -> Option<Self>;
}

/// A native signed integer that EWE can read into and write from.
pub trait EweInt: Copy {
    /// Widen to `i128` (always exact).
    fn to_i128(self) -> i128;
    /// Narrow from `i128`, `None` if the value doesn't fit.
    fn from_i128(v: i128) -> Option<Self>;
}

macro_rules! ewe_uint {
    ($($t:ty),*) => {$(
        impl EweUint for $t {
            fn to_u128(self) -> u128 { self as u128 }
            fn from_u128(v: u128) -> Option<Self> { <$t>::try_from(v).ok() }
        }
    )*};
}
ewe_uint!(u8, u16, u32, u64, u128, usize);

macro_rules! ewe_int {
    ($($t:ty),*) => {$(
        impl EweInt for $t {
            fn to_i128(self) -> i128 { self as i128 }
            fn from_i128(v: i128) -> Option<Self> { <$t>::try_from(v).ok() }
        }
    )*};
}
ewe_int!(i8, i16, i32, i64, i128, isize);

/// Byte width of marker `3`..`7` (1, 2, 4, 8, 16).
fn width_of(marker: u8) -> usize {
    1 << (marker - b'3')
}

/// Write `v` as marker + big-endian bytes at the narrowest width that holds it.
pub fn write_uint<T: EweUint>(v: T, out: &mut Vec<u8>) {
    let v = v.to_u128();
    let mut marker = b'3';
    while width_of(marker) < 16 && v >> (width_of(marker) * 8) != 0 {
        marker += 1;
    }
    let w = width_of(marker);
    out.push(marker);
    out.extend_from_slice(&v.to_be_bytes()[16 - w..]);
}

/// Write `v` as marker + big-endian two's-complement bytes at the narrowest width that holds it (128 is `4 00 80`, never `3 80`, which reads back as −128).
pub fn write_int<T: EweInt>(v: T, out: &mut Vec<u8>) {
    let v = v.to_i128();
    let mut marker = b'3';
    while width_of(marker) < 16 && !fits_signed(v, width_of(marker) * 8) {
        marker += 1;
    }
    let w = width_of(marker);
    out.push(marker);
    out.extend_from_slice(&v.to_be_bytes()[16 - w..]);
}

/// Whether `v` fits a signed integer of `bits` bits (`bits` < 128).
fn fits_signed(v: i128, bits: usize) -> bool {
    let half = 1i128 << (bits - 1);
    v >= -half && v < half
}

/// Read the marker and its bytes, returning the marker and the big-endian payload.
fn read_raw<'a>(data: &'a [u8], pointer: &mut usize) -> Result<(u8, &'a [u8]), DecodeError> {
    let Some(&marker) = data.get(*pointer) else {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for size marker".into()));
    };
    if !(b'3'..=b'7').contains(&marker) {
        return Err(if marker.is_ascii_alphanumeric() && marker > b'7' {
            DecodeError::InvalidDataMsg(format!("EWE marker '{}' exceeds 128 bits; no native reader", marker as char))
        } else {
            DecodeError::InvalidSizeMarker(marker)
        });
    }
    let w = width_of(marker);
    let start = *pointer + 1;
    let Some(bytes) = data.get(start..start + w) else {
        return Err(DecodeError::UnexpectedEofMsg(format!("Not enough data for {}-byte integer", w)));
    };
    *pointer = start + w;
    Ok((marker, bytes))
}

/// Read an unsigned EWE integer into `T`: rejects non-minimal widths and values that don't fit `T`.
pub fn read_uint<T: EweUint>(data: &[u8], pointer: &mut usize) -> Result<T, DecodeError> {
    let before = *pointer;
    let (marker, bytes) = read_raw(data, pointer)?;
    let mut buf = [0u8; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    let v = u128::from_be_bytes(buf);
    if marker > b'3' && v >> (bytes.len() * 4) == 0 {
        *pointer = before;
        return Err(DecodeError::InvalidDataMsg(format!("Non-canonical EWE: {} fits a narrower marker than '{}'", v, marker as char)));
    }
    T::from_u128(v).ok_or_else(|| {
        *pointer = before;
        DecodeError::ValueOutOfRange
    })
}

/// Read a signed EWE integer into `T`: rejects non-minimal widths and values that don't fit `T`.
pub fn read_int<T: EweInt>(data: &[u8], pointer: &mut usize) -> Result<T, DecodeError> {
    let before = *pointer;
    let (marker, bytes) = read_raw(data, pointer)?;
    let fill = if bytes[0] & 0x80 != 0 { 0xFF } else { 0 };
    let mut buf = [fill; 16];
    buf[16 - bytes.len()..].copy_from_slice(bytes);
    let v = i128::from_be_bytes(buf);
    if marker > b'3' && fits_signed(v, bytes.len() * 4) {
        *pointer = before;
        return Err(DecodeError::InvalidDataMsg(format!("Non-canonical EWE: {} fits a narrower marker than '{}'", v, marker as char)));
    }
    T::from_i128(v).ok_or_else(|| {
        *pointer = before;
        DecodeError::ValueOutOfRange
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wu<T: EweUint>(v: T) -> Vec<u8> {
        let mut out = Vec::new();
        write_uint(v, &mut out);
        out
    }

    fn wi<T: EweInt>(v: T) -> Vec<u8> {
        let mut out = Vec::new();
        write_int(v, &mut out);
        out
    }

    fn ru<T: EweUint>(b: &[u8]) -> Result<T, DecodeError> {
        let mut p = 0;
        let r = read_uint(b, &mut p);
        if r.is_ok() {
            assert_eq!(p, b.len(), "reader must consume exactly the encoding");
        }
        r
    }

    fn ri<T: EweInt>(b: &[u8]) -> Result<T, DecodeError> {
        let mut p = 0;
        let r = read_int(b, &mut p);
        if r.is_ok() {
            assert_eq!(p, b.len(), "reader must consume exactly the encoding");
        }
        r
    }

    #[test]
    fn writer_ignores_rust_width() {
        assert_eq!(wu(5u8), vec![b'3', 5]);
        assert_eq!(wu(5u32), vec![b'3', 5]);
        assert_eq!(wu(5u128), vec![b'3', 5]);
        assert_eq!(wu(5usize), vec![b'3', 5]);
        assert_eq!(wi(5i64), vec![b'3', 5]);
        assert_eq!(wi(-5i128), vec![b'3', 0xFB]);
    }

    #[test]
    fn unsigned_boundaries_roundtrip() {
        let cases: [(u128, u8); 10] = [
            (0, b'3'),
            (255, b'3'),
            (256, b'4'),
            (65535, b'4'),
            (65536, b'5'),
            (u32::MAX as u128, b'5'),
            (u32::MAX as u128 + 1, b'6'),
            (u64::MAX as u128, b'6'),
            (u64::MAX as u128 + 1, b'7'),
            (u128::MAX, b'7'),
        ];
        for (v, m) in cases {
            let b = wu(v);
            assert_eq!(b[0], m, "marker for {}", v);
            assert_eq!(ru::<u128>(&b), Ok(v));
        }
    }

    #[test]
    fn signed_boundaries_roundtrip() {
        let cases: [(i128, u8); 12] = [
            (0, b'3'),
            (127, b'3'),
            (128, b'4'),
            (-128, b'3'),
            (-129, b'4'),
            (i16::MAX as i128 + 1, b'5'),
            (i16::MIN as i128 - 1, b'5'),
            (i32::MAX as i128 + 1, b'6'),
            (i64::MAX as i128 + 1, b'7'),
            (i64::MIN as i128 - 1, b'7'),
            (i128::MAX, b'7'),
            (i128::MIN, b'7'),
        ];
        for (v, m) in cases {
            let b = wi(v);
            assert_eq!(b[0], m, "marker for {}", v);
            assert_eq!(ri::<i128>(&b), Ok(v));
        }
    }

    #[test]
    fn rejects_padded_unsigned() {
        assert!(ru::<u32>(&[b'4', 0, 5]).is_err());
        assert!(ru::<u32>(&[b'4', 0, 255]).is_err());
        assert_eq!(ru::<u32>(&[b'4', 1, 0]), Ok(256));
        let mut padded = vec![b'7'];
        padded.extend_from_slice(&[0; 15]);
        padded.push(2);
        assert!(ru::<usize>(&padded).is_err());
    }

    #[test]
    fn rejects_padded_signed() {
        assert!(ri::<i32>(&[b'4', 0, 5]).is_err());
        assert!(ri::<i32>(&[b'4', 0xFF, 0xFB]).is_err(), "-5 padded to 2 bytes");
        assert!(ri::<i32>(&[b'4', 0xFF, 0x80]).is_err(), "-128 padded to 2 bytes");
        assert_eq!(ri::<i32>(&[b'4', 0, 0x80]), Ok(128));
        assert_eq!(ri::<i32>(&[b'4', 0xFF, 0x7F]), Ok(-129));
    }

    #[test]
    fn no_silent_truncation() {
        // Bit 64 set: a u7 that doesn't fit usize/u64 must error, never wrap to a small length.
        let mut b = vec![b'7'];
        b.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(ru::<u64>(&b), Err(DecodeError::ValueOutOfRange));
        assert_eq!(ru::<u128>(&b), Ok((1u128 << 64) + 2));
        assert_eq!(ru::<u8>(&[b'4', 1, 0]), Err(DecodeError::ValueOutOfRange));
        assert_eq!(ri::<i8>(&[b'4', 0, 0x80]), Err(DecodeError::ValueOutOfRange));
        assert_eq!(ri::<i8>(&[b'3', 0xFF]), Ok(-1));
    }

    #[test]
    fn errors_leave_pointer_alone() {
        let b = [b'4', 0, 5];
        let mut p = 0;
        assert!(read_uint::<u32>(&b, &mut p).is_err());
        assert_eq!(p, 0);
        let mut p = 0;
        assert!(read_uint::<u8>(&[b'4', 1, 0], &mut p).is_err());
        assert_eq!(p, 0);
    }

    #[test]
    fn rejects_bad_markers_and_truncation() {
        assert!(ru::<u64>(&[]).is_err());
        assert!(ru::<u64>(&[b'8', 0]).is_err());
        assert!(ru::<u64>(&[b'2', 0]).is_err());
        assert!(ru::<u64>(&[b'5', 0, 1]).is_err());
    }
}
