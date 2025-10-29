// Test to verify offset units
use vsf::*;

fn main() -> Result<(), String> {
    // Create a simple VSF file with two sections
    let section1_data = vec![1u8, 2, 3, 4];  // 4 bytes
    let section2_data = vec![5u8, 6, 7, 8, 9, 10];  // 6 bytes
    
    let builder = VsfBuilder::new()
        .add_unboxed("section1", section1_data)
        .add_unboxed("section2", section2_data);
    
    let vsf_bytes = builder.build()?;
    
    // Parse header
    let header = vsf::file_format::parse_full_header(&vsf_bytes)?;
    
    // Check section1
    let section1_label = &header.labels[0];
    println!("Section1 name: {}", section1_label.name);
    println!("Section1 offset_bits: {}", section1_label.offset_bits);
    println!("Section1 size_bits: {}", section1_label.size_bits);
    println!("Section1 offset_bytes would be: {}", section1_label.offset_bits / 8);
    
    // Check section2
    let section2_label = &header.labels[1];
    println!("\nSection2 name: {}", section2_label.name);
    println!("Section2 offset_bits: {}", section2_label.offset_bits);
    println!("Section2 size_bits: {}", section2_label.size_bits);
    println!("Section2 offset_bytes would be: {}", section2_label.offset_bits / 8);
    
    // What should the offset be?
    // Header is at the start, sections come after
    println!("\nTotal file size: {} bytes", vsf_bytes.len());
    println!("Expected section2 offset: section1_offset + 4 bytes");
    
    // Try to extract section data
    let s1_offset = section1_label.offset_bits / 8;
    let s1_size = section1_label.size_bits / 8;
    let s1_data = &vsf_bytes[s1_offset..s1_offset + s1_size];
    println!("\nSection1 data: {:?}", s1_data);
    
    let s2_offset = section2_label.offset_bits / 8;
    let s2_size = section2_label.size_bits / 8;
    let s2_data = &vsf_bytes[s2_offset..s2_offset + s2_size];
    println!("Section2 data: {:?}", s2_data);
    
    Ok(())
}
