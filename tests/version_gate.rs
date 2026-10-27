//! Version-gate tests: z (format version) and y (backward-compat floor) are the wire contract, and `parse_full_header` is the enforcement point. The builder's `.version()` override lets us forge files from other eras, so every gate direction is exercised on real bytes, not mocked values. Companion to the compile-time gate in lib.rs (VSF_VERSION must equal the semver-breaking number) — that one guards the WRITER's constants, these guard the READER's behavior.

use vsf::verification::parse_full_header;
use vsf::vsf_builder::VsfBuilder;
use vsf::types::{HEADER_SINCE, SINCE_TABLE};
use vsf::{VsfType, VSF_BACKWARD_COMPAT, VSF_VERSION};

fn minimal_file(builder: VsfBuilder) -> Vec<u8> {
    builder
        .add_inline_field("probe", vec![VsfType::u3(42)])
        .build()
        .expect("build must succeed regardless of claimed version")
}

#[test]
fn default_markers_are_the_version_and_the_computed_floor() {
    // A freshly built file carries the CURRENT z, and a y worked out from what it holds: a u3 in an inline field needs nothing newer than the header, so y is the header's version.
    let bytes = minimal_file(VsfBuilder::new());
    let header = parse_full_header(&bytes).expect("current-era file must parse");
    assert_eq!(header.version, VSF_VERSION);
    assert_eq!(header.backward_compat, HEADER_SINCE);
    assert!(header.backward_compat >= VSF_BACKWARD_COMPAT, "a file this build writes must be one this build reads");
}

#[test]
fn floor_is_the_latest_type_the_file_holds() {
    let builder = VsfBuilder::new()
        .add_section("data", vec![("t".to_string(), VsfType::t_f5(vsf::Tensor::new(vec![3], vec![1.0, 2.0, 3.0]))), ("n".to_string(), VsfType::u3(1))])
        .add_inline_field("probe", vec![VsfType::a("x".into())]);
    let expect = SINCE_TABLE.iter().filter(|(m, _)| ["t_f5", "u3", "a"].contains(m)).map(|(_, v)| *v).fold(HEADER_SINCE, usize::max);
    assert_eq!(builder.floor().version, expect);
    let header = parse_full_header(&builder.build().unwrap()).unwrap();
    assert_eq!(header.backward_compat, expect);
}

#[test]
fn requires_raises_the_floor_and_a_lower_claim_is_raised_to_it() {
    // What the builder cannot see (ciphertext, a VSF blob) is declared; a declaration can raise y but never lower it below what the values need.
    let raised = parse_full_header(&minimal_file(VsfBuilder::new().requires(VSF_VERSION))).unwrap();
    assert_eq!(raised.backward_compat, VSF_VERSION);
    let low = parse_full_header(&minimal_file(VsfBuilder::new().version(VSF_VERSION, 1))).unwrap();
    assert_eq!(low.backward_compat, HEADER_SINCE);
}

#[test]
fn a_generation_keeps_the_previous_floor() {
    // The carried sections were written under the previous generation's y; an append that adds only old types must not lower it.
    use vsf::container::{Appender, Document};
    let gen0 = minimal_file(VsfBuilder::new().requires(VSF_VERSION));
    let doc0 = Document::open(&gen0[..]).unwrap();
    let tail = Appender::new("test", "add").add_section("more", vec![("n".to_string(), VsfType::u3(2))]).build(&doc0).unwrap();
    let mut all = gen0.clone();
    all.extend_from_slice(&tail);
    let doc1 = Document::open(&all[..]).unwrap();
    assert_eq!(doc1.generation(), 1);
    assert_eq!(doc1.header().backward_compat, VSF_VERSION);
}

#[test]
fn future_floor_is_rejected() {
    // y ABOVE our version: the writer declares "you need at least v10 to read this correctly" — we are v9, so the only correct behavior is a loud version error, never a parse attempt.
    let future = VSF_VERSION + 1;
    let bytes = minimal_file(VsfBuilder::new().version(future, future));
    let err = match parse_full_header(&bytes) {
        Err(e) => e,
        Ok(_) => panic!("future floor must be rejected"),
    };
    assert!(
        err.contains(&format!("requires VSF v{future}")),
        "wrong error: {err}"
    );
}

#[test]
fn future_version_with_current_floor_is_accepted() {
    // The whole POINT of y: a v10 writer that stayed v9-compatible sets y=9, and a v9 reader accepts. Forward compatibility rides on the floor, not the version.
    let bytes = minimal_file(VsfBuilder::new().version(VSF_VERSION + 1, VSF_VERSION));
    let header = parse_full_header(&bytes).expect("y at our version must be readable");
    assert_eq!(header.version, VSF_VERSION + 1);
    assert_eq!(header.backward_compat, VSF_VERSION);
}

#[test]
fn below_floor_is_rejected() {
    // z BELOW our compat floor: v8 spirix payload bytes mean different values under v9 semantics, so parsing would silently misdecode — the floor turns that into a loud error naming the compat feature that would be needed.
    let old = VSF_BACKWARD_COMPAT - 1;
    let bytes = minimal_file(VsfBuilder::new().version(old, old));
    let err = match parse_full_header(&bytes) {
        Err(e) => e,
        Ok(_) => panic!("below-floor file must be rejected"),
    };
    assert!(
        err.contains(&format!("compat-v{old}")),
        "wrong error: {err}"
    );
}
