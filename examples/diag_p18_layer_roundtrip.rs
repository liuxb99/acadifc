//! Phase 18 diagnostic: does a layer name survive a no-op save of a REAL file?
use acadrust::{DwgReader, DwgWriter};

fn show(tag: &str, doc: &acadrust::CadDocument) {
    for l in doc.layers.iter() {
        if l.name.contains('赻') || l.name.chars().any(|c| c as u32 > 0xF000) || l.name.contains("map") {
            println!("{tag}: layer {:?} codepoints={:?}", l.name,
                     l.name.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>());
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: IN.dwg [OUT.dwg]");
    let out = std::env::args().nth(2).unwrap_or("/tmp/p18_layertest.dwg".into());
    let mut r = DwgReader::from_file(&path).expect("open");
    let doc = r.read().expect("read");
    println!("source layers={}", doc.layers.iter().count());
    show("SRC", &doc);
    DwgWriter::write_to_file(&out, &doc).expect("write");
    let mut r2 = DwgReader::from_file(&out).expect("reopen");
    let d2 = r2.read().expect("reread");
    println!("output layers={}", d2.layers.iter().count());
    show("DST", &d2);
    // full name multiset diff
    use std::collections::BTreeSet;
    let a: BTreeSet<String> = doc.layers.iter().map(|l| l.name.clone()).collect();
    let b: BTreeSet<String> = d2.layers.iter().map(|l| l.name.clone()).collect();
    for n in a.difference(&b) { println!("ONLY-SRC {:?} {:?}", n, n.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>()); }
    for n in b.difference(&a) { println!("ONLY-DST {:?} {:?}", n, n.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>()); }
}
