//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files
//! including metadata, structure verification, and field extraction.

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use vsf::decoding::parse::parse;
use vsf::decoding::parse_preamble;
use vsf::types::VsfType;

#[derive(Parser)]
#[command(name = "vsfinfo")]
#[command(about = "VSF File Inspector - Inspect, verify, and extract VSF file contents", long_about = None)]
#[command(version)]
struct Cli {
    /// VSF file to inspect
    #[arg(value_name = "FILE")]
    file: PathBuf,

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
}

fn main() {
    let cli = Cli::parse();

    // Read the file
    let data = match fs::read(&cli.file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Execute the appropriate command
    let result = match cli.command {
        Some(Commands::Info) | None => show_info(&data),
        Some(Commands::Verify) => verify_file(&data),
        Some(Commands::Extract { field_path }) => extract_field(&data, &field_path),
        Some(Commands::Tree) => show_tree(&data),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Parse VSF header and return structured information
struct VsfHeader {
    version: usize,
    backward_compat: usize,
    labels: Vec<LabelInfo>,
}

struct LabelInfo {
    name: String,
    offset: usize,
    size: usize,
    child_count: usize,
}

impl VsfHeader {
    fn parse(data: &[u8]) -> Result<Self, String> {
        // Verify magic number
        if data.len() < 4 {
            return Err("File too small to be valid VSF".to_string());
        }
        if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
            return Err("Invalid VSF magic number".to_string());
        }

        let mut pointer = 4; // Skip "RÅ<"

        // Parse header length (in bits)
        let header_length_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse header length: {}", e))?;
        let _header_length_bits = match header_length_type {
            VsfType::b(bits) => bits,
            _ => return Err("Expected b type for header length".to_string()),
        };

        // Parse version and backward compat
        let version_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
        let version = match version_type {
            VsfType::z(v) => v,
            _ => return Err("Expected z type for version".to_string()),
        };

        let backward_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse backward compat: {}", e))?;
        let backward_compat = match backward_type {
            VsfType::y(v) => v,
            _ => return Err("Expected y type for backward compat".to_string()),
        };

        // Parse file hash (mandatory BLAKE3 hash)
        let _file_hash_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse file hash: {}", e))?;
        // Note: We don't verify the file hash here, just skip past it

        // Parse label count
        let label_count_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse label count: {}", e))?;
        let label_count = match label_count_type {
            VsfType::n(count) => count,
            _ => return Err("Expected n type for label count".to_string()),
        };

        // Parse each label definition
        let mut labels = Vec::new();
        for _ in 0..label_count {
            if data[pointer] != b'(' {
                return Err("Expected '(' for label definition".to_string());
            }
            pointer += 1;

            let label_name_type = parse(data, &mut pointer)
                .map_err(|e| format!("Failed to parse label name: {}", e))?;
            let label_name = match label_name_type {
                VsfType::d(name) => name,
                _ => return Err("Expected d type for label name".to_string()),
            };

            let offset_type =
                parse(data, &mut pointer).map_err(|e| format!("Failed to parse offset: {}", e))?;
            let offset_bits = match offset_type {
                VsfType::o(bits) => bits,
                _ => return Err("Expected o type for offset".to_string()),
            };

            let size_type =
                parse(data, &mut pointer).map_err(|e| format!("Failed to parse size: {}", e))?;
            let size_bits = match size_type {
                VsfType::b(bits) => bits,
                _ => return Err("Expected b type for size".to_string()),
            };

            let field_count_type = parse(data, &mut pointer)
                .map_err(|e| format!("Failed to parse field count: {}", e))?;
            let field_count = match field_count_type {
                VsfType::n(count) => count,
                _ => return Err("Expected n type for field count".to_string()),
            };

            if data[pointer] != b')' {
                return Err("Expected ')' after label definition".to_string());
            }
            pointer += 1;

            labels.push(LabelInfo {
                name: label_name,
                offset: offset_bits / 8, // Convert bits to bytes
                size: size_bits / 8,     // Convert bits to bytes
                child_count: field_count,
            });
        }

        Ok(VsfHeader {
            version,
            backward_compat,
            labels,
        })
    }
}

/// Format a VsfType value for display
fn format_value(vsf: &VsfType) -> String {
    match vsf {
        VsfType::u0(b) => format!("{}", b),
        VsfType::u(v, _) => format!("{}", v),
        VsfType::u3(v) => format!("{}", v),
        VsfType::u4(v) => format!("{}", v),
        VsfType::u5(v) => format!("{}", v),
        VsfType::u6(v) => format!("{}", v),
        VsfType::u7(v) => format!("{}", v),
        VsfType::i(v) => format!("{}", v),
        VsfType::i3(v) => format!("{}", v),
        VsfType::i4(v) => format!("{}", v),
        VsfType::i5(v) => format!("{}", v),
        VsfType::i6(v) => format!("{}", v),
        VsfType::i7(v) => format!("{}", v),
        VsfType::f5(v) => format!("{:.4}", v),
        VsfType::f6(v) => format!("{:.8}", v),
        VsfType::x(s) => format!("\"{}\"", s),
        VsfType::p(tensor) => format!(
            "p[{}-bit] {}×{} ({} pixels, {} bytes)",
            tensor.bit_depth,
            tensor.shape.get(0).unwrap_or(&0),
            tensor.shape.get(1).unwrap_or(&0),
            tensor.len(),
            tensor.data.len()
        ),
        VsfType::t_u3(tensor) => {
            let shape_str = tensor
                .shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("×");
            format!("t_u3[{}] {} bytes", shape_str, tensor.data.len())
        }
        VsfType::t_f5(tensor) => {
            let shape_str = tensor
                .shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("×");
            format!("t_f5[{}] {} elements", shape_str, tensor.data.len())
        }
        VsfType::w(coord) => {
            let (lat, lon) = coord.to_lat_lon();
            format!("({:.4}°N, {:.4}°W)", lat, lon)
        }
        VsfType::e(et) => {
            // Format EtType based on its variant
            match et {
                vsf::types::EtType::u(v) => format!("ET {}", v),
                vsf::types::EtType::u5(v) => format!("ET {}", v),
                vsf::types::EtType::u6(v) => format!("ET {}", v),
                vsf::types::EtType::u7(v) => format!("ET {}", v),
                vsf::types::EtType::i(v) => format!("ET {}", v),
                vsf::types::EtType::i5(v) => format!("ET {}", v),
                vsf::types::EtType::i6(v) => format!("ET {}", v),
                vsf::types::EtType::i7(v) => format!("ET {}", v),
                vsf::types::EtType::f5(v) => format!("ET {:.6}", v),
                vsf::types::EtType::f6(v) => format!("ET {:.6}", v),
            }
        }
        VsfType::h(algo, hash) => {
            let algo_name = match *algo as char {
                'b' => "BLAKE3",
                's' => "SHA256",
                _ => "unknown",
            };
            format!(
                "h{}[{}] {}...",
                *algo as char,
                hash.len() * 8, // Show bits
                hex_preview(hash)
            )
        }
        VsfType::g(algo, sig) => format!(
            "sig[algo={} {} bytes] {}...",
            algo,
            sig.len(),
            hex_preview(sig)
        ),
        VsfType::k(algo, key) => format!(
            "key[algo={} {} bytes] {}...",
            algo,
            key.len(),
            hex_preview(key)
        ),
        VsfType::a(algo, mac) => format!(
            "mac[algo={} {} bytes] {}...",
            algo,
            mac.len(),
            hex_preview(mac)
        ),
        _ => format!("{:?}", vsf),
    }
}

/// Format a VsfType value for short display (tree view)
fn format_value_short(vsf: &VsfType) -> String {
    match vsf {
        VsfType::p(tensor) => format!(
            "{}×{} {}-bit",
            tensor.shape.get(0).unwrap_or(&0),
            tensor.shape.get(1).unwrap_or(&0),
            tensor.bit_depth
        ),
        VsfType::x(s) if s.len() > 30 => format!("\"{}...\"", &s[..27]),
        VsfType::x(s) => format!("\"{}\"", s),
        _ => format_value(vsf),
    }
}

/// Generate hex preview of bytes
fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(4)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// Parse section fields and return as vec of (name, value) tuples
fn parse_section_fields(data: &[u8], label: &LabelInfo) -> Result<Vec<(String, VsfType)>, String> {
    let mut pointer = label.offset;
    let mut fields = Vec::new();

    // Parse preamble if structured
    if label.child_count > 0 {
        let _ = parse_preamble(data, &mut pointer)
            .map_err(|e| format!("Failed to parse preamble: {}", e))?;
    }

    // Parse fields
    if pointer >= data.len() {
        return Ok(fields);
    }

    if data[pointer] == b'[' {
        pointer += 1;

        // Parse section name
        let section_name_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse section name: {}", e))?;
        let _section_name = match section_name_type {
            VsfType::d(name) => name,
            _ => return Err("Expected d type for section name".to_string()),
        };

        for _ in 0..label.child_count {
            if pointer >= data.len() {
                break;
            }

            if data[pointer] == b'(' {
                pointer += 1;

                let field_name_type = parse(data, &mut pointer)
                    .map_err(|e| format!("Failed to parse field name: {}", e))?;

                if let VsfType::d(name) = field_name_type {
                    if pointer < data.len() && data[pointer] == b':' {
                        pointer += 1;

                        let field_value = parse(data, &mut pointer)
                            .map_err(|e| format!("Failed to parse field value: {}", e))?;

                        fields.push((name, field_value));
                    }
                }

                if pointer < data.len() && data[pointer] == b')' {
                    pointer += 1;
                }
            }
        }
    }

    Ok(fields)
}

/// Show basic file information (default mode)
fn show_info(data: &[u8]) -> Result<(), String> {
    let header = VsfHeader::parse(data)?;

    // Calculate actual header length by parsing
    let mut pointer = 4; // After "RÅ<"
    let header_length_type = parse(data, &mut pointer)
        .map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bits = match header_length_type {
        VsfType::b(bits) => bits,
        _ => return Err("Expected b type for header length".to_string()),
    };

    println!(
        "VSF File: {} bytes ({:.1} KB)",
        data.len(),
        data.len() as f64 / 1024.0
    );
    println!();

    // Show actual VSF header structure
    println!("RÅ<");
    println!("  b[{}] : Header length (bits)", header_length_bits);
    println!("  z[{}] : Version", header.version);
    println!("  y[{}] : Backward compat", header.backward_compat);
    println!("  hb[256][...] : File hash (BLAKE3)");
    println!("  n[{}] : Label count", header.labels.len());

    // Show label definitions
    for label in &header.labels {
        println!(
            "  (d\"{}\" o[{}] b[{}] n[{}])",
            label.name,
            label.offset * 8, // Show in bits
            label.size * 8,
            label.child_count
        );
    }
    println!(">");
    println!();

    // Show sections with their actual structure
    for (i, label) in header.labels.iter().enumerate() {
        let is_last = i == header.labels.len() - 1;
        let connector = if is_last { "└─" } else { "├─" };

        // Show preamble
        let mut pointer = label.offset;
        let (preamble_count, preamble_size_bits, has_hash, has_sig) =
            if let Ok((count, size_bits, hash, sig)) = parse_preamble(data, &mut pointer) {
                (count, size_bits, hash.is_some(), sig.is_some())
            } else {
                (label.child_count, label.size * 8, false, false)
            };

        let mut preamble_str = format!("{{n[{}] b[{}]", preamble_count, preamble_size_bits);
        if has_hash {
            preamble_str.push_str(" hb3[32]");
        }
        if has_sig {
            preamble_str.push_str(" g[...]");
        }
        preamble_str.push('}');

        println!("{}{}[d\"{}\"", connector, preamble_str, label.name);

        // Parse and show fields
        if let Ok(fields) = parse_section_fields(data, label) {
            for (j, (field_name, field_value)) in fields.iter().enumerate() {
                let is_field_last = j == fields.len() - 1;
                let field_prefix = if is_last { "  " } else { "│ " };
                let field_connector = if is_field_last { "└─" } else { "├─" };
                println!(
                    "{}{} (d\"{}\":{})",
                    field_prefix,
                    field_connector,
                    field_name,
                    format_value_short(field_value)
                );
            }
        }
        println!("{}]", if is_last { "  " } else { "│ " });
        if !is_last {
            println!("│");
        }
    }

    println!();
    println!("Integrity Check:");

    // Quick integrity verification
    verify_integrity_summary(data, &header)?;

    Ok(())
}

/// Quick integrity summary (used by show_info)
fn verify_integrity_summary(data: &[u8], header: &VsfHeader) -> Result<(), String> {
    // First, verify the file-level BLAKE3 hash
    let mut pointer = 4; // After "RÅ<"

    // Skip header length
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse header: {}", e))?;
    // Skip version
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    // Skip backward compat
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse file hash
    let file_hash_type = parse(data, &mut pointer)
        .map_err(|e| format!("Failed to parse file hash: {}", e))?;

    let file_hash_verified = if let VsfType::h(_algo, stored_hash) = file_hash_type {
        // The hash is computed over the entire file with the hash bytes zeroed out
        // Find where the hash bytes start (after 'h', algo byte, and size encoding)
        let mut hash_field_start = 4; // After "RÅ<"
        let _ = parse(data, &mut hash_field_start)
            .map_err(|e| format!("Failed to skip header length: {}", e))?;
        let _ = parse(data, &mut hash_field_start)
            .map_err(|e| format!("Failed to skip version: {}", e))?;
        let _ = parse(data, &mut hash_field_start)
            .map_err(|e| format!("Failed to skip backward compat: {}", e))?;

        // Parse the hash again to find where the actual hash bytes are
        let mut temp_pointer = hash_field_start;
        let _hash_reparsed = parse(data, &mut temp_pointer)
            .map_err(|e| format!("Failed to reparse hash: {}", e))?;

        // Hash bytes are at the end of the hash field
        let hash_bytes_start = temp_pointer - stored_hash.len();

        // Create a copy with hash bytes zeroed
        let mut temp_data = data.to_vec();
        for i in 0..stored_hash.len() {
            temp_data[hash_bytes_start + i] = 0;
        }

        // Compute hash over entire file with zeroed hash bytes
        let computed = blake3::hash(&temp_data);

        if computed.as_bytes() == stored_hash.as_slice() {
            true
        } else {
            false
        }
    } else {
        false
    };

    // Check section-level hashes
    let mut verified_sections = 0;
    let mut total_sections = 0;

    for label in &header.labels {
        if label.child_count > 0 {
            total_sections += 1;
            let mut pointer = label.offset;
            if let Ok((_, _, hash, _)) = parse_preamble(data, &mut pointer) {
                if let Some(h) = hash {
                    let section_end = label.offset + label.size;
                    if section_end <= data.len() {
                        let section_data = &data[label.offset..section_end];
                        let computed = blake3::hash(section_data);
                        if computed.as_bytes() == h.as_slice() {
                            verified_sections += 1;
                        }
                    }
                }
            }
        }
    }

    if file_hash_verified {
        println!("PASS");
    } else {
        println!("FAIL");
    }

    Ok(())
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
    let header = match VsfHeader::parse(data) {
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

    // Verify each section
    for label in &header.labels {
        let mut pointer = label.offset;

        // Check preamble if structured
        if label.child_count > 0 {
            match parse_preamble(data, &mut pointer) {
                Ok((count, size_bits, hash, sig)) => {
                    // Verify count matches
                    if count != label.child_count {
                        println!(
                            "✗ Section '{}': preamble count {} != label count {}",
                            label.name, count, label.child_count
                        );
                        errors += 1;
                    }

                    // Verify size matches
                    let expected_bits = label.size * 8;
                    if size_bits != expected_bits {
                        println!(
                            "✗ Section '{}': preamble size {} != label size {}",
                            label.name, size_bits, expected_bits
                        );
                        errors += 1;
                    }

                    // Verify hash if present
                    if let Some(h) = hash {
                        let section_end = label.offset + label.size;
                        if section_end <= data.len() {
                            let section_data = &data[label.offset..section_end];
                            let computed = blake3::hash(section_data);
                            if computed.as_bytes() == h.as_slice() {
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

                    // Note signature presence
                    if sig.is_some() {
                        println!(
                            "✓ Section '{}': signature present (verification TBD)",
                            label.name
                        );
                        warnings += 1;
                    }
                }
                Err(e) => {
                    println!("✗ Section '{}': preamble parsing failed: {}", label.name, e);
                    errors += 1;
                }
            }
        }
    }

    // Look for TOKEN signature
    if let Some(_token_section) = header
        .labels
        .iter()
        .find(|l| l.name == "token_auth" || l.name == "token auth")
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
    // field_path like "raw.iso_speed" or "token_auth.location"
    let parts: Vec<&str> = field_path.split('.').collect();

    if parts.len() != 2 {
        return Err("Field path must be 'section.field'".into());
    }

    let section_name = parts[0];
    let field_name = parts[1];

    let header = VsfHeader::parse(data)?;

    // Find section (handle both space and underscore variants)
    let section = header
        .labels
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
    for (name, value) in fields {
        if name == field_name
            || name.replace(' ', "_") == field_name
            || name.replace('_', " ") == field_name
        {
            println!("{}", format_value(&value));
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
    let header = VsfHeader::parse(data)?;

    println!("VSF File Tree");
    println!("{}", "=".repeat(50));
    println!();

    for (i, label) in header.labels.iter().enumerate() {
        let is_last = i == header.labels.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };

        println!(
            "{}{} ({} bytes, {} fields)",
            prefix, label.name, label.size, label.child_count
        );

        // Parse fields
        if let Ok(fields) = parse_section_fields(data, label) {
            for (j, (field_name, field_value)) in fields.iter().enumerate() {
                let is_field_last = j == fields.len() - 1;
                let field_prefix = if is_last { "    " } else { "│   " };
                let field_marker = if is_field_last {
                    "└── "
                } else {
                    "├── "
                };

                println!(
                    "{}{}{}: {}",
                    field_prefix,
                    field_marker,
                    field_name,
                    format_value_short(field_value)
                );
            }
        }

        if !is_last {
            println!("│");
        }
    }

    Ok(())
}
