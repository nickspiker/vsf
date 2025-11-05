//! VSF File Inspector - A tool for viewing, verifying, and extracting VSF file contents
//!
//! Similar to exiftool for images, vsfinfo provides detailed inspection of VSF files
//! including metadata, structure verification, and field extraction.

use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::PathBuf;
use vsf::decoding::parse::parse;
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
    file_hash: Option<VsfType>,
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
        let _header_length_bytes = match header_length_type {
            VsfType::b(bytes, _) => bytes,
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

        // Parse file hash (optional)
        let file_hash = if pointer < data.len() && data[pointer] == b'h' {
            let hash_type = parse(data, &mut pointer)
                .map_err(|e| format!("Failed to parse file hash: {}", e))?;
            Some(hash_type)
        } else {
            None
        };

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

            // Parse optional crypto fields (h, g, k, v)
            let mut hash = None;
            let mut signature = None;
            let mut key = None;
            let mut wrap = None;

            // Keep parsing crypto fields until we hit 'o' (offset)
            while pointer < data.len() && data[pointer] != b'o' && data[pointer] != b')' {
                let next_type = parse(data, &mut pointer)
                    .map_err(|e| format!("Failed to parse label crypto field: {}", e))?;
                match next_type {
                    VsfType::hb3(_) | VsfType::hb4(_) | VsfType::h23(_) | VsfType::h53(_) => {
                        hash = Some(next_type)
                    }
                    VsfType::ge3(_) | VsfType::gp3(_) | VsfType::gr4(_) => {
                        signature = Some(next_type)
                    }
                    VsfType::ke3(_)
                    | VsfType::kx3(_)
                    | VsfType::kp3(_)
                    | VsfType::kc3(_)
                    | VsfType::ka3(_) => key = Some(next_type),
                    VsfType::v(_, _) => wrap = Some(next_type),
                    _ => {
                        return Err(format!(
                            "Unexpected type in label definition, expected h/g/k/v or o"
                        ))
                    }
                }
            }

            let offset_type =
                parse(data, &mut pointer).map_err(|e| format!("Failed to parse offset: {}", e))?;
            let offset_bytes = match offset_type {
                VsfType::o(bytes) => bytes,
                _ => return Err("Expected o type for offset".to_string()),
            };

            let size_type =
                parse(data, &mut pointer).map_err(|e| format!("Failed to parse size: {}", e))?;
            let size_bytes = match size_type {
                VsfType::b(bytes, _) => bytes,
                _ => return Err("Expected b type for size".to_string()),
            };

            // Child count is optional if encrypted (has wrap field)
            let field_count = if wrap.is_some() {
                // Encrypted blobs have no child count (implied n[0])
                0
            } else {
                // Parse child count
                let field_count_type = parse(data, &mut pointer)
                    .map_err(|e| format!("Failed to parse field count: {}", e))?;
                match field_count_type {
                    VsfType::n(count) => count,
                    _ => return Err("Expected n type for field count".to_string()),
                }
            };

            if data[pointer] != b')' {
                return Err("Expected ')' after label definition".to_string());
            }
            pointer += 1;

            labels.push(LabelInfo {
                name: label_name,
                hash,
                signature,
                key,
                wrap,
                offset: offset_bytes,
                size: size_bytes,
                child_count: field_count,
            });
        }

        Ok(VsfHeader {
            version,
            backward_compat,
            file_hash,
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
fn format_et(et_ms: i64) -> String {
    // ET is milliseconds since Unix epoch (1970-01-01 00:00:00 UTC)
    let seconds = et_ms / 1000;
    let milliseconds = (et_ms % 1000).abs();

    // Calculate date/time components
    let mut days_since_epoch = seconds / 86400;
    let mut seconds_in_day = (seconds % 86400).abs();

    // Handle negative times (before epoch)
    if seconds < 0 && seconds_in_day != 0 {
        days_since_epoch -= 1;
        seconds_in_day = 86400 - seconds_in_day;
    }

    // Calculate year, month, day
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    // Handle negative years
    if days_since_epoch < 0 {
        remaining_days = -days_since_epoch;
        while remaining_days > 365 {
            let days_in_year = if is_leap_year(year - 1) { 366 } else { 365 };
            remaining_days -= days_in_year;
            year -= 1;
        }
        year -= 1;
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        remaining_days = days_in_year - remaining_days;
    } else {
        loop {
            let days_in_year = if is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }
    }

    // Find month and day
    let is_leap = is_leap_year(year);
    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    let mut day = remaining_days + 1;
    for (i, &days_in_month) in days_in_months.iter().enumerate() {
        if day <= days_in_month {
            month = i + 1;
            break;
        }
        day -= days_in_month;
    }

    // Calculate time
    let hour = (seconds_in_day / 3600) as i32;
    let minute = ((seconds_in_day % 3600) / 60) as i32;
    let second = (seconds_in_day % 60) as i32;

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

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
        VsfType::hb3(hash) | VsfType::hb4(hash) => hash,
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
            format!(
                "{}, 8-bit tensor ({} Bytes)",
                shape_str,
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
        VsfType::e(et) => {
            // Format EtType based on its variant - ET is always in seconds, convert to milliseconds
            let et_ms = match et {
                vsf::types::EtType::u(v) => (*v as i64) * 1000,
                vsf::types::EtType::u5(v) => (*v as i64) * 1000,
                vsf::types::EtType::u6(v) => (*v as i64) * 1000,
                vsf::types::EtType::u7(v) => (*v as i64) * 1000,
                vsf::types::EtType::i(v) => (*v as i64) * 1000,
                vsf::types::EtType::i5(v) => (*v as i64) * 1000,
                vsf::types::EtType::i6(v) => (*v as i64) * 1000,
                vsf::types::EtType::i7(v) => (*v as i64) * 1000,
                vsf::types::EtType::f5(v) => (*v * 1000.0) as i64,
                vsf::types::EtType::f6(v) => (*v * 1000.0) as i64,
            };
            format_et(et_ms)
        }
        VsfType::hb3(hash) => format!("hb3[BLAKE3 {} Bytes] {}...", hash.len(), hex_preview(hash)),
        VsfType::hb4(hash) => format!("hb4[BLAKE3 {} Bytes] {}...", hash.len(), hex_preview(hash)),
        VsfType::h23(hash) => format!("h23[SHA-256 {} Bytes] {}...", hash.len(), hex_preview(hash)),
        VsfType::h53(hash) => format!("h53[SHA-512 {} Bytes] {}...", hash.len(), hex_preview(hash)),

        VsfType::ge3(sig) => format!("ge3[Ed25519 {} Bytes] {}...", sig.len(), hex_preview(sig)),
        VsfType::gp3(sig) => format!(
            "gp3[ECDSA-P256 {} Bytes] {}...",
            sig.len(),
            hex_preview(sig)
        ),
        VsfType::gr4(sig) => format!("gr4[RSA-2048 {} Bytes] {}...", sig.len(), hex_preview(sig)),

        VsfType::ke3(key) => format!(
            "ke3[Ed25519 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kx3(key) => format!(
            "kx3[X25519 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kp3(key) => format!(
            "kp3[ECDSA-P256 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kc3(key) => format!(
            "kc3[ChaCha20-Poly1305 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::ka3(key) => format!(
            "ka3[AES-256-GCM key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),

        VsfType::ah3(mac) => format!(
            "ah3[HMAC-SHA256 {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::as3(mac) => format!(
            "as3[HMAC-SHA512 {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::ap3(mac) => format!("ap3[Poly1305 {} Bytes] {}...", mac.len(), hex_preview(mac)),
        VsfType::ab3(mac) => format!(
            "ab3[BLAKE3-keyed {} Bytes] {}...",
            mac.len(),
            hex_preview(mac)
        ),
        VsfType::ac3(mac) => format!("ac3[CMAC-AES {} Bytes] {}...", mac.len(), hex_preview(mac)),

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
fn show_info(data: &[u8]) -> Result<(), String> {
    let header = VsfHeader::parse(data)?;

    // Calculate actual header length by parsing
    let mut pointer = 4; // After "RÅ<"
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
    println!(
        " {} {} Bytes",
        "Header size:".cyan(),
        header_length_bytes_encoded.to_string().white()
    );

    // Integrity check (includes hash display)
    verify_integrity_summary(data, &header)?;

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
                VsfType::ge3(_) => crypto_parts.push("Signed with Ed25519".to_string()),
                VsfType::gp3(_) => crypto_parts.push("Signed with ECDSA-P256".to_string()),
                VsfType::gr4(_) => crypto_parts.push("Signed with RSA-2048".to_string()),
                _ => {}
            }
        }
        if let Some(ref _w) = label.wrap {
            if let Some(ref key) = label.key {
                match key {
                    VsfType::kc3(_) => {
                        crypto_parts.push("Encrypted with ChaCha20-Poly1305".to_string())
                    }
                    VsfType::ka3(_) => crypto_parts.push("Encrypted with AES-256-GCM".to_string()),
                    _ => {}
                }
            }
        }
        let crypto_str = if crypto_parts.is_empty() {
            String::new()
        } else {
            crypto_parts.join(", ")
        };

        // Field count string
        let field_str = if label.child_count == 0 {
            "with unknown".to_string()
        } else if label.child_count == 1 {
            "with 1 field".to_string()
        } else {
            format!("with {} fields", label.child_count)
        };

        // Print with alignment
        print!(" {}", "(".truecolor(128, 128, 128));
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
        println!("{}", ")".truecolor(128, 128, 128));
    }

    print!("{}", ">".truecolor(128, 128, 128));
    println!("{}", "┐".white());

    // Show sections with their actual structure
    for (i, label) in header.labels.iter().enumerate() {
        let is_last = i == header.labels.len() - 1;
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
            println!(
                "{}  (opaque blob - encrypted or unstructured)",
                field_prefix
            );
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
    let _ =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;

    // Parse file hash
    let file_hash_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse file hash: {}", e))?;

    let (file_hash_verified, stored_hash, computed_hash) = match file_hash_type {
        VsfType::hb3(stored_hash) | VsfType::hb4(stored_hash) => {
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

            let verified = computed.as_bytes() == stored_hash.as_slice();
            (
                verified,
                Some(stored_hash.clone()),
                Some(computed.as_bytes().to_vec()),
            )
        }
        _ => (false, None, None),
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
                    VsfType::hb3(ref bytes)
                    | VsfType::hb4(ref bytes)
                    | VsfType::h23(ref bytes)
                    | VsfType::h53(ref bytes) => bytes,
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

    // Display hash header
    if let (Some(expected), Some(_)) = (&stored_hash, &computed_hash) {
        println!(
            " {}-Byte {} file hash:",
            expected.len().to_string().white(),
            "BLAKE3".green()
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
        print!(" {} ", "Integrity check:".cyan());
        println!("{}", "PASS".truecolor(0, 255, 0));
    } else {
        // Show both expected and computed hashes on failure
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
        print!(" {} ", "Integrity check:".cyan());
        println!("{}", "FAIL".truecolor(255, 0, 0));
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

        // Check section hash (now in label, not preamble)
        if let Some(ref hash_vsf) = label.hash {
            let hash_bytes = match hash_vsf {
                VsfType::hb3(ref bytes)
                | VsfType::hb4(ref bytes)
                | VsfType::h23(ref bytes)
                | VsfType::h53(ref bytes) => bytes,
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
