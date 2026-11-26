//! Dump SHAPE / STYLE / LTYPE references relevant to "shape 0 undefined".
//! Usage: cargo run --example diag_shape_linetype -- FILE.dwg

use acadrust::entities::Entity;
use acadrust::{DwgReader, EntityType, TableEntry};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: diag_shape_linetype FILE.dwg");
    let mut reader = DwgReader::from_file(&path).expect("open");
    let doc = reader.read().expect("read");

    println!(
        "VERSION {} SOURCE {:?} CODEPAGE {}",
        doc.version, doc.dwg_source_version, doc.header.code_page
    );
    println!("--- LAYERS ({}) ---", doc.layers.len());
    for layer in doc.layers.iter() {
        println!("LAYER {:?} handle={}", layer.name, layer.handle.value());
    }
    println!("--- TEXT STYLES ({}) ---", doc.text_styles.len());
    for style in doc.text_styles.iter() {
        println!(
            "STYLE {:?} handle={} font={:?} bigfont={:?} ttf={:?} shape={} flags={:?}",
            style.name,
            style.handle.value(),
            style.font_file,
            style.big_font_file,
            style.true_type_font,
            style.is_shape_file,
            style.flags
        );
    }
    println!("--- LTYPE COMPLEX ELEMENTS ---");
    let mut zero = 0usize;
    let mut null_style = 0usize;
    for lt in doc.line_types.iter() {
        for (index, element) in lt.elements.iter().enumerate() {
            if let Some(cx) = &element.complex {
                let shape = cx.shape_number();
                if shape == Some(0) {
                    zero += 1;
                }
                if cx.style_handle.is_null() {
                    null_style += 1;
                }
                println!(
                    "LTYPE {:?} elem={} shape={:?} text={:?} handle={:?} scale={} rot={} abs={}",
                    lt.name,
                    index,
                    shape,
                    cx.text(),
                    cx.style_handle.value(),
                    cx.scale,
                    cx.rotation,
                    cx.absolute_rotation
                );
            }
        }
    }
    println!(
        "SUMMARY complex_shape_zero={} complex_null_style={}",
        zero, null_style
    );

    println!("--- SHAPE ENTITIES ---");
    let mut shape_entities = 0usize;
    for entity in doc.entities() {
        if let EntityType::Shape(shape) = entity {
            shape_entities += 1;
            println!(
                "SHAPE handle={} name={:?} number={} size={} owner={} layer={:?}",
                shape.common.handle.value(),
                shape.shape_name,
                shape.shape_number,
                shape.size,
                shape.common.owner_handle.value(),
                shape.common.layer
            );
        }
    }
    println!("SUMMARY shape_entities={}", shape_entities);
}
