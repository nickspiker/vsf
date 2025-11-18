//! Build script to generate Huffman encoding table from frequency data

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Download a file from a URL if it doesn't exist locally
fn download_if_missing(url: &str, path: &str) -> std::io::Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }

    eprintln!("Downloading {} from GitHub...", path);

    let response = ureq::get(url)
        .call()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut file = File::create(path)?;
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file)?;

    eprintln!("Downloaded {} successfully", path);
    Ok(())
}

#[derive(Debug)]
struct Node {
    freq: f32,
    value: Option<char>,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn leaf(ch: char, freq: f32) -> Self {
        Node {
            freq,
            value: Some(ch),
            left: None,
            right: None,
        }
    }

    fn internal(left: Node, right: Node) -> Self {
        Node {
            freq: left.freq + right.freq,
            value: None,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

// Reverse ordering for min-heap
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .freq
            .partial_cmp(&self.freq)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}

fn build_huffman_tree(frequencies: &[(u32, f32)]) -> Node {
    let mut heap = BinaryHeap::new();

    // Create leaf nodes
    for (codepoint, freq) in frequencies {
        if let Some(ch) = char::from_u32(*codepoint) {
            heap.push(Node::leaf(ch, *freq));
        }
    }

    // Build tree bottom-up
    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        heap.push(Node::internal(left, right));
    }

    heap.pop().unwrap()
}

fn generate_codes(tree: &Node) -> HashMap<char, (u32, u8)> {
    let mut codes = HashMap::new();
    let mut path: Vec<bool> = Vec::new();

    fn traverse(node: &Node, path: &mut Vec<bool>, codes: &mut HashMap<char, (u32, u8)>) {
        if let Some(ch) = node.value {
            // Leaf node - convert path to (bits, length) MSB-first
            let mut bits = 0u32;
            let len = path.len();
            for (i, &bit) in path.iter().enumerate() {
                if bit {
                    bits |= 1 << (len - 1 - i); // MSB-first
                }
            }
            codes.insert(ch, (bits, path.len() as u8));
        } else {
            // Internal node - traverse both sides
            if let Some(ref left) = node.left {
                path.push(false);
                traverse(left, path, codes);
                path.pop();
            }

            if let Some(ref right) = node.right {
                path.push(true);
                traverse(right, path, codes);
                path.pop();
            }
        }
    }

    traverse(tree, &mut path, &mut codes);
    codes
}

fn main() {
    // Tell Cargo about the huffman_available cfg so it doesn't warn
    println!("cargo::rustc-check-cfg=cfg(huffman_available)");

    println!("cargo:rerun-if-changed=frequencies.bin");

    // Skip if output is already newer than input
    if let (Ok(out_meta), Ok(in_meta)) = (
        std::fs::metadata("huffman_codes.bin"),
        std::fs::metadata("frequencies.bin"),
    ) {
        if let (Ok(out_time), Ok(in_time)) = (out_meta.modified(), in_meta.modified()) {
            if out_time > in_time {
                // Output is newer, skip regeneration but still emit cfg flag
                println!("cargo:rustc-cfg=huffman_available");
                return;
            }
        }
    }

    // 0. Download frequencies.bin from GitHub if not present
    const FREQUENCIES_URL: &str = "https://github.com/nickspiker/vsf/raw/main/frequencies.bin";

    // Try to download, but continue without if we're in a restricted environment (e.g., docs.rs)
    let download_result = download_if_missing(FREQUENCIES_URL, "frequencies.bin");

    // Load frequencies - if download failed and file doesn't exist, skip Huffman table generation
    let mut file = match File::open("frequencies.bin") {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Warning: frequencies.bin not available, skipping Huffman table generation");
            eprintln!("Huffman text compression will not be available in this build.");
            if download_result.is_err() {
                eprintln!("Download failed (likely building in restricted environment like docs.rs)");
            }
            return; // Skip Huffman generation, library will still compile
        }
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    // Parse frequency file
    if &bytes[0..4] != b"FREQ" {
        panic!("Invalid frequency file format");
    }

    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut frequencies = Vec::new();

    for i in 0..count {
        let offset = 12 + i * 8;
        let codepoint = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let frequency = f32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        frequencies.push((codepoint, frequency));
    }

    println!("cargo:warning=Loaded {} character frequencies", count);

    // 1. Build Huffman tree
    let tree = build_huffman_tree(&frequencies);

    // 2. Generate codes
    let codes = generate_codes(&tree);

    println!("cargo:warning=Generated {} Huffman codes", codes.len());

    // 3. Write huffman_codes.bin
    let mut output = File::create("huffman_codes.bin").unwrap();

    // Header
    output.write_all(b"HUFF").unwrap(); // Magic
    output.write_all(&1u32.to_le_bytes()).unwrap(); // Version
    output
        .write_all(&(codes.len() as u32).to_le_bytes())
        .unwrap();

    // Entries (sorted by codepoint)
    let mut sorted_codes: Vec<_> = codes.iter().collect();
    sorted_codes.sort_by_key(|(ch, _)| **ch as u32);

    for (ch, (bits, length)) in sorted_codes {
        let codepoint = *ch as u32;
        let packed = bits | ((*length as u32) << 24);

        output.write_all(&codepoint.to_le_bytes()).unwrap();
        output.write_all(&packed.to_le_bytes()).unwrap();
    }

    println!(
        "cargo:warning=Generated huffman_codes.bin: {} entries",
        codes.len()
    );

    // Emit cfg flag so text_encoding.rs knows the file is available
    println!("cargo:rustc-cfg=huffman_available");
}
