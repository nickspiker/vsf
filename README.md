# VSF (Versatile Storage Format)

VSF is an open-standard for data storage and representation. Designed for efficiency, security, and adaptability, VSF aims to provide a complete and unified solution for storing and managing any type of data, from simple values to complex structures like images or 3D objects.

## Key Features

- Optimized for efficiency and compact size
- Built-in security and integrity measures
- Transparent data exchange
- Unified metadata framework
- Spectral accuracy in colour and data representation
- Proof of authenticity and chain of trust
- Future-proof design for technological advances

## Supported Data Types

VSF supports a wide range of basic constructor data types, including unsigned and signed integers, floating-point numbers, complex numbers, boolean values, Unicode text, and arrays. It also includes VSF-specific types for metadata like labels, offsets, and versions.

## Usage

Here's a simple example of creating a minimal VSF structure:

```rust
use vsf::{VsfType, parse, EncodeNumber};

fn create_minimal_vsf() -> Result<Vec<u8>, std::io::Error> {
    let mut vsf = vec!["RÅ".as_bytes().to_owned()];

    // Header
    let mut header_index = 0;
    vsf[header_index].append(&mut b"<".to_vec());
    let header_length_index = vsf.len();
    let mut header_length = 42; // Placeholder
    vsf.push(VsfType::b(header_length).flatten()?);
    header_index = vsf.len();
    vsf.push(VsfType::z(1).flatten()?); // Version
    vsf[header_index].append(&mut VsfType::y(1).flatten()?); // Backward version
    vsf[header_index].append(&mut VsfType::c(1).flatten()?); // Label definition count
    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("example data".to_string()).flatten()?);
    let label_offset_index = vsf.len();
    let mut label_offset = 42; // Placeholder
    vsf.push(VsfType::o(label_offset).flatten()?);
    let label_size_index = vsf.len();
    let mut label_size = 42; // Placeholder
    vsf.push(VsfType::b(label_size).flatten()?);
    header_index = vsf.len();
    vsf.push(VsfType::c(2).flatten()?); // Number of elements in example data
    vsf[header_index].append(&mut b")".to_vec());
    vsf[header_index].append(&mut b">".to_vec());
    let header_end_index = vsf.len();

    // Label set
    header_index = vsf.len();
    vsf.push(b"[".to_vec());
    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("example value one".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::f5(3.14159).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"(".to_vec());
    vsf[header_index].append(&mut VsfType::d("example value two".to_string()).flatten()?);
    vsf[header_index].append(&mut b":".to_vec());
    vsf[header_index].append(&mut VsfType::s7(i128::MAX).flatten()?);
    vsf[header_index].append(&mut b")".to_vec());

    vsf[header_index].append(&mut b"]".to_vec());

    let mut prev_header_length = 0;
    let mut prev_label_offset = 0;
    let mut prev_label_size = 0;

    while header_length != prev_header_length
        || label_offset != prev_label_offset
        || label_size != prev_label_size
    {
        prev_header_length = header_length;
        prev_label_offset = label_offset;
        prev_label_size = label_size;

        header_length = 0;
        for i in 0..header_end_index {
            header_length += vsf[i].len();
        }
        vsf[header_length_index] = VsfType::b(header_length * 8).flatten()?;

        label_offset = header_length;
        vsf[label_offset_index] = VsfType::o(label_offset * 8).flatten()?;

        label_size = 0;
        for i in header_end_index..vsf.len() {
            label_size += vsf[i].len();
        }
        vsf[label_size_index] = VsfType::b(label_size * 8).flatten()?;
    }

    let vsf_vector: Vec<u8> = vsf.into_iter().flatten().collect();
    Ok(vsf_vector)
}
```

## Future Capabilities

We're actively developing VSF to handle more complex data structures. In future releases, you'll be able to:

- Save and load images with all necessary metadata
- Handle advanced data types and structures
- Implement more sophisticated constructors and destructors

Stay tuned for updates!

## License

VSF is released under a custom open-source license. You're free to use, modify, and distribute VSF for any purpose, including commercial use. However, selling VSF itself or directly derived formats is not permitted. For full terms, see the LICENSE file in the repository.

## Contributing

We welcome contributions! Please contact nick@verichrome.cc if you'd like to contribute to this project.

For more information about VSF or the TOKEN system, visit [https://sunsarrow.com/vsf](https://sunsarrow.com/vsf).