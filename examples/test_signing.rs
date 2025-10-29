use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use vsf::types::VsfType;
use vsf::verification::sign_section;
use vsf::vsf_builder::VsfBuilder;

fn main() -> Result<(), String> {
    println!("Testing VSF signing...\n");

    // Create a simple VSF file
    let vsf = VsfBuilder::new()
        .add_section("test", vec![("value".to_string(), VsfType::u(42, false))])
        .build()?;

    println!("Created VSF file: {} bytes", vsf.len());

    // Generate signing key
    let signing_key = SigningKey::generate(&mut OsRng);
    println!("Generated Ed25519 signing key");

    // Sign the "test" section
    let signed = sign_section(vsf, "test", signing_key.as_bytes())?;

    println!("Signed VSF file: {} bytes", signed.len());

    // Save to file for inspection
    std::fs::write("test_signed.vsf", &signed).unwrap();
    println!("Saved to test_signed.vsf");

    println!("\nSigning successful!");

    Ok(())
}
