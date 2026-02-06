//! Metadata parsers

use super::helpers::{decode_isize, decode_usize};
use crate::types::{EtType, VsfType, WorldCoord};
use std::io::{Error, ErrorKind};

// ==================== METADATA ====================

pub fn parse_string(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    #[cfg(feature = "text")]
    {
        use crate::text_encoding::decode_text_with_size;

        // Read character count
        let char_count = decode_usize(data, pointer)?;

        // Rest of data is Huffman-encoded bytes
        let huffman_bytes = &data[*pointer..];

        if huffman_bytes.is_empty() && char_count > 0 {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "No Huffman data for non-zero char count",
            ));
        }

        // Decode using Huffman decoder and get bytes consumed
        let (value, bytes_consumed) = decode_text_with_size(huffman_bytes, char_count)
            .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Huffman decode: {}", e)))?;

        // Advance pointer by actual bytes consumed
        *pointer += bytes_consumed;

        Ok(VsfType::x(value))
    }
    #[cfg(not(feature = "text"))]
    {
        // Without text feature, decode as raw UTF-8 bytes
        let byte_count = decode_usize(data, pointer)?;

        if *pointer + byte_count > data.len() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Not enough data for string bytes",
            ));
        }

        let bytes = &data[*pointer..*pointer + byte_count];
        let value = String::from_utf8(bytes.to_vec())
            .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

        *pointer += byte_count;
        Ok(VsfType::x(value))
    }
}

pub fn parse_eagle_time(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for eagle time type marker",
        ));
    }

    let time_type = data[*pointer];
    *pointer += 1;

    match time_type {
        b'u' => {
            let value = decode_usize(data, pointer)?;
            Ok(VsfType::e(EtType::u(value)))
        }
        b'i' => {
            let value = decode_isize(data, pointer)?;
            Ok(VsfType::e(EtType::i(value)))
        }
        b'f' => {
            // Eagle Time floats: [e][f][5 or 6][bytes]
            if *pointer >= data.len() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Not enough data for float precision marker",
                ));
            }

            let precision = data[*pointer];
            *pointer += 1;

            match precision {
                b'5' => {
                    // f32 (f5)
                    if *pointer + 4 > data.len() {
                        return Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "Not enough data for f5",
                        ));
                    }
                    let value = f32::from_be_bytes([
                        data[*pointer],
                        data[*pointer + 1],
                        data[*pointer + 2],
                        data[*pointer + 3],
                    ]);
                    *pointer += 4;
                    Ok(VsfType::e(EtType::f5(value)))
                }
                b'6' => {
                    // f64 (f6)
                    if *pointer + 8 > data.len() {
                        return Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "Not enough data for f6",
                        ));
                    }
                    let value = f64::from_be_bytes([
                        data[*pointer],
                        data[*pointer + 1],
                        data[*pointer + 2],
                        data[*pointer + 3],
                        data[*pointer + 4],
                        data[*pointer + 5],
                        data[*pointer + 6],
                        data[*pointer + 7],
                    ]);
                    *pointer += 8;
                    Ok(VsfType::e(EtType::f6(value)))
                }
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid Eagle Time float precision marker: {}",
                        precision as char
                    ),
                )),
            }
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid eagle time type",
        )),
    }
}

pub fn parse_world_coord(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    if *pointer + 8 > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for world coordinate",
        ));
    }

    let raw = u64::from_be_bytes([
        data[*pointer],
        data[*pointer + 1],
        data[*pointer + 2],
        data[*pointer + 3],
        data[*pointer + 4],
        data[*pointer + 5],
        data[*pointer + 6],
        data[*pointer + 7],
    ]);
    *pointer += 8;

    Ok(VsfType::w(WorldCoord::from_raw(raw)))
}

pub fn parse_dtype(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for dtype",
        ));
    }

    let bytes = &data[*pointer..*pointer + length];

    // Validate ASCII-only (identifiers like "imaging.raw", "iso_speed")
    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "dtype must be ASCII (identifiers only)",
        ));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::d(value))
}

pub fn parse_label(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for label",
        ));
    }

    let bytes = &data[*pointer..*pointer + length];

    // Validate ASCII-only (identifiers like field names, keys)
    if !bytes.iter().all(|&b| b.is_ascii()) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "label must be ASCII (identifiers only)",
        ));
    }

    let value = String::from_utf8(bytes.to_vec()).unwrap(); // Safe: validated ASCII
    *pointer += length;
    Ok(VsfType::l(value))
}

pub fn parse_offset(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let offset = decode_usize(data, pointer)?;
    Ok(VsfType::o(offset))
}

pub fn parse_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::b(length, false)) // Inclusive flag not relevant when parsing
}

pub fn parse_file_length(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let length = decode_usize(data, pointer)?;
    Ok(VsfType::L(length, false)) // Inclusive flag not relevant when parsing
}

pub fn parse_count(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let count = decode_usize(data, pointer)?;
    Ok(VsfType::n(count))
}

pub fn parse_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::z(version))
}

pub fn parse_backward_version(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let version = decode_usize(data, pointer)?;
    Ok(VsfType::y(version))
}

pub fn parse_marker_def(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::m(value))
}

pub fn parse_marker_ref(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    let value = decode_usize(data, pointer)?;
    Ok(VsfType::m(value))
}

pub fn parse_mac(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm byte (h for HMAC-SHA256, s for HMAC-SHA512, etc.)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for MAC algorithm",
        ));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read MAC tag data
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for MAC tag",
        ));
    }
    let mac_tag = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate MAC type based on algorithm
    match algo {
        b'h' => Ok(VsfType::ah(mac_tag)), // HMAC-SHA (size disambiguates 256/512)
        b'p' => Ok(VsfType::ap(mac_tag)),
        b'b' => Ok(VsfType::ab(mac_tag)),
        b'c' => Ok(VsfType::ac(mac_tag)),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown MAC algorithm: {}", algo as char),
        )),
    }
}

pub fn parse_hash(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm byte (b for BLAKE3, s for SHA)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for hash algorithm",
        ));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read hash data
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for hash",
        ));
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
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown hash algorithm: {}", algo as char),
        )),
    }
}

pub fn parse_signature(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm byte (e for Ed25519, p for ECDSA-P256, r for RSA)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signature algorithm",
        ));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read signature data
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for signature",
        ));
    }
    let sig = data[*pointer..*pointer + length].to_vec();
    *pointer += length;

    // Return appropriate signature type
    match algo {
        b'e' => Ok(VsfType::ge(sig)),
        b'p' => Ok(VsfType::gp(sig)),
        b'r' => Ok(VsfType::gr(sig)),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown signature algorithm: {}", algo as char),
        )),
    }
}

pub fn parse_key(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm byte (e for Ed25519, x for X25519, s for shared secrets, etc.)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for key algorithm",
        ));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Handle shared secrets (ks*) - these have a 3-byte prefix
    if algo == b's' {
        // This is a shared secret - dispatch to parse_shared_secret
        // Note: pointer is now at the third byte (algorithm variant)
        return parse_shared_secret(data, pointer);
    }

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read key data
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for key",
        ));
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
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown key algorithm: {}", algo as char),
        )),
    }
}

/// Parse shared secret (ks* types - 3-byte prefix)
pub fn parse_shared_secret(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm byte (x for X25519, p for P-curve, k for secp256k1, etc.)
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for shared secret algorithm",
        ));
    }
    let algo = data[*pointer];
    *pointer += 1;

    // Read length (stored as len-1) using standard VSF variable-length encoding
    let length = decode_usize(data, pointer)? + 1; // Add 1 back

    // Read secret data
    if *pointer + length > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for shared secret",
        ));
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
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown shared secret algorithm: {}", algo as char),
        )),
    }
}

/// Parse wrapped/encoded data (v type)
pub fn parse_wrapped(data: &[u8], pointer: &mut usize) -> Result<VsfType, Error> {
    // Read algorithm ID byte
    if *pointer >= data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for wrapped data algorithm ID",
        ));
    }
    let algorithm = data[*pointer];
    *pointer += 1;

    // Read wrapped data length and data
    let length_bits = decode_usize(data, pointer)?;
    let length_bytes = (length_bits + 7) >> 3; // Convert bits to bytes (round up)
    if *pointer + length_bytes > data.len() {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Not enough data for wrapped data",
        ));
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
) -> Result<(usize, usize, Option<Vec<u8>>, Option<Vec<u8>>), Error> {
    // Expect opening brace
    if *pointer >= data.len() || data[*pointer] != b'{' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Expected '{{' for preamble at byte {}", pointer),
        ));
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
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Preamble hash extends beyond data",
                    ));
                }
                hash = Some(data[*pointer..*pointer + hash_len].to_vec());
                *pointer += hash_len;
            }
            b'g' => {
                // Parse signature
                let sig_len = decode_usize(data, pointer)?;
                if *pointer + sig_len > data.len() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "Preamble signature extends beyond data",
                    ));
                }
                signature = Some(data[*pointer..*pointer + sig_len].to_vec());
                *pointer += sig_len;
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Unknown preamble marker: {}", marker as char),
                ));
            }
        }
    }

    // Expect closing brace
    if *pointer >= data.len() || data[*pointer] != b'}' {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Expected '}}' to close preamble at byte {}", pointer),
        ));
    }
    *pointer += 1;

    // Verify required fields
    let count = count
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing 'n' (count) in preamble"))?;

    let size_bits = size_bits
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Missing 'b' (size) in preamble"))?;

    Ok((count, size_bits, hash, signature))
}
