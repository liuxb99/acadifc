// Build a minimal AC1018 document with Chinese text-style font names and text,
// then inspect the encoded bytes through the project reader and LibreDWG.
use acadrust::document::CadDocument;
use acadrust::tables::{Layer, TextStyle};
use acadrust::types::DxfVersion;
use acadrust::entities::{Text, EntityType};
use acadrust::types::{Handle, Vector3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).expect("usage: p15_font_roundtrip OUT.dwg");
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    doc.header.code_page = "BIG5".to_string();
    let mut style = TextStyle::new("標楷體樣式");
    style.font_file = "標楷體.shx".to_string();
    style.big_font_file = "chineset.shx".to_string();
    doc.text_styles.add(style)?;
    let mut layer = Layer::new("圖框");
    layer.handle = Handle::new(0x20);
    doc.layers.add(layer)?;
    let mut text = Text::new();
    text.common.handle = Handle::new(0x30);
    text.common.layer = "圖框".to_string();
    text.insertion_point = Vector3::new(1.0, 2.0, 0.0);
    text.value = "測試文字".to_string();
    text.height = 2.5;
    text.style = "標楷體樣式".to_string();
    doc.add_entity(EntityType::Text(text))?;
    acadrust::DwgWriter::write_to_file(&out, &doc)?;
    println!("wrote {out}");
    Ok(())
}
