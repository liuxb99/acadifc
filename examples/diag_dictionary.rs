//! Stage A/B/C/D diagnostic for DWG Dictionary round-trip fidelity.
//!
//! Usage: cargo run --example diag_dictionary -- <path/to/file.dwg>
//!
//! A = read original, B = clone, C = no mutation, D = write_to_vec + re-read.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use acadrust::objects::ObjectType;
use acadrust::types::Handle;
use acadrust::{CadDocument, DwgReader, DwgWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    dictionaries: BTreeMap<u64, DictSnap>,
    object_count: usize,
    object_handles: BTreeSet<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DictSnap {
    handle: u64,
    owner: u64,
    hard_owner: bool,
    duplicate_cloning: i16,
    xdictionary: u64,
    reactors: Vec<u64>,
    entries: Vec<(String, u64)>,
}

fn variant_name(obj: &ObjectType) -> String {
    let dbg = format!("{:?}", obj);
    dbg.split('(').next().unwrap_or(&dbg).to_string()
}

fn snapshot(doc: &CadDocument) -> Snapshot {
    let mut dictionaries = BTreeMap::new();
    for (h, obj) in &doc.objects {
        if let ObjectType::Dictionary(d) = obj {
            let snap = DictSnap {
                handle: h.value(),
                owner: d.owner.value(),
                hard_owner: d.hard_owner,
                duplicate_cloning: d.duplicate_cloning,
                xdictionary: d.xdictionary_handle.map(|x| x.value()).unwrap_or(0),
                reactors: d.reactors.iter().map(|r| r.value()).collect(),
                entries: d
                    .entries
                    .iter()
                    .map(|(n, eh)| (n.clone(), eh.value()))
                    .collect(),
            };
            dictionaries.insert(h.value(), snap);
        }
    }
    Snapshot {
        dictionaries,
        object_count: doc.objects.len(),
        object_handles: doc.objects.keys().map(|h| h.value()).collect(),
    }
}

fn print_snapshot(label: &str, doc: &CadDocument, snap: &Snapshot) {
    println!("========== {} ==========", label);
    println!(
        "version={:?} source={:?} objects={} dictionaries={}",
        doc.version,
        doc.dwg_source_version,
        snap.object_count,
        snap.dictionaries.len()
    );
    for (h, d) in &snap.dictionaries {
        println!(
            "  DICT handle={:#X}({}) owner={:#X} hard_owner={} dup_clone={} xdic={:#X} reactors={:?} entries={}",
            h,
            h,
            d.owner,
            d.hard_owner,
            d.duplicate_cloning,
            d.xdictionary,
            d.reactors,
            d.entries.len()
        );
        for (name, eh) in &d.entries {
            let target = doc.objects.get(&Handle::from(*eh));
            let tdesc = match target {
                Some(o) => format!("{}", variant_name(o)),
                None => "<MISSING>".to_string(),
            };
            println!("    {:<24} -> {:#X}({}) [{}]", name, eh, eh, tdesc);
        }
    }
    println!();
}

fn diff(label: &str, a: &Snapshot, b: &Snapshot) {
    println!("---- DIFF {} ----", label);
    println!(
        "object_count: {} -> {} ; object_handles added={} removed={}",
        a.object_count,
        b.object_count,
        b.object_handles.difference(&a.object_handles).count(),
        a.object_handles.difference(&b.object_handles).count()
    );
    let handles: BTreeSet<u64> = a
        .dictionaries
        .keys()
        .chain(b.dictionaries.keys())
        .copied()
        .collect();
    let mut n = 0;
    for h in handles {
        match (a.dictionaries.get(&h), b.dictionaries.get(&h)) {
            (Some(x), Some(y)) if x == y => {}
            (Some(x), Some(y)) => {
                n += 1;
                println!("  DICT {:#X} CHANGED:", h);
                if x.owner != y.owner {
                    println!("    owner {:#X} -> {:#X}", x.owner, y.owner);
                }
                if x.hard_owner != y.hard_owner {
                    println!("    hard_owner {} -> {}", x.hard_owner, y.hard_owner);
                }
                if x.duplicate_cloning != y.duplicate_cloning {
                    println!(
                        "    duplicate_cloning {} -> {}",
                        x.duplicate_cloning, y.duplicate_cloning
                    );
                }
                if x.xdictionary != y.xdictionary {
                    println!("    xdic {:#X} -> {:#X}", x.xdictionary, y.xdictionary);
                }
                let am: BTreeMap<&str, u64> =
                    x.entries.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                let bm: BTreeMap<&str, u64> =
                    y.entries.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                for (k, v) in &am {
                    match bm.get(k) {
                        None => println!("    ENTRY LOST: {:?} -> {:#X}", k, v),
                        Some(v2) if v2 != v => {
                            println!("    ENTRY REMAPPED: {:?} {:#X} -> {:#X}", k, v, v2)
                        }
                        _ => {}
                    }
                }
                for (k, v) in &bm {
                    if !am.contains_key(k) {
                        println!("    ENTRY ADDED: {:?} -> {:#X}", k, v);
                    }
                }
            }
            (Some(_), None) => {
                n += 1;
                println!("  DICT {:#X} LOST", h);
            }
            (None, Some(_)) => {
                n += 1;
                println!("  DICT {:#X} ADDED", h);
            }
            (None, None) => {}
        }
    }
    if n == 0 {
        println!("  all dictionaries identical");
    }
    println!();
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: diag_dictionary <file.dwg>");
    let mut reader = DwgReader::from_file(&path).expect("cannot open DWG");
    let a = reader.read().expect("cannot read DWG");

    let sa = snapshot(&a);
    print_snapshot("STAGE A (original read)", &a, &sa);

    let b = a.clone();
    let sb = snapshot(&b);
    print_snapshot("STAGE B (clone)", &b, &sb);
    diff("A -> B (clone)", &sa, &sb);

    // C = no mutation at all.
    let c = b.clone();
    let sc = snapshot(&c);
    print_snapshot("STAGE C (no mutation)", &c, &sc);
    diff("B -> C (no mutation)", &sb, &sc);

    let bytes = DwgWriter::write_to_vec(&c).expect("DWG write failed");
    println!("DWG write produced {} bytes", bytes.len());
    let mut reread = DwgReader::from_stream(Cursor::new(bytes));
    let d = reread.read().expect("DWG re-read failed");
    let sd = snapshot(&d);
    print_snapshot("STAGE D (write + re-read)", &d, &sd);
    diff("C -> D (write + re-read)", &sc, &sd);
    diff("A -> D (overall)", &sa, &sd);

    // Print dictionary helper records that are visual-style-like, to see what
    // survived as objects even when entries disappeared.
    println!("========== VISUALSTYLE / MATERIAL OBJECTS ==========");
    for (h, obj) in &d.objects {
        let n = variant_name(obj);
        if n.contains("VisualStyle") || n.contains("Material") {
            println!("  D object {:#X}: {}", h, n);
        }
    }
    for (h, obj) in &a.objects {
        let n = variant_name(obj);
        if n.contains("VisualStyle") || n.contains("Material") {
            println!("  A object {:#X}: {}", h, n);
        }
    }
}
