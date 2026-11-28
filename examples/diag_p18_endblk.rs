//! Phase 18 diagnostic: block record / marker inventory before and after a
//! no-op write, to locate phantom BLOCK/ENDBLK records.
use acadrust::{DwgReader, DwgWriter};
use acadrust::entities::EntityType;

fn dump(tag: &str, doc: &acadrust::CadDocument) {
    println!("== {tag}: block_records={} entities={}", doc.block_records.iter().count(), doc.entities().count());
    let mut ends = 0usize;
    for br in doc.block_records.iter() {
        println!("  BR name={:24} handle={:X} block={:X} end={:X} members={}",
                 br.name, br.handle.value(), br.block_entity_handle.value(),
                 br.block_end_handle.value(), br.entity_handles.len());
        if let Some(e) = doc.get_entity(br.block_end_handle) {
            println!("     end entity: {:?} handle={:X} owner={:X}",
                     matches!(e, EntityType::BlockEnd(_)), e.common().handle.value(),
                     e.common().owner_handle.value());
        } else {
            println!("     end entity: MISSING from entity_index (handle {:X})", br.block_end_handle.value());
            who(doc, br.block_end_handle);
        }
    }
    for e in doc.entities() {
        if matches!(e, EntityType::BlockEnd(_)) { ends += 1; }
    }
    println!("  BlockEnd entities in stream: {ends}");
}

fn who(doc: &acadrust::CadDocument, h: acadrust::types::Handle) {
    let v = h.value();
    for l in doc.layers.iter() { if l.handle.value() == v { println!("     handle {:X} = LAYER {:?}", v, l.name); } }
    for t in doc.text_styles.iter() { if t.handle.value() == v { println!("     handle {:X} = STYLE {:?}", v, t.name); } }
    for t in doc.line_types.iter() { if t.handle.value() == v { println!("     handle {:X} = LTYPE {:?}", v, t.name); } }
    for t in doc.dim_styles.iter() { if t.handle.value() == v { println!("     handle {:X} = DIMSTYLE {:?}", v, t.name); } }
    for t in doc.app_ids.iter() { if t.handle.value() == v { println!("     handle {:X} = APPID {:?}", v, t.name); } }
    for t in doc.vports.iter() { if t.handle.value() == v { println!("     handle {:X} = VPORT {:?}", v, t.name); } }
    if let Some(e) = doc.get_entity(h) { println!("     handle {:X} = ENTITY {:?}", v, e); }
    if let Some(o) = doc.objects.get(&h) { println!("     handle {:X} = OBJECT {:?}", v, std::mem::discriminant(o)); }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: IN.dwg [OUT.dwg]");
    let output = args.next();
    let mut reader = DwgReader::from_file(&input).expect("open");
    let doc = reader.read().expect("read");
    dump("read", &doc);
    println!("-- probe handlesseed 0x1D:");
    who(&doc, acadrust::types::Handle::new(0x1D));
    if let Some(out) = output {
        DwgWriter::write_to_file(&out, &doc).expect("write");
        let mut r2 = DwgReader::from_file(&out).expect("reopen");
        let d2 = r2.read().expect("reread");
        dump("reread", &d2);
    }
}
