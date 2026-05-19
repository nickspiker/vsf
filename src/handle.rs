//! Handle identity system
//!
//! Handles are plaintext names that map to deterministic public IDs via memory-hard proof-of-work. The same handle always produces the same ID.
//!
//! Flow: plaintext → VsfType::x(handle).flatten() → blake3 → handle_proof → public ID
//!
//! The public ID becomes the capsule's provenance hash, and the base64url-encoded
//! ID becomes the filename for storage/retrieval.

use crate::prelude::*;
use blake3;
use i256::U256;

/// Size of the scratch buffer (24MB). Fits in L3 cache, prevents bulk parallelization.
const SIZE: usize = 24_873_856;
/// BLAKE3 output / chunk size.
const CHUNK_SIZE: usize = 32;
/// Number of sequential rounds. Tuned for ~1s on 2025 desktop hardware.
const ROUNDS: usize = 17;

/// Compute the deterministic public ID for a handle hash.
///
/// Takes the BLAKE3 hash of a VSF-encoded handle string and produces a public ID through 17 rounds of memory-hard sequential processing.
///
/// # Algorithm
///
/// Each round:
/// 0. Variable fill determination — hash determines buffer fill (25-75% of 24MB)
/// 1. Sequential hash chain — each chunk depends on previous (non-seekable)
/// 2. Data-dependent reads — random reads from earlier chunks (cache-hostile)
/// 3. State advancement — round output becomes input to next round
///
/// # Security Properties
///
/// - ~1 second per handle (anti-squatting)
/// - Sequential rounds prevent parallelization
/// - Data-dependent reads resist ASIC optimization
/// - Deterministic: same handle → same ID
/// - Verifiable: anyone can recompute
pub fn handle_proof(hash: &blake3::Hash) -> blake3::Hash {
    let mut scratch = Vec::with_capacity(SIZE);
    // SAFETY: Buffer is completely filled by Phase 1 and Phase 2 before the final hash. SIZE is exactly divisible by CHUNK_SIZE (24_873_856 / 32 = 777_308).
    unsafe {
        scratch.set_len(SIZE);
    }

    let mut round_hash = *hash;

    for round in 0..ROUNDS {
        let hash_num =
            U256::from_be_bytes(*round_hash.as_bytes()).wrapping_add(U256::from(round as u128));

        // Phase 0: Determine fill size (25-75% of buffer), aligned to CHUNK_SIZE
        let min_fill = SIZE / 4;
        let max_fill = SIZE * 3 / 4;
        let fill_range = max_fill - min_fill;
        let fill_size_raw =
            min_fill + ((hash_num % U256::from(fill_range as u128)).as_u128() as usize);
        let fill_size = (fill_size_raw / CHUNK_SIZE) * CHUNK_SIZE;

        // Phase 1: Sequential hash chain (memory-hard, non-seekable)
        scratch[..CHUNK_SIZE].copy_from_slice(round_hash.as_bytes());

        for i in 1..(fill_size / CHUNK_SIZE) {
            let prev_start = (i - 1) * CHUNK_SIZE;
            let curr_start = i * CHUNK_SIZE;

            let prev_hash = blake3::hash(&scratch[prev_start..prev_start + CHUNK_SIZE]);

            let hash_num_out = U256::from_be_bytes(*prev_hash.as_bytes())
                .wrapping_add(hash_num)
                .wrapping_add(U256::from(i as u128));

            scratch[curr_start..curr_start + CHUNK_SIZE]
                .copy_from_slice(&hash_num_out.to_be_bytes());
        }

        // Phase 2: Data-dependent random reads (cache-hostile, ASIC-resistant)
        let mut curr_start = fill_size;

        while curr_start + CHUNK_SIZE <= SIZE {
            let prev_hash_num = U256::from_be_bytes(
                scratch[curr_start - CHUNK_SIZE..curr_start]
                    .try_into()
                    .unwrap(),
            );

            let read_idx =
                (prev_hash_num % U256::from((curr_start - CHUNK_SIZE) as u128)).as_u128() as usize;

            let prev_hash = blake3::hash(&scratch[read_idx..read_idx + CHUNK_SIZE]);

            let new_val = U256::from_be_bytes(*prev_hash.as_bytes())
                .wrapping_add(hash_num)
                .wrapping_add(U256::from(curr_start as u128));

            scratch[curr_start..curr_start + CHUNK_SIZE].copy_from_slice(&new_val.to_be_bytes());
            curr_start += CHUNK_SIZE;
        }

        round_hash = blake3::hash(&scratch);
    }

    round_hash
}

/// Compute the public ID for a plaintext handle string.
///
/// VSF-encodes the handle as `VsfType::a` (ASCII text), hashes it, runs the proof. Uses `a` not `x` to avoid the Huffman/text feature dependency — handles are ASCII.
///
/// NOTE: photon's derive_identity_seed uses VsfType::x — update it to VsfType::a.
pub fn handle_to_public_id(handle: &str) -> blake3::Hash {
    let vsf_bytes = crate::types::VsfType::a(handle.to_string()).flatten();
    let handle_hash = blake3::hash(&vsf_bytes);
    handle_proof(&handle_hash)
}

/// Encode a public ID as a base64url filename (no padding) with .vsf extension.
///
/// Uses RFC 4648 base64url alphabet (A-Z, a-z, 0-9, -, _) for filesystem safety.
pub fn public_id_to_filename(id: &blake3::Hash) -> String {
    let bytes = id.as_bytes();
    let mut encoded = String::with_capacity(44); // ceil(32 * 4/3) = 43 chars

    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        encoded.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        encoded.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < bytes.len() {
            encoded.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        }
        if i + 2 < bytes.len() {
            encoded.push(ALPHABET[(triple & 0x3F) as usize] as char);
        }

        i += 3;
    }

    format!("{}.vsf", encoded)
}

/// Resolve a plaintext handle to its base64url filename.
///
/// Convenience: handle_to_public_id + public_id_to_filename.
pub fn resolve_handle(handle: &str) -> (blake3::Hash, String) {
    let id = handle_to_public_id(handle);
    let filename = public_id_to_filename(&id);
    (id, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_proof() {
        let hash = blake3::hash(b"test");
        let id1 = handle_proof(&hash);
        let id2 = handle_proof(&hash);
        assert_eq!(id1, id2, "same input must produce same output");
    }

    #[test]
    fn filename_is_valid() {
        let hash = blake3::hash(b"test");
        let filename = public_id_to_filename(&hash);
        assert!(filename.ends_with(".vsf"));
        // base64url: only A-Z, a-z, 0-9, -, _
        let stem = &filename[..filename.len() - 4];
        assert!(stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn handle_roundtrip() {
        let (id, filename) = resolve_handle("text test");
        println!("Handle 'text test' → {} → {}", id.to_hex(), filename);
        assert!(filename.ends_with(".vsf"));
    }
}
