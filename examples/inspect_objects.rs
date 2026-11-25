//! Diagnostic: inspect object type/class/owner/raw record bytes/references/CRC.
//! Usage: inspect_objects <file.dwg> <hex-handle> [hex-handle ...]
use acadrust::{DwgReader, EntityType, Handle};
use std::io::Cursor;

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xC0C1u16;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let targets: Vec<u64> = args[2..].iter()
        .filter_map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .collect();
    let bytes = std::fs::read(path).expect("read");
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    let doc = r.read().expect("read DWG");
    println!("version={:?} codepage={} classes={}", doc.version, doc.header.code_page, doc.classes.len());
    for &h in &targets {
        println!("=== {:#X} ===", h);
        let mut found = false;
        for e in doc.entities() {
            let handle = e.common().handle;
            if handle != Handle::new(h) { continue; }
            found = true;
            println!("kind=ENTITY");
            println!("  handle={} owner={} layer={:?} linetype={:?}",
                     handle.value(), e.common().owner_handle.value(),
                     e.common().layer, e.common().linetype);
            println!("  reactors={:?} xdict={:?} eed_len={} xdata={}",
                     e.common().reactors.iter().map(|x| x.value()).collect::<Vec<_>>(),
                     e.common().xdictionary_handle.map(|x| x.value()),
                     e.common().extended_data.raw_dwg_eed.len(),
                     e.common().extended_data.len());
            if let EntityType::Unknown(u) = e {
                println!("  dxf_name={} dwg_type={} handle_bits={} source={:?}",
                         u.dxf_name, u.dwg_type_code, u.dwg_handle_bits, u.dwg_source_version);
                if let Some(raw) = &u.raw_dwg_data {
                    println!("  raw_len={} raw_crc16={:#06X} sha_like_bytes={:02X?}",
                             raw.len(), crc16(raw), &raw[..raw.len().min(24)]);
                }
                let layer_handle = doc.layers.get(&u.common.layer).map(|l| l.handle.value());
                println!("  refs: layer_handle={:?} owner={}", layer_handle, u.common.owner_handle.value());
            }
        }
        if let Some(obj) = doc.objects.get(&Handle::new(h)) {
            found = true;
            println!("kind=OBJECT {:?}", obj);
        }
        if !found { println!("NOT_FOUND"); }
    }
}
