//! Regression guard: TEXT/MTEXT placement must survive a native DWG round trip.
//!
//! Covers AC1015 (pre-R2004 two-stream) and AC1018 (paged R2004), the family
//! used by the reported AutoCAD 2006 overlap symptom.  This does not claim to
//! reproduce the customer drawing; it pins the low-level geometry contract.
use acadrust::entities::{AttachmentPoint, DrawingDirection, EntityType, MText, Text,
                        TextHorizontalAlignment, TextVerticalAlignment};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }

fn roundtrip(version: DxfVersion) {
    let mut doc = CadDocument::with_version(version);
    let mut text = Text::with_value("EL48.07m", Vector3::new(10.0, 20.0, 3.0));
    text.height = 1.2;
    text.width_factor = 0.8;
    text.rotation = 0.7;
    text.oblique_angle = 0.2;
    text.horizontal_alignment = TextHorizontalAlignment::Center;
    text.vertical_alignment = TextVerticalAlignment::Middle;
    text.alignment_point = Some(Vector3::new(12.0, 22.0, 3.0));
    text.normal = Vector3::new(0.0, 0.0, 1.0);
    let _ = doc.add_entity(EntityType::Text(text));

    let mut mtext = MText::with_value("IP1\\Pline two", Vector3::new(30.0, 40.0, 0.0));
    mtext.height = 3.0;
    mtext.rectangle_width = 22.955590551181;
    mtext.rotation = 1.396263404935;
    mtext.attachment_point = AttachmentPoint::TopLeft;
    mtext.drawing_direction = DrawingDirection::LeftToRight;
    mtext.line_spacing_factor = 0.7;
    mtext.normal = Vector3::new(0.0, 0.0, 1.0);
    let _ = doc.add_entity(EntityType::MText(mtext));

    let bytes = DwgWriter::write_to_vec(&doc).expect("write");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let back = reader.read().expect("read back");

    let t = back.entities().find_map(|e| match e {
        EntityType::Text(t) if t.value == "EL48.07m" => Some(t),
        _ => None,
    }).expect("TEXT survived");
    assert_eq!(t.value, "EL48.07m");
    assert!(close(t.insertion_point.x, 10.0) && close(t.insertion_point.y, 20.0)
            && close(t.insertion_point.z, 3.0), "TEXT insert: {:?}", t.insertion_point);
    let ap = t.alignment_point.expect("alignment point survived");
    assert!(close(ap.x, 12.0) && close(ap.y, 22.0) && close(ap.z, 3.0), "TEXT align: {:?}", ap);
    assert!(close(t.height, 1.2) && close(t.width_factor, 0.8)
            && close(t.rotation, 0.7) && close(t.oblique_angle, 0.2));
    assert_eq!(t.horizontal_alignment, TextHorizontalAlignment::Center);
    assert_eq!(t.vertical_alignment, TextVerticalAlignment::Middle);

    let m = back.entities().find_map(|e| match e {
        EntityType::MText(m) if m.value == "IP1\\Pline two" => Some(m),
        _ => None,
    }).expect("MTEXT survived");
    assert_eq!(m.value, "IP1\\Pline two");
    assert!(close(m.insertion_point.x, 30.0) && close(m.insertion_point.y, 40.0));
    assert!(close(m.height, 3.0) && close(m.rectangle_width, 22.955590551181));
    assert!(close(m.rotation, 1.396263404935) && close(m.line_spacing_factor, 0.7));
    assert_eq!(m.attachment_point, AttachmentPoint::TopLeft);
    assert_eq!(m.drawing_direction, DrawingDirection::LeftToRight);
}

#[test]
fn text_geometry_roundtrip_ac1015() { roundtrip(DxfVersion::AC1015); }

#[test]
fn text_geometry_roundtrip_ac1018() { roundtrip(DxfVersion::AC1018); }
