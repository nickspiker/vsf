//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files
//! including metadata, structure verification, and field extraction.

use chrono::{Datelike, Timelike};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use vsf::decoding::parse::parse;
use vsf::file_format::VsfHeader as LibVsfHeader;
use vsf::schema::SchemaRegistry;
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
    let data = match fs::read(&file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Execute the appropriate command
    let result = match cli.command {
        Some(Commands::Info) | None => show_info(&data, cli.detailed, cli.key.as_deref()),
        Some(Commands::Verify) => verify_file(&data),
        Some(Commands::Extract { field_path }) => extract_field(&data, &field_path),
        Some(Commands::Tree) => show_tree(&data),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// VSF header wrapper for display - uses library's VsfHeader::decode()
struct VsfHeader {
    version: usize,
    backward_compat: usize,
    modified_time: Option<VsfType>,   // Timestamp
    provenance_hash: Option<VsfType>, // BLAKE3 provenance hash (required)
    signer_pubkey: Option<VsfType>,   // Ed25519 public key (optional, ke)
    signature: Option<VsfType>,       // Ed25519 signature (optional, ge)
    rolling_hash: Option<VsfType>,    // BLAKE3 rolling hash (optional)
    labels: Vec<LabelInfo>,
}

struct LabelInfo {
    name: String,
    hash: Option<VsfType>,
    signature: Option<VsfType>,
    key: Option<VsfType>,
    wrap: Option<VsfType>, // v: wrapped/encrypted data marker
    offset: usize,
    size: usize,
    child_count: usize,
}

impl VsfHeader {
    fn parse(data: &[u8]) -> Result<Self, String> {
        // Use the library's VsfHeader::decode() for parsing
        let (lib_header, _bytes_consumed) = LibVsfHeader::decode(data)?;

        // Convert library header to local format for display
        let labels = lib_header
            .fields
            .iter()
            .map(|field| LabelInfo {
                name: field.name.clone(),
                hash: field.hash.clone(),
                signature: field.signature.clone(),
                key: field.key.clone(),
                wrap: field.wrap.clone(),
                offset: field.offset_bytes,
                size: field.size_bytes,
                child_count: field.child_count,
            })
            .collect();

        Ok(VsfHeader {
            version: lib_header.version,
            backward_compat: lib_header.backward_compat,
            modified_time: Some(lib_header.creation_time),
            provenance_hash: Some(lib_header.provenance_hash),
            signer_pubkey: lib_header.signer_pubkey,
            signature: lib_header.signature,
            rolling_hash: lib_header.rolling_hash,
            labels,
        })
    }
}

/// Format bytes with proper units and 4 significant figures
fn format_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    const PB: f64 = TB * 1024.0;

    let bytes_f64 = bytes as f64;

    if bytes_f64 >= PB {
        let pb = bytes_f64 / PB;
        if pb >= 100.0 {
            format!("{:.1} PB", pb)
        } else if pb >= 10.0 {
            format!("{:.2} PB", pb)
        } else {
            format!("{:.3} PB", pb)
        }
    } else if bytes_f64 >= TB {
        let tb = bytes_f64 / TB;
        if tb >= 100.0 {
            format!("{:.1} TB", tb)
        } else if tb >= 10.0 {
            format!("{:.2} TB", tb)
        } else {
            format!("{:.3} TB", tb)
        }
    } else if bytes_f64 >= GB {
        let gb = bytes_f64 / GB;
        if gb >= 100.0 {
            format!("{:.1} GB", gb)
        } else if gb >= 10.0 {
            format!("{:.2} GB", gb)
        } else {
            format!("{:.3} GB", gb)
        }
    } else if bytes_f64 >= MB {
        let mb = bytes_f64 / MB;
        if mb >= 100.0 {
            format!("{:.1} MB", mb)
        } else if mb >= 10.0 {
            format!("{:.2} MB", mb)
        } else {
            format!("{:.3} MB", mb)
        }
    } else if bytes_f64 >= KB {
        let kb = bytes_f64 / KB;
        if kb >= 100.0 {
            format!("{:.1} KB", kb)
        } else if kb >= 10.0 {
            format!("{:.2} KB", kb)
        } else {
            format!("{:.3} KB", kb)
        }
    } else {
        format!("{} Bytes", bytes)
    }
}

/// Format number with spaces every 3 digits (e.g., 1 000 000)
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(*c);
    }

    result
}

/// Format Eagle Time (ET) in human-readable format: 2025-OCT-29 6:42:21.813 PM
fn format_et(et: &vsf::types::EtType) -> String {
    // Convert EtType to EagleTime and then to DateTime using chrono
    let eagle_time = vsf::types::EagleTime::new(et.clone());
    let dt = eagle_time.to_datetime();

    // Extract milliseconds from fractional seconds if available
    let milliseconds = match et {
        vsf::types::EtType::f5(v) => ((v.fract().abs() * 1000.0) as u32) % 1000,
        vsf::types::EtType::f6(v) => ((v.fract().abs() * 1000.0) as u32) % 1000,
        _ => 0,
    };

    let year = dt.year();
    let month = dt.month();
    let day = dt.day();
    let hour = dt.hour();
    let minute = dt.minute();
    let second = dt.second();

    // Convert to 12-hour format
    let (hour_12, am_pm) = if hour == 0 {
        (12, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12, "PM")
    } else {
        (hour - 12, "PM")
    };

    let month_name = match month {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        12 => "DEC",
        _ => "UNK",
    };

    format!(
        "{}-{}-{:02} {}:{:02}:{:02}.{:03} {}",
        year, month_name, day, hour_12, minute, second, milliseconds, am_pm
    )
}

/// Quick file hash verification
fn verify_file_hash(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    let mut pointer = 4; // Skip "RÅ<"

    // Skip header length
    if parse(data, &mut pointer).is_err() {
        return false;
    }

    // Skip version and backward compat
    if parse(data, &mut pointer).is_err() {
        return false;
    }
    if parse(data, &mut pointer).is_err() {
        return false;
    }

    // Check if hash exists
    if pointer >= data.len() || data[pointer] != b'h' {
        return false;
    }

    let hash_position = pointer;

    // Parse hash
    let hash_type = match parse(data, &mut pointer) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let stored_hash = match hash_type {
        VsfType::hb(hash) => hash,
        _ => return false,
    };

    // Find where hash bytes start by reparsing
    let mut temp_pointer = hash_position;
    let _ = parse(data, &mut temp_pointer);
    let hash_bytes_start = temp_pointer - stored_hash.len();

    // Create copy with zeroed hash
    let mut temp_data = data.to_vec();
    for i in 0..stored_hash.len() {
        temp_data[hash_bytes_start + i] = 0;
    }

    // Compute and compare
    let computed = blake3::hash(&temp_data);
    computed.as_bytes() == stored_hash.as_slice()
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
        VsfType::x(s) => s.clone(),
        VsfType::p(tensor) => {
            let shape_str = tensor
                .shape
                .iter()
                .map(|d| format_number(*d))
                .collect::<Vec<_>>()
                .join(" × ");
            format!(
                "{}, {}-bit packed tensor ({} Bytes)",
                shape_str,
                tensor.bit_depth,
                format_number(tensor.data.len())
            )
        }
        VsfType::t_u3(tensor) => {
            let shape_str = tensor
                .shape
                .iter()
                .map(|d| format_number(*d))
                .collect::<Vec<_>>()
                .join(" × ");
            let format_hint = if tensor.ndim() == 1 {
                " [1D vector]"
            } else {
                ""
            };
            format!(
                "{}, 8-bit tensor{} ({} Bytes)",
                shape_str,
                format_hint,
                format_number(tensor.data.len())
            )
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
        VsfType::e(et) => format_et(et),
        VsfType::hp(hash) => format!(
            "hp[BLAKE3 Provenance {} Bytes] {}...",
            hash.len(),
            hex_preview(hash)
        ),
        VsfType::hb(hash) => format!(
            "hb[BLAKE3 Rolling {} Bytes] {}...",
            hash.len(),
            hex_preview(hash)
        ),
        VsfType::hs(hash) => format!("hs[SHA-2 {} Bytes] {}...", hash.len(), hex_preview(hash)),

        VsfType::ge(sig) => format!("ge[Ed25519 {} Bytes] {}...", sig.len(), hex_preview(sig)),
        VsfType::gp(sig) => format!("gp[ECDSA-P256 {} Bytes] {}...", sig.len(), hex_preview(sig)),
        VsfType::gr(sig) => format!("gr[RSA {} Bytes] {}...", sig.len(), hex_preview(sig)),

        VsfType::ke(key) => format!(
            "ke[Ed25519 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kx(key) => format!("kx[X25519 key {} Bytes] {}...", key.len(), hex_preview(key)),
        VsfType::kp(key) => format!(
            "kp[ECDSA-P256 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kc(key) => format!(
            "kc[ChaCha20-Poly1305 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::ka(key) => format!(
            "ka[AES-256-GCM key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),

        VsfType::ah(mac) => format!(
            "ah[HMAC-SHA256 {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::as_(mac) => format!(
            "as_[HMAC-SHA512 {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::ap(mac) => format!("ap[Poly1305 {} Bytes] {}...", mac.len(), hex_preview(mac)),
        VsfType::ab(mac) => format!(
            "ab[BLAKE3-keyed {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::ac(mac) => format!("ac[CMAC-AES {} Bytes] {}...", mac.len(), hex_preview(mac)),

        VsfType::v(algo, data) => {
            use vsf::crypto_algorithms::wrap_algorithm_name;
            let algo_name = wrap_algorithm_name(*algo).unwrap_or("unknown");
            format!(
                "wrap[{} {} Bytes] {}",
                algo_name,
                data.len(),
                if data.is_empty() {
                    ""
                } else {
                    &hex_preview(data)
                }
            )
        }
        _ => format!("{:?}", vsf),
    }
}

/// Format a VsfType value for short display (tree view)
fn format_value_short(vsf: &VsfType) -> String {
    match vsf {
        VsfType::p(tensor) => {
            let shape_str = tensor
                .shape
                .iter()
                .map(|d| format_number(*d))
                .collect::<Vec<_>>()
                .join(" × ");
            format!(
                "{}, {}-bit packed tensor ({} Bytes)",
                shape_str,
                tensor.bit_depth,
                format_number(tensor.data.len())
            )
        }
        VsfType::x(s) if s.len() > 30 => format!("{}...", &s[..27]),
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

    // Parse fields
    if pointer >= data.len() {
        return Err(format!(
            "Offset {} beyond file length {}",
            pointer,
            data.len()
        ));
    }

    if data[pointer] != b'[' {
        return Err(format!(
            "Expected '[' at offset {}, found '{}'",
            pointer, data[pointer] as char
        ));
    }

    pointer += 1;

    // Parse section name
    let section_name_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse section name: {}", e))?;
    let _section_name = match section_name_type {
        VsfType::d(name) => name,
        _ => return Err("Expected d type for section name".to_string()),
    };

    for i in 0..label.child_count {
        if pointer >= data.len() {
            return Err(format!(
                "Unexpected end of file at field {}/{}",
                i, label.child_count
            ));
        }

        if data[pointer] != b'(' {
            return Err(format!(
                "Expected '(' at field {}, found '{}'",
                i, data[pointer] as char
            ));
        }
        pointer += 1;

        let field_name_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse field name at field {}: {}", i, e))?;

        let name = match field_name_type {
            VsfType::d(n) => n,
            _ => return Err(format!("Expected d type for field name at field {}", i)),
        };

        if pointer >= data.len() || data[pointer] != b':' {
            return Err(format!(
                "Expected ':' after field name '{}', found '{}'",
                name,
                if pointer < data.len() {
                    data[pointer] as char
                } else {
                    '?'
                }
            ));
        }
        pointer += 1;

        let field_value = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse value for field '{}': {}", name, e))?;

        fields.push((name, field_value));

        if pointer >= data.len() || data[pointer] != b')' {
            return Err(format!(
                "Expected ')' after field value, found '{}'",
                if pointer < data.len() {
                    data[pointer] as char
                } else {
                    '?'
                }
            ));
        }
        pointer += 1;
    }

    Ok(fields)
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

    let header = VsfHeader::parse(data)?;

    // Calculate actual header length by parsing
    let mut pointer = 4; // After "RÅ<"

    // Parse version and backward_compat first (VSF v4+ format)
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Now parse header length
    let header_length_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    // Header length is encoded inclusively (with overhead baked in)
    // Need to subtract the encoding overhead to get actual header size
    let header_length_bytes_encoded = match header_length_type {
        VsfType::b(bytes, _) => bytes,
        _ => return Err("Expected b type for header length".to_string()),
    };

    // Determine overhead based on encoded size (in bytes)
    let overhead = if header_length_bytes_encoded < 256 {
        2 // b[3][value] = 2 bytes overhead
    } else if header_length_bytes_encoded < 65536 {
        3 // b[4][value] = 3 bytes overhead
    } else {
        5 // b[5][value] = 5 bytes overhead
    };

    let _header_length_bytes = header_length_bytes_encoded - overhead;

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
        println!("{:<25} {}", "RÅ".truecolor(128, 128, 128), "Magic".truecolor(128, 128, 128));
        println!("{:<25} {}", "<".truecolor(128, 128, 128), "Header start".truecolor(128, 128, 128));
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
        if let Some(ref creation) = header.modified_time {
            if let VsfType::e(ref et) = creation {
                let timestamp = match et {
                    vsf::types::EtType::f5(v) => *v as f64,
                    vsf::types::EtType::f6(v) => *v,
                    _ => 0.0,
                };
                println!(
                    " {:<24} {} {}",
                    format!("ef6 {:.10}", timestamp).truecolor(128, 128, 128),
                    "Created".cyan(),
                    format_et(et).white()
                );
            }
        }

        // Display provenance hash with encoding
        if let Some(ref hp) = header.provenance_hash {
            if let VsfType::hp(hash_bytes) = hp {
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
        }

        // Display signer pubkey (ke) with encoding
        if let Some(ref pk) = header.signer_pubkey {
            if let VsfType::ke(pk_bytes) = pk {
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
        }

        // Display Ed25519 signature with encoding
        if let Some(ref sig) = header.signature {
            if let VsfType::ge(sig_bytes) = sig {
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
        }

        // Display rolling hash with encoding
        if let Some(ref hb) = header.rolling_hash {
            if let VsfType::hb(hash_bytes) = hb {
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
        }

        // Display label count with encoding
        let label_count = header.labels.len();
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
        if let Some(ref creation) = header.modified_time {
            if let VsfType::e(ref et) = creation {
                println!(" {} {}", "Created".cyan(), format_et(et).white());
            }
        }
    }

    println!(
        " {} {} Bytes",
        "Header size:".cyan(),
        header_length_bytes_encoded.to_string().white()
    );

    // Integrity check (includes hash display)
    let integrity_ok = verify_integrity_summary(data, &header)?;

    println!();

    // Labels section
    println!(
        " {} labels",
        header.labels.len().to_string().yellow().bold()
    );

    // Calculate max widths
    let max_size_len = header
        .labels
        .iter()
        .map(|l| format_bytes(l.size).len())
        .max()
        .unwrap_or(0);
    let max_name_len = header
        .labels
        .iter()
        .map(|l| l.name.len())
        .max()
        .unwrap_or(0);
    let max_offset_str_len = header
        .labels
        .iter()
        .map(|l| format_number(l.offset).len())
        .max()
        .unwrap_or(0);

    for label in &header.labels {
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
            // Empty section - just show name
            print!("{}", label.name.white().bold());
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
    let has_nonempty_sections = header.labels.iter().any(|l| l.size > 0);

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
    let nonempty_labels: Vec<_> = header.labels.iter().filter(|l| l.size > 0).collect();
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

        // Show schema validation if available
        let field_prefix = if is_last { "   " } else { " │ " };
        let registry = SchemaRegistry::global();
        if let Ok(schema) = registry.get(&label.name) {
            if let Some(ref desc) = schema.description {
                println!(
                    "{}  {} {}",
                    field_prefix,
                    "Schema:".cyan(),
                    desc.truecolor(200, 200, 200)
                );
            }
            println!(
                "{}  {} {} fields defined",
                field_prefix,
                "✓".truecolor(0, 255, 0),
                schema.fields.len()
            );
        }

        // Parse and show fields (skip for n[0] unboxed blobs)
        if label.child_count == 0 {
            let field_prefix = if is_last { "   " } else { " │ " };

            // Check if this is an empty section vs opaque blob
            // Empty section: [dname] with no content after the name
            // Opaque blob: [dname <content>] with encrypted/binary data
            let section_start = label.offset;
            let section_end = section_start + label.size;
            let is_empty_section = if section_end <= data.len() {
                let section_data = &data[section_start..section_end];
                // Parse [dname] and check if ] comes right after
                let mut ptr = 0;
                if ptr < section_data.len() && section_data[ptr] == b'[' {
                    ptr += 1;
                    if parse(section_data, &mut ptr).is_ok() {
                        // After parsing dname, should be at ]
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
                    // Extract section data
                    if section_end <= data.len() {
                        let section_data = &data[section_start..section_end];

                        // Parse the section to extract the v'e' wrapper with encrypted content
                        // Structure: [ d"name" v'e'[encrypted_bytes] ]
                        let mut ptr = 0;
                        if ptr < section_data.len() && section_data[ptr] == b'[' {
                            ptr += 1;

                            // Skip section name
                            if parse(section_data, &mut ptr).is_ok() {
                                // Try to parse a v'e' wrapper
                                if let Ok(VsfType::v(b'e', encrypted_bytes)) = parse(section_data, &mut ptr) {
                                    match vsf::decrypt::decrypt_ve(&encrypted_bytes, &kp.x25519_secret) {
                                        Ok(plaintext) => {
                                            println!("{}  {} Decrypted content:", field_prefix, "🔓".green());
                                            // Parse and display decrypted VSF content
                                            let mut ptr = 0;
                                            while ptr < plaintext.len() {
                                                match parse(&plaintext, &mut ptr) {
                                                    Ok(val) => {
                                                        println!("{}    {}", field_prefix, format_value_short(&val));
                                                    }
                                                    Err(e) => {
                                                        println!("{}    <parse error: {}>", field_prefix, e);
                                                        break;
                                                    }
                                                }
                                            }
                                            Some(())
                                        }
                                        Err(e) => {
                                            println!("{}  {} Decryption failed: {}", field_prefix, "✗".red(), e);
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
                    for (j, (field_name, field_value)) in fields.iter().enumerate() {
                        let is_field_last = j == fields.len() - 1;
                        let field_prefix = if is_last { "   " } else { " │ " };
                        let field_connector = if is_field_last { "└─" } else { "├─" };
                        println!(
                            "{}{} {}: {}",
                            field_prefix,
                            field_connector,
                            field_name,
                            format_value_short(field_value)
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
/// Returns true if all integrity checks pass
fn verify_integrity_summary(data: &[u8], header: &VsfHeader) -> Result<bool, String> {
    let mut all_checks_pass = true;

    // Display provenance hash (hp) - NO verification, it's for identity/linking only
    if let Some(ref hp) = header.provenance_hash {
        match hp {
            VsfType::hp(stored_hash) => {
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
                println!(
                    " {} {}",
                    "Semantics:".cyan(),
                    "Content identifier for linking (not verified)"
                        .truecolor(200, 200, 200)
                );
            }
            _ => {}
        }
    }

    // Display signer pubkey (ke) if present
    if let Some(ref pk) = header.signer_pubkey {
        match pk {
            VsfType::ke(pk_bytes) => {
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
            _ => {}
        }
    }

    // Display optional signature (ge)
    // NOTE: We can't verify the signature here because we don't know what was signed.
    // The signed data is protocol-specific (e.g., for status pings it's the provenance hash,
    // but for other protocols it could be the file content, a subset of fields, etc.)
    // Rolling hash (hb) is the correct mechanism for file-level integrity verification.
    if let Some(ref sig) = header.signature {
        match sig {
            VsfType::ge(sig_bytes) => {
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
            _ => {}
        }
    }

    // Display and verify rolling hash (hb)
    let (file_hash_verified, stored_hash, computed_hash) = if let Some(ref hb) = header.rolling_hash
    {
        match hb {
            VsfType::hb(stored_hash) => {
                let computed =
                    vsf::verification::compute_file_hash(data).unwrap_or_else(|_| [0u8; 32]);
                let verified = computed.as_slice() == stored_hash.as_slice();
                (verified, Some(stored_hash.clone()), Some(computed.to_vec()))
            }
            _ => (false, None, None),
        }
    } else {
        (false, None, None)
    };

    // Check section-level hashes
    let mut verified_sections = 0;
    let mut total_sections = 0;

    for label in &header.labels {
        if label.child_count > 0 {
            total_sections += 1;
            // Hash is now in the label, not preamble
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
        // Show only the hash if it passes
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
        // Show both expected and computed hashes on failure
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
            "{}{} ({} Bytes, {} fields)",
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
