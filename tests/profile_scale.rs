//! An entry's magnitude, factored out of its matrix (`idt/fields.md`, "Proposed additions — 2026-09-22", implemented 2026-10-20): `sensitivities`, `gain_refs` and `units` travel together, a Relative entry's unit is reflectance and an Absolute entry's is radiance, and an entry without a scale keeps the v9 meaning (the matrix carries magnitude).

use vsf::spectral_image::{ColourProfile, EntryScale, IdtClass, PlaneLayout, ProfileEntry, ProfileTier, SpectralChannel, Transfer, Unit};
use vsf::types::BitPackedTensor;
use vsf::visual::{Plane, Visual};

fn entry(class: IdtClass, scale: Option<EntryScale>) -> ProfileEntry {
    ProfileEntry { matrix: [0.7, 0.2, 0.1, 0.1, 0.8, 0.1, 0.0, 0.1, 0.9], source: "magic9".into(), class, tier: ProfileTier::Unit, illuminant: 21, transfer: Transfer::Linear, scale }
}

fn round_trip(entries: Vec<ProfileEntry>) -> Result<ColourProfile, String> {
    let ch = |n: &str| SpectralChannel { name: n.into(), curve: None };
    let plane = Plane::sensor("b0", "s0", 1, 1, vec![ch("R"), ch("G"), ch("B")], PlaneLayout::Planar, vec![0.; 3], vec![4095.; 3], BitPackedTensor::pack(12, vec![3, 1, 1], &[1u16, 2, 3]));
    let profile = ColourProfile { target: "vsf_rgb".into(), entries, dng_colormatrix: [None, None], patches: None, cal: None };
    let bytes = Visual::still(plane, None, Some(profile), &[]).write()?;
    Visual::read(&bytes).map_err(|e| e.to_string()).map(|v| v.characterization.unwrap())
}

#[test]
fn a_scaled_entry_round_trips_beside_one_without() {
    let sc = EntryScale { sensitivity: [0.62, 0.81, 0.47], gain_ref: 1.0, unit: Unit::Reflectance };
    let back = round_trip(vec![entry(IdtClass::Relative, Some(sc)), entry(IdtClass::Absolute, None)]).unwrap();
    assert_eq!(back.entries[0].scale, Some(sc));
    assert_eq!(back.entries[1].scale, None, "no scale stays no scale: the matrix keeps the v9 meaning");
}

#[test]
fn the_unit_must_fit_the_class() {
    let radiance = EntryScale { sensitivity: [1e-3; 3], gain_ref: 1.0, unit: Unit::SiRadiance };
    assert!(round_trip(vec![entry(IdtClass::Absolute, Some(radiance))]).is_ok());
    let err = round_trip(vec![entry(IdtClass::Relative, Some(radiance))]).unwrap_err();
    assert!(err.contains("cannot carry unit"), "{err}");
    let reflect = EntryScale { unit: Unit::Reflectance, ..radiance };
    assert!(round_trip(vec![entry(IdtClass::Creative, Some(reflect))]).is_err(), "a look has no physical scale");
}

#[test]
fn transfers_are_no_longer_written_and_read_as_linear() {
    let back = round_trip(vec![entry(IdtClass::Relative, None)]).unwrap();
    assert_eq!(back.entries[0].transfer, Transfer::Linear);
}

#[test]
fn units_fail_loud_on_an_unknown_value() {
    assert!(Unit::parse("nits").is_err());
    assert!(Unit::Reflectance.fits(IdtClass::Relative) && !Unit::Reflectance.fits(IdtClass::Absolute));
}
