use std::fs;
use std::time::Instant;
use vsf::text_encoding::{decode_text, encode_text};

fn main() {
    let input_path = "tools/english_test.txt";

    println!("Reading file: {}", input_path);
    let text = fs::read_to_string(input_path).expect("Failed to read input file");

    let utf8_size = text.as_bytes().len();
    println!(
        "UTF-8 size: {} bytes ({:.2} KB)",
        utf8_size,
        utf8_size as f64 / 1024.0
    );

    // Count ASCII vs Unicode and total chars
    let ascii_count = text.chars().filter(|c| c.is_ascii()).count();
    let unicode_count = text.chars().filter(|c| !c.is_ascii()).count();
    let char_count = text.chars().count();
    println!(
        "Characters: {} total ({} ASCII, {} Unicode)",
        char_count, ascii_count, unicode_count
    );

    // Encode (with warm-up and multiple runs for accuracy)
    println!("\n=== Encoding Performance ===");

    // Warm-up
    let _ = encode_text(&text);

    // Benchmark with multiple runs
    let runs = 100;
    let mut total_encode_time = std::time::Duration::ZERO;
    let mut encoded = Vec::new();

    for _ in 0..runs {
        let start = Instant::now();
        encoded = encode_text(&text);
        total_encode_time += start.elapsed();
    }

    let avg_encode_time = total_encode_time / runs;
    let huffman_size = encoded.len();

    println!(
        "Huffman size: {} bytes ({:.2} KB)",
        huffman_size,
        huffman_size as f64 / 1024.0
    );
    println!("Avg encode time: {:.2?} ({} runs)", avg_encode_time, runs);
    println!(
        "Encoding speed: {:.2} MB/s",
        utf8_size as f64 / avg_encode_time.as_secs_f64() / 1_000_000.0
    );

    // Calculate compression
    let compression_ratio = 100.0 * (1.0 - huffman_size as f64 / utf8_size as f64);
    let space_saved = utf8_size - huffman_size;

    println!("\n=== Compression Results ===");
    println!("Compression ratio: {:.2}%", compression_ratio);
    println!(
        "Space saved: {} bytes ({:.2} KB)",
        space_saved,
        space_saved as f64 / 1024.0
    );
    println!(
        "Compressed to: {:.2}% of original size",
        100.0 - compression_ratio
    );

    // Decode (with warm-up and multiple runs)
    println!("\n=== Decoding Performance ===");

    // Warm-up
    let _ = decode_text(&encoded, char_count).expect("Decode failed");

    // Benchmark with multiple runs
    let mut total_decode_time = std::time::Duration::ZERO;
    let mut decoded = String::new();

    for _ in 0..runs {
        let start = Instant::now();
        decoded = decode_text(&encoded, char_count).expect("Decode failed");
        total_decode_time += start.elapsed();
    }

    let avg_decode_time = total_decode_time / runs;

    println!("Avg decode time: {:.2?} ({} runs)", avg_decode_time, runs);
    println!(
        "Decoding speed: {:.2} MB/s",
        utf8_size as f64 / avg_decode_time.as_secs_f64() / 1_000_000.0
    );

    // Verify round-trip
    if decoded == text {
        println!("\n✓ Round-trip successful!");
    } else {
        println!("\n✗ Round-trip FAILED!");
        std::process::exit(1);
    }

    // Summary
    println!("\n=== Summary ===");
    println!(
        "Input: {} chars ({} bytes)",
        ascii_count + unicode_count,
        utf8_size
    );
    println!("Output: {} bytes", huffman_size);
    println!(
        "Encode: {:.2} MB/s",
        utf8_size as f64 / avg_encode_time.as_secs_f64() / 1_000_000.0
    );
    println!(
        "Decode: {:.2} MB/s",
        utf8_size as f64 / avg_decode_time.as_secs_f64() / 1_000_000.0
    );
    println!("Compression: {:.2}%", compression_ratio);
}
