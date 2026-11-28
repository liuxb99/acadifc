//! AC1018 raw record passthrough MVP: untouched entities keep their original
//! merged-stream bytes; only the dirty TEXT/MTEXT is re-encoded.

use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType, Handle};
use std::io::Cursor;

fn fixture() -> Option<std::path::PathBuf> {
    std::env::var("ACADIFC_FIXTURE")
        .ok()
        .map(std::path::PathBuf::from)
        .filter(|path| path.exists())
}

#[test]
fn ac1018_text_update_preserves_untouched_raw_records() {
    let Some(path) = fixture() else {
        eprintln!("ACADIFC_FIXTURE not set; skipping generic raw passthrough regression");
        return;
    };
    let mut before = CadDocument::new();
    let mut reader = DwgReader::from_file(&path).expect("read fixture");
    before = reader.read().expect("decode fixture");
    assert_eq!(before.version, acadrust::DxfVersion::AC1018);
    assert!(!before.raw_object_records.is_empty());

    let target = before
        .entities()
        .find_map(|entity| match entity {
            EntityType::MText(text) if !text.value.is_empty() => Some(text.common.handle),
            _ => None,
        })
        .expect("fixture must contain an MTEXT");
    before.dirty_handles.insert(target);

    let bytes = DwgWriter::write_to_vec(&before).expect("write candidate");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let after: CadDocument = reader.read().expect("reopen candidate");

    let entity_handles: std::collections::HashSet<Handle> = before
        .entities()
        .filter(|entity| !matches!(entity,
            EntityType::Polyline2D(_) | EntityType::Polyline3D(_)
            | EntityType::PolyfaceMesh(_) | EntityType::PolygonMesh(_)))
        .filter(|entity| !matches!(entity, EntityType::Insert(i) if i.has_attributes()))
        .map(|entity| entity.common().handle)
        .collect();
    let mut compared = 0usize;
    for (handle, original) in &before.raw_object_records {
        if !entity_handles.contains(handle) {
            continue;
        }
        if before.dirty_handles.contains(handle) {
            continue;
        }
        let Some(written) = after.raw_object_records.get(handle) else {
            continue; // table records may not be captured identically
        };
        if original.bytes != written.bytes {
            println!(
                "RAW-DIFF handle={:?} before_len={} after_len={}",
                handle, original.bytes.len(), written.bytes.len()
            );
            panic!("untouched record {handle:?} changed");
        }
        compared += 1;
    }
    assert!(compared > 0, "no untouched records were compared");
    assert!(after.raw_object_records.contains_key(&Handle::from(target.value())));
}

// Phase 18 regression: a DOS-era code page id (24 = BIG5) normalises to the
// ANSI id 41, but both decode to the SAME legacy encoding, so a no-op save must
// NOT be treated as a cross-code-page transcode.  Before this fix, every
// AC1018 drawing whose header stored 24 failed with
// "raw-only object ... cannot be transcoded (source codepage 24 -> target codepage 41)".
#[test]
fn dos_big5_codepage_id_24_is_not_a_transcode() {
    use acadrust::io::dxf::code_page::EncodingContext;
    let mut doc = acadrust::CadDocument::default();
    doc.version = acadrust::types::DxfVersion::AC1018;
    doc.dwg_code_page_index = Some(24);
    doc.header.code_page = "BIG5".to_string();
    let ctx = EncodingContext::new(&doc);
    assert_eq!(ctx.codepage_id, 41, "cp24 must normalise to ANSI_950 (41)");
    assert_eq!(ctx.source_codepage_id, 24);
    assert!(!ctx.transcoding,
            "cp24 and cp41 are both BIG5: raw replay is byte-identical");
    assert_eq!(ctx.source_encoding.map(|e| e.name()),
               Some(ctx.target_encoding.name()));

    // A genuine change of encoding must still be flagged: an ANSI_1252 file
    // that actually contains Chinese must be promoted to ANSI_950 and its
    // raw records may not be copied verbatim.
    let mut doc2 = acadrust::CadDocument::default();
    doc2.version = acadrust::types::DxfVersion::AC1018;
    doc2.dwg_code_page_index = Some(30); // ANSI_1252
    doc2.header.code_page = "ANSI_1252".to_string();
    doc2.layers.add(acadrust::tables::Layer::new("縱斷面")).unwrap();
    let ctx2 = EncodingContext::new(&doc2);
    assert_eq!(ctx2.codepage_id, 41, "1252 + CJK content is promoted to BIG5");
    assert_eq!(ctx2.source_codepage_id, 30);
    assert!(ctx2.transcoding, "1252 -> 950 changes bytes and must transcode");
}

// Phase 18 regression: when a save transcodes to a different legacy encoding,
// every record the reader DECODED must be planned as a rewrite, not replayed.
// Before this, the plan still said "Replay" and try_replay_raw's transcode
// guard aborted the whole save on the first control record
// ("raw-only object handle=0x1 type=48 cannot be transcoded").
#[test]
fn transcoded_write_plans_decoded_records_as_rewrite() {
    use acadrust::io::dwg::raw_passthrough::{plan, RecordAction, RewriteReason};
    use acadrust::types::{DxfVersion, Handle};

    let mut doc = acadrust::CadDocument::new();
    doc.version = DxfVersion::AC1018;
    doc.dwg_source_version = Some(DxfVersion::AC1018);
    doc.dwg_code_page_index = Some(30); // ANSI_1252 source
    doc.header.code_page = "ANSI_1252".to_string();

    // CJK content forces the target code page to ANSI_950 (41) => transcoding.
    let mut layer = acadrust::tables::Layer::new("縱斷面");
    layer.handle = Handle::new(0x20);
    doc.layers.add(layer).unwrap();

    let handle = Handle::new(0x20);
    doc.raw_object_records.insert(
        handle,
        acadrust::document::RawObjectRecord {
            bytes: vec![0u8; 4],
            handle_bits: 0,
            version: DxfVersion::AC1018,
            type_code: 51, // LAYER: decodable, so it must be re-encoded
        },
    );

    let p = plan(&doc).expect("writer plan");
    match p.action(handle) {
        Some(RecordAction::Rewrite(RewriteReason::EncodingChanged)) => {}
        other => panic!("decoded record must be rewritten on transcode, got {other:?}"),
    }
}
