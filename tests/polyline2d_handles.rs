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
