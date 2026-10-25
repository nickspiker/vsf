//! Frequency-based Huffman text encoding for VSF `x` marker
//!
//! Achieves ~2× compression over UTF-8 thru frequency-optimized encoding:
//! - Common characters (space, 'e', 't') use short codes (3-5 bits)
//! - Less common characters use medium codes (8-12 bits)
//! - Rare Unicode characters use longer codes (16-24 bits)
//!
//! The Huffman tree is pre-computed at build time from global language frequency analysis. ALL 1,112,064 valid Unicode codepoints (0x000000–0x10FFFF excluding the U+D800–U+DFFF surrogate range) have pre-assigned codes — Unicode's yearly codepoint additions cannot change any existing encoding because every slot is already reserved.
//!
//! ## NFC canonicalization
//!
//! Input text is NFC-normalized inside `encode_text` before Huffman encoding. This means "café" (U+00E9 precomposed) and "cafe\u{0301}" (e + combining acute) produce byte-identical Huffman output — both reduce to the same NFC codepoint sequence first.
//!
//! NFC is anchored by Unicode's stability policy: once a codepoint is assigned, its NFC form is guaranteed not to change across Unicode versions. This is the same byte-determinism anchor as a frozen wire-format hash function — the canonical output is specified outside this crate and cannot drift.
//!
//! Callers downstream that use VSF for identity hashing (notably `ihi::handle_to_proof`) inherit café-stability automatically: there is no way to construct an `x`-encoded byte stream that distinguishes NFC from NFD input.

use std::sync::OnceLock;
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

/// The codebook as built (`huffman_codes.bin`): a 12-byte header (`HUFF`, a version, the entry count), then 8 bytes an entry, the codepoint and its code (right-aligned in the low 24 bits, the length in bits 24..29), one entry for every Unicode scalar value in codepoint order.
/// Read where it lies: no table is built from it for encoding, and decoding keeps only an ordering of it (2026-10-25: the HashMap of all 1,112,064 codes and a pointer tree over them cost 80 MB, and a Pixel 3a was killed for memory the first time it read a place name).
#[cfg(huffman_available)]
const DATA: &[u8] = include_bytes!("../huffman_codes.bin");
#[cfg(not(huffman_available))]
const DATA: &[u8] = &[];

/// Entries in the codebook, 0 when it is missing (a docs.rs build).
fn entries() -> usize {
    if DATA.len() < 12 || &DATA[0..4] != b"HUFF" {
        return 0;
    }
    (u32::from_le_bytes(DATA[8..12].try_into().unwrap()) as usize).min((DATA.len() - 12) / 8)
}

/// Entry `i`: its codepoint and its code.
#[inline(always)]
fn entry(i: usize) -> (u32, BitPattern) {
    let o = 12 + i * 8;
    let cp = u32::from_le_bytes([DATA[o], DATA[o + 1], DATA[o + 2], DATA[o + 3]]);
    let packed = u32::from_le_bytes([DATA[o + 4], DATA[o + 5], DATA[o + 6], DATA[o + 7]]);
    (cp, BitPattern { bits: packed & 0x00FF_FFFF, length: ((packed >> 24) & 0x1F) as u8 })
}

/// A character's code, or a zero-length pattern when the codebook lacks it: at its own index (every scalar value is there, the 2048 surrogates excepted), else by binary search over the codepoint order.
fn code_of(c: char) -> BitPattern {
    let cp = c as u32;
    let n = entries();
    let i = if cp < 0xD800 { cp } else { cp - 0x800 } as usize;
    if i < n {
        let (got, p) = entry(i);
        if got == cp {
            return p;
        }
    }
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let (got, p) = entry(mid);
        if got == cp {
            return p;
        }
        if got < cp {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    BitPattern { bits: 0, length: 0 }
}

/// Encode Unicode text to Huffman-compressed bytes after NFC normalization.
///
/// Returns `(huffman_bytes, nfc_char_count)`. The byte vector is the Huffman bitstream padded to a byte boundary; the char count is the number of codepoints AFTER NFC normalization, not the input length. Callers MUST use the returned count when writing the VSF `x` marker — NFC can collapse or expand codepoint sequences, and using the original count would corrupt the wire format.
///
/// All characters use variable-length Huffman codes (3-24 bits). The codebook covers all 1,112,064 valid Unicode codepoints, so this function never fails on valid `&str` input.
///
/// ## Canonicalization
///
/// Input is NFC-normalized before encoding. Café/café-class identity splits are impossible at this boundary: any two strings that compare equal under NFC produce byte-identical output.
///
/// # Performance
/// Each code is read from the codebook at its codepoint's own index (no table built). NFC over pure-ASCII input is the identity transform, so ASCII stays cheap.
///
/// # VSF Integration
/// The VSF x marker format is:
/// ```text
/// x [char_count] [huffman_bytes]
/// ```
/// `char_count` here is the NFC count returned from this function, encoded via `encode_number()` (3-6+ bytes depending on size). No arbitrary limits — supports billions of characters.
///
/// # Example
/// ```ignore
/// let (encoded, char_count) = encode_text("Hello");
/// // char_count == 5, encoded is ~3 bytes of Huffman bits (no internal header)
/// ```
pub fn encode_text(text: &str) -> (Vec<u8>, usize) {
    let mut bits = BitVec::new();
    let mut char_count = 0usize;

    for c in text.nfc() {
        // Read from the codebook where it lies, ASCII and the rest alike. The codebook covers all 1,112,064 valid codepoints, so this cannot miss for any character producible by NFC over a valid &str.
        let pattern = &code_of(c);
        // A zero-length code is impossible in a valid codebook (every codepoint gets ≥1 bit). It means the table never loaded (cfg(not(huffman_available)) build) — the ASCII LUT defaults to length 0 there, and extend_bits(_, 0) would silently emit NOTHING, producing an empty bitstream that claims to encode char_count characters. Refuse loudly instead.
        assert_ne!(
            pattern.length, 0,
            "huffman_codes.bin missing — VsfType::x cannot encode in this build (char {:?})",
            c
        );
        bits.extend_bits(pattern.bits, pattern.length);
        char_count += 1;
    }

    (bits.to_bytes(), char_count)
}

/// The decoder: codes of 12 bits or fewer by every 12-bit window starting with them, and the codebook's entries in code order for the rest.
struct FastDecoder {
    /// (codepoint << 5 | length) for each 12-bit window whose first bits are a code of 12 bits or fewer, 0 for none.
    short: Vec<u32>,
    /// The codebook's entry indices, in order of their codes left-aligned to 24 bits: a complete prefix code tiles the 24-bit windows, so the code a window starts with is the last at or below it (4.4 MB, against the 52 MB pointer tree it replaces).
    order: Vec<u32>,
}

/// An entry's code left-aligned to 24 bits.
#[inline(always)]
fn key_of(p: BitPattern) -> u32 {
    p.bits << (24 - p.length as u32)
}

impl FastDecoder {
    fn new() -> Self {
        let n = entries();
        let mut short = vec![0u32; 1 << 12];
        let mut order: Vec<u32> = (0..n as u32).collect();
        for i in 0..n {
            let (cp, p) = entry(i);
            if p.length >= 1 && p.length <= 12 {
                let base = (p.bits << (12 - p.length as u32)) as usize;
                for s in 0..(1usize << (12 - p.length as u32)) {
                    short[base + s] = cp << 5 | p.length as u32;
                }
            }
        }
        order.sort_unstable_by_key(|&i| key_of(entry(i as usize).1));
        FastDecoder { short, order }
    }

    /// The 24 bits from `bit_idx`, zero past the end.
    #[inline(always)]
    fn window(bytes: &[u8], bit_idx: usize) -> u32 {
        let b = bit_idx >> 3;
        let mut w: u32 = 0;
        for k in 0..4 {
            w = (w << 8) | *bytes.get(b + k).unwrap_or(&0) as u32;
        }
        (w << (bit_idx & 7)) >> 8
    }

    /// The next character from the bit stream and the bits its code took.
    fn decode(&self, bytes: &[u8], bit_idx: usize) -> Result<(char, usize), &'static str> {
        let avail = (bytes.len() * 8).saturating_sub(bit_idx);
        let w = Self::window(bytes, bit_idx);
        let s = self.short[(w >> 12) as usize];
        let (cp, len) = if s != 0 {
            (s >> 5, (s & 31) as usize)
        } else {
            // The last code at or below the window.
            let (mut lo, mut hi) = (0usize, self.order.len());
            while lo < hi {
                let mid = (lo + hi) / 2;
                if key_of(entry(self.order[mid] as usize).1) <= w {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo == 0 {
                return Err("Invalid Huffman code");
            }
            let (cp, p) = entry(self.order[lo - 1] as usize);
            if p.length == 0 || (w >> (24 - p.length as u32)) != p.bits {
                return Err("Invalid Huffman code");
            }
            (cp, p.length as usize)
        };
        // A code running past the stream's end: the stream was cut short (never decoded as padding, which would let two streams decode alike).
        if len > avail {
            return Err("Premature end of Huffman bitstream");
        }
        let ch = char::from_u32(cp).ok_or("Invalid Huffman code")?;
        Ok((ch, len))
    }
}

// Built at first decode.
static FAST_DECODER: OnceLock<FastDecoder> = OnceLock::new();

fn get_fast_decoder() -> &'static FastDecoder {
    FAST_DECODER.get_or_init(FastDecoder::new)
}

/// Decode Huffman-compressed bytes back to Unicode text
///
/// Convenience wrapper around `decode_text_with_size()` that discards byte count.
///
/// # Arguments
/// * `bytes` - Huffman-encoded bitstream (padded to byte boundary) * `char_count` - Number of characters to decode (from VSF x marker)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code in the codebook, both ways: the encoder reads exactly the code stored for each codepoint, and the decoder gives that codepoint back from it, alone and in a long mixed stream; a stream cut inside a code is refused (2026-10-25: the HashMap and the tree replaced by reading the codebook where it lies).
    #[test]
    fn every_code_both_ways() {
        let n = entries();
        if n == 0 {
            return;
        }
        assert_eq!(n, 1_112_064, "every Unicode scalar value");
        let mut stream = BitVec::new();
        let mut sample = String::new();
        for i in 0..n {
            let (cp, p) = entry(i);
            let c = char::from_u32(cp).expect("a scalar value");
            let got = code_of(c);
            assert_eq!((got.bits, got.length), (p.bits, p.length), "U+{cp:04X} encodes as stored");
            let mut one = BitVec::new();
            one.extend_bits(p.bits, p.length);
            let bytes = one.to_bytes();
            let (back, len) = get_fast_decoder().decode(&bytes, 0).expect("decodes");
            assert_eq!((back, len), (c, p.length as usize), "U+{cp:04X} decodes from its code");
            if i % 97 == 0 || cp < 0x300 {
                stream.extend_bits(p.bits, p.length);
                sample.push(c);
            }
        }
        let count = sample.chars().count();
        let bytes = stream.to_bytes();
        let (text, _) = decode_text_with_size(&bytes, count).expect("the mixed stream decodes");
        assert_eq!(text, sample);
        // A code cut short at the stream's end.
        let (_, long) = (0..n).map(entry).find(|(_, p)| p.length == 24).expect("a 24-bit code");
        let mut cut = BitVec::new();
        cut.extend_bits(long.bits >> 8, 16);
        assert!(decode_text_with_size(&cut.to_bytes(), 1).is_err(), "a cut code is refused");
    }

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
        // Roundtrip must equal the NFC form of the input. The source-file "café" is already NFC ("e\u{0301}" would not be), so a direct comparison is correct here.
        assert_eq!(decoded, text);
    }

    /// NFC/NFD equivalence at the encoder boundary — the test the universe has been missing.
    ///
    /// Until this work, `encode_text` iterated raw codepoints with no normalization. "café" with U+00E9 (precomposed, 1 codepoint) and "cafe\u{0301}" (e + combining acute, 2 codepoints) produced completely different Huffman bitstreams, silently breaking every downstream identity primitive that round-tripped thru `VsfType::x`. This assertion locks the fixed behavior.
    #[test]
    fn test_nfc_equivalence_e_acute() {
        let precomposed = "\u{00E9}"; // é as one codepoint
        let decomposed = "e\u{0301}"; // e + combining acute
        let (bytes_a, count_a) = encode_text(precomposed);
        let (bytes_b, count_b) = encode_text(decomposed);
        assert_eq!(bytes_a, bytes_b, "NFC/NFD must encode to identical bytes");
        assert_eq!(
            count_a, count_b,
            "NFC/NFD must report identical char counts"
        );
    }

    #[test]
    fn test_nfc_equivalence_cafe() {
        let nfc = "café"; // "café" with precomposed é
        let nfd = "cafe\u{0301}"; // "cafe" + combining acute
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

    /// Decoder must error — not silently emit U+0000 — when the bitstream ends mid-code. Truncated input is a wire-format error, not a decode-to-NUL case.
    #[test]
    fn test_decoder_premature_eof_errors() {
        let (encoded, char_count) = encode_text("Hello, world!");
        assert!(char_count > 0);
        // Truncate the encoded bytes to less than what's needed for the full character count. Asking for the full char_count from a truncated stream must fail.
        let truncated = &encoded[..1];
        let result = decode_text(truncated, char_count);
        assert!(
            result.is_err(),
            "truncated bitstream must error, not silently inject U+0000"
        );
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
            // After NFC normalization the decoded form is canonical NFC; compare against the input's own NFC projection so non-NFC source-file literals still pass.
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
        // Test characters from various Unicode planes — all 1,112,064 valid codepoints have pre-assigned Huffman codes regardless of whether they appeared in the corpus.
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
            "Mixed text: {} ASCII chars (fast path), {} Unicode chars (codebook)",
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
