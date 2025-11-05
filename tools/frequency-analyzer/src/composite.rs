//! Build composite Unicode frequency table from multiple sources

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

/// Build a comprehensive Unicode frequency table
pub fn build_composite_frequencies(
    word_freq_path: &str,
    output_path: &str,
) -> std::io::Result<()> {
    println!("Building composite Unicode frequency table...");

    // 1. Load word-derived letter frequencies
    let letter_freqs = load_frequency_file(word_freq_path)?;

    let mut freqs: HashMap<u32, f32> = HashMap::new();

    // 2. Add letter frequencies (scaled down to leave room for space/punct)
    let letter_scale = 0.70; // Letters are 70% of text
    for (cp, freq) in letter_freqs {
        freqs.insert(cp, freq * letter_scale);
    }

    // 3. Add space (most common single character)
    freqs.insert(' ' as u32, 0.15); // 15% space

    // 4. Add punctuation (based on typical English text)
    freqs.insert('.' as u32, 0.01);
    freqs.insert(',' as u32, 0.008);
    freqs.insert('\n' as u32, 0.005);
    freqs.insert('!' as u32, 0.002);
    freqs.insert('?' as u32, 0.001);
    freqs.insert(';' as u32, 0.0005);
    freqs.insert(':' as u32, 0.0005);
    freqs.insert('-' as u32, 0.001);
    freqs.insert('\'' as u32, 0.002);
    freqs.insert('"' as u32, 0.002);
    freqs.insert('(' as u32, 0.0005);
    freqs.insert(')' as u32, 0.0005);

    // 5. Add digits (0-9)
    for digit in '0'..='9' {
        freqs.insert(digit as u32, 0.003);
    }

    // 6. Add uppercase (much less common)
    for letter in 'A'..='Z' {
        let lower = (letter as u8 + 32) as char;
        if let Some(&lower_freq) = freqs.get(&(lower as u32)) {
            freqs.insert(letter as u32, lower_freq * 0.05); // Uppercase is ~5% of lowercase
        }
    }

    // 7. Add common Latin extended (accented characters)
    let accented = [
        ('é', 0.0005), ('á', 0.0003), ('í', 0.0002), ('ó', 0.0002),
        ('ú', 0.0001), ('ñ', 0.0002), ('ü', 0.0001), ('ö', 0.0001),
        ('ä', 0.0001), ('è', 0.0002), ('à', 0.0001), ('ç', 0.0001),
    ];
    for (ch, freq) in &accented {
        freqs.insert(*ch as u32, *freq);
    }

    // 8. Add top emoji (common in modern text)
    let emoji = [
        ('😀', 0.00005), ('😂', 0.00005), ('❤', 0.00003), ('🎉', 0.00002),
        ('👍', 0.00002), ('🔥', 0.00002), ('💯', 0.00001), ('✨', 0.00001),
    ];
    for (ch, freq) in &emoji {
        freqs.insert(*ch as u32, *freq);
    }

    // 9. Add common CJK characters (top 100 most frequent)
    // Based on CJK frequency analysis
    let cjk_samples = [
        ('的', 0.0001), ('一', 0.00008), ('是', 0.00007), ('不', 0.00006),
        ('了', 0.00005), ('人', 0.00005), ('我', 0.00004), ('在', 0.00004),
        ('有', 0.00003), ('他', 0.00003), ('这', 0.00003), ('为', 0.00003),
        // Add more as needed...
    ];
    for (ch, freq) in &cjk_samples {
        freqs.insert(*ch as u32, *freq);
    }

    // Normalize to sum to 1.0
    let total: f32 = freqs.values().sum();
    for freq in freqs.values_mut() {
        *freq /= total;
    }

    println!("Total unique codepoints: {}", freqs.len());

    // Sort by frequency
    let mut sorted: Vec<(u32, f32)> = freqs.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Write output
    write_frequency_file(&sorted, output_path)?;

    // Show top 50
    println!("\nTop 50 most frequent:");
    for (i, (cp, freq)) in sorted.iter().take(50).enumerate() {
        let ch = char::from_u32(*cp).unwrap_or('?');
        println!("{:2}. {:?} (U+{:04X}): {:.4}%", i+1, ch, cp, freq * 100.0);
    }

    Ok(())
}

fn load_frequency_file(path: &str) -> std::io::Result<HashMap<u32, f32>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Skip header (12 bytes)
    if &bytes[0..4] != b"FREQ" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid frequency file format",
        ));
    }

    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut freqs = HashMap::new();

    for i in 0..count {
        let offset = 12 + i * 8;
        let codepoint = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let frequency = f32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        freqs.insert(codepoint, frequency);
    }

    Ok(freqs)
}

fn write_frequency_file(freqs: &[(u32, f32)], path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    // Header
    file.write_all(b"FREQ")?;
    file.write_all(&1u32.to_le_bytes())?; // Version
    file.write_all(&(freqs.len() as u32).to_le_bytes())?;

    // Entries
    for (codepoint, frequency) in freqs {
        file.write_all(&codepoint.to_le_bytes())?;
        file.write_all(&frequency.to_le_bytes())?;
    }

    println!("Wrote {} frequencies to {}", freqs.len(), path);
    Ok(())
}
