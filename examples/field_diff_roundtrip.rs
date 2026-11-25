//! P2 diagnostic: field-level comparison of TEXT/MTEXT/ATTRIB/ATTDEF across a
//! genuine no-op DWG round trip (read -> write -> read; never a file copy).
//!
//! Usage: field_diff_roundtrip <input.dwg> [written.dwg] > fields.tsv
//! Dumps stage B and stage D with the same field set so `diff -u` or the
//! companion Python script can attribute a change to a specific field.
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};
use std::io::Cursor;

fn f(v: f64) -> String { format!("{:.12}", v) }
fn v3(v: &acadrust::types::Vector3) -> String { format!("{:.12},{:.12},{:.12}", v.x, v.y, v.z) }
fn oh(h: &Option<acadrust::Handle>) -> String {
    h.map(|x| x.value().to_string()).unwrap_or_else(|| "-".into())
}
fn common(_doc: &CadDocument, c: &acadrust::entities::EntityCommon) -> Vec<String> {
    let h = c.handle.value();
    vec![
        h.to_string(), c.owner_handle.value().to_string(), c.layer.clone(),
        format!("{:?}", c.color), format!("{:?}", c.line_weight), c.linetype.clone(),
        oh(&c.linetype_handle), f(c.linetype_scale), format!("{:?}", c.transparency),
        c.invisible.to_string(),
        c.reactors.iter().map(|h| h.value().to_string()).collect::<Vec<_>>().join(","),
        oh(&c.xdictionary_handle),
        format!("eed:{}", c.extended_data.raw_dwg_eed.iter().map(|(_, v)| v.len()).sum::<usize>()),
        format!("xdata:{}", c.extended_data.len()),
        format!("graphic:{}", c.graphic_data.as_ref().map(|v| v.len()).unwrap_or(0)),
        format!("{:?}", c.entity_mode), c.material_flags.to_string(), oh(&c.material_handle),
        c.shadow_flags.to_string(), c.plotstyle_flags.to_string(), oh(&c.plotstyle_handle),
        oh(&c.color_book_handle), oh(&c.full_visual_style_handle),
        oh(&c.face_visual_style_handle), oh(&c.edge_visual_style_handle),
    ]
}

fn dump(stage: &str, doc: &CadDocument) {
    println!("#stage\t{}", stage);
    for s in doc.text_styles.iter() {
        println!("STYLE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", stage, s.handle.value(),
                 s.name, s.font_file, s.big_font_file, s.true_type_font,
                 f(s.height), f(s.width_factor));
    }
    for e in doc.entities() {
        match e {
            EntityType::Text(t) => {
                let mut r = vec!["TEXT".to_string(), stage.to_string()];
                r.extend(common(doc, &t.common));
                r.extend([v3(&t.insertion_point),
                          t.alignment_point.as_ref().map(v3).unwrap_or_else(|| "-".into()),
                          f(t.height), f(t.width_factor), f(t.rotation), f(t.oblique_angle),
                          v3(&t.normal), f(t.thickness), t.style.clone(),
                          format!("{:?}", t.horizontal_alignment),
                          format!("{:?}", t.vertical_alignment),
                          t.generation_flags.to_string(), t.value.clone()]);
                println!("{}", r.join("\t"));
            }
            EntityType::MText(m) => {
                let mut r = vec!["MTEXT".to_string(), stage.to_string()];
                r.extend(common(doc, &m.common));
                r.extend([v3(&m.insertion_point), v3(&m.normal),
                          m.dwg_x_direction.as_ref().map(v3).unwrap_or_else(|| "-".into()),
                          f(m.height), f(m.rectangle_width),
                          m.rectangle_height.map(f).unwrap_or_else(|| "-".into()),
                          f(m.rotation), format!("{:?}", m.attachment_point),
                          format!("{:?}", m.drawing_direction), f(m.line_spacing_factor),
                          format!("{:?}", m.line_spacing_style), f(m.extents_width),
                          f(m.extents_height), m.background_fill_flags.to_string(),
                          f(m.background_scale), format!("{:?}", m.background_color),
                          m.background_transparency.to_string(), m.is_annotative.to_string(),
                          m.style.clone(), m.value.clone()]);
                println!("{}", r.join("\t"));
            }
            EntityType::AttributeDefinition(a) => {
                let mut r = vec!["ATTDEF".to_string(), stage.to_string()];
                r.extend(common(doc, &a.common));
                r.extend([v3(&a.insertion_point), v3(&a.alignment_point), f(a.height),
                          f(a.rotation), f(a.width_factor), f(a.oblique_angle), v3(&a.normal),
                          a.text_style.clone(), format!("{:?}", a.horizontal_alignment),
                          format!("{:?}", a.vertical_alignment), a.tag.clone(),
                          a.prompt.clone(), a.default_value.clone()]);
                println!("{}", r.join("\t"));
            }
            EntityType::AttributeEntity(a) => {
                let mut r = vec!["ATTRIB".to_string(), stage.to_string()];
                r.extend(common(doc, &a.common));
                r.extend([v3(&a.insertion_point), v3(&a.alignment_point), f(a.height),
                          f(a.rotation), f(a.width_factor), f(a.oblique_angle), v3(&a.normal),
                          a.text_style.clone(), format!("{:?}", a.horizontal_alignment),
                          format!("{:?}", a.vertical_alignment), a.tag.clone(), a.value.clone()]);
                println!("{}", r.join("\t"));
            }
            EntityType::Insert(i) => {
                for a in &i.attributes {
                    let mut r = vec!["ATTRIB".to_string(), stage.to_string()];
                    r.extend(common(doc, &a.common));
                    r.extend([v3(&a.insertion_point), v3(&a.alignment_point), f(a.height),
                              f(a.rotation), f(a.width_factor), f(a.oblique_angle), v3(&a.normal),
                              a.text_style.clone(), format!("{:?}", a.horizontal_alignment),
                              format!("{:?}", a.vertical_alignment), a.tag.clone(), a.value.clone()]);
                    println!("{}", r.join("\t"));
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: field_diff_roundtrip <input.dwg> [written.dwg]");
    let bytes = std::fs::read(path).expect("read input");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let b = reader.read().expect("stage B read");
    dump("B", &b);
    let out = DwgWriter::write_to_vec(&b).expect("no-op write");
    if let Some(p) = args.get(2) { std::fs::write(p, &out).expect("write candidate"); }
    let mut r2 = DwgReader::from_stream(Cursor::new(out));
    let d = r2.read().expect("stage D read");
    dump("D", &d);
}
