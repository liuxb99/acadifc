// Regression: a damaged font name written by older builds (UTF-8 mojibake in a
// single-byte field) must be sanitized so AutoCAD 2006's SHLOAD cannot abort.
use acadrust::document::CadDocument;
use acadrust::types::DxfVersion;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).expect("usage: p15_badfont_test OUT.dwg");
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    doc.header.code_page = "ANSI_1252".to_string();
    let mut style = acadrust::tables::TextStyle::new("BadFontStyle");
    // Latin-1 mojibake of UTF-8 bytes (what AutoCAD reports as \U+00EF...).
    style.font_file = "ï¿½Ø±dï¿½Ê".to_string();
    style.big_font_file = "".to_string();
    style.handle = acadrust::types::Handle::new(0x40);
    style.handle = acadrust::types::Handle::new(0x40);
    doc.text_styles.add(style)?;
    acadrust::DwgWriter::write_to_file(&out, &doc)?;
    println!("wrote {out}");
    Ok(())
}
