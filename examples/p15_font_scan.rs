// Scan every raw object record for font-like strings and print the bytes
// around them (diagnostic for AutoCAD font-substitution crashes).
use acadrust::DwgReader;
fn main() {
    let path = std::env::args().nth(1).expect("usage: p15_font_scan FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");
    println!("code_page={:?} records={}", doc.header.code_page, doc.raw_object_records.len());
    for (handle, rec) in doc.raw_object_records.iter() {
        let b = &rec.bytes;
        for pat in [b"shx".as_slice(), b"ttf".as_slice(), b"SHX".as_slice()] {
            let mut i = 0;
            while i + pat.len() <= b.len() {
                if &b[i..i + pat.len()] == pat {
                    let start = i.saturating_sub(24);
                    let end = (i + 24).min(b.len());
                    println!("handle={:?} type={} ctx={:02X?}", handle, rec.type_code, &b[start..end]);
                    break;
                }
                i += 1;
            }
        }
    }
}
