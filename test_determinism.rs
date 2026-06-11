use vsf::types::VsfType;

fn main() {
    // Test 1: NFC vs NFD
    let nfc = "café";  // Precomposed U+00E9
    let nfd = "cafe\u{0301}";  // e + combining accent
    
    println!("=== Test 1: NFC vs NFD ===");
    println!("NFC: {:?}", nfc);
    println!("NFD: {:?}", nfd);
    println!("Equal as strings? {}", nfc == nfd);
    
    let nfc_bytes = VsfType::x(nfc.to_string()).flatten();
    let nfd_bytes = VsfType::x(nfd.to_string()).flatten();
    
    println!("NFC encoded: {:02x?}", nfc_bytes);
    println!("NFD encoded: {:02x?}", nfd_bytes);
    println!("Equal as bytes? {}", nfc_bytes == nfd_bytes);
    
    // Test 2: ZWJ Sequence (👨‍👩‍👧 = man + ZWJ + woman + ZWJ + girl)
    let zwj_seq = "👨‍👩‍👧";  // 3 emoji + 2 ZWJs
    println!("\n=== Test 2: ZWJ Emoji ===");
    println!("ZWJ string: {:?} (char count: {})", zwj_seq, zwj_seq.chars().count());
    for (i, c) in zwj_seq.chars().enumerate() {
        println!("  Codepoint {}: U+{:04X} ({:?})", i, c as u32, c);
    }
    
    // Test 3: Variation selectors (FE0E = text, FE0F = emoji)
    let text_presentation = "❤\u{FE0E}";  // Red heart + text variation selector
    let emoji_presentation = "❤\u{FE0F}";  // Red heart + emoji variation selector
    println!("\n=== Test 3: Variation Selectors ===");
    println!("Text: {:?} (chars: {})", text_presentation, text_presentation.chars().count());
    println!("Emoji: {:?} (chars: {})", emoji_presentation, emoji_presentation.chars().count());
    
    let text_bytes = VsfType::x(text_presentation.to_string()).flatten();
    let emoji_bytes = VsfType::x(emoji_presentation.to_string()).flatten();
    println!("Text encoded:  {:02x?}", text_bytes);
    println!("Emoji encoded: {:02x?}", emoji_bytes);
    println!("Equal? {}", text_bytes == emoji_bytes);
    
    // Test 4: Skin tone modifier
    let without_tone = "👋";  // Wave hand
    let with_tone = "👋\u{1F3FD}";  // Wave hand + medium skin tone
    println!("\n=== Test 4: Skin Tone Modifier ===");
    println!("Without tone: {:?} (chars: {})", without_tone, without_tone.chars().count());
    println!("With tone: {:?} (chars: {})", with_tone, with_tone.chars().count());
    
    // Test 5: Confusable characters
    let cyrillic_a = "\u{0430}";  // Cyrillic SMALL LETTER A
    let latin_a = "a";  // Latin SMALL LETTER A
    println!("\n=== Test 5: Confusables (Cyrillic vs Latin) ===");
    println!("Cyrillic: {:?} U+{:04X}", cyrillic_a, cyrillic_a.chars().next().unwrap() as u32);
    println!("Latin: {:?} U+{:04X}", latin_a, latin_a.chars().next().unwrap() as u32);
    println!("Equal? {}", cyrillic_a == latin_a);
    
    let cyrillic_bytes = VsfType::x(cyrillic_a.to_string()).flatten();
    let latin_bytes = VsfType::x(latin_a.to_string()).flatten();
    println!("Cyrillic encoded: {:02x?}", cyrillic_bytes);
    println!("Latin encoded:    {:02x?}", latin_bytes);
    println!("Equal as bytes? {}", cyrillic_bytes == latin_bytes);
}
