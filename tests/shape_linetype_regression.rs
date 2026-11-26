//! Real-drawing regression: no "shape 0 undefined" and no literal \U+ layer names.

use acadrust::{DwgReader, DwgWriter, TableEntry};

fn fixture() -> Option<std::path::PathBuf> {
    std::env::var("ACADIFC_FIXTURE")
        .ok()
        .map(std::path::PathBuf::from)
        .filter(|path| path.exists())
}

#[test]
fn real_ac1018_linetypes_and_layer_names_survive_noop() {
    let Some(path) = fixture() else {
        eprintln!("ACADIFC_FIXTURE not set; skip real drawing regression");
        return;
    };
    let mut reader = DwgReader::from_file(&path).expect("read");
    let before = reader.read().expect("decode");
    assert_eq!(before.version, acadrust::DxfVersion::AC1018);

    for layer in before.layers.iter() {
        assert!(
            !layer.name.contains("\\U+"),
            "layer name still contains literal Unicode escape: {:?}",
            layer.name
        );
    }
    for lt in before.line_types.iter() {
        for element in &lt.elements {
            if let Some(complex) = &element.complex {
                assert_ne!(
                    complex.shape_number(),
                    Some(0),
                    "linetype {:?} still has shape number 0",
                    lt.name
                );
            }
        }
    }

    let out = DwgWriter::write_to_vec(&before).expect("write no-op");
    let mut reader = DwgReader::from_stream(std::io::Cursor::new(out));
    let after = reader.read().expect("reopen");
    let before_names: Vec<String> = before.layers.iter().map(|l| l.name.clone()).collect();
    let after_names: Vec<String> = after.layers.iter().map(|l| l.name.clone()).collect();
    assert_eq!(before_names, after_names, "layer names changed after no-op");
    let describe = |doc: &acadrust::CadDocument| -> Vec<String> {
        doc.line_types.iter().map(|lt| format!("{lt:?}")).collect()
    };
    assert_eq!(describe(&before), describe(&after), "linetypes changed after no-op");

    // The merged xref geometry entity from the real drawing must survive.
    let xref_entity = acadrust::types::Handle::new(0xF100BFC);
    assert!(
        before.get_entity(xref_entity).is_some(),
        "fixture no longer contains the merged xref entity"
    );
    assert!(
        after.get_entity(xref_entity).is_some(),
        "no-op dropped an entity owned by an xref block"
    );
}
