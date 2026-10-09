//! `Document` opens by reading the header alone, locates sections by their table-of-contents entries, checks content hashes, and reads a plane's rows without unpacking — over a slice and over a file.

use std::cell::Cell;
use vsf::container::{ByteSource, Document, SourceError, ValueKind};
use vsf::types::{BitPackedTensor, Tensor, VsfType};
use vsf::{SectionMeta, VsfBuilder, VsfSection};

/// A source that counts the bytes read, so laziness is an assertion rather than a hope.
struct Counting<'a> {
    bytes: &'a [u8],
    read: Cell<u64>,
}

impl ByteSource for Counting<'_> {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        self.read.set(self.read.get() + buf.len() as u64);
        self.bytes.read_at(offset, buf)
    }
}

fn plane_samples() -> Vec<u16> {
    (0..64 * 48).map(|i| ((i * 7919) % 4096) as u16).collect()
}

/// A document with a bit-packed plane, a text field, a second section, and an unboxed blob — content hashes on.
fn build() -> Vec<u8> {
    let plane = BitPackedTensor::pack_u16(12, vec![48, 64], &plane_samples());
    VsfBuilder::new()
        .content_hashes(true)
        .add_section(
            "plane",
            vec![
                ("width".to_string(), VsfType::u(64, false)),
                ("height".to_string(), VsfType::u(48, false)),
                ("note".to_string(), VsfType::a("sensor".to_string())),
                ("samples".to_string(), VsfType::p(plane)),
                ("wrapped".to_string(), VsfType::v(b'b', vec![9u8; 100])),
            ],
        )
        .add_section("meta", vec![("tool".to_string(), VsfType::a("test".to_string())), ("curve".to_string(), VsfType::t_f5(Tensor::new(vec![3], vec![1.0f32, 2.0, 3.0])))])
        .add_unboxed("blob", vec![0xA5u8; 1000])
        .build()
        .unwrap()
}

#[test]
fn open_reads_only_the_header() {
    let bytes = build();
    let src = Counting { bytes: &bytes, read: Cell::new(0) };
    let doc = Document::open(&src).unwrap();
    assert!(src.read.get() < 512, "open read {} bytes of a {}-byte document", src.read.get(), bytes.len());
    assert_eq!(doc.entries().len(), 3);
    assert_eq!(doc.entries().iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["plane", "meta", "blob"]);
    // Reading one section's bytes reads that section, not the file.
    let before = src.read.get();
    let meta = doc.section(doc.entry("meta").unwrap()).unwrap();
    assert_eq!(meta.text("tool"), Some("test"));
    assert!(src.read.get() - before < 200);
}

#[test]
fn content_hashes_are_emitted_checked_and_tamper_evident() {
    let mut bytes = build();
    {
        let doc = Document::open(&bytes[..]).unwrap();
        for e in doc.entries() {
            assert!(matches!(e.hash, Some(VsfType::hb(_))), "entry '{}' carries no content hash", e.name);
            assert_eq!(doc.content_hash_ok(e).unwrap(), Some(true), "entry '{}'", e.name);
        }
        // Without content hashes the entry carries none and the check says so.
        let plain = VsfBuilder::new().add_section("x", vec![("a".to_string(), VsfType::u(1, false))]).build().unwrap();
        let d = Document::open(&plain[..]).unwrap();
        assert_eq!(d.content_hash_ok(d.entry("x").unwrap()).unwrap(), None);
        // Per-section opt-in through SectionMeta.
        let one = VsfBuilder::new()
            .add_section_with_meta("y", vec![("a".to_string(), VsfType::u(1, false))], SectionMeta { key: None, content_hash: true })
            .add_section("z", vec![("a".to_string(), VsfType::u(1, false))])
            .build()
            .unwrap();
        let d = Document::open(&one[..]).unwrap();
        assert_eq!(d.content_hash_ok(d.entry("y").unwrap()).unwrap(), Some(true));
        assert_eq!(d.content_hash_ok(d.entry("z").unwrap()).unwrap(), None);
    }
    // Flip one byte inside the plane's samples: only that entry fails.
    let doc = Document::open(&bytes[..]).unwrap();
    let span = doc.packed(doc.entry("plane").unwrap(), "samples").unwrap().unwrap();
    let victim = span.data_offset as usize + 100;
    bytes[victim] ^= 0x10;
    let doc = Document::open(&bytes[..]).unwrap();
    assert_eq!(doc.content_hash_ok(doc.entry("plane").unwrap()).unwrap(), Some(false));
    assert_eq!(doc.content_hash_ok(doc.entry("meta").unwrap()).unwrap(), Some(true));
    assert_eq!(doc.content_hash_ok(doc.entry("blob").unwrap()).unwrap(), Some(true));
}

#[test]
fn planes_are_located_and_read_by_rows_without_a_full_unpack() {
    let bytes = build();
    let doc = Document::open(&bytes[..]).unwrap();
    let plane = doc.entry("plane").unwrap();
    let fields = doc.fields(plane).unwrap();
    assert_eq!(fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["width", "height", "note", "samples", "wrapped"]);
    assert!(matches!(fields[2].values[0].kind, ValueKind::Other(VsfType::a(ref s)) if s == "sensor"));
    assert!(matches!(fields[3].values[0].kind, ValueKind::Packed { bit_depth: 12, .. }));
    assert!(matches!(fields[4].values[0].kind, ValueKind::Wrapped { code: b'b', data_len: 100, .. }));
    // Every located span lies inside the entry's own bytes.
    let (off, len) = doc.span(plane);
    for f in &fields {
        for v in &f.values {
            assert!(v.start as u64 >= off && v.end as u64 <= off + len, "field {} value outside the entry", f.name);
        }
    }
    let span = doc.packed(plane, "samples").unwrap().unwrap();
    assert_eq!(span.shape, vec![48, 64]);
    assert_eq!(span.data_len, (48 * 64 * 12 + 7) / 8);
    let full = plane_samples();
    assert_eq!(doc.packed_rows_u16(&span, 0, 48).unwrap(), full);
    assert_eq!(doc.packed_rows_u16(&span, 10, 13).unwrap(), full[10 * 64..13 * 64]);
    assert_eq!(doc.packed_range_u16(&span, 1000, 77).unwrap(), full[1000..1077]);
    assert!(doc.packed_range_u16(&span, 48 * 64, 1).is_err());
    assert_eq!(doc.packed(plane, "note").unwrap(), None);
    assert_eq!(doc.wrapped(plane, "wrapped").unwrap().unwrap().data_len, 100);
    // The unboxed blob is raw bytes.
    assert_eq!(doc.raw(doc.entry("blob").unwrap()).unwrap(), vec![0xA5u8; 1000]);
}

#[test]
fn a_file_source_reads_the_same_rows_through_a_window() {
    let bytes = build();
    let path = std::env::temp_dir().join(format!("vsf-container-lazy-{}.vsf", std::process::id()));
    std::fs::write(&path, &bytes).unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let doc = Document::open(file).unwrap();
    let plane = doc.entry("plane").unwrap();
    let span = doc.packed(plane, "samples").unwrap().unwrap();
    let full = plane_samples();
    assert_eq!(doc.packed_rows_u16(&span, 20, 25).unwrap(), full[20 * 64..25 * 64]);
    assert_eq!(doc.packed_range_u16(&span, 3, 9).unwrap(), full[3..12]);
    assert_eq!(doc.content_hash_ok(plane).unwrap(), Some(true));
    std::fs::remove_file(&path).ok();
}

#[test]
fn far_form_bodies_parse_on_their_own_and_are_walked_by_name() {
    let mut s = VsfSection::new("plane.b0.s0.sensor");
    s.add_field("width", VsfType::u(4, false));
    s.add_field("samples", VsfType::p(BitPackedTensor::pack_u8(4, vec![4], &[1, 2, 3, 4])));
    let far = s.encode_far();
    // Self-describing: the name is inside the body.
    let mut ptr = 0;
    let back = VsfSection::parse(&far, &mut ptr).unwrap();
    assert_eq!(back.name, "plane.b0.s0.sensor");
    assert_eq!(back.fields.len(), 2);
    assert_eq!(ptr, far.len());
    assert_eq!(back.length_hint, Some(far.len()));
    assert_eq!(back.count_hint, Some(2));
    // The skip walker reads the far form too.
    let mut ptr = 0;
    let (name, fields) = vsf::container::skip::skip_section(&far, &mut ptr).unwrap();
    assert_eq!(name, "plane.b0.s0.sensor");
    assert_eq!(fields.len(), 2);
    assert_eq!(ptr, far.len());
    // The near form is unchanged by the refactor: anonymous, name from the TOC.
    let near = s.encode_at_offset(0);
    assert_eq!(near[0], b'[');
    assert_eq!(near[1], b'(');
    assert!(near.len() < far.len());
    // Past 1 MB the near encoder hands out the far form.
    assert_eq!(s.encode_at_offset(2_000_000), far);
}

#[test]
fn truncated_entries_and_bad_headers_fail_loud() {
    let bytes = build();
    let cut = &bytes[..bytes.len() - 500];
    let doc = Document::open(cut).unwrap();
    assert!(matches!(doc.raw(doc.entry("blob").unwrap()), Err(vsf::container::ContainerError::Truncated { .. })));
    assert!(Document::open(&b"not a vsf"[..]).is_err());
    assert!(Document::open(&bytes[..10]).is_err());
}
