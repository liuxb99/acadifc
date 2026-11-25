//! Diagnostic: ATTRIB / ATTDEF geometry + owner chain + INSERT/SEQEND handles.
//! Usage: diag_attrib_owner <file.dwg> [stage-tag]
use acadrust::{DwgReader, EntityType};
use std::io::Cursor;

fn f(v: f64) -> String { format!("{:.12}", v) }
fn v3(v: &acadrust::types::Vector3) -> String { format!("{:.12},{:.12},{:.12}", v.x, v.y, v.z) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: diag_attrib_owner <file.dwg> [tag]");
    let tag = args.get(2).cloned().unwrap_or_else(|| "X".into());
    let bytes = std::fs::read(path).expect("read");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    let doc = r.read().expect("read dwg");
    for e in doc.entities() {
        match e {
            EntityType::AttributeEntity(a) => println!("{}", [
                "ATTRIB", &tag, &a.common.handle.value().to_string(),
                &a.common.owner_handle.value().to_string(), &a.common.layer, &a.tag,
                &v3(&a.insertion_point), &v3(&a.alignment_point), &f(a.height), &f(a.rotation),
                &f(a.width_factor), &f(a.oblique_angle), &v3(&a.normal),
                &format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment),
                &a.value].join("\t")),
            EntityType::AttributeDefinition(a) => println!("{}", [
                "ATTDEF", &tag, &a.common.handle.value().to_string(),
                &a.common.owner_handle.value().to_string(), &a.common.layer, &a.tag,
                &v3(&a.insertion_point), &v3(&a.alignment_point), &f(a.height), &f(a.rotation),
                &f(a.width_factor), &f(a.oblique_angle), &v3(&a.normal),
                &format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment),
                &a.default_value].join("\t")),
            EntityType::Insert(i) => {
                println!("{}", [
                    "INSERT", &tag, &i.common.handle.value().to_string(),
                    &i.common.owner_handle.value().to_string(), &i.common.layer, &i.block_name,
                    &v3(&i.insert_point), &f(i.rotation), &f(i.x_scale()), &f(i.y_scale()), &f(i.z_scale()),
                    &i.seqend_handle.map(|h| h.value().to_string()).unwrap_or_else(|| "-".into()),
                    &i.attributes.iter().map(|a| a.common.handle.value().to_string()).collect::<Vec<_>>().join(",")
                ].join("\t"));
                for a in &i.attributes {
                    println!("{}", [
                        "ATTRIB-CHILD", &tag, &a.common.handle.value().to_string(),
                        &i.common.handle.value().to_string(), &a.common.layer, &a.tag,
                        &v3(&a.insertion_point), &v3(&a.alignment_point), &f(a.height), &f(a.rotation),
                        &f(a.width_factor), &f(a.oblique_angle), &v3(&a.normal),
                        &format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment),
                        &a.value].join("\t"));
                }
            },
            _ => {}
        }
    }
}
