//! Explain why each handle entered the dirty closure (Phase 9 debugging aid).
//! Usage: cargo run --example diag_dirty_closure -- FILE.dwg [HANDLE_HEX]
use acadrust::io::dwg::raw_passthrough as rp;
use acadrust::types::Handle;
use acadrust::DwgReader;
use std::collections::{HashSet, VecDeque};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: diag_dirty_closure FILE.dwg [HANDLE_HEX]");
    let seed_hex = args.next();
    let mut reader = DwgReader::from_file(&path).expect("open");
    let mut doc = reader.read().expect("read");
    if let Some(hex) = seed_hex {
        let value = u64::from_str_radix(hex.trim_start_matches("0x"), 16).expect("handle hex");
        doc.dirty_handles.insert(Handle::new(value));
    }
    let index = rp::HandleIndex::build(&doc);
    let graph = match rp::build_dependencies_with_index(&doc, &index) {
        Ok(g) => g,
        Err(e) => {
            println!("GRAPH ERROR {e}");
            return;
        }
    };
    // Reverse BFS: which referrer became dirty because of which target.
    let mut parent: std::collections::HashMap<Handle, (Handle, rp::EdgeKind)> = Default::default();
    let mut queue: VecDeque<Handle> = doc.dirty_handles.iter().copied().collect();
    let mut dirty: HashSet<Handle> = doc.dirty_handles.iter().copied().collect();
    while let Some(current) = queue.pop_front() {
        for (referrer, targets) in &graph.edges {
            if dirty.contains(referrer) {
                continue;
            }
            for (target, kind) in targets {
                if *target == current && *kind != rp::EdgeKind::Ownership {
                    dirty.insert(*referrer);
                    parent.insert(*referrer, (current, *kind));
                    queue.push_back(*referrer);
                    break;
                }
            }
        }
    }
    if std::env::var_os("PLAN_DUMP").is_some() {
        let plan = rp::plan(&doc).expect("plan");
        for hex in ["C31F2", "F100BFC"] {
            let h = Handle::new(u64::from_str_radix(hex, 16).unwrap());
            println!("PLAN {:X} {:?}", h.value(), plan.action(h));
        }
        println!("BLOCKED {:?}", plan.blocked);
        let f = Handle::new(0xF100BFC);
        println!(
            "IDX known={} raw={} raw_only={} deleted={} new={}",
            index.known.contains(&f), index.raw.contains(&f), index.raw_only.contains(&f),
            index.deleted.contains(&f), index.new.contains(&f)
        );
        println!("ENTITY {} OBJECTS {}", doc.get_entity(f).is_some(), doc.objects.contains_key(&f));
        println!("SOURCE_HAS {} RAW_HAS {}", doc.source_handles.contains(&f), doc.raw_object_records.contains_key(&f));
    }
    println!("SEED {:?}", doc.dirty_handles.iter().map(|h| h.value()).collect::<Vec<_>>());
    println!("DIRTY {}", dirty.len());
    let mut sorted: Vec<Handle> = dirty.iter().copied().collect();
    sorted.sort_by_key(|h| h.value());
    for h in sorted.iter().take(40) {
        match parent.get(h) {
            Some((via, kind)) => println!(
                "{:X} because {:?} via {:?}",
                h.value(),
                via.value(),
                kind
            ),
            None => println!("{:X} (seed)", h.value()),
        }
    }
}
