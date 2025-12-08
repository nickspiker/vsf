//! VSF inspection and formatting utilities
//!
//! Provides human-readable coloured formatting for VSF types, headers, and sections.
//! Used by vsfinfo CLI and can be embedded in other applications (photon, fgtw, etc.).
//!
//! Supports multiple output formats via `OutputFormat`:
//! - `Terminal` - ANSI escape codes for terminal display
//! - `Html` - CSS-styled spans for web display
//! - `Plain` - No colour codes, just text

use crate::decoding::parse::parse;
use crate::file_format::{VsfField, VsfHeader};
use crate::types::{EagleTime, EtType, VsfType};
use chrono::{Datelike, Local, Timelike};
use colored::*;

/// Wrapper that forces truecolor ANSI output regardless of COLORTERM detection
/// This is needed because the colored crate falls back to 8-color in WASM
struct Tc(String);

impl Tc {
    fn new<S: AsRef<str>>(s: S, r: u8, g: u8, b: u8) -> Self {
        Tc(format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, s.as_ref()))
    }
}

impl std::fmt::Display for Tc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Shorthand for forced truecolor - replaces .truecolor(r,g,b)
fn tc<S: AsRef<str>>(s: S, r: u8, g: u8, b: u8) -> Tc {
    Tc::new(s, r, g, b)
}

/// Output format for VSF inspection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// ANSI escape codes for terminal display
    #[default]
    Terminal,
    /// HTML with inline CSS styles
    Html,
    /// Plain text, no colour codes
    Plain,
    // Future formats:
    // Rtf,      // Rich Text Format
    // Pdf,      // PDF with colour
    // Spectral, // VSF-native spectral colour encoding
}

/// Colour roles for semantic styling
#[derive(Debug, Clone, Copy)]
pub enum Colour {
    /// Data values - brightest (white)
    Data,
    /// Type markers - cyan
    Type,
    /// Size/count values - soft yellow
    Size,
    /// Labels and descriptions - mid grey
    Label,
    /// Punctuation - dark grey
    Punct,
    /// Tree lines - darkest grey
    Tree,
    /// Success status - soft green
    Pass,
    /// Error status - soft red
    Fail,
    /// Bold variant of Data
    DataBold,
}

/// Styler for format-agnostic colour output
#[derive(Debug, Clone, Copy)]
pub struct Styler {
    format: OutputFormat,
}

impl Styler {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Apply colour to text based on output format
    pub fn style(&self, text: &str, colour: Colour) -> String {
        match self.format {
            OutputFormat::Terminal => self.terminal_style(text, colour),
            OutputFormat::Html => self.html_style(text, colour),
            OutputFormat::Plain => text.to_string(),
        }
    }

    fn terminal_style(&self, text: &str, colour: Colour) -> String {
        match colour {
            Colour::Data => text.white().to_string(),
            Colour::DataBold => text.white().bold().to_string(),
            Colour::Type => text.cyan().to_string(),
            Colour::Size => text.truecolor(200, 200, 100).to_string(),
            Colour::Label => text.truecolor(128, 128, 128).to_string(),
            Colour::Punct => text.truecolor(100, 100, 100).to_string(),
            Colour::Tree => text.truecolor(64, 64, 64).to_string(),
            Colour::Pass => text.truecolor(100, 220, 100).to_string(),
            Colour::Fail => text.truecolor(220, 100, 100).to_string(),
        }
    }

    fn html_style(&self, text: &str, colour: Colour) -> String {
        let css = match colour {
            Colour::Data => "color:#ffffff",
            Colour::DataBold => "color:#ffffff;font-weight:bold",
            Colour::Type => "color:#4ec9b0",
            Colour::Size => "color:#c8c864",
            Colour::Label => "color:#808080",
            Colour::Punct => "color:#646464",
            Colour::Tree => "color:#404040",
            Colour::Pass => "color:#64dc64",
            Colour::Fail => "color:#dc6464",
        };
        // Escape HTML entities
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        format!("<span style='{}'>{}</span>", css, escaped)
    }

    /// Tree drawing characters with appropriate styling
    pub fn tree_vert(&self) -> String {
        self.style(BOX_VERT, Colour::Tree)
    }

    pub fn tree_corner(&self) -> String {
        self.style(&format!("{}{}", BOX_CORNER, BOX_HORIZ), Colour::Tree)
    }

    pub fn tree_tee(&self) -> String {
        self.style(&format!("{}{}", BOX_TEE, BOX_HORIZ), Colour::Tree)
    }
}

/// Convert ANSI escape codes to HTML spans
/// Supports truecolor (38;2;r;g;b), basic colors, and bold
pub fn ansi_to_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    let mut chars = input.chars().peekable();
    let mut in_span = false;

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Start of escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut params = String::new();
                while let Some(&pc) = chars.peek() {
                    if pc.is_ascii_digit() || pc == ';' {
                        params.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if chars.peek() == Some(&'m') {
                    chars.next(); // consume 'm'

                    // Close any existing span
                    if in_span {
                        output.push_str("</span>");
                        in_span = false;
                    }

                    // Parse SGR parameters
                    if params == "0" || params.is_empty() {
                        // Reset - don't open new span
                    } else {
                        let css = parse_sgr_to_css(&params);
                        if !css.is_empty() {
                            output.push_str(&format!("<span style='{}'>", css));
                            in_span = true;
                        }
                    }
                }
            }
        } else {
            // Escape HTML entities
            match c {
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '&' => output.push_str("&amp;"),
                '\n' => {
                    if in_span {
                        output.push_str("</span>");
                    }
                    output.push('\n');
                    if in_span {
                        // Re-open span on new line (for pre blocks)
                        // Actually, let's not - cleaner HTML
                        in_span = false;
                    }
                }
                _ => output.push(c),
            }
        }
    }

    if in_span {
        output.push_str("</span>");
    }

    output
}

/// Parse SGR (Select Graphic Rendition) parameters to CSS
fn parse_sgr_to_css(params: &str) -> String {
    let parts: Vec<&str> = params.split(';').collect();
    let mut css = Vec::new();
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "0" => {} // Reset
            "1" => css.push("font-weight:bold".to_string()),
            "38" if i + 4 < parts.len() && parts[i + 1] == "2" => {
                // Truecolor foreground: 38;2;r;g;b
                let r = parts[i + 2];
                let g = parts[i + 3];
                let b = parts[i + 4];
                css.push(format!("color:rgb({},{},{})", r, g, b));
                i += 4;
            }
            "48" if i + 4 < parts.len() && parts[i + 1] == "2" => {
                // Truecolor background: 48;2;r;g;b
                let r = parts[i + 2];
                let g = parts[i + 3];
                let b = parts[i + 4];
                css.push(format!("background:rgb({},{},{})", r, g, b));
                i += 4;
            }
            // Basic colors (30-37 foreground, 40-47 background)
            "30" => css.push("color:#000".to_string()),
            "31" => css.push("color:#c00".to_string()),
            "32" => css.push("color:#0c0".to_string()),
            "33" => css.push("color:#cc0".to_string()),
            "34" => css.push("color:#00c".to_string()),
            "35" => css.push("color:#c0c".to_string()),
            "36" => css.push("color:#0cc".to_string()),
            "37" => css.push("color:#ccc".to_string()),
            // Bright colors (90-97)
            "90" => css.push("color:#666".to_string()),
            "91" => css.push("color:#f66".to_string()),
            "92" => css.push("color:#6f6".to_string()),
            "93" => css.push("color:#ff6".to_string()),
            "94" => css.push("color:#66f".to_string()),
            "95" => css.push("color:#f6f".to_string()),
            "96" => css.push("color:#6ff".to_string()),
            "97" => css.push("color:#fff".to_string()),
            _ => {}
        }
        i += 1;
    }

    css.join(";")
}

/// Inspect VSF and return HTML-formatted output
pub fn inspect_vsf_html(data: &[u8]) -> Result<String, String> {
    // Force colour output even when not connected to a TTY (e.g., WASM)
    colored::control::set_override(true);
    // Force truecolor mode - colored crate checks COLORTERM env var
    // std::env::set_var panics in WASM, so use a fallback check
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_var("COLORTERM", "truecolor");
    let terminal_output = inspect_vsf(data)?;
    Ok(ansi_to_html(&terminal_output))
}

/// Inspect VSF and return plain text (no colours)
pub fn inspect_vsf_plain(data: &[u8]) -> Result<String, String> {
    let terminal_output = inspect_vsf(data)?;
    Ok(strip_ansi(&terminal_output))
}

/// Inspect a standalone VSF section and return HTML
pub fn inspect_section_html(data: &[u8]) -> Result<String, String> {
    colored::control::set_override(true);
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_var("COLORTERM", "truecolor");
    let terminal_output = inspect_section(data)?;
    Ok(ansi_to_html(&terminal_output))
}

/// Strip ANSI escape codes from string
pub fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&pc) = chars.peek() {
                    if pc == 'm' {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            }
        } else {
            output.push(c);
        }
    }

    output
}

// Box-drawing characters (heavy variants for consistent stroke weight)
const BOX_VERT: &str = "┃";      // U+2503 Heavy vertical
const BOX_CORNER: &str = "┗";   // U+2517 Heavy up and right
const BOX_TEE: &str = "┣";      // U+2523 Heavy vertical and right
const BOX_HORIZ: &str = "━";    // U+2501 Heavy horizontal

// Tree drawing helpers - dark grey (64,64,64) for subtlety
fn tree_vert() -> ColoredString { BOX_VERT.truecolor(64, 64, 64) }
fn tree_corner() -> ColoredString { format!("{}{}", BOX_CORNER, BOX_HORIZ).truecolor(64, 64, 64) }
fn tree_tee() -> ColoredString { format!("{}{}", BOX_TEE, BOX_HORIZ).truecolor(64, 64, 64) }

/// Format hex data as lines of 16 bytes (32 hex chars each)
/// Returns a Vec of hex line strings for caller to join with appropriate indent
fn format_hex_lines(data: &[u8]) -> Vec<String> {
    let hex_str = hex::encode(data).to_uppercase();
    if data.len() <= 16 {
        vec![hex_str]
    } else {
        hex_str
            .as_bytes()
            .chunks(32)
            .map(|chunk| std::str::from_utf8(chunk).unwrap().to_string())
            .collect()
    }
}

/// Format hex data with newlines every 16 bytes (for header display)
fn format_hex_wrapped(data: &[u8]) -> String {
    format_hex_lines(data).join("\n")
}

/// Format crypto literal (no colours) showing first 1KB with line wrapping
/// Returns format like "hp{32}0x\nHEXLINE..." with CRYPTO_LINE_SEP markers
fn format_crypto_literal(type_name: &str, data: &[u8]) -> String {
    let max_bytes = 1024; // Show first 1KB
    let truncated = data.len() > max_bytes;
    let display_data = if truncated { &data[..max_bytes] } else { data };
    let hex_lines = format_hex_lines(display_data);
    let suffix = if truncated { "..." } else { "" };

    if hex_lines.len() == 1 && !truncated {
        format!("{}{{{}}}0x{}", type_name, data.len(), hex_lines[0])
    } else {
        format!(
            "{}{{{}}}0x{}{}{}",
            type_name,
            data.len(),
            CRYPTO_LINE_SEP,
            hex_lines.join(CRYPTO_LINE_SEP),
            suffix
        )
    }
}

/// Format crypto field with colour coding: type{size}0xHEX
/// type=cyan, size=yellow, 0x=gray, hex=white
/// For multi-line hex, lines are joined with CRYPTO_LINE_SEP marker for later replacement
const CRYPTO_LINE_SEP: &str = "\x00HEXLINE\x00";

fn format_crypto_hex(type_name: &str, data: &[u8]) -> String {
    let hex_lines = format_hex_lines(data);
    let size_str = format!("{{{}}}", data.len());

    if hex_lines.len() == 1 {
        // Single line - inline
        format!(
            "{}{}{}{}",
            type_name.cyan(),
            tc(&size_str, 200, 200, 100),
            tc("0x", 100, 100, 100),
            hex_lines[0].white()
        )
    } else {
        // Multi-line - use separator for later replacement with proper indent
        format!(
            "{}{}{}{}{}",
            type_name.cyan(),
            tc(&size_str, 200, 200, 100),
            tc("0x", 100, 100, 100),
            CRYPTO_LINE_SEP,
            hex_lines.iter().map(|l| l.white().to_string()).collect::<Vec<_>>().join(CRYPTO_LINE_SEP)
        )
    }
}

/// Format v wrapper type with colour coding: ve{size}0xHEX
/// Shows first 1KB of data with line wrapping, truncates larger data with ...
fn format_crypto_wrap(algo: u8, data: &[u8]) -> String {
    let size_str = format!("{{{}}}", data.len());
    let max_bytes = 1024; // Show first 1KB
    let truncated = data.len() > max_bytes;
    let display_data = if truncated { &data[..max_bytes] } else { data };
    let hex_lines = format_hex_lines(display_data);

    if hex_lines.len() == 1 && !truncated {
        // Single line - inline
        format!(
            "{}{}{}{}",
            format!("v{}", algo as char).cyan(),
            tc(&size_str, 200, 200, 100),
            tc("0x", 100, 100, 100),
            hex_lines[0].white()
        )
    } else {
        // Multi-line with optional truncation indicator
        let suffix = if truncated { tc("...", 100, 100, 100).to_string() } else { String::new() };
        format!(
            "{}{}{}{}{}{}",
            format!("v{}", algo as char).cyan(),
            tc(&size_str, 200, 200, 100),
            tc("0x", 100, 100, 100),
            CRYPTO_LINE_SEP,
            hex_lines.iter().map(|l| l.white().to_string()).collect::<Vec<_>>().join(CRYPTO_LINE_SEP),
            suffix
        )
    }
}

/// Get the VSF size marker for a length value
/// Returns '3' for u8, '4' for u16, '5' for u32, '6' for u64, '7' for u128
fn size_marker(len: usize) -> char {
    if len <= 255 {
        '3'
    } else if len <= 65535 {
        '4'
    } else if len <= 0xFFFFFFFF {
        '5'
    } else {
        // 64-bit length (size code '6') - any usize > 32-bit
        '6'
    }
}

/// Format a VsfType as literal VSF wire notation with colour coding
/// Shows actual encoding: type code, size marker, length/value, content
///
/// Examples:
/// - `l3{7}"message"` for an ASCII string
/// - `d3{5}"error"` for a dictionary key
/// - `u3{42}` for an unsigned int
pub fn format_value_literal(vsf: &VsfType) -> String {
    match vsf {
        // Text types: type + size_marker + {length} + "content"
        VsfType::d(s) => {
            format!(
                "{}{}{}",
                format!("d{}", size_marker(s.len())).cyan(),
                format!("{{{}}}", s.len()).truecolor(200, 200, 100),
                s.white()
            )
        }
        VsfType::l(s) => {
            format!(
                "{}{}{}",
                format!("l{}", size_marker(s.len())).cyan(),
                format!("{{{}}}", s.len()).truecolor(200, 200, 100),
                s.white()
            )
        }
        VsfType::x(s) => {
            format!(
                "{}{}{}",
                format!("x{}", size_marker(s.len())).cyan(),
                format!("{{{}}}", s.len()).truecolor(200, 200, 100),
                s.white()
            )
        }

        // Unsigned integers: type{value}
        VsfType::u0(b) => format!("{}{}", "u0".cyan(), if *b { "1" } else { "0" }.truecolor(200, 200, 100)),
        VsfType::u3(v) => format!("{}{}", "u3".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::u4(v) => format!("{}{}", "u4".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::u5(v) => format!("{}{}", "u5".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::u6(v) => format!("{}{}", "u6".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::u7(v) => format!("{}{}", "u7".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::u(v, _) => format!("{}{}", format!("u{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),

        // Signed integers: type{value}
        VsfType::i(v) => format!("{}{}", format!("i{}", size_marker(v.unsigned_abs())).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::i3(v) => format!("{}{}", "i3".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::i4(v) => format!("{}{}", "i4".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::i5(v) => format!("{}{}", "i5".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::i6(v) => format!("{}{}", "i6".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::i7(v) => format!("{}{}", "i7".cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),

        // Floats
        VsfType::f5(v) => format!("{}{}", "f5".cyan(), format!("{{{:.6}}}", v).truecolor(200, 200, 100)),
        VsfType::f6(v) => format!("{}{}", "f6".cyan(), format!("{{{:.10}}}", v).truecolor(200, 200, 100)),

        // Metadata types
        VsfType::n(v) => format!("{}{}", format!("n{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::b(v, _) => format!("{}{}", format!("b{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::o(v) => format!("{}{}", format!("o{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::z(v) => format!("{}{}", format!("z{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),
        VsfType::y(v) => format!("{}{}", format!("y{}", size_marker(*v)).cyan(), format!("{{{}}}", v).truecolor(200, 200, 100)),

        // Crypto types - use existing format
        VsfType::hp(h) => format_crypto_hex("hp", h),
        VsfType::hb(h) => format_crypto_hex("hb", h),
        VsfType::hs(h) => format_crypto_hex("hs", h),
        VsfType::ke(k) => format_crypto_hex("ke", k),
        VsfType::kx(k) => format_crypto_hex("kx", k),
        VsfType::kp(k) => format_crypto_hex("kp", k),
        VsfType::kc(k) => format_crypto_hex("kc", k),
        VsfType::ka(k) => format_crypto_hex("ka", k),
        VsfType::ge(s) => format_crypto_hex("ge", s),
        VsfType::gp(s) => format_crypto_hex("gp", s),
        VsfType::gr(s) => format_crypto_hex("gr", s),
        VsfType::v(algo, data) => format_crypto_wrap(*algo, data),

        // Tensors
        VsfType::t_u3(tensor) => {
            // Special case: 16-byte 1D tensor = IPv6 address
            if tensor.shape == vec![16] && tensor.data.len() == 16 {
                let bytes: [u8; 16] = tensor.data.as_slice().try_into().unwrap_or([0u8; 16]);
                let ipv6 = std::net::Ipv6Addr::from(bytes);
                // Format IPv6 with uppercase hex segments
                let ipv6_upper = ipv6.segments()
                    .iter()
                    .map(|s| format!("{:X}", s))
                    .collect::<Vec<_>>()
                    .join(":");
                format!("{}{{{}}}", "t_u3".cyan(), ipv6_upper.white())
            } else {
                let shape_str = tensor
                    .shape
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("×");
                format!("{}{}{}{}{}{}",
                    "t_u3".cyan(),
                    tc("[", 100, 100, 100),
                    shape_str.truecolor(200, 200, 100),
                    tc("]", 100, 100, 100),
                    tc("(", 100, 100, 100),
                    format!("{} bytes)", tensor.data.len()).truecolor(200, 200, 100)
                )
            }
        }

        // Eagle Time - show human-readable date/time (e cyan, time amber)
        VsfType::e(et) => {
            let formatted = format_eagle_time(et);
            format!("{}{}{}{}", "e".cyan(), "{".truecolor(255, 200, 120), formatted.truecolor(255, 200, 120), "}".truecolor(255, 200, 120))
        }

        // Fall back to debug for unhandled types
        _ => format!("{:?}", vsf),
    }
}

/// Section label info for display
pub struct LabelInfo {
    pub name: String,
    pub hash: Option<VsfType>,
    pub signature: Option<VsfType>,
    pub key: Option<VsfType>,
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
            // Special case: 16-byte 1D tensor = IPv6 address
            if tensor.shape == vec![16] && tensor.data.len() == 16 {
                let bytes: [u8; 16] = tensor.data.as_slice().try_into().unwrap_or([0u8; 16]);
                let ipv6 = std::net::Ipv6Addr::from(bytes);
                format!("t u3{{{}}}", ipv6)
            } else {
                // Generic tensor: t_u3{shape}(data preview)
                let shape_str = tensor
                    .shape
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("×");
                format!("t_u3[{}]({} bytes)", shape_str, tensor.data.len())
            }
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
            // Wire literal format: ef5{value} or ef6{value}
            match et {
                EtType::f5(v) => format!("ef5{{{:.2}}}", v),
                EtType::f6(v) => format!("ef6{{{:.2}}}", v),
                _ => format!("e{{{:?}}}", et),
            }
        }
        // Crypto types - show first 1KB with line wrapping
        VsfType::hp(hash) => format_crypto_literal("hp", hash),
        VsfType::hb(hash) => format_crypto_literal("hb", hash),
        VsfType::hs(hash) => format_crypto_literal("hs", hash),
        VsfType::ge(sig) => format_crypto_literal("ge", sig),
        VsfType::gp(sig) => format_crypto_literal("gp", sig),
        VsfType::gr(sig) => format_crypto_literal("gr", sig),
        VsfType::ke(key) => format_crypto_literal("ke", key),
        VsfType::kx(key) => format_crypto_literal("kx", key),
        VsfType::kp(key) => format_crypto_literal("kp", key),
        VsfType::kc(key) => format_crypto_literal("kc", key),
        VsfType::ka(key) => format_crypto_literal("ka", key),
        VsfType::ah(mac) => format_crypto_literal("ah", mac),
        VsfType::ap(mac) => format_crypto_literal("ap", mac),
        VsfType::ab(mac) => format_crypto_literal("ab", mac),
        VsfType::ac(mac) => format_crypto_literal("ac", mac),

        VsfType::v(algo, data) => format_crypto_literal(&format!("v{}", *algo as char), data),
        VsfType::d(name) => format!("d\"{}\"", name),
        VsfType::l(s) => s.clone(),
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
        // Show literal VSF notation for crypto fields with colour coding
        // type{size}0xHEX - type=cyan, size=yellow, 0x=gray, hex=white
        VsfType::hp(hash) => format_crypto_hex("hp", hash),
        VsfType::hb(hash) => format_crypto_hex("hb", hash),
        VsfType::hs(hash) => format_crypto_hex("hs", hash),
        VsfType::ke(key) => format_crypto_hex("ke", key),
        VsfType::kx(key) => format_crypto_hex("kx", key),
        VsfType::kp(key) => format_crypto_hex("kp", key),
        VsfType::kc(key) => format_crypto_hex("kc", key),
        VsfType::ka(key) => format_crypto_hex("ka", key),
        VsfType::ge(sig) => format_crypto_hex("ge", sig),
        VsfType::gp(sig) => format_crypto_hex("gp", sig),
        VsfType::gr(sig) => format_crypto_hex("gr", sig),
        VsfType::v(algo, data) => format_crypto_wrap(*algo, data),
        _ => format_value(vsf),
    }
}

/// Parse section fields and return as vec of VsfField (supports multi-value fields)
pub fn parse_section_fields(data: &[u8], label: &LabelInfo) -> Result<Vec<VsfField>, String> {
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

    // Parse fields using VsfField::parse() which handles multi-value fields
    for i in 0..label.child_count {
        if pointer >= data.len() {
            return Err(format!(
                "Unexpected end of file at field {}/{}",
                i, label.child_count
            ));
        }

        let field =
            VsfField::parse(data, &mut pointer).map_err(|e| format!("Field {}: {}", i, e))?;
        fields.push(field);
    }

    Ok(fields)
}

/// Try to parse section fields without knowing child_count - parse until ']'
/// Used for sections with wrap/signature where child_count is omitted from header
pub fn try_parse_section_fields(data: &[u8], offset: usize) -> Result<Vec<VsfField>, String> {
    let mut pointer = offset;
    let mut fields = Vec::new();

    if pointer >= data.len() {
        return Err("Offset beyond file length".into());
    }

    if data[pointer] != b'[' {
        return Err("Expected '[' at section start".into());
    }
    pointer += 1;

    // Parse section name
    let section_name_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse section name: {}", e))?;
    let _section_name = match section_name_type {
        VsfType::d(_) => {}
        _ => return Err("Expected d type for section name".into()),
    };

    // Parse fields until we hit ']' using VsfField::parse()
    while pointer < data.len() && data[pointer] != b']' {
        let field = VsfField::parse(data, &mut pointer)?;
        fields.push(field);
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
            offset: field.offset_bytes,
            size: field.size_bytes,
            child_count: field.child_count,
        })
        .collect()
}

/// Format complete VSF stream for inspection (coloured output with tree structure)
/// Returns multi-line string with header info, labels, and section tree
pub fn inspect_vsf(data: &[u8]) -> Result<String, String> {
    // Check magic number
    if data.len() < 4 {
        return Err("Data too short for Versatile Storage Format".into());
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

    // Title (not literal VSF data, so darker)
    out.push_str(&format!("{}\n", tc("Versatile Storage Format", 128, 128, 128)));
    out.push_str(&format!(
        "{} {}{} {}{}\n",
        tc(&format_bytes(data.len()), 128, 128, 128),
        tc("(", 100, 100, 100),
        tc(&format_number(data.len()), 128, 128, 128),
        tc("Bytes", 100, 100, 100),
        tc(")", 100, 100, 100)
    ));
    out.push('\n');

    // Header section marker
    out.push_str(&format!("{}\n", tc("<", 128, 128, 128)));

    // Version info
    out.push_str(&format!(
        " {} {}\n",
        tc("Version", 128, 128, 128),
        header.version.to_string().white()
    ));
    out.push_str(&format!(
        " {} {}\n",
        tc("Backward compat", 128, 128, 128),
        header.backward_compat.to_string().white()
    ));

    // Creation time
    if let VsfType::e(ref et) = header.creation_time {
        out.push_str(&format!(
            " {} {}\n",
            tc("Created", 128, 128, 128),
            format_eagle_time(et).white()
        ));
    }

    // Header size
    out.push_str(&format!(
        " {} {} Bytes\n",
        tc("Header size:", 128, 128, 128),
        header_length_bytes.to_string().white()
    ));

    // Provenance hash (full)
    if let VsfType::hp(ref hash) = header.provenance_hash {
        out.push_str(&format!(
            " {}{}{} {} {} {}\n",
            hash.len().to_string().truecolor(200, 200, 100),
            tc("-", 100, 100, 100),
            tc("Byte", 128, 128, 128),
            "BLAKE3".cyan(),
            tc("provenance hash", 128, 128, 128),
            tc("hex", 100, 100, 100)
        ));
        let hash_lines = format_hex_lines(hash);
        for line in &hash_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Signer pubkey (full, wrapped at 16 bytes)
    if let Some(VsfType::ke(ref key)) = header.signer_pubkey {
        out.push_str(&format!(
            " {}{}{} {} {} {}\n",
            key.len().to_string().truecolor(200, 200, 100),
            tc("-", 100, 100, 100),
            tc("Byte", 128, 128, 128),
            "Ed25519".cyan(),
            tc("signer pubkey", 128, 128, 128),
            tc("hex", 100, 100, 100)
        ));
        let key_lines = format_hex_lines(key);
        for line in &key_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Signature (full, wrapped at 16 bytes)
    if let Some(VsfType::ge(ref sig)) = header.signature {
        out.push_str(&format!(
            " {}{}{} {} {} {}\n",
            sig.len().to_string().truecolor(200, 200, 100),
            tc("-", 100, 100, 100),
            tc("Byte", 128, 128, 128),
            "Ed25519".cyan(),
            tc("signature", 128, 128, 128),
            tc("hex", 100, 100, 100)
        ));
        let sig_lines = format_hex_lines(sig);
        for line in &sig_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Rolling hash if present (wrapped at 16 bytes)
    if let Some(VsfType::hb(ref hash)) = header.rolling_hash {
        out.push_str(&format!(
            " {}{}{} {} {} {}\n",
            hash.len().to_string().truecolor(200, 200, 100),
            tc("-", 100, 100, 100),
            tc("Byte", 128, 128, 128),
            "BLAKE3".cyan(),
            tc("rolling hash", 128, 128, 128),
            tc("hex", 100, 100, 100)
        ));
        let hash_lines = format_hex_lines(hash);
        for line in &hash_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
        // Verify rolling hash
        if let Ok(computed) = crate::verification::compute_file_hash(data) {
            if computed.as_slice() == hash.as_slice() {
                out.push_str(&format!(
                    " {} {}\n",
                    tc("Verification:", 128, 128, 128),
                    tc("PASS", 100, 220, 100)
                ));
            } else {
                out.push_str(&format!(
                    " {} {}\n",
                    tc("Verification:", 128, 128, 128),
                    tc("FAIL", 220, 100, 100)
                ));
            }
        }
    }

    // Labels section
    out.push_str(&format!(
        " {} {}\n",
        labels.len().to_string().truecolor(200, 200, 100),
        tc("labels", 128, 128, 128)
    ));

    for label in &labels {
        let offset_str = format_number(label.offset);

        // Build crypto suffix with algorithm annotations (matching header style)
        let mut crypto_parts = Vec::new();
        if let Some(ref key) = label.key {
            // Annotate ke with algorithm name
            if let VsfType::ke(k) = key {
                crypto_parts.push(format!(
                    "{}{}{} {} {} {}",
                    k.len().to_string().truecolor(200, 200, 100),
                    tc("-", 100, 100, 100),
                    tc("Byte", 128, 128, 128),
                    "Ed25519".cyan(),
                    tc("pubkey", 128, 128, 128),
                    tc("hex", 100, 100, 100)
                ));
                let hex_lines = format_hex_lines(k);
                for line in &hex_lines {
                    crypto_parts.push(format!("    {}", line.white()));
                }
            } else {
                crypto_parts.push(format_value_short(key));
            }
        }
        if let Some(ref sig) = label.signature {
            // Annotate ge with algorithm name
            if let VsfType::ge(s) = sig {
                crypto_parts.push(format!(
                    "{}{}{} {} {} {}",
                    s.len().to_string().truecolor(200, 200, 100),
                    tc("-", 100, 100, 100),
                    tc("Byte", 128, 128, 128),
                    "Ed25519".cyan(),
                    tc("signature", 128, 128, 128),
                    tc("hex", 100, 100, 100)
                ));
                let hex_lines = format_hex_lines(s);
                for line in &hex_lines {
                    crypto_parts.push(format!("    {}", line.white()));
                }
            } else {
                crypto_parts.push(format_value_short(sig));
            }
        }
        // Field count string - try to determine actual count for signed sections
        let actual_count = if label.child_count == 0 && label.size > 0 {
            // Try to parse and count fields for signed sections
            try_parse_section_fields(data, label.offset)
                .map(|fields| fields.len())
                .ok()
        } else {
            None
        };

        // Field count: number is white (literal), "field(s)" is gray (annotation)
        let field_str = if label.size == 0 {
            String::new()
        } else if let Some(count) = actual_count {
            if count == 1 {
                format!("{} {}", "1".white(), tc("field", 128, 128, 128))
            } else {
                format!("{} {}", count.to_string().white(), tc("fields", 128, 128, 128))
            }
        } else if label.child_count == 0 {
            format!("{} {}", tc("?", 128, 128, 128), tc("fields", 128, 128, 128))
        } else if label.child_count == 1 {
            format!("{} {}", "1".white(), tc("field", 128, 128, 128))
        } else {
            format!("{} {}", label.child_count.to_string().white(), tc("fields", 128, 128, 128))
        };

        // Print label line - matches encoding order: d(name), o(offset), b(size), n(count)
        // Literal values (name, offset, size, count) are white; annotations (Bytes, field) are gray
        out.push_str(&format!(" {}", tc("(", 100, 100, 100)));
        if label.size == 0 {
            out.push_str(&format!("{}", label.name.white().bold()));
        } else {
            out.push_str(&format!("{}", label.name.white().bold()));
            out.push_str(&format!(" {}{}", tc("@", 100, 100, 100), offset_str.white()));
            out.push_str(&format!(" {} {}", label.size.to_string().white(), tc("Bytes", 128, 128, 128)));
            out.push_str(&format!(" {}", field_str));
        }
        out.push_str(&format!("{}\n", tc(")", 100, 100, 100)));

        // Print crypto fields on separate indented lines
        for part in &crypto_parts {
            out.push_str(&format!("   {}\n", part));
        }
    }

    // Check if there are any non-empty sections
    let has_nonempty_sections = labels.iter().any(|l| l.size > 0);

    if has_nonempty_sections {
        out.push_str(&format!("{}{}\n", tc(">", 64, 64, 64), tc("┓", 64, 64, 64))); // Heavy down and left
    } else {
        out.push_str(&format!("{}\n", tc(">", 64, 64, 64)));
    }

    // Show sections with tree structure (skip empty sections)
    let nonempty_labels: Vec<_> = labels.iter().filter(|l| l.size > 0).collect();
    for (i, label) in nonempty_labels.iter().enumerate() {
        let is_last = i == nonempty_labels.len() - 1;
        let connector = if is_last { format!(" {}", tree_corner()) } else { format!(" {}", tree_tee()) };

        // For sections < 1MB, just show `[` - name is already in header labels
        // For sections >= 1MB, show `[name n{count}b{size}` for navigation
        if label.size < 1024 * 1024 {
            out.push_str(&format!(
                "{}{}\n",
                connector,
                tc("[", 64, 64, 64)
            ));
        } else {
            out.push_str(&format!(
                "{}{}{} {}{}{}{}{}{}{}b{}{}\n",
                connector,
                tc("[", 64, 64, 64),
                label.name.white().bold(),
                tc("n", 128, 128, 128),
                tc("{", 100, 100, 100),
                label.child_count.to_string().truecolor(200, 200, 100),
                tc("}", 100, 100, 100),
                " ".normal(),
                tc("b", 128, 128, 128),
                tc("{", 100, 100, 100),
                label.size.to_string().truecolor(200, 200, 100),
                tc("}", 100, 100, 100)
            ));
        }

        // Parse and show fields
        // For child_count == 0 (signed/wrapped sections), try dynamic parsing
        // For child_count > 0, use the known count
        let field_prefix = if is_last { "   " } else { &format!(" {} ", tree_vert()) };

        let fields_result = if label.child_count == 0 {
            // Try to parse fields dynamically (for signed sections where count is omitted)
            try_parse_section_fields(data, label.offset)
        } else {
            parse_section_fields(data, label)
        };

        match fields_result {
            Ok(fields) if fields.is_empty() => {
                // Empty section - check if it's truly empty [name] or has unparseable content
                let section_start = label.offset;
                let section_end = section_start + label.size;
                if section_end <= data.len() && section_end > section_start {
                    let section_data = &data[section_start..section_end];
                    // Skip past [name] to see if there's content
                    let mut ptr = 0;
                    if ptr < section_data.len() && section_data[ptr] == b'[' {
                        ptr += 1;
                        if parse(section_data, &mut ptr).is_ok() {
                            // Check if immediately followed by ]
                            if ptr < section_data.len() && section_data[ptr] == b']' {
                                // Truly empty section, don't show anything
                            } else {
                                // Has content we couldn't parse - show hex dump
                                let content_data = &section_data[ptr..];
                                let hex_preview: String = content_data
                                    .iter()
                                    .take(32)
                                    .map(|b| format!("{:02X}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                let suffix = if content_data.len() > 32 { "..." } else { "" };
                                out.push_str(&format!(
                                    "{}  {}{}\n",
                                    field_prefix,
                                    hex_preview.truecolor(128, 128, 128),
                                    suffix
                                ));
                            }
                        }
                    }
                }
            }
            Ok(fields) => {
                // Show parsed fields in literal VSF notation
                for (j, field) in fields.iter().enumerate() {
                    let is_field_last = j == fields.len() - 1;
                    let field_connector = if is_field_last { tree_corner() } else { tree_tee() };
                    let continuation_bar = if is_field_last { "   " } else { &format!("{}  ", tree_vert()) };
                    let name_literal = format_value_literal(&VsfType::d(field.name.clone()));
                    let values_literal: Vec<String> = field
                        .values
                        .iter()
                        .map(|v| format_value_literal(v))
                        .collect();

                    if values_literal.len() == 1 {
                        // Single value: (name : value) - handle multi-line hex
                        let val = &values_literal[0];
                        if val.contains(CRYPTO_LINE_SEP) {
                            // Multi-line crypto value
                            let hex_indent = format!("{}{}    ", field_prefix, continuation_bar);
                            let formatted = val.replace(CRYPTO_LINE_SEP, &format!("\n{}", hex_indent));
                            out.push_str(&format!(
                                "{}{} {}{} {} {}{}\n",
                                field_prefix,
                                field_connector,
                                tc("(", 64, 64, 64),
                                name_literal,
                                tc(":", 64, 64, 64),
                                formatted,
                                tc(")", 64, 64, 64),
                            ));
                        } else {
                            out.push_str(&format!(
                                "{}{} {}{} {} {}{}\n",
                                field_prefix,
                                field_connector,
                                tc("(", 64, 64, 64),
                                name_literal,
                                tc(":", 64, 64, 64),
                                val,
                                tc(")", 64, 64, 64),
                            ));
                        }
                    } else {
                        // Multi-value: vertical layout with continuation bars
                        out.push_str(&format!(
                            "{}{} {}{}:\n",
                            field_prefix,
                            field_connector,
                            tc("(", 64, 64, 64),
                            name_literal,
                        ));
                        for (k, val) in values_literal.iter().enumerate() {
                            let is_val_last = k == values_literal.len() - 1;
                            let separator = if is_val_last { ")" } else { "," };
                            // Handle multi-line hex values
                            if val.contains(CRYPTO_LINE_SEP) {
                                let hex_indent = format!("{}{}    ", field_prefix, continuation_bar);
                                let formatted = val.replace(CRYPTO_LINE_SEP, &format!("\n{}", hex_indent));
                                out.push_str(&format!(
                                    "{}{}  {}{}\n",
                                    field_prefix,
                                    continuation_bar,
                                    formatted,
                                    separator.truecolor(64, 64, 64),
                                ));
                            } else {
                                out.push_str(&format!(
                                    "{}{}  {}{}\n",
                                    field_prefix,
                                    continuation_bar,
                                    val,
                                    separator.truecolor(64, 64, 64),
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // Couldn't parse fields - show hex dump of section content
                out.push_str(&format!("{}  <parse error: {}>\n", field_prefix, e));
                let section_start = label.offset;
                let section_end = section_start + label.size;
                if section_end <= data.len() && section_end > section_start {
                    let section_data = &data[section_start..section_end];
                    let hex_preview: String = section_data
                        .iter()
                        .take(32)
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let suffix = if section_data.len() > 32 { "..." } else { "" };
                    out.push_str(&format!(
                        "{}  {}{}\n",
                        field_prefix,
                        hex_preview.truecolor(128, 128, 128),
                        suffix
                    ));
                }
            }
        }

        out.push_str(&format!(
            "{}{}\n",
            if is_last { "   " } else { &format!(" {} ", tree_vert()) },
            tc("]", 64, 64, 64)
        ));

        if !is_last {
            out.push_str(&format!(" {}\n", tree_vert()));
        }
    }

    // Valid indicator at end
    out.push('\n');
    out.push_str(&format!("{}\n", tc("Valid", 100, 220, 100)));

    Ok(out)
}

/// Format a section fragment (starts with '[')
/// Used for inspecting VSF section bytes before they're wrapped in a file
///
/// Shows literal VSF wire notation:
/// ```text
/// [error
///   └─ (d3{7}message : l3{24}handle claimed elsewhere)
/// ]
/// ```
pub fn inspect_section(data: &[u8]) -> Result<String, String> {
    if data.is_empty() || data[0] != b'[' {
        return Err("Not a section fragment (doesn't start with '[')".into());
    }

    let mut out = String::new();
    let mut pointer = 1usize; // Skip '['

    // Parse section name first
    let section_name = match parse(data, &mut pointer) {
        Ok(VsfType::d(name)) => name,
        Ok(_) => return Err("Expected d type for section name".into()),
        Err(e) => return Err(format!("Failed to parse section name: {}", e)),
    };

    // Check for optional n{count}b{length} suffix AFTER name (large sections)
    let mut length_hint: Option<usize> = None;
    let mut count_hint: Option<usize> = None;

    if pointer < data.len() && data[pointer] == b'n' {
        match parse(data, &mut pointer) {
            Ok(VsfType::n(count)) => count_hint = Some(count),
            _ => {}
        }
        if pointer < data.len() && data[pointer] == b'b' {
            match parse(data, &mut pointer) {
                Ok(VsfType::b(len, _)) => length_hint = Some(len),
                _ => {}
            }
        }
    }

    // Build header line: [name SIZE Bytes COUNT fields
    let mut header = format!("{}{}", tc("[", 128, 128, 128), section_name.white().bold());
    if let Some(len) = length_hint {
        header.push_str(&format!(
            " {} {}",
            len.to_string().white(),
            tc("Bytes", 128, 128, 128)
        ));
    }
    if let Some(count) = count_hint {
        header.push_str(&format!(
            " {} {}",
            count.to_string().white(),
            tc(if count == 1 { "field" } else { "fields" }, 128, 128, 128)
        ));
    }
    header.push('\n');
    out.push_str(&header);

    // Collect parsed fields first to know which is last
    let mut fields: Vec<VsfField> = Vec::new();
    while pointer < data.len() && data[pointer] != b']' {
        if data[pointer] == b'(' {
            match VsfField::parse(data, &mut pointer) {
                Ok(field) => fields.push(field),
                Err(e) => {
                    out.push_str(&format!("  {} <parse error: {}>\n", tree_corner(), e));
                    break;
                }
            }
        } else {
            // Skip unexpected bytes
            pointer += 1;
        }
    }

    // Output fields with multi-line formatting
    let comma = tc(",", 128, 128, 128).to_string();
    let pipe = tc("┃", 64, 64, 64).to_string();

    for (i, field) in fields.iter().enumerate() {
        let is_last_field = i == fields.len() - 1;
        let connector = if is_last_field { tree_corner() } else { tree_tee() };
        let continuation = if is_last_field { "  ".to_string() } else { pipe.clone() };

        // First line: connector + (name : first_value
        let name_literal = format_value_literal(&VsfType::d(field.name.clone()));
        out.push_str(&format!("  {} {}{} {} ", connector, tc("(", 128, 128, 128), name_literal, tc(":", 128, 128, 128)));

        // Format each value, one per line after the first
        for (vi, val) in field.values.iter().enumerate() {
            let is_last_val = vi == field.values.len() - 1;
            let val_str = format_value_literal(val);

            if vi == 0 {
                // First value on same line as field name
                // Check if it has hex lines that need continuation
                if val_str.contains(CRYPTO_LINE_SEP) {
                    let parts: Vec<&str> = val_str.split(CRYPTO_LINE_SEP).collect();
                    out.push_str(parts[0]);
                    out.push('\n');
                    for (hi, hex_line) in parts[1..].iter().enumerate() {
                        out.push_str(&format!("  {}     {}", continuation, hex_line));
                        if hi == parts.len() - 2 {
                            // Last hex line - add comma if not last value
                            if !is_last_val {
                                out.push_str(&comma);
                            }
                        }
                        out.push('\n');
                    }
                } else {
                    out.push_str(&val_str);
                    if !is_last_val {
                        out.push_str(&comma);
                    }
                    out.push('\n');
                }
            } else {
                // Subsequent values on new lines with continuation
                if val_str.contains(CRYPTO_LINE_SEP) {
                    let parts: Vec<&str> = val_str.split(CRYPTO_LINE_SEP).collect();
                    out.push_str(&format!("  {}   {}", continuation, parts[0]));
                    out.push('\n');
                    for (hi, hex_line) in parts[1..].iter().enumerate() {
                        out.push_str(&format!("  {}     {}", continuation, hex_line));
                        if hi == parts.len() - 2 && !is_last_val {
                            out.push_str(&comma);
                        }
                        out.push('\n');
                    }
                } else {
                    out.push_str(&format!("  {}   {}", continuation, val_str));
                    if !is_last_val {
                        out.push_str(&comma);
                    }
                    out.push('\n');
                }
            }
        }

        // Close the field with )
        out.push_str(&format!("  {}  {}\n", continuation, tc(")", 128, 128, 128)));
    }

    // Closing bracket
    out.push_str(&format!("{}\n", tc("]", 64, 64, 64)));

    // Add validation line if hints were present
    if length_hint.is_some() || count_hint.is_some() {
        let actual_len = if pointer < data.len() && data[pointer] == b']' {
            pointer + 1 // Include closing ']'
        } else {
            pointer
        };

        let mut validation = String::new();
        let mut valid = true;

        if let Some(expected_len) = length_hint {
            if actual_len == expected_len {
                validation.push_str(&format!(" {}B", actual_len).truecolor(100, 220, 100).to_string());
            } else {
                validation.push_str(
                    &format!(" {}B/{}", actual_len, expected_len)
                        .truecolor(220, 100, 100)
                        .to_string(),
                );
                valid = false;
            }
        }

        if let Some(expected_count) = count_hint {
            validation.push_str(&format!(" n={}", expected_count).truecolor(128, 128, 128).to_string());
        }

        if valid {
            out.push_str(&format!("  {}\n", tc("✓", 100, 220, 100)));
        } else {
            out.push_str(&format!("  {} MISMATCH{}\n", tc("✗", 220, 100, 100), validation));
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
