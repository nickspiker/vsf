use vsf::{VsfBuilder, VsfType};

fn main() {
    println!("=== VSF Creation Timestamp Demo ===\n");

    // Build a simple VSF file
    let result = VsfBuilder::new()
        .add_section(
            "metadata",
            vec![
                ("width".to_string(), VsfType::u(1920, false)),
                ("height".to_string(), VsfType::u(1080, false)),
                ("format".to_string(), VsfType::x("RAW".to_string())),
            ],
        )
        .build();

    match result {
        Ok(bytes) => {
            println!("✓ Successfully built VSF file: {} bytes\n", bytes.len());

            // Write to file
            std::fs::write("/tmp/demo.vsf", &bytes).expect("Failed to write file");
            println!("✓ Written to /tmp/demo.vsf");
            println!("\nRun: cargo run --bin vsfinfo /tmp/demo.vsf");
            println!("\nYou should see the creation timestamp in the header!");
        }
        Err(e) => {
            eprintln!("✗ Build error: {}", e);
            std::process::exit(1);
        }
    }
}
