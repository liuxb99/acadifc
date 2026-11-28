//! Phase 18 diagnostic: can a layer name with a supplementary CJK character
//! survive an AC1018 write/read round trip?
use acadrust::types::{DxfVersion, Handle};
use acadrust::{DwgReader, DwgWriter};

fn main() {
    let names = vec![
        "赻𠅅華倛".to_string(),
        "赻華倛".to_string(),
        "中文層".to_string(),
        "𠅅單".to_string(),
    ];
    let mut doc = acadrust::CadDocument::new();
    doc.version = DxfVersion::AC1018;
    doc.header.code_page = "BIG5".to_string();
    doc.dwg_code_page_index = Some(41);
    for (i, name) in names.iter().enumerate() {
        let mut layer = acadrust::tables::Layer::new(name.clone());
        layer.handle = Handle::new(0x100 + i as u64);
        doc.layers.add(layer).unwrap();
    }
    let path = "/tmp/p18_layer_name.dwg";
    DwgWriter::write_to_file(path, &doc).expect("write");
    let mut rd = DwgReader::from_file(path).expect("open");
    let back = rd.read().expect("read");
    let mut ok = true;
    for name in &names {
        let got = back.layers.iter().find(|l| {
            l.name.chars().next() == name.chars().next()
        }).map(|l| l.name.clone());
        let same = got.as_deref() == Some(name.as_str());
        println!("{:?} -> {:?} {}", name, got, if same { "OK" } else { "DRIFT" });
        ok &= same;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
