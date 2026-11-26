//! Audit DWG read coverage: handle map vs captured raw records.
//! Usage: cargo run --example diag_read_audit -- FILE.dwg
use acadrust::DwgReader;

fn main() {
    let path = std::env::args().nth(1).expect("usage: diag_read_audit FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");
    if let Some(defaults) = doc.read_audit.as_ref() {
        println!("AUDIT handle_map_total={}", defaults.handle_map_total);
        println!("AUDIT records_read={}", defaults.records_read);
        println!("AUDIT raw_preserved={}", defaults.raw_records_preserved);
        println!("AUDIT failures={}", defaults.failures.len());
        for f in defaults.failures.iter().take(10) {
            println!(
                "FAIL handle={:X} offset={} type={} reason={}",
                f.handle, f.offset, f.raw_type_code, f.reason
            );
        }
        for m in defaults.unreadable.iter().take(10) {
            println!("UNREADABLE handle={:X} reason={}", m.handle, m.reason);
        }
    } else {
        println!("AUDIT none");
    }
    println!("ENTITY {}", doc.entity_count());
    println!("OBJECTS {}", doc.objects.len());
    println!("RAWREC {}", doc.raw_object_records.len());
    println!("BLOCKREC {}", doc.block_records.len());
}
