//! Handle identity system — thin re-export shim over the canonical [`ihi`] crate.
//!
//! Historical: this module used to carry its own copy of the memory-hard handle-proof algorithm. The ihi crate (`/mnt/Octopus/Code/ihi`) is now the single source of truth — its `handle.rs` documents the consolidation explicitly: *"every component in the stack (photon, vsf::handle, toka, fgtw) should call this rather than rolling its own pre-hash step. Drift between callers caused real bugs (handle 'octopus' producing different proofs on different clients) before this consolidation."*
//!
//! **Wire-format note.** ihi hashes the **raw UTF-8 bytes** of the handle (no VSF framing, no length prefix, no salt). The previous vsf-internal `handle_to_public_id` wrapped the handle in `VsfType::a(...).flatten()` before hashing, producing different output bytes for the same input handle. Capsules registered under the old algorithm need re-publishing under the ihi spec. Per AGENT.md's no-fork-bullshit rule this is a clean cut — all clients update, old proofs are invalidated.

use crate::prelude::*;

/// Memory-hard public-ID proof from a pre-hashed handle. Re-export of [`ihi::handle_proof`].
pub use ihi::handle_proof;

/// Convenience: handle string → BLAKE3 hash → memory-hard proof. Re-export of [`ihi::handle_to_proof`]. Raw UTF-8 input, no VSF framing.
pub use ihi::handle_to_proof;

/// Base64url filename (with `.vsf` extension) from a handle proof. Re-export of [`ihi::proof_to_filename`].
pub use ihi::proof_to_filename;

/// Convenience: handle string → base64url `.vsf` filename. Re-export of [`ihi::handle_to_filename`].
pub use ihi::handle_to_filename;

/// Backwards-compatibility alias for the pre-consolidation `handle_to_public_id` entry point. Now delegates to [`ihi::handle_to_proof`] (raw UTF-8 hash → memory-hard proof). Returns a [`blake3::Hash`] — the proof's 32 bytes.
#[deprecated(since = "0.3.6", note = "Use ihi::handle_to_proof directly")]
pub fn handle_to_public_id(handle: &str) -> blake3::Hash {
    ihi::handle_to_proof(handle)
}

/// Backwards-compatibility alias for the pre-consolidation `public_id_to_filename` entry point. Now delegates to [`ihi::proof_to_filename`].
#[deprecated(since = "0.3.6", note = "Use ihi::proof_to_filename directly")]
pub fn public_id_to_filename(id: &blake3::Hash) -> String {
    ihi::proof_to_filename(id)
}

/// Backwards-compatibility convenience: returns `(public_id, filename)` in one call. Existing toka call sites depend on this two-value return; new callers should use [`ihi::handle_to_proof`] + [`ihi::proof_to_filename`] directly (single ~1-second proof computation, then cheap filename derivation).
#[deprecated(since = "0.3.6", note = "Use ihi::handle_to_proof + ihi::proof_to_filename")]
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
        // Smoke test: the deprecated shim path produces the same bytes as calling ihi directly.
        #[allow(deprecated)]
        let (shim_id, shim_filename) = resolve_handle("octopus");
        let canonical_id = ihi::handle_to_proof("octopus");
        let canonical_filename = ihi::proof_to_filename(&canonical_id);
        assert_eq!(shim_id, canonical_id);
        assert_eq!(shim_filename, canonical_filename);
    }
}
