//! Convert word frequency data to character frequency data

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub fn process_word_frequencies(
    csv_path: &str,
    output_path: &str,
    word_col: usize,
    freq_col: usize,
    has_header: bool,
) -> std::io::Result<()> {
    let file = File::open(csv_path)?;
    let reader = BufReader::new(file);

    let mut char_counts: HashMap<u32, u64> = HashMap::new();
    let mut total_chars: u64 = 0;
    let mut line_num = 0;

    for line in reader.lines() {
        let line = line?;
        line_num += 1;

        // Skip header
        if has_header && line_num == 1 {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() <= freq_col.max(word_col) {
            continue;
        }

        let word = parts[word_col].trim().trim_matches('"');
        let freq_str = parts[freq_col].trim().trim_matches('"');

        let frequency: u64 = match freq_str.parse() {
            Ok(f) => f,
            Err(_) => continue,
        };

        // Count each character in the word, weighted by word frequency
        for c in word.chars() {
            let codepoint = c as u32;
            *char_counts.entry(codepoint).or_insert(0) += frequency;
            total_chars += frequency;
        }
    }

    println!("Processed {} lines", line_num);
    println!("Total character occurrences: {}", total_chars);
    println!("Unique codepoints: {}", char_counts.len());

    // Convert to frequencies
    let mut frequencies: Vec<(u32, f32)> = char_counts.iter()
        .map(|(cp, count)| (*cp, *count as f32 / total_chars as f32))
        .collect();

    // Sort by frequency (descending)
    frequencies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Write output
    let mut output = File::create(output_path)?;

    // Header
    output.write_all(b"FREQ")?;
    output.write_all(&1u32.to_le_bytes())?;
    output.write_all(&(frequencies.len() as u32).to_le_bytes())?;

    // Entries
    for (codepoint, frequency) in &frequencies {
        output.write_all(&codepoint.to_le_bytes())?;
        output.write_all(&frequency.to_le_bytes())?;
    }

    println!("Wrote {} character frequencies to {}", frequencies.len(), output_path);

    // Show top 30
    println!("\nTop 30 most frequent characters:");
    for (i, (cp, freq)) in frequencies.iter().take(30).enumerate() {
        let ch = char::from_u32(*cp).unwrap_or('?');
        println!("{:2}. {:?} (U+{:04X}): {:.4}%", i+1, ch, cp, freq * 100.0);
    }

    Ok(())
}
