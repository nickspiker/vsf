//! Character frequency analyzer for Huffman encoding
//!
//! Usage: cargo run -- input.txt output.bin
//!
//! Or use stdin:
//!   cat corpus/*.txt | cargo run -- - output.bin

mod word_to_char;
mod composite;
mod global_unicode;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use word_to_char::process_word_frequencies;
use composite::build_composite_frequencies;
use global_unicode::build_global_frequencies;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  {} <input.txt|-for-stdin> <output.bin>", args[0]);
        eprintln!("  {} --from-words <word-freq.csv> <output.bin>", args[0]);
        eprintln!("  {} --composite <word-freq-letters.bin> <output.bin>", args[0]);
        eprintln!("  {} --global <output.bin>  (generates full Unicode table)", args[0]);
        std::process::exit(1);
    }

    if args[1] == "--global" {
        if args.len() < 3 {
            eprintln!("Usage: {} --global <output.bin>", args[0]);
            std::process::exit(1);
        }
        build_global_frequencies(&args[2])?;
        return Ok(());
    }

    if args[1] == "--from-words" {
        if args.len() < 4 {
            eprintln!("Usage: {} --from-words <word-freq.csv> <output.bin>", args[0]);
            std::process::exit(1);
        }
        // CSV format: word,frequency,pronunciation,rank
        // Columns: 0=word, 1=frequency
        process_word_frequencies(&args[2], &args[3], 0, 1, true)?;
        return Ok(());
    }

    if args[1] == "--composite" {
        if args.len() < 4 {
            eprintln!("Usage: {} --composite <letter-freq.bin> <output.bin>", args[0]);
            std::process::exit(1);
        }
        build_composite_frequencies(&args[2], &args[3])?;
        return Ok(());
    }

    if args.len() < 3 {
        eprintln!("Error: Missing required arguments");
        eprintln!("Usage: {} <input.txt|-for-stdin> <output.bin>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Analyzing character frequencies...");

    let mut counts: HashMap<u32, u64> = HashMap::new();
    let mut total: u64 = 0;

    // Read input
    let content = if input_path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        let file = File::open(input_path)?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        buf
    };

    // Count codepoints
    for c in content.chars() {
        let codepoint = c as u32;
        *counts.entry(codepoint).or_insert(0) += 1;
        total += 1;
    }

    println!("Total characters: {}", total);
    println!("Unique codepoints: {}", counts.len());

    // Convert to frequencies
    let mut frequencies: Vec<(u32, f32)> = counts.iter()
        .map(|(cp, count)| (*cp, *count as f32 / total as f32))
        .collect();

    // Sort by frequency (descending)
    frequencies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Write binary format
    let mut output = File::create(output_path)?;

    // Header
    output.write_all(b"FREQ")?;  // Magic
    output.write_all(&1u32.to_le_bytes())?;  // Version
    output.write_all(&(frequencies.len() as u32).to_le_bytes())?;

    // Entries
    for (codepoint, frequency) in &frequencies {
        output.write_all(&codepoint.to_le_bytes())?;
        output.write_all(&frequency.to_le_bytes())?;
    }

    println!("Wrote {} frequency entries to {}", frequencies.len(), output_path);

    // Show top 20
    println!("\nTop 20 most frequent:");
    for (i, (cp, freq)) in frequencies.iter().take(20).enumerate() {
        let ch = char::from_u32(*cp).unwrap_or('?');
        println!("{:2}. {:?} (U+{:04X}): {:.4}%", i+1, ch, cp, freq * 100.0);
    }

    Ok(())
}
