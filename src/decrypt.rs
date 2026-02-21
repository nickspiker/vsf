//! VSF Decryption Support
//!
//! Provides functions for decrypting VSF v'e' (ephemeral X25519 + AES-256-GCM) encrypted content.

#[cfg(feature = "crypto")]
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
#[cfg(feature = "crypto")]
use std::fs;
#[cfg(feature = "crypto")]
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
    /// Load keypair from VSF file (expects fgtw_device_key or similar format)
    /// Format: [d"fgtw_device_key" (d"secret":ke{32}{bytes}) (d"public":ke{32}{bytes})]
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
        use std::collections::HashMap;
        let mut fields: HashMap<String, VsfType> = HashMap::new();

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
                    fields.insert(field_name, value);
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
        let secret_bytes = match fields.get("secret") {
            Some(VsfType::ke(bytes)) => {
                if bytes.len() != 32 {
                    return Err("Secret key must be 32 bytes".to_string());
                }
                bytes.clone()
            }
            _ => return Err("Secret key not found".to_string()),
        };

        let public_bytes = match fields.get("public") {
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

        // Derive X25519 keys from Ed25519
        // Ed25519 secret key is 32 bytes, X25519 needs 32 bytes
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

/// Decrypt v'e' encrypted data (ephemeral X25519 + AES-256-GCM)
/// Format: [ephemeral_pubkey:32][nonce:12][ciphertext+tag]
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
}
