//! Dump header variables for A/B comparison (Phase 14 diagnostics).
use acadrust::DwgReader;
fn main() {
    let path = std::env::args().nth(1).expect("usage: p14_header_dump FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");
    println!("{:#?}", doc.header);
    println!("version = {}", doc.version.as_str());
}
