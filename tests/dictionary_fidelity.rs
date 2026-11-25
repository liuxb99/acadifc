//! Synthetic, fixture-independent DWG round-trip tests for Dictionary fidelity.
//!
//! A Dictionary whose entries point at Material / VisualStyle records must
//! keep every `(name, handle)` pair after write + re-read.  These records are
//! emitted by the object writer for every supported version (once their class
//! is present), so a pre-R2007 version must not silently discard the entries.
//!
//! Regression case: AC1018 / ANSI_950, where the old pre-R2007 writability
//! gate dropped `"ByBlock"`, `"ByLayer"`, `"Global"`, `"2dWireframe"` etc.
//! from the NOD while leaving the referenced objects in the file.

use std::collections::BTreeMap;
use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{Material, ObjectType, VisualStyle};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter};

/// Build a deterministic document with several named dictionary entries that
/// point at real, writable Material / VisualStyle records.
fn build_document(version: DxfVersion, code_page: &str) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    doc.header.code_page = code_page.to_string();

    // The object writer emits these only when their CLASSES entry exists.
    doc.classes
        .add_or_update(DxfClass::new("MATERIAL", "AcDbMaterial"));
    doc.classes
        .add_or_update(DxfClass::new("VISUALSTYLE", "AcDbVisualStyle"));

    let material_dict = doc.header.acad_material_dict_handle;
    let visualstyle_dict = doc.header.acad_visualstyle_dict_handle;

    // Material records — the "ByBlock"/"ByLayer"/"Global" pattern from the
    // AC1018 regression fixture, but with locally allocated handles.
    let mut mat_handles = Vec::new();
    for name in ["ByBlock", "ByLayer", "Global"] {
        let handle = doc.allocate_handle();
        let mut material = Material::new();
        material.handle = handle;
        material.owner = material_dict;
        material.name = name.to_string();
        material.description = format!("synthetic material {name}");
        doc.objects.insert(handle, ObjectType::Material(material));
        mat_handles.push(handle);
    }

    // VisualStyle records — multiple named entries in one dictionary.
    let mut style_handles = Vec::new();
    for name in ["2dWireframe", "Basic", "Brighten"] {
        let handle = doc.allocate_handle();
        let mut style = VisualStyle::new();
        style.handle = handle;
        style.owner = visualstyle_dict;
        style.description = name.to_string();
        doc.objects.insert(handle, ObjectType::VisualStyle(style));
        style_handles.push(handle);
    }

    let material_entries: Vec<(String, Handle)> = ["ByBlock", "ByLayer", "Global"]
        .iter()
        .zip(mat_handles)
        .map(|(name, h)| ((*name).to_string(), h))
        .collect();
    let visualstyle_entries: Vec<(String, Handle)> = ["2dWireframe", "Basic", "Brighten"]
        .iter()
        .zip(style_handles)
        .map(|(name, h)| ((*name).to_string(), h))
        .collect();

    for (dict_handle, entries) in [
        (material_dict, material_entries),
        (visualstyle_dict, visualstyle_entries),
    ] {
        let Some(ObjectType::Dictionary(dict)) = doc.objects.get_mut(&dict_handle) else {
            panic!("missing seeded dictionary {dict_handle:?}");
        };
        dict.entries = entries;
        // Non-default flags must survive too; a fidelity fix must not ignore
        // duplicate_cloning / hard_owner.
        dict.duplicate_cloning = 2;
        dict.hard_owner = true;
    }

    doc
}

fn dictionary_snapshot(doc: &CadDocument) -> BTreeMap<u64, (u64, i16, bool, Vec<(String, u64)>)> {
    doc.objects
        .iter()
        .filter_map(|(handle, object)| match object {
            ObjectType::Dictionary(dict) => Some((
                handle.value(),
                (
                    dict.owner.value(),
                    dict.duplicate_cloning,
                    dict.hard_owner,
                    dict.entries
                        .iter()
                        .map(|(name, h)| (name.clone(), h.value()))
                        .collect(),
                ),
            )),
            _ => None,
        })
        .collect()
}

fn roundtrip(version: DxfVersion, code_page: &str) -> (CadDocument, CadDocument) {
    let original = build_document(version, code_page);
    let before = original.clone();

    let bytes = DwgWriter::write_to_vec(&original).expect("DWG write");
    let reread = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("DWG re-read");

    // Clone must be lossless (Stage A/B); losses would be in write + re-read.
    assert_eq!(
        dictionary_snapshot(&before),
        dictionary_snapshot(&original),
        "clone changed dictionaries"
    );
    (original, reread)
}

fn assert_dictionaries_preserved(original: &CadDocument, reread: &CadDocument, label: &str) {
    let before = dictionary_snapshot(original);
    let after = dictionary_snapshot(reread);

    for (handle, expected) in &before {
        let actual = after
            .get(handle)
            .unwrap_or_else(|| panic!("{label}: dictionary {handle:#X} missing after round trip"));
        assert_eq!(actual, expected, "{label}: dictionary {handle:#X} differs");
    }

    // The named entries we added must still be present with their exact
    // handles, not merely "some entries of the right count".
    for (dict_handle, names) in [
        (
            original.header.acad_material_dict_handle,
            vec!["ByBlock", "ByLayer", "Global"],
        ),
        (
            original.header.acad_visualstyle_dict_handle,
            vec!["2dWireframe", "Basic", "Brighten"],
        ),
    ] {
        let expected = &before[&dict_handle.value()];
        let expected_entries: BTreeMap<&str, u64> =
            expected.3.iter().map(|(n, h)| (n.as_str(), *h)).collect();
        let actual = &after[&dict_handle.value()];
        let actual_entries: BTreeMap<&str, u64> =
            actual.3.iter().map(|(n, h)| (n.as_str(), *h)).collect();
        for name in names {
            let h = expected_entries[name];
            assert_eq!(
                actual_entries.get(name),
                Some(&h),
                "{label}: entry {name:?} lost from dictionary {dict_handle:#X}"
            );
            assert!(
                reread.objects.contains_key(&Handle::from(h)),
                "{label}: entry {name:?} target {h:#X} missing as object"
            );
        }
    }
}

#[test]
fn dictionary_entries_survive_ac1018_ansi_950() {
    let (original, reread) = roundtrip(DxfVersion::AC1018, "ANSI_950");
    assert_eq!(reread.version, DxfVersion::AC1018);
    // The DWG header stores a numeric code page; re-reading maps 950/Big5 to
    // the canonical "BIG5" spelling. Both names select the same encoding.
    assert_eq!(
        acadrust::io::dxf::code_page::encoding_from_code_page(&reread.header.code_page),
        acadrust::io::dxf::code_page::encoding_from_code_page("ANSI_950"),
    );
    assert_dictionaries_preserved(&original, &reread, "AC1018/ANSI_950");
}

#[test]
fn dictionary_entries_survive_ac1015() {
    // Another pre-R2007 version: same legacy Material/VisualStyle layout.
    let (original, reread) = roundtrip(DxfVersion::AC1015, "ANSI_1252");
    assert_eq!(reread.version, DxfVersion::AC1015);
    assert_dictionaries_preserved(&original, &reread, "AC1015");
}

#[test]
fn dictionary_entries_survive_ac1021() {
    // Control: R2007 passed the old gate; must still pass after the fix.
    let (original, reread) = roundtrip(DxfVersion::AC1021, "ANSI_1252");
    assert_eq!(reread.version, DxfVersion::AC1021);
    assert_dictionaries_preserved(&original, &reread, "AC1021");
}
