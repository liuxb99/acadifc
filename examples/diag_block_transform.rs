//! Diagnostic: BLOCK record base point / handles / owner lists + INSERT transform.
//! Usage: diag_block_transform <file.dwg> [stage-tag]
use acadrust::{DwgReader, EntityType};
use std::io::Cursor;

fn f(v: f64) -> String { format!("{:.12}", v) }
fn v3(v: &acadrust::types::Vector3) -> String { format!("{:.12},{:.12},{:.12}", v.x, v.y, v.z) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: diag_block_transform <file.dwg> [tag]");
    let tag = args.get(2).cloned().unwrap_or_else(|| "X".into());
    let bytes = std::fs::read(path).expect("read");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    let doc = r.read().expect("read dwg");
    for br in doc.block_records.iter() {
        println!("{}", [
            "BLOCK", &tag, &br.handle.value().to_string(), &br.name,
            &v3(&br.base_point),
            &br.block_entity_handle.value().to_string(),
            &br.block_end_handle.value().to_string(),
            &br.entity_handles.iter().map(|h| h.value().to_string()).collect::<Vec<_>>().join(","),
            &br.insert_handles.iter().map(|h| h.value().to_string()).collect::<Vec<_>>().join(","),
            &format!("{:?}", br.flags),
        ].join("\t"));
    }
    for e in doc.entities() {
        if let EntityType::Insert(i) = e {
            println!("{}", [
                "INSERT", &tag, &i.common.handle.value().to_string(), &i.block_name,
                &v3(&i.insert_point), &f(i.rotation), &v3(&i.normal),
                &f(i.x_scale()), &f(i.y_scale()), &f(i.z_scale()),
                &i.column_count.to_string(), &i.row_count.to_string(),
            ].join("\t"));
        }
    }
}
