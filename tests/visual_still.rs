//! The still-image slice of the visual container: a `Visual` round-trips through bytes, the `SpectralImage` bridge round-trips both ways, a legacy five-section document reads as a one-track visual with its plane linear, the default graph carries the view ops, and every entry carries a content hash.

use vsf::container::Document;
use vsf::spectral_image::{self, CalProvenance, ColourProfile, IdtClass, PlaneLayout, ProfileEntry, ProfileTier, SpectralChannel, SpectralCurve, SpectralImage, Transfer, ViewOp, ViewTransform};
use vsf::types::{BitPackedTensor, Tensor, WorldCoord};
use vsf::visual::{Capture, Encoding, Foreign, Plane, PlaneRole, Samples, SpectralResponse, TrackKind, TrackRole, Visual, OP_BLACK_WHITE, OP_IDT, OP_PLANE};

fn samples() -> Vec<u16> {
    (0..24 * 16).map(|i| ((i * 4099) % 16384) as u16).collect()
}

fn profile() -> ColourProfile {
    ColourProfile {
        target: "vsf_rgb".into(),
        entries: vec![ProfileEntry { matrix: [1.2, -0.3, -0.1, -0.5, 1.6, 0.26, -0.2, 0.38, 0.66], source: "magic9".into(), class: IdtClass::Relative, tier: ProfileTier::Unit, illuminant: 23, transfer: Transfer::Linear, scale: None }],
        dng_colormatrix: [Some(([1.5, -0.7, -0.2, -1.1, 2.2, 0.05, 0.08, -0.3, 1.6], 17)), None],
        patches: Some((vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], vec![0.11, 0.21, 0.31, 0.41, 0.51, 0.61])),
        cal: Some(CalProvenance { target_type: 1, target_serial: 13, timestamp: "2026-07-01".into() }),
    }
}

fn capture() -> Capture {
    Capture {
        exposure_s: Some(1.0 / 125.0),
        f_number: Some(2.8),
        iso: Some(200.0),
        focal_mm: Some(45.0),
        focal_35mm: Some(45.0),
        at: Some(1_234_567_890_123),
        at_text: "2026:10:09 10:11:12".into(),
        location: Some(WorldCoord::from_lat_lon(48.8584, 2.2945)),
        altitude_m: Some(35.0),
        make: "SIGMA".into(),
        model: "SIGMA fp".into(),
        lens: "45mm F2.8 DG DN".into(),
        mode: "single".into(),
        baseline: 0.5,
        ..Capture::default()
    }
}

fn still() -> Visual {
    let mut channels = vec![SpectralChannel { name: "R".into(), curve: None }, SpectralChannel { name: "G".into(), curve: Some(SpectralCurve { start_nm: 400.0, step_nm: 10.0, values: vec![0.1, 0.5, 0.9, 0.5, 0.1] }) }, SpectralChannel { name: "B".into(), curve: None }];
    channels[0].name = "R".into();
    let plane = Plane {
        maker_crop: Some([2, 1, 20, 14]),
        orientation: 6,
        ..Plane::sensor("b0", "s0", 24, 16, channels, PlaneLayout::Mosaic { cfa: Tensor::new(vec![2, 2], vec![0u8, 1, 1, 2]) }, vec![1024.0; 3], vec![16383.0; 3], BitPackedTensor::pack_u16(14, vec![16, 24], &samples()))
    };
    let ops = vec![ViewOp { name: "orientation".into(), class: IdtClass::Technical, params: vec![6.0] }, ViewOp { name: "crop".into(), class: IdtClass::Technical, params: vec![1.0, 2.0, 10.0, 8.0] }, ViewOp { name: "exposure".into(), class: IdtClass::Technical, params: vec![0.7] }, ViewOp { name: "dr_curve".into(), class: IdtClass::Creative, params: vec![0.0, 1.5, 0.0, -0.5] }];
    let mut v = Visual::still(plane, Some(capture()), Some(profile()), &ops);
    v.text = "a test frame".into();
    v.observer = "cvrl2006".into();
    v.foreign = Foreign { xmp: Some(b"<x:xmpmeta/>".to_vec()), icc: Some(vec![1, 2, 3, 4]), profile_name: "Verichrome scene-relative IDT".into(), calibration_signature: "VERICHROME relative unit target#13 -".into() };
    // The sensor's response under the calibration light: three channels on a 5-band grid, one common scale.
    v.response = Some(SpectralResponse { start_nm: 400.0, step_nm: 80.0, channels: vec![vec![0.0, 0.1, 0.4, 1.0, 0.3], vec![0.1, 0.7, 1.3, 0.5, 0.0], vec![0.9, 1.1, 0.2, 0.0, 0.0]], prior: "shortest".into(), light: Some([7u8; 32]) });
    v
}

#[test]
fn a_still_round_trips_with_content_hashes_on_every_entry() {
    let v = still();
    let bytes = v.write().unwrap();
    let doc = Document::open(&bytes[..]).unwrap();
    let names: Vec<&str> = doc.body_entries().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["visual", "track.b0", "span.b0.s0", "capture", "characterization", "graph.g0", "view.v0", "foreign", "plane.b0.s0.sensor"]);
    for e in doc.body_entries() {
        assert_eq!(doc.content_hash_ok(e).unwrap(), Some(true), "entry '{}'", e.name);
    }
    let back = Visual::read(&bytes).unwrap();
    assert_eq!(back, v);
    assert_eq!(back.response.as_ref().map(|r| r.channels.len()), Some(3));
    // The plane is reachable lazily, by name, rows at a time.
    let plane = doc.entry("plane.b0.s0.sensor").unwrap();
    let span = doc.packed(plane, "samples").unwrap().unwrap();
    assert_eq!(span.shape, vec![16, 24]);
    assert_eq!(doc.packed_rows_u16(&span, 3, 5).unwrap(), samples()[3 * 24..5 * 24]);
    // Creation time came from the capture instant.
    assert!(matches!(doc.header().creation_time, Some(vsf::types::VsfType::e(vsf::types::EtType::e6(1_234_567_890_123)))));
}

#[test]
fn the_default_graph_carries_the_view_ops_and_the_bridge_round_trips() {
    let v = still();
    let g = v.default_graph().unwrap();
    assert_eq!(g.ops.iter().map(|o| o.name.as_str()).collect::<Vec<_>>(), [OP_PLANE, OP_BLACK_WHITE, OP_IDT, "orientation", "crop", "exposure", "dr_curve"]);
    assert_eq!(g.ops[0].reference, "plane.b0.s0.sensor");
    assert_eq!(g.root, 6);
    assert!(g.ops[3..].iter().all(|o| o.arity == 1));
    let vt = g.to_view_transform();
    assert_eq!(vt.ops.len(), 4);
    assert_eq!(vt.ops[2], ViewOp { name: "exposure".into(), class: IdtClass::Technical, params: vec![0.7] });
    // To the in-memory model and back.
    let img = v.to_spectral_image().unwrap();
    assert_eq!((img.width, img.height), (24, 16));
    assert_eq!(img.make, "SIGMA");
    assert_eq!(img.profile, Some(profile()));
    assert_eq!(img.view.as_ref().map(|v| v.ops.len()), Some(4));
    assert_eq!(img.samples.unpack_u16(), samples());
    let again = Visual::from_spectral_image(&img, Some(capture()), v.foreign.clone());
    assert_eq!(again.primary_plane().unwrap().samples, v.primary_plane().unwrap().samples);
    assert_eq!(again.default_graph().unwrap().to_view_transform(), vt);
    assert_eq!(again.capture, v.capture);
    // The graph's hash names the program, and changes with it.
    let h1 = g.hash();
    let mut g2 = g.clone();
    g2.ops[5].params[0] = 0.8;
    assert_ne!(h1, g2.hash());
}

#[test]
fn a_legacy_spectral_image_document_reads_as_a_linear_one_track_visual() {
    let img = SpectralImage {
        width: 24,
        height: 16,
        channels: vec![SpectralChannel { name: "R".into(), curve: None }, SpectralChannel { name: "G".into(), curve: None }, SpectralChannel { name: "B".into(), curve: None }],
        layout: PlaneLayout::Mosaic { cfa: Tensor::new(vec![2, 2], vec![1u8, 2, 0, 1]) },
        samples: BitPackedTensor::pack_u16(16, vec![16, 24], &samples()),
        black: vec![3023.0; 3],
        white: vec![48320.0; 3],
        make: "Android".into(),
        model: "Lumis".into(),
        provenance: Default::default(),
        profile: Some(profile()),
        view: Some(ViewTransform { space: "vsf_rgb_linear".into(), ops: vec![ViewOp { name: "orientation".into(), class: IdtClass::Technical, params: vec![8.0] }, ViewOp { name: "exposure".into(), class: IdtClass::Technical, params: vec![-1.0] }] }),
    };
    let legacy = spectral_image::write(&img).unwrap();
    let v = Visual::read(&legacy).unwrap();
    assert_eq!(v.tracks.len(), 1);
    assert_eq!((v.tracks[0].kind, v.tracks[0].role), (TrackKind::Beam, TrackRole::Capture));
    let p = v.primary_plane().unwrap();
    assert_eq!(p.role, PlaneRole::Sensor);
    assert_eq!(p.encoding, Encoding::Linear, "a legacy plane is linear: its only writer wrote linear counts");
    assert_eq!(p.depth, 16);
    assert!(matches!(&p.samples, Samples::Packed(t) if t.unpack_u16() == samples()));
    assert_eq!(v.capture.as_ref().map(|c| c.model.as_str()), Some("Lumis"));
    assert_eq!(v.characterization, Some(profile()));
    assert_eq!(v.default_graph().unwrap().to_view_transform(), img.view.clone().unwrap());
    // And the bridge gives the legacy reader's result back.
    let bridged = v.to_spectral_image().unwrap();
    assert_eq!(bridged, spectral_image::read(&legacy).unwrap());
    // Legacy write/read is itself unchanged by the refactor.
    assert_eq!(spectral_image::read(&legacy).unwrap(), img);
}

#[test]
fn a_plane_without_encoding_reads_as_gamma2_and_an_unknown_role_fails_loud() {
    // Hand-forge a plane section lacking `encoding` to pin the absence rule.
    let v = still();
    let bytes = v.write().unwrap();
    let doc = Document::open(&bytes[..]).unwrap();
    let mut s = doc.section(doc.entry("plane.b0.s0.sensor").unwrap()).unwrap();
    s.fields.retain(|f| f.name != "encoding");
    let gen0 = vsf::VsfBuilder::new().add_section("visual", vec![("default_view".to_string(), vsf::types::VsfType::a("view.v0".into()))]).add_section_direct(s.clone()).build().unwrap();
    let read = Visual::read(&gen0).unwrap();
    assert_eq!(read.primary_plane().unwrap().encoding, Encoding::Gamma2);
    let mut bad = s.clone();
    for f in &mut bad.fields {
        if f.name == "role" {
            f.values[0] = vsf::types::VsfType::a("hologram".into());
        }
    }
    let gen0 = vsf::VsfBuilder::new().add_section("visual", vec![]).add_section_direct(bad).build().unwrap();
    assert!(Visual::read(&gen0).is_err());
}
