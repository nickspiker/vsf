//! Metadata parsers

use crate::decoding::traits::DecodeError;
use crate::prelude::*;
use super::helpers::{decode_i64, decode_u64, decode_usize};
use crate::types::{EtType, VsfType, WorldCoord};

// ==================== METADATA ====================

pub fn parse_string(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Gate must match the ENCODER's gate in encoding/flatten.rs (`any(text, text-encode)`). vsf 0.5.0 shipped with `text` alone here, so builds with only `text-encode` wrote Huffman but read raw UTF-8 — every x roundtrip failed.
    #[cfg(any(feature = "text", feature = "text-encode"))]
    {
        use crate::text_encoding::decode_text_with_size;

        // Read character count
        let char_count = decode_usize(data, pointer)?;

        // Rest of data is Huffman-encoded bytes
        let huffman_bytes = &data[*pointer..];

        if huffman_bytes.is_empty() && char_count > 0 {
            return Err(DecodeError::UnexpectedEofMsg("No Huffman data for non-zero char count".into()));
        }

        // Decode using Huffman decoder and get bytes consumed
        let (value, bytes_consumed) = decode_text_with_size(huffman_bytes, char_count)
            .map_err(|e| DecodeError::InvalidDataMsg(format!("Huffman decode: {}", e)))?;

        // Advance pointer by actual bytes consumed
        *pointer += bytes_consumed;

        Ok(VsfType::x(value))
    }
    #[cfg(not(any(feature = "text", feature = "text-encode")))]
    {
        // x is Huffman-coded by spec — ALWAYS. There is no raw-bytes form of x on the wire; ASCII text has its own type. A build without the text machinery cannot interpret x, and silently reinterpreting the bitstream as UTF-8 (what this branch did thru 0.5.0) can return a WRONG string without erroring. Mirror the encoder (which panics) by refusing loudly.
        let _ = (data, pointer);
        Err(DecodeError::InvalidDataMsg(
            "VsfType::x is Huffman-coded and requires the 'text' or 'text-encode' feature to decode".into(),
        ))
    }
}

pub fn parse_eagle_time(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for eagle time type marker".into()));
    }

    let time_type = data[*pointer];
    *pointer += 1;

    #[allow(deprecated)]
    match time_type {
        b'5' => {
            // e5: 32-bit signed oscillation count, 4 bytes BE
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Not enough data for e5".into()));
            }
            let value = i32::from_be_bytes([
                data[*pointer], data[*pointer + 1],
                data[*pointer + 2], data[*pointer + 3],
            ]);
            *pointer += 4;
            Ok(VsfType::e(EtType::e5(value)))
        }
        b'6' => {
            // e6: 64-bit signed oscillation count, 8 bytes BE (canonical form)
            if *pointer + 8 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Not enough data for e6".into()));
            }
            let value = i64::from_be_bytes([
                data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
                data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
            ]);
            *pointer += 8;
            Ok(VsfType::e(EtType::e6(value)))
        }
        b'7' => {
            // e7: 128-bit signed oscillation count, 16 bytes BE
            if *pointer + 16 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Not enough data for e7".into()));
            }
            let value = i128::from_be_bytes([
                data[*pointer],     data[*pointer + 1],  data[*pointer + 2],  data[*pointer + 3],
                data[*pointer + 4], data[*pointer + 5],  data[*pointer + 6],  data[*pointer + 7],
                data[*pointer + 8], data[*pointer + 9],  data[*pointer + 10], data[*pointer + 11],
                data[*pointer + 12],data[*pointer + 13], data[*pointer + 14], data[*pointer + 15],
            ]);
            *pointer += 16;
            Ok(VsfType::e(EtType::e7(value)))
        }
        b'u' => {
            // Legacy: eu = unsigned EWE, widen to i64 → e6 (compat, not emitted)
            let value = decode_u64(data, pointer)?;
            Ok(VsfType::e(EtType::e6(value as i64)))
        }
        b'i' => {
            // Legacy: ei = signed EWE i64 → e6 (compat, not emitted)
            let value = decode_i64(data, pointer)?;
            Ok(VsfType::e(EtType::e6(value)))
        }
        b'f' => {
            // Legacy: ef5/ef6 = float seconds (deprecated, still parsed)
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Not enough data for float precision marker".into()));
            }
            let precision = data[*pointer];
            *pointer += 1;
            match precision {
                b'5' => {
                    if *pointer + 4 > data.len() {
                        return Err(DecodeError::UnexpectedEofMsg("Not enough data for ef5".into()));
                    }
                    let value = f32::from_be_bytes([
                        data[*pointer], data[*pointer + 1],
                        data[*pointer + 2], data[*pointer + 3],
                    ]);
                    *pointer += 4;
                    Ok(VsfType::e(EtType::f5(value)))
                }
                b'6' => {
                    if *pointer + 8 > data.len() {
                        return Err(DecodeError::UnexpectedEofMsg("Not enough data for ef6".into()));
                    }
                    let value = f64::from_be_bytes([
                        data[*pointer], data[*pointer + 1], data[*pointer + 2], data[*pointer + 3],
                        data[*pointer + 4], data[*pointer + 5], data[*pointer + 6], data[*pointer + 7],
                    ]);
                    *pointer += 8;
                    Ok(VsfType::e(EtType::f6(value)))
                }
                _ => Err(DecodeError::InvalidDataMsg(format!("Invalid Eagle Time float precision marker: {}", precision as char))),
            }
        }
        _ => Err(DecodeError::InvalidDataMsg(format!("Invalid eagle time type marker: {}", time_type as char))),
    }
}

pub fn parse_world_coord(
    size_byte: u8,
    data: &[u8],
    pointer: &mut usize,
) -> Result<VsfType, DecodeError> {
    let bytes_needed = match size_byte {
        b'3' => 1,
        b'4' => 2,
        b'5' => 4,
        b'6' => 8,
        b'7' => 16,
        _ => {
            return Err(DecodeError::InvalidDataMsg(format!("Invalid world coord size: w{}", size_byte as char)))
        }
    };
    if *pointer + bytes_needed > data.len() {
        return Err(DecodeError::UnexpectedEofMsg(format!("Not enough data for w{}", size_byte as char)));
    }
    let coord = WorldCoord::from_wire(size_byte, &data[*pointer..]).ok_or_else(|| {
        DecodeError::InvalidDataMsg("WorldCoord::from_wire rejected input".into())
    })?;
    *pointer += bytes_needed;
    Ok(VsfType::w(coord))
}

pub fn parse_dtype(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for dtype".into()));
    }

    let bytes = &data[*pointer..*pointer + length];

    // Validate ASCII-only (identifiers like "imaging.raw", "iso_speed")
    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(DecodeError::InvalidDataMsg("dtype must be ASCII (identifiers only.into())".into()));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::d(value))
}

pub fn parse_ascii(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for ASCII text".into()));
    }

    let bytes = &data[*pointer..*pointer + length];

    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(DecodeError::InvalidDataMsg("value must be ASCII".into()));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::a(value))
}

pub fn parse_offset(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let offset = decode_usize(data, pointer)?;
    Ok(VsfType::o(offset))
}

pub fn parse_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::b(length, false)) // Inclusive flag not relevant when parsing
}

pub fn parse_l_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::l(length, false))
}

pub fn parse_count(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let count = decode_usize(data, pointer)?;
    Ok(VsfType::n(count))
}

pub fn parse_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::z(version))
}

pub fn parse_backward_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::y(version))
}

/// Parse colour constant: `rc[a-z]`
///
/// Format: 3 bytes total - 'r', 'c', colour letter Examples: rcn (green), rcr (red), rcb (blue), etc.
pub fn parse_colour_constant(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Already consumed 'r', now expect 'c' and colour letter
    if *pointer + 2 > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Incomplete colour constant (need 2 more bytes after 'r'.into())".into()));
    }

    let second = data[*pointer];
    let third = data[*pointer + 1];
    *pointer += 2;

    // Validate 'c' marker
    if second != b'c' {
        return Err(DecodeError::InvalidDataMsg(format!(
                "Invalid colour constant: expected 'c', got '{}'",
                second as char)));
    }

    // Match colour letter
    match third {
        b'k' => Ok(VsfType::rck), // Black
        b'w' => Ok(VsfType::rcw), // White
        b'r' => Ok(VsfType::rcr), // Red
        b'n' => Ok(VsfType::rcn), // Green
        b'b' => Ok(VsfType::rcb), // Blue
        b'c' => Ok(VsfType::rcc), // Cyan
        b'j' => Ok(VsfType::rcj), // Magenta
        b'y' => Ok(VsfType::rcy), // Yellow
        b'g' => Ok(VsfType::rcg), // Gray
        b'o' => Ok(VsfType::rco), // Orange
        b'v' => Ok(VsfType::rcv), // Violet
        b'l' => Ok(VsfType::rcl), // Lime
        b'q' => Ok(VsfType::rcq), // Aqua
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown colour constant: rc{}", third as char))),
    }
}

/// Parse colour array types: ra (RGBA u8x4), rt (RGBA u16x4), rp (RGB565 u16)
pub fn parse_colour_array(
    data: &[u8],
    pointer: &mut usize,
    colour_type: u8,
) -> Result<VsfType, DecodeError> {
    match colour_type {
        b'a' => {
            // ra: [u8; 4] RGBA
            if *pointer + 4 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Incomplete ra colour (need 4 bytes.into())".into()));
            }
            let rgba = [
                data[*pointer],
                data[*pointer + 1],
                data[*pointer + 2],
                data[*pointer + 3],
            ];
            *pointer += 4;
            Ok(VsfType::ra(rgba))
        }
        b't' => {
            // rt: [u16; 4] RGBA
            if *pointer + 8 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Incomplete rt colour (need 8 bytes.into())".into()));
            }
            let rt_rgba = [
                u16::from_le_bytes([data[*pointer], data[*pointer + 1]]),
                u16::from_le_bytes([data[*pointer + 2], data[*pointer + 3]]),
                u16::from_le_bytes([data[*pointer + 4], data[*pointer + 5]]),
                u16::from_le_bytes([data[*pointer + 6], data[*pointer + 7]]),
            ];
            *pointer += 8;
            Ok(VsfType::rt(rt_rgba))
        }
        b'p' => {
            // rp: u16 RGB565
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg("Incomplete rp colour (need 2 bytes.into())".into()));
            }
            let rgb565 = u16::from_le_bytes([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
            Ok(VsfType::rp(rgb565))
        }
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown colour array type: r{}", colour_type as char))),
    }
}

pub fn parse_mac(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm byte (h for HMAC-SHA256, s for HMAC-SHA512, etc.)
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for MAC algorithm".into()));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read MAC tag data
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for MAC tag".into()));
    }
    let mac_tag = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate MAC type based on algorithm
    match algo {
        b'h' => Ok(VsfType::mh(mac_tag)), // HMAC-SHA (size disambiguates 256/512)
        b'p' => Ok(VsfType::mp(mac_tag)),
        b'b' => Ok(VsfType::mb(mac_tag)),
        b'c' => Ok(VsfType::mc(mac_tag)),
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown MAC algorithm: {}", algo as char))),
    }
}

pub fn parse_hash(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm byte (b for BLAKE3, s for SHA)
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for hash algorithm".into()));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read hash data
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for hash".into()));
    }
    let hash = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate hash type based on algorithm
    match algo {
        b'p' => Ok(VsfType::hp(hash)),
        b'b' => Ok(VsfType::hb(hash)),
        b's' => Ok(VsfType::hs(hash)),
        b'm' => Ok(VsfType::hm(hash)),
        b'g' => Ok(VsfType::hg(hash)),
        b'P' => Ok(VsfType::hP(hash)), // Photon handle proof
        b'R' => Ok(VsfType::hR(hash)), // Random padding material
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown hash algorithm: {}", algo as char))),
    }
}

pub fn parse_signature(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm byte (e for Ed25519, p for ECDSA-P256, r for RSA)
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for signature algorithm".into()));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read signature data
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for signature".into()));
    }
    let sig = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate signature type
    match algo {
        b'e' => Ok(VsfType::ge(sig)),
        b'p' => Ok(VsfType::gp(sig)),
        b'r' => Ok(VsfType::gr(sig)),
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown signature algorithm: {}", algo as char))),
    }
}

pub fn parse_key(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm byte (e for Ed25519, x for X25519, s for shared secrets, etc.)
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for key algorithm".into()));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Handle shared secrets (ks*) - these have a 3-byte prefix
    if algo == b's' {
        // This is a shared secret - dispatch to parse_shared_secret Note: pointer is now at the third byte (algorithm variant)
        return parse_shared_secret(data, pointer);
    }

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read key data
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for key".into()));
    }
    let key = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate key type
    match algo {
        b'e' => Ok(VsfType::ke(key)),
        b'x' => Ok(VsfType::kx(key)),
        b'p' => Ok(VsfType::kp(key)),
        b'k' => Ok(VsfType::kk(key)),
        b'c' => Ok(VsfType::kc(key)),
        b'a' => Ok(VsfType::ka(key)),
        b'm' => Ok(VsfType::km(key)),
        b'f' => Ok(VsfType::kf(key)),
        b'l' => Ok(VsfType::kl(key)),
        b'n' => Ok(VsfType::kn(key)), // NTRU public key
        b'h' => Ok(VsfType::kh(key)), // HQC public key
        b'd' => Ok(VsfType::kd(key)), // Dilithium/ML-DSA public key
        b'b' => Ok(VsfType::kb(key)), // BIKE public key
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown key algorithm: {}", algo as char))),
    }
}

/// Parse shared secret (ks* types - 3-byte prefix)
pub fn parse_shared_secret(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm byte (x for X25519, p for P-curve, k for secp256k1, etc.)
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for shared secret algorithm".into()));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read secret data
    if *pointer + length > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for shared secret".into()));
    }
    let secret = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate shared secret type
    match algo {
        b'x' => Ok(VsfType::ksx(secret)), // X25519
        b'p' => Ok(VsfType::ksp(secret)), // P-curve (P-256/P-384)
        b'k' => Ok(VsfType::ksk(secret)), // secp256k1
        b'f' => Ok(VsfType::ksf(secret)), // Frodo
        b'n' => Ok(VsfType::ksn(secret)), // NTRU
        b'l' => Ok(VsfType::ksl(secret)), // McEliece
        b'h' => Ok(VsfType::ksh(secret)), // HQC
        b'm' => Ok(VsfType::ksm(secret)), // ML-KEM
        _ => Err(DecodeError::InvalidDataMsg(format!("Unknown shared secret algorithm: {}", algo as char))),
    }
}

/// Parse wrapped/encoded data (v type)
pub fn parse_wrapped(data: &[u8], pointer: &mut usize) -> Result<VsfType, DecodeError> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for wrapped data algorithm ID".into()));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read wrapped data length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    if *pointer + length_bytes > data.len() {
        return Err(DecodeError::UnexpectedEofMsg("Not enough data for wrapped data".into()));
    }
    let wrapped_data = data[*pointer..*pointer + length_bytes].to_vec();
    *pointer += length_bytes;
    Ok(VsfType::v(algorithm, wrapped_data))
}

// ==================== PREAMBLE ====================

/// Parse a preamble from VSF data
///
/// Format: {n[count] b[size] h?[hash] g?[signature]}
///
/// Returns (count, size_bits, hash, signature, bytes_consumed)
pub fn parse_preamble(
    data: &[u8],
    pointer: &mut usize,
) -> Result<(usize, usize, Option<Vec<u8>>, Option<Vec<u8>>), DecodeError> {
    // Expect opening brace
    if *pointer >= data.len() || data[*pointer] != b'{' {
        return Err(DecodeError::InvalidDataMsg(format!("Expected '{{' for preamble at byte {}", pointer)));
    }
    *pointer += 1;

    let mut count = None;
    let mut size_bits = None;
    let mut hash = None;
    let mut signature = None;

    // Parse fields until closing brace
    while *pointer < data.len() && data[*pointer] != b'}' {
        let marker = data[*pointer];
        *pointer += 1;

        match marker {
            b'n' => {
                // Parse count
                count = Some(decode_usize(data, pointer)?);
            }
            b'b' => {
                // Parse size in bits
                size_bits = Some(decode_usize(data, pointer)?);
            }
            b'h' => {
                // Parse hash
                let hash_len = decode_usize(data, pointer)?;
                if *pointer + hash_len > data.len() {
                    return Err(DecodeError::UnexpectedEofMsg("Preamble hash extends beyond data".into()));
                }
                hash = Some(data[*pointer..*pointer + hash_len].to_vec());
                *pointer += hash_len;
            }
            b'g' => {
                // Parse signature
                let sig_len = decode_usize(data, pointer)?;
                if *pointer + sig_len > data.len() {
                    return Err(DecodeError::UnexpectedEofMsg("Preamble signature extends beyond data".into()));
                }
                signature = Some(data[*pointer..*pointer + sig_len].to_vec());
                *pointer += sig_len;
            }
            _ => {
                return Err(DecodeError::InvalidDataMsg(format!("Unknown preamble marker: {}", marker as char)));
            }
        }
    }

    // Expect closing brace
    if *pointer >= data.len() || data[*pointer] != b'}' {
        return Err(DecodeError::InvalidDataMsg(format!("Expected '}}' to close preamble at byte {}", pointer)));
    }
    *pointer += 1;

    // Verify required fields
    let count = count
        .ok_or_else(|| DecodeError::InvalidDataMsg("Missing 'n' (count) in preamble".into()))?;

    let size_bits = size_bits
        .ok_or_else(|| DecodeError::InvalidDataMsg("Missing 'b' (size) in preamble".into()))?;

    Ok((count, size_bits, hash, signature))
}
