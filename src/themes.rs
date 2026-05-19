//! VSF Inspector Themes
//!
//! Centralized colour scheme definitions for VSF inspection output. Supports terminal (ANSI), HTML, and plain text output formats.

use crate::prelude::*;

/// RGB colour triplet
pub type Colour = (u8, u8, u8);

/// Theme for VSF inspector output
///
/// All colours are RGB triplets (r, g, b) where each component is 0-255.
#[derive(Debug, Clone)]
pub struct Theme {
    // === Type Categories ===
    /// Time types (e, eu, ei, ef5, ef6)
    pub time: Colour,

    /// Unsigned integers (u, u3-u7, y, z, b, L)
    pub uint: Colour,

    /// Signed integers (i, i3-i7)
    pub sint: Colour,

    /// Floating point (f5, f6, s44, c44)
    pub float: Colour,

    /// Text/strings (x)
    pub text: Colour,

    /// Binary data (q)
    pub binary: Colour,

    /// Colour types (r*, ra, rw)
    pub colour: Colour,

    /// Crypto types (h*, k*, g*, sig)
    pub crypto: Colour,

    /// Opcodes ({ab})
    pub opcode: Colour,

    /// Renderable objects (ro*)
    pub renderable: Colour,

    /// Scene graph types (vt)
    pub scene_graph: Colour,

    // === Semantic Colors ===
    /// Verification passed (✓)
    pub pass: Colour,

    /// Verification failed (✗)
    pub fail: Colour,

    /// Warnings
    pub warn: Colour,

    /// Hints/descriptions
    pub hint: Colour,

    /// Punctuation/structure (⦉⦊, {}, <>, etc.)
    pub punct: Colour,

    /// Field labels and names (section counts, field names, punctuation in displays)
    pub label: Colour,

    /// Comments and annotations
    pub comment: Colour,

    /// Byte sizes and counts (section sizes, child counts)
    pub size: Colour,

    // === UI Elements ===
    /// Section headers
    pub header: Colour,

    /// Tree structure characters (╮, ╰, │, ─)
    pub tree: Colour,

    /// Background highlight (for selected items)
    pub highlight_bg: Option<Colour>,
}

impl Theme {
    /// Default dark theme (current VSF inspector colours)
    pub fn dark() -> Self {
        Self {
            // Type categories
            time: (255, 150, 220),        // Pink-magenta
            uint: (160, 255, 160),        // Soft green
            sint: (255, 160, 160),        // Soft red
            float: (180, 200, 255),       // Soft blue
            text: (255, 220, 150),        // Soft yellow
            binary: (200, 150, 255),      // Soft purple
            colour: (255, 180, 120),      // Soft orange
            crypto: (120, 200, 200),      // Soft cyan
            opcode: (255, 200, 100),      // Golden
            renderable: (150, 255, 200),  // Mint green
            scene_graph: (200, 150, 255), // Lavender

            // Semantic
            pass: (0, 200, 0),        // Bright green
            fail: (255, 80, 80),      // Bright red
            warn: (255, 180, 0),      // Orange
            hint: (64, 64, 64),       // Dark gray
            punct: (80, 80, 80),      // Medium gray
            label: (128, 128, 128),   // Mid gray
            comment: (100, 100, 100), // Light gray
            size: (200, 200, 100),    // Warm yellow

            // UI elements
            header: (64, 64, 64), // Dark gray
            tree: (64, 64, 64),   // Dark gray
            highlight_bg: None,
        }
    }

    /// Light theme (for light terminal backgrounds)
    pub fn light() -> Self {
        Self {
            // Type categories
            time: (180, 0, 120),        // Deep magenta
            uint: (0, 150, 0),          // Forest green
            sint: (180, 0, 0),          // Deep red
            float: (0, 80, 200),        // Deep blue
            text: (150, 100, 0),        // Brown
            binary: (100, 0, 180),      // Deep purple
            colour: (200, 80, 0),       // Deep orange
            crypto: (0, 120, 120),      // Deep cyan
            opcode: (150, 100, 0),      // Dark gold
            renderable: (0, 150, 100),  // Dark mint
            scene_graph: (120, 0, 180), // Deep lavender

            // Semantic
            pass: (0, 150, 0),        // Green
            fail: (200, 0, 0),        // Red
            warn: (200, 100, 0),      // Dark orange
            hint: (120, 120, 120),    // Gray
            punct: (100, 100, 100),   // Medium gray
            label: (80, 80, 80),      // Mid gray
            comment: (150, 150, 150), // Light gray
            size: (140, 140, 0),      // Dark yellow

            // UI elements
            header: (80, 80, 80),  // Medium gray
            tree: (100, 100, 100), // Medium gray
            highlight_bg: None,
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        // Solarized base colours
        let base02 = (7, 54, 66);
        let base01 = (88, 110, 117);
        let base1 = (147, 161, 161);
        let yellow = (181, 137, 0);
        let orange = (203, 75, 22);
        let red = (220, 50, 47);
        let magenta = (211, 54, 130);
        let violet = (108, 113, 196);
        let blue = (38, 139, 210);
        let cyan = (42, 161, 152);
        let green = (133, 153, 0);

        Self {
            time: magenta,
            uint: green,
            sint: red,
            float: blue,
            text: yellow,
            binary: violet,
            colour: orange,
            crypto: cyan,
            opcode: yellow,
            renderable: cyan,
            scene_graph: violet,

            pass: green,
            fail: red,
            warn: orange,
            hint: base01,
            punct: base01,
            label: base1,
            comment: base01,
            size: yellow,

            header: base1,
            tree: base01,
            highlight_bg: Some(base02),
        }
    }

    /// Nord theme (popular dark theme)
    pub fn nord() -> Self {
        // Nord palette
        let nord1 = (59, 66, 82);
        let nord3 = (76, 86, 106);
        let nord4 = (216, 222, 233); // Snow Storm
        let nord7 = (143, 188, 187); // Frost
        let nord8 = (136, 192, 208);
        let nord9 = (129, 161, 193);
        let nord10 = (94, 129, 172);
        let nord11 = (191, 97, 106); // Aurora
        let nord12 = (208, 135, 112);
        let nord13 = (235, 203, 139);
        let nord14 = (163, 190, 140);
        let nord15 = (180, 142, 173);

        Self {
            time: nord15,
            uint: nord14,
            sint: nord11,
            float: nord9,
            text: nord13,
            binary: nord10,
            colour: nord12,
            crypto: nord7,
            opcode: nord13,
            renderable: nord8,
            scene_graph: nord15,

            pass: nord14,
            fail: nord11,
            warn: nord12,
            hint: nord3,
            punct: nord3,
            label: nord4,
            comment: nord3,
            size: nord13,

            header: nord4,
            tree: nord3,
            highlight_bg: Some(nord1),
        }
    }

    /// Gruvbox Dark theme
    pub fn gruvbox_dark() -> Self {
        // Gruvbox dark colours
        let dark1 = (60, 56, 54);
        let dark3 = (102, 92, 84);
        let dark4 = (124, 111, 100);
        let light1 = (235, 219, 178);
        let red = (251, 73, 52);
        let green = (184, 187, 38);
        let yellow = (250, 189, 47);
        let blue = (131, 165, 152);
        let purple = (211, 134, 155);
        let aqua = (142, 192, 124);
        let orange = (254, 128, 25);

        Self {
            time: purple,
            uint: green,
            sint: red,
            float: blue,
            text: yellow,
            binary: purple,
            colour: orange,
            crypto: aqua,
            opcode: yellow,
            renderable: aqua,
            scene_graph: purple,

            pass: green,
            fail: red,
            warn: orange,
            hint: dark4,
            punct: dark3,
            label: light1,
            comment: dark4,
            size: yellow,

            header: light1,
            tree: dark3,
            highlight_bg: Some(dark1),
        }
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "dark" | "default" => Some(Self::dark()),
            "light" => Some(Self::light()),
            "solarized" | "solarized-dark" => Some(Self::solarized_dark()),
            "nord" => Some(Self::nord()),
            "gruvbox" | "gruvbox-dark" => Some(Self::gruvbox_dark()),
            _ => None,
        }
    }

    /// List available theme names
    pub fn available() -> Vec<&'static str> {
        vec!["dark", "light", "solarized-dark", "nord", "gruvbox-dark"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_by_name() {
        assert!(Theme::by_name("dark").is_some());
        assert!(Theme::by_name("light").is_some());
        assert!(Theme::by_name("nord").is_some());
        assert!(Theme::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_available_themes() {
        let themes = Theme::available();
        assert!(themes.len() >= 5);
        assert!(themes.contains(&"dark"));
        assert!(themes.contains(&"nord"));
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.pass, (0, 200, 0)); // Bright green
    }
}
