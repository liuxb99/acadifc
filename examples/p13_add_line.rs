//! Debug: read a real DWG, add one LINE to model space, write, and report the
//! model-space block record before/after.
use acadrust::types::{Handle, Vector3};
fn main() {
    let path = std::env::args().nth(1).expect("usage: p13_add_line FILE OUT");
    let out = std::env::args().nth(2).expect("out");
    let mut reader = acadrust::DwgReader::from_file(&path).expect("open");
    let mut doc = reader.read().expect("read");
    let ms = doc
        .block_records
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case("*Model_Space"))
        .expect("model space")
        .clone();
    println!(
        "BEFORE ms handle={:X} block={:X} end={:X} members={}",
        ms.handle.value(),
        ms.block_entity_handle.value(),
        ms.block_end_handle.value(),
        ms.entity_handles.len()
    );
    let mut line = acadrust::entities::Line::new();
    line.common.layer = "0".to_string();
    line.start = Vector3::new(900000.0, 900000.0, 0.0);
    line.end = Vector3::new(901000.0, 900000.0, 0.0);
    line.common.handle = doc.allocate_handle();
    line.common.owner_handle = ms.handle;
    doc.add_entity(acadrust::EntityType::Line(line)).expect("add");
    let bytes = acadrust::DwgWriter::write_to_vec(&doc).expect("write");
    std::fs::write(&out, &bytes).expect("save");
    let mut reader = acadrust::DwgReader::from_file(&out).expect("reopen");
    let back = reader.read().expect("re-read");
    let ms = back
        .block_records
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case("*Model_Space"))
        .expect("model space after");
    println!(
        "AFTER ms handle={:X} block={:X} end={:X} members={}",
        ms.handle.value(),
        ms.block_entity_handle.value(),
        ms.block_end_handle.value(),
        ms.entity_handles.len()
    );
    if let Some(report) = acadrust::take_writer_report() {
        println!("report replayed={} rewritten={} blocked={:?}", report.replayed, report.rewritten, report.blocked);
    }
}
