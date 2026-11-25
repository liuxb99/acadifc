//! Stage A/B/C/D diagnosis for Linetype.xref_dependent round-trip loss.
//! A = read original into CadDocument; B = clone; C = no mutation;
//! D = DwgWriter::write_to_vec then DwgReader re-read.
//! Compare Linetype table entries field-by-field at each stage.

use acadrust::io::dwg::{DwgReader, DwgWriter};
use acadrust::CadDocument;
use std::collections::BTreeMap;
use std::io::Cursor;

type Snap = BTreeMap<String, (bool, String, f64, char, usize)>;

fn snap(doc: &CadDocument) -> Snap {
    doc.line_types
        .iter()
        .map(|lt| {
            (
                lt.name.clone(),
                (
                    lt.xref_dependent,
                    lt.description.clone(),
                    lt.pattern_length,
                    lt.alignment,
                    lt.elements.len(),
                ),
            )
        })
        .collect()
}

fn diff(label: &str, a: &Snap, b: &Snap) {
    let mut changed = 0;
    for (name, av) in a {
        if let Some(bv) = b.get(name) {
            if av != bv {
                changed += 1;
                if changed <= 12 {
                    println!(
                        "  DIFF {}: {:?} -> {:?}",
                        name, av, bv
                    );
                }
            }
        } else {
            println!("  MISSING in {}: {}", label, name);
        }
    }
    println!("{}: {} linetype(s) changed", label, changed);
}

fn count_xref(s: &Snap) -> usize {
    s.values().filter(|(x, ..)| *x).count()
}

fn main() {
    let path = "/root/workspace/ai-media/drive/dwg/路線平縱斷面圖.dwg";
    let mut reader = DwgReader::from_file(path).expect("open fixture");
    let doc_a = reader.read().expect("read fixture");
    println!("source version: {:?}", doc_a.version);

    // Stage A
    let a = snap(&doc_a);
    println!(
        "A (read original): {} linetypes, {} xref_dependent=true",
        a.len(),
        count_xref(&a)
    );

    // Stage B: clone
    let doc_b = doc_a.clone();
    let b = snap(&doc_b);
    println!(
        "B (clone):        {} linetypes, {} xref_dependent=true",
        b.len(),
        count_xref(&b)
    );
    diff("A vs B", &a, &b);

    // Stage C: no mutation (clone again, verify in-memory stable)
    let doc_c = doc_b.clone();
    let c = snap(&doc_c);
    println!(
        "C (no mutation):  {} linetypes, {} xref_dependent=true",
        c.len(),
        count_xref(&c)
    );
    diff("B vs C", &b, &c);

    // Stage D: write then re-read
    let bytes = DwgWriter::write_to_vec(&doc_c).expect("write vec");
    println!("written DWG bytes: {}", bytes.len());
    let mut reread = DwgReader::from_stream(Cursor::new(bytes)).read().expect("re-read");
    let d = snap(&reread);
    println!(
        "D (write+reread): {} linetypes, {} xref_dependent=true",
        d.len(),
        count_xref(&d)
    );
    diff("A vs D", &a, &d);

    // Show the specific fixture linetypes named map|...
    println!("--- map| linetypes A -> D ---");
    for (name, av) in &a {
        if name.starts_with("map|") {
            let dv = d.get(name);
            println!(
                "  {}: A.xref={} D.xref={:?}",
                name,
                av.0,
                dv.map(|v| v.0)
            );
        }
    }

    // Keep reread alive/used to avoid unused warnings changing control flow.
    let _ = &mut reread;
}
