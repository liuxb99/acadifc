//! Diagnostic: round-trip a DWG and compare EntityType::Unknown layer/handle/owner/raw payload.
use acadrust::{DwgReader, DwgWriter, EntityType};
use std::collections::BTreeMap;
use std::io::Cursor;

type Key = (u64, String, u64, usize);

fn collect(bytes: &[u8]) -> BTreeMap<u64, Key> {
    let mut r = DwgReader::from_stream(Cursor::new(bytes.to_vec()));
    let doc = r.read().expect("read");
    let mut m = BTreeMap::new();
    for e in doc.entities() {
        if let EntityType::Unknown(u) = e {
            m.insert(u.common.handle.value(), (
                u.common.handle.value(),
                u.common.layer.clone(),
                u.common.owner_handle.value(),
                u.raw_dwg_data.as_ref().map(|v| v.len()).unwrap_or(0),
            ));
        }
    }
    m
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: roundtrip_unknown <file.dwg>");
    let bytes = std::fs::read(&path).expect("read file");
    let before = collect(&bytes);
    println!("BEFORE count={} sample={:?}", before.len(), before.values().take(3).collect::<Vec<_>>());

    let mut r = DwgReader::from_stream(Cursor::new(bytes.clone()));
    let doc = r.read().expect("read src");
    let src_layers: Vec<(u64, String)> = doc.layers.iter().map(|l| (l.handle.value(), l.name.clone())).collect();
    println!("SRC_LAYERS sample={:?}", src_layers.iter().filter(|(_, n)| n.contains("華") || n.contains("&#")).take(6).collect::<Vec<_>>());
    let out = DwgWriter::write_to_vec(&doc).expect("write");
    println!("written bytes={}", out.len());

    let mut r2 = DwgReader::from_stream(Cursor::new(out.clone()));
    let doc2 = r2.read().expect("read back");
    let dst_layers: Vec<(u64, String)> = doc2.layers.iter().map(|l| (l.handle.value(), l.name.clone())).collect();
    println!("DST_LAYERS sample={:?}", dst_layers.iter().filter(|(_, n)| n.contains("華") || n.contains("&#")).take(6).collect::<Vec<_>>());
    println!("AFTER version={:?} entities={} entities_src={}", doc2.version, doc2.entities().count(), doc.entities().count());
    let mut n = 0;
    let mut changed = 0;
    for e in doc2.entities() {
        if let EntityType::Unknown(u) = e {
            n += 1;
            let h = u.common.handle.value();
            let after = (h, u.common.layer.clone(), u.common.owner_handle.value(),
                         u.raw_dwg_data.as_ref().map(|v| v.len()).unwrap_or(0));
            match before.get(&h) {
                None => { if changed < 10 { println!("NEW handle={} after={:?}", h, after); } changed += 1; }
                Some(b) if *b != after => {
                    if changed < 10 { println!("CHANGED handle={} before={:?} after={:?}", h, b, after); }
                    changed += 1;
                }
                _ => {}
            }
        }
    }
    println!("AFTER_UNKNOWN_COUNT={} CHANGED={} BEFORE_COUNT={}", n, changed, before.len());
}
