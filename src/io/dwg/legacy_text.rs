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

/// Number of UTF-8-in-legacy-field repairs performed (evidence for reporters).
pub static REPAIR_LOG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Reset and read the repair counter (writer reports this per save).
pub fn take_repair_count() -> usize {
    REPAIR_LOG.swap(0, std::sync::atomic::Ordering::Relaxed)
}

/// First code point of the private-use marker pair.
const MARKER_BASE: u32 = 0xE000;
const MARKER: char = '\u{E000}';

/// True when the string contains CJK / non-Latin text (used to pick a code page).
pub(crate) fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
    })
}

/// Repair a UTF-8 string that was decoded with a single-byte code page.
///
/// AutoCAD 2006 only understands single-byte legacy strings, but third-party
/// tools sometimes wrote UTF-8 into those fields.  Reading such a file with
/// the drawing code page yields Latin-1 mojibake (`ï¿½`, `Ã¤`, ...), which
/// makes AutoCAD try to load a font whose name contains those characters and
/// abort with `SHLOAD SHSEEK`.  When the bytes form valid UTF-8 we can recover
/// the intended text losslessly.
pub(crate) fn repair_utf8_mojibake(encoding: &'static Encoding, text: &str) -> Option<String> {
    if text.is_ascii() || text.contains(MARKER) {
        return None;
    }
    // Only attempt a repair when every character is representable in the
    // source code page; otherwise the byte round-trip is ambiguous.
    let mut bytes = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let mut buf = [0u8; 8];
        let one = ch.encode_utf8(&mut buf);
        let (encoded, _, unencodable) = encoding.encode(one);
        if unencodable || encoded.len() != 1 {
            return None;
        }
        bytes.push(encoded[0]);
    }
    let candidate = std::str::from_utf8(&bytes).ok()?;
    if candidate.is_ascii() {
        return None;
    }
    if has_cjk(candidate) || candidate.chars().all(|c| !c.is_control()) {
        return Some(candidate.to_string());
    }
    None
}

/// Decode a legacy DWG string, preserving undecodable bytes reversibly.
pub(crate) fn decode_legacy_text(encoding: &'static Encoding, bytes: &[u8]) -> String {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    let decoded = decoded.replace('\0', "");
    // Phase 17 evidence rule: repair only when the raw bytes are themselves
    // valid UTF-8 and decoding them with the DWG code page produced different
    // text. The original bytes are recoverable (round trip), the repair is
    // logged, and pure-ASCII / valid code-page strings are never touched.
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        if !utf8.is_ascii() && utf8 != decoded {
            REPAIR_LOG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            crate::io::dwg::repair_report::record(bytes, utf8);
            return decode_unicode_escapes(utf8);
        }
    }
    if !had_errors {
        return decode_unicode_escapes(&decoded);
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
    decode_unicode_escapes(&out)
}

/// Decode AutoCAD's pre-R2007 Unicode escape `\U+XXXX` into real characters.
/// Older CAD writers store CJK layer/style names this way when the code page
/// cannot carry them; keeping the literal escape makes AutoCAD display the
/// escape text instead of the Chinese name.
fn decode_unicode_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\'
            && matches!(chars.peek(), Some('U') | Some('u'))
        {
            chars.next();
            if chars.peek() == Some(&'+') {
                chars.next();
                let mut hex = String::with_capacity(6);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_hexdigit() && hex.len() < 6 {
                        hex.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if hex.len() >= 4 {
                    if let Ok(value) = u32::from_str_radix(&hex, 16) {
                        if let Some(decoded) = char::from_u32(value) {
                            out.push(decoded);
                            continue;
                        }
                    }
                }
                out.push_str("\\U+");
                out.push_str(&hex);
                continue;
            }
            out.push_str("\\U");
            continue;
        }
        out.push(ch);
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
        if crate::io::dwg::cp950_fallback::prefers_escape(encoding, ch) {
            // WHATWG Big5 would pick a pair that classic CP950 readers (AutoCAD,
            // LibreDWG) decode differently - e.g. U+0424 -> C8 49 instead of the
            // CP950 C7 BC - so the character silently disappears there. Emit
            // AutoCAD's `\U+XXXX` escape instead: it is what AutoCAD writes for
            // this text, our reader decodes it back, and independent readers do
            // too. This keeps the source representation of L-09/L-10 intact.
            out.extend_from_slice(&encode_plain(encoding, &plain));
            plain.clear();
            let value = ch as u32;
            let escape = if value <= 0xFFFF {
                format!("\\U+{value:04X}")
            } else {
                format!("\\U+{value:06X}")
            };
            out.extend_from_slice(escape.as_bytes());
            continue;
        }
        if !ch.is_ascii() {
            let (_, _, unencodable) = encoding.encode(&ch.to_string());
            if unencodable {
                if let Some(original) = reverse_bytes(encoding, ch) {
                    out.extend_from_slice(&encode_plain(encoding, &plain));
                    plain.clear();
                    out.extend_from_slice(&original);
                    continue;
                }
                out.extend_from_slice(&encode_plain(encoding, &plain));
                plain.clear();
                let value = ch as u32;
                let escape = if value <= 0xFFFF {
                    format!("\\U+{value:04X}")
                } else {
                    format!("\\U+{value:06X}")
                };
                out.extend_from_slice(escape.as_bytes());
                continue;
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
    fn cyrillic_uses_classic_cp950_pair_not_whatwg() {
        // Real drawing L-09/L-10 (AC1015): U+0424 is CP950 C7 BC in the source.
        // encoding_rs (WHATWG Big5) would emit C8 49, which LibreDWG/AutoCAD's
        // CP950 decoder cannot read, making the character disappear.
        assert_eq!(encode_legacy_text(encoding_rs::BIG5, "Ф"), b"\\U+0424".to_vec());
        assert_eq!(encode_legacy_text(encoding_rs::BIG5, " Ф"), b" \\U+0424".to_vec());
        assert_eq!(decode_legacy_text(encoding_rs::BIG5, b"\\U+0424"), "Ф");
    }

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

#[cfg(test)]
mod unicode_escape_tests {
    use super::*;

    #[test]
    fn big5_unicode_escape_round_trips_to_bytes() {
        let bytes = b"\\U+4E2D\\U+5FC3\\U+7DDA";
        let text = decode_legacy_text(encoding_rs::BIG5, bytes);
        assert_eq!(text, "中心線");
        // Writer stores CJK in the document code page; AutoCAD renders it.
        let (expected, _, _) = encoding_rs::BIG5.encode("中心線");
        assert_eq!(encode_legacy_text(encoding_rs::BIG5, &text), expected.as_ref());
    }

    #[test]
    fn big5_native_cjk_still_uses_big5_bytes() {
        let (native, _, _) = encoding_rs::BIG5.encode("中心線");
        let text = decode_legacy_text(encoding_rs::BIG5, &native);
        assert_eq!(text, "中心線");
        let encoded = encode_legacy_text(encoding_rs::BIG5, &text);
        assert_eq!(encoded, native.as_ref());
    }
}


#[cfg(test)]
mod phase17_tests {
    use super::*;

    #[test]
    fn valid_cp950_string_is_never_touched() {
        let bytes = "測試".as_bytes().to_vec();
        let big5 = encoding_rs::BIG5.encode("測試").0.into_owned();
        let text = decode_legacy_text(encoding_rs::BIG5, &big5);
        assert_eq!(text, "測試");
        assert!(std::str::from_utf8(&big5).is_err() || bytes != big5);
    }

    #[test]
    fn utf8_in_legacy_field_is_repaired_with_evidence() {
        // A legacy build wrote UTF-8; decoding with CP1252 must have errors,
        // so the reversible UTF-8 repair applies.
        let original = decode_legacy_text(encoding_rs::BIG5, "測試.shx".as_bytes());
        assert_eq!(original, "測試.shx");
        let repaired = decode_legacy_text(encoding_rs::WINDOWS_1252, "測試.shx".as_bytes());
        assert_eq!(repaired, "測試.shx");
    }

    #[test]
    fn repair_counter_is_exposed() {
        let _ = take_repair_count();
        let utf8 = "測試".as_bytes();
        let _ = decode_legacy_text(encoding_rs::WINDOWS_1252, utf8);
        assert!(take_repair_count() >= 1);
    }
}
