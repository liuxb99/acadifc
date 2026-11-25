//! A/B/C/D diagnostic for TEXT/MTEXT/ATTRIB geometry across a native DWG round trip.
//!
//! Usage: diag_text_fidelity <input.dwg> [noop|patch] [output.dwg]
//! Prints one TSV row per text entity for stage B (acadrust read) and C
//! (clone / optional tiny MTEXT patch). Independent-reader A/D uses ezdwg.
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::io::Cursor;

fn f(v: f64) -> String { format!("{:.12}", v) }
fn v3(v: &acadrust::types::Vector3) -> String { format!("{:.12},{:.12},{:.12}", v.x, v.y, v.z) }

fn dump(stage: &str, doc: &CadDocument) {
    println!("#stage\t{}\tversion\t{:?}\tentities\t{}", stage, doc.version, doc.entities().count());
    for s in doc.text_styles.iter() {
        println!("STYLE\t{}\t{}\t{}\t{}\t{}\t{}", stage, s.handle.value(), s.name,
                 s.font_file, f(s.height), s.width_factor);
    }
    for e in doc.entities() {
        match e {
            EntityType::Text(t) => println!("{}", ["TEXT", stage, &t.common.handle.value().to_string(), &t.common.owner_handle.value().to_string(), &t.common.layer,
                 &v3(&t.insertion_point),
                 &t.alignment_point.as_ref().map(v3).unwrap_or_else(|| "-".into()),
                 &f(t.height), &f(t.width_factor), &f(t.rotation), &f(t.oblique_angle),
                 &v3(&t.normal), &t.style,
                 &format!("{:?}", t.horizontal_alignment), &format!("{:?}", t.vertical_alignment),
                 &f(t.thickness), &t.generation_flags.to_string(),
                 &t.value.replace('\t', " ").replace('\n', "\\P")].join("\t")
            ),
            EntityType::MText(m) => println!("{}", ["MTEXT", stage, &m.common.handle.value().to_string(), &m.common.owner_handle.value().to_string(), &m.common.layer,
                 &v3(&m.insertion_point), &f(m.height), &f(m.rectangle_width),
                 &m.rectangle_height.map(f).unwrap_or_else(|| "-".into()),
                 &f(m.rotation), &v3(&m.normal),
                 &format!("{:?}", m.attachment_point), &format!("{:?}", m.drawing_direction),
                 &f(m.line_spacing_factor), &format!("{:?}", m.line_spacing_style),
                 &f(m.extents_width), &f(m.extents_height), &f(m.background_scale), &m.style,
                 &m.value.replace('\t', " ").replace('\n', "\\P")].join("\t")
            ),
            EntityType::AttributeDefinition(a) => println!("{}", ["ATTDEF", stage, &a.common.handle.value().to_string(), &a.common.owner_handle.value().to_string(), &a.common.layer,
                 &v3(&a.insertion_point), &v3(&a.alignment_point), &f(a.height), &f(a.rotation),
                 &f(a.width_factor), &f(a.oblique_angle), &v3(&a.normal), &a.tag,
                 &format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment),
                 &a.default_value].join("\t")
            ),
            EntityType::AttributeEntity(a) => println!("{}", ["ATTRIB", stage, &a.common.handle.value().to_string(), &a.common.owner_handle.value().to_string(), &a.common.layer,
                 &v3(&a.insertion_point), &v3(&a.alignment_point), &f(a.height), &f(a.rotation),
                 &f(a.width_factor), &f(a.oblique_angle), &v3(&a.normal), &a.tag,
                 &format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment),
                 &a.value].join("\t")
            ),
            EntityType::Insert(i) => println!("{}", ["INSERT", stage, &i.common.handle.value().to_string(), &i.common.owner_handle.value().to_string(), &i.common.layer,
                 &i.block_name, &v3(&i.insert_point), &f(i.rotation), &v3(&i.normal),
                 &f(i.x_scale()), &f(i.y_scale()), &f(i.z_scale()),
                 &i.column_count.to_string(), &i.row_count.to_string(),
                 &i.seqend_handle.map(|h| h.value().to_string()).unwrap_or_else(|| "-".into()),
                 &i.attributes.len().to_string()].join("\t")
            ),
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: diag_text_fidelity <input.dwg> [noop|patch] [output.dwg]");
    let mode = args.get(2).map(String::as_str).unwrap_or("noop");
    let bytes = std::fs::read(path).expect("read input");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let b = reader.read().expect("stage B read");
    dump("B", &b);

    let mut c = b.clone();
    if mode == "patch" {
        let target = args.get(4).and_then(|s| {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                s.parse::<u64>().ok()
            }
        });
        let mut patched = 0;
        for e in c.entities_mut() {
            match e {
                EntityType::MText(m) => {
                    let hit = match target {
                        Some(h) => m.common.handle.value() == h,
                        None => m.value.len() < 16 && !m.value.is_empty(),
                    };
                    if hit {
                        m.value.push_str(" [diag]");
                        patched += 1;
                        if target.is_some() { break; }
                    }
                }
                EntityType::Text(t) if target.is_some() => {
                    if t.common.handle.value() == target.unwrap() {
                        t.value.push_str(" [diag]");
                        patched += 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        eprintln!("patched MTEXT count={}", patched);
    }
    dump("C", &c);

    if let Some(out) = args.get(3) {
        DwgWriter::write_to_file(out, &c).expect("stage C write");
        eprintln!("wrote {}", out);
    }
}
