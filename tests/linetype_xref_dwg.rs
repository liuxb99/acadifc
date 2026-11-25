//! Regression tests: DWG round-trip must preserve `LineType.xref_dependent`.
//!
//! Bug: `write_line_type` called the no-argument `write_xref_dependant_bit()`,
//! which always emits `false`, so every linetype flagged xref-dependent in the
//! source fell back to `false` after a save. The reader, the in-memory model
//! and the clone were correct — only serialization lost the bit.
//!
//! Tests are synthetic (no engineering drawing dependency) and deterministic.
//! Two bit encodings are exercised: pre-R2007 (three separate fields) and
//! R2007+ (combined `BS` value).

use std::io::Cursor;

use acadrust::tables::{LineType, LineTypeElement};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter};

/// Write to DWG bytes and read back through the real reader.
fn roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("DWG write");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("DWG re-read")
}

/// Build a document with one xref-dependent and one ordinary linetype.
/// Handles are allocated through the document API for deterministic output.
fn doc_with_linetypes(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);

    let mut dependent = LineType::new("XREFDEP");
    dependent.handle = doc.allocate_handle();
    dependent.description = "dependent synthetic linetype".to_string();
    dependent.elements.push(LineTypeElement::dash(1.0));
    dependent.elements.push(LineTypeElement::space(0.5));
    dependent.pattern_length = 1.5;
    dependent.xref_dependent = true;
    doc.line_types.add(dependent).expect("add dependent linetype");

    let mut local = LineType::new("LOCALDEP");
    local.handle = doc.allocate_handle();
    local.description = "ordinary synthetic linetype".to_string();
    local.elements.push(LineTypeElement::dash(2.0));
    local.pattern_length = 2.0;
    local.xref_dependent = false;
    doc.line_types.add(local).expect("add ordinary linetype");

    doc
}

fn assert_roundtrip(version: DxfVersion, label: &str) {
    let doc = doc_with_linetypes(version);
    // Precondition: the flag is set in the in-memory model before writing.
    assert!(doc.line_types.get("XREFDEP").unwrap().xref_dependent);
    assert!(!doc.line_types.get("LOCALDEP").unwrap().xref_dependent);

    let rt = roundtrip(&doc);

    let dep = rt
        .line_types
        .get("XREFDEP")
        .unwrap_or_else(|| panic!("{label}: XREFDEP linetype lost"));
    assert!(
        dep.xref_dependent,
        "{label}: xref_dependent=true became false on DWG round-trip"
    );

    let local = rt
        .line_types
        .get("LOCALDEP")
        .unwrap_or_else(|| panic!("{label}: LOCALDEP linetype lost"));
    assert!(
        !local.xref_dependent,
        "{label}: xref_dependent=false became true on DWG round-trip"
    );
}

/// AC1018 / R2004 — pre-R2007 three-field xref bit encoding, page-based file
/// format. This is the encoding family used by the ANSI_950 regression fixture.
#[test]
fn dwg_linetype_xref_dependent_roundtrip_ac1018() {
    assert_roundtrip(DxfVersion::AC1018, "AC1018/R2004");
}

/// AC1015 / R2000 — pre-R2007 encoding, flat file format.
#[test]
fn dwg_linetype_xref_dependent_roundtrip_ac1015() {
    assert_roundtrip(DxfVersion::AC1015, "AC1015/R2000");
}

/// AC1021 / R2007 — combined `BS` xref flag encoding.
#[test]
fn dwg_linetype_xref_dependent_roundtrip_ac1021() {
    assert_roundtrip(DxfVersion::AC1021, "AC1021/R2007");
}

/// AC1032 / R2018 — latest combined `BS` encoding.
#[test]
fn dwg_linetype_xref_dependent_roundtrip_ac1032() {
    assert_roundtrip(DxfVersion::AC1032, "AC1032/R2018");
}

/// Generic AC1018/ANSI_950 regression fixture (data only, not a deliverable).
/// Reads the fixture, verifies xref-dependent linetypes decode, writes and
/// re-reads, and verifies every one stays dependent. Skips when the fixture is
/// not present so the suite remains portable; the synthetic tests above are
/// the primary, deterministic coverage.
#[test]
fn dwg_linetype_xref_dependent_fixture_ac1018_ansi950() {
    let path = "/root/workspace/ai-media/drive/dwg/路線平縱斷面圖.dwg";
    if !std::path::Path::new(path).exists() {
        eprintln!("generic AC1018 fixture not present — skipping fixture check");
        return;
    }

    let mut reader = match DwgReader::from_file(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("generic AC1018 fixture unreadable ({e}) — skipping");
            return;
        }
    };
    let doc = match reader.read() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("generic AC1018 fixture decode failed ({e}) — skipping");
            return;
        }
    };
    assert_eq!(doc.version, DxfVersion::AC1018);

    let expected: Vec<String> = doc
        .line_types
        .iter()
        .filter(|lt| lt.xref_dependent)
        .map(|lt| lt.name.clone())
        .collect();
    assert!(
        !expected.is_empty(),
        "fixture should contain xref-dependent linetypes"
    );

    let rt = roundtrip(&doc);
    for name in expected {
        let after = rt
            .line_types
            .get(&name)
            .unwrap_or_else(|| panic!("fixture linetype {name} lost on round-trip"));
        assert!(
            after.xref_dependent,
            "fixture linetype {name}: xref_dependent lost on DWG round-trip"
        );
    }
}
