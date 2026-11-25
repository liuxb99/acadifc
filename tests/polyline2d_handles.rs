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
