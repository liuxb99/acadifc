use acadrust::{DwgReader, EntityType};
use std::io::Cursor;
fn main(){
 let p=std::env::args().nth(1).unwrap();
 let bytes=std::fs::read(p).unwrap();
 let mut r=DwgReader::from_stream(Cursor::new(bytes));
 let d=r.read().unwrap();
 for e in d.entities(){
  if let EntityType::Polyline2D(poly)=e{
   if format!("{:X}",poly.common.handle.value())=="C515C"{
    println!("C515C seq={:?}",poly.seqend_handle);
    println!("handles={:?}",poly.vertices.iter().map(|v|v.handle.value()).collect::<Vec<_>>());
    println!("points={:?}",poly.vertices.iter().map(|v|(v.location.x,v.location.y)).collect::<Vec<_>>());
   }
  }
 }
}
