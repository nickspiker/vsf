//! Handle identity system — thin re-export shim over the canonical [`ihi`] crate.
//!
//! The memory-hard handle-proof algorithm lives in the ihi crate (`/mnt/Octopus/Code/ihi`) as the single source of truth — every component in the stack (photon, vsf::handle, toka, fgtw) routes through it rather than rolling its own pre-hash step.
//!
//! **Wire format.** `handle_to_hash` encodes the handle via `VsfType::x` (NFC normalization + Huffman codebook covering all 1,112,064 valid Unicode codepoints) and BLAKE3-hashes the resulting bytes. `handle_to_proof` then feeds that hash into the memory-hard PoW. See [`ihi::handle_to_hash`] for the full canonicalization contract.

use crate::prelude::*;

/// Memory-hard public-ID proof from a pre-hashed handle. Re-export of [`ihi::handle_proof`].
pub use ihi::handle_proof;

/// Convenience: handle string → `VsfType::x` + BLAKE3 → memory-hard proof. Re-export of [`ihi::handle_to_proof`].
pub use ihi::handle_to_proof;

/// Base64url filename (with `.vsf` extension) from a handle proof. Re-export of [`ihi::proof_to_filename`].
pub use ihi::proof_to_filename;

/// Convenience: handle string → base64url `.vsf` filename. Re-export of [`ihi::handle_to_filename`].
pub use ihi::handle_to_filename;

/// Alias for [`ihi::handle_to_proof`]. Returns a [`blake3::Hash`] — the proof's 32 bytes.
pub fn handle_to_public_id(handle: &str) -> blake3::Hash {
    ihi::handle_to_proof(handle)
}

/// Alias for [`ihi::proof_to_filename`].
pub fn public_id_to_filename(id: &blake3::Hash) -> String {
    ihi::proof_to_filename(id)
}

/// Convenience: returns `(public_id, filename)` in one call. The proof computation is the expensive step (~1 s); the filename derivation is cheap.
pub fn resolve_handle(handle: &str) -> (blake3::Hash, String) {
    let id = ihi::handle_to_proof(handle);
    let filename = ihi::proof_to_filename(&id);
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
    fn handle_roundtrip() {
        let id = ihi::handle_to_proof("text test");
        let filename = ihi::proof_to_filename(&id);
        assert!(filename.ends_with(".vsf"));
    }

    #[test]
    fn matches_ihi_canonical() {
        let (shim_id, shim_filename) = resolve_handle("octopus");
        let canonical_id = ihi::handle_to_proof("octopus");
        let canonical_filename = ihi::proof_to_filename(&canonical_id);
        assert_eq!(shim_id, canonical_id);
        assert_eq!(shim_filename, canonical_filename);
    }
}
