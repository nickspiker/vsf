use colored::*;
use vsf::*;

fn main() {
   let test_image = build_test_image();
   for &byte in &test_image {
       if byte.is_ascii_graphic() || byte == b' ' { // Checks if byte is printable, including space
           print!("{}", (byte as char).to_string().green());
       } else {
           print!("{}", format!("[{}]", byte).red());
       }
   }
   println!(); // Just to add a newline at the end
}

fn build_test_image() -> Vec<u8> {
    //Example VSF header and parent label set. Note to maintain bit alignment, all values are required to be at intervals of and padded to 8 bits for version 1.
    let mut vsf_header_a: Vec<u8> = "RÅ{<l".as_bytes().to_vec(); // RÅ is the file ID or magic number, 'l' marks the length of the header and magic only.  This entire bitstring must be present in a valid VSF as the length of the header must come first after the magic number.
    let mut vsf_header_b: Vec<u8> = "z3".as_bytes().to_vec(); // VSF version marker, 2^n notation (2^3=8 bits)
    vsf_header_b.push(1); // VSF version number
    vsf_header_b.append(&mut "y3".as_bytes().to_vec()); // VSF backward version marker, 2^n notation
    vsf_header_b.push(1); // VSF backward version number
    let type_text = vsf::VsfType::d("Image".to_owned()); // Data type
    vsf_header_b.append(&mut type_text.flatten().unwrap()); // Converts the type to a VSF style byte vector and appends it to the header
    vsf_header_b.append(&mut "c3".as_bytes().to_vec()); // Label count marker, 2^n notation
    vsf_header_b.push(3); // Label count
    vsf_header_b.append(&mut "s5".as_bytes().to_vec()); // File size marker, 2^n notation
    vsf_header_b.extend_from_slice(&(123456 as u32).to_be_bytes()); // File size in bits
    vsf_header_b.push(b'>'); // End of header
    let header_length = vsf_header_a.len() + vsf_header_b.len();
    vsf_header_a.extend_from_slice(&header_length.encode_length(false)); // Encode the length of the header and magic number
    vsf_header_a.append(&mut vsf_header_b);
    // RÅ{<l3\0FV0\01v0\01t3\05Imagec0\03s5\12\34\56\78>[(t0#13#RGB thumbnailo1#5474#l1#65536#)(tN0#13#RAW CFA frameo1#72360#l1#65536#)(tN0#8#Metadatao1#123456#l1##)]}

    // RÅ is the file ID or magic number
    // l# Length of parent label set including brackets {...}
    // z# VSF version
    // y# VSF backward version
    // t# File type
    // c# Label count
    // s# File size

    // VSF header and parent label set explanation:
    // RÅ{<file header/parent label set stats>[(Child label set name, pointer and size one)(Child label set name, pointer and size two)(Child label set name, pointer and size three)]}

    // Child label set:
    // {<child label set stats>[(Child label 1)(child label 2)]}

    // The parent labels organize and point to multiple child label sets, each containing
    // related information and pointers to specific data. The parent label set also
    // includes details about each child set, such as its size, location in the file, and
    // purpose. This parent-child structure allows readers to access only the data they
    // need, rather than reading the entire file. For example, if you only want to display
    // a small thumbnail icon, you can read just a small portion of the file.
    // Magic must be first and parent label set must immediately follow. Child label
    // sets can be placed after any data so that changes to the labels can generally be
    // made without re-writing the entire file.
    vsf_header_a
}
