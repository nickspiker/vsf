//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files including metadata, structure verification, and field extraction.

use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use vsf::file_format::VsfHeader;
use vsf::inspect::{
    format_value, format_value_short, labels_from_header, parse_section_fields,
};
use vsf::types::VsfType;

#[derive(Parser)]
#[command(name = "vsfinfo")]
#[command(about = "VSF File Inspector - Inspect, verify, and extract VSF file contents", long_about = None)]
#[command(version)]
struct Cli {
    /// VSF file to inspect
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Show detailed encoding information
    #[arg(short, long)]
    detailed: bool,

    /// Path to VSF keypair file for decrypting encrypted sections
    #[arg(short, long)]
    key: Option<PathBuf>,

    /// Symmetric key (hex) for ChaCha20-Poly1305 encrypted files
    #[arg(long, value_name = "HEX")]
    symmetric_key: Option<String>,

    /// Identity seed (hex) - derives symmetric key for Photon contact list
    #[arg(long, value_name = "HEX")]
    identity_seed: Option<String>,

    /// Their identity seed (hex) - for Photon per-contact state files Key = BLAKE3("photon_contact_state_v2" || our_seed || their_seed)
    #[arg(long, value_name = "HEX")]
    their_seed: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show detailed file information (default)
    Info,

    /// Verify file integrity and signatures
    Verify,

    /// Extract a specific field value
    #[command(name = "get")]
    Extract {
        /// Field path in format "section.field"
        #[arg(value_name = "FIELD_PATH")]
        field_path: String,
    },

    /// Show file structure as a tree
    Tree,

    /// Derive Photon identity seed from a handle string Formula: BLAKE3(VsfType::x(handle).flatten())
    #[command(name = "seed")]
    DeriveSeed {
        /// Handle string to derive seed from
        #[arg(value_name = "HANDLE")]
        handle: String,
    },
}

fn main() {
    let cli = Cli::parse();

    // Handle seed command separately (doesn't need a file)
    if let Some(Commands::DeriveSeed { handle }) = cli.command {
        if let Err(e) = derive_seed(&handle) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // If no file provided, print help and exit
    let file = match cli.file {
        Some(f) => f,
        None => {
            use clap::CommandFactory;
            Cli::command().print_help().unwrap();
            println!();
            std::process::exit(0);
        }
    };

    // Read the file
    let raw_data = match fs::read(&file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Decrypt if symmetric key provided
    let data = if let (Some(ref our_seed_hex), Some(ref their_seed_hex)) =
        (&cli.identity_seed, &cli.their_seed)
    {
        // Derive key for per-contact state: BLAKE3("photon_contact_state_v2" || our || their)
        match decrypt_photon_contact_state(&raw_data, our_seed_hex, their_seed_hex) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Decryption error: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(ref seed_hex) = cli.identity_seed {
        // Derive key from identity seed (Photon contact list format)
        match decrypt_photon_contacts(&raw_data, seed_hex) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Decryption error: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(ref key_hex) = cli.symmetric_key {
        // Use raw symmetric key
        match decrypt_symmetric(&raw_data, key_hex) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Decryption error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        raw_data
    };

    // Execute the appropriate command
    let result = match cli.command {
        Some(Commands::Info) | None => show_info(&data, cli.detailed, cli.key.as_deref()),
        Some(Commands::Verify) => verify_file(&data),
        Some(Commands::Extract { field_path }) => extract_field(&data, &field_path),
        Some(Commands::Tree) => show_tree(&data),
        Some(Commands::DeriveSeed { handle }) => derive_seed(&handle),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Decrypt Photon contact list using identity seed Key derivation: BLAKE3("photon_contact_list_v2" || seed) Format: [12-byte nonce][ciphertext + 16-byte auth tag]
#[cfg(feature = "crypto")]
fn decrypt_photon_contacts(encrypted: &[u8], seed_hex: &str) -> Result<Vec<u8>, String> {
    use blake3::Hasher;

    let seed = hex::decode(seed_hex).map_err(|e| format!("Invalid hex: {}", e))?;
    if seed.len() != 32 {
        return Err(format!("Seed must be 32 bytes (got {})", seed.len()));
    }

    // Derive key using same domain separation as Photon
    let mut hasher = Hasher::new();
    hasher.update(b"photon_contact_list_v2");
    hasher.update(&seed);
    let key = hasher.finalize();

    decrypt_chacha20poly1305(encrypted, key.as_bytes())
}

#[cfg(not(feature = "crypto"))]
fn decrypt_photon_contacts(_encrypted: &[u8], _seed_hex: &str) -> Result<Vec<u8>, String> {
    Err("Crypto feature not enabled - rebuild with --features crypto".to_string())
}

/// Decrypt Photon per-contact state using both identity seeds Key derivation: BLAKE3("photon_contact_state_v2" || our_seed || their_seed) Format: [12-byte nonce][ciphertext + 16-byte auth tag]
#[cfg(feature = "crypto")]
fn decrypt_photon_contact_state(
    encrypted: &[u8],
    our_seed_hex: &str,
    their_seed_hex: &str,
) -> Result<Vec<u8>, String> {
    use blake3::Hasher;

    let our_seed = hex::decode(our_seed_hex).map_err(|e| format!("Invalid our_seed hex: {}", e))?;
    let their_seed =
        hex::decode(their_seed_hex).map_err(|e| format!("Invalid their_seed hex: {}", e))?;

    if our_seed.len() != 32 {
        return Err(format!(
            "Our seed must be 32 bytes (got {})",
            our_seed.len()
        ));
    }
    if their_seed.len() != 32 {
        return Err(format!(
            "Their seed must be 32 bytes (got {})",
            their_seed.len()
        ));
    }

    // Derive key using same domain separation as Photon
    let mut hasher = Hasher::new();
    hasher.update(b"photon_contact_state_v2");
    hasher.update(&our_seed);
    hasher.update(&their_seed);
    let key = hasher.finalize();

    decrypt_chacha20poly1305(encrypted, key.as_bytes())
}

#[cfg(not(feature = "crypto"))]
fn decrypt_photon_contact_state(
    _encrypted: &[u8],
    _our_seed_hex: &str,
    _their_seed_hex: &str,
) -> Result<Vec<u8>, String> {
    Err("Crypto feature not enabled - rebuild with --features crypto".to_string())
}

/// Decrypt using raw symmetric key (ChaCha20-Poly1305) Format: [12-byte nonce][ciphertext + 16-byte auth tag]
#[cfg(feature = "crypto")]
fn decrypt_symmetric(encrypted: &[u8], key_hex: &str) -> Result<Vec<u8>, String> {
    let key = hex::decode(key_hex).map_err(|e| format!("Invalid hex: {}", e))?;
    if key.len() != 32 {
        return Err(format!("Key must be 32 bytes (got {})", key.len()));
    }

    let key_array: [u8; 32] = key.try_into().unwrap();
    decrypt_chacha20poly1305(encrypted, &key_array)
}

#[cfg(not(feature = "crypto"))]
fn decrypt_symmetric(_encrypted: &[u8], _key_hex: &str) -> Result<Vec<u8>, String> {
    Err("Crypto feature not enabled - rebuild with --features crypto".to_string())
}

/// ChaCha20-Poly1305 decryption helper
#[cfg(feature = "crypto")]
fn decrypt_chacha20poly1305(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};

    if encrypted.len() < 12 + 16 {
        return Err("File too short for ChaCha20-Poly1305 (need at least 28 bytes)".to_string());
    }

    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("Key init error: {}", e))?;

    let nonce_bytes: [u8; 12] = encrypted[..12]
        .try_into()
        .map_err(|_| "Invalid nonce length")?;
    let nonce: Nonce = nonce_bytes.into();
    let ciphertext = &encrypted[12..];

    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {} (wrong key?)", e))
}

/// Show basic file information in literal VSF format
fn show_info(data: &[u8], _detailed: bool, _key_path: Option<&Path>) -> Result<(), String> {
    // Use the unified inspect_vsf() function from the library This ensures consistent output between CLI and browser/WASM
    let formatted = vsf::inspect::inspect_vsf(data)?;
    println!("{}", formatted);

    // The inspect view SHOWS the crypto fields but only re-computes the rolling hash.
    // A file inspector must actually VERIFY every anchor it displays, so run the library verify functions and report an unambiguous per-anchor result.
    print_verification_summary(data);

    Ok(())
}

/// Verify every integrity/authenticity anchor the header carries and print a per-anchor PASS/FAIL/absent line.
///
/// This is the acceptance gate: parsing a document is not the same as trusting it.
/// We report three anchors independently:
/// - provenance (hp): content matches its claimed immutable identity ([`is_original`]).
/// - integrity (hb): the rolling hash covers the whole file ([`verify_file_hash`]); absent when the file is signed instead.
/// - signature (ge): the header-level Ed25519 signature ([`verify_file_signature`]); absent when the file is unsigned, and reported as needing the crypto feature when present but this build lacks it.
///
/// The final line mirrors [`vsf::verification::read_verified`]'s doctrine: a document that carries neither a rolling hash nor a valid signature is UNVERIFIABLE, not merely "hp-clean".
fn print_verification_summary(data: &[u8]) {
    // Decode the header once to know which anchors are even present.
    let header = match VsfHeader::decode(data) {
        Ok((h, _)) => h,
        Err(e) => {
            println!("\n {} {}", "verification:".cyan(), format!("header parse failed: {e}").truecolor(255, 0, 0));
            return;
        }
    };

    println!("\n {}", "Verification".cyan().bold());

    // provenance (hp) — always present in a well-formed VSF file.
    let provenance_ok = vsf::verification::is_original(data).is_ok();
    print_anchor("provenance (hp)", Some(provenance_ok));

    // integrity (hb) — the rolling hash. Absent on signed files (signature replaces it).
    let integrity = if header.rolling_hash.is_some() {
        Some(vsf::verification::verify_file_hash(data).is_ok())
    } else {
        None
    };
    print_anchor("integrity  (hb)", integrity);

    // signature (ge) — header-level Ed25519. Present iff both ke and ge are in the header.
    let is_signed = header.signature.is_some() && header.signer_pubkey.is_some();
    if is_signed {
        #[cfg(feature = "crypto")]
        {
            let sig_ok = vsf::verification::verify_file_signature(data).unwrap_or(false);
            print_anchor("signature  (ge)", Some(sig_ok));
        }
        #[cfg(not(feature = "crypto"))]
        {
            println!(
                " {} {}",
                "signature  (ge):".cyan(),
                "present (rebuild with --features crypto to verify)".yellow()
            );
        }
    } else {
        print_anchor("signature  (ge)", None);
    }

    // Overall verdict, matching read_verified's un-skippable semantics.
    let has_semantic_anchor = integrity == Some(true) || {
        #[cfg(feature = "crypto")]
        {
            is_signed && vsf::verification::verify_file_signature(data).unwrap_or(false)
        }
        #[cfg(not(feature = "crypto"))]
        {
            // Without crypto we cannot confirm a signature; treat signed-but-unverifiable conservatively.
            false
        }
    };

    print!(" {} ", "verdict:".cyan());
    if !provenance_ok {
        println!("{}", "TAMPERED (provenance mismatch)".truecolor(255, 0, 0));
    } else if has_semantic_anchor {
        println!("{}", "VERIFIED".truecolor(0, 255, 0));
    } else if is_signed {
        // Signed doc but this build can't check the signature.
        println!("{}", "UNCHECKED (signed; crypto feature off)".yellow());
    } else {
        println!(
            "{}",
            "UNVERIFIABLE (no rolling hash, no signature)".truecolor(255, 0, 0)
        );
    }
}

/// Print one anchor line: `Some(true)` → PASS, `Some(false)` → FAIL, `None` → absent.
fn print_anchor(label: &str, result: Option<bool>) {
    let status = match result {
        Some(true) => "PASS".truecolor(0, 255, 0),
        Some(false) => "FAIL".truecolor(255, 0, 0),
        None => "absent".truecolor(160, 160, 160),
    };
    println!(" {} {}", format!("{label}:").cyan(), status);
}

/// Verify file integrity
fn verify_file(data: &[u8]) -> Result<(), String> {
    println!("Verifying VSF file...\n");

    let mut errors = 0;
    let mut warnings = 0;

    // Check magic number
    if data.len() < 4 || &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        println!("✗ Invalid magic number");
        errors += 1;
    } else {
        println!("✓ Magic number valid");
    }

    // Parse header
    let (header, _) = match VsfHeader::decode(data) {
        Ok(h) => {
            println!("✓ Header structure valid");
            h
        }
        Err(e) => {
            println!("✗ Header parsing failed: {}", e);
            errors += 1;
            return Err("Cannot continue verification".into());
        }
    };
    let labels = labels_from_header(&header);

    // Verify each section
    for label in &labels {
        // Check section hash (now in label, not preamble)
        if let Some(ref hash_vsf) = label.hash {
            let hash_bytes = match hash_vsf {
                VsfType::hb(ref bytes) | VsfType::hs(ref bytes) => bytes,
                _ => continue,
            };

            let section_end = label.offset + label.size;
            if section_end <= data.len() {
                let section_data = &data[label.offset..section_end];
                let computed = blake3::hash(section_data);
                if computed.as_bytes() == hash_bytes.as_slice() {
                    println!("✓ Section '{}': hash verified", label.name);
                } else {
                    println!("✗ Section '{}': hash mismatch!", label.name);
                    errors += 1;
                }
            } else {
                println!("✗ Section '{}': section exceeds file size", label.name);
                errors += 1;
            }
        }

        // Check signature presence
        if label.signature.is_some() {
            println!(
                "✓ Section '{}': signature present (verification TBD)",
                label.name
            );
            warnings += 1;
        }
    }

    // Look for TOKEN signature
    if labels
        .iter()
        .any(|l| l.name == "token_auth" || l.name == "token auth")
    {
        println!("\n✓ Found TOKEN auth section");
        println!("  (Full signature verification TBD)");
        warnings += 1;
    } else {
        println!("\n○ No TOKEN auth section found");
    }

    println!("\n{}", "=".repeat(50));
    if errors == 0 && warnings == 0 {
        println!("✓ ALL CHECKS PASSED");
    } else if errors == 0 {
        println!("✓ VALID ({} warnings)", warnings);
    } else {
        println!("✗ INVALID ({} errors, {} warnings)", errors, warnings);
    }

    Ok(())
}

/// Extract a specific field value
fn extract_field(data: &[u8], field_path: &str) -> Result<(), String> {
    let parts: Vec<&str> = field_path.split('.').collect();

    if parts.len() != 2 {
        return Err("Field path must be 'section.field'".into());
    }

    let section_name = parts[0];
    let field_name = parts[1];

    let (header, _) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    // Find section (handle both space and underscore variants)
    let section = labels
        .iter()
        .find(|l| {
            l.name == section_name
                || l.name.replace(' ', "_") == section_name
                || l.name.replace('_', " ") == section_name
        })
        .ok_or(format!("Section '{}' not found", section_name))?;

    // Parse section fields
    let fields = parse_section_fields(data, section)?;

    // Find the requested field (handle both space and underscore variants)
    for field in fields {
        if field.name == field_name
            || field.name.replace(' ', "_") == field_name
            || field.name.replace('_', " ") == field_name
        {
            // Format multi-value fields as comma-separated
            let values_str: Vec<String> = field.values.iter().map(|v| format_value(v)).collect();
            println!("{}", values_str.join(", "));
            return Ok(());
        }
    }

    Err(format!(
        "Field '{}' not found in section '{}'",
        field_name, section_name
    ))
}

/// Show file structure as a tree
fn show_tree(data: &[u8]) -> Result<(), String> {
    let (header, _) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    println!("VSF File Tree");
    println!("{}", "=".repeat(50));
    println!();

    for (i, label) in labels.iter().enumerate() {
        let is_last = i == labels.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };

        println!(
            "{}{} ({} Bytes, {} fields)",
            prefix, label.name, label.size, label.child_count
        );

        // Parse fields
        if let Ok(fields) = parse_section_fields(data, label) {
            for (j, field) in fields.iter().enumerate() {
                let is_field_last = j == fields.len() - 1;
                let field_prefix = if is_last { "    " } else { "│   " };
                let field_marker = if is_field_last {
                    "└── "
                } else {
                    "├── "
                };

                // Format multi-value fields: newline before each opcode
                println!("{}{}{}: ", field_prefix, field_marker, field.name);

                let continuation_prefix = if is_field_last { "   " } else { "│  " };
                let mut line_buffer = String::new();

                for val_vsf in field.values.iter() {
                    let val_str = format_value_short(val_vsf);
                    let is_opcode = matches!(val_vsf, VsfType::op(_, _));

                    // If we hit an opcode and have buffered content, flush the line
                    if is_opcode && !line_buffer.is_empty() {
                        println!("{}{}", field_prefix, line_buffer);
                        line_buffer.clear();
                    }

                    // Add spacing and append value
                    if line_buffer.is_empty() {
                        line_buffer.push_str(&format!("{}   ", continuation_prefix));
                    }
                    line_buffer.push_str(&val_str);
                }

                // Flush remaining buffer
                if !line_buffer.is_empty() {
                    println!("{}{}", field_prefix, line_buffer);
                }
            }
        }

        if !is_last {
            println!("│");
        }
    }

    Ok(())
}

/// Derive Photon identity seed from a handle string Formula: BLAKE3(VsfType::x(handle).flatten())
fn derive_seed(handle: &str) -> Result<(), String> {
    use blake3::Hasher;

    let vsf_bytes = VsfType::x(handle.to_string()).flatten();
    let hash = blake3::hash(&vsf_bytes);
    let seed = hash.as_bytes();

    // Derive the contact list encryption key
    let mut hasher = Hasher::new();
    hasher.update(b"photon_contact_list_v2");
    hasher.update(seed);
    let list_key = hasher.finalize();

    println!("{}", "Photon Identity Seed Derivation".cyan().bold());
    println!();
    println!(" {} {}", "Handle:".cyan(), handle.white());
    println!(
        " {} {} Bytes",
        "VSF encoding:".cyan(),
        vsf_bytes.len().to_string().white()
    );
    println!();
    println!(
        " {} {}",
        "Identity seed:".cyan(),
        hex::encode(seed).to_uppercase().yellow()
    );
    println!(
        " {} {}",
        "Directory:".cyan(),
        hex::encode(&seed[..8]).white()
    );
    println!();
    println!(
        " {} {}",
        "Contact list key:".cyan(),
        hex::encode(list_key.as_bytes()).to_uppercase().green()
    );
    println!();
    println!("{}", "Usage:".cyan().bold());
    println!(
        "  vsfinfo --symmetric-key {} index.enc",
        hex::encode(list_key.as_bytes()).to_uppercase()
    );

    Ok(())
}
