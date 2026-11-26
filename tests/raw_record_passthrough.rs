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
