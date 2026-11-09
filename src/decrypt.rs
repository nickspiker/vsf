//! VSF Decryption Support
//!
//! Provides functions for decrypting VSF v'e' (ephemeral X25519 + AES-256-GCM) encrypted content.

#[cfg(feature = "crypto")]
use crate::prelude::*;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(feature = "crypto")]
use ed25519_dalek::{SigningKey, VerifyingKey};
#[cfg(feature = "crypto")]
use x25519_dalek::{PublicKey, StaticSecret};

#[cfg(feature = "crypto")]
use crate::{parse, VsfType};
#[cfg(all(feature = "crypto", feature = "std"))]
use std::fs;
#[cfg(all(feature = "crypto", feature = "std"))]
use std::path::Path;

/// Keypair with both Ed25519 and derived X25519 keys
#[cfg(feature = "crypto")]
#[derive(Clone)]
pub struct Keypair {
    pub ed25519_secret: SigningKey,
    pub ed25519_public: VerifyingKey,
    pub x25519_secret: StaticSecret,
    pub x25519_public: PublicKey,
}

#[cfg(feature = "crypto")]
impl Keypair {
    /// Load keypair from VSF file (expects fgtw_device_key or similar format) Format: [d"fgtw_device_key" (d"secret":ke{32}{bytes}) (d"public":ke{32}{bytes})]
    #[cfg(feature = "std")]
    pub fn load_from_vsf(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes =
            fs::read(path.as_ref()).map_err(|e| format!("Failed to read key file: {}", e))?;

        // Verify magic number
        if bytes.len() < 4 || &bytes[0..4] != b"R\xC3\x85<" {
            return Err("Invalid VSF magic number".to_string());
        }

        // Use VsfHeader::decode() for proper parsing
        let (_, header_bytes_consumed) = crate::file_format::VsfHeader::decode(&bytes)
            .map_err(|e| format!("Failed to parse VSF header: {}", e))?;

        // Parse the section after the header
        let mut ptr = header_bytes_consumed;

        // Skip to section start '['
        while ptr < bytes.len() && bytes[ptr] != b'[' {
            ptr += 1;
        }

        if ptr >= bytes.len() {
            return Err("No section found".to_string());
        }

        ptr += 1; // Skip '['

        // Parse section name
        let _section_name =
            parse(&bytes, &mut ptr).map_err(|e| format!("Parse section name: {}", e))?;

        // Parse fields until ']'
        // A handful of fields: a Vec searched from the back, so a repeated name resolves to its last value as a map insert would.
        let mut fields: Vec<(String, VsfType)> = Vec::new();

        while ptr < bytes.len() && bytes[ptr] != b']' {
            if bytes[ptr] == b'(' {
                ptr += 1;

                // Parse field name
                let field_name = match parse(&bytes, &mut ptr)
                    .map_err(|e| format!("Parse field name: {}", e))?
                {
                    VsfType::d(name) => name,
                    _ => return Err("Expected field name".to_string()),
                };

                // Check for ':' and parse value
                if ptr < bytes.len() && bytes[ptr] == b':' {
                    ptr += 1;
                    let value =
                        parse(&bytes, &mut ptr).map_err(|e| format!("Parse field value: {}", e))?;
                    fields.push((field_name, value));
                }

                // Skip ')'
                if ptr < bytes.len() && bytes[ptr] == b')' {
                    ptr += 1;
                }
            } else {
                ptr += 1;
            }
        }

        // Extract secret and public keys from fields
        let secret_bytes = match fields.iter().rev().find(|(n, _)| n == "secret").map(|(_, v)| v) {
            Some(VsfType::ke(bytes)) => {
                if bytes.len() != 32 {
                    return Err("Secret key must be 32 bytes".to_string());
                }
                bytes.clone()
            }
            _ => return Err("Secret key not found".to_string()),
        };

        let public_bytes = match fields.iter().rev().find(|(n, _)| n == "public").map(|(_, v)| v) {
            Some(VsfType::ke(bytes)) => {
                if bytes.len() != 32 {
                    return Err("Public key must be 32 bytes".to_string());
                }
                bytes.clone()
            }
            _ => return Err("Public key not found".to_string()),
        };

        // Reconstruct Ed25519 keypair
        let ed25519_secret = SigningKey::from_bytes(
            &secret_bytes
                .try_into()
                .map_err(|_| "Invalid secret key length".to_string())?,
        );

        let ed25519_public = VerifyingKey::from_bytes(
            &public_bytes
                .try_into()
                .map_err(|_| "Invalid public key length".to_string())?,
        )
        .map_err(|e| format!("Invalid public key: {}", e))?;

        // Derive X25519 keys from Ed25519 Ed25519 secret key is 32 bytes, X25519 needs 32 bytes
        let x25519_secret = StaticSecret::from(ed25519_secret.to_bytes());
        let x25519_public = PublicKey::from(&x25519_secret);

        Ok(Self {
            ed25519_secret,
            ed25519_public,
            x25519_secret,
            x25519_public,
        })
    }
}

/// Decrypt v'e' encrypted data (ephemeral X25519 + AES-256-GCM) Format: [ephemeral_pubkey:32][nonce:12][ciphertext+tag]
#[cfg(feature = "crypto")]
pub fn decrypt_ve(encrypted_bytes: &[u8], x25519_secret: &StaticSecret) -> Result<Vec<u8>, String> {
    // Extract ephemeral public key (32 bytes)
    if encrypted_bytes.len() < 32 + 12 + 16 {
        return Err(format!(
            "Encrypted data too short: {} bytes (need at least 60)",
            encrypted_bytes.len()
        ));
    }

    let ephemeral_pubkey_bytes: [u8; 32] = encrypted_bytes[0..32]
        .try_into()
        .map_err(|_| "Failed to extract ephemeral pubkey".to_string())?;
    let ephemeral_pubkey = PublicKey::from(ephemeral_pubkey_bytes);

    // Extract nonce (12 bytes)
    let nonce_bytes: [u8; 12] = encrypted_bytes[32..44]
        .try_into()
        .map_err(|_| "Failed to extract nonce".to_string())?;
    let nonce = Nonce::from(nonce_bytes);

    // Remaining bytes are ciphertext + tag
    let ciphertext = &encrypted_bytes[44..];

    // Perform ECDH to get shared secret
    let shared_secret = x25519_secret.diffie_hellman(&ephemeral_pubkey);

    // WHY/PROOF (security audit VSF-13): the ephemeral key is untrusted input, and a low-order one makes the shared secret all zeros whatever our secret is, so the AES key would be a constant anyone can compute.
    // An honest sender's random ephemeral key is never low order, so this refuses only a forged frame.
    // The raw shared secret stays the AES key: photon's encrypt_for_fgtw and fgtw-bootstrap's decrypt_from_sender speak this exact format on the wire, so a KDF over it needs a versioned format, not an edit here.
    if !shared_secret.was_contributory() {
        return Err("Ephemeral public key is low order: the shared secret is not contributory".to_string());
    }

    // Decrypt using AES-256-GCM
    let cipher = Aes256Gcm::new(shared_secret.as_bytes().into());
    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

#[cfg(test)]
#[cfg(feature = "crypto")]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_roundtrip() {
        use aes_gcm::aead::OsRng;
        use x25519_dalek::EphemeralSecret;

        // Generate a static keypair for the recipient
        let recipient_secret = StaticSecret::random_from_rng(OsRng);
        let recipient_public = PublicKey::from(&recipient_secret);

        // Encrypt some data
        let plaintext = b"Hello, VSF encryption!";

        // Generate ephemeral keypair
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        // Perform ECDH
        let shared_secret = ephemeral_secret.diffie_hellman(&recipient_public);

        // Encrypt
        let cipher = Aes256Gcm::new(shared_secret.as_bytes().into());
        let mut nonce_bytes = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();

        // Build encrypted blob: [ephemeral_pubkey:32][nonce:12][ciphertext+tag]
        let mut encrypted_blob = Vec::new();
        encrypted_blob.extend_from_slice(ephemeral_public.as_bytes());
        encrypted_blob.extend_from_slice(&nonce_bytes);
        encrypted_blob.extend_from_slice(&ciphertext);

        // Decrypt using our function
        let decrypted = decrypt_ve(&encrypted_blob, &recipient_secret).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Security audit VSF-13: a low-order ephemeral key yields the all-zero shared secret, so its AES key is public.
    /// A frame sealed under that zero key must be refused, not decrypted.
    #[test]
    fn test_decrypt_refuses_low_order_ephemeral() {
        use aes_gcm::aead::OsRng;

        let recipient_secret = StaticSecret::random_from_rng(OsRng);
        // The all-zero u-coordinate is a low-order point: every scalar times it is zero.
        let low_order = [0u8; 32];
        assert!(!recipient_secret.diffie_hellman(&PublicKey::from(low_order)).was_contributory());

        // Sealed under the key an attacker can compute without any secret: 32 zero bytes.
        let cipher = Aes256Gcm::new(&[0u8; 32].into());
        let nonce_bytes = [7u8; 12];
        let ciphertext = cipher.encrypt(&Nonce::from(nonce_bytes), b"forged".as_ref()).unwrap();
        let mut blob = Vec::new();
        blob.extend_from_slice(&low_order);
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        let err = decrypt_ve(&blob, &recipient_secret).unwrap_err();
        assert!(err.contains("low order"), "unexpected error: {err}");
    }
}
