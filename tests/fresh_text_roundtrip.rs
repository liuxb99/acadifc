//! Regression: a freshly generated drawing must keep entity TEXT/MTEXT values.
//!
//! 1.0.9 wrote entity text to the main stream instead of the R2004+ text stream,
//! so a generated drawing reopened with every TEXT value empty. These tests use
//! a fresh document (no raw replay), which is the path that broke.
use acadrust::entities::{MText, Text};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::io::Cursor;

fn roundtrip(mut doc: CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(&doc).expect("write");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    r.read().expect("read back")
}

fn check_text(version: DxfVersion) {
    let mut doc = CadDocument::with_version(version);
    let mut text = Text::new();
    text.value = "HELLO WORLD".to_string();
    text.insertion_point = Vector3::new(1.0, 2.0, 0.0);
    text.height = 2.5;
    doc.add_entity(EntityType::Text(text)).unwrap();

    let back = roundtrip(doc);
    let got = back.entities().find_map(|e| match e {
        EntityType::Text(t) => Some(t),
        _ => None,
    }).expect("TEXT present");
    assert_eq!(got.value, "HELLO WORLD", "fresh TEXT value must survive reopen ({version:?})");
}

#[test]
fn fresh_text_value_survives_roundtrip_ac1018() {
    // R2004+ (two-stream merge mode)
    check_text(DxfVersion::AC1018);
}

#[test]
fn fresh_text_value_survives_roundtrip_ac1032() {
    // R2018+ uses the three-stream merge mode, where entity text must go to the
    // text stream - this is the 1.0.9 regression path (generated TEXT empty).
    check_text(DxfVersion::AC1032);
}

fn check_mtext(version: DxfVersion) {
    let mut doc = CadDocument::with_version(version);
    let mut m = MText::new();
    m.value = "STA 0+000.00".to_string();
    m.insertion_point = Vector3::new(0.0, 0.0, 0.0);
    doc.add_entity(EntityType::MText(m)).unwrap();

    let back = roundtrip(doc);
    let got = back.entities().find_map(|e| match e {
        EntityType::MText(m) => Some(m),
        _ => None,
    }).expect("MTEXT present");
    assert_eq!(got.value, "STA 0+000.00", "fresh MTEXT value must survive reopen ({version:?})");
}

#[test]
fn fresh_mtext_value_survives_roundtrip_ac1018() {
    check_mtext(DxfVersion::AC1018);
}

#[test]
fn fresh_mtext_value_survives_roundtrip_ac1032() {
    check_mtext(DxfVersion::AC1032);
}
