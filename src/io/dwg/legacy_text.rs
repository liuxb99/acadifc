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

/// Encode an ordinary run.  Unencodable characters become `?` instead of the
/// HTML entity `&#NNN;` that `encoding_rs` would otherwise emit verbatim.
fn encode_plain(encoding: &'static Encoding, text: &str) -> Vec<u8> {
    let (encoded, _, had_errors) = encoding.encode(text);
    if !had_errors {
        return encoded.into_owned();
    }
    let mut safe = String::with_capacity(text.len());
    for ch in text.chars() {
        let (_, _, err) = encoding.encode(&ch.to_string());
        safe.push(if err { '?' } else { ch });
    }
    encoding.encode(&safe).0.into_owned()
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

}
