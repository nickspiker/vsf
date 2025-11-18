use chrono::Utc;
use vsf::types::{EtType, VsfType};

fn main() {
    // Simulate what vsf_builder does
    let mut vsf: Vec<Vec<u8>> = Vec::new();

    // Magic + header start
    vsf.push("RÅ".as_bytes().to_vec());
    vsf[0].push(b'<');

    // Header length placeholder
    vsf.push(VsfType::b(0, true).flatten());

    // Version and backward compat (in same vec)
    let header_index = vsf.len();
    vsf.push(VsfType::z(2).flatten());
    vsf[header_index].extend_from_slice(&VsfType::y(2).flatten());

    // Creation time (ef5)
    let now = Utc::now();
    let et = vsf::datetime_to_eagle_time(now);
    let et_f32 = match et.et_type() {
        EtType::f6(v) => *v as f32,
        EtType::f5(v) => *v,
        EtType::u(v) => *v as f32,
        EtType::i(v) => *v as f32,
    };
    let creation_time = VsfType::e(EtType::f5(et_f32));
    vsf[header_index].extend_from_slice(&creation_time.flatten());

    // Hash placeholder
    vsf[header_index].extend_from_slice(&VsfType::hb(vec![0u8; 32]).flatten());

    // Flatten
    let bytes: Vec<u8> = vsf.into_iter().flatten().collect();

    println!("Total bytes: {}", bytes.len());
    println!("\nFirst 60 bytes with positions:");
    for (i, &b) in bytes.iter().take(60).enumerate() {
        if i % 16 == 0 {
            println!();
            print!("{:04X}: ", i);
        }
        print!("{:02X} ", b);
    }
    println!("\n");

    // Find where 'h' appears
    for (i, &b) in bytes.iter().enumerate().take(40) {
        if b == b'h' {
            println!("Found 'h' at position {}", i);
        }
    }

    // Parse like verification does
    println!("\nParsing simulation:");
    let mut pointer = 0;

    // Skip magic
    pointer += 3; // "RÅ"
    pointer += 1; // '<'
    println!("After magic: pointer = {}", pointer);

    // Parse header length
    let hl_start = pointer;
    if bytes[pointer] == b'b' {
        pointer += 1;
        let len_marker = bytes[pointer];
        pointer += 1;
        println!(
            "Header length type at {}, marker={}, pointer now={}",
            hl_start, len_marker as char, pointer
        );
    }

    // Parse version (z)
    let v_start = pointer;
    if bytes[pointer] == b'z' {
        pointer += 1;
        let len_marker = bytes[pointer];
        pointer += 1;
        println!(
            "Version at {}, marker={}, pointer now={}",
            v_start, len_marker as char, pointer
        );
    }

    // Parse backward compat (y)
    let bc_start = pointer;
    if bytes[pointer] == b'y' {
        pointer += 1;
        let len_marker = bytes[pointer];
        pointer += 1;
        println!(
            "Backward compat at {}, marker={}, pointer now={}",
            bc_start, len_marker as char, pointer
        );
    }

    // Parse creation time (e)
    let et_start = pointer;
    if bytes[pointer] == b'e' {
        pointer += 1;
        if bytes[pointer] == b'f' {
            pointer += 1;
            if bytes[pointer] == b'5' {
                pointer += 1;
                pointer += 4; // f32
                println!("Eagle Time (ef5) at {}, pointer now={}", et_start, pointer);
            }
        }
    }

    // Now we should be at the hash
    println!(
        "\nAt position {}: byte = 0x{:02X} ('{}')",
        pointer, bytes[pointer], bytes[pointer] as char
    );
    if bytes[pointer] == b'h' {
        println!("SUCCESS: Found hash placeholder at correct position!");
    } else {
        println!("ERROR: Expected 'h', found something else");
    }
}
