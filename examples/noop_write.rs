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
    println!("wrote {output}");
}
