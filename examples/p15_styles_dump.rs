use acadrust::DwgReader;
fn main() {
    let path = std::env::args().nth(1).expect("usage: p15_styles_dump FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");
    println!("code_page={:?}", doc.header.code_page);
    for style in doc.text_styles.iter() {
        let codes: Vec<String> = style.font_file.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        let big: Vec<String> = style.big_font_file.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("STYLE name={:?} font={:?} font_codes=[{}] big={:?} big_codes=[{}]",
            style.name, style.font_file, codes.join(","), style.big_font_file, big.join(","));
    }
}
