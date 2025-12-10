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

/// Show basic file information (default mode)
fn show_info(data: &[u8], detailed: bool, key_path: Option<&Path>) -> Result<(), String> {
    // Load keypair for decryption if provided
    #[cfg(feature = "crypto")]
    let keypair = if let Some(path) = key_path {
        Some(vsf::decrypt::Keypair::load_from_vsf(path)?)
    } else {
        None
    };
    #[cfg(not(feature = "crypto"))]
    let _ = key_path;

    let (header, _consumed) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    // Calculate actual header length by parsing
    let mut pointer = 4; // After "RÅ<"

    // Parse version and backward_compat first (VSF v4+ format)
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _ =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Now parse header length
    let header_length_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bytes_encoded = match header_length_type {
        VsfType::b(bytes, _) => bytes,
        _ => return Err("Expected b type for header length".to_string()),
    };

    // Title
    println!("{}", "VSF File".cyan().bold());
    println!(
        "{} ({} Bytes)",
        format_bytes(data.len()).yellow(),
        format_number(data.len()).truecolor(64, 50, 255)
    );
    println!();

    // Header section
    if detailed {
        // Detailed mode - show raw encoding
        println!(
            "{:<25} {}",
            "RÅ".truecolor(128, 128, 128),
            "Magic".truecolor(128, 128, 128)
        );
        println!(
            "{:<25} {}",
            "<".truecolor(128, 128, 128),
            "Header start".truecolor(128, 128, 128)
        );
        println!(
            " {:<24} {} {}",
            format!("z3 {:02x}", header.version).truecolor(128, 128, 128),
            "Version".cyan(),
            header.version.to_string().white()
        );
        println!(
            " {:<24} {} {}",
            format!("y3 {:02x}", header.backward_compat).truecolor(128, 128, 128),
            "Backward compat".cyan(),
            header.backward_compat.to_string().white()
        );

        // Display creation time with encoding
        if let VsfType::e(ref et) = header.creation_time {
            let timestamp = match et {
                vsf::types::EtType::f5(v) => *v as f64,
                vsf::types::EtType::f6(v) => *v,
                _ => 0.0,
            };
            println!(
                " {:<24} {} {}",
                format!("ef6 {:.10}", timestamp).truecolor(128, 128, 128),
                "Created".cyan(),
                format_eagle_time(et).white()
            );
        }

        // Display provenance hash with encoding
        if let VsfType::hp(ref hash_bytes) = header.provenance_hash {
            let preview = if hash_bytes.len() >= 8 {
                format!("{}...", hex::encode(&hash_bytes[..8]).to_uppercase())
            } else {
                hex::encode(hash_bytes).to_uppercase()
            };
            println!(
                " {:<24} {} 0x{}",
                format!("hp3 1f [{} Bytes]", hash_bytes.len()).truecolor(128, 128, 128),
                "Provenance hash:".cyan(),
                preview.white()
            );
        }

        // Display signer pubkey (ke) with encoding
        if let Some(VsfType::ke(ref pk_bytes)) = header.signer_pubkey {
            let preview = if pk_bytes.len() >= 8 {
                format!("{}...", hex::encode(&pk_bytes[..8]).to_uppercase())
            } else {
                hex::encode(pk_bytes).to_uppercase()
            };
            println!(
                " {:<24} {} 0x{}",
                format!("ke3 1f [{} Bytes]", pk_bytes.len()).truecolor(128, 128, 128),
                "Signer pubkey:".cyan(),
                preview.white()
            );
        }

        // Display Ed25519 signature with encoding
        if let Some(VsfType::ge(ref sig_bytes)) = header.signature {
            let preview = if sig_bytes.len() >= 8 {
                format!("{}...", hex::encode(&sig_bytes[..8]).to_uppercase())
            } else {
                hex::encode(sig_bytes).to_uppercase()
            };
            println!(
                " {:<24} {} 0x{}",
                format!("ge3 3f [{} Bytes]", sig_bytes.len()).truecolor(128, 128, 128),
                "Ed25519 signature:".cyan(),
                preview.white()
            );
        }

        // Display rolling hash with encoding
        if let Some(VsfType::hb(ref hash_bytes)) = header.rolling_hash {
            let preview = if hash_bytes.len() >= 8 {
                format!("{}...", hex::encode(&hash_bytes[..8]).to_uppercase())
            } else {
                hex::encode(hash_bytes).to_uppercase()
            };
            println!(
                " {:<24} {} 0x{}",
                format!("hb3 1f [{} Bytes]", hash_bytes.len()).truecolor(128, 128, 128),
                "Rolling hash:".cyan(),
                preview.white()
            );
        }

        // Display label count with encoding
        let label_count = labels.len();
        let encoding = if label_count <= 63 {
            format!("n3 {:02x}", label_count)
        } else if label_count <= 255 {
            format!("n4 {:02x}", label_count)
        } else {
            format!("n [{}]", label_count)
        };
        println!(
            " {:<24} {} {}",
            encoding.truecolor(128, 128, 128),
            "Labels:".cyan(),
            label_count.to_string().white()
        );
    } else {
        // Normal mode - just descriptions
        println!("{}", "<".truecolor(128, 128, 128));
        println!(
            " {} {}",
            "Version".cyan(),
            header.version.to_string().white()
        );
        println!(
            " {} {}",
            "Backward compat".cyan(),
            header.backward_compat.to_string().white()
        );

        // Display creation time if present
        if let VsfType::e(ref et) = header.creation_time {
            println!(" {} {}", "Created".cyan(), format_eagle_time(et).white());
        }
    }

    println!(
        " {} {} Bytes",
        "Header size:".cyan(),
        header_length_bytes_encoded.to_string().white()
    );

    // Integrity check (includes hash display)
    let integrity_ok = verify_integrity_summary(data, &header, &labels)?;

    println!();

    // Labels section
    println!(" {} labels", labels.len().to_string().yellow().bold());

    // Calculate max widths
    let max_size_len = labels
        .iter()
        .map(|l| format_bytes(l.size).len())
        .max()
        .unwrap_or(0);
    let max_name_len = labels.iter().map(|l| l.name.len()).max().unwrap_or(0);
    let max_offset_str_len = labels
        .iter()
        .map(|l| format_number(l.offset).len())
        .max()
        .unwrap_or(0);

    for label in &labels {
        let size_str = format_bytes(label.size);
        let offset_str = format_number(label.offset);

        // Build crypto suffix
        let mut crypto_parts = Vec::new();
        if let Some(ref sig) = label.signature {
            match sig {
                VsfType::ge(_) => crypto_parts.push("Signed with Ed25519".to_string()),
                VsfType::gp(_) => crypto_parts.push("Signed with ECDSA-P256".to_string()),
                VsfType::gr(_) => crypto_parts.push("Signed with RSA".to_string()),
                _ => {}
            }
        }
        if let Some(ref _w) = label.wrap {
            if let Some(ref key) = label.key {
                match key {
                    VsfType::kc(_) => {
                        crypto_parts.push("Encrypted with ChaCha20-Poly1305".to_string())
                    }
                    VsfType::ka(_) => crypto_parts.push("Encrypted with AES-256-GCM".to_string()),
                    _ => {}
                }
            }
        }
        let crypto_str = if crypto_parts.is_empty() {
            String::new()
        } else {
            crypto_parts.join(", ")
        };

        // Field count string - empty sections (size=0) are intentionally empty
        let field_str = if label.size == 0 {
            String::new() // Empty section - no field info needed
        } else if label.child_count == 0 {
            "with unknown".to_string()
        } else if label.child_count == 1 {
            "with 1 field".to_string()
        } else {
            format!("with {} fields", label.child_count)
        };

        // Print with alignment
        print!(" {}", "(".truecolor(128, 128, 128));
        if label.size == 0 {
            // Empty section or inline field - show name
            print!("{}", label.name.white().bold());
            // Show inline values if present: (name:val1,val2,...)
            if !label.inline_values.is_empty() {
                print!("{}", ":".truecolor(128, 128, 128));
                for (i, val) in label.inline_values.iter().enumerate() {
                    if i > 0 {
                        print!("{}", ",".truecolor(128, 128, 128));
                    }
                    print!("{}", format_value_short(val));
                }
            }
        } else {
            // Regular section with size/offset info
            print!("{:>width$}", size_str.bright_yellow(), width = max_size_len);
            print!("      ");
            print!(
                "{:<width$}",
                label.name.white().bold(),
                width = max_name_len
            );
            print!("    @");
            print!(
                "{:>width$}",
                offset_str.truecolor(64, 50, 255),
                width = max_offset_str_len
            );
            print!("   ");
            print!("{:<15}", field_str.cyan());
            print!(" ");
            print!("{:<33}", crypto_str.magenta());
        }
        println!("{}", ")".truecolor(128, 128, 128));
    }

    // Check if there are any non-empty sections to display
    let has_nonempty_sections = labels.iter().any(|l| l.size > 0);

    if has_nonempty_sections {
        print!("{}", ">".truecolor(128, 128, 128));
        println!("{}", "┐".white());
    } else {
        // All sections are empty - just show the closing bracket
        println!("{}", ">".truecolor(128, 128, 128));
    }

    // Track validation errors
    let mut has_errors = false;

    // Show sections with their actual structure (skip empty sections)
    let nonempty_labels: Vec<_> = labels.iter().filter(|l| l.size > 0).collect();
    for (i, label) in nonempty_labels.iter().enumerate() {
        let is_last = i == nonempty_labels.len() - 1;
        let connector = if is_last { " └─" } else { " ├─" };

        // Show section (crypto is in header label now, not preamble)
        println!(
            "{}{}{}",
            connector,
            "[".truecolor(128, 128, 128),
            label.name.bold()
        );

        // Parse and show fields (skip for n[0] unboxed blobs)
        if label.child_count == 0 {
            let field_prefix = if is_last { "   " } else { " │ " };

            // Check if this is an empty section vs opaque blob
            let section_start = label.offset;
            let section_end = section_start + label.size;
            let is_empty_section = if section_end <= data.len() {
                let section_data = &data[section_start..section_end];
                let mut ptr = 0;
                if ptr < section_data.len() && section_data[ptr] == b'[' {
                    ptr += 1;
                    if parse(section_data, &mut ptr).is_ok() {
                        ptr < section_data.len() && section_data[ptr] == b']'
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if is_empty_section {
                // Empty section - don't display anything
            } else {
                // Try to decrypt if we have a key and this is encrypted
                #[cfg(feature = "crypto")]
                let decrypted = if let Some(ref kp) = keypair {
                    if section_end <= data.len() {
                        let section_data = &data[section_start..section_end];
                        let mut ptr = 0;
                        if ptr < section_data.len() && section_data[ptr] == b'[' {
                            ptr += 1;
                            if parse(section_data, &mut ptr).is_ok() {
                                if let Ok(VsfType::v(b'e', encrypted_bytes)) =
                                    parse(section_data, &mut ptr)
                                {
                                    match vsf::decrypt::decrypt_ve(&encrypted_bytes, &kp.x25519_secret)
                                    {
                                        Ok(plaintext) => {
                                            println!(
                                                "{}  {} Decrypted content:",
                                                field_prefix,
                                                "🔓".green()
                                            );
                                            let mut ptr = 0;
                                            while ptr < plaintext.len() {
                                                match parse(&plaintext, &mut ptr) {
                                                    Ok(val) => {
                                                        println!(
                                                            "{}    {}",
                                                            field_prefix,
                                                            format_value_short(&val)
                                                        );
                                                    }
                                                    Err(e) => {
                                                        println!(
                                                            "{}    <parse error: {}>",
                                                            field_prefix, e
                                                        );
                                                        break;
                                                    }
                                                }
                                            }
                                            Some(())
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}  {} Decryption failed: {}",
                                                field_prefix,
                                                "✗".red(),
                                                e
                                            );
                                            None
                                        }
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                #[cfg(feature = "crypto")]
                if decrypted.is_none() {
                    println!(
                        "{}  (opaque blob - encrypted or unstructured)",
                        field_prefix
                    );
                }

                #[cfg(not(feature = "crypto"))]
                println!(
                    "{}  (opaque blob - encrypted or unstructured)",
                    field_prefix
                );
            }
        } else {
            match parse_section_fields(data, label) {
                Ok(fields) => {
                    if fields.is_empty() && label.child_count > 0 {
                        let field_prefix = if is_last { "   " } else { " │ " };
                        println!(
                            "{}  <parsing error: expected {} fields>",
                            field_prefix, label.child_count
                        );
                    }
                    for (j, field) in fields.iter().enumerate() {
                        let is_field_last = j == fields.len() - 1;
                        let field_prefix = if is_last { "   " } else { " │ " };
                        let field_connector = if is_field_last { "└─" } else { "├─" };
                        // Format multi-value fields as: name: val1, val2, val3
                        let values_str: Vec<String> = field
                            .values
                            .iter()
                            .map(|v| format_value_short(v))
                            .collect();
                        println!(
                            "{}{} {}: {}",
                            field_prefix,
                            field_connector,
                            field.name,
                            values_str.join(", ")
                        );
                    }
                }
                Err(e) => {
                    has_errors = true;
                    let field_prefix = if is_last { "   " } else { " │ " };
                    println!("{}  <error parsing: {}>", field_prefix, e);
                }
            }
        }
        println!(
            "{}{}",
            if is_last { "   " } else { " │ " },
            "]".truecolor(128, 128, 128)
        );
        if !is_last {
            println!(" │");
        }
    }

    // Print validation status if no errors found
    if !has_errors && integrity_ok {
        println!();
        println!("{}", "Valid".truecolor(0, 255, 0).bold());
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

                // Format multi-value fields as: name: val1, val2, val3
                let values_str: Vec<String> = field
                    .values
                    .iter()
                    .map(|v| format_value_short(v))
                    .collect();
                println!(
                    "{}{}{}: {}",
                    field_prefix,
                    field_marker,
                    field.name,
                    values_str.join(", ")
                );
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
