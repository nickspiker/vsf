//! Build script to generate Huffman encoding table from frequency data

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{Read, Write};

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
    println!("cargo:rerun-if-changed=frequencies.bin");

    // 1. Load frequencies
    let mut file = File::open("frequencies.bin").expect("Need frequencies.bin");
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

    // 2. Build Huffman tree
    let tree = build_huffman_tree(&frequencies);

    // 3. Generate codes
    let codes = generate_codes(&tree);

    println!("cargo:warning=Generated {} Huffman codes", codes.len());

    // 4. Write huffman_codes.bin
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
}
