//! The no_std x text variants (src/x_text.rs) against the std encoder and decoder (src/text_encoding.rs): the same bytes and the same NFC count for every string a variant supports, a clean refusal for any other.
//!
//! With `text-encode` on (the comparison): `cargo test --release --features text-encode,x-compact,x-latin,x-lang16,x-full --test x_variants`.
//! Without it, `VsfType::x` itself goes thru the variant (the wiring): `cargo test --no-default-features --features x-lang16 --test x_variants` (or any one x-* feature).

#![cfg(any(feature = "x-compact", feature = "x-latin", feature = "x-lang16", feature = "x-full"))]

/// The pinned codebook, read here only as the reference the variants are checked against.
const CODEBOOK: &[u8] = include_bytes!("../huffman_codes.bin");

/// Every codebook entry: the character, its code right-aligned, its length.
fn entries() -> impl Iterator<Item = (char, u32, u32)> {
    let n = u32::from_le_bytes(CODEBOOK[8..12].try_into().unwrap()) as usize;
    (0..n).map(|i| {
        let o = 12 + i * 8;
        let cp = u32::from_le_bytes(CODEBOOK[o..o + 4].try_into().unwrap());
        let packed = u32::from_le_bytes(CODEBOOK[o + 4..o + 8].try_into().unwrap());
        (char::from_u32(cp).unwrap(), packed & 0x00FF_FFFF, packed >> 24)
    })
}

/// Text in each of ferros's sixteen languages (ferros_lang: de en es fil fr hi id it mi pl pt ru sw tr uk vi), with the punctuation each uses.
const SIXTEEN: &[&str] = &[
    "Größe: „Straße“ – ẞ und Äpfel… Übermäßig!",
    "The quick brown fox jumps over the lazy dog (1, 2, 3) – “quoted” ‘text’ & more…",
    "¿Dónde está el niño? ¡Mañana! Año, corazón, pingüino.",
    "Magandang umaga! Kumusta ka? Salamat po, Ñiño.",
    "« Œuvre » à l’été : ça, où, naïve, Noël, cœur.",
    "नमस्ते दुनिया! क़िला, ज़रूर, फ़ोन, ड़, ढ़ – हिन्दी। ₹१२३",
    "Selamat pagi, apa kabar? Terima kasih.",
    "Perché è così? Più città, caffè.",
    "Kia ora koutou! Māori, whānau, tēnā koe, kōrero, hūrō.",
    "Zażółć gęślą jaźń. Łódź, Kraków, źródło.",
    "Não, coração, ação, avô, à tarde.",
    "Съешь же ещё этих мягких французских булок, да выпей чаю. «Ёлка» – ₽",
    "Habari za asubuhi? Asante sana, rafiki.",
    "Iğdır, İstanbul, şeker, çiçek, ğüöı. ₺",
    "Ґанок, їжак, єнот, ім’я, пʼять. ₴",
    "Tiếng Việt: Xin chào, cảm ơn, người, được, ừ, ở đây. ₫",
];

/// Strings of many scripts, emoji and planes (the std module's own test list and more).
const MIXED: &[&str] = &[
    "Hello, world!",
    "مرحبا بالعالم",
    "你好世界",
    "こんにちは世界",
    "안녕하세요 세계",
    "🌍🌎🌏 Hello 世界! مرحبا Привет",
    "\u{1F600}\u{1F601}\u{1F602}",
    "Ελληνικά",
    "עברית",
    "ไทย ภาษาไทย น่ารัก",
    "x^2 + y^2 = r^2",
    "tab\there\r\nline",
    "\u{10000}\u{20000}\u{E0000}\u{F0000}\u{10FFFF}",
    "",
];

/// Decomposed and out-of-order input: NFC must fold each to the same bytes as its composed form.
const DECOMPOSED: &[&str] = &[
    "cafe\u{0301}",
    "nai\u{0308}ve",
    "Vie\u{0302}\u{0323}t",
    "Vie\u{0323}\u{0302}t",
    "\u{0958}\u{0915}\u{093C}",
    "\u{212B}ngstro\u{0308}m \u{2126}",
    "\u{0E19}\u{0E48}\u{094D}\u{0E49}",
    "\u{1100}\u{1161}\u{11A8}",
    "<\u{0338}",
];

#[cfg(feature = "text-encode")]
mod against_std {
    use super::*;
    use unicode_normalization::char::{canonical_combining_class, decompose_canonical};
    use unicode_normalization::{is_nfc_quick, IsNormalized, UnicodeNormalization};
    use vsf::text_encoding::{decode_text_with_size as std_decode, encode_text as std_encode};
    use vsf::x_text::Unsupported;

    type Encode = fn(&str) -> Result<(Vec<u8>, usize), Unsupported>;
    type Decode = fn(&[u8], usize) -> Result<(String, usize), &'static str>;

    /// One variant: its name, its two halves, what it holds, and whether it normalizes (compact refuses what NFC would fold instead).
    struct Variant {
        name: &'static str,
        encode: Encode,
        decode: Decode,
        holds: fn(char) -> bool,
        nfc: bool,
    }

    fn variants() -> Vec<Variant> {
        let mut v = Vec::new();
        #[cfg(feature = "x-compact")]
        v.push(Variant {
            name: "compact",
            encode: vsf::x_text::compact::encode,
            decode: vsf::x_text::compact::decode_text_with_size,
            holds: vsf::x_text::compact::holds,
            nfc: false,
        });
        #[cfg(feature = "x-latin")]
        v.push(Variant {
            name: "latin",
            encode: vsf::x_text::latin::encode,
            decode: vsf::x_text::latin::decode_text_with_size,
            holds: vsf::x_text::latin::holds,
            nfc: true,
        });
        #[cfg(feature = "x-lang16")]
        v.push(Variant {
            name: "lang16",
            encode: vsf::x_text::lang16::encode,
            decode: vsf::x_text::lang16::decode_text_with_size,
            holds: vsf::x_text::lang16::holds,
            nfc: true,
        });
        #[cfg(feature = "x-full")]
        v.push(Variant {
            name: "full",
            encode: vsf::x_text::full::encode,
            decode: vsf::x_text::full::decode_text_with_size,
            holds: vsf::x_text::full::holds,
            nfc: true,
        });
        v
    }

    /// The contract on one string: if the variant holds every character it must (of the NFC form, or of the input for compact), its bytes and count are the std encoder's and its decode of them is the std decoder's; otherwise it refuses, naming a character it lacks, and refuses the std bytes too.
    fn check(v: &Variant, s: &str) -> bool {
        let (bytes, count) = std_encode(s);
        let nfc: String = s.nfc().collect();
        let held = if v.nfc { nfc.chars().all(v.holds) } else { s.chars().all(v.holds) };
        match (v.encode)(s) {
            Ok(got) => {
                assert!(held, "{}: accepted {s:?}, which holds a character it lacks", v.name);
                assert_eq!(got, (bytes.clone(), count), "{}: bytes or count differ from std for {s:?}", v.name);
            }
            Err(Unsupported(c)) => {
                assert!(!held, "{}: refused {s:?} (U+{:04X}), which it holds", v.name, c as u32);
                assert!(!(v.holds)(c), "{}: refused U+{:04X}, which it holds", v.name, c as u32);
            }
        }
        let want = std_decode(&bytes, count).expect("std decodes its own bytes");
        let all_held = nfc.chars().all(v.holds);
        match (v.decode)(&bytes, count) {
            Ok(got) => {
                assert!(all_held, "{}: decoded a stream holding a code it lacks ({s:?})", v.name);
                assert_eq!(got, want, "{}: decode differs from std for {s:?}", v.name);
            }
            Err(_) => assert!(!all_held, "{}: refused to decode {s:?}, which it holds", v.name),
        }
        held
    }

    /// Every Unicode scalar value alone, thru every variant: held ones encode and decode exactly as std, others refuse both ways.
    #[test]
    fn every_scalar_value() {
        let mut buf = [0u8; 4];
        for v in variants() {
            let mut held = 0usize;
            for (c, _, _) in entries() {
                if check(&v, c.encode_utf8(&mut buf)) {
                    held += 1;
                }
            }
            println!("{}: {held} scalar values encode alone", v.name);
        }
    }

    /// Each variant round-trips every character it holds, in one string, and agrees with std on it.
    #[test]
    fn round_trip_whole_range() {
        for v in variants() {
            let all: String = entries().map(|e| e.0).filter(|&c| (v.holds)(c)).collect();
            assert!(check(&v, &all), "{}: its own range must encode", v.name);
            let (bytes, count) = (v.encode)(&all).unwrap();
            let (back, used) = (v.decode)(&bytes, count).unwrap();
            assert_eq!(back, all.nfc().collect::<String>(), "{}: round trip", v.name);
            assert_eq!(used, bytes.len(), "{}: bytes consumed", v.name);
        }
    }

    /// Languages, scripts, emoji and decomposed input: each variant matches std or refuses; the widest variants accept what they promise.
    #[test]
    fn corpora() {
        let english = include_str!("../tools/english_test.txt");
        for v in variants() {
            for s in SIXTEEN.iter().chain(MIXED).chain(DECOMPOSED).copied().chain([english]) {
                check(&v, s);
            }
            if v.name == "lang16" || v.name == "full" {
                for s in SIXTEEN {
                    assert!(check(&v, s), "{}: must hold {s:?}", v.name);
                }
            }
            if v.name == "latin" {
                for s in SIXTEEN
                    .iter()
                    .filter(|s| !s.chars().any(|c| ('\u{0400}'..='\u{097F}').contains(&c)))
                {
                    assert!(check(&v, s), "latin: must hold {s:?}");
                }
            }
            if v.name == "full" {
                for s in MIXED.iter().chain(DECOMPOSED) {
                    assert!(check(&v, s), "full: must hold {s:?}");
                }
            }
        }
    }

    /// The count is the NFC count: composed and decomposed input encode alike.
    #[test]
    fn nfc_count() {
        for v in variants() {
            let composed = (v.encode)("caf\u{E9}").unwrap();
            assert_eq!(composed.1, 4, "{}", v.name);
            if v.nfc {
                assert_eq!((v.encode)("cafe\u{0301}").unwrap(), composed, "{}: NFC folds e + U+0301", v.name);
                assert_eq!(
                    (v.encode)("Vie\u{0323}\u{0302}t").unwrap(),
                    (v.encode)("Vi\u{1EC7}t").unwrap(),
                    "{}: marks in either order",
                    v.name
                );
            } else {
                assert_eq!(
                    (v.encode)("cafe\u{0301}"),
                    Err(Unsupported('\u{0301}')),
                    "compact refuses the mark it lacks"
                );
            }
        }
        #[cfg(feature = "x-lang16")]
        {
            // U+0958 is a composition exclusion: NFC splits it into U+0915 U+093C, two characters.
            let split = vsf::x_text::lang16::encode("\u{0958}").unwrap();
            assert_eq!(split, std_encode("\u{0958}"));
            assert_eq!(split.1, 2);
        }
    }

    /// What lies outside a variant is refused, on encode with the character named and on decode as a whole string.
    #[test]
    fn refusals() {
        for v in variants() {
            for c in ['^', '€', 'Ж', '\u{0462}', '\u{0958}', '世', '😎', '\u{10FFFF}'] {
                let s = alloc_string(['a', c, 'b']);
                let held = if v.nfc { s.nfc().all(v.holds) } else { s.chars().all(v.holds) };
                let got = (v.encode)(&s);
                assert_eq!(got.is_err(), !held, "{}: U+{:04X}", v.name, c as u32);
                let (bytes, count) = std_encode(&s);
                assert_eq!(
                    (v.decode)(&bytes, count).is_err(),
                    !s.nfc().all(v.holds),
                    "{}: decode U+{:04X}",
                    v.name,
                    c as u32
                );
            }
        }
        #[cfg(feature = "x-compact")]
        assert_eq!(vsf::x_text::compact::encode("a^b"), Err(Unsupported('^')));
        #[cfg(feature = "x-latin")]
        assert_eq!(vsf::x_text::latin::encode("\u{0462}ть"), Err(Unsupported('\u{0462}')));
        #[cfg(feature = "x-lang16")]
        assert_eq!(vsf::x_text::lang16::encode("ok 世界"), Err(Unsupported('世')));
    }

    fn alloc_string(cs: [char; 3]) -> String {
        cs.iter().collect()
    }

    /// The facts that let compact skip NFC tables (module docs): every character is NFC_Quick_Check=Yes; the marks are exactly `compact::MARKS`; every mark in a character's decomposition has a class above every compact mark's.
    #[cfg(feature = "x-compact")]
    #[test]
    fn compact_nfc_facts() {
        use vsf::x_text::compact::{holds, MARKS};
        let top = MARKS.iter().map(|m| m.1).max().unwrap();
        let set: Vec<char> = entries().map(|e| e.0).filter(|&c| holds(c)).collect();
        assert_eq!(set.len(), 712);
        for &c in &set {
            assert_eq!(is_nfc_quick(core::iter::once(c)), IsNormalized::Yes, "U+{:04X}", c as u32);
            let class = canonical_combining_class(c);
            assert_eq!(
                class,
                MARKS.iter().find(|m| m.0 == c).map_or(0, |m| m.1),
                "U+{:04X} class",
                c as u32
            );
            decompose_canonical(c, |d| {
                let k = canonical_combining_class(d);
                assert!(d == c || k == 0 || k > top, "U+{:04X} decomposes to a mark of class {k}", c as u32);
            });
        }
    }

    /// Every pair of compact characters, and every starter followed by every sequence of up to three compact marks, encodes exactly as std.
    #[cfg(feature = "x-compact")]
    #[test]
    fn compact_pairs_and_mark_runs() {
        use vsf::x_text::compact::{encode, holds, MARKS};
        let set: Vec<char> = entries().map(|e| e.0).filter(|&c| holds(c)).collect();
        let mut s = String::new();
        for &a in &set {
            for &b in &set {
                s.clear();
                s.push(a);
                s.push(b);
                assert_eq!(encode(&s).unwrap(), std_encode(&s), "U+{:04X} U+{:04X}", a as u32, b as u32);
            }
        }
        let marks: Vec<char> = MARKS.iter().map(|m| m.0).collect();
        let mut runs: Vec<Vec<char>> = vec![vec![]];
        for _ in 0..3 {
            let longer: Vec<Vec<char>> = runs
                .iter()
                .flat_map(|r| marks.iter().map(move |&m| [r.clone(), vec![m]].concat()))
                .collect();
            runs.extend(longer.into_iter().filter(|r| r.len() <= 3));
            runs.sort();
            runs.dedup();
        }
        for &a in &set {
            for r in &runs {
                s.clear();
                s.push(a);
                s.extend(r);
                s.push(a);
                s.extend(r.iter().rev());
                assert_eq!(encode(&s).unwrap(), std_encode(&s), "{s:?}");
            }
        }
    }

    /// The tables' sizes, printed for the record.
    #[test]
    fn sizes() {
        #[cfg(feature = "x-compact")]
        println!(
            "compact: {} characters, {} bytes",
            vsf::x_text::compact::len(),
            vsf::x_text::compact::size()
        );
        #[cfg(feature = "x-latin")]
        println!(
            "latin: {} characters, {} bytes",
            vsf::x_text::latin::len(),
            vsf::x_text::latin::size()
        );
        #[cfg(feature = "x-lang16")]
        println!(
            "lang16: {} characters, {} bytes",
            vsf::x_text::lang16::len(),
            vsf::x_text::lang16::size()
        );
        #[cfg(feature = "x-full")]
        println!("full: 1112064 characters, {} bytes", vsf::x_text::full::size());
    }

    /// With the std path on, VsfType::x still uses it; the widest variant's bytes are the same.
    #[test]
    fn vsf_type_x_same_bytes() {
        for s in SIXTEEN.iter().chain(["café", "a"].iter()) {
            if let Ok((bytes, count)) = vsf::x_text::encode(s) {
                let flat = vsf::VsfType::x(s.to_string()).flatten();
                let mut want = vec![b'x'];
                want.extend_from_slice(&vsf::EncodeNumber::encode_number(&count));
                want.extend_from_slice(&bytes);
                assert_eq!(flat, want, "{s:?}");
            }
        }
    }
}

/// Without the std text path, VsfType::x flattens and parses thru the widest x-* variant enabled.
#[cfg(not(any(feature = "text", feature = "text-encode")))]
mod wiring {
    use super::*;
    use vsf::{parse, VsfType};

    /// The reference: each character's code read straight from the codebook (input already in NFC).
    fn reference(s: &str) -> Vec<u8> {
        let codes: Vec<(char, u32, u32)> = entries().collect();
        let mut bits: Vec<bool> = Vec::new();
        for c in s.chars() {
            let i = if (c as u32) < 0xD800 { c as u32 } else { c as u32 - 0x800 } as usize;
            let (_, b, l) = codes[i];
            for k in (0..l).rev() {
                bits.push(b >> k & 1 == 1);
            }
        }
        let mut out = vec![b'x'];
        out.extend_from_slice(&vsf::EncodeNumber::encode_number(&s.chars().count()));
        for chunk in bits.chunks(8) {
            out.push(chunk.iter().enumerate().fold(0u8, |a, (k, &b)| a | (b as u8) << (7 - k)));
        }
        out
    }

    #[test]
    fn flatten_and_parse() {
        for s in ["Hello, VSF!", "café crème", "Привет", "नमस्ते", "", "\n"] {
            let x = VsfType::x(s.to_string());
            match x.try_flatten() {
                Ok(flat) => {
                    assert_eq!(flat, reference(s), "{s:?}");
                    let mut at = 0;
                    match parse(&flat, &mut at) {
                        Ok(VsfType::x(back)) => assert_eq!(back, s),
                        _ => panic!("{s:?} did not parse back as x"),
                    }
                    assert_eq!(at, flat.len());
                }
                // Only a variant lacking a character may refuse; ASCII letters are in every one.
                Err(e) => assert!(!s.is_ascii(), "{s:?} refused: {e}"),
            }
        }
    }

    #[test]
    fn refusal_is_an_error_not_a_panic() {
        #[cfg(not(feature = "x-full"))]
        assert!(VsfType::x("世界 \u{10FFFF}".to_string()).try_flatten().is_err());
        // A stream holding a code no variant but full holds: refused whole on parse.
        let wide = reference("\u{10FFFF}");
        let mut at = 0;
        #[cfg(not(feature = "x-full"))]
        assert!(parse(&wide, &mut at).is_err());
        #[cfg(feature = "x-full")]
        assert!(matches!(parse(&wide, &mut at), Ok(VsfType::x(s)) if s == "\u{10FFFF}"));
    }
}

/// A stream cut short, or a count larger than its bits, is refused by every variant as by std.
#[test]
fn truncated_streams() {
    let decoders: &[fn(&[u8], usize) -> Result<(String, usize), &'static str>] = &[
        #[cfg(feature = "x-compact")]
        vsf::x_text::compact::decode_text_with_size,
        #[cfg(feature = "x-latin")]
        vsf::x_text::latin::decode_text_with_size,
        #[cfg(feature = "x-lang16")]
        vsf::x_text::lang16::decode_text_with_size,
        #[cfg(feature = "x-full")]
        vsf::x_text::full::decode_text_with_size,
    ];
    let (bytes, count) = vsf::x_text::encode("Hello, world!").unwrap();
    // A 24-bit code (U+10FFFF's) cut to its first 16 bits.
    let (_, bits, len) = entries().last().unwrap();
    assert_eq!(len, 24);
    let cut = [(bits >> 16) as u8, (bits >> 8) as u8];
    for decode in decoders {
        assert_eq!(decode(&bytes, count).unwrap().0, "Hello, world!");
        assert!(decode(&bytes[..1], count).is_err());
        assert!(decode(&bytes, bytes.len() * 8 + 1).is_err());
        assert!(decode(&[], 1).is_err());
        assert_eq!(decode(&[], 0).unwrap(), (String::new(), 0));
        assert!(decode(&cut, 1).is_err());
    }
}
