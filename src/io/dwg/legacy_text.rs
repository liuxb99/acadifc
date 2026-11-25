//! Lossless legacy code-page text for pre-R2007 DWG records.
//!
//! `encoding_rs` deliberately replaces characters it cannot encode with HTML
//! numeric character references (`&#NNN;`).  A string that was decoded from a
//! DWG byte stream containing an undecodable byte therefore contains U+FFFD,
//! and writing it back produced the literal ASCII text `&#65533;`.  On the
//! next read the layer name (or any other legacy string) was different, so a
//! no-op DWG round trip reported e.g. `Unknown/common/layer` as a change.
//!
//! This module makes the decode/encode pair reversible without changing any
//! valid string: undecodable bytes are represented in memory by a private-use
//! pair `U+E000`, `U+E000 + byte`.  Legacy DWG code pages cannot encode the
//! private-use area, so a genuine string can never contain that pair and the
//! mapping is unambiguous.  The writer turns the pair back into the original
//! byte instead of letting `encoding_rs` emit `&#NNN;`.

use encoding_rs::Encoding;
use std::collections::HashMap;
use std::sync::RwLock;

/// First code point of the private-use marker pair.
const MARKER_BASE: u32 = 0xE000;
const MARKER: char = '\u{E000}';

/// Decode a legacy DWG string, preserving undecodable bytes reversibly.
pub(crate) fn decode_legacy_text(encoding: &'static Encoding, bytes: &[u8]) -> String {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    let decoded = decoded.replace('\0', "");
    if !had_errors {
        return decoded;
    }

    // Re-scan with a *strict* decoder so we know exactly which bytes were
    // undecodable.  Guessing at valid prefixes is unsafe: in Big5 a stray
    // lead byte followed by valid text can be mis-read as a different
    // character and shift the rest of the string.
    let mut decoder = encoding.new_decoder();
    let mut out = String::with_capacity(decoded.len() + 16);
    let mut rest = bytes;
    loop {
        out.reserve(rest.len() + 8);
        let (result, read) = decoder.decode_to_string_without_replacement(rest, &mut out, true);
        match result {
            encoding_rs::DecoderResult::InputEmpty => break,
            encoding_rs::DecoderResult::OutputFull => {
                rest = &rest[read..];
            }
            encoding_rs::DecoderResult::Malformed(bad_len, after) => {
                let bad_end = read.saturating_sub(after as usize);
                let bad_start = bad_end.saturating_sub(bad_len as usize);
                for &b in &rest[bad_start..bad_end] {
                    out.push(MARKER);
                    // `b` is 0..=255, so the second code point stays in the
                    // private-use range U+E000..=U+E0FF.
                    out.push(char::from_u32(MARKER_BASE + b as u32).unwrap());
                }
                rest = &rest[read..];
            }
        }
    }
    out
}

/// Encode a legacy DWG string without `encoding_rs`'s numeric-entity fallback.
pub(crate) fn encode_legacy_text(encoding: &'static Encoding, text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == MARKER {
            if let Some(&next) = chars.peek() {
                let value = next as u32;
                if (MARKER_BASE..=MARKER_BASE + 0xFF).contains(&value) {
                    out.extend_from_slice(&encode_plain(encoding, &plain));
                    plain.clear();
                    out.push((value - MARKER_BASE) as u8);
                    chars.next();
                    continue;
                }
            }
        }
        plain.push(ch);
    }
    out.extend_from_slice(&encode_plain(encoding, &plain));
    out
}

/// Some legacy decoders (notably Big5) map byte pairs to supplementary-plane
/// CJK characters that their encoder cannot encode back. `encoding_rs` then
/// emits `&#NNN;` (or, after our earlier fix, `?`), which silently changes the
/// value. Build the decoder's own reverse map so the original byte pair can be
/// restored exactly. This is derived from the code page itself, never from a
/// drawing-specific string.
fn reverse_byte_map(encoding: &'static Encoding) -> HashMap<char, Vec<u8>> {
    let mut map = HashMap::new();
    for first in 0u16..=0xFF {
        let single = [first as u8];
        let (decoded, _, had_errors) = encoding.decode(&single);
        if !had_errors {
            if let Some(ch) = decoded.chars().next() {
                if decoded.chars().count() == 1 {
                    map.entry(ch).or_insert_with(|| single.to_vec());
                }
            }
        }
        for second in 0u16..=0xFF {
            let pair = [first as u8, second as u8];
            let (decoded, _, had_errors) = encoding.decode(&pair);
            if !had_errors && decoded.chars().count() == 1 {
                let ch = decoded.chars().next().unwrap();
                map.entry(ch).or_insert_with(|| pair.to_vec());
            }
        }
    }
    map
}

fn reverse_maps() -> &'static RwLock<HashMap<String, HashMap<char, Vec<u8>>>> {
    static MAPS: std::sync::OnceLock<RwLock<HashMap<String, HashMap<char, Vec<u8>>>>> =
        std::sync::OnceLock::new();
    MAPS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn reverse_bytes(encoding: &'static Encoding, ch: char) -> Option<Vec<u8>> {
    let key = encoding.name().to_string();
    if let Some(map) = reverse_maps().read().unwrap().get(&key) {
        return map.get(&ch).cloned();
    }
    let map = reverse_byte_map(encoding);
    let result = map.get(&ch).cloned();
    reverse_maps()
        .write()
        .unwrap()
        .entry(key)
        .or_insert(map);
    result
}

/// Encode an ordinary run without numeric-entity fallback or silent `?`.
fn encode_plain(encoding: &'static Encoding, text: &str) -> Vec<u8> {
    let (encoded, _, had_errors) = encoding.encode(text);
    if !had_errors {
        return encoded.into_owned();
    }
    let mut out = Vec::with_capacity(text.len());
    let mut run = String::with_capacity(text.len());
    for ch in text.chars() {
        let (one, _, err) = encoding.encode(&ch.to_string());
        if !err {
            run.push(ch);
            continue;
        }
        if !run.is_empty() {
            out.extend_from_slice(encoding.encode(&run).0.as_ref());
            run.clear();
        }
        match reverse_bytes(encoding, ch) {
            Some(bytes) => out.extend_from_slice(&bytes),
            None => out.push(b'?'),
        }
    }
    if !run.is_empty() {
        out.extend_from_slice(encoding.encode(&run).0.as_ref());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undecodable_big5_bytes_round_trip_bit_for_bit() {
        // `C8 BB` is not a valid Big5 pair; `encoding_rs` reports an error for
        // it.  This is the byte pair in the real fixture's `Unknown` layer.
        let bytes = [0xD7, 0xD4, 0xC8, 0xBB, 0xB5, 0xD8, 0xD0, 0xCE];
        let text = decode_legacy_text(encoding_rs::BIG5, &bytes);
        assert!(!text.contains('\u{FFFD}'), "replacement char must not be public");
        // The exact character count is an encoding_rs implementation detail;
        // the contract is that the original bytes are recovered exactly.
        assert_eq!(encode_legacy_text(encoding_rs::BIG5, &text), bytes);
    }

    #[test]
    fn ordinary_strings_stay_unchanged() {
        let bytes = b"MAP|standard";
        let text = decode_legacy_text(encoding_rs::WINDOWS_1252, bytes);
        assert_eq!(text, "MAP|standard");
        assert_eq!(encode_legacy_text(encoding_rs::WINDOWS_1252, &text), bytes);
    }
    #[test]
    fn layer_name_with_undecodable_legacy_bytes_survives_dwg_round_trip() {
        use crate::{CadDocument, DwgReader, DwgWriter};
        use std::io::Cursor;

        let mut document = CadDocument::new();
        document.header.code_page = "ANSI_950".to_string();
        // U+E000 + byte is the reversible representation of an undecodable
        // Big5 byte produced by `decode_legacy_text`.
        let mut name = String::from("MAP|");
        name.push('\u{E000}');
        name.push(char::from_u32(MARKER_BASE + 0xC8).unwrap());
        name.push_str("standard");
        let mut layer = crate::tables::Layer::new(name.clone());
        layer.handle = crate::types::Handle::new(0x1234);
        let _ = document.layers.add(layer);

        let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
        let mut reader = DwgReader::from_stream(Cursor::new(bytes));
        let reopened = reader.read().expect("reopen DWG");
        let found = reopened
            .layers
            .iter()
            .find(|layer| layer.handle.value() == 0x1234)
            .expect("layer must survive");
        assert_eq!(found.name, name, "legacy layer bytes must be preserved verbatim");
        assert!(!found.name.contains("&#"), "numeric entity fallback must never be written");
    }

    #[test]
    fn supplementary_big5_character_round_trips_bit_for_bit() {
        // Big5 decoder maps 0x8745 to U+27267, but encoding_rs' Big5 encoder
        // cannot encode that character back. The reverse map must recover the
        // original bytes instead of writing `?` or `&#NNN;`.
        let bytes = [0x87, 0x45];
        let text = decode_legacy_text(encoding_rs::BIG5, &bytes);
        assert_eq!(text.chars().count(), 1);
        assert_eq!(encode_legacy_text(encoding_rs::BIG5, &text), bytes);
    }
}
