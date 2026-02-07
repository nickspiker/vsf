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
#[cfg(feature = "spirix")]
use crate::decoding::toka_tree::parse_vt_toka_node;
use crate::file_format::{VsfField, VsfHeader};
#[cfg(feature = "spirix")]
use crate::types::Fill;
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
const BOX_VERT: &str = "│";      // U+2502 Light vertical
const BOX_CORNER: &str = "╰";   // U+2570 Light arc up and right
const BOX_TEE: &str = "├";      // U+251C Light vertical and right
const BOX_HORIZ: &str = "─";    // U+2500 Light horizontal

// ==================== SEMANTIC COLOUR PALETTE ====================
// Type markers are coloured by semantic category for visual grouping
//
// Data values: white (actual hex, numbers, strings - brightest)
// Hints/labels: (100, 100, 100) - descriptions, "Bytes", algorithm names
// Punctuation: (80, 80, 80) - {}, (), :, size markers (3/4/5/6/7)
// Tree/structure: (64, 64, 64) - [], >, <, box-drawing chars

/// Semantic colour for unsigned integers (u0, u3-u7, n, b, o, z, y)
const COL_UINT: (u8, u8, u8) = (100, 220, 100);  // Soft green

/// Semantic colour for signed integers (i, i3-i7)
const COL_SINT: (u8, u8, u8) = (255, 150, 200);  // Pink

/// Semantic colour for floats (f5, f6)
const COL_FLOAT: (u8, u8, u8) = (255, 220, 100);  // Amber

/// Semantic colour for complex numbers (j5, j6)
const COL_COMPLEX: (u8, u8, u8) = (255, 180, 80);  // Orange-amber

/// Semantic colour for time (e, eu6, ef5, ef6)
const COL_TIME: (u8, u8, u8) = (255, 150, 220);  // Pink-magenta

/// Semantic colour for text types (d, l, x)
const COL_TEXT: (u8, u8, u8) = (100, 200, 255);  // Light blue

/// Semantic colour for hashes (hp, hb, hs, hm, hg, hc, hk) and MACs (ah, ap, ab, ac)
const COL_HASH: (u8, u8, u8) = (100, 200, 180);  // Teal/cyan

/// Semantic colour for signatures (ge, gp, gd, gs, gf, gr)
const COL_SIG: (u8, u8, u8) = (180, 150, 255);  // Purple

/// Semantic colour for keys (ke, kx, kp, kk, kc, ka, km, kf, kl, kn, kh, kd, kb, ks)
const COL_KEY: (u8, u8, u8) = (100, 150, 255);  // Blue

/// Semantic colour for wrapped/encoded data (v, ve, vz, vr...)
const COL_WRAP: (u8, u8, u8) = (150, 130, 200);  // Muted purple

/// Semantic colour for tensors (t_*, v_*, q_*, p)
const COL_TENSOR: (u8, u8, u8) = (150, 200, 255);  // Light blue

/// Semantic colour for world coordinates (w)
const COL_WORLD: (u8, u8, u8) = (100, 180, 120);  // Earth green

/// Hints, labels, descriptions
const COL_HINT: (u8, u8, u8) = (64, 64, 64);  // Darker grey for hints

/// Punctuation: {}, (), :, size markers
const COL_PUNCT: (u8, u8, u8) = (80, 80, 80);  // Darker gray

/// Tree/structure: [], >, <, box-drawing
const COL_TREE: (u8, u8, u8) = (64, 64, 64);  // Darkest gray

/// Success/pass
const COL_PASS: (u8, u8, u8) = (0, 200, 0);  // Green

/// Error/fail
const COL_FAIL: (u8, u8, u8) = (220, 100, 100);  // Soft red

// Tree drawing helpers - darkest grey for subtlety
fn tree_vert() -> ColoredString { BOX_VERT.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2) }
fn tree_corner() -> ColoredString { BOX_CORNER.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2) }
fn tree_corner_line() -> String {
    format!("{}{}", BOX_CORNER.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2), BOX_HORIZ.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2))
}
fn tree_tee() -> ColoredString { BOX_TEE.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2) }
fn tree_tee_line() -> String {
    format!("{}{}", BOX_TEE.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2), BOX_HORIZ.truecolor(COL_TREE.0, COL_TREE.1, COL_TREE.2))
}

// Rounded box-drawing for ro* types (lighter, more subtle)
const RO_TOP: &str = "╭─";       // U+256D U+2500 Arc down-right + horizontal
const RO_MID: &str = "├─";       // U+251C U+2500 Vert + right + horizontal
const RO_BOT: &str = "╰─";       // U+2570 U+2500 Arc up-right + horizontal
const RO_VERT: &str = "│";       // U+2502 Light vertical
const RO_SPACE: &str = " ";      // Continuation indent

// Helper to format a field for ro* display: value first, label as hint
fn ro_field(value: String, label: &str) -> String {
    format!("  {} {}",
        value,
        label.truecolor(64, 64, 64))  // Darker grey (64) for hints
}

// Helper to add semantic hint for color types
fn color_hint(vsf: &VsfType) -> &'static str {
    match vsf {
        VsfType::rcr => " VSF Red",
        VsfType::rcn => " VSF Green",
        VsfType::rcb => " VSF Blue",
        VsfType::rcw => " VSF White",
        VsfType::rck => " VSF Black",
        _ => "",
    }
}

// Helper to format children with proper indentation
fn format_children(children: &[VsfType]) -> String {
    if children.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[\n");
    for child in children {
        // Format each child and indent by 4 spaces
        let child_str = format_value_literal(child);
        for line in child_str.lines() {
            result.push_str(&format!("    {}\n", line));
        }
    }
    result.push_str("  ]");
    result
}

/// Universal VSF formatter: value first, label as hint, with indentation tracking
/// This provides consistent formatting across all VSF types
fn format_vsf_universal(vsf: &VsfType, indent_level: usize, label: Option<&str>) -> String {
    let indent = "  ".repeat(indent_level);
    let hint_color = (64, 64, 64);  // Dark grey for hints

    // Get the value representation
    let value = format_value_literal(vsf);

    // Add label if provided
    let label_str = if let Some(l) = label {
        format!(" {}", l.truecolor(hint_color.0, hint_color.1, hint_color.2))
    } else {
        String::new()
    };

    format!("{}{}{}", indent, value, label_str)
}

/// Format hex data as lines of 8 bytes (16 hex chars each)
/// Returns a Vec of hex line strings for caller to join with appropriate indent
fn format_hex_lines(data: &[u8]) -> Vec<String> {
    let hex_str = hex::encode(data).to_uppercase();
    if data.len() <= 8 {
        vec![hex_str]
    } else {
        hex_str
            .as_bytes()
            .chunks(16)  // 16 hex chars = 8 bytes
            .map(|chunk| std::str::from_utf8(chunk).unwrap().to_string())
            .collect()
    }
}

/// Format hex data with newlines every 16 bytes (for header display)
fn format_hex_wrapped(data: &[u8]) -> String {
    format_hex_lines(data).join("\n")
}

/// Format crypto literal (no colours) showing first 64 bytes with line wrapping
/// Returns format like "hp{32}0x\nHEXLINE..." with CRYPTO_LINE_SEP markers
/// Large PQC keys (McEliece 512KB, Frodo 15KB) are truncated to keep logs readable
fn format_crypto_literal(type_name: &str, data: &[u8]) -> String {
    let max_bytes = 64; // Show first 64 bytes - enough for 32-byte hashes/keys
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
/// Type markers are colored by semantic category (hash=teal, sig=purple, key=blue, etc.)
/// Size shown as len-1 (wire encoding) with punctuation in dark gray
/// For multi-line hex, lines are joined with CRYPTO_LINE_SEP marker for later replacement
/// Large PQC keys (McEliece 512KB, Frodo 15KB) are truncated to 64 bytes for readable logs
const CRYPTO_LINE_SEP: &str = "\x00HEXLINE\x00";

/// Get semantic color for a crypto type based on its prefix
fn crypto_type_color(type_name: &str) -> (u8, u8, u8) {
    match type_name.chars().next() {
        Some('h') => COL_HASH,   // Hashes: hp, hb, hs, hm, hg, hc, hk
        Some('g') => COL_SIG,    // Signatures: ge, gp, gd, gs, gf, gr
        Some('k') => COL_KEY,    // Keys: ke, kx, kp, kk, kc, ka, km, kf, kl, kn, kh, kd, kb, ks
        Some('a') => COL_HASH,   // MACs: ah, ap, ab, ac (same as hashes)
        Some('v') => COL_WRAP,   // Wrapped: ve, vz, vr, etc.
        _ => COL_HINT,           // Fallback
    }
}

fn format_crypto_hex(type_name: &str, data: &[u8]) -> String {
    // Wire encoding uses len-1 for crypto types (no zero-length crypto primitives)
    let wire_len = if data.len() > 0 { data.len() - 1 } else { 0 };
    let size_str = format!("3⦉{}⦊", wire_len);
    let col = crypto_type_color(type_name);

    // Literal format: hp3⦉31⦊⦉G^0*\nHEXVALUE\n⦊
    let hex_str = hex::encode(data).to_uppercase();

    format!(
        "{}{}{}{}\n{}\n{}",
        type_name.truecolor(col.0, col.1, col.2),
        tc(&size_str, COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        "G^0*".truecolor(COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        hex_str.white(),
        tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
    )
}

/// Format v wrapper type with colour coding: ve{size}0xHEX
/// Shows first 64 bytes of data with line wrapping, truncates larger data with ...
/// Large PQC ciphertexts are truncated to keep logs readable
fn format_crypto_wrap(algo: u8, data: &[u8]) -> String {
    // Wire encoding uses len-1 for wrapped data (no zero-length wrappers)
    let wire_len = if data.len() > 0 { data.len() - 1 } else { 0 };
    // Wire format: v{algo} + "3" (length field size) + {len-1}
    let size_str = format!("3⦉{}⦊", wire_len);
    let max_bytes = 64; // Show first 64 bytes
    let truncated = data.len() > max_bytes;
    let display_data = if truncated { &data[..max_bytes] } else { data };
    let hex_lines = format_hex_lines(display_data);

    if hex_lines.len() == 1 && !truncated {
        // Single line - inline
        format!(
            "{}{}{}{}",
            format!("v{}", algo as char).truecolor(COL_WRAP.0, COL_WRAP.1, COL_WRAP.2),
            tc(&size_str, COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("0x", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            hex_lines[0].white()
        )
    } else {
        // Multi-line with optional truncation indicator
        let suffix = if truncated { tc("...", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2).to_string() } else { String::new() };
        format!(
            "{}{}{}{}{}{}",
            format!("v{}", algo as char).truecolor(COL_WRAP.0, COL_WRAP.1, COL_WRAP.2),
            tc(&size_str, COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("0x", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
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

/// Get a short hint for an opcode (e.g., "ps" -> "push")
fn opcode_hint(a: u8, b: u8) -> Option<&'static str> {
    let opcode = [a, b];
    match &opcode {
        // Stack manipulation
        b"ps" => Some("push"),
        b"pp" => Some("pop"),
        b"dp" => Some("dup"),
        b"dn" => Some("dup n"),
        b"sw" => Some("swap"),
        b"rt" => Some("rotate"),

        // Local variables
        b"la" => Some("alloc locals"),
        b"lg" => Some("get local"),
        b"ls" => Some("set local"),
        b"lt" => Some("tee local"),

        // Arithmetic
        b"ad" => Some("add"),
        b"sb" => Some("sub"),
        b"ml" => Some("mul"),
        b"dv" => Some("div"),
        b"rc" => Some("recip"),
        b"md" => Some("mod"),
        b"ng" => Some("negate"),
        b"ab" => Some("abs"),
        b"sq" => Some("sqrt"),
        b"pw" => Some("pow"),
        b"mn" => Some("min"),
        b"mx" => Some("max"),
        b"cm" => Some("clamp"),
        b"fl" => Some("floor"),
        b"cl" => Some("ceil"),
        b"rn" => Some("round"),
        b"fa" => Some("frac"),
        b"lp" => Some("lerp"),

        // Trigonometry
        b"sn" => Some("sin"),
        b"cs" => Some("cos"),
        b"tn" => Some("tan"),
        b"is" => Some("asin"),
        b"ic" => Some("acos"),
        b"ia" => Some("atan"),
        b"at" => Some("atan2"),

        // Comparison
        b"eq" => Some("equal"),
        b"ne" => Some("not equal"),
        b"lo" => Some("less than"),
        b"le" => Some("less or equal"),
        b"gt" => Some("greater than"),
        b"ge" => Some("greater or equal"),

        // Logic
        b"an" => Some("and"),
        b"or" => Some("or"),
        b"nt" => Some("not"),

        // Bitwise
        b"ba" => Some("bit and"),
        b"bo" => Some("bit or"),
        b"bx" => Some("bit xor"),
        b"bn" => Some("bit not"),

        // Type system
        b"ty" => Some("typeof"),
        b"ts" => Some("to scalar"),
        b"tu" => Some("to uint"),
        b"tx" => Some("to string"),

        // Drawing
        b"cr" => Some("clear"),
        b"fr" => Some("fill rect"),
        b"sr" => Some("stroke rect"),
        b"fc" => Some("fill circle"),
        b"so" => Some("stroke circle"),
        b"dl" => Some("draw line"),
        b"dt" => Some("draw text"),
        b"sf" => Some("set font"),

        // Colour utilities
        b"ca" => Some("rgba"),
        b"cb" => Some("rgb"),
        b"ci" => Some("colour lerp"),

        // Control flow
        b"cn" => Some("call"),
        b"cd" => Some("call indirect"),
        b"re" => Some("return"),
        b"rv" => Some("return value"),
        b"jm" => Some("jump"),
        b"ji" => Some("jump if"),
        b"jz" => Some("jump if zero"),
        b"hl" => Some("halt"),

        // Debug
        b"db" => Some("debug print"),
        b"ds" => Some("debug stack"),
        b"np" => Some("nop"),

        _ => None,
    }
}

/// Format a VsfType as literal VSF wire notation with semantic colour coding
/// Shows actual encoding: type code, size marker, length/value, content
///
/// Colour scheme by category:
/// - Text (d, l, x): COL_TEXT (light blue)
/// - Unsigned (u*): COL_UINT (soft green)
/// - Signed (i*): COL_SINT (pink)
/// - Float (f*): COL_FLOAT (amber)
/// - Complex (j*): COL_COMPLEX (orange-amber)
/// - Time (e*): COL_TIME (pink-magenta)
/// - Metadata (n, b, o, z, y): COL_UINT (soft green - they're counts/sizes)
/// - Size markers, braces, punctuation: COL_PUNCT (darker gray)
/// - Values (numbers, strings, hex): white (brightest)
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
                "{}{}{}{}{}{}",
                "d".truecolor(COL_TEXT.0, COL_TEXT.1, COL_TEXT.2),
                tc(&size_marker(s.len()).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.len().to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.white().bold()
            )
        }
        VsfType::l(s) => {
            format!(
                "{}{}{}{}{}{}",
                "l".truecolor(COL_TEXT.0, COL_TEXT.1, COL_TEXT.2),
                tc(&size_marker(s.len()).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.len().to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.white()
            )
        }
        VsfType::x(s) => {
            format!(
                "{}{}{}{}{}\"{}\"",
                "x".truecolor(COL_TEXT.0, COL_TEXT.1, COL_TEXT.2),
                tc(&size_marker(s.len()).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.len().to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                s.escape_default().to_string().white()
            )
        }

        // Unsigned integers: soft green
        VsfType::u0(b) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("0", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            if *b { "1" } else { "0" }.white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u3(v) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u4(v) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("4", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u5(v) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("5", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u6(v) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("6", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u7(v) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("7", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::u(v, _) => format!(
            "{}{}{}{}{}",
            "u".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),

        // Signed integers: pink
        VsfType::i(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc(&size_marker(v.unsigned_abs()).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::i3(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::i4(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc("4", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::i5(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc("5", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::i6(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc("6", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::i7(v) => format!(
            "{}{}{}{}{}",
            "i".truecolor(COL_SINT.0, COL_SINT.1, COL_SINT.2),
            tc("7", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),

        // Floats: amber
        VsfType::f5(v) => format!(
            "{}{}{}{}{}",
            "f".truecolor(COL_FLOAT.0, COL_FLOAT.1, COL_FLOAT.2),
            tc("5", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{:.6}", v).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::f6(v) => format!(
            "{}{}{}{}{}",
            "f".truecolor(COL_FLOAT.0, COL_FLOAT.1, COL_FLOAT.2),
            tc("6", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{:.10}", v).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),

        // Metadata types (n, b, o, z, y) - soft green like unsigned (they're counts/sizes)
        VsfType::n(v) => format!(
            "{}{}{}{}{}",
            "n".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::b(v, _) => format!(
            "{}{}{}{}{}",
            "b".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::o(v) => format!(
            "{}{}{}{}{}",
            "o".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::z(v) => format!(
            "{}{}{}{}{}",
            "z".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::y(v) => format!(
            "{}{}{}{}{}",
            "y".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),
        VsfType::L(v, _) => format!(
            "{}{}{}{}{}",
            "L".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc(&size_marker(*v).to_string(), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            v.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ),

        // Crypto types - use semantic colors via format_crypto_hex
        // Hashes (teal)
        VsfType::hp(h) => format_crypto_hex("hp", h),
        VsfType::hb(h) => format_crypto_hex("hb", h),
        VsfType::hs(h) => format_crypto_hex("hs", h),
        VsfType::hm(h) => format_crypto_hex("hm", h),
        VsfType::hg(h) => format_crypto_hex("hg", h),
        VsfType::hc(h) => format_crypto_hex("hc", h),  // SHA-3/Keccak
        VsfType::hk(h) => format_crypto_hex("hk", h),  // BLAKE2
        VsfType::hP(h) => format_crypto_hex("hP", h),  // Photon handle proof
        // Keys (blue)
        VsfType::ke(k) => format_crypto_hex("ke", k),  // Ed25519
        VsfType::kx(k) => format_crypto_hex("kx", k),  // X25519
        VsfType::kp(k) => format_crypto_hex("kp", k),  // P-curve
        VsfType::kc(k) => format_crypto_hex("kc", k),  // ChaCha20-Poly1305
        VsfType::ka(k) => format_crypto_hex("ka", k),  // AES-256-GCM
        VsfType::kk(k) => format_crypto_hex("kk", k),  // secp256k1
        VsfType::kf(k) => format_crypto_hex("kf", k),  // Frodo
        VsfType::kn(k) => format_crypto_hex("kn", k),  // NTRU
        VsfType::kl(k) => format_crypto_hex("kl", k),  // McEliece
        VsfType::kh(k) => format_crypto_hex("kh", k),  // HQC
        VsfType::kd(k) => format_crypto_hex("kd", k),  // Dilithium/ML-DSA
        VsfType::km(k) => format_crypto_hex("km", k),  // ML-KEM
        VsfType::kb(k) => format_crypto_hex("kb", k),  // BIKE
        // Shared secrets (typed by algorithm)
        VsfType::ksx(k) => format_crypto_hex("ksx", k),  // X25519 shared secret
        VsfType::ksp(k) => format_crypto_hex("ksp", k),  // P-curve shared secret
        VsfType::ksk(k) => format_crypto_hex("ksk", k),  // secp256k1 shared secret
        VsfType::ksf(k) => format_crypto_hex("ksf", k),  // Frodo shared secret
        VsfType::ksn(k) => format_crypto_hex("ksn", k),  // NTRU shared secret
        VsfType::ksl(k) => format_crypto_hex("ksl", k),  // McEliece shared secret
        VsfType::ksh(k) => format_crypto_hex("ksh", k),  // HQC shared secret
        VsfType::ksm(k) => format_crypto_hex("ksm", k),  // ML-KEM shared secret
        // Signatures (purple)
        VsfType::ge(s) => format_crypto_hex("ge", s),  // Ed25519
        VsfType::gp(s) => format_crypto_hex("gp", s),  // ECDSA-P256
        VsfType::gd(s) => format_crypto_hex("gd", s),  // Dilithium/ML-DSA
        VsfType::gs(s) => format_crypto_hex("gs", s),  // Sphincs+
        VsfType::gf(s) => format_crypto_hex("gf", s),  // Falcon
        #[allow(deprecated)]
        VsfType::gr(s) => format_crypto_hex("gr", s),  // RSA (deprecated)
        // MACs (teal, like hashes)
        VsfType::ah(m) => format_crypto_hex("ah", m),  // HMAC-SHA256
        VsfType::ap(m) => format_crypto_hex("ap", m),  // Poly1305
        VsfType::ab(m) => format_crypto_hex("ab", m),  // BLAKE3-keyed
        VsfType::ac(m) => format_crypto_hex("ac", m),  // CMAC-AES
        // Wrapped/encoded (muted purple)
        VsfType::v(algo, data) => {
            // Check if this is a Toka Tree type (algo == b't')
            #[cfg(feature = "spirix")]
            if *algo == b't' {
                // Use existing parser to decode Toka Tree structure
                if let Ok(node) = parse_vt_toka_node(&VsfType::v(*algo, data.clone())) {
                    return format!(
                        "{}{}{}{}",
                        "vt".truecolor(COL_WRAP.0, COL_WRAP.1, COL_WRAP.2),
                        tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                        tc(&format!("{{{}}}", data.len()), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                        format!(" {}", node).white()
                    );
                }
            }
            // Fallback to existing crypto_wrap formatting for other algorithms
            format_crypto_wrap(*algo, data)
        }

        // Tensors: light blue
        // Wire format: t3{dims}u3 + shape values + data
        // e.g., t3{1}u3 3{16} [38,0,16,15,...] for a 16-element 1D u8 tensor
        VsfType::t_u3(tensor) => {
            let dims = tensor.shape.len();
            // Build shape encoding: 3{dim0}3{dim1}...
            let shape_encoded: String = tensor.shape.iter()
                .map(|d| format!(
                    "{}{}{}{}",
                    tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                    tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                    d.to_string().white(),
                    tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
                ))
                .collect();

            // Show u8 values as decimal (native representation)
            let preview_len = tensor.data.len().min(32);
            let values_preview: String = tensor.data[..preview_len]
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let ellipsis = if tensor.data.len() > 32 { ",..." } else { "" };

            // Build shape hint like "16" or "24×71"
            let shape_hint = tensor.shape.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("×");

            format!(
                "{}{}{}{}{}{} {} {}{}{}{}{}",
                "t".truecolor(COL_TENSOR.0, COL_TENSOR.1, COL_TENSOR.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                dims.to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                "u3".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                shape_encoded,
                tc("[", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                values_preview.white(),
                tc(ellipsis, COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("]", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                format!(" {}", tc(&format!("{} u8 tensor", shape_hint), COL_HINT.0, COL_HINT.1, COL_HINT.2))
            )
        }

        // Eagle Time: pink-magenta - show underlying type (eu6, ef5, ef6)
        VsfType::e(et) => {
            let formatted = format_eagle_time(et);
            let type_marker = match et {
                EtType::u(_) => "eu6",
                EtType::f5(_) => "ef5",
                EtType::f6(_) => "ef6",
                _ => "e",
            };
            format!(
                "{}{}{}{}",
                type_marker.truecolor(COL_TIME.0, COL_TIME.1, COL_TIME.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                formatted.white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
            )
        }

        // Opcodes: {xx} format with optional hint
        VsfType::op(a, b) => {
            let base = format!(
                "{}{}{}{}",
                tc("{", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                char::from(*a).to_string().truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                char::from(*b).to_string().truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("}", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
            );

            // Add hint if available (without # comment marker)
            if let Some(hint) = opcode_hint(*a, *b) {
                format!(
                    "{} {}",
                    base,
                    hint.truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2)
                )
            } else {
                base
            }
        },

        // Spirix scalars (25 types: s33-s77) - scalar Display already includes ⦉value⦊
        #[cfg(feature = "spirix")]
        VsfType::s33(s) => format!("{}{}", "s33".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s34(s) => format!("{}{}", "s34".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s35(s) => format!("{}{}", "s35".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s36(s) => format!("{}{}", "s36".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s37(s) => format!("{}{}", "s37".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s43(s) => format!("{}{}", "s43".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s44(s) => format!("{}{}", "s44".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s45(s) => format!("{}{}", "s45".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s46(s) => format!("{}{}", "s46".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s47(s) => format!("{}{}", "s47".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s53(s) => format!("{}{}", "s53".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s54(s) => format!("{}{}", "s54".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s55(s) => format!("{}{}", "s55".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s56(s) => format!("{}{}", "s56".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s57(s) => format!("{}{}", "s57".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s63(s) => format!("{}{}", "s63".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s64(s) => format!("{}{}", "s64".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s65(s) => format!("{}{}", "s65".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s66(s) => format!("{}{}", "s66".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s67(s) => format!("{}{}", "s67".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s73(s) => format!("{}{}", "s73".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s74(s) => format!("{}{}", "s74".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s75(s) => format!("{}{}", "s75".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s76(s) => format!("{}{}", "s76".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),
        #[cfg(feature = "spirix")]
        VsfType::s77(s) => format!("{}{}", "s77".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), s),

        // Spirix circles (25 types: c33-c77) - circle Display already includes ⦇value⦈
        #[cfg(feature = "spirix")]
        VsfType::c33(c) => format!("{}{}", "c33".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c34(c) => format!("{}{}", "c34".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c35(c) => format!("{}{}", "c35".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c36(c) => format!("{}{}", "c36".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c37(c) => format!("{}{}", "c37".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c43(c) => format!("{}{}", "c43".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c44(c) => format!("{}{}", "c44".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c45(c) => format!("{}{}", "c45".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c46(c) => format!("{}{}", "c46".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c47(c) => format!("{}{}", "c47".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c53(c) => format!("{}{}", "c53".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c54(c) => format!("{}{}", "c54".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c55(c) => format!("{}{}", "c55".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c56(c) => format!("{}{}", "c56".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c57(c) => format!("{}{}", "c57".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c63(c) => format!("{}{}", "c63".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c64(c) => format!("{}{}", "c64".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c65(c) => format!("{}{}", "c65".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c66(c) => format!("{}{}", "c66".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c67(c) => format!("{}{}", "c67".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c73(c) => format!("{}{}", "c73".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c74(c) => format!("{}{}", "c74".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c75(c) => format!("{}{}", "c75".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c76(c) => format!("{}{}", "c76".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),
        #[cfg(feature = "spirix")]
        VsfType::c77(c) => format!("{}{}", "c77".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2), c),

        // Renderable object types (ro* - scene graph primitives)
        #[cfg(feature = "spirix")]
        VsfType::rob(pos, size, fill, stroke, children) => {
            let mut result = format!("rob {}", "rectangle".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            // Add fill with color hint
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            if !children.is_empty() {
                result.push_str(&format!("\n{}", ro_field(format_children(children), "children")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roc(center, radius, fill, stroke) => {
            let mut result = format!("roc {}", "circle".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", center), "center")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", radius), "radius")));
            // Add fill with color hint
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::ron(pos, size, children) => {
            let mut result = format!("ron {}", "container".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format_children(children), "children")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roe(center, size, fill, stroke) => {
            let mut result = format!("roe {}", "ellipse".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", center), "center")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rol(start, end, width, colour) => {
            let mut result = format!("rol {}", "line".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", start), "start")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", end), "end")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", colour), "colour")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rop(commands, fill, stroke) => {
            let mut result = format!("rop {}", "path".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", commands), "commands")));
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roo(points, width, colour, closed) => {
            let mut result = format!("roo {}", "polyline".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", points), "points")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", colour), "colour")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", closed), "closed")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::ror(controls, knots, degree, fill, stroke) => {
            let mut result = format!("ror {}", "NURBS".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", controls), "controls")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", knots), "knots")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", degree), "degree")));
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rox(points, spline_type, fill, stroke) => {
            let mut result = format!("rox {}", "spline".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", points), "points")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", spline_type), "type")));
            let fill_str = match fill {
                Fill::Solid(color) => {
                    format!("Solid({}{})",
                        match color.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        color_hint(color.as_ref()).truecolor(64, 64, 64))
                },
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", stroke), "stroke")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rot(pos, text, size, colour, font) => {
            let mut result = format!("rot {}", "text".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", text), "text")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", colour), "colour")));
            if font.is_some() {
                result.push_str(&format!("\n{}", ro_field(format!("{:?}", font), "font")));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rou(pos, size, label, variant, colour) => {
            let mut result = format!("rou {}", "button".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", label), "label")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", variant), "variant")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", colour), "colour")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roi(pos, size, handle, tint) => {
            let mut result = format!("roi {}", "image".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", handle), "handle")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", tint), "tint")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rof(pos, size, handle) => {
            let mut result = format!("rof {}", "surface".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", handle), "handle")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rom(shape, children) => {
            let mut result = format!("rom {}", "mask".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", shape), "shape")));
            result.push_str(&format!("\n{}", ro_field(format_children(children), "children")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::row(transform, children) => {
            let mut result = format!("row {}", "group".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", transform), "transform")));
            result.push_str(&format!("\n{}", ro_field(format_children(children), "children")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rog(variant, stops) => {
            let mut result = format!("rog {}", "gradient".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", variant), "variant")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", stops), "stops")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rok(width, colour, join, cap) => {
            let mut result = format!("rok {}", "stroke".truecolor(64, 64, 64));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", colour), "colour")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", join), "join")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", cap), "cap")));
            result
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
    pub wrap: Option<VsfType>, // Wrap marker (not used but kept for compatibility)
    pub offset: usize,
    pub size: usize,
    pub child_count: usize,
    pub inline_values: Vec<VsfType>, // Inline values for header-only fields
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

    // Handle timestamps that are outside chrono's representable range
    let dt_utc = match eagle_time.to_datetime_opt() {
        Some(dt) => dt,
        None => {
            // Fallback: show the raw wire encoding
            return match et {
                EtType::u(v) => format!("eu6{{{}}}", v),
                EtType::i(v) => format!("ei6{{{}}}", v),
                EtType::f5(v) => format!("ef5{{{}}}", v),
                EtType::f6(v) => format!("ef6{{{}}}", v),
            };
        }
    };
    let dt = dt_utc.with_timezone(&Local);

    // Extract milliseconds from fractional seconds
    // For integer types (u/i), oscillations are converted to seconds with picosecond precision
    // For float types (f5/f6), seconds are stored directly
    let seconds_f64 = eagle_time.to_seconds_f64();
    let milliseconds = ((seconds_f64.fract().abs() * 1000.0) as u32) % 1000;

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
        VsfType::x(s) => format!("\"{}\"", s.escape_default()),
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
                format!("t_u3{{{}}}", ipv6)
            } else {
                // Generic tensor: t_u3{shape}(data preview with first 64 bytes as hex)
                let shape_str = tensor
                    .shape
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("×");
                let preview_len = tensor.data.len().min(64);
                let hex_preview = tensor.data[..preview_len]
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join("");
                let ellipsis = if tensor.data.len() > 64 { "..." } else { "" };
                format!("t_u3[{}]({} bytes)0x{}{}", shape_str, tensor.data.len(), hex_preview, ellipsis)
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
            // Wire literal format: eu6{value}, ef5{value}, or ef6{value}
            match et {
                EtType::u(v) => format!("eu6{{{}}}", v),
                EtType::f5(v) => format!("ef5{{{:.2}}}", v),
                EtType::f6(v) => format!("ef6{{{:.2}}}", v),
                _ => format!("e{{{:?}}}", et),
            }
        }
        // Crypto types - show first 1KB with line wrapping
        VsfType::hp(hash) => format_crypto_literal("hp", hash),
        VsfType::hb(hash) => format_crypto_literal("hb", hash),
        VsfType::hs(hash) => format_crypto_literal("hs", hash),
        VsfType::hm(hash) => format_crypto_literal("hm", hash),
        VsfType::hg(hash) => format_crypto_literal("hg", hash),
        VsfType::hP(hash) => format_crypto_literal("hP", hash),
        VsfType::ge(sig) => format_crypto_literal("ge", sig),
        VsfType::gp(sig) => format_crypto_literal("gp", sig),
        VsfType::gr(sig) => format_crypto_literal("gr", sig),
        VsfType::ke(key) => format_crypto_literal("ke", key),
        VsfType::kx(key) => format_crypto_literal("kx", key),
        VsfType::kp(key) => format_crypto_literal("kp", key),
        VsfType::kc(key) => format_crypto_literal("kc", key),
        VsfType::ka(key) => format_crypto_literal("ka", key),
        // Extended key types (post-quantum, additional curves)
        VsfType::kk(key) => format_crypto_literal("kk", key),  // secp256k1
        VsfType::kf(key) => format_crypto_literal("kf", key),  // Frodo
        VsfType::kn(key) => format_crypto_literal("kn", key),  // NTRU
        VsfType::kl(key) => format_crypto_literal("kl", key),  // McEliece
        VsfType::kh(key) => format_crypto_literal("kh", key),  // HQC
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

        // Opcodes
        VsfType::op(a, b) => format!("{{{}{}}}", char::from(*a), char::from(*b)),

        // Spirix scalars - delegate to Display
        #[cfg(feature = "spirix")]
        VsfType::s33(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s34(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s35(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s36(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s37(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s43(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s44(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s45(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s46(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s47(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s53(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s54(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s55(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s56(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s57(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s63(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s64(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s65(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s66(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s67(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s73(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s74(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s75(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s76(s) => format!("{}", s),
        #[cfg(feature = "spirix")]
        VsfType::s77(s) => format!("{}", s),

        // Spirix circles - delegate to Display
        #[cfg(feature = "spirix")]
        VsfType::c33(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c34(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c35(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c36(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c37(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c43(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c44(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c45(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c46(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c47(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c53(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c54(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c55(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c56(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c57(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c63(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c64(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c65(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c66(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c67(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c73(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c74(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c75(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c76(c) => format!("{}", c),
        #[cfg(feature = "spirix")]
        VsfType::c77(c) => format!("{}", c),

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
        VsfType::x(s) if s.len() > 30 => format!("\"{}\"...", s[..27].escape_default()),
        // Show literal VSF notation for crypto fields with colour coding
        // type{size}0xHEX - type=cyan, size=yellow, 0x=gray, hex=white
        VsfType::hp(hash) => format_crypto_hex("hp", hash),
        VsfType::hb(hash) => format_crypto_hex("hb", hash),
        VsfType::hs(hash) => format_crypto_hex("hs", hash),
        VsfType::hm(hash) => format_crypto_hex("hm", hash),
        VsfType::hg(hash) => format_crypto_hex("hg", hash),
        VsfType::hP(hash) => format_crypto_hex("hP", hash),
        VsfType::ke(key) => format_crypto_hex("ke", key),
        VsfType::kx(key) => format_crypto_hex("kx", key),
        VsfType::kp(key) => format_crypto_hex("kp", key),
        VsfType::kc(key) => format_crypto_hex("kc", key),
        VsfType::ka(key) => format_crypto_hex("ka", key),
        // Extended key types (post-quantum, additional curves)
        VsfType::kk(key) => format_crypto_hex("kk", key),  // secp256k1
        VsfType::kf(key) => format_crypto_hex("kf", key),  // Frodo
        VsfType::kn(key) => format_crypto_hex("kn", key),  // NTRU
        VsfType::kl(key) => format_crypto_hex("kl", key),  // McEliece
        VsfType::kh(key) => format_crypto_hex("kh", key),  // HQC
        VsfType::ge(sig) => format_crypto_hex("ge", sig),
        VsfType::gp(sig) => format_crypto_hex("gp", sig),
        VsfType::gr(sig) => format_crypto_hex("gr", sig),
        VsfType::v(algo, data) => {
            // Check if this is a Toka Tree type (algo == b't')
            #[cfg(feature = "spirix")]
            if *algo == b't' {
                // Use existing parser to decode Toka Tree structure
                if let Ok(node) = parse_vt_toka_node(&VsfType::v(*algo, data.clone())) {
                    return format!(
                        "{}{}{}{}",
                        "vt".truecolor(COL_WRAP.0, COL_WRAP.1, COL_WRAP.2),
                        tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                        tc(&format!("{{{}}}", data.len()), COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                        format!(" {}", node).white()
                    );
                }
            }
            // Fallback to existing crypto_wrap formatting
            format_crypto_wrap(*algo, data)
        }
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

    // Skip '>' marker if present (vsfinfo display marker, not part of VSF format)
    if data[pointer] == b'>' {
        pointer += 1;
    }

    if data[pointer] != b'[' {
        return Err(format!(
            "Expected '[' at offset {}, found {:02x} ('{}')",
            pointer, data[pointer], data[pointer] as char
        ));
    }

    pointer += 1;

    // For sections <1MB, no name is present - fields start immediately with '('
    // For sections >1MB, name + n{count}b{length} are required
    if pointer < data.len() && data[pointer] != b'(' {
        // Parse section name
        let section_name_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse section name: {}", e))?;
        let _section_name = match section_name_type {
            VsfType::d(name) => name,
            _ => return Err("Expected d type for section name".to_string()),
        };

        // Skip n{count} and b{length} (required when name is present)
        let _ = parse(data, &mut pointer); // n{count}
        let _ = parse(data, &mut pointer); // b{length}
    }

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

    // For sections <1MB, no name is present - fields start immediately with '('
    // For sections >1MB, name + n{count}b{length} are required
    if pointer < data.len() && data[pointer] != b'(' {
        // Parse and skip section name
        let section_name_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse section name: {}", e))?;
        let _section_name = match section_name_type {
            VsfType::d(_) => {}
            _ => return Err("Expected d type for section name".into()),
        };

        // Skip n{count} and b{length} (required when name is present)
        let _ = parse(data, &mut pointer); // n{count}
        let _ = parse(data, &mut pointer); // b{length}
    }

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
            wrap: None,
            offset: field.offset_bytes,
            size: field.size_bytes,
            child_count: field.child_count,
            inline_values: field.inline_values.clone(),
        })
        .collect()
}

/// Format complete VSF stream for inspection (coloured output with tree structure)
/// Returns multi-line string with header info, labels, and section tree
/// Shows literal VSF encoding first (white), then descriptive hints (dark grey)
pub fn inspect_vsf(data: &[u8]) -> Result<String, String> {
    // Check magic number
    if data.len() < 4 {
        return Err("Data too short for Versatile Storage Format".into());
    }
    if &data[0..3] != "RÅ".as_bytes() || data[3] != b'<' {
        return Err("Invalid VSF magic number".into());
    }

    let (header, actual_header_size) = VsfHeader::decode(data)?;
    let labels = labels_from_header(&header);

    // Parse header length and file length
    let mut pointer = 4; // After "RÅ<"
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse version: {}", e))?;
    let _ = parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let header_length_type = parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bytes = match header_length_type {
        VsfType::b(bytes, _) => bytes,
        _ => 0,
    };
    // Parse optional file length (L field) - only present in newer files
    let file_length_bytes = if pointer < data.len() && data[pointer] == b'L' {
        let file_length_type = parse(data, &mut pointer).map_err(|e| format!("Failed to parse file length: {}", e))?;
        match file_length_type {
            VsfType::L(bytes, _) => Some(bytes),
            _ => None,
        }
    } else {
        None // No L field present
    };

    let mut out = String::new();

    // Show literal magic number with title as hint
    out.push_str("RÅ\n");
    out.push_str(&format!("< {}\n", tc("Versatile Storage Format", 64, 64, 64)));

    // Version: z3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        "z".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
        tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        header.version.to_string().white(),
        tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("version", 64, 64, 64)
    ));

    // Backward compat: y3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        "y".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
        tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        header.backward_compat.to_string().white(),
        tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("backward compat", 64, 64, 64)
    ));

    // Creation time: ef6{timestamp} - time (pink-magenta)
    if let VsfType::e(ref et) = header.creation_time {
        let tier = match et {
            crate::types::EtType::f5(_) => "5",
            crate::types::EtType::f6(_) => "6",
            _ => "?",
        };
        out.push_str(&format!(
            "  {}{}{}{}{}\n",
            "e".truecolor(COL_TIME.0, COL_TIME.1, COL_TIME.2),
            format!("f{}", tier).truecolor(COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format_eagle_time(et).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
        ));
    }

    // Header size: b3{N} Bytes - metadata/unsigned (soft green)
    let header_size_valid = header_length_bytes == actual_header_size;
    if header_size_valid {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {} {}\n",
            "b".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            header_length_bytes.to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("Header size", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            tc("Bytes", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            tc("✓", COL_PASS.0, COL_PASS.1, COL_PASS.2)
        ));
    } else {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {} {} {}\n",
            "b".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            header_length_bytes.to_string().truecolor(COL_FAIL.0, COL_FAIL.1, COL_FAIL.2),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("Header size", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            tc("Bytes", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            tc("✗ MISMATCH", COL_FAIL.0, COL_FAIL.1, COL_FAIL.2),
            format!("(actual: {})", actual_header_size).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2)
        ));
    }

    // File length: L3{N} Bytes - metadata/unsigned (soft green)
    if let Some(file_len) = file_length_bytes {
        let actual_len = data.len();
        let length_valid = file_len == actual_len;
        if length_valid {
            out.push_str(&format!(
                "  {}{}{}{}{} {} {} {}\n",
                "L".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                file_len.to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("File length", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                tc("Bytes", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                tc("✓", COL_PASS.0, COL_PASS.1, COL_PASS.2)
            ));
        } else {
            out.push_str(&format!(
                "  {}{}{}{}{} {} {} {} {}\n",
                "L".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                file_len.to_string().truecolor(COL_FAIL.0, COL_FAIL.1, COL_FAIL.2),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("File length", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                tc("Bytes", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                tc("✗ MISMATCH", COL_FAIL.0, COL_FAIL.1, COL_FAIL.2),
                format!("(actual: {})", actual_len).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2)
            ));
        }
    }

    // Provenance hash: hp3{31} (32 Bytes) - encoded as len-1
    if let VsfType::hp(ref hash) = header.provenance_hash {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {}\n",
            "hp".truecolor(COL_HASH.0, COL_HASH.1, COL_HASH.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{}", hash.len() - 1).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("BLAKE3 provenance hash", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            format!("({} Bytes)", hash.len()).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2),
        ));
        let hash_lines = format_hex_lines(hash);
        for line in &hash_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Signer pubkey: ke3{31} (32 Bytes) - encoded as len-1
    if let Some(VsfType::ke(ref key)) = header.signer_pubkey {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {}\n",
            "ke".truecolor(COL_KEY.0, COL_KEY.1, COL_KEY.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{}", key.len() - 1).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("Ed25519 signer pubkey", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            format!("({} Bytes)", key.len()).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2),
        ));
        let key_lines = format_hex_lines(key);
        for line in &key_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Signature: ge3{63} (64 Bytes) - encoded as len-1
    if let Some(VsfType::ge(ref sig)) = header.signature {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {}\n",
            "ge".truecolor(COL_SIG.0, COL_SIG.1, COL_SIG.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{}", sig.len() - 1).white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("Ed25519 signature", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            format!("({} Bytes)", sig.len()).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2),
        ));
        let sig_lines = format_hex_lines(sig);
        for line in &sig_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
    }

    // Rolling hash: hb3{31} (32 Bytes) - encoded as len-1 like other crypto types
    if let Some(VsfType::hb(ref hash)) = header.rolling_hash {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {}\n",
            "hb".truecolor(COL_HASH.0, COL_HASH.1, COL_HASH.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            format!("{}", hash.len() - 1).white(),  // len-1 encoding
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("BLAKE3 rolling hash", COL_HINT.0, COL_HINT.1, COL_HINT.2),
            format!("({} Bytes)", hash.len()).truecolor(COL_HINT.0, COL_HINT.1, COL_HINT.2),
        ));
        let hash_lines = format_hex_lines(hash);
        for line in &hash_lines {
            out.push_str(&format!("    {}\n", line.white()));
        }
        // Verify rolling hash
        if let Ok(computed) = crate::verification::compute_file_hash(data) {
            if computed.as_slice() == hash.as_slice() {
                out.push_str(&format!(
                    "  {} {}\n",
                    tc("Verification:", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                    tc("PASS", COL_PASS.0, COL_PASS.1, COL_PASS.2)
                ));
            } else {
                out.push_str(&format!(
                    "  {} {}\n",
                    tc("Verification:", COL_HINT.0, COL_HINT.1, COL_HINT.2),
                    tc("FAIL", COL_FAIL.0, COL_FAIL.1, COL_FAIL.2)
                ));
            }
        }
    }

    // Label count: n3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        "n".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
        tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        labels.len().to_string().white(),
        tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
        tc("labels", COL_HINT.0, COL_HINT.1, COL_HINT.2)
    ));

    for label in &labels {
        // Show literal VSF encoding: (d3{N}name o3{offset} b3{size} n3{count} ke3{31}... ge3{63}...)
        out.push_str(&format!("  {}", tc("(", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)));

        // d3{len}name - field name with length prefix - text (light blue)
        out.push_str(&format!(
            "{}{}{}{}{}{}\n",
            "d".truecolor(COL_TEXT.0, COL_TEXT.1, COL_TEXT.2),
            tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            label.name.len().to_string().white(),
            tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
            label.name.white().bold()
        ));

        if label.size == 0 {
            // Inline field: (d3{N}name:val1,val2,...)
            if !label.inline_values.is_empty() {
                out.push_str("    ");
                out.push_str(&format!("{}", tc(":", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)));
                for (i, val) in label.inline_values.iter().enumerate() {
                    if i > 0 {
                        out.push_str(&format!("{}", tc(",", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)));
                    }
                    out.push_str(&format!("{}", format_value_literal(val)));
                }
                out.push_str("\n");
            }
        } else {
            // Section pointer: o3{offset} b3{size} n3{count} - metadata/unsigned (soft green) - each on own line
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                "o".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                label.offset.to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
            ));
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                "b".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                label.size.to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
            ));
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                "n".truecolor(COL_UINT.0, COL_UINT.1, COL_UINT.2),
                tc("3", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                tc("⦉", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2),
                label.child_count.to_string().white(),
                tc("⦊", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)
            ));
        }

        // NOTE: ke/ge are NOT part of the label pointer - they appear in section content
        out.push_str(&format!("  {}\n", tc(")", COL_PUNCT.0, COL_PUNCT.1, COL_PUNCT.2)));
    }

    // Check if there are any non-empty sections
    let has_nonempty_sections = labels.iter().any(|l| l.size > 0);

    if has_nonempty_sections {
        out.push_str(&format!("{}{}\n", tc(">", 64, 64, 64), tc("╮", 64, 64, 64))); // Light arc down and left
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
                tc("⦉", 100, 100, 100),
                label.child_count.to_string().truecolor(200, 200, 100),
                tc("⦊", 100, 100, 100),
                " ".normal(),
                tc("b", 128, 128, 128),
                tc("⦉", 100, 100, 100),
                label.size.to_string().truecolor(200, 200, 100),
                tc("⦊", 100, 100, 100)
            ));
        }

        // Parse and show fields
        // For child_count == 0 (signed/wrapped sections), try dynamic parsing
        // For child_count > 0, use the known count
        let field_prefix = if is_last { "  " } else { &format!(" {} ", tree_vert()) };

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
                    let field_connector = if is_field_last { tree_corner_line() } else { tree_tee_line() };
                    let continuation_bar = if is_field_last { "    " } else { &format!("{}   ", tree_vert()) };
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
                                "{}{}{}{} {} {}{}\n",
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
                                "{}{}{}{} {} {}{}\n",
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
                        // Multi-value: group opcodes with following values (newline before each opcode)
                        out.push_str(&format!(
                            "{}{}{}{}:\n",
                            field_prefix,
                            field_connector,
                            tc("(", 64, 64, 64),
                            name_literal,
                        ));

                        let mut line_buffer = String::new();
                        let mut prev_was_opcode = false;
                        for (k, val_vsf) in field.values.iter().enumerate() {
                            let val = &values_literal[k];
                            let is_val_last = k == values_literal.len() - 1;

                            // Check if this value is an opcode
                            let is_opcode = matches!(val_vsf, VsfType::op(_, _));

                            // If we hit an opcode and have buffered content, flush the line
                            if is_opcode && !line_buffer.is_empty() {
                                // Indent multi-line content: first line at 6 spaces, subsequent at 6 spaces (they already have 2 from ro_field)
                                let lines: Vec<&str> = line_buffer.lines().collect();
                                for (i, line) in lines.iter().enumerate() {
                                    if i == 0 {
                                        out.push_str(&format!("      {}\n", line));
                                    } else {
                                        out.push_str(&format!("      {}\n", line));
                                    }
                                }
                                line_buffer.clear();
                            }

                            // Add value to buffer
                            // If previous value was an opcode and this isn't, add newline before this value
                            if prev_was_opcode && !is_opcode && !line_buffer.is_empty() {
                                line_buffer.push('\n');
                            }
                            line_buffer.push_str(val);
                            prev_was_opcode = is_opcode;

                            // If this is the last value, flush buffer with closing paren
                            if is_val_last {
                                // Indent multi-line content: all lines at 6 spaces (ro_field already adds 2 for nested fields)
                                let lines: Vec<&str> = line_buffer.lines().collect();
                                if lines.is_empty() {
                                    out.push_str(&format!("      {}\n", tc(")", 64, 64, 64)));
                                } else {
                                    for (i, line) in lines.iter().enumerate() {
                                        if i == lines.len() - 1 {
                                            // Last line gets closing paren
                                            out.push_str(&format!("      {}{}\n", line, tc(")", 64, 64, 64)));
                                        } else {
                                            out.push_str(&format!("      {}\n", line));
                                        }
                                    }
                                }
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
        let connector = if is_last_field { tree_corner_line() } else { tree_tee_line() };
        let continuation = if is_last_field { "   ".to_string() } else { format!("{}  ", pipe) };

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
