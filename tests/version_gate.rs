//! Version-gate tests: z (format version) and y (backward-compat floor) are the wire contract, and `parse_full_header` is the enforcement point.
//! The builder's `.version()` override lets us forge files from other eras, so every gate direction is exercised on real bytes, not mocked values.
//! Companion to the compile-time gate in lib.rs (VSF_VERSION must equal the semver-breaking number) — that one guards the WRITER's constants, these guard the READER's behavior.

use vsf::verification::parse_full_header;
use vsf::vsf_builder::VsfBuilder;
use vsf::{VsfType, VSF_BACKWARD_COMPAT, VSF_VERSION};

fn minimal_file(builder: VsfBuilder) -> Vec<u8> {
    builder
        .add_inline_field("probe", vec![VsfType::u3(42)])
        .build()
        .expect("build must succeed regardless of claimed version")
}

#[test]
fn default_markers_are_the_crate_constants() {
    // A freshly built file must carry the CURRENT z and y — this is the test that would have caught the 0.9.0 yank (spirix wire break shipped still saying z8/y8).
    let bytes = minimal_file(VsfBuilder::new());
    let header = parse_full_header(&bytes).expect("current-era file must parse");
    assert_eq!(header.version, VSF_VERSION);
    assert_eq!(header.backward_compat, VSF_BACKWARD_COMPAT);
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
