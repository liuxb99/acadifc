//! Dev-time generator: CP950 vs WHATWG-Big5 differences for the DWG legacy encoder.
//!
//! Reads /tmp/cp950_pairs.txt (produced from Python's cp950 codec, one
//! `U+XXXX bb bb` per line) and prints the Rust static table of characters whose
//! WHATWG encoding_rs BIG5 bytes differ from the classic CP950 bytes. Those are
//! exactly the characters where AutoCAD's CP950 decoder and our encoder disagree.
use encoding_rs::BIG5;
use std::io::{BufRead, BufReader};

fn main() {
    let path = std::env::args().nth(1).expect("table file");
    let f = std::fs::File::open(&path).expect("open");
    let mut diffs: Vec<(u32, u8, u8, String)> = Vec::new();
    let mut total = 0usize;
    for line in BufReader::new(f).lines() {
        let line = line.expect("line");
        let mut it = line.split_whitespace();
        let cp = u32::from_str_radix(it.next().expect("cp"), 16).expect("hex");
        let a = u8::from_str_radix(it.next().expect("a"), 16).expect("hex");
        let b = u8::from_str_radix(it.next().expect("b"), 16).expect("hex");
        total += 1;
        let ch = match char::from_u32(cp) { Some(c) => c, None => continue };
        let mut buf = [0u8; 4]; let s = ch.encode_utf8(&mut buf); let (enc, _, err) = BIG5.encode(s);
        let why = if err {
            "not-encodable".to_string()
        } else if enc.len() != 2 || enc[0] != a || enc[1] != b {
            format!("{}{}", enc.iter().map(|x| format!("{x:02X}")).collect::<String>(), "")
        } else {
            continue;
        };
        diffs.push((cp, a, b, why));
    }
    eprintln!("cp950 pairs: {total}, differing: {}", diffs.len());
    diffs.sort_by_key(|d| d.0);
    println!("pub(crate) static CP950_DIFF: &[(u32, [u8; 2])] = &[");
    for (cp, a, b, why) in &diffs {
        println!("    (0x{cp:04X}, [0x{a:02X}, 0x{b:02X}]), // {}", why);
    }
    println!("];");
}
