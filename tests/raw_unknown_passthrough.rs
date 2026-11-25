//! Phase 5 evidence: existing Unknown-entity raw passthrough is byte-identical
//! for the two documented handles in the generic AC1018 fixture.
//!
//! Set ACADIFC_FIXTURE=/path/to/dwg to run; otherwise the test is skipped with
//! a printed message (no external fixture is committed to the crate).
use acadrust::{DwgReader, DwgWriter, EntityType};
use std::io::Cursor;
use std::sync::Arc;

const HANDLES: [u64; 2] = [0xC0C6B6B, 0xC0C6B6C];

fn raw_map(doc: &acadrust::CadDocument) -> std::collections::BTreeMap<u64, Arc<Vec<u8>>> {
    doc.entities().filter_map(|e| match e {
        EntityType::Unknown(u) => Some((u.common.handle.value(), Arc::new(u.raw_dwg_data.clone().unwrap_or_default()))),
        _ => None,
    }).collect()
}

#[test]
fn unknown_raw_records_survive_noop_write() {
    let path = match std::env::var("ACADIFC_FIXTURE") {
        Ok(p) => p,
        Err(_) => { println!("SKIP: ACADIFC_FIXTURE not set"); return; }
    };
    let bytes = std::fs::read(&path).expect("read fixture");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    let before = r.read().expect("read");
    let raw_before = raw_map(&before);
    for h in HANDLES {
        assert!(raw_before.contains_key(&h), "handle {h:X} missing from fixture");
        assert!(!raw_before[&h].is_empty(), "handle {h:X} has no raw record");
    }
    let out = DwgWriter::write_to_vec(&before).expect("no-op write");
    let mut r2 = DwgReader::from_stream(Cursor::new(out));
    let after = r2.read().expect("read back");
    let raw_after = raw_map(&after);
    for h in HANDLES {
        assert_eq!(raw_before[&h], raw_after[&h], "raw record changed for {h:X}");
    }
}
