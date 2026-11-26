//! Write a true read -> write no-op DWG (no verification bypass in production;
//! developer diagnostic for field/record comparison).
use acadrust::{DwgReader, DwgWriter};
fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: noop_write IN.dwg OUT.dwg");
    let output = args.next().expect("usage: noop_write IN.dwg OUT.dwg");
    let mut reader = DwgReader::from_file(&input).expect("open");
    let doc = reader.read().expect("read");
    DwgWriter::write_to_file(&output, &doc).expect("write");
    if let Some(report) = acadrust::take_writer_report() {
        println!(
            "writer replayed={} rewritten={} new={} deleted={} omitted={} errors={} blocked={}",
            report.replayed,
            report.rewritten,
            report.new_objects,
            report.deleted,
            report.omitted.len(),
            report.errors.len(),
            report.blocked.len()
        );
        for o in report.omitted.iter().take(10) {
            println!("OMITTED {o}");
        }
        for b in report.blocked.iter().take(10) {
            println!("BLOCKED {b}");
        }
    }
    println!("wrote {output}");
}
