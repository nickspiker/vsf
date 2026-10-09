//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files including metadata, structure verification, and field extraction.

use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use vsf::container::{truncate_file, Document};
use vsf::file_format::VsfHeader;
use vsf::spectral_image::PlaneLayout;
use vsf::visual::{FrameData, Samples, Visual};
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

    /// Show file structure as a tree (latest generation; tracks, planes and the layer program for a visual)
    Tree,

    /// List the generations of an append-only document, oldest first, each with its content hash checked
    History,

    /// Write the document exactly as it stood at generation N (its first bytes, byte for byte)
    Checkout {
        /// Generation index (0 = the original)
        #[arg(value_name = "N")]
        generation: u32,
        /// Output path
        #[arg(value_name = "OUT")]
        out: PathBuf,
    },

    /// Remove the latest generation in place by truncating the file (or discard a torn append's orphaned tail)
    Undo,

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
        Some(Commands::History) => show_history(&data),
        Some(Commands::Checkout { generation, out }) => checkout(&data, generation, &out),
        Some(Commands::Undo) => undo(&file),
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

/// ChaCha20-Poly1305 decryption helper — reads the current 24-byte-nonce XChaCha20-Poly1305 first,
/// then the legacy 12-byte-nonce ChaCha20-Poly1305 (the 2026-08-18 stack-wide migration). The Poly1305
/// tag disambiguates, so an inspection of either format Just Works.
#[cfg(feature = "crypto")]
fn decrypt_chacha20poly1305(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use chacha20poly1305::{
        aead::Aead, ChaCha20Poly1305, KeyInit, Nonce, XChaCha20Poly1305, XNonce,
    };

    if encrypted.len() < 12 + 16 {
        return Err("File too short for ChaCha20-Poly1305 (need at least 28 bytes)".to_string());
    }

    // Current: XChaCha20-Poly1305, [nonce:24][ct+tag].
    if encrypted.len() >= 24 + 16 {
        if let (Ok(cipher), Ok(nonce)) = (
            XChaCha20Poly1305::new_from_slice(key),
            <[u8; 24]>::try_from(&encrypted[..24]),
        ) {
            if let Ok(pt) = cipher.decrypt(&XNonce::from(nonce), &encrypted[24..]) {
                return Ok(pt);
            }
        }
    }

    // Legacy: ChaCha20-Poly1305, [nonce:12][ct+tag].
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("Key init error: {}", e))?;
    let nonce_bytes: [u8; 12] = encrypted[..12]
        .try_into()
        .map_err(|_| "Invalid nonce length")?;
    let nonce: Nonce = nonce_bytes.into();
    cipher
        .decrypt(&nonce, &encrypted[12..])
        .map_err(|e| format!("Decryption failed under both XChaCha and legacy: {} (wrong key?)", e))
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
/// The final line mirrors [`vsf::verification::read_verified`]'s doctrine: hp alone verifies an unsigned document (hp and hb are equally self-attesting BLAKE3 anchors); a signature, when present, must check out; any anchor mismatch is TAMPERED.
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

    // Overall verdict, matching read_verified's semantics: hp alone verifies an unsigned doc; every PRESENT anchor must pass.
    let sig_ok: Option<bool> = if is_signed {
        #[cfg(feature = "crypto")]
        {
            Some(vsf::verification::verify_file_signature(data).unwrap_or(false))
        }
        #[cfg(not(feature = "crypto"))]
        {
            None // present but uncheckable in this build
        }
    } else {
        None
    };

    print!(" {} ", "verdict:".cyan());
    if !provenance_ok {
        println!("{}", "TAMPERED (provenance mismatch)".truecolor(255, 0, 0));
    } else if integrity == Some(false) {
        println!("{}", "TAMPERED (rolling hash mismatch)".truecolor(255, 0, 0));
    } else if sig_ok == Some(false) {
        println!("{}", "TAMPERED (bad signature)".truecolor(255, 0, 0));
    } else if is_signed && sig_ok.is_none() {
        // Signed doc but this build can't check the signature.
        println!("{}", "UNCHECKED (signed; crypto feature off)".yellow());
    } else {
        println!("{}", "VERIFIED".truecolor(0, 255, 0));
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

    // Parse header — the latest generation's when the document has been appended to, so the live table of contents is what gets checked.
    let doc = Document::open(data).ok();
    let header = match &doc {
        Some(d) => {
            println!("✓ Header structure valid (generation {} at offset {})", d.generation(), d.header_offset());
            d.header().clone()
        }
        None => match VsfHeader::decode(data) {
            Ok((h, _)) => {
                println!("✓ Header structure valid");
                h
            }
            Err(e) => {
                println!("✗ Header parsing failed: {}", e);
                return Err("Cannot continue verification".into());
            }
        },
    };
    let labels = labels_from_header(&header);

    // Generations: each header's content hash over its own bytes, and the prev chain between them.
    if let Some(d) = &doc {
        match d.generations() {
            Ok(gens) => {
                for g in &gens {
                    match d.generation_hash_ok(g) {
                        Ok(Some(true)) => println!("✓ Generation {}: content hash verified{}", g.index, gen_label(&g.tool, &g.action)),
                        Ok(Some(false)) => {
                            println!("✗ Generation {}: content hash mismatch!{}", g.index, gen_label(&g.tool, &g.action));
                            errors += 1;
                        }
                        Ok(None) => {
                            println!("○ Generation {}: no content hash", g.index);
                            warnings += 1;
                        }
                        Err(e) => {
                            println!("✗ Generation {}: {}", g.index, e);
                            errors += 1;
                        }
                    }
                }
                if gens.len() > 1 {
                    println!("✓ History chain: {} generations, every prev link matches its header", gens.len());
                }
            }
            Err(e) => {
                println!("✗ History chain broken: {}", e);
                errors += 1;
            }
        }
        if d.orphaned_bytes() > 0 {
            println!("! {} orphaned bytes after generation {} (a torn append; `vsfinfo undo` discards them)", d.orphaned_bytes(), d.generation());
            warnings += 1;
        }
    }

    // Verify each section
    for label in &labels {
        // Inline header fields (a generation's `prev` carries the previous header's hash as a value) are checked by the chain walk above, not as sections.
        if is_inline_label(label) {
            continue;
        }
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
    // The latest generation's table of contents when the document has one; the plain header otherwise (encrypted and pre-container files).
    let doc = Document::open(data).ok();
    let header = match &doc {
        Some(d) => d.header().clone(),
        None => VsfHeader::decode(data)?.0,
    };
    // Inline generation fields (gen, start, prev, tool, action) are shown in the history lines, not as sections.
    let labels: Vec<_> = labels_from_header(&header).into_iter().filter(|l| !is_inline_label(l)).collect();

    println!("VSF File Tree");
    println!("{}", "=".repeat(50));
    if let Some(d) = &doc {
        if d.generation() > 0 || d.orphaned_bytes() > 0 {
            for line in history_lines(d)? {
                println!("{}", line);
            }
            println!();
        }
    }
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
                    // A newline-joined list (channel names, graph ops) stays on its own line of the tree.
                    let val_str = format_value_short(val_vsf).replace('\n', " ⏎ ");
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

    if let Some(d) = &doc {
        if d.entry("visual").is_some() || d.entry("spectral_image").is_some() {
            match Visual::read_document(d) {
                Ok(v) => print_visual(&v),
                Err(e) => println!("\n(visual: {})", e),
            }
        }
    }

    Ok(())
}

/// A header field with no body: inline values (a generation's `gen`/`start`/`prev`/`tool`/`action`) or a bare name.
fn is_inline_label(l: &vsf::inspect::LabelInfo) -> bool {
    l.size == 0 && l.offset == 0 && l.child_count == 0
}

fn gen_label(tool: &str, action: &str) -> String {
    if tool.is_empty() && action.is_empty() {
        String::new()
    } else {
        format!("  ({} — {})", tool, action)
    }
}

/// One line per generation, oldest first, each content hash checked; a trailing line for a torn append's orphaned tail.
fn history_lines(doc: &Document<&[u8]>) -> Result<Vec<String>, String> {
    let gens = doc.generations().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for g in &gens {
        let mark = match doc.generation_hash_ok(g).map_err(|e| e.to_string())? {
            Some(true) => "✓",
            Some(false) => "✗",
            None => "○",
        };
        let who = if g.tool.is_empty() && g.action.is_empty() { "original".to_string() } else { format!("{} — {}", g.tool, g.action) };
        out.push(format!("{} gen {:<3} bytes {}..{} ({} B)  header @{} ({} B)  {}", mark, g.index, g.start, g.end, g.end - g.start, g.header_offset, g.header_len, who));
    }
    if doc.orphaned_bytes() > 0 {
        out.push(format!("! {} orphaned bytes after generation {} (a torn append; `vsfinfo undo` discards them)", doc.orphaned_bytes(), doc.generation()));
    }
    Ok(out)
}

/// `history`: the generations of an append-only document.
fn show_history(data: &[u8]) -> Result<(), String> {
    let doc = Document::open(data).map_err(|e| e.to_string())?;
    println!("{}", "VSF History".cyan().bold());
    println!("{}", "=".repeat(50));
    for line in history_lines(&doc)? {
        println!("{}", line);
    }
    if doc.generation() == 0 && doc.orphaned_bytes() == 0 {
        println!("(the original; nothing has been appended)");
    }
    Ok(())
}

/// `checkout N OUT`: the document exactly as it stood at generation N.
fn checkout(data: &[u8], generation: u32, out: &Path) -> Result<(), String> {
    let doc = Document::open(data).map_err(|e| e.to_string())?;
    let bytes = doc.checkout(generation).map_err(|e| e.to_string())?;
    fs::write(out, &bytes).map_err(|e| format!("write {}: {}", out.display(), e))?;
    println!("wrote generation {} ({} bytes) to {}", generation, bytes.len(), out.display());
    Ok(())
}

/// `undo`: truncate the file to the previous generation's end — or, when the file ends in a torn append, to the last provable header.
fn undo(path: &Path) -> Result<(), String> {
    let f = fs::File::open(path).map_err(|e| format!("open {}: {}", path.display(), e))?;
    let doc = Document::open(f).map_err(|e| e.to_string())?;
    let (len, what) = if doc.orphaned_bytes() > 0 {
        (doc.end(), format!("{} orphaned bytes of a torn append", doc.orphaned_bytes()))
    } else {
        match doc.undo_len().map_err(|e| e.to_string())? {
            Some(len) => {
                let gens = doc.generations().map_err(|e| e.to_string())?;
                let g = gens.last().unwrap();
                (len, format!("generation {}{}", g.index, gen_label(&g.tool, &g.action)))
            }
            None => {
                println!("{} is the original; nothing to undo", path.display());
                return Ok(());
            }
        }
    };
    drop(doc);
    truncate_file(path, len).map_err(|e| e.to_string())?;
    println!("removed {}; {} is now {} bytes", what, path.display(), len);
    Ok(())
}

fn hex8(h: &[u8]) -> String {
    h.iter().take(4).map(|b| format!("{:02x}", b)).collect::<String>() + "…"
}

fn floats(v: &[f32]) -> String {
    v.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(" ")
}

fn opt(v: Option<f32>, unit: &str) -> String {
    v.map(|x| format!("{}{}", x, unit)).unwrap_or_default()
}

/// The visual model of a document: tracks, spans, planes, capture, characterization, the layer programs, views and foreign metadata.
fn print_visual(v: &Visual) {
    println!();
    println!("{}", "Visual".cyan().bold());
    println!("{}", "=".repeat(50));
    if !v.title.is_empty() {
        println!("title: {}", v.title);
    }
    if !v.text.is_empty() {
        println!("text: {}", v.text);
    }
    if !v.default_view.is_empty() {
        println!("default view: {}", v.default_view);
    }
    for t in &v.tracks {
        let codec = if t.codec.is_empty() { "packed".to_string() } else { t.codec.clone() };
        println!("track.{}: {} {} · {} channel(s) · {}{}", t.id, t.kind.as_str(), t.role.as_str(), t.channels, codec, if t.title.is_empty() { String::new() } else { format!(" · {}", t.title) });
    }
    for sp in &v.spans {
        let (mut refs, mut inline, mut holes) = (0, 0, 0);
        for d in &sp.data {
            match d {
                FrameData::PlaneRef(_) => refs += 1,
                FrameData::Packed(_) | FrameData::Wrapped(..) => inline += 1,
                FrameData::Hole => holes += 1,
            }
        }
        let first_ref = sp.data.iter().find_map(|d| if let FrameData::PlaneRef(n) = d { Some(n.as_str()) } else { None }).unwrap_or("");
        println!("span.{}.{}: first {}, {} frame(s) — {} plane ref(s) {} inline {} hole(s) {}", sp.track, sp.seq, sp.first, sp.count(), refs, inline, holes, if refs == 1 { format!("→ {}", first_ref) } else { String::new() });
    }
    for p in &v.planes {
        let layout = match &p.layout {
            PlaneLayout::Mosaic { cfa } => format!("mosaic {}×{} cfa", cfa.shape.first().copied().unwrap_or(0), cfa.shape.get(1).copied().unwrap_or(0)),
            PlaneLayout::Planar => "planar".to_string(),
        };
        let samples = match &p.samples {
            Samples::Packed(t) => format!("packed {:?}", t.shape),
            Samples::Wrapped(c, b) => format!("wrapped '{}' {} B", *c as char, b.len()),
        };
        let names: Vec<&str> = p.channels.iter().map(|c| c.name.as_str()).collect();
        println!("plane.{}.{}.{}: {} {}×{} [{}] {} {}-bit {} · {}", p.track, p.seq, p.name, p.role.as_str(), p.width, p.height, names.join(""), layout, p.depth, p.encoding.as_str(), samples);
        println!("    black [{}] white [{}]", floats(&p.black), floats(&p.white));
        let mut extras = Vec::new();
        if let Some(c) = p.maker_crop {
            extras.push(format!("maker_crop {:?}", c));
        }
        if p.orientation != 1 {
            extras.push(format!("orientation {}", p.orientation));
        }
        if let Some(at) = p.at {
            extras.push(format!("at {}", at));
        }
        if let Some((hp, hb)) = &p.parent {
            extras.push(format!("parent {} plane {}", hex8(hp), hex8(hb)));
        }
        if !p.program.is_empty() {
            extras.push(format!("program {}", p.program));
        }
        if p.channels.iter().any(|c| c.curve.is_some()) {
            extras.push("spectral curves".to_string());
        }
        if !extras.is_empty() {
            println!("    {}", extras.join(" · "));
        }
    }
    if let Some(c) = &v.capture {
        let mut parts = Vec::new();
        for s in [c.make.as_str(), c.model.as_str(), c.sensor_module.as_str(), c.lens.as_str()] {
            if !s.is_empty() {
                parts.push(s.to_string());
            }
        }
        println!("capture: {}", parts.join(" · "));
        let mut exp = Vec::new();
        if let Some(e) = c.exposure_s {
            exp.push(if e < 1.0 && e > 0.0 { format!("1/{:.0} s", 1.0 / e) } else { format!("{} s", e) });
        }
        if c.f_number.is_some() {
            exp.push(format!("f/{}", c.f_number.unwrap()));
        }
        if c.t_stop.is_some() {
            exp.push(format!("T{}", c.t_stop.unwrap()));
        }
        if c.iso.is_some() {
            exp.push(format!("ISO {}", c.iso.unwrap()));
        }
        if c.gain.is_some() {
            exp.push(format!("gain {}", c.gain.unwrap()));
        }
        if c.focal_mm.is_some() {
            exp.push(format!("{}mm{}", c.focal_mm.unwrap(), c.focal_35mm.map(|f| format!(" ({}mm eq)", f)).unwrap_or_default()));
        }
        if c.subject_m.is_some() {
            exp.push(opt(c.subject_m, " m"));
        }
        exp.push(format!("baseline {:+} EV", c.baseline));
        if !c.mode.is_empty() {
            exp.push(c.mode.clone());
        }
        println!("    {}", exp.join(" · "));
        let mut when = Vec::new();
        if !c.at_text.is_empty() {
            when.push(c.at_text.clone());
        }
        if let Some(at) = c.at {
            when.push(format!("eagle {}", at));
        }
        if let Some(loc) = &c.location {
            let (lat, lon) = loc.to_lat_lon();
            when.push(format!("{:.6}, {:.6}{}", lat, lon, c.altitude_m.map(|a| format!(" @ {} m", a)).unwrap_or_default()));
        }
        if !when.is_empty() {
            println!("    {}", when.join(" · "));
        }
        if !c.stack.is_empty() || !c.cal_applied.is_empty() || !c.shading.is_empty() {
            println!("    stack {} · cal [{}] · shading {}", c.stack, c.cal_applied.join(" "), c.shading);
        }
    }
    if let Some(p) = &v.characterization {
        println!("characterization → {}{}: {} entr{}", p.target, if v.observer.is_empty() { String::new() } else { format!(" ({})", v.observer) }, p.entries.len(), if p.entries.len() == 1 { "y" } else { "ies" });
        for (i, e) in p.entries.iter().enumerate() {
            println!("    [{}] {} {}/{} illuminant {} · [{}]", i, e.source, e.class.as_str(), e.tier.as_str(), e.illuminant, floats(&e.matrix));
        }
        for (i, cm) in p.dng_colormatrix.iter().enumerate() {
            if let Some((m, ill)) = cm {
                println!("    dng ColorMatrix{} illuminant {} · [{}]", i + 1, ill, floats(m));
            }
        }
        if let Some(cal) = &p.cal {
            println!("    target type {} serial {} calibrated {}", cal.target_type, cal.target_serial, cal.timestamp);
        }
        if let Some((cam, refv)) = &p.patches {
            println!("    {} patches", cam.len().min(refv.len()) / 3);
        }
    }
    for g in &v.graphs {
        println!("graph.{} (root {}, hash {}):", g.id, g.root, hex8(&g.hash()));
        for (i, op) in g.ops.iter().enumerate() {
            let mut line = format!("    {:>3}  {:<12} {:<9} arity {}", i, op.name, op.class.as_str(), op.arity);
            if !op.params.is_empty() {
                line.push_str(&format!("  [{}]", floats(&op.params)));
            }
            if !op.reference.is_empty() {
                line.push_str(&format!("  → {}", op.reference));
            }
            if i == g.root {
                line.push_str("  ◂ root");
            }
            println!("{}", line);
        }
    }
    for w in &v.views {
        println!("view.{}: {} op {} → {} {} {}-bit · {}", w.id, w.graph, w.root, w.space, w.encoding.as_str(), w.bits, w.purpose.as_str());
    }
    if !v.foreign.is_empty() {
        let mut f = Vec::new();
        if let Some(x) = &v.foreign.xmp {
            f.push(format!("xmp {} B", x.len()));
        }
        if let Some(x) = &v.foreign.icc {
            f.push(format!("icc {} B", x.len()));
        }
        if !v.foreign.profile_name.is_empty() {
            f.push(format!("profile '{}'", v.foreign.profile_name));
        }
        if !v.foreign.calibration_signature.is_empty() {
            f.push(format!("calibration signature '{}'", v.foreign.calibration_signature));
        }
        println!("foreign: {}", f.join(" · "));
    }
    if !v.provenance.handle.is_empty() || v.provenance.calibration_hash.is_some() || v.provenance.camera_ihi.is_some() {
        println!("provenance: handle '{}'{}{}", v.provenance.handle, v.provenance.calibration_hash.map(|h| format!(" · calibration {}", hex8(&h))).unwrap_or_default(), v.provenance.camera_ihi.map(|h| format!(" · camera ihi {}", hex8(&h))).unwrap_or_default());
    }
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
