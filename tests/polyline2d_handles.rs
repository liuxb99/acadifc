//! Regression: POLYLINE_2D child handles must survive a native DWG round trip.
//!
//! An independent reader previously saw every VERTEX handle and the SEQEND
//! handle re-addressed after `DwgWriter` rewrote an unmodified drawing.

use acadrust::entities::polyline::{Vertex2D, VertexFlags};
use acadrust::entities::Polyline2D;
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType, Handle, DxfVersion};
use std::io::Cursor;

fn roundtrip(version: DxfVersion) {
    let mut document = CadDocument::new();
    document.version = version;

    let mut polyline = Polyline2D::new();
    polyline.common.handle = Handle::new(0x400);
    polyline.vertices.push(Vertex2D {
        handle: Handle::new(0x401),
        location: acadrust::Vector3::new(1.0, 2.0, 0.0),
        flags: VertexFlags::new(),
        start_width: 0.0,
        end_width: 0.0,
        bulge: 0.0,
        curve_tangent: 0.0,
        id: 0,
    });
    polyline.vertices.push(Vertex2D {
        handle: Handle::new(0x402),
        location: acadrust::Vector3::new(3.0, 4.0, 0.0),
        ..polyline.vertices[0].clone()
    });
    polyline.seqend_handle = Some(Handle::new(0x403));
    document.add_entity(EntityType::Polyline2D(polyline)).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let reopened = reader.read().expect("reopen DWG");
    let found = reopened
        .entities()
        .find_map(|entity| match entity {
            EntityType::Polyline2D(polyline) if polyline.common.handle == Handle::new(0x400) => {
                Some(polyline)
            }
            _ => None,
        })
        .expect("polyline must survive");

    assert_eq!(found.vertices.len(), 2);
    assert_eq!(found.vertices[0].handle, Handle::new(0x401));
    assert_eq!(found.vertices[1].handle, Handle::new(0x402));
    assert_eq!(found.seqend_handle, Some(Handle::new(0x403)));
}

#[test]
fn ac1018_polyline2d_child_handles_survive() {
    roundtrip(DxfVersion::AC1018);
}

#[test]
fn ac1027_polyline2d_child_handles_survive() {
    roundtrip(DxfVersion::AC1027);
}

/// The source DWG can list VERTEX children in an order that is NOT ascending
/// by handle. The authoritative order is the POLYLINE record's owned-handle
/// list, not the VERTEX object-map order. Regression for C515C in the AC1018
/// generic fixture (independent reader order 807266, 807265, 807261, ...).
#[test]
fn ac1018_polyline2d_child_order_survives() {
    let mut document = CadDocument::new();
    document.version = DxfVersion::AC1018;

    let mut polyline = Polyline2D::new();
    polyline.common.handle = Handle::new(0x500);
    // Deliberately unsorted: 0x521, 0x51F, 0x520.
    for (handle, x) in [(0x521u64, 1.0), (0x51F, 2.0), (0x520, 3.0)] {
        let mut v = Vertex2D::new(acadrust::Vector3::new(x, x * 10.0, 0.0));
        v.handle = Handle::new(handle);
        polyline.vertices.push(v);
    }
    polyline.seqend_handle = Some(Handle::new(0x522));
    document.add_entity(EntityType::Polyline2D(polyline)).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let reopened = reader.read().expect("reopen DWG");
    let found = reopened
        .entities()
        .find_map(|entity| match entity {
            EntityType::Polyline2D(p) if p.common.handle == Handle::new(0x500) => Some(p),
            _ => None,
        })
        .expect("polyline must survive");

    let handles: Vec<u64> = found.vertices.iter().map(|v| v.handle.value()).collect();
    let xs: Vec<f64> = found.vertices.iter().map(|v| v.location.x).collect();
    assert_eq!(handles, vec![0x521, 0x51F, 0x520], "chain order must be preserved");
    assert_eq!(xs, vec![1.0, 2.0, 3.0], "vertex/geometry association must hold");
    assert_eq!(found.seqend_handle, Some(Handle::new(0x522)));
}

/// A copied polyline must be re-addressed independently: assigning new
/// handles to the copy (parent + every VERTEX + SEQEND) must not change the
/// original entity or its child chain.
#[test]
fn ac1018_copied_polyline_gets_independent_child_handles() {
    let mut document = CadDocument::new();
    document.version = DxfVersion::AC1018;

    let mut original = Polyline2D::new();
    original.common.handle = Handle::new(0x600);
    for (handle, x) in [(0x603u64, 1.0), (0x601, 2.0), (0x602, 3.0)] {
        let mut v = Vertex2D::new(acadrust::Vector3::new(x, x * 10.0, 0.0));
        v.handle = Handle::new(handle);
        original.vertices.push(v);
    }
    original.seqend_handle = Some(Handle::new(0x604));
    let original_handles: Vec<u64> = original.vertices.iter().map(|v| v.handle.value()).collect();

    let mut copy = original.clone();
    copy.common.handle = Handle::new(0x700);
    for (v, new_h) in copy.vertices.iter_mut().zip([0x703u64, 0x701, 0x702]) {
        v.handle = Handle::new(new_h);
    }
    copy.seqend_handle = Some(Handle::new(0x704));

    document.add_entity(EntityType::Polyline2D(original)).unwrap();
    document.add_entity(EntityType::Polyline2D(copy)).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let reopened = reader.read().expect("reopen DWG");
    let polys: Vec<&Polyline2D> = reopened.entities().filter_map(|e| match e {
        EntityType::Polyline2D(p) => Some(p),
        _ => None,
    }).collect();
    assert_eq!(polys.len(), 2);
    let orig = polys.iter().find(|p| p.common.handle == Handle::new(0x600)).unwrap();
    let copy = polys.iter().find(|p| p.common.handle == Handle::new(0x700)).unwrap();

    assert_eq!(orig.vertices.iter().map(|v| v.handle.value()).collect::<Vec<_>>(), original_handles);
    assert_eq!(orig.seqend_handle, Some(Handle::new(0x604)));
    assert_eq!(copy.vertices.iter().map(|v| v.handle.value()).collect::<Vec<_>>(),
               vec![0x703, 0x701, 0x702]);
    assert_eq!(copy.seqend_handle, Some(Handle::new(0x704)));
    // Geometry association must follow the child order in both objects.
    assert_eq!(orig.vertices.iter().map(|v| v.location.x).collect::<Vec<_>>(), vec![1.0, 2.0, 3.0]);
    assert_eq!(copy.vertices.iter().map(|v| v.location.x).collect::<Vec<_>>(), vec![1.0, 2.0, 3.0]);
}

// Phase 18 regression (ACadSharp "Item Vertex2D already has an owner"):
// the R2004+ BLOCK_RECORD owned list must contain only top-level entities.
// VERTEX/SEQEND children are owned by their POLYLINE; listing them here gave
// each child two owners and a strict DWG reader rejected the file.
#[test]
fn ac1018_block_header_owned_list_excludes_polyline_children() {
    let mut document = CadDocument::new();
    document.version = DxfVersion::AC1018;
    let mut polyline = Polyline2D::new();
    polyline.common.handle = Handle::new(0x400);
    polyline.vertices.push(Vertex2D {
        handle: Handle::new(0x401),
        location: acadrust::Vector3::new(1.0, 2.0, 0.0),
        flags: VertexFlags::new(),
        start_width: 0.0,
        end_width: 0.0,
        bulge: 0.0,
        curve_tangent: 0.0,
        id: 0,
    });
    polyline.seqend_handle = Some(Handle::new(0x403));
    document.add_entity(EntityType::Polyline2D(polyline)).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let reopened = reader.read().expect("reopen DWG");
    let model = reopened.header.model_space_block_handle;
    let owned = reopened.block_header_owned_handles(model);
    assert!(owned.iter().any(|h| h.value() == 0x400),
            "the polyline itself must stay in the block owned list: {owned:?}");
    for child in [0x401u64, 0x403] {
        assert!(!owned.iter().any(|h| h.value() == child),
                "child handle {child:#X} must not be owned by the block record: {owned:?}");
    }
}

// Phase 18 defect B (synthesised BLOCK marker keeps its unique layout name) is
// covered by the in-module unit tests in object_writer/mod.rs; the writer-level
// end-to-end gate is the ACadSharp scan (deliverables/phase18/engines/dup-owner).
