use acadrust::DwgReader;
fn main() {
    let path = std::env::args().nth(1).expect("usage: p15_font_dump FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");
    println!("code_page={:?}", doc.header.code_page);
    for style in doc.text_styles.iter() {
        println!("STYLE name={:?} font={:?} big={:?}", style.name, style.font_file, style.big_font_file);
    }
    for e in doc.entities() {
        if let acadrust::entities::EntityType::Text(t) = e {
            println!("TEXT value={:?} style={:?} layer={:?}", t.value, t.style, t.common.layer);
        }
    }
}
