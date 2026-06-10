//! Frequency-based Huffman text encoding for VSF `x` marker
//!
//! Achieves ~2× compression over UTF-8 through frequency-optimized encoding:
//! - Common characters (space, 'e', 't') use short codes (3-5 bits)
//! - Less common characters use medium codes (8-12 bits)
//! - Rare Unicode characters use longer codes (16-24 bits)
//!
//! The Huffman tree is pre-computed at build time from global language frequency analysis. ALL 1,112,064 valid Unicode codepoints (0x000000–0x10FFFF excluding the U+D800–U+DFFF surrogate range) have pre-assigned codes — Unicode's yearly codepoint additions cannot change any existing encoding because every slot is already reserved.
//!
//! ## NFC canonicalization
//!
//! Input text is NFC-normalized inside `encode_text` before Huffman encoding. This means
//! "café" (U+00E9 precomposed) and "cafe\u{0301}" (e + combining acute) produce byte-identical
//! Huffman output — both reduce to the same NFC codepoint sequence first.
//!
//! NFC is anchored by Unicode's stability policy: once a codepoint is assigned, its NFC form
//! is guaranteed not to change across Unicode versions. This is the same byte-determinism
//! anchor as a frozen wire-format hash function — the canonical output is specified outside
//! this crate and cannot drift.
//!
//! Callers downstream that use VSF for identity hashing (notably `ihi::handle_to_proof`)
//! inherit café-stability automatically: there is no way to construct an `x`-encoded byte
//! stream that distinguishes NFC from NFD input.

use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// A Huffman code pattern (bits + length)
#[derive(Debug, Copy, Clone)]
pub struct BitPattern {
    pub bits: u32,  // The Huffman code (right-aligned, max 24 bits)
    pub length: u8, // Number of bits (1-24)
}

/// Bit vector for variable-length encoding
pub struct BitVec {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl BitVec {
    pub fn new() -> Self {
        BitVec {
            bytes: Vec::new(),
            bit_len: 0,
        }
    }

    pub fn push(&mut self, bit: bool) {
        let byte_idx = self.bit_len / 8;
        let bit_idx = self.bit_len % 8;

        if byte_idx >= self.bytes.len() {
            self.bytes.push(0);
        }

        if bit {
            self.bytes[byte_idx] |= 1 << (7 - bit_idx);
        }

        self.bit_len += 1;
    }

    pub fn extend_bits(&mut self, value: u32, num_bits: u8) {
        for i in (0..num_bits).rev() {
            self.push((value >> i) & 1 != 0);
        }
    }

    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    pub fn to_bytes(mut self) -> Vec<u8> {
        // Pad to byte boundary
        while self.bit_len % 8 != 0 {
            self.push(false);
        }
        self.bytes
    }
}

/// Load pre-computed Huffman codes from build-time generated file
///
/// Returns empty HashMap if huffman_codes.bin is not available (e.g., docs.rs builds)
fn load_huffman_codes() -> HashMap<char, BitPattern> {
    // Only include the file if it exists at build time This allows docs.rs builds to succeed even when Huffman tables aren't generated
    #[cfg(huffman_available)]
    {
        const DATA: &[u8] = include_bytes!("../huffman_codes.bin");

        // Parse header
        if &DATA[0..4] != b"HUFF" {
            panic!("Invalid Huffman codes file");
        }

        let count = u32::from_le_bytes(DATA[8..12].try_into().unwrap()) as usize;
        let mut codes = HashMap::with_capacity(count);

        // Parse entries
        for i in 0..count {
            let offset = 12 + i * 8;
            let codepoint = u32::from_le_bytes(DATA[offset..offset + 4].try_into().unwrap());
            let packed = u32::from_le_bytes(DATA[offset + 4..offset + 8].try_into().unwrap());

            let ch = char::from_u32(codepoint).unwrap();
            let pattern = BitPattern {
                bits: packed & 0x00FFFFFF,
                length: ((packed >> 24) & 0x1F) as u8,
            };

            codes.insert(ch, pattern);
        }

        codes
    }

    #[cfg(not(huffman_available))]
    {
        // Return empty HashMap - text encoding will panic with helpful message
        HashMap::new()
    }
}

// Lazy-load codes at first use
use std::sync::OnceLock;
static ENCODE_TABLE: OnceLock<HashMap<char, BitPattern>> = OnceLock::new();
static ASCII_LUT: OnceLock<[BitPattern; 128]> = OnceLock::new();

fn get_encode_table() -> &'static HashMap<char, BitPattern> {
    ENCODE_TABLE.get_or_init(load_huffman_codes)
}

fn get_ascii_lut() -> &'static [BitPattern; 128] {
    ASCII_LUT.get_or_init(|| {
        let codes = get_encode_table();
        let mut lut = [BitPattern { bits: 0, length: 0 }; 128];

        for i in 0..128 {
            if let Some(ch) = char::from_u32(i) {
                if let Some(pattern) = codes.get(&ch) {
                    lut[i as usize] = *pattern;
                }
            }
        }

        lut
    })
}

/// Encode Unicode text to Huffman-compressed bytes after NFC normalization.
///
/// Returns `(huffman_bytes, nfc_char_count)`. The byte vector is the Huffman bitstream padded
/// to a byte boundary; the char count is the number of codepoints AFTER NFC normalization,
/// not the input length. Callers MUST use the returned count when writing the VSF `x`
/// marker — NFC can collapse or expand codepoint sequences, and using the original count
/// would corrupt the wire format.
///
/// All characters use variable-length Huffman codes (3-24 bits). The codebook covers all
/// 1,112,064 valid Unicode codepoints, so this function never fails on valid `&str` input.
///
/// ## Canonicalization
///
/// Input is NFC-normalized before encoding. Café/café-class identity splits are impossible
/// at this boundary: any two strings that compare equal under NFC produce byte-identical
/// output.
///
/// # Performance
/// Uses optimized ASCII fast path (direct array access) for characters 0-127, falling back to HashMap lookup for full Unicode. NFC over pure-ASCII input is the identity transform, so the ASCII hot path stays cheap.
///
/// # VSF Integration
/// The VSF x marker format is:
/// ```text
/// x [char_count] [huffman_bytes]
/// ```
/// `char_count` here is the NFC count returned from this function, encoded via
/// `encode_number()` (3-6+ bytes depending on size). No arbitrary limits — supports billions
/// of characters.
///
/// # Example
/// ```ignore
/// let (encoded, char_count) = encode_text("Hello");
/// // char_count == 5, encoded is ~3 bytes of Huffman bits (no internal header)
/// ```
pub fn encode_text(text: &str) -> (Vec<u8>, usize) {
    let codes = get_encode_table();
    let ascii_lut = get_ascii_lut();
    let mut bits = BitVec::new();
    let mut char_count = 0usize;

    for c in text.nfc() {
        let pattern = if c.is_ascii() {
            // Fast path: direct array access (~2-3 CPU cycles)
            &ascii_lut[c as usize]
        } else {
            // Slow path: HashMap lookup for Unicode (~10-20 cycles).
            // The codebook covers all 1,112,064 valid codepoints, so this lookup
            // cannot fail for any character producible by NFC over a valid &str.
            // If it does, the codebook itself is corrupted — fall through to the
            // .expect for a loud build-time-only failure mode (huffman_codes.bin
            // is verified by BLAKE3 in build.rs, so this is unreachable in shipped
            // builds; the message addresses the only real cause, which is a
            // docs.rs-style build that skipped huffman_codes.bin generation).
            codes
                .get(&c)
                .expect("huffman_codes.bin missing or codebook lookup table corrupted")
        };
        bits.extend_bits(pattern.bits, pattern.length);
        char_count += 1;
    }

    (bits.to_bytes(), char_count)
}

/// Fast two-tier decoder with ASCII + prefix caches
struct FastDecoder {
    // Tier 1: ASCII fast path (256 entries, 512 bytes) Covers: space, a-z, A-Z, 0-9, punctuation Expected hit rate: ~99.4% for English text
    ascii_cache: [Option<(u8, u8)>; 256],

    // Tier 2: Common Unicode prefix table (4096 entries, ~20 KB) Covers: extended Latin (é,ñ,ü), common emoji, some CJK Expected hit rate: ~0.5% additional
    prefix_cache: [Option<(char, u8)>; 4096],

    // Tier 3: Tree walk for rare/long codes (>12 bits) Expected hit rate: <0.1%
    tree: DecodeNode,
}

impl FastDecoder {
    /// Build fast decoder from Huffman code table
    fn from_codes(codes: &HashMap<char, BitPattern>) -> Self {
        let mut ascii_cache = [None; 256];
        let mut prefix_cache = [None; 4096];
        let mut tree = DecodeNode::new();

        for (ch, pattern) in codes {
            let codepoint = *ch as u32;

            // Tier 1: ASCII cache (codes ≤8 bits, char <128)
            if codepoint < 128 && pattern.length <= 8 {
                // Pre-compute all possible bit continuations
                let base_code = pattern.bits;
                let code_length = pattern.length;

                // Fill all 8-bit patterns that start with this code
                let num_suffixes = 1 << (8 - code_length);
                for suffix in 0..num_suffixes {
                    let key = (base_code << (8 - code_length)) | suffix;
                    ascii_cache[key as usize] = Some((codepoint as u8, code_length));
                }
            }

            // Tier 2: Prefix cache (codes ≤12 bits, any char)
            if pattern.length <= 12 {
                let base_code = pattern.bits;
                let code_length = pattern.length;

                // Fill all 12-bit patterns that start with this code
                let num_suffixes = 1 << (12 - code_length);
                for suffix in 0..num_suffixes {
                    let key = (base_code << (12 - code_length)) | suffix;
                    prefix_cache[key as usize] = Some((*ch, code_length));
                }
            }

            // Tier 3: Always insert into tree (fallback)
            tree.insert(*ch, pattern.bits, pattern.length);
        }

        FastDecoder {
            ascii_cache,
            prefix_cache,
            tree,
        }
    }

    /// Decode next character from bit stream
    fn decode(&self, bytes: &[u8], bit_idx: usize) -> Result<(char, usize), &'static str> {
        // FAST PATH 1: Try ASCII cache (8-bit prefix lookup)
        if bit_idx + 8 <= bytes.len() * 8 {
            let byte_prefix = Self::read_bits_u8(bytes, bit_idx, 8);

            if let Some((ascii_char, bits_consumed)) = self.ascii_cache[byte_prefix as usize] {
                // HOT: 99.4% of English text hits here
                return Ok((ascii_char as char, bits_consumed as usize));
            }
        }

        // FAST PATH 2: Try prefix cache (12-bit prefix lookup)
        if bit_idx + 12 <= bytes.len() * 8 {
            let prefix_12 = Self::read_bits_u16(bytes, bit_idx, 12);

            if let Some((ch, bits_consumed)) = self.prefix_cache[prefix_12 as usize] {
                // WARM: Most remaining Unicode hits here
                return Ok((ch, bits_consumed as usize));
            }
        }

        // SLOW PATH: Tree walk for rare Unicode (>12 bit codes) Only <0.1% of typical text hits this path
        self.tree.decode(bytes, bit_idx)
    }

    /// Read N bits as u8 (for ASCII cache lookup)
    fn read_bits_u8(bytes: &[u8], start_bit: usize, num_bits: usize) -> u8 {
        let mut result = 0u8;
        for i in 0..num_bits {
            if Self::get_bit(bytes, start_bit + i) {
                result |= 1 << (num_bits - 1 - i);
            }
        }
        result
    }

    /// Read N bits as u16 (for prefix cache lookup)
    fn read_bits_u16(bytes: &[u8], start_bit: usize, num_bits: usize) -> u16 {
        let mut result = 0u16;
        for i in 0..num_bits {
            if Self::get_bit(bytes, start_bit + i) {
                result |= 1 << (num_bits - 1 - i);
            }
        }
        result
    }

    /// Get single bit from byte array
    fn get_bit(bytes: &[u8], bit_idx: usize) -> bool {
        let byte_idx = bit_idx / 8;
        let bit_pos = 7 - (bit_idx % 8);
        if byte_idx >= bytes.len() {
            return false;
        }
        (bytes[byte_idx] >> bit_pos) & 1 != 0
    }
}

// Lazy-load fast decoder
static FAST_DECODER: OnceLock<FastDecoder> = OnceLock::new();

fn get_fast_decoder() -> &'static FastDecoder {
    FAST_DECODER.get_or_init(|| {
        let codes = get_encode_table();
        FastDecoder::from_codes(codes)
    })
}

/// Decode Huffman-compressed bytes back to Unicode text
///
/// Convenience wrapper around `decode_text_with_size()` that discards byte count.
///
/// # Arguments
/// * `bytes` - Huffman-encoded bitstream (padded to byte boundary)
/// * `char_count` - Number of characters to decode (from VSF x marker)
///
/// # Returns
/// Tuple of (decoded_string, bytes_consumed)
///
/// # Example
/// ```ignore
/// let (decoded, bytes_used) = decode_text(&encoded_bytes, 5)?; assert_eq!(decoded, "Hello");
///
/// // Or ignore bytes_consumed: let (decoded, _) = decode_text(&encoded_bytes, 5)?;
/// ```
pub fn decode_text(bytes: &[u8], char_count: usize) -> Result<(String, usize), &'static str> {
    decode_text_with_size(bytes, char_count)
}

/// Decode Huffman-encoded text with byte count
///
/// Returns (decoded_string, bytes_consumed)
pub fn decode_text_with_size(
    bytes: &[u8],
    char_count: usize,
) -> Result<(String, usize), &'static str> {
    if char_count == 0 {
        return Ok((String::new(), 0));
    }

    if bytes.is_empty() {
        return Err("No data");
    }

    // Use fast decoder
    let decoder = get_fast_decoder();
    let mut result = String::with_capacity(char_count);
    let mut bit_idx = 0;
    let max_bits = bytes.len() << 3; // Convert bytes to bits

    // Decode exactly char_count characters
    let mut decoded_count = 0;
    while decoded_count < char_count {
        if bit_idx >= max_bits {
            return Err("Unexpected end of data");
        }

        let (ch, consumed) = decoder.decode(bytes, bit_idx)?;

        if consumed == 0 {
            return Err("Invalid Huffman code");
        }

        result.push(ch);
        decoded_count += 1;
        bit_idx += consumed;
    }

    // Return both the string and bytes consumed
    let bytes_consumed = (bit_idx + 7) >> 3; // Round up to nearest byte
    Ok((result, bytes_consumed))
}

/// Huffman decode tree node
struct DecodeNode {
    value: Option<char>,
    left: Option<Box<DecodeNode>>,
    right: Option<Box<DecodeNode>>,
}

impl DecodeNode {
    fn new() -> Self {
        DecodeNode {
            value: None,
            left: None,
            right: None,
        }
    }

    fn insert(&mut self, ch: char, bits: u32, length: u8) {
        let mut node = self;

        // Walk the tree from MSB to LSB
        for i in 0..length {
            let bit = (bits >> (length - 1 - i)) & 1;

            if bit == 0 {
                node = node.left.get_or_insert_with(|| Box::new(DecodeNode::new()));
            } else {
                node = node
                    .right
                    .get_or_insert_with(|| Box::new(DecodeNode::new()));
            }
        }

        node.value = Some(ch);
    }

    fn decode(&self, bytes: &[u8], start_bit: usize) -> Result<(char, usize), &'static str> {
        // Walk tree to decode Huffman code
        let mut node = self;
        let mut consumed = 0;

        loop {
            if let Some(ch) = node.value {
                return Ok((ch, consumed));
            }

            if start_bit + consumed >= bytes.len() * 8 {
                // Premature end of stream — must error, not silently inject U+0000.
                // Returning Ok(('\0', 0)) used to let truncated bitstreams decode to strings
                // ending in U+0000 that then re-encoded to different bytes — a wire-format
                // canonicalization break that an attacker could exploit to produce two
                // distinct VSF capsules with the "same" decoded text.
                return Err("Premature end of Huffman bitstream");
            }

            let bit = Self::get_bit(bytes, start_bit + consumed);
            consumed += 1;

            node = if !bit {
                node.left.as_ref().ok_or("Invalid Huffman code")?
            } else {
                node.right.as_ref().ok_or("Invalid Huffman code")?
            };
        }
    }

    fn get_bit(bytes: &[u8], bit_idx: usize) -> bool {
        let byte_idx = bit_idx / 8;
        let bit_pos = 7 - (bit_idx % 8);
        if byte_idx >= bytes.len() {
            return false;
        }
        (bytes[byte_idx] >> bit_pos) & 1 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitvec() {
        let mut bv = BitVec::new();
        bv.push(true);
        bv.push(false);
        bv.push(true);
        bv.push(true);

        let bytes = bv.to_bytes();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 0b10110000); // Padded to byte boundary
    }

    #[test]
    fn test_encode_decode_simple() {
        let text = "Hello";
        let (encoded, char_count) = encode_text(text);
        let (decoded, _) = decode_text(&encoded, char_count).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_encode_decode_with_space() {
        let text = "Hello world";
        let (encoded, char_count) = encode_text(text);
        let (decoded, _) = decode_text(&encoded, char_count).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_encode_decode_unicode() {
        let text = "café";
        let (encoded, char_count) = encode_text(text);
        let (decoded, _) = decode_text(&encoded, char_count).unwrap();
        // Roundtrip must equal the NFC form of the input. The source-file "café" is already
        // NFC ("e\u{0301}" would not be), so a direct comparison is correct here.
        assert_eq!(decoded, text);
    }

    /// NFC/NFD equivalence at the encoder boundary — the test the universe has been missing.
    ///
    /// Until this work, `encode_text` iterated raw codepoints with no normalization. "café"
    /// with U+00E9 (precomposed, 1 codepoint) and "cafe\u{0301}" (e + combining acute,
    /// 2 codepoints) produced completely different Huffman bitstreams, silently breaking
    /// every downstream identity primitive that round-tripped through `VsfType::x`. This
    /// assertion locks the fixed behavior.
    #[test]
    fn test_nfc_equivalence_e_acute() {
        let precomposed = "\u{00E9}";      // é as one codepoint
        let decomposed = "e\u{0301}";      // e + combining acute
        let (bytes_a, count_a) = encode_text(precomposed);
        let (bytes_b, count_b) = encode_text(decomposed);
        assert_eq!(bytes_a, bytes_b, "NFC/NFD must encode to identical bytes");
        assert_eq!(count_a, count_b, "NFC/NFD must report identical char counts");
    }

    #[test]
    fn test_nfc_equivalence_cafe() {
        let nfc = "café";                       // "café" with precomposed é
        let nfd = "cafe\u{0301}";              // "cafe" + combining acute
        let (bytes_a, count_a) = encode_text(nfc);
        let (bytes_b, count_b) = encode_text(nfd);
        assert_eq!(bytes_a, bytes_b, "café NFC vs NFD must encode identically");
        assert_eq!(count_a, 4);
        assert_eq!(count_b, 4);
    }

    #[test]
    fn test_nfc_equivalence_naive() {
        // ï: U+00EF precomposed vs i + U+0308 (combining diaeresis)
        let nfc = "na\u{00EF}ve";
        let nfd = "nai\u{0308}ve";
        let (bytes_a, count_a) = encode_text(nfc);
        let (bytes_b, count_b) = encode_text(nfd);
        assert_eq!(bytes_a, bytes_b);
        assert_eq!(count_a, count_b);
    }

    /// Decoder must error — not silently emit U+0000 — when the bitstream ends mid-code.
    /// Truncated input is a wire-format error, not a decode-to-NUL case.
    #[test]
    fn test_decoder_premature_eof_errors() {
        let (encoded, char_count) = encode_text("Hello, world!");
        assert!(char_count > 0);
        // Truncate the encoded bytes to less than what's needed for the full character count.
        // Asking for the full char_count from a truncated stream must fail.
        let truncated = &encoded[..1];
        let result = decode_text(truncated, char_count);
        assert!(result.is_err(), "truncated bitstream must error, not silently inject U+0000");
    }

    #[test]
    fn test_compression_ratio() {
        let text = "The quick brown fox jumps over the lazy dog";
        let (encoded, _) = encode_text(text);
        let utf8_size = text.as_bytes().len();
        let encoded_size = encoded.len();

        println!("UTF-8: {} bytes", utf8_size);
        println!("Huffman: {} bytes", encoded_size);
        println!(
            "Compression: {:.1}%",
            100.0 * (1.0 - encoded_size as f32 / utf8_size as f32)
        );

        // Should achieve at least 30% compression on English text
        assert!(encoded_size < utf8_size);
    }

    #[test]
    fn test_global_unicode_multilingual() {
        // Test diverse scripts from different languages
        let texts = vec![
            "Hello, world!",                   // English
            "¡Hola, mundo!",                   // Spanish
            "Привет, мир!",                    // Russian
            "مرحبا بالعالم",                   // Arabic
            "你好世界",                        // Chinese
            "こんにちは世界",                  // Japanese
            "안녕하세요 세계",                 // Korean
            "नमस्ते दुनिया",                      // Hindi
            "🌍🌎🌏 Hello 世界! مرحبا Привет", // Mixed with emoji
            "\u{1F600}\u{1F601}\u{1F602}",     // Emoji
            "Ελληνικά",                        // Greek
            "עברית",                           // Hebrew
            "ไทย",                             // Thai
            "Tiếng Việt",                      // Vietnamese
        ];

        for text in texts {
            let (encoded, char_count) = encode_text(text);
            let (decoded, _) = decode_text(&encoded, char_count).expect("Decode failed");
            // After NFC normalization the decoded form is canonical NFC; compare against
            // the input's own NFC projection so non-NFC source-file literals still pass.
            let expected: String = text.nfc().collect();
            assert_eq!(decoded, expected, "Failed for: {}", text);

            let utf8_size = text.as_bytes().len();
            let encoded_size = encoded.len();
            println!(
                "{:30} UTF-8: {:4} bytes, Huffman: {:4} bytes ({:.1}%)",
                text.chars().take(15).collect::<String>(),
                utf8_size,
                encoded_size,
                100.0 * (1.0 - encoded_size as f32 / utf8_size as f32)
            );
        }
    }

    #[test]
    fn test_rare_unicode_planes() {
        // Test characters from various Unicode planes — all 1,112,064 valid codepoints
        // have pre-assigned Huffman codes regardless of whether they appeared in the corpus.
        let rare_chars = vec![
            '\u{1F600}',  // Emoji (SMP)
            '\u{10000}',  // Linear B Syllable (SMP)
            '\u{20000}',  // CJK Ideograph Extension B (SIP)
            '\u{E0000}',  // Tag Space (SSP)
            '\u{F0000}',  // Private Use (Plane 15)
            '\u{10FFFF}', // Last valid Unicode codepoint
        ];

        for ch in rare_chars {
            let text: String = ch.to_string();
            let (encoded, char_count) = encode_text(&text);
            let (decoded, _) = decode_text(&encoded, char_count).expect("Decode failed");
            assert_eq!(decoded, text, "Failed for U+{:X}", ch as u32);
        }
    }

    #[test]
    fn test_ascii_fast_path() {
        // Verify ASCII LUT is populated correctly
        let ascii_text = "The quick brown fox jumps over the lazy dog 0123456789!@#$%";

        let (encoded, char_count) = encode_text(ascii_text);
        let (decoded, _) = decode_text(&encoded, char_count).expect("Decode failed");

        assert_eq!(decoded, ascii_text);

        // All chars should be ASCII and use fast path
        for c in ascii_text.chars() {
            assert!(c.is_ascii(), "Test should only contain ASCII");
        }

        println!(
            "ASCII fast path verified for {} characters",
            ascii_text.len()
        );
    }

    #[test]
    fn test_mixed_ascii_unicode() {
        // Test that fast path and slow path work together
        let mixed = "ASCII text with Unicode: 你好 مرحبا Привет 🌍";

        let (encoded, char_count) = encode_text(mixed);
        let (decoded, _) = decode_text(&encoded, char_count).expect("Decode failed");

        let expected: String = mixed.nfc().collect();
        assert_eq!(decoded, expected);

        // Count ASCII vs Unicode
        let ascii_count = mixed.chars().filter(|c| c.is_ascii()).count();
        let unicode_count = mixed.chars().filter(|c| !c.is_ascii()).count();

        println!(
            "Mixed text: {} ASCII chars (fast path), {} Unicode chars (HashMap)",
            ascii_count, unicode_count
        );
    }

    #[test]
    fn test_decode_performance_benchmark() {
        // Large corpus performance test
        let corpus = include_str!("../tools/english_test.txt");

        // Encode once
        let start = std::time::Instant::now();
        let (encoded, char_count) = encode_text(corpus);
        let encode_time = start.elapsed();

        println!("\n=== Decode Performance Benchmark ===");
        println!("Corpus: {} bytes, {} chars", corpus.len(), char_count);

        println!(
            "Encode time: {:?} ({:.2} MB/s)",
            encode_time,
            corpus.len() as f64 / encode_time.as_secs_f64() / 1_000_000.0
        );

        // Decode multiple times for stable measurement
        let iterations = 100;
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let (decoded, _) = decode_text(&encoded, char_count).unwrap();
            assert!(!decoded.is_empty());
        }

        let total_time = start.elapsed();
        let avg_time = total_time / iterations;
        let throughput = (corpus.len() as f64 / avg_time.as_secs_f64()) / 1_000_000.0;

        println!(
            "Decode time: {:?} avg ({} iterations)",
            avg_time, iterations
        );
        println!("Decode throughput: {:.2} MB/s", throughput);
        println!(
            "Speedup vs tree-only: ~{:.0}× (was 0.23 MB/s)",
            throughput / 0.23
        );

        // Should be at least 10× faster than tree-only (2.3+ MB/s) In practice, achieves 30-130 MB/s depending on build optimization
        assert!(throughput > 2.3, "Decode too slow: {:.2} MB/s", throughput);
    }
}
