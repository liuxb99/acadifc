//! Phase 4 P1 regressions: MTEXT X direction / unknown bit / STYLE handle.
use acadrust::entities::MText;
use acadrust::types::{DxfVersion, Handle, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::f64::consts::FRAC_PI_2;
use std::io::Cursor;

fn roundtrip(mut doc: CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(&doc).expect("write");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    r.read().expect("read back")
}

fn first_mtext(doc: &CadDocument) -> &MText {
    doc.entities().find_map(|e| match e {
        EntityType::MText(m) => Some(m),
        _ => None,
    }).expect("MTEXT present")
}

#[test]
fn x_direction_with_nonzero_z_is_preserved_verbatim() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut m = MText::new();
    let direction = Vector3::new(0.3, 0.4, 0.866025403784).normalize();
    m.dwg_x_direction = Some(direction);
    m.normal = Vector3::new(0.0, 0.0, 1.0);
    m.rotation = 0.643501108793;
    m.dwg_unknown_bit = true;
    doc.add_entity(EntityType::MText(m)).unwrap();

    let back = roundtrip(doc);
    let got = first_mtext(&back);
    let d = got.dwg_x_direction.expect("direction preserved");
    assert!((d.x - direction.x).abs() < 1e-9
            && (d.y - direction.y).abs() < 1e-9
            && (d.z - direction.z).abs() < 1e-9, "got {d:?}");
    assert!(d.z.abs() > 0.1, "non-zero Z must survive");
    assert!(got.dwg_unknown_bit, "unknown bit must survive");
}

#[test]
fn changed_rotation_recomputes_direction_in_tilted_ocs() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut m = MText::new();
    m.dwg_x_direction = Some(Vector3::new(1.0, 0.0, 0.0));
    m.normal = Vector3::new(0.0, 1.0, 0.0).normalize();
    m.set_rotation(FRAC_PI_2);
    doc.add_entity(EntityType::MText(m)).unwrap();

    let back = roundtrip(doc);
    let got = first_mtext(&back);
    let d = got.dwg_x_direction.expect("direction recomputed");
    assert!(d.z.abs() > 0.9, "OCS normal (0,1,0): rotated X has Z=-1, got {d:?}");
    assert!((d.x * d.x + d.y * d.y + d.z * d.z - 1.0).abs() < 1e-9);
}

#[test]
fn tiny_angle_error_does_not_rewrite_original_direction() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut m = MText::new();
    let direction = Vector3::new(0.999_999_999, 1e-9, 1e-9).normalize();
    m.dwg_x_direction = Some(direction);
    m.rotation = 1e-9; // not bit-identical to atan2(direction)
    doc.add_entity(EntityType::MText(m)).unwrap();

    let back = roundtrip(doc);
    let d = first_mtext(&back).dwg_x_direction.expect("direction");
    assert_eq!((d.x.to_bits(), d.y.to_bits(), d.z.to_bits()),
               (direction.x.to_bits(), direction.y.to_bits(), direction.z.to_bits()));
}

#[test]
fn style_handle_is_preserved_when_style_unchanged() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let style_handle = doc.text_styles.iter().next().map(|s| s.handle)
        .unwrap_or(Handle::new(1));
    let mut m = MText::new();
    m.style_handle = Some(style_handle);
    doc.add_entity(EntityType::MText(m)).unwrap();

    let back = roundtrip(doc);
    assert_eq!(first_mtext(&back).style_handle, Some(style_handle));
}

#[test]
fn unresolvable_changed_style_is_a_fidelity_error() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut m = MText::new();
    m.style_handle = Some(Handle::new(1234));
    m.set_style("NO_SUCH_STYLE");
    doc.add_entity(EntityType::MText(m)).unwrap();

    let err = DwgWriter::write_to_vec(&doc).expect_err("must refuse lossy STYLE");
    assert!(err.to_string().contains("fidelity error"), "{err}");
}
