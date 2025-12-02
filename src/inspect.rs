//! VSF inspection and formatting utilities
//!
//! Provides human-readable colored formatting for VSF types, headers, and sections.
//! Used by vsfinfo CLI and can be embedded in other applications (photon, etc.).

use crate::decoding::parse::parse;
use crate::file_format::VsfHeader;
use crate::types::{EagleTime, EtType, VsfType};
use chrono::{Datelike, Local, Timelike};
use colored::*;

/// Section label info for display
pub struct LabelInfo {
    pub name: String,
    pub hash: Option<VsfType>,
    pub signature: Option<VsfType>,
    pub key: Option<VsfType>,
    pub wrap: Option<VsfType>,
    pub offset: usize,
    pub size: usize,
    pub child_count: usize,
}

/// Format bytes with proper units and 4 significant figures
pub fn format_bytes(bytes: usize) -> String {
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
pub fn format_number(n: usize) -> String {
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
/// Displays in local timezone (Eagle Time → UTC → Local)
pub fn format_eagle_time(et: &EtType) -> String {
    // Convert EtType to EagleTime and then to DateTime (UTC), then to local
    let eagle_time = EagleTime::new(et.clone());
    let dt_utc = eagle_time.to_datetime();
    let dt = dt_utc.with_timezone(&Local);

    // Extract milliseconds from fractional seconds if available
    let milliseconds = match et {
        EtType::f5(v) => ((v.fract().abs() * 1000.0) as u32) % 1000,
        EtType::f6(v) => ((v.fract().abs() * 1000.0) as u32) % 1000,
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

/// Generate hex preview of first N bytes (default 4)
pub fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(4)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// Format a VsfType value for human-readable display
pub fn format_value(vsf: &VsfType) -> String {
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
            let format_hint = if tensor.ndim() == 1 { " [1D vector]" } else { "" };
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
        VsfType::e(et) => format_eagle_time(et),
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
        VsfType::gp(sig) => {
            format!("gp[ECDSA-P256 {} Bytes] {}...", sig.len(), hex_preview(sig))
        }
        VsfType::gr(sig) => format!("gr[RSA {} Bytes] {}...", sig.len(), hex_preview(sig)),

        VsfType::ke(key) => format!(
            "ke[Ed25519 key {} Bytes] {}...",
            key.len(),
            hex_preview(key)
        ),
        VsfType::kx(key) => {
            format!("kx[X25519 key {} Bytes] {}...", key.len(), hex_preview(key))
        }
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
        VsfType::at(mac) => format!(
            "at[HMAC-SHA512 {} Bytes] {}...",
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
            let algo_name = match *algo {
                b'a' => "AV1",
                b'z' => "zstd",
                b'r' => "Reed-Solomon",
                b'x' => "XZ/LZMA",
                b'e' => "Encrypted",
                b'u' => "Units",
                _ => "unknown",
            };
            format!(
                "wrap[{} {} Bytes] {}",
                algo_name,
                data.len(),
                if data.is_empty() {
                    String::new()
                } else {
                    hex_preview(data)
                }
            )
        }
        VsfType::d(name) => format!("d\"{}\"", name),
        VsfType::o(offset) => format!("o[{}]", offset),
        VsfType::n(count) => format!("n[{}]", count),
        VsfType::b(size, _) => format!("b[{}]", size),
        _ => format!("{:?}", vsf),
    }
}

/// Format a VsfType value for compact display (tree view)
/// Shows full hex for crypto fields (32/64 byte hashes, keys, signatures)
pub fn format_value_short(vsf: &VsfType) -> String {
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
        // Show full hex for crypto fields (important for debugging protocols)
        VsfType::hp(hash) => format!(
            "hp[BLAKE3 {} Bytes] {}",
            hash.len(),
            hex::encode(hash).to_uppercase()
        ),
        VsfType::hb(hash) => format!(
            "hb[BLAKE3 {} Bytes] {}",
            hash.len(),
            hex::encode(hash).to_uppercase()
        ),
        VsfType::hs(hash) => format!(
            "hs[SHA-2 {} Bytes] {}",
            hash.len(),
            hex::encode(hash).to_uppercase()
        ),
        VsfType::ke(key) => format!(
            "ke[Ed25519 {} Bytes] {}",
            key.len(),
            hex::encode(key).to_uppercase()
        ),
        VsfType::kx(key) => format!(
            "kx[X25519 {} Bytes] {}",
            key.len(),
            hex::encode(key).to_uppercase()
        ),
        VsfType::kp(key) => format!(
            "kp[ECDSA-P256 {} Bytes] {}",
            key.len(),
            hex::encode(key).to_uppercase()
        ),
        VsfType::ge(sig) => format!(
            "ge[Ed25519 {} Bytes] {}",
            sig.len(),
            hex::encode(sig).to_uppercase()
        ),
        VsfType::gp(sig) => format!(
            "gp[ECDSA-P256 {} Bytes] {}",
            sig.len(),
            hex::encode(sig).to_uppercase()
        ),
        _ => format_value(vsf),
    }
}

/// Parse section fields and return as vec of (name, value) tuples
pub fn parse_section_fields(
    data: &[u8],
    label: &LabelInfo,
) -> Result<Vec<(String, VsfType)>, String> {
    let mut pointer = label.offset;
    let mut fields = Vec::new();

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

/// Convert VsfHeader fields to LabelInfo for display
pub fn labels_from_header(header: &VsfHeader) -> Vec<LabelInfo> {
    header
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
        .collect()
}

/// Format complete VSF file for inspection (colored output with tree structure)
/// Returns multi-line string with header info, labels, and section tree
pub fn inspect_vsf(data: &[u8]) -> Result<String, String> {
    // Check magic number
    if data.len() < 4 {
        return Err("Data too short for VSF file".into());
    }
    if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic number".into());
    }

    let (header, _consumed) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    // Parse header length
    let mut pointer = 4; // After "RÅ<"
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let header_length_type = parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bytes = match header_length_type {
        VsfType::b(bytes, _) => bytes,
        _ => 0,
    };

    let mut out = String::new();

    // Title
    out.push_str(&format!("{}\n", "VSF File".cyan().bold()));
    out.push_str(&format!(
        "{} ({} Bytes)\n",
        format_bytes(data.len()).yellow(),
        format_number(data.len()).truecolor(64, 50, 255)
    ));
    out.push('\n');

    // Header section marker
    out.push_str(&format!("{}\n", "<".truecolor(128, 128, 128)));

    // Version info
    out.push_str(&format!(
        " {} {}\n",
        "Version".cyan(),
        header.version.to_string().white()
    ));
    out.push_str(&format!(
        " {} {}\n",
        "Backward compat".cyan(),
        header.backward_compat.to_string().white()
    ));

    // Creation time
    if let VsfType::e(ref et) = header.creation_time {
        out.push_str(&format!(
            " {} {}\n",
            "Created".cyan(),
            format_eagle_time(et).white()
        ));
    }

    // Header size
    out.push_str(&format!(
        " {} {} Bytes\n",
        "Header size:".cyan(),
        header_length_bytes.to_string().white()
    ));

    // Provenance hash (full)
    if let VsfType::hp(ref hash) = header.provenance_hash {
        out.push_str(&format!(
            " {}-Byte {} {}:\n",
            hash.len().to_string().white(),
            "BLAKE3".green(),
            "provenance hash".cyan()
        ));
        out.push_str(&format!(
            " {} {}\n",
            "0x".truecolor(64, 50, 255),
            hex::encode(hash).to_uppercase()
        ));
    }

    // Signer pubkey (full)
    if let Some(VsfType::ke(ref key)) = header.signer_pubkey {
        out.push_str(&format!(
            " {}-Byte {} {}:\n",
            key.len().to_string().white(),
            "Ed25519".green(),
            "signer pubkey".cyan()
        ));
        out.push_str(&format!(
            " {} {}\n",
            "0x".truecolor(64, 50, 255),
            hex::encode(key).to_uppercase().magenta()
        ));
    }

    // Signature (truncated for display)
    if let Some(VsfType::ge(ref sig)) = header.signature {
        out.push_str(&format!(
            " {}-Byte {} {}:\n",
            sig.len().to_string().white(),
            "Ed25519".green(),
            "signature".cyan()
        ));
        let sig_preview = if sig.len() > 32 {
            format!("{}...", hex::encode(&sig[..32]).to_uppercase())
        } else {
            hex::encode(sig).to_uppercase()
        };
        out.push_str(&format!(
            " {} {}\n",
            "0x".truecolor(64, 50, 255),
            sig_preview.magenta()
        ));
        out.push_str(&format!(
            " {} {}\n",
            "Semantics:".cyan(),
            "Protocol-specific (signed data unknown)".truecolor(200, 200, 200)
        ));
    }

    // Rolling hash if present
    if let Some(VsfType::hb(ref hash)) = header.rolling_hash {
        out.push_str(&format!(
            " {}-Byte {} {}:\n",
            hash.len().to_string().white(),
            "BLAKE3".green(),
            "rolling hash".cyan()
        ));
        out.push_str(&format!(
            " {} {}\n",
            "0x".truecolor(64, 50, 255),
            hex::encode(hash).to_uppercase()
        ));
        // Verify rolling hash
        if let Ok(computed) = crate::verification::compute_file_hash(data) {
            if computed.as_slice() == hash.as_slice() {
                out.push_str(&format!(
                    " {} {}\n",
                    "Verification:".cyan(),
                    "PASS".truecolor(0, 255, 0)
                ));
            } else {
                out.push_str(&format!(
                    " {} {}\n",
                    "Verification:".cyan(),
                    "FAIL".truecolor(255, 0, 0)
                ));
            }
        }
    }

    out.push('\n');

    // Labels section
    out.push_str(&format!(
        " {} labels\n",
        labels.len().to_string().yellow().bold()
    ));

    // Calculate max widths for alignment
    let max_size_len = labels
        .iter()
        .map(|l| format_bytes(l.size).len())
        .max()
        .unwrap_or(0);
    let max_name_len = labels.iter().map(|l| l.name.len()).max().unwrap_or(0);
    let max_offset_len = labels
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
                VsfType::ge(_) => crypto_parts.push("Ed25519".to_string()),
                VsfType::gp(_) => crypto_parts.push("ECDSA-P256".to_string()),
                VsfType::gr(_) => crypto_parts.push("RSA".to_string()),
                _ => {}
            }
        }
        if label.wrap.is_some() {
            if let Some(ref key) = label.key {
                match key {
                    VsfType::kc(_) => crypto_parts.push("ChaCha20".to_string()),
                    VsfType::ka(_) => crypto_parts.push("AES-GCM".to_string()),
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
        let field_str = if label.size == 0 {
            String::new()
        } else if label.child_count == 0 {
            "with unknown".to_string()
        } else if label.child_count == 1 {
            "with 1 field".to_string()
        } else {
            format!("with {} fields", label.child_count)
        };

        // Print label line with alignment
        out.push_str(&format!(" {}", "(".truecolor(128, 128, 128)));
        if label.size == 0 {
            out.push_str(&format!("{}", label.name.white().bold()));
        } else {
            out.push_str(&format!("{:>width$}", size_str.bright_yellow(), width = max_size_len));
            out.push_str("      ");
            out.push_str(&format!("{:<width$}", label.name.white().bold(), width = max_name_len));
            out.push_str("    @");
            out.push_str(&format!("{:>width$}", offset_str.truecolor(64, 50, 255), width = max_offset_len));
            out.push_str("   ");
            out.push_str(&format!("{:<15}", field_str.cyan()));
            out.push_str(" ");
            out.push_str(&format!("{:<33}", crypto_str.magenta()));
        }
        out.push_str(&format!("{}\n", ")".truecolor(128, 128, 128)));
    }

    // Check if there are any non-empty sections
    let has_nonempty_sections = labels.iter().any(|l| l.size > 0);

    if has_nonempty_sections {
        out.push_str(&format!("{}{}\n", ">".truecolor(128, 128, 128), "┐".white()));
    } else {
        out.push_str(&format!("{}\n", ">".truecolor(128, 128, 128)));
    }

    // Show sections with tree structure (skip empty sections)
    let nonempty_labels: Vec<_> = labels.iter().filter(|l| l.size > 0).collect();
    for (i, label) in nonempty_labels.iter().enumerate() {
        let is_last = i == nonempty_labels.len() - 1;
        let connector = if is_last { " └─" } else { " ├─" };

        out.push_str(&format!(
            "{}{}{}\n",
            connector,
            "[".truecolor(128, 128, 128),
            label.name.bold()
        ));

        // Parse and show fields for sections with child_count > 0
        if label.child_count == 0 {
            let field_prefix = if is_last { "   " } else { " │ " };

            // Check if empty section or opaque blob
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

            if !is_empty_section {
                out.push_str(&format!(
                    "{}  (opaque blob - encrypted or unstructured)\n",
                    field_prefix
                ));
            }
        } else {
            match parse_section_fields(data, label) {
                Ok(fields) => {
                    if fields.is_empty() && label.child_count > 0 {
                        let field_prefix = if is_last { "   " } else { " │ " };
                        out.push_str(&format!(
                            "{}  <parsing error: expected {} fields>\n",
                            field_prefix, label.child_count
                        ));
                    }
                    for (j, (field_name, field_value)) in fields.iter().enumerate() {
                        let is_field_last = j == fields.len() - 1;
                        let field_prefix = if is_last { "   " } else { " │ " };
                        let field_connector = if is_field_last { "└─" } else { "├─" };
                        out.push_str(&format!(
                            "{}{} {}: {}\n",
                            field_prefix,
                            field_connector,
                            field_name,
                            format_value_short(field_value)
                        ));
                    }
                }
                Err(e) => {
                    let field_prefix = if is_last { "   " } else { " │ " };
                    out.push_str(&format!("{}  <error parsing: {}>\n", field_prefix, e));
                }
            }
        }

        out.push_str(&format!(
            "{}{}\n",
            if is_last { "   " } else { " │ " },
            "]".truecolor(128, 128, 128)
        ));

        if !is_last {
            out.push_str(" │\n");
        }
    }

    // Valid indicator at end
    out.push('\n');
    out.push_str(&format!("{}\n", "Valid".truecolor(0, 255, 0).bold()));

    Ok(out)
}

/// Format a section fragment (starts with '[')
/// Used for inspecting VSF section bytes before they're wrapped in a file
pub fn inspect_section(data: &[u8]) -> Result<String, String> {
    if data.is_empty() || data[0] != b'[' {
        return Err("Not a section fragment (doesn't start with '[')".into());
    }

    let mut out = String::new();
    let mut pointer = 1usize; // Skip '['

    // Parse section name
    let section_name = match parse(data, &mut pointer) {
        Ok(VsfType::d(name)) => name,
        Ok(_) => return Err("Expected d type for section name".into()),
        Err(e) => return Err(format!("Failed to parse section name: {}", e)),
    };

    out.push_str(&format!("[{}]\n", section_name.white().bold()));

    // Parse remaining values until ']' or end
    while pointer < data.len() && data[pointer] != b']' {
        match parse(data, &mut pointer) {
            Ok(value) => {
                out.push_str(&format!("  {}\n", format_value_short(&value)));
            }
            Err(e) => {
                out.push_str(&format!("  <parse error: {}>\n", e));
                break;
            }
        }
    }

    Ok(out)
}

/// Hex dump with ASCII sidebar (like xxd)
pub fn hex_dump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        // Offset
        out.push_str(&format!("{:08x}: ", i * 16));

        // Hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                out.push(' ');
            }
            out.push_str(&format!("{:02x} ", byte));
        }

        // Padding for incomplete lines
        let padding = 16 - chunk.len();
        for j in 0..padding {
            if chunk.len() + j == 8 {
                out.push(' ');
            }
            out.push_str("   ");
        }

        // ASCII
        out.push(' ');
        for byte in chunk {
            if *byte >= 0x20 && *byte < 0x7f {
                out.push(*byte as char);
            } else {
                out.push('.');
            }
        }
        out.push('\n');
    }
    out
}
