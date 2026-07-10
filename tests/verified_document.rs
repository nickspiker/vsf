//! Committed coverage for the GENERIC verified-document round-trip.
//!
//! Phases 1-3 built the pieces (VsfBuilder auto-fills hp+hb, is_original/verify_file_hash verify them, read_verified composes an un-skippable check, and SectionBuilder::parse_document reads a named section only after read_verified passes).
//! Nothing pinned those guarantees down as tests on a NON-image document, so this module does: it builds a plain "payload" section, verifies both hashes, reads it back thru parse_document, and — the load-bearing part — proves that the things that MUST be rejected actually are (a bare error-frame-shaped doc with no integrity, a tampered byte) while the things that MUST be tolerated are (an unknown forward-compat field).

use vsf::schema::{SectionSchema, TypeConstraint};
use vsf::verification::{is_original, read_verified, verify_file_hash};
use vsf::vsf_builder::VsfBuilder;
use vsf::VsfType;

/// blake3 of a fixed input, so the digest is a real 32-byte hash rather than zeros.
fn sample_digest() -> [u8; 32] {
    *blake3::hash(b"octopus-payload-digest").as_bytes()
}

/// The schema describing the "payload" section the tests build and read back.
fn payload_schema() -> SectionSchema {
    SectionSchema::new("payload")
        .field("count", TypeConstraint::AnyUnsigned)
        .field("digest", TypeConstraint::AnyHash)
        .field("label", TypeConstraint::AnyString)
}

/// Build the canonical well-formed payload document (rolling hash, no signature).
fn build_payload_doc() -> Vec<u8> {
    VsfBuilder::new()
        .add_section(
            "payload",
            vec![
                ("count".to_string(), VsfType::u4(0xBEEF)),
                ("digest".to_string(), VsfType::hp(sample_digest().to_vec())),
                ("label".to_string(), VsfType::a("octopus".to_string())),
            ],
        )
        .build()
        .expect("well-formed payload document must build")
}

#[test]
fn verified_round_trip_reads_fields_back() {
    let doc = build_payload_doc();

    // Both hashes the builder auto-filled must verify on the untouched bytes.
    is_original(&doc).expect("provenance (hp) must self-verify on a fresh build");
    verify_file_hash(&doc).expect("rolling hash (hb) must verify on a fresh build");

    // read_verified is the composed gate — a rolling-hash doc with no signature passes.
    read_verified(&doc, None).expect("well-formed doc must pass read_verified");

    // Read the section back thru the verifying front door.
    let parsed = vsf::schema::SectionBuilder::parse_document(payload_schema(), &doc, None)
        .expect("parse_document must read a verified section");

    let count: u16 = parsed
        .get_value("count")
        .expect("count field must be present");
    assert_eq!(count, 0xBEEF, "count must round-trip exactly");

    // The digest bytes must survive the round-trip unchanged.
    let digest_values = parsed.get("digest").expect("digest field must be present");
    let digest_bytes = match digest_values.first() {
        Some(VsfType::hp(bytes)) => bytes.clone(),
        other => panic!("digest should decode as hp, got {:?}", other),
    };
    assert_eq!(
        digest_bytes,
        sample_digest().to_vec(),
        "digest bytes must round-trip unchanged"
    );

    // And the string label too.
    let label: String = parsed.get_value("label").expect("label field must be present");
    assert_eq!(label, "octopus", "label must round-trip exactly");
}

#[test]
fn hp_only_documents_verify_on_provenance_alone() {
    // An hp-only document (provenance_only shape): for an UNSIGNED doc, hp and hb are equally self-attesting BLAKE3 anchors, so hp alone suffices — is_original is the integrity check.
    // (Error frames are NOT filtered here — error-frame detection is a separate, explicit, FIRST step at every read; see fgtw::client::error_frame.)
    let hp_only = VsfBuilder::new()
        .provenance_only()
        .add_section(
            "error",
            vec![
                ("reason".to_string(), VsfType::a("worker_failed".to_string())),
                (
                    "detail".to_string(),
                    VsfType::a("upstream timeout".to_string()),
                ),
            ],
        )
        .build()
        .expect("hp-only doc must build");

    is_original(&hp_only).expect("hp self-consistency");
    read_verified(&hp_only, None).expect("hp-only doc verifies on provenance alone");

    // Tampering must still be caught by the hp check.
    let mut tampered = hp_only.clone();
    let last = tampered.len() - 2;
    tampered[last] ^= 0xFF;
    read_verified(&tampered, None).expect_err("tampered hp-only doc must be rejected");

    // A PINNED-SIGNER read still refuses an unsigned doc — hp cannot substitute for authenticity.
    read_verified(&hp_only, Some([0x42; 32]))
        .expect_err("pinned-signer read must refuse an unsigned hp-only doc");

    // parse_document inherits the same acceptance.
    let schema = SectionSchema::new("error").field("reason", TypeConstraint::AnyString);
    let sec = vsf::schema::SectionBuilder::parse_document(schema, &hp_only, None)
        .expect("parse_document accepts an hp-only doc");
    let reason: String = sec.get_value("reason").expect("reason field");
    assert_eq!(reason, "worker_failed");
}

#[test]
fn tampering_breaks_both_hashes() {
    let mut doc = build_payload_doc();

    // Both hashes verify before tampering.
    is_original(&doc).expect("clean doc provenance ok");
    verify_file_hash(&doc).expect("clean doc rolling hash ok");

    // Flip a byte inside the section body (the last byte before the closing ']').
    // Walk back from the end to find the final ']' and tamper the byte just before it.
    let close = doc
        .iter()
        .rposition(|&b| b == b']')
        .expect("document must have a closing bracket");
    let target = close - 1;
    doc[target] ^= 0x01;

    // Provenance no longer matches the mutated content.
    assert!(
        is_original(&doc).is_err(),
        "provenance must fail after a payload byte is flipped"
    );
    // Rolling hash likewise fails.
    assert!(
        verify_file_hash(&doc).is_err(),
        "rolling hash must fail after a payload byte is flipped"
    );
    // And the composed gate refuses it.
    assert!(
        read_verified(&doc, None).is_err(),
        "read_verified must refuse a tampered doc"
    );
}

#[test]
fn parse_document_tolerates_unknown_field() {
    // Build a doc whose section carries an extra field the schema does not know about.
    let doc = VsfBuilder::new()
        .add_section(
            "payload",
            vec![
                ("count".to_string(), VsfType::u4(0xBEEF)),
                ("digest".to_string(), VsfType::hp(sample_digest().to_vec())),
                ("label".to_string(), VsfType::a("octopus".to_string())),
                // Forward-compat: a newer writer added this; an older schema must not choke.
                ("surprise".to_string(), VsfType::u3(1)),
            ],
        )
        .build()
        .expect("doc with an extra field must build");

    // The schema deliberately omits "surprise".
    let parsed = vsf::schema::SectionBuilder::parse_document(payload_schema(), &doc, None)
        .expect("parse_document must tolerate an unknown extra field");

    // Known fields still come thru untouched.
    let count: u16 = parsed.get_value("count").expect("count still present");
    assert_eq!(count, 0xBEEF);
    let label: String = parsed.get_value("label").expect("label still present");
    assert_eq!(label, "octopus");

    // The unknown field is discarded, not surfaced.
    assert!(
        parsed.get("surprise").is_err(),
        "unknown field must be dropped from the parsed builder"
    );
}

/// Write the well-formed acceptance fixture to /tmp/accept.vsf so the vsfinfo acceptance run has a real, verified file to inspect.
/// Not a guarantee test — a fixture emitter — but it keeps the file generation in-tree and reproducible.
#[test]
fn emit_accept_fixture() {
    let doc = build_payload_doc();
    is_original(&doc).expect("fixture provenance ok");
    verify_file_hash(&doc).expect("fixture rolling hash ok");
    std::fs::write("/tmp/accept.vsf", &doc).expect("write /tmp/accept.vsf");
}

#[cfg(feature = "crypto")]
#[test]
fn emit_accept_signed_fixture() {
    use ed25519_dalek::SigningKey;
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let pubkey: [u8; 32] = signing_key.verifying_key().to_bytes();
    let secret: [u8; 32] = signing_key.to_bytes();
    let unsigned = VsfBuilder::new()
        .signed_only(VsfType::ke(pubkey.to_vec()))
        .add_section(
            "payload",
            vec![
                ("count".to_string(), VsfType::u4(0xBEEF)),
                ("label".to_string(), VsfType::a("octopus".to_string())),
            ],
        )
        .build()
        .expect("signed_only doc must build");
    let signed = vsf::verification::sign_file(unsigned, &secret).expect("sign_file");
    read_verified(&signed, Some(pubkey)).expect("signed fixture must verify");
    std::fs::write("/tmp/accept_signed.vsf", &signed).expect("write /tmp/accept_signed.vsf");
}

#[cfg(feature = "crypto")]
#[test]
fn signed_round_trip_checks_signer() {
    use ed25519_dalek::SigningKey;

    // A deterministic keypair for the test.
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let pubkey: [u8; 32] = signing_key.verifying_key().to_bytes();
    let secret: [u8; 32] = signing_key.to_bytes();

    // Build an unsigned doc carrying the signer pubkey (ke) + a ge placeholder, then fill the header-level signature with sign_file.
    let unsigned = VsfBuilder::new()
        .signed_only(VsfType::ke(pubkey.to_vec()))
        .add_section(
            "payload",
            vec![
                ("count".to_string(), VsfType::u4(0xBEEF)),
                ("label".to_string(), VsfType::a("octopus".to_string())),
            ],
        )
        .build()
        .expect("signed_only doc must build");

    let signed =
        vsf::verification::sign_file(unsigned, &secret).expect("sign_file must fill the signature");

    // read_verified with the correct signer passes.
    read_verified(&signed, Some(pubkey)).expect("correct signer must pass read_verified");

    // And with no expected signer it still passes (signature itself is valid).
    read_verified(&signed, None).expect("valid signature with no pin must pass");

    // A wrong expected signer must be rejected as untrusted.
    let wrong = [9u8; 32];
    let err = read_verified(&signed, Some(wrong))
        .expect_err("wrong signer key must be rejected");
    assert!(
        err.contains("untrusted"),
        "wrong-signer rejection must name the reason, got: {err}"
    );

    // The section still reads back thru parse_document under the correct signer.
    let schema = SectionSchema::new("payload")
        .field("count", TypeConstraint::AnyUnsigned)
        .field("label", TypeConstraint::AnyString);
    let parsed = vsf::schema::SectionBuilder::parse_document(schema, &signed, Some(pubkey))
        .expect("parse_document must read a signed, verified section");
    let count: u16 = parsed.get_value("count").expect("count present in signed doc");
    assert_eq!(count, 0xBEEF);
}

/// A pinned-signer read must FAIL on an unsigned (hb-only) document — stripping ke/ge cannot demote the check to integrity-only.
#[test]
fn pinned_signer_rejects_unsigned_document() {
    let doc = vsf::VsfBuilder::new()
        .creation_time_oscillations(vsf::eagle_time_oscillations())
        .add_section("data", vec![("x".to_string(), vsf::VsfType::u(7, false))])
        .build()
        .unwrap();
    // Unpinned read: fine (hb verifies).
    assert!(vsf::verification::read_verified(&doc, None).is_ok());
    // Pinned read: must refuse the unsigned doc.
    let err = vsf::verification::read_verified(&doc, Some([0x42; 32])).unwrap_err();
    assert!(err.contains("unsigned"), "unexpected error: {err}");
}
