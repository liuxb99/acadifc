fn main(){
    let mut doc = acadrust::CadDocument::new();
    doc.version = acadrust::DxfVersion::AC1018;
    let mut arc = acadrust::entities::Arc::new();
    arc.common.handle = acadrust::types::Handle::new(0x100);
    arc.common.owner_handle = doc.header.model_space_block_handle;
    arc.center = acadrust::types::Vector3::new(0.0,0.0,0.0);
    arc.radius = 300.0;
    arc.start_angle = 0.0;
    arc.end_angle = 135f64.to_radians();
    doc.add_entity(acadrust::EntityType::Arc(arc)).unwrap();
    let bytes = acadrust::DwgWriter::write_to_vec(&doc).unwrap();
    let mut r = acadrust::DwgReader::from_stream(std::io::Cursor::new(bytes));
    let back = r.read().unwrap();
    for e in back.entities(){
        if let acadrust::EntityType::Arc(a)=e {
            println!("start={} end={} deg start={:.2} end={:.2}", a.start_angle, a.end_angle, a.start_angle.to_degrees(), a.end_angle.to_degrees());
        }
    }
}
