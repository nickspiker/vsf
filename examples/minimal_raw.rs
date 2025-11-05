use std::fs;
use vsf::builders::*;
use vsf::types::BitPackedTensor;

fn main() -> Result<(), String> {
    println!("Creating minimal barebones RAW image for hex inspection...\n");

    // Tiny 4x4 8-bit image with simple pattern
    // Create simple gradient: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, A, B, C, D, E, F
    let samples: Vec<u64> = (0..16).collect();

    println!("Image data:");
    println!("  Size: 4x4 @ 8 bits/pixel");
    println!("  Samples: {:02X?}", samples);
    println!();

    // Create BitPackedTensor
    let image = BitPackedTensor::pack(8, vec![4, 4], &samples);

    // Create minimal RAW (no metadata, just the image)
    let raw_bytes = build_raw_image(
        image, None, // No sensor metadata
        None, // No camera settings
        None, // No lens info
    )?;

    println!("VSF file size: {} bytes", raw_bytes.len());
    println!();

    // Write to file
    fs::write("minimal_raw.vsf", &raw_bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    println!("✓ Written to minimal_raw.vsf");
    println!();

    // Show hex dump of first 128 bytes
    println!("Hex dump (first 128 bytes):");
    for (i, chunk) in raw_bytes.chunks(16).take(8).enumerate() {
        print!("{:04x}  ", i * 16);
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02x} ", byte);
            if j == 7 {
                print!(" ");
            }
        }
        print!(" ");
        for byte in chunk {
            if *byte >= 0x20 && *byte <= 0x7E {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!();
    }

    println!();
    println!("Full file size: {} bytes", raw_bytes.len());
    println!();
    println!("Structure:");
    println!("  RÅ<           - Magic number + header start");
    println!("  b[length]     - Header length in BITS");
    println!("  z2            - Version 2");
    println!("  y2            - Backward compat 2");
    println!("  n1            - 1 label (just 'raw')");
    println!("  (d...)        - Label definition (name, offset, size, child count)");
    println!("  >             - Header end marker");
    println!("  [...]         - 'raw' section with one field:");
    println!("    (d5image:p[bitdepth, shape, pixels]) - Fully boxed p type");
    println!();
    println!("KEY INSIGHT: p type is SELF-DESCRIBING!");
    println!("  - bit_depth is IN the p encoding");
    println!("  - shape [4, 4] is IN the p encoding");
    println!("  - pixels are IN the p encoding");
    println!("  NO redundant width/height/bits_per_pixel fields needed!");

    Ok(())
}
