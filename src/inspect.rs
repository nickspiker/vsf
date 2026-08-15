//! VSF inspection and formatting utilities
//!
//! Provides human-readable coloured formatting for VSF types, headers, and sections. Used by vsfinfo CLI and can be embedded in other applications (photon, fgtw, etc.).
//!
//! Supports multiple output formats via `OutputFormat`:
//! - `Terminal` - ANSI escape codes for terminal display
//! - `Html` - CSS-styled spans for web display
//! - `Plain` - No colour codes, just text

use crate::decoding::parse::parse;
// vt hack support removed - use ro* types instead use crate::decoding::toka_tree::parse_vt_node;
use crate::file_format::{VsfField, VsfHeader};
use crate::themes::Theme;
#[cfg(feature = "spirix")]
use crate::types::Fill;
use crate::types::{EagleTime, EtType, NaScheme, VsfType};
use chrono::{Datelike, Local, Timelike};
use colored::*;

static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::dark);

/// Wrapper that forces truecolor ANSI output regardless of COLORTERM detection This is needed because the colored crate falls back to 8-colour in WASM
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

/// Shorthand for forced truecolor - accepts a theme colour tuple
fn tc<S: AsRef<str>>(s: S, (r, g, b): (u8, u8, u8)) -> Tc {
    Tc::new(s, r, g, b)
}

/// Colorize wrapper accepting a theme colour tuple (works with both &str and String)
fn trc<S: AsRef<str>>(s: S, (r, g, b): (u8, u8, u8)) -> ColoredString {
    s.as_ref().truecolor(r, g, b)
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
    // Future formats: Rtf,      // Rich Text Format Pdf,      // PDF with colour Spectral, // VSF-native spectral colour encoding
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
        let (lr, lg, lb) = THEME.label;
        let (pr, pg, pb) = THEME.comment;
        let (tr, tg, tb) = THEME.tree;
        let (pasr, pasg, pasb) = THEME.pass;
        let (failr, failg, failb) = THEME.fail;
        let (sr, sg, sb) = THEME.size;
        match colour {
            Colour::Data => text.white().to_string(),
            Colour::DataBold => text.white().bold().to_string(),
            Colour::Type => text.cyan().to_string(),
            Colour::Size => text.truecolor(sr, sg, sb).to_string(),
            Colour::Label => text.truecolor(lr, lg, lb).to_string(),
            Colour::Punct => text.truecolor(pr, pg, pb).to_string(),
            Colour::Tree => text.truecolor(tr, tg, tb).to_string(),
            Colour::Pass => text.truecolor(pasr, pasg, pasb).to_string(),
            Colour::Fail => text.truecolor(failr, failg, failb).to_string(),
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

/// Convert ANSI escape codes to HTML spans Supports truecolor (38;2;r;g;b), basic colours, and bold
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
                        // Re-open span on new line (for pre blocks) Actually, let's not - cleaner HTML
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
            // Basic colours (30-37 foreground, 40-47 background)
            "30" => css.push("color:#000".to_string()),
            "31" => css.push("color:#c00".to_string()),
            "32" => css.push("color:#0c0".to_string()),
            "33" => css.push("color:#cc0".to_string()),
            "34" => css.push("color:#00c".to_string()),
            "35" => css.push("color:#c0c".to_string()),
            "36" => css.push("color:#0cc".to_string()),
            "37" => css.push("color:#ccc".to_string()),
            // Bright colours (90-97)
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
    // Force truecolor mode - colored crate checks COLORTERM env var std::env::set_var panics in WASM, so use a fallback check
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
const BOX_VERT: &str = "│"; // U+2502 Light vertical
const BOX_CORNER: &str = "╰"; // U+2570 Light arc up and right
const BOX_TEE: &str = "├"; // U+251C Light vertical and right
const BOX_HORIZ: &str = "─"; // U+2500 Light horizontal

// ==================== SEMANTIC COLOUR PALETTE ==================== All colours are sourced from the active theme (THEME static above). To add theme support: replace `std::sync::LazyLock::new(Theme::dark)` with a runtime loader, e.g. from ~/.vsf/theme.toml or a --theme CLI flag.

fn col_uint() -> (u8, u8, u8) {
    THEME.uint
} // u0, u3-u7, n, b, o, z, y
fn col_sint() -> (u8, u8, u8) {
    THEME.sint
} // i, i3-i7
fn col_float() -> (u8, u8, u8) {
    THEME.float
} // f5, f6, s44, c44
fn col_time() -> (u8, u8, u8) {
    THEME.time
} // e, eu, ei, ef5, ef6
fn col_text() -> (u8, u8, u8) {
    THEME.text
} // d, l, x
fn col_hash() -> (u8, u8, u8) {
    THEME.crypto
} // hp, hb, hs, hm, hg, hc, hk
fn col_sig() -> (u8, u8, u8) {
    THEME.crypto
} // ge, gp, gd, gs, gf, gr
fn col_key() -> (u8, u8, u8) {
    THEME.crypto
} // ke, kx, kp, kk, kc, ka, km...
fn col_wrap() -> (u8, u8, u8) {
    THEME.binary
} // v, ve, vz, vr...
fn col_tensor() -> (u8, u8, u8) {
    THEME.binary
} // t_*, v_*, q_*, p
fn col_colour() -> (u8, u8, u8) {
    THEME.colour
} // r*, ra, rw
fn col_ro() -> (u8, u8, u8) {
    THEME.renderable
} // ro* scene graph types
fn col_hint() -> (u8, u8, u8) {
    THEME.hint
} // hints, labels, descriptions
fn col_label() -> (u8, u8, u8) {
    THEME.label
} // field names, section counts
fn col_comment() -> (u8, u8, u8) {
    THEME.comment
} // comments and annotations
fn col_punct() -> (u8, u8, u8) {
    THEME.punct
} // {}, (), :, size markers
fn col_tree() -> (u8, u8, u8) {
    THEME.tree
} // [], >, <, box-drawing
fn col_pass() -> (u8, u8, u8) {
    THEME.pass
} // success/pass
fn col_fail() -> (u8, u8, u8) {
    THEME.fail
} // error/fail
fn col_size() -> (u8, u8, u8) {
    THEME.size
} // byte sizes and counts

// Tree drawing helpers
fn tree_vert() -> ColoredString {
    let (r, g, b) = col_tree();
    BOX_VERT.truecolor(r, g, b)
}
fn tree_corner() -> ColoredString {
    let (r, g, b) = col_tree();
    BOX_CORNER.truecolor(r, g, b)
}
fn tree_corner_line() -> String {
    let (r, g, b) = col_tree();
    format!(
        "{}{}",
        BOX_CORNER.truecolor(r, g, b),
        BOX_HORIZ.truecolor(r, g, b)
    )
}
fn tree_tee() -> ColoredString {
    let (r, g, b) = col_tree();
    BOX_TEE.truecolor(r, g, b)
}
fn tree_tee_line() -> String {
    let (r, g, b) = col_tree();
    format!(
        "{}{}",
        BOX_TEE.truecolor(r, g, b),
        BOX_HORIZ.truecolor(r, g, b)
    )
}

// Rounded box-drawing for ro* types (lighter, more subtle)
#[allow(dead_code)]
const RO_TOP: &str = "╭─"; // U+256D U+2500 Arc down-right + horizontal
#[allow(dead_code)]
const RO_MID: &str = "├─"; // U+251C U+2500 Vert + right + horizontal
#[allow(dead_code)]
const RO_BOT: &str = "╰─"; // U+2570 U+2500 Arc up-right + horizontal
#[allow(dead_code)]
const RO_VERT: &str = "│"; // U+2502 Light vertical
#[allow(dead_code)]
const RO_SPACE: &str = " "; // Continuation indent

// Helper to format a field for ro* display: value first, label as hint
fn ro_field(value: String, label: &str) -> String {
    let (r, g, b) = col_hint();
    format!("  {} {}", value, label.truecolor(r, g, b))
}

// Helper to add semantic hint for colour types
fn colour_hint(vsf: &VsfType) -> &'static str {
    match vsf {
        VsfType::rcr => " VSF Red",
        VsfType::rcn => " VSF Green",
        VsfType::rcb => " VSF Blue",
        VsfType::rcw => " VSF White",
        VsfType::rck => " VSF Black",
        _ => "",
    }
}

// Helper to format Node compactly for vt types
#[cfg(feature = "spirix")]
#[allow(dead_code)]
fn format_node_compact(node: &crate::types::Node) -> String {
    use crate::types::NodeKind;

    // Format node kind name
    let kind_name = match &node.kind {
        NodeKind::Box(_) => "Box",
        NodeKind::Container(_) => "Container",
        NodeKind::Circle(_) => "Circle",
        NodeKind::Line(_) => "Line",
        NodeKind::Text(_) => "Text",
        NodeKind::Button(_) => "Button",
        NodeKind::Path(_) => "Path",
        NodeKind::Image(_) => "Image",
        NodeKind::Surface(_) => "Surface",
    };

    // Build compact representation
    let mut parts = vec![kind_name.to_string()];

    // Add transform indicators if present
    if !node.transform.is_identity() {
        let mut transform_parts = Vec::new();
        if node.transform.translate.is_some() {
            transform_parts.push("translate");
        }
        if node.transform.rotate.is_some() {
            transform_parts.push("rotate");
        }
        if node.transform.scale.is_some() {
            transform_parts.push("scale");
        }
        if node.transform.origin.is_some() {
            transform_parts.push("origin");
        }
        if !transform_parts.is_empty() {
            parts.push(format!("[{}]", transform_parts.join(" + ")));
        }
    }

    // Add child count
    match node.children.len() {
        0 => {}
        1 => parts.push("(1 child)".to_string()),
        n => parts.push(format!("({} children)", n)),
    }

    parts.join(" ")
}

// Helper to format children with proper indentation
fn format_children(children: &[VsfType]) -> String {
    if children.is_empty() {
        return format!("{}{}", trc("(", col_punct()), trc(")", col_punct()));
    }

    let mut result = String::from(format!("{}\n", trc("(", col_punct())));
    for child in children {
        // Format each child and indent by 4 spaces
        let child_str = format_value_literal(child);
        for line in child_str.lines() {
            result.push_str(&format!("    {}\n", line));
        }
    }
    result.push_str(&format!("  {}", trc(")", col_punct())));
    result
}

/// Universal VSF formatter: value first, label as hint, with indentation tracking This provides consistent formatting across all VSF types
#[allow(dead_code)]
fn format_vsf_universal(vsf: &VsfType, indent_level: usize, label: Option<&str>) -> String {
    let indent = "  ".repeat(indent_level);
    let hint_colour = (64, 64, 64); // Dark grey for hints

    // Get the value representation
    let value = format_value_literal(vsf);

    // Add label if provided
    let label_str = if let Some(l) = label {
        format!(
            " {}",
            l.truecolor(hint_colour.0, hint_colour.1, hint_colour.2)
        )
    } else {
        String::new()
    };

    format!("{}{}{}", indent, value, label_str)
}

/// Bytes per line in hex display (default 32 — future: user configurable)
const HEX_BYTES_PER_LINE: usize = 32;

/// Default bytes shown at the head/tail of a large binary blob before eliding the middle.
/// Small enough to keep logs readable (a 16+16 fingerprint distinguishes distinct payloads and confirms presence + size) yet recoverable for spot-checks. The old default was 1024/1024, which dumped up to 2KB of hex PER FIELD — readable for one packet, ruinous for whole-session logs.
const HEX_HEAD_DEFAULT: usize = 32;
const HEX_TAIL_DEFAULT: usize = 32;

/// Runtime-configurable head/tail elision lengths. This is the "local settings" knob: set it once at startup (a consumer can read its own config file and call [`set_hex_elision`]) or override per-run with the `VSF_HEX_HEAD` / `VSF_HEX_TAIL` environment variables — no recompile needed.
/// 0/0 prints nothing but the notice; a huge value effectively disables elision.
static HEX_ELISION: std::sync::OnceLock<(usize, usize)> = std::sync::OnceLock::new();

/// Set the hex head/tail elision lengths programmatically. First writer wins (it's a OnceLock), so call this before any inspect output if you want it to take effect. Ignored if elision was already initialized (e.g. a prior inspect call read the env defaults).
pub fn set_hex_elision(head: usize, tail: usize) {
    let _ = HEX_ELISION.set((head, tail));
}

fn hex_elision() -> (usize, usize) {
    *HEX_ELISION.get_or_init(|| {
        let env = |k: &str, d: usize| {
            std::env::var(k)
                .ok()
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(d)
        };
        (
            env("VSF_HEX_HEAD", HEX_HEAD_DEFAULT),
            env("VSF_HEX_TAIL", HEX_TAIL_DEFAULT),
        )
    })
}

/// Format hex data as lines of HEX_BYTES_PER_LINE bytes each Returns a Vec of hex line strings for caller to join with appropriate separator
fn format_hex_lines(data: &[u8]) -> Vec<String> {
    let hex_str = hex::encode(data).to_uppercase();
    let chars_per_line = HEX_BYTES_PER_LINE * 2;
    if data.len() <= HEX_BYTES_PER_LINE {
        vec![hex_str]
    } else {
        hex_str
            .as_bytes()
            .chunks(chars_per_line)
            .map(|chunk| std::str::from_utf8(chunk).unwrap().to_string())
            .collect()
    }
}

/// Format binary data with head+tail truncation and omission notice Returns (head_lines, omitted_notice, tail_lines)
fn format_hex_head_tail(data: &[u8]) -> (Vec<String>, Option<String>, Vec<String>) {
    let total = data.len();
    let (max_head, max_tail) = hex_elision();
    if total <= max_head + max_tail {
        (format_hex_lines(data), None, vec![])
    } else {
        let head = format_hex_lines(&data[..max_head]);
        let omitted = total - max_head - max_tail;
        let notice = format!("... {} bytes omitted ...", omitted);
        let tail = format_hex_lines(&data[total - max_tail..]);
        (head, Some(notice), tail)
    }
}

/// Format crypto literal (no colours) with head+tail truncation
fn format_crypto_literal(type_name: &str, data: &[u8]) -> String {
    let (head, notice, tail) = format_hex_head_tail(data);
    if head.len() == 1 && notice.is_none() {
        format!("{}{{{}}}0x{}", type_name, data.len(), head[0])
    } else {
        let mut parts: Vec<String> = head.into_iter().map(|l| format!("        {}", l)).collect();
        if let Some(n) = notice {
            parts.push(format!("        {}", n));
            parts.extend(tail.into_iter().map(|l| format!("        {}", l)));
        }
        format!("{}{{{}}}0x\n{}", type_name, data.len(), parts.join("\n"))
    }
}

/// Format crypto field with colour coding: type{size}0xHEX Type markers are coloured by semantic category (hash=teal, sig=purple, key=blue, etc.) Size shown as len-1 (wire encoding) with punctuation in dark gray Large PQC keys (McEliece 512KB, Frodo 15KB) are truncated to 64 bytes for readable logs

/// Get semantic colour for a crypto type based on its prefix
fn crypto_type_colour(type_name: &str) -> (u8, u8, u8) {
    match type_name.chars().next() {
        Some('h') => col_hash(), // Hashes: hp, hb, hs, hm, hg, hc, hk
        Some('g') => col_sig(),  // Signatures: ge, gp, gd, gs, gf, gr
        Some('k') => col_key(),  // Keys: ke, kx, kp, kk, kc, ka, km, kf, kl, kn, kh, kd, kb, ks
        Some('m') => col_hash(), // MACs: mh, mp, mb, mc (same colour as hashes)
        Some('v') => col_wrap(), // Wrapped: ve, vz, vr, etc.
        _ => col_hint(),         // Fallback
    }
}

fn format_crypto_hex(type_name: &str, data: &[u8]) -> String {
    let wire_len = if data.len() > 0 { data.len() - 1 } else { 0 };
    let size_str = format!("3⦉{}⦊", wire_len);
    let col = crypto_type_colour(type_name);

    let (head, notice, tail) = format_hex_head_tail(data);
    let mut lines: Vec<String> = head.iter().map(|l| l.white().to_string()).collect();
    if let Some(n) = notice {
        lines.push(n.dimmed().to_string());
        lines.extend(tail.iter().map(|l| l.white().to_string()));
    }

    format!(
        "{}{}{}{}\n{}\n{}",
        type_name.truecolor(col.0, col.1, col.2),
        tc(&size_str, col_punct()),
        tc("⦉", col_punct()),
        trc("G^0*", col_punct()),
        lines.join("\n"),
        tc("⦊", col_punct())
    )
}

/// Format v wrapper type with colour coding: v{algo}3⦉size⦊0xHEX
fn format_crypto_wrap(algo: u8, data: &[u8]) -> String {
    let wire_len = if data.len() > 0 { data.len() - 1 } else { 0 };
    let size_str = format!("3⦉{}⦊", wire_len);

    let (head, notice, tail) = format_hex_head_tail(data);
    if head.len() == 1 && notice.is_none() {
        format!(
            "{}{}{}{}",
            trc(format!("v{}", algo as char), col_wrap()),
            tc(&size_str, col_punct()),
            tc("0x", col_punct()),
            head[0].white()
        )
    } else {
        let mut lines: Vec<String> = head
            .iter()
            .map(|l| format!("        {}", l.white()))
            .collect();
        if let Some(n) = notice {
            lines.push(format!("        {}", n.dimmed()));
            lines.extend(tail.iter().map(|l| format!("        {}", l.white())));
        }
        format!(
            "{}{}{}\n{}",
            trc(format!("v{}", algo as char), col_wrap()),
            tc(&size_str, col_punct()),
            tc("0x", col_punct()),
            lines.join("\n")
        )
    }
}

/// Get the VSF size marker for a length value Returns '3' for u8, '4' for u16, '5' for u32, '6' for u64, '7' for u128
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

/// Format a VsfType as literal VSF wire notation with semantic colour coding Shows actual encoding: type code, size marker, length/value, content
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
                trc("d", col_text()),
                tc(&size_marker(s.len()).to_string(), col_punct()),
                tc("⦉", col_punct()),
                s.len().to_string().white(),
                tc("⦊", col_punct()),
                s.white().bold()
            )
        }
        VsfType::a(s) => {
            format!(
                "{}{}{}{}{}{}",
                trc("a", col_text()),
                tc(&size_marker(s.len()).to_string(), col_punct()),
                tc("⦉", col_punct()),
                s.len().to_string().white(),
                tc("⦊", col_punct()),
                s.white()
            )
        }
        VsfType::x(s) => {
            format!(
                "{}{}{}{}{}\"{}\"",
                trc("x", col_text()),
                tc(&size_marker(s.len()).to_string(), col_punct()),
                tc("⦉", col_punct()),
                s.len().to_string().white(),
                tc("⦊", col_punct()),
                display_x_value(s).white()
            )
        }

        // Unsigned integers: soft green
        VsfType::u0(b) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("0", col_punct()),
            tc("⦉", col_punct()),
            if *b { "1" } else { "0" }.white(),
            tc("⦊", col_punct())
        ),
        VsfType::u3(v) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::u4(v) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("4", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::u5(v) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("5", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::u6(v) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("6", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::u7(v) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc("7", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::u(v, _) => format!(
            "{}{}{}{}{}",
            trc("u", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),

        // Signed integers: pink
        VsfType::i(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc(&size_marker(v.unsigned_abs()).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::i3(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::i4(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc("4", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::i5(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc("5", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::i6(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc("6", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::i7(v) => format!(
            "{}{}{}{}{}",
            trc("i", col_sint()),
            tc("7", col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),

        // Floats: amber
        VsfType::f5(v) => format!(
            "{}{}{}{}{}",
            trc("f", col_float()),
            tc("5", col_punct()),
            tc("⦉", col_punct()),
            format!("{:.6}", v).white(),
            tc("⦊", col_punct())
        ),
        VsfType::f6(v) => format!(
            "{}{}{}{}{}",
            trc("f", col_float()),
            tc("6", col_punct()),
            tc("⦉", col_punct()),
            format!("{:.10}", v).white(),
            tc("⦊", col_punct())
        ),

        // Metadata types (n, b, o, z, y) - soft green like unsigned (they're counts/sizes)
        VsfType::n(v) => format!(
            "{}{}{}{}{}",
            trc("n", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::b(v, _) => format!(
            "{}{}{}{}{}",
            trc("b", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::o(v) => format!(
            "{}{}{}{}{}",
            trc("o", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::z(v) => format!(
            "{}{}{}{}{}",
            trc("z", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::y(v) => format!(
            "{}{}{}{}{}",
            trc("y", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),
        VsfType::l(v, _) => format!(
            "{}{}{}{}{}",
            trc("l", col_uint()),
            tc(&size_marker(*v).to_string(), col_punct()),
            tc("⦉", col_punct()),
            v.to_string().white(),
            tc("⦊", col_punct())
        ),

        // Crypto types - use semantic colours via format_crypto_hex Hashes (teal)
        VsfType::hp(h) => format_crypto_hex("hp", h),
        VsfType::hb(h) => format_crypto_hex("hb", h),
        VsfType::hs(h) => format_crypto_hex("hs", h),
        VsfType::hm(h) => format_crypto_hex("hm", h),
        VsfType::hg(h) => format_crypto_hex("hg", h),
        VsfType::hc(h) => format_crypto_hex("hc", h), // SHA-3/Keccak
        VsfType::hk(h) => format_crypto_hex("hk", h), // BLAKE2
        VsfType::hP(h) => format_crypto_hex("hP", h), // Photon handle proof
        VsfType::hR(h) => format_crypto_hex("hR", h), // Photon random material
        VsfType::hI(h) => format_crypto_hex("hI", h), // Photon identity seed
        VsfType::hV(h) => format_crypto_hex("hV", h), // Photon vault seed
        // Keys (blue)
        VsfType::ke(k) => format_crypto_hex("ke", k), // Ed25519
        VsfType::kx(k) => format_crypto_hex("kx", k), // X25519
        VsfType::kp(k) => format_crypto_hex("kp", k), // P-curve
        VsfType::kc(k) => format_crypto_hex("kc", k), // ChaCha20-Poly1305
        VsfType::ka(k) => format_crypto_hex("ka", k), // AES-256-GCM
        VsfType::kk(k) => format_crypto_hex("kk", k), // secp256k1
        VsfType::kf(k) => format_crypto_hex("kf", k), // Frodo
        VsfType::kn(k) => format_crypto_hex("kn", k), // NTRU
        VsfType::kl(k) => format_crypto_hex("kl", k), // McEliece
        VsfType::kh(k) => format_crypto_hex("kh", k), // HQC
        VsfType::kd(k) => format_crypto_hex("kd", k), // Dilithium/ML-DSA
        VsfType::km(k) => format_crypto_hex("km", k), // ML-KEM
        VsfType::kb(k) => format_crypto_hex("kb", k), // BIKE
        // Shared secrets (typed by algorithm)
        VsfType::ksx(k) => format_crypto_hex("ksx", k), // X25519 shared secret
        VsfType::ksp(k) => format_crypto_hex("ksp", k), // P-curve shared secret
        VsfType::ksk(k) => format_crypto_hex("ksk", k), // secp256k1 shared secret
        VsfType::ksf(k) => format_crypto_hex("ksf", k), // Frodo shared secret
        VsfType::ksn(k) => format_crypto_hex("ksn", k), // NTRU shared secret
        VsfType::ksl(k) => format_crypto_hex("ksl", k), // McEliece shared secret
        VsfType::ksh(k) => format_crypto_hex("ksh", k), // HQC shared secret
        VsfType::ksm(k) => format_crypto_hex("ksm", k), // ML-KEM shared secret
        // Signatures (purple)
        VsfType::ge(s) => format_crypto_hex("ge", s), // Ed25519
        VsfType::gp(s) => format_crypto_hex("gp", s), // ECDSA-P256
        VsfType::gd(s) => format_crypto_hex("gd", s), // Dilithium/ML-DSA
        VsfType::gs(s) => format_crypto_hex("gs", s), // Sphincs+
        VsfType::gf(s) => format_crypto_hex("gf", s), // Falcon
        #[allow(deprecated)]
        VsfType::gr(s) => format_crypto_hex("gr", s), // RSA (deprecated)
        // MACs (teal, like hashes)
        VsfType::mh(m) => format_crypto_hex("mh", m), // HMAC-SHA256
        VsfType::mp(m) => format_crypto_hex("mp", m), // Poly1305
        VsfType::mb(m) => format_crypto_hex("mb", m), // BLAKE3-keyed
        VsfType::mc(m) => format_crypto_hex("mc", m), // CMAC-AES
        // Wrapped/encoded (muted purple)
        VsfType::v(algo, data) => {
            // Check if this is a Toka Tree type (algo == b't')
            #[cfg(feature = "spirix")]
            if *algo == b't' {
                // vt hack support removed - show as generic wrapped type
                return format!(
                    "{}{}{}{}",
                    trc("vt", col_wrap()),
                    tc("3", col_punct()),
                    tc(&format!("{{{}}}", data.len()), col_punct()),
                    " (deprecated - use ro* types)".dimmed()
                );
            }
            // Fallback to existing crypto_wrap formatting for other algorithms
            format_crypto_wrap(*algo, data)
        }

        // Tensors: light blue Wire format: t3{dims}u3 + shape values + data e.g., t3{1}u3 3{16} [38,0,16,15,...] for a 16-element 1D u8 tensor
        VsfType::t_u3(tensor) => {
            let dims = tensor.shape.len();
            // Build shape encoding: 3{dim0}3{dim1}...
            let shape_encoded: String = tensor
                .shape
                .iter()
                .map(|d| {
                    format!(
                        "{}{}{}{}",
                        tc("3", col_punct()),
                        tc("⦉", col_punct()),
                        d.to_string().white(),
                        tc("⦊", col_punct())
                    )
                })
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
            let shape_hint = tensor
                .shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("×");

            format!(
                "{}{}{}{}{}{} {} {}{}{}{}{}",
                trc("t", col_tensor()),
                tc("3", col_punct()),
                tc("⦉", col_punct()),
                dims.to_string().white(),
                tc("⦊", col_punct()),
                trc("u3", col_uint()),
                shape_encoded,
                tc("[", col_punct()),
                values_preview.white(),
                tc(ellipsis, col_punct()),
                tc("]", col_punct()),
                format!(" {}", tc(&format!("{} u8 tensor", shape_hint), col_hint()))
            )
        }

        // Eagle Time: pink-magenta - show underlying type (e5/e6/e7/ef5/ef6)
        VsfType::e(et) => {
            let formatted = format_eagle_time(et);
            #[allow(deprecated)]
            let type_marker = match et {
                EtType::e5(_) => "e5",
                EtType::e6(_) => "e6",
                EtType::e7(_) => "e7",
                EtType::f5(_) => "ef5",
                EtType::f6(_) => "ef6",
            };
            format!(
                "{}{}{}{}",
                trc(type_marker, col_time()),
                tc("⦉", col_punct()),
                formatted.white(),
                tc("⦊", col_punct())
            )
        }

        // Opcodes: {xx} format with optional hint
        VsfType::op(a, b) => {
            let base = format!(
                "{}{}{}{}",
                tc("{", col_punct()),
                trc(char::from(*a).to_string(), col_uint()),
                trc(char::from(*b).to_string(), col_uint()),
                tc("}", col_punct())
            );

            // Add hint if available (without # comment marker)
            if let Some(hint) = opcode_hint(*a, *b) {
                format!("{} {}", base, trc(hint, col_hint()))
            } else {
                base
            }
        }

        // Spirix scalars (25 types: s33-s77) - scalar Display already includes ⦉value⦊
        #[cfg(feature = "spirix")]
        VsfType::s33(s) => format!("{}{}", trc("s33", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s34(s) => format!("{}{}", trc("s34", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s35(s) => format!("{}{}", trc("s35", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s36(s) => format!("{}{}", trc("s36", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s37(s) => format!("{}{}", trc("s37", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s43(s) => format!("{}{}", trc("s43", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s44(s) => format!("{}{}", trc("s44", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s45(s) => format!("{}{}", trc("s45", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s46(s) => format!("{}{}", trc("s46", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s47(s) => format!("{}{}", trc("s47", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s53(s) => format!("{}{}", trc("s53", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s54(s) => format!("{}{}", trc("s54", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s55(s) => format!("{}{}", trc("s55", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s56(s) => format!("{}{}", trc("s56", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s57(s) => format!("{}{}", trc("s57", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s63(s) => format!("{}{}", trc("s63", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s64(s) => format!("{}{}", trc("s64", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s65(s) => format!("{}{}", trc("s65", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s66(s) => format!("{}{}", trc("s66", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s67(s) => format!("{}{}", trc("s67", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s73(s) => format!("{}{}", trc("s73", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s74(s) => format!("{}{}", trc("s74", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s75(s) => format!("{}{}", trc("s75", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s76(s) => format!("{}{}", trc("s76", col_uint()), s),
        #[cfg(feature = "spirix")]
        VsfType::s77(s) => format!("{}{}", trc("s77", col_uint()), s),

        // Spirix circles (25 types: c33-c77) - circle Display already includes ⦇value⦈
        #[cfg(feature = "spirix")]
        VsfType::c33(c) => format!("{}{}", trc("c33", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c34(c) => format!("{}{}", trc("c34", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c35(c) => format!("{}{}", trc("c35", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c36(c) => format!("{}{}", trc("c36", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c37(c) => format!("{}{}", trc("c37", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c43(c) => format!("{}{}", trc("c43", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c44(c) => format!("{}{}", trc("c44", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c45(c) => format!("{}{}", trc("c45", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c46(c) => format!("{}{}", trc("c46", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c47(c) => format!("{}{}", trc("c47", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c53(c) => format!("{}{}", trc("c53", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c54(c) => format!("{}{}", trc("c54", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c55(c) => format!("{}{}", trc("c55", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c56(c) => format!("{}{}", trc("c56", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c57(c) => format!("{}{}", trc("c57", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c63(c) => format!("{}{}", trc("c63", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c64(c) => format!("{}{}", trc("c64", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c65(c) => format!("{}{}", trc("c65", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c66(c) => format!("{}{}", trc("c66", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c67(c) => format!("{}{}", trc("c67", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c73(c) => format!("{}{}", trc("c73", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c74(c) => format!("{}{}", trc("c74", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c75(c) => format!("{}{}", trc("c75", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c76(c) => format!("{}{}", trc("c76", col_uint()), c),
        #[cfg(feature = "spirix")]
        VsfType::c77(c) => format!("{}{}", trc("c77", col_uint()), c),

        // Renderable object types (ro* - scene graph primitives)
        #[cfg(feature = "spirix")]
        VsfType::rob(pos, size, fill, stroke, children) => {
            let mut result = format!("rob {}", trc("rectangle", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            // Add fill with colour hint
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            if !children.is_empty() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format_children(children), "children")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roc(center, radius, fill, stroke) => {
            let mut result = format!("roc {}", trc("circle", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", center), "center")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", radius), "radius")));
            // Add fill with colour hint
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::ron(pos, size, children) => {
            let mut result = format!("ron {}", trc("container", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format_children(children), "children")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roe(center, size, fill, stroke) => {
            let mut result = format!("roe {}", trc("ellipse", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", center), "center")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rol(start, end, width, colour) => {
            let mut result = format!("rol {}", trc("line", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", start), "start")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", end), "end")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rop(commands, fill, stroke) => {
            let mut result = format!("rop {}", trc("path", col_ro()));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", commands), "commands")
            ));
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roo(points, width, colour, closed) => {
            let mut result = format!("roo {}", trc("polyline", col_ro()));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", points), "points")
            ));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            result.push_str(&format!("\n{}", ro_field(format!("{}", closed), "closed")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::ror(controls, knots, degree, fill, stroke) => {
            let mut result = format!("ror {}", trc("NURBS", col_ro()));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", controls), "controls")
            ));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", knots), "knots")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", degree), "degree")));
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rox(points, spline_type, fill, stroke) => {
            let mut result = format!("rox {}", trc("spline", col_ro()));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", points), "points")
            ));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", spline_type), "type")
            ));
            let fill_str = match fill {
                Fill::Solid(colour) => {
                    format!(
                        "Solid({}{})",
                        match colour.as_ref() {
                            VsfType::rcr => "rcr",
                            VsfType::rcn => "rcn",
                            VsfType::rcb => "rcb",
                            VsfType::rcw => "rcw",
                            VsfType::rck => "rck",
                            _ => "?",
                        },
                        trc(colour_hint(colour.as_ref()), col_colour())
                    )
                }
                _ => format!("{:?}", fill),
            };
            result.push_str(&format!("\n{}", ro_field(fill_str, "fill")));
            if stroke.is_some() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", stroke), "stroke")
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rot(pos, text, size, colour, style) => {
            let mut result = format!("rot {}", trc("text", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", text), "text")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            if let Some(s) = style {
                let align_str = match s.align {
                    Some(1) => "left",
                    Some(2) => "right",
                    _ => "center",
                };
                result.push_str(&format!("\n{}", ro_field(align_str.to_string(), "align")));
                if let Some(v) = s.leading {
                    result.push_str(&format!("\n{}", ro_field(format!("{}", v), "leading")));
                }
                if let Some(v) = s.kerning {
                    result.push_str(&format!("\n{}", ro_field(format!("{}", v), "kerning")));
                }
                if let Some(v) = s.weight {
                    result.push_str(&format!("\n{}", ro_field(format!("{}", v), "weight")));
                }
                if let Some(v) = s.tilt {
                    result.push_str(&format!("\n{}", ro_field(format!("{}", v), "tilt")));
                }
                if let Some(v) = s.wrap {
                    result.push_str(&format!("\n{}", ro_field(format!("{}", v), "wrap")));
                }
                if let Some(h) = s.font {
                    result.push_str(&format!("\n{}", ro_field(hex::encode(h), "font hash")));
                }
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rou(pos, size, label, variant, colour) => {
            let mut result = format!("rou {}", trc("button", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", label), "label")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", variant), "variant")
            ));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roq(pos, size, placeholder, colour) => {
            let mut result = format!("roq {}", trc("text_input", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", placeholder), "placeholder")
            ));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roa(cols, rows, children) => {
            let mut result = format!("roa {}", trc("array", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", cols), "cols")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", rows), "rows")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{}", children.len()), "children")
            ));
            for (i, child) in children.iter().enumerate() {
                result.push_str(&format!(
                    "\n{}",
                    ro_field(format!("{:?}", child), &format!("[{}]", i))
                ));
            }
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::roi(pos, size, handle, tint) => {
            let mut result = format!("roi {}", trc("image", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", handle), "handle")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", tint), "tint")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rof(pos, size, handle) => {
            let mut result = format!("rof {}", trc("surface", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", pos), "position")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", size), "size")));
            result.push_str(&format!("\n{}", ro_field(format!("{}", handle), "handle")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rom(shape, children) => {
            let mut result = format!("rom {}", trc("mask", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", shape), "shape")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format_children(children), "children")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::row(transform, children) => {
            let mut result = format!("row {}", trc("group", col_ro()));

            // Format transform with Display instead of Debug for readable output
            let transform_str = {
                let mut parts = Vec::new();
                if let Some(t) = &transform.translate {
                    parts.push(format!("translate: {}", t));
                }
                if let Some(r) = &transform.rotate {
                    parts.push(format!("rotate: {} rad", r));
                }
                if let Some(s) = &transform.scale {
                    parts.push(format!("scale: {}", s));
                }
                if let Some(o) = &transform.origin {
                    parts.push(format!("origin: {}", o));
                }
                if parts.is_empty() {
                    "identity".to_string()
                } else {
                    parts.join(", ")
                }
            };

            result.push_str(&format!("\n{}", ro_field(transform_str, "transform")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format_children(children), "children")
            ));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rog(variant, stops) => {
            let mut result = format!("rog {}", trc("gradient", col_ro()));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", variant), "variant")
            ));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", stops), "stops")));
            result
        }
        #[cfg(feature = "spirix")]
        VsfType::rok(width, colour, join, cap) => {
            let mut result = format!("rok {}", trc("stroke", col_ro()));
            result.push_str(&format!("\n{}", ro_field(format!("{}", width), "width")));
            result.push_str(&format!(
                "\n{}",
                ro_field(format!("{:?}", colour), "colour")
            ));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", join), "join")));
            result.push_str(&format!("\n{}", ro_field(format!("{:?}", cap), "cap")));
            result
        }

        // ==================== NETWORK FAMILY ====================
        VsfType::ni(b) => {
            let ip = std::net::Ipv4Addr::from(*b);
            format!(
                "{}{}{}{}",
                trc("ni", col_uint()),
                tc("⦉", col_punct()),
                ip,
                tc("⦊", col_punct())
            )
        }
        VsfType::nj(b) => {
            let ip = std::net::Ipv6Addr::from(*b);
            format!(
                "{}{}{}{}",
                trc("nj", col_uint()),
                tc("⦉", col_punct()),
                ip,
                tc("⦊", col_punct())
            )
        }
        VsfType::nh(h) => format!(
            "{}{}{}{}",
            trc("nh", col_uint()),
            tc("⦉", col_punct()),
            h,
            tc("⦊", col_punct())
        ),
        VsfType::na(scheme, host, port) => {
            let scheme_str =
                NaScheme::from_byte(*scheme).map_or_else(|| scheme.to_string(), |s| s.to_string());
            let addr = match port {
                Some(p) => format!("{}://{}:{}", scheme_str, host, p),
                None => format!("{}://{}", scheme_str, host),
            };
            format!(
                "{}{}{}{}",
                trc("na", col_uint()),
                tc("⦉", col_punct()),
                addr,
                tc("⦊", col_punct())
            )
        }
        VsfType::nc(addr, prefix) => {
            format!(
                "{}{}{}/{}{}",
                trc("nc", col_uint()),
                tc("⦉", col_punct()),
                addr,
                prefix,
                tc("⦊", col_punct())
            )
        }
        VsfType::nm(mac) => {
            let s = mac
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(":");
            format!(
                "{}{}{}{}",
                trc("nm", col_uint()),
                tc("⦉", col_punct()),
                s,
                tc("⦊", col_punct())
            )
        }
        VsfType::np(port) => format!(
            "{}{}{}{}",
            trc("np", col_uint()),
            tc("⦉", col_punct()),
            port,
            tc("⦊", col_punct())
        ),
        VsfType::ns(host, port) => {
            format!(
                "{}{}{}:{}{}",
                trc("ns", col_uint()),
                tc("⦉", col_punct()),
                host,
                port,
                tc("⦊", col_punct())
            )
        }
        VsfType::nu(url) => format!(
            "{}{}{}{}",
            trc("nu", col_uint()),
            tc("⦉", col_punct()),
            url,
            tc("⦊", col_punct())
        ),
        VsfType::nn(name) => format!(
            "{}{}{}{}",
            trc("nn", col_uint()),
            tc("⦉", col_punct()),
            name,
            tc("⦊", col_punct())
        ),

        // ==================== WORLD ADDRESS ====================
        VsfType::wa(addr) => {
            format!(
                "{}{}{}{}",
                trc("wa", col_uint()),
                tc("⦉", col_punct()),
                addr,
                tc("⦊", col_punct())
            )
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

/// Format Eagle Time (ET) in human-readable format: 2025-OCT-29 6:42:21.813 PM Displays in local timezone (Eagle Time → UTC → Local)
pub fn format_eagle_time(et: &EtType) -> String {
    // Convert EtType to EagleTime and then to DateTime (UTC), then to local
    let eagle_time = EagleTime::new(et.clone());

    // Handle timestamps that are outside chrono's representable range
    let dt_utc = match eagle_time.to_datetime_opt() {
        Some(dt) => dt,
        None => {
            // Fallback: show the raw wire encoding
            #[allow(deprecated)]
            return match et {
                EtType::e5(v) => format!("e5{{{}}}", v),
                EtType::e6(v) => format!("e6{{{}}}", v),
                EtType::e7(v) => format!("e7{{{}}}", v),
                EtType::f5(v) => format!("ef5{{{}}}", v),
                EtType::f6(v) => format!("ef6{{{}}}", v),
            };
        }
    };
    let dt = dt_utc.with_timezone(&Local);

    // Extract milliseconds from fractional seconds For integer types (u/i), oscillations are converted to seconds with picosecond precision For float types (f5/f6), seconds are stored directly
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

/// Human-readable rendering of a VSF `x` (string) value's CONTENTS.
///
/// VSF `x` fields nominally hold text, but the stack also stuffs binary blobs into them (an encrypted message, a VSF-encoded sub-record, random padding). Rust's `escape_default()` turns such a blob into an unreadable wall of `\u{fffd}\u{7}\\` debug escapes. Instead:
/// - mostly-printable text → shown verbatim (control chars still escaped so the line stays intact),
/// - binary-ish content → a compact hex dump (`<N bytes binary> AABBCC…`), which is what you can actually read at a glance.
/// "Binary-ish" = >15% of bytes are non-tab/newline control chars or the content isn't valid UTF-8 text. Caps the hex at 64 bytes with an ellipsis so a 286-byte blob doesn't blow up the line.
pub fn display_x_value(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let nonprint = bytes
        .iter()
        .filter(|&&b| (b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') || b == 0x7f)
        .count();
    let binary_ish = s.contains('\u{fffd}') || (nonprint * 100 / bytes.len()) > 15;
    if binary_ish {
        const MAX: usize = 64;
        let hex: String = bytes
            .iter()
            .take(MAX)
            .map(|b| format!("{:02X}", b))
            .collect();
        let ell = if bytes.len() > MAX { "…" } else { "" };
        format!("<{} bytes binary> {}{}", bytes.len(), hex, ell)
    } else {
        // Printable text: escape only the few control chars so multi-line content can't break the surrounding tree layout, but leave normal characters untouched (no \u{} for every char).
        s.replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
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
        VsfType::x(s) => format!("\"{}\"", display_x_value(s)),
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
                format!(
                    "t_u3[{}]({} bytes)0x{}{}",
                    shape_str,
                    tensor.data.len(),
                    hex_preview,
                    ellipsis
                )
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
            // Wire literal format: e5{value}, e6{value}, e7{value}, ef5{value}, ef6{value}
            #[allow(deprecated)]
            match et {
                EtType::e5(v) => format!("e5{{{}}}", v),
                EtType::e6(v) => format!("e6{{{}}}", v),
                EtType::e7(v) => format!("e7{{{}}}", v),
                EtType::f5(v) => format!("ef5{{{:.2}}}", v),
                EtType::f6(v) => format!("ef6{{{:.2}}}", v),
            }
        }
        // Crypto types - show first 1KB with line wrapping
        VsfType::hp(hash) => format_crypto_literal("hp", hash),
        VsfType::hb(hash) => format_crypto_literal("hb", hash),
        VsfType::hs(hash) => format_crypto_literal("hs", hash),
        VsfType::hm(hash) => format_crypto_literal("hm", hash),
        VsfType::hg(hash) => format_crypto_literal("hg", hash),
        VsfType::hP(hash) => format_crypto_literal("hP", hash),
        VsfType::hR(hash) => format_crypto_literal("hR", hash),
        VsfType::hI(hash) => format_crypto_literal("hI", hash),
        VsfType::hV(hash) => format_crypto_literal("hV", hash),
        VsfType::ge(sig) => format_crypto_literal("ge", sig),
        VsfType::gp(sig) => format_crypto_literal("gp", sig),
        VsfType::gr(sig) => format_crypto_literal("gr", sig),
        VsfType::ke(key) => format_crypto_literal("ke", key),
        VsfType::kx(key) => format_crypto_literal("kx", key),
        VsfType::kp(key) => format_crypto_literal("kp", key),
        VsfType::kc(key) => format_crypto_literal("kc", key),
        VsfType::ka(key) => format_crypto_literal("ka", key),
        // Extended key types (post-quantum, additional curves)
        VsfType::kk(key) => format_crypto_literal("kk", key), // secp256k1
        VsfType::kf(key) => format_crypto_literal("kf", key), // Frodo
        VsfType::kn(key) => format_crypto_literal("kn", key), // NTRU
        VsfType::kl(key) => format_crypto_literal("kl", key), // McEliece
        VsfType::kh(key) => format_crypto_literal("kh", key), // HQC
        VsfType::mh(mac) => format_crypto_literal("mh", mac),
        VsfType::mp(mac) => format_crypto_literal("mp", mac),
        VsfType::mb(mac) => format_crypto_literal("mb", mac),
        VsfType::mc(mac) => format_crypto_literal("mc", mac),

        VsfType::v(algo, data) => format_crypto_literal(&format!("v{}", *algo as char), data),
        VsfType::d(name) => format!("d\"{}\"", name),
        VsfType::a(s) => s.clone(),
        VsfType::o(offset) => format!("o[{}]", offset),
        VsfType::n(count) => format!("n[{}]", count),
        VsfType::b(size, _) => format!("b[{}]", size),
        VsfType::l(size, _) => format!("l[{}]", size),

        // Network family
        VsfType::ni(b) => std::net::Ipv4Addr::from(*b).to_string(),
        VsfType::nj(b) => std::net::Ipv6Addr::from(*b).to_string(),
        VsfType::nh(h) => h.clone(),
        VsfType::na(scheme, host, port) => {
            let s =
                NaScheme::from_byte(*scheme).map_or_else(|| scheme.to_string(), |s| s.to_string());
            match port {
                Some(p) => format!("{}://{}:{}", s, host, p),
                None => format!("{}://{}", s, host),
            }
        }
        VsfType::nc(addr, prefix) => format!("{}/{}", addr, prefix),
        VsfType::nm(mac) => mac
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":"),
        VsfType::np(port) => format!(":{}", port),
        VsfType::ns(host, port) => format!("{}:{}", host, port),
        VsfType::nu(url) => url.clone(),
        VsfType::nn(name) => name.clone(),

        // World address
        VsfType::wa(addr) => addr.to_string(),

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

/// Format a VsfType value for compact display (tree view) Shows full hex for crypto fields (32/64 byte hashes, keys, signatures)
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
        // display_x_value self-caps binary at 64B; for long printable text show a head + ellipsis.
        VsfType::x(s) if s.len() > 30 => {
            let shown = display_x_value(s);
            if shown.starts_with('<') {
                // binary-ish: helper already produced "<N bytes binary> HEX…"
                shown
            } else {
                // printable: head-truncate by chars (not bytes — no UTF-8 boundary panic)
                let head: String = shown.chars().take(27).collect();
                format!("\"{}\"...", head)
            }
        }
        // Show literal VSF notation for crypto fields with colour coding type{size}0xHEX - type=cyan, size=yellow, 0x=gray, hex=white
        VsfType::hp(hash) => format_crypto_hex("hp", hash),
        VsfType::hb(hash) => format_crypto_hex("hb", hash),
        VsfType::hs(hash) => format_crypto_hex("hs", hash),
        VsfType::hm(hash) => format_crypto_hex("hm", hash),
        VsfType::hg(hash) => format_crypto_hex("hg", hash),
        VsfType::hP(hash) => format_crypto_hex("hP", hash),
        VsfType::hR(hash) => format_crypto_hex("hR", hash),
        VsfType::hI(hash) => format_crypto_hex("hI", hash),
        VsfType::hV(hash) => format_crypto_hex("hV", hash),
        VsfType::ke(key) => format_crypto_hex("ke", key),
        VsfType::kx(key) => format_crypto_hex("kx", key),
        VsfType::kp(key) => format_crypto_hex("kp", key),
        VsfType::kc(key) => format_crypto_hex("kc", key),
        VsfType::ka(key) => format_crypto_hex("ka", key),
        // Extended key types (post-quantum, additional curves)
        VsfType::kk(key) => format_crypto_hex("kk", key), // secp256k1
        VsfType::kf(key) => format_crypto_hex("kf", key), // Frodo
        VsfType::kn(key) => format_crypto_hex("kn", key), // NTRU
        VsfType::kl(key) => format_crypto_hex("kl", key), // McEliece
        VsfType::kh(key) => format_crypto_hex("kh", key), // HQC
        VsfType::ge(sig) => format_crypto_hex("ge", sig),
        VsfType::gp(sig) => format_crypto_hex("gp", sig),
        VsfType::gr(sig) => format_crypto_hex("gr", sig),
        VsfType::v(algo, data) => {
            // Check if this is a Toka Tree type (algo == b't')
            #[cfg(feature = "spirix")]
            if *algo == b't' {
                // vt hack support removed - show as generic wrapped type
                return format!(
                    "{}{}{}{}",
                    trc("vt", col_wrap()),
                    tc("3", col_punct()),
                    tc(&format!("{{{}}}", data.len()), col_punct()),
                    " (deprecated - use ro* types)".dimmed()
                );
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

    // For sections <1MB, no name is present - fields start immediately with '(' For sections >1MB, name + n{count}b{length} are required
    if pointer < data.len() && data[pointer] != b'(' {
        // Parse section name
        let section_name_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse section name: {}", e))?;
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

/// Try to parse section fields without knowing child_count - parse until ']' Used for sections with wrap/signature where child_count is omitted from header
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

    // For sections <1MB, no name is present - fields start immediately with '(' For sections >1MB, name + n{count}b{length} are required
    if pointer < data.len() && data[pointer] != b'(' {
        // Parse and skip section name
        let section_name_type = parse(data, &mut pointer)
            .map_err(|e| format!("Failed to parse section name: {}", e))?;
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

/// Format complete VSF stream for inspection (coloured output with tree structure) Returns multi-line string with header info, labels, and section tree Shows literal VSF encoding first (white), then descriptive hints (dark grey) Wire-format size suffix for an unsigned value, matching `EncodeNumber for usize` in encoding/primitives.rs. `3`=u8, `4`=u16, `5`=u32, `6`=u64, `7`=u128. Used by the inspector so the displayed marker (e.g. `l5⦉65636⦊`) matches the actual bytes on disk rather than a hardcoded `3`.
fn usize_size_suffix(n: usize) -> &'static str {
    if n <= u8::MAX as usize {
        "3"
    } else if n <= u16::MAX as usize {
        "4"
    } else if n <= u32::MAX as usize {
        "5"
    } else if n <= u64::MAX as usize {
        "6"
    } else {
        "7"
    }
}

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
    let _ =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse backward compat: {}", e))?;
    let header_length_type =
        parse(data, &mut pointer).map_err(|e| format!("Failed to parse header length: {}", e))?;
    let header_length_bytes = match header_length_type {
        VsfType::b(bytes, _) => bytes,
        _ => 0,
    };
    // Parse optional file length (l field) - only present in newer files
    let file_length_bytes = if pointer < data.len() && data[pointer] == b'l' {
        let file_length_type =
            parse(data, &mut pointer).map_err(|e| format!("Failed to parse file length: {}", e))?;
        match file_length_type {
            VsfType::l(bytes, _) => Some(bytes),
            _ => None,
        }
    } else {
        None // No l field present
    };

    let mut out = String::new();

    // Show literal magic number with title as hint
    out.push_str("RÅ\n");
    out.push_str(&format!(
        "< {}\n",
        tc("Versatile Storage Format", col_tree())
    ));

    // Version: z3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        trc("z", col_uint()),
        tc("3", col_punct()),
        tc("⦉", col_punct()),
        header.version.to_string().white(),
        tc("⦊", col_punct()),
        tc("version", col_tree())
    ));

    // Backward compat: y3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        trc("y", col_uint()),
        tc("3", col_punct()),
        tc("⦉", col_punct()),
        header.backward_compat.to_string().white(),
        tc("⦊", col_punct()),
        tc("backward compat", col_tree())
    ));

    // Creation time: et{timestamp} - time (pink-magenta). creation_time is Option<VsfType>; absent when the producing device had no clock.
    if let Some(VsfType::e(ref et)) = header.creation_time {
        #[allow(deprecated)]
        let type_suffix = match et {
            crate::types::EtType::e5(_) => "5".to_string(),
            crate::types::EtType::e6(_) => "6".to_string(),
            crate::types::EtType::e7(_) => "7".to_string(),
            crate::types::EtType::f5(_) => "f5".to_string(),
            crate::types::EtType::f6(_) => "f6".to_string(),
        };
        out.push_str(&format!(
            "  {}{}{}{}{}\n",
            trc("e", col_time()),
            trc(type_suffix, col_punct()),
            tc("⦉", col_punct()),
            format_eagle_time(et).white(),
            tc("⦊", col_punct())
        ));
    }

    // Header size: b{suffix}{N} Bytes - metadata/unsigned (soft green). Suffix tracks the actual wire encoding width.
    let header_size_valid = header_length_bytes == actual_header_size;
    let header_size_suffix = usize_size_suffix(header_length_bytes);
    if header_size_valid {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {} {}\n",
            trc("b", col_uint()),
            tc(header_size_suffix, col_punct()),
            tc("⦉", col_punct()),
            header_length_bytes.to_string().white(),
            tc("⦊", col_punct()),
            tc("Header size", col_hint()),
            tc("Bytes", col_hint()),
            tc("✓", col_pass())
        ));
    } else {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {} {} {}\n",
            trc("b", col_uint()),
            tc(header_size_suffix, col_punct()),
            tc("⦉", col_punct()),
            trc(header_length_bytes.to_string(), col_fail()),
            tc("⦊", col_punct()),
            tc("Header size", col_hint()),
            tc("Bytes", col_hint()),
            tc("✗ MISMATCH", col_fail()),
            trc(format!("(actual: {})", actual_header_size), col_hint())
        ));
    }

    // File length: l{suffix}{N} Bytes - metadata/unsigned (soft green). Lowercase `l` matches the on-disk byte (`b'l'`); suffix tracks the actual wire encoding width.
    if let Some(file_len) = file_length_bytes {
        let actual_len = data.len();
        let length_valid = file_len == actual_len;
        let file_len_suffix = usize_size_suffix(file_len);
        if length_valid {
            out.push_str(&format!(
                "  {}{}{}{}{} {} {} {}\n",
                trc("l", col_uint()),
                tc(file_len_suffix, col_punct()),
                tc("⦉", col_punct()),
                file_len.to_string().white(),
                tc("⦊", col_punct()),
                tc("File length", col_hint()),
                tc("Bytes", col_hint()),
                tc("✓", col_pass())
            ));
        } else {
            out.push_str(&format!(
                "  {}{}{}{}{} {} {} {} {}\n",
                trc("l", col_uint()),
                tc(file_len_suffix, col_punct()),
                tc("⦉", col_punct()),
                trc(file_len.to_string(), col_fail()),
                tc("⦊", col_punct()),
                tc("File length", col_hint()),
                tc("Bytes", col_hint()),
                tc("✗ MISMATCH", col_fail()),
                trc(format!("(actual: {})", actual_len), col_hint())
            ));
        }
    }

    // Provenance hash: hp3{31} (32 Bytes) - encoded as len-1
    if let VsfType::hp(ref hash) = header.provenance_hash {
        out.push_str(&format!(
            "  {}{}{}{}{} {} {}\n",
            trc("hp", col_hash()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            format!("{}", hash.len() - 1).white(),
            tc("⦊", col_punct()),
            tc("BLAKE3 provenance hash", col_hint()),
            trc(format!("({} Bytes)", hash.len()), col_hint()),
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
            trc("ke", col_key()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            format!("{}", key.len() - 1).white(),
            tc("⦊", col_punct()),
            tc("Ed25519 signer pubkey", col_hint()),
            trc(format!("({} Bytes)", key.len()), col_hint()),
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
            trc("ge", col_sig()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            format!("{}", sig.len() - 1).white(),
            tc("⦊", col_punct()),
            tc("Ed25519 signature", col_hint()),
            trc(format!("({} Bytes)", sig.len()), col_hint()),
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
            trc("hb", col_hash()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            format!("{}", hash.len() - 1).white(), // len-1 encoding
            tc("⦊", col_punct()),
            tc("BLAKE3 rolling hash", col_hint()),
            trc(format!("({} Bytes)", hash.len()), col_hint()),
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
                    tc("Verification:", col_hint()),
                    tc("PASS", col_pass())
                ));
            } else {
                out.push_str(&format!(
                    "  {} {}\n",
                    tc("Verification:", col_hint()),
                    tc("FAIL", col_fail())
                ));
            }
        }
    }

    // Label count: n3{N} - metadata/unsigned (soft green)
    out.push_str(&format!(
        "  {}{}{}{}{} {}\n",
        trc("n", col_uint()),
        tc("3", col_punct()),
        tc("⦉", col_punct()),
        labels.len().to_string().white(),
        tc("⦊", col_punct()),
        tc("labels", col_hint())
    ));

    for label in &labels {
        // Show literal VSF encoding: (d3{N}name o3{offset} b3{size} n3{count} ke3{31}... ge3{63}...)
        out.push_str(&format!("  {}", tc("(", col_punct())));

        // d3{len}name - field name with length prefix - text (light blue)
        out.push_str(&format!(
            "{}{}{}{}{}{}\n",
            trc("d", col_text()),
            tc("3", col_punct()),
            tc("⦉", col_punct()),
            label.name.len().to_string().white(),
            tc("⦊", col_punct()),
            label.name.white().bold()
        ));

        if label.size == 0 {
            // Inline field: (d3{N}name:val1,val2,...)
            if !label.inline_values.is_empty() {
                out.push_str("    ");
                out.push_str(&format!("{}", tc(":", col_punct())));
                for (i, val) in label.inline_values.iter().enumerate() {
                    if i > 0 {
                        out.push_str(&format!("{}", tc(",", col_punct())));
                    }
                    out.push_str(&format!("{}", format_value_literal(val)));
                }
                out.push_str("\n");
            }
        } else {
            // Section pointer: o{suffix}{offset} b{suffix}{size} n{suffix}{count} - metadata/unsigned (soft green) - each on own line. Suffix tracks actual wire encoding width.
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                trc("o", col_uint()),
                tc(usize_size_suffix(label.offset), col_punct()),
                tc("⦉", col_punct()),
                label.offset.to_string().white(),
                tc("⦊", col_punct())
            ));
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                trc("b", col_uint()),
                tc(usize_size_suffix(label.size), col_punct()),
                tc("⦉", col_punct()),
                label.size.to_string().white(),
                tc("⦊", col_punct())
            ));
            out.push_str(&format!(
                "    {}{}{}{}{}\n",
                trc("n", col_uint()),
                tc(usize_size_suffix(label.child_count), col_punct()),
                tc("⦉", col_punct()),
                label.child_count.to_string().white(),
                tc("⦊", col_punct())
            ));
        }

        // NOTE: ke/ge are NOT part of the label pointer - they appear in section content
        out.push_str(&format!("  {}\n", tc(")", col_punct())));
    }

    // Check if there are any non-empty sections
    let has_nonempty_sections = labels.iter().any(|l| l.size > 0);

    if has_nonempty_sections {
        out.push_str(&format!("{}{}\n", tc(">", col_tree()), tc("╮", col_tree())));
    // Light arc down and left
    } else {
        out.push_str(&format!("{}\n", tc(">", col_tree())));
    }

    // Show sections with tree structure (skip empty sections)
    let nonempty_labels: Vec<_> = labels.iter().filter(|l| l.size > 0).collect();
    for (i, label) in nonempty_labels.iter().enumerate() {
        let is_last = i == nonempty_labels.len() - 1;
        let connector = if is_last {
            format!(" {}", tree_corner())
        } else {
            format!(" {}", tree_tee())
        };

        // For sections < 1MB, just show `[` - name is already in header labels For sections >= 1MB, show `[name n{count}b{size}` for navigation
        if label.size < 1024 * 1024 {
            out.push_str(&format!("{}{}\n", connector, tc("[", col_tree())));
        } else {
            out.push_str(&format!(
                "{}{}{} {}{}{}{}{}{}{}b{}{}\n",
                connector,
                tc("[", col_tree()),
                label.name.white().bold(),
                tc("n", col_label()),
                tc("⦉", col_comment()),
                trc(label.child_count.to_string(), col_size()),
                tc("⦊", col_comment()),
                " ".normal(),
                tc("b", col_label()),
                tc("⦉", col_comment()),
                trc(label.size.to_string(), col_size()),
                tc("⦊", col_comment())
            ));
        }

        // Parse and show fields For child_count == 0 (signed/wrapped sections), try dynamic parsing For child_count > 0, use the known count
        let field_prefix = if is_last {
            "  "
        } else {
            &format!(" {} ", tree_vert())
        };

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
                                    trc(hex_preview, col_hint()),
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
                    let field_connector = if is_field_last {
                        tree_corner_line()
                    } else {
                        tree_tee_line()
                    };
                    let continuation_bar = if is_field_last {
                        "    "
                    } else {
                        &format!("{}   ", tree_vert())
                    };
                    let name_literal = format_value_literal(&VsfType::d(field.name.clone()));
                    let values_literal: Vec<String> = field
                        .values
                        .iter()
                        .map(|v| format_value_literal(v))
                        .collect();

                    if values_literal.len() == 1 {
                        // Single value: (name : value) - handle multi-line hex
                        let val = &values_literal[0];
                        if val.contains('\n') {
                            // Multi-line crypto value
                            let hex_indent = format!("{}{}    ", field_prefix, continuation_bar);
                            let formatted = val.replace('\n', &format!("\n{}", hex_indent));
                            out.push_str(&format!(
                                "{}{}{}{} {} {}{}\n",
                                field_prefix,
                                field_connector,
                                tc("(", col_tree()),
                                name_literal,
                                tc(":", col_tree()),
                                formatted,
                                tc(")", col_tree()),
                            ));
                        } else {
                            out.push_str(&format!(
                                "{}{}{}{} {} {}{}\n",
                                field_prefix,
                                field_connector,
                                tc("(", col_tree()),
                                name_literal,
                                tc(":", col_tree()),
                                val,
                                tc(")", col_tree()),
                            ));
                        }
                    } else {
                        // Multi-value: group opcodes with following values (newline before each opcode)
                        out.push_str(&format!(
                            "{}{}{}{}:\n",
                            field_prefix,
                            field_connector,
                            tc("(", col_tree()),
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

                            // Add value to buffer If previous value was an opcode and this isn't, add newline before this value
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
                                    out.push_str(&format!("      {}\n", tc(")", col_tree())));
                                } else {
                                    for (i, line) in lines.iter().enumerate() {
                                        if i == lines.len() - 1 {
                                            // Last line gets closing paren
                                            out.push_str(&format!(
                                                "      {}{}\n",
                                                line,
                                                tc(")", col_tree())
                                            ));
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
                        trc(hex_preview, col_hint()),
                        suffix
                    ));
                }
            }
        }

        let tree_suffix = if is_last {
            "   ".to_string()
        } else {
            format!(" {} ", tree_vert())
        };
        out.push_str(&format!("{}{}\n", tree_suffix, tc("]", col_tree())));

        if !is_last {
            out.push_str(&format!(" {}\n", tree_vert()));
        }
    }

    Ok(out)
}

/// Format a section fragment (starts with '[') Used for inspecting VSF section bytes before they're wrapped in a file
///
/// Shows literal VSF wire notation:
/// ```text
/// [error └─ (d3{7}message : l3{24}handle claimed elsewhere) ]
/// ```
pub fn inspect_section(data: &[u8]) -> Result<String, String> {
    if data.is_empty() || data[0] != b'[' {
        return Err("Not a section fragment (doesn't start with '[')".into());
    }

    let mut out = String::new();
    let mut pointer = 1usize; // Skip '['

    // Parse section name if present (encoder omits it for sections within 1MB of header)
    let section_name = if pointer < data.len() && data[pointer] != b'(' && data[pointer] != b']' {
        match parse(data, &mut pointer) {
            Ok(VsfType::d(name)) => name,
            Ok(_) => return Err("Expected d type for section name".into()),
            Err(e) => return Err(format!("Failed to parse section name: {}", e)),
        }
    } else {
        String::new()
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
    let mut header = format!("{}{}", tc("[", col_label()), section_name.white().bold());
    if let Some(len) = length_hint {
        header.push_str(&format!(
            " {} {}",
            len.to_string().white(),
            tc("Bytes", col_label())
        ));
    }
    if let Some(count) = count_hint {
        header.push_str(&format!(
            " {} {}",
            count.to_string().white(),
            tc(if count == 1 { "field" } else { "fields" }, col_label())
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
    let comma = tc(",", col_label()).to_string();
    let pipe = tc("┃", col_tree()).to_string();

    for (i, field) in fields.iter().enumerate() {
        let is_last_field = i == fields.len() - 1;
        let connector = if is_last_field {
            tree_corner_line()
        } else {
            tree_tee_line()
        };
        let continuation = if is_last_field {
            "   ".to_string()
        } else {
            format!("{}  ", pipe)
        };

        // First line: connector + (name : first_value
        let name_literal = format_value_literal(&VsfType::d(field.name.clone()));
        out.push_str(&format!(
            "  {} {}{} {} ",
            connector,
            tc("(", col_label()),
            name_literal,
            tc(":", col_label())
        ));

        // Format each value, one per line after the first
        for (vi, val) in field.values.iter().enumerate() {
            let is_last_val = vi == field.values.len() - 1;
            let val_str = format_value_literal(val);

            if vi == 0 {
                // First value on same line as field name Check if it has hex lines that need continuation
                if val_str.contains('\n') {
                    let parts: Vec<&str> = val_str.split('\n').collect();
                    out.push_str(parts[0]);
                    out.push('\n');
                    for (hi, hex_line) in parts[1..].iter().enumerate() {
                        out.push_str(&format!("  {}     {}", continuation, hex_line));
                        if hi == parts.len() - 2 {
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
                if val_str.contains('\n') {
                    let parts: Vec<&str> = val_str.split('\n').collect();
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
        out.push_str(&format!("  {}  {}\n", continuation, tc(")", col_label())));
    }

    // Closing bracket
    out.push_str(&format!("{}\n", tc("]", col_tree())));

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
                validation.push_str(&trc(format!(" {}B", actual_len), col_pass()).to_string());
            } else {
                validation.push_str(
                    &trc(format!(" {}B/{}", actual_len, expected_len), col_fail()).to_string(),
                );
                valid = false;
            }
        }

        if let Some(expected_count) = count_hint {
            validation.push_str(&trc(format!(" n={}", expected_count), col_hint()).to_string());
        }

        if valid {
            out.push_str(&format!("  {}\n", tc("✓", col_pass())));
        } else {
            out.push_str(&format!(
                "  {} MISMATCH{}\n",
                tc("✗", col_fail()),
                validation
            ));
        }
    }

    Ok(out)
}

/// Hex dump with ASCII sidebar (like xxd), truncated to 1KB head + 1KB tail
pub fn hex_dump(data: &[u8]) -> String {
    // Honor the same runtime elision knob as the structured inspector (VSF_HEX_HEAD/TAIL or set_hex_elision), rounded UP to whole 16-byte lines so this ASCII hexdump stays aligned.
    let (cfg_head, cfg_tail) = hex_elision();
    let round_up = |n: usize| n.div_ceil(16) * 16;
    let max_head = round_up(cfg_head);
    let max_tail = round_up(cfg_tail);
    let mut out = String::new();

    let (head, omitted, tail_offset) = if data.len() > max_head + max_tail {
        let omitted = data.len() - max_head - max_tail;
        (&data[..max_head], Some(omitted), data.len() - max_tail)
    } else {
        (data, None, 0)
    };

    let format_chunk = |offset: usize, chunk: &[u8]| {
        let mut line = format!("{:08x}: ", offset);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                line.push(' ');
            }
            line.push_str(&format!("{:02x} ", byte));
        }
        let padding = 16 - chunk.len();
        for j in 0..padding {
            if chunk.len() + j == 8 {
                line.push(' ');
            }
            line.push_str("   ");
        }
        line.push(' ');
        for byte in chunk {
            if *byte >= 0x20 && *byte < 0x7f {
                line.push(*byte as char);
            } else {
                line.push('.');
            }
        }
        line.push('\n');
        line
    };

    for (i, chunk) in head.chunks(16).enumerate() {
        out.push_str(&format_chunk(i * 16, chunk));
    }

    if let Some(n) = omitted {
        out.push_str(&format!("... {} bytes omitted ...\n", n));
        let tail = &data[tail_offset..];
        for (i, chunk) in tail.chunks(16).enumerate() {
            out.push_str(&format_chunk(tail_offset + i * 16, chunk));
        }
    }

    out
}

#[cfg(test)]
mod hex_elision_tests {
    use super::*;

    #[test]
    fn elision_uses_configured_lengths_and_elides_large_blobs() {
        // First writer wins on the OnceLock, so set before any inspect output in this test binary.
        set_hex_elision(16, 16);
        let (h, t) = hex_elision();
        assert_eq!((h, t), (16, 16));

        // Small blob (<= head+tail): printed whole, no notice.
        let small = vec![0xABu8; 24];
        let (head, notice, tail) = format_hex_head_tail(&small);
        assert!(notice.is_none(), "24B should not be elided with 16+16");
        assert!(tail.is_empty());
        assert_eq!(head.join("").replace(' ', "").len(), 24 * 2); // full hex

        // Large blob: head 16B + notice + tail 16B, middle omitted.
        let big = (0..15791u32).map(|i| i as u8).collect::<Vec<u8>>();
        let (head, notice, tail) = format_hex_head_tail(&big);
        let notice = notice.expect("15791B must be elided");
        assert!(notice.contains(&format!("{} bytes omitted", 15791 - 16 - 16)));
        assert_eq!(head.join("").len(), 16 * 2); // 16 bytes of hex
        assert_eq!(tail.join("").len(), 16 * 2);
    }
}
