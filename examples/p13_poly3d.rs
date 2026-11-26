use acadrust::entities::polyline3d::{Polyline3D, Vertex3DPolyline};
use acadrust::types::{Handle, Vector3};
fn main(){
    let mut doc = acadrust::CadDocument::new();
    doc.version = acadrust::DxfVersion::AC1018;
    let model = doc.header.model_space_block_handle;
    let mut pl = Polyline3D::new();
    pl.common.handle = Handle::new(0x100);
    pl.common.owner_handle = model;
    pl.seqend_handle = Some(Handle::new(0x103));
    for (i,x) in [0.0,10.0,20.0].into_iter().enumerate() {
        let mut v = Vertex3DPolyline::new(Vector3::new(x,0.0,0.0));
        v.handle = Handle::new(0x101+i as u64);
        pl.vertices.push(v);
    }
    doc.add_entity(acadrust::EntityType::Polyline3D(pl)).unwrap();
    let bytes = acadrust::DwgWriter::write_to_vec(&doc).unwrap();
    std::fs::write("/tmp/p13/poly3d.dwg", &bytes).unwrap();
}
