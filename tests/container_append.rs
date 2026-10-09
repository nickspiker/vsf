//! Append-only generations: an edit adds bytes at the end and never touches one before it; the latest header lists every live section by absolute offset; `checkout(n)` is the exact document of generation n; undo is truncation; a torn append falls back to the last provable state.

use vsf::container::{append_to_file, truncate_file, Appender, ContainerError, Document};
use vsf::types::{BitPackedTensor, VsfType};
use vsf::VsfBuilder;

fn samples() -> Vec<u16> {
    (0..32 * 16).map(|i| ((i * 977) % 4096) as u16).collect()
}

/// A gen-0 document as today's writers produce one — content hashes on, a plane, a view, a blob.
fn gen0() -> Vec<u8> {
    VsfBuilder::new()
        .content_hashes(true)
        .creation_time_oscillations(1_000_000)
        .add_section("plane", vec![("samples".to_string(), VsfType::p(BitPackedTensor::pack_u16(12, vec![16, 32], &samples())))])
        .add_section("view", vec![("exposure".to_string(), VsfType::f5(0.0))])
        .add_unboxed("blob", vec![7u8; 300])
        .build()
        .unwrap()
}

#[test]
fn an_append_leaves_every_original_byte_in_place_and_supersedes_by_name() {
    let g0 = gen0();
    let doc0 = Document::open(&g0[..]).unwrap();
    assert_eq!(doc0.generation(), 0);
    assert_eq!(doc0.end(), g0.len() as u64);
    let add = Appender::new("opsin", "exposure +1")
        .creation_time_oscillations(2_000_000)
        .add_section("view", vec![("exposure".to_string(), VsfType::f5(1.0))])
        .add_section("graph.g1", vec![("ops".to_string(), VsfType::a("plane\nexposure".to_string()))])
        .supersede("blob")
        .build(&doc0)
        .unwrap();
    let mut all = g0.clone();
    all.extend_from_slice(&add);
    assert_eq!(&all[..g0.len()], &g0[..], "an append must not touch a byte of the original");

    let doc1 = Document::open(&all[..]).unwrap();
    assert_eq!(doc1.generation(), 1);
    assert_eq!(doc1.orphaned_bytes(), 0);
    assert_eq!(doc1.end(), all.len() as u64);
    let names: Vec<&str> = doc1.body_entries().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["plane", "view", "graph.g1"], "blob superseded, view replaced, graph added");
    // The old plane is still served from its original offset; the new view reads the new value; every live entry's content hash checks.
    let plane = doc1.entry("plane").unwrap();
    assert!((plane.offset_bytes as usize) < g0.len());
    let span = doc1.packed(plane, "samples").unwrap().unwrap();
    assert_eq!(doc1.packed_rows_u16(&span, 0, 16).unwrap(), samples());
    let view = doc1.section(doc1.entry("view").unwrap()).unwrap();
    assert!(matches!(view.get_field("exposure").unwrap().values[0], VsfType::f5(v) if v == 1.0));
    assert!((doc1.entry("view").unwrap().offset_bytes as usize) >= g0.len());
    for e in doc1.body_entries() {
        assert_eq!(doc1.content_hash_ok(e).unwrap(), Some(true), "entry '{}'", e.name);
    }
    // The new bodies are far form: they parse on their own with their names.
    let raw = doc1.raw(doc1.entry("graph.g1").unwrap()).unwrap();
    let mut ptr = 0;
    assert_eq!(vsf::VsfSection::parse(&raw, &mut ptr).unwrap().name, "graph.g1");

    // History: two generations, the chain verified; checkout(0) is the original byte for byte.
    let gens = doc1.generations().unwrap();
    assert_eq!(gens.len(), 2);
    assert_eq!((gens[0].index, gens[0].start, gens[0].end), (0, 0, g0.len() as u64));
    assert_eq!((gens[1].index, gens[1].start, gens[1].end), (1, g0.len() as u64, all.len() as u64));
    assert_eq!((gens[1].tool.as_str(), gens[1].action.as_str()), ("opsin", "exposure +1"));
    assert_eq!(doc1.checkout(0).unwrap(), g0);
    assert_eq!(doc1.checkout(1).unwrap(), all);
    assert_eq!(doc1.undo_len().unwrap(), Some(g0.len() as u64));
    assert_eq!(doc0.undo_len().unwrap(), None);
    // The original header still verifies as the gen-0 document it always was.
    assert!(vsf::verification::verify_file_hash(&g0).is_ok());
    // And a reader that stops at the gen-0 header sees the original, unchanged.
    let (h0, _) = vsf::VsfHeader::decode(&all).unwrap();
    assert_eq!(h0.file_length, g0.len());
}

#[test]
fn a_second_generation_chains_and_undo_is_truncation() {
    let g0 = gen0();
    let mut all = g0.clone();
    let add1 = Appender::new("opsin", "crop").add_section("view", vec![("crop".to_string(), VsfType::u(1, false))]).build(&Document::open(&all[..]).unwrap()).unwrap();
    all.extend_from_slice(&add1);
    let len1 = all.len();
    let add2 = Appender::new("opsin", "rotate").add_section("view", vec![("crop".to_string(), VsfType::u(2, false))]).add_unboxed("mask", vec![1u8; 50]).build(&Document::open(&all[..]).unwrap()).unwrap();
    all.extend_from_slice(&add2);

    let doc = Document::open(&all[..]).unwrap();
    assert_eq!(doc.generation(), 2);
    let gens = doc.generations().unwrap();
    assert_eq!(gens.iter().map(|g| g.index).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(gens.iter().map(|g| g.action.as_str()).collect::<Vec<_>>(), ["", "crop", "rotate"]);
    assert_eq!(doc.checkout(1).unwrap(), all[..len1]);
    assert_eq!(doc.checkout(0).unwrap(), g0);
    // Only one `view` is live, the latest.
    assert_eq!(doc.entries_named("view").count(), 1);
    assert_eq!(doc.body_entries().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["plane", "blob", "view", "mask"]);
    // Undo: truncate to generation 1's end, open again, generation 1 is current and its view is the crop=1 one.
    let undone = &all[..doc.undo_len().unwrap().unwrap() as usize];
    let d1 = Document::open(undone).unwrap();
    assert_eq!(d1.generation(), 1);
    assert!(matches!(d1.section(d1.entry("view").unwrap()).unwrap().get_field("crop").unwrap().values[0], VsfType::u3(1)));
    assert!(d1.entry("mask").is_none());
}

#[test]
fn a_torn_append_falls_back_to_the_last_provable_state() {
    let g0 = gen0();
    let add = Appender::new("opsin", "edit").add_section("view", vec![("exposure".to_string(), VsfType::f5(2.0))]).build(&Document::open(&g0[..]).unwrap()).unwrap();
    for cut in [1usize, 10, add.len() / 2, add.len() - 17, add.len() - 1] {
        let mut torn = g0.clone();
        torn.extend_from_slice(&add[..cut]);
        let doc = Document::open(&torn[..]).unwrap();
        assert_eq!(doc.generation(), 0, "cut at {cut}");
        assert_eq!(doc.orphaned_bytes(), cut as u64, "cut at {cut}");
        assert_eq!(doc.checkout(0).unwrap(), g0);
        assert!(matches!(doc.section(doc.entry("view").unwrap()).unwrap().get_field("exposure").unwrap().values[0], VsfType::f5(v) if v == 0.0));
    }
    // A torn third generation after two complete ones falls back to generation 1, not to the original: its header and trailer are still in place.
    let mut two = g0.clone();
    two.extend_from_slice(&add);
    let add2 = Appender::new("opsin", "later").add_section("view", vec![("exposure".to_string(), VsfType::f5(3.0))]).add_unboxed("big", vec![9u8; 3000]).build(&Document::open(&two[..]).unwrap()).unwrap();
    for cut in [1usize, 2000, add2.len() - 17, add2.len() - 1] {
        let mut torn = two.clone();
        torn.extend_from_slice(&add2[..cut]);
        let doc = Document::open(&torn[..]).unwrap();
        assert_eq!(doc.generation(), 1, "cut at {cut}");
        assert_eq!(doc.orphaned_bytes(), cut as u64, "cut at {cut}");
        assert!(matches!(doc.section(doc.entry("view").unwrap()).unwrap().get_field("exposure").unwrap().values[0], VsfType::f5(v) if v == 2.0));
        assert_eq!(doc.generations().unwrap().len(), 2);
        assert_eq!(doc.checkout(1).unwrap(), two);
    }
    // A forged trailer pointing at a header whose `l` does not match the file is ignored the same way.
    let mut forged = g0.clone();
    forged.extend_from_slice(&add);
    let last = forged.len() - 1;
    forged[last] ^= 1; // the trailer's offset is now wrong
    let doc = Document::open(&forged[..]).unwrap();
    assert_eq!(doc.generation(), 0);
    // A tampered previous header breaks the chain, loudly.
    let mut tampered = g0.clone();
    tampered.extend_from_slice(&add);
    tampered[40] ^= 1; // inside the gen-0 header's hash bytes: still parses, no longer matches the chain
    let doc = Document::open(&tampered[..]).unwrap();
    assert_eq!(doc.generation(), 1);
    assert!(matches!(doc.generations(), Err(ContainerError::Chain { generation: 0, .. })));
}

#[test]
fn append_and_truncate_on_a_real_file() {
    let g0 = gen0();
    let path = std::env::temp_dir().join(format!("vsf-append-{}.vsf", std::process::id()));
    std::fs::write(&path, &g0).unwrap();
    let add = {
        let f = std::fs::File::open(&path).unwrap();
        Appender::new("test", "file append").add_section("note", vec![("text".to_string(), VsfType::a("hello".to_string()))]).build(&Document::open(f).unwrap()).unwrap()
    };
    append_to_file(&path, &add).unwrap();
    let f = std::fs::File::open(&path).unwrap();
    let doc = Document::open(f).unwrap();
    assert_eq!(doc.generation(), 1);
    assert_eq!(doc.section(doc.entry("note").unwrap()).unwrap().text("text"), Some("hello"));
    let undo = doc.undo_len().unwrap().unwrap();
    drop(doc);
    truncate_file(&path, undo).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), g0);
    std::fs::remove_file(&path).ok();
}
