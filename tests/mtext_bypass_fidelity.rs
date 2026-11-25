//! Phase 5 regressions: dirty tracking cannot be bypassed, extents invalidated.
use acadrust::entities::MText;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::io::Cursor;

fn write_read(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    r.read().expect("read back")
}

/// Read a written MTEXT once so the DWG reader has populated its semantic
/// snapshot; the returned entity can then be mutated by any bypass route.
fn seeded() -> MText {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut m = MText::new();
    m.value = "IP1".into();
    m.extents_width = 5.0;
    m.extents_height = 2.0;
    m.dwg_x_direction = Some(Vector3::new(1.0, 0.0, 0.0));
    doc.add_entity(EntityType::MText(m)).unwrap();
    let back = write_read(&doc);
    let found = back.entities().find_map(|e| match e { EntityType::MText(m) => Some(m.clone()), _ => None }).unwrap();
    found
}

fn with(m: MText) -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    doc.add_entity(EntityType::MText(m)).unwrap();
    doc
}

fn reopened(m: MText) -> MText {
    let back = write_read(&with(m));
    let found = back.entities().find_map(|e| match e { EntityType::MText(m) => Some(m.clone()), _ => None }).unwrap();
    found
}

#[test]
fn no_op_preserves_direction_and_extents() {
    let m = seeded();
    assert_eq!(m.source_rotation, Some(m.rotation));
    assert_eq!(m.source_value.as_deref(), Some(m.value.as_str()));
    let out = reopened(m.clone());
    assert_eq!(out.dwg_x_direction, m.dwg_x_direction);
    assert_eq!((out.extents_width, out.extents_height), (m.extents_width, m.extents_height));
}

#[test]
fn direct_rotation_assignment_cannot_bypass_detection() {
    let mut m = seeded();
    let original = m.dwg_x_direction.unwrap();
    m.rotation = 1.0; // direct public-field assignment, no setter
    let out = reopened(m);
    let got = out.dwg_x_direction.unwrap();
    assert!((got.x - original.x).abs() > 0.1, "direction must be recomputed: {got:?}");
}

#[test]
fn clone_then_direct_mutation_is_detected_and_original_untouched() {
    let m = seeded();
    let original_direction = m.dwg_x_direction.unwrap();
    let mut mutated = m.clone();
    mutated.rotation = 1.0;
    let out = reopened(mutated);
    let got = out.dwg_x_direction.unwrap();
    assert!((got.x - original_direction.x).abs() > 0.1);
    assert_eq!(m.dwg_x_direction, Some(original_direction), "source clone unchanged");
}

#[test]
fn patch_caller_setter_invalidates_extents() {
    let mut m = seeded();
    assert!(m.extents_are_valid(), "seeded extents valid");
    m.set_value("changed text");
    assert!(!m.extents_are_valid());
    let out = reopened(m);
    assert_eq!((out.extents_width, out.extents_height), (0.0, 0.0));
}

#[test]
fn direct_text_assignment_invalidates_extents() {
    let mut m = seeded();
    m.value = "direct edit".into();
    assert!(!m.extents_are_valid(), "snapshot must catch direct assignment");
    let out = reopened(m);
    assert_eq!((out.extents_width, out.extents_height), (0.0, 0.0));
}

#[cfg(feature = "serde")]
#[test]
fn serde_roundtrip_keeps_snapshot_and_detects_change() {
    let m = seeded();
    let json = serde_json::to_string(&m).unwrap();
    let mut back: MText = serde_json::from_str(&json).unwrap();
    assert_eq!(back.source_rotation, m.source_rotation);
    back.rotation = 1.0;
    let out = reopened(back);
    assert!(out.dwg_x_direction != m.dwg_x_direction, "serde snapshot must not bypass detection");
}
