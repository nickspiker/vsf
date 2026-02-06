//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files
//! including metadata, structure verification, and field extraction.

use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use vsf::decoding::parse::parse;
use vsf::file_format::VsfHeader;
use vsf::inspect::{
    format_bytes, format_eagle_time, format_number, format_value, format_value_short,
    labels_from_header, parse_section_fields, LabelInfo,
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

    /// Their identity seed (hex) - for Photon per-contact state files
    /// Key = BLAKE3("photon_contact_state_v2" || our_seed || their_seed)
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

    /// Derive Photon identity seed from a handle string
    /// Formula: BLAKE3(VsfType::x(handle).flatten())
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

/// Decrypt Photon contact list using identity seed
/// Key derivation: BLAKE3("photon_contact_list_v2" || seed)
/// Format: [12-byte nonce][ciphertext + 16-byte auth tag]
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

/// Decrypt Photon per-contact state using both identity seeds
/// Key derivation: BLAKE3("photon_contact_state_v2" || our_seed || their_seed)
/// Format: [12-byte nonce][ciphertext + 16-byte auth tag]
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
        return Err(format!("Our seed must be 32 bytes (got {})", our_seed.len()));
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

/// Decrypt using raw symmetric key (ChaCha20-Poly1305)
/// Format: [12-byte nonce][ciphertext + 16-byte auth tag]
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

    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| format!("Key init error: {}", e))?;

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
    use vsf::inspect::format_value_literal;

    // Parse header
    let (header, _consumed) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    // Parse header fields manually to get the literal values as they appear in the file
    let mut ptr = 4; // Skip "RÅ<"

    // Print magic number
    println!("{}", "RÅ<".truecolor(128, 128, 128));

    // Parse and print version
    let version_vsf = parse(data, &mut ptr).map_err(|e| format!("Failed to parse version: {}", e))?;
    print!("{}", format_value_literal(&version_vsf));
    println!(" {}", "version".truecolor(100, 100, 100));

    // Parse and print backward compat
    let backward_compat_vsf = parse(data, &mut ptr).map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    print!("{}", format_value_literal(&backward_compat_vsf));
    println!(" {}", "backward compat".truecolor(100, 100, 100));

    // Parse and print header size
    let header_size_vsf = parse(data, &mut ptr).map_err(|e| format!("Failed to parse header size: {}", e))?;
    print!("{}", format_value_literal(&header_size_vsf));
    println!(" {}", "header size".truecolor(100, 100, 100));

    // Parse and print file length (optional - only if next byte is 'L')
    if ptr < data.len() && data[ptr] == b'L' {
        let file_length_vsf = parse(data, &mut ptr).map_err(|e| format!("Failed to parse file length: {}", e))?;
        print!("{}", format_value_literal(&file_length_vsf));
        println!(" {}", "file length".truecolor(100, 100, 100));
    }

    // Print creation time if present
    if let VsfType::e(_) = header.creation_time {
        print!("{}", format_value_literal(&header.creation_time));
        println!(" {}", "created".truecolor(100, 100, 100));
    }

    // Print provenance hash
    print!("{}", format_value_literal(&header.provenance_hash));
    println!(" {}", "provenance".truecolor(100, 100, 100));

    // Print signer pubkey if present
    if let Some(ref pk) = header.signer_pubkey {
        print!("{}", format_value_literal(pk));
        println!(" {}", "signer pubkey".truecolor(100, 100, 100));
    }

    // Print signature if present
    if let Some(ref sig) = header.signature {
        print!("{}", format_value_literal(sig));
        println!(" {}", "signature".truecolor(100, 100, 100));
    }

    // Print rolling hash if present
    if let Some(ref hb) = header.rolling_hash {
        print!("{}", format_value_literal(hb));
        println!(" {}", "integrity".truecolor(100, 100, 100));
    }

    // Print section TOC
    let label_count_vsf = VsfType::n(labels.len());
    print!("{}", format_value_literal(&label_count_vsf));
    println!("{}", "(".truecolor(80, 80, 80));

    // Print each section TOC entry
    for (i, label) in labels.iter().enumerate() {
        let is_last = i == labels.len() - 1;

        // Section name
        let name_vsf = VsfType::d(label.name.clone());
        print!("  {}", format_value_literal(&name_vsf));
        print!("{}", ":".truecolor(80, 80, 80));

        // Offset
        let offset_vsf = VsfType::o(label.offset);
        print!("{}", format_value_literal(&offset_vsf));
        print!("{}", ",".truecolor(80, 80, 80));

        // Size
        let size_vsf = VsfType::b(label.size, false);
        print!("{}", format_value_literal(&size_vsf));
        print!("{}", ",".truecolor(80, 80, 80));

        // Field count
        let count_vsf = VsfType::n(label.child_count);
        print!("{}", format_value_literal(&count_vsf));

        if !is_last {
            println!("{}", ",".truecolor(80, 80, 80));
        } else {
            println!();
        }
    }

    println!("{}", ")>".truecolor(80, 80, 80));

    // Print section bodies
    let nonempty_labels: Vec<_> = labels.iter().filter(|l| l.size > 0).collect();

    for (i, label) in nonempty_labels.iter().enumerate() {
        println!("{}", "[".truecolor(80, 80, 80));

        // Parse section fields
        match parse_section_fields(data, label) {
            Ok(fields) => {
                for (j, field) in fields.iter().enumerate() {
                    let is_last_field = j == fields.len() - 1;

                    // Field name
                    let field_name_vsf = VsfType::d(field.name.clone());
                    print!("  {}", "(".truecolor(80, 80, 80));
                    print!("{}", format_value_literal(&field_name_vsf));
                    println!("{}",":".truecolor(80, 80, 80));

                    // Field values
                    for (k, val) in field.values.iter().enumerate() {
                        let is_last_value = k == field.values.len() - 1;
                        print!("    {}", format_value_literal(val));

                        if !is_last_value {
                            println!("{}", ",".truecolor(80, 80, 80));
                        } else {
                            println!();
                        }
                    }

                    print!("  {}", ")".truecolor(80, 80, 80));
                    if !is_last_field {
                        println!("{}", ",".truecolor(80, 80, 80));
                    } else {
                        println!();
                    }
                }
            }
            Err(e) => {
                println!("  {} {}", "<parse error:".truecolor(220, 100, 100), format!("{}>", e).truecolor(220, 100, 100));
            }
        }

        println!("{}", "]".truecolor(80, 80, 80));

        // Add newline between sections
        if i < nonempty_labels.len() - 1 {
            println!();
        }
    }

    Ok(())
}

/// Quick integrity summary (used by show_info)
fn verify_integrity_summary(
    data: &[u8],
    header: &VsfHeader,
    labels: &[LabelInfo],
) -> Result<bool, String> {
    let mut all_checks_pass = true;

    // Display provenance hash (hp)
    if let VsfType::hp(ref stored_hash) = header.provenance_hash {
        println!(
            " {}-Byte {} {}:",
            stored_hash.len().to_string().white(),
            "BLAKE3".green(),
            "provenance hash".cyan()
        );
        print!(" {} ", "0x".truecolor(64, 50, 255));
        for byte in stored_hash.iter() {
            print!("{:02X}", byte);
        }
        println!();
    }

    // Display signer pubkey (ke) if present
    if let Some(VsfType::ke(ref pk_bytes)) = header.signer_pubkey {
        println!(
            " {}-Byte {} {}:",
            pk_bytes.len().to_string().white(),
            "Ed25519".green(),
            "signer pubkey".cyan()
        );
        print!(" {} ", "0x".truecolor(64, 50, 255));
        for byte in pk_bytes.iter() {
            print!("{:02X}", byte);
        }
        println!();
    }

    // Display optional signature (ge)
    if let Some(VsfType::ge(ref sig_bytes)) = header.signature {
        println!(
            " {}-Byte {} {}:",
            sig_bytes.len().to_string().white(),
            "Ed25519".green(),
            "signature".cyan()
        );
        print!(" {} ", "0x".truecolor(64, 50, 255));
        for byte in sig_bytes.iter().take(32) {
            print!("{:02X}", byte);
        }
        if sig_bytes.len() > 32 {
            print!("...");
        }
        println!();
        println!(
            " {} {}",
            "Semantics:".cyan(),
            "Protocol-specific (signed data unknown)".truecolor(200, 200, 200)
        );
    }

    // Display and verify rolling hash (hb)
    let (file_hash_verified, stored_hash, computed_hash) =
        if let Some(VsfType::hb(ref stored_hash)) = header.rolling_hash {
            let computed =
                vsf::verification::compute_file_hash(data).unwrap_or_else(|_| [0u8; 32]);
            let verified = computed.as_slice() == stored_hash.as_slice();
            (verified, Some(stored_hash.clone()), Some(computed.to_vec()))
        } else {
            (false, None, None)
        };

    // Check section-level hashes
    let mut verified_sections = 0;
    let mut total_sections = 0;

    for label in labels {
        if label.child_count > 0 {
            total_sections += 1;
            if let Some(ref hash_vsf) = label.hash {
                let hash_bytes = match hash_vsf {
                    VsfType::hp(ref bytes) | VsfType::hb(ref bytes) | VsfType::hs(ref bytes) => {
                        bytes
                    }
                    _ => continue,
                };

                let section_end = label.offset + label.size;
                if section_end <= data.len() {
                    let section_data = &data[label.offset..section_end];
                    let computed = blake3::hash(section_data);
                    if computed.as_bytes() == hash_bytes.as_slice() {
                        verified_sections += 1;
                    }
                }
            }
        }
    }
    let _ = (verified_sections, total_sections); // Suppress unused warnings

    // Display rolling hash (hb) if present
    if stored_hash.is_some() {
        println!(
            " {}-Byte {} {}:",
            32.to_string().white(),
            "BLAKE3".green(),
            "rolling hash".cyan()
        );
    }

    if file_hash_verified {
        if let Some(hash) = stored_hash {
            print!(" {} ", "0x".truecolor(64, 50, 255));
            for byte in hash.iter() {
                print!("{:02X}", byte);
            }
            println!();
        }
        print!(" {} ", "Verification:".cyan());
        println!("{}", "PASS".truecolor(0, 255, 0));
    } else if stored_hash.is_some() {
        all_checks_pass = false;
        if let (Some(expected), Some(computed)) = (stored_hash, computed_hash) {
            print!(" {} {} ", "Expected:".cyan(), "0x".truecolor(64, 50, 255));
            for byte in expected.iter() {
                print!("{:02X}", byte);
            }
            println!();
            print!(" {} {} ", "Got:".cyan(), "     0x".truecolor(64, 50, 255));
            for byte in computed.iter() {
                print!("{:02X}", byte);
            }
            println!();
        }
        print!(" {} ", "Verification:".cyan());
        println!("{}", "FAIL".truecolor(255, 0, 0));
    }

    Ok(all_checks_pass)
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
                let field_marker = if is_field_last { "└── " } else { "├── " };

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

/// Derive Photon identity seed from a handle string
/// Formula: BLAKE3(VsfType::x(handle).flatten())
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
