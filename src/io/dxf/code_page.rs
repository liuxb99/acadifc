//! DXF code page ($DWGCODEPAGE) to encoding mapping.
//!
//! Maps DXF code page names to `encoding_rs` encodings, following the same
//! mapping table used by ACadSharp's `CadUtils._dxfEncodingMap`.

use encoding_rs::Encoding;

/// Get the `encoding_rs` encoding for a DXF code page string.
///
/// Returns `None` if the encoding is UTF-8 (no transcoding needed) or the
/// code page string is not recognized.
///
/// # Rules
/// - If the DXF version is AC1021 (AutoCAD 2007+) or later, UTF-8 is always
///   used regardless of $DWGCODEPAGE — callers should not call this function.
/// - Otherwise, the code page string (case-insensitive) is looked up in the
///   mapping table.
pub fn encoding_from_code_page(code_page: &str) -> Option<&'static Encoding> {
    match code_page.to_ascii_lowercase().as_str() {
        // Asian encodings
        "gb2312" | "ansi_936" => Some(encoding_rs::GBK),
        "big5" | "ansi_950" => Some(encoding_rs::BIG5),
        "korean" | "ansi_949" | "johab" => Some(encoding_rs::EUC_KR),
        "ansi_932" => Some(encoding_rs::SHIFT_JIS),

        // DOS/OEM code pages
        "dos437" => Some(encoding_rs::IBM866), // closest available in encoding_rs
        "dos850" => Some(encoding_rs::WINDOWS_1252), // Western European
        "dos852" => Some(encoding_rs::WINDOWS_1250), // Central European
        "dos855" | "dos866" => Some(encoding_rs::IBM866), // Cyrillic
        "dos857" => Some(encoding_rs::WINDOWS_1254), // Turkish
        "dos860" => Some(encoding_rs::WINDOWS_1252), // Portuguese
        "dos861" => Some(encoding_rs::WINDOWS_1252), // Icelandic
        "dos863" => Some(encoding_rs::WINDOWS_1252), // Canadian-French
        "dos865" => Some(encoding_rs::WINDOWS_1252), // Nordic
        "dos869" => Some(encoding_rs::WINDOWS_1253), // Greek

        // Windows/ANSI code pages
        "ansi_874" => Some(encoding_rs::WINDOWS_874),
        "ansi_1250" => Some(encoding_rs::WINDOWS_1250),
        "ansi_1251" => Some(encoding_rs::WINDOWS_1251),
        "ansi_1252" => Some(encoding_rs::WINDOWS_1252),
        "ansi_1253" => Some(encoding_rs::WINDOWS_1253),
        "ansi_1254" => Some(encoding_rs::WINDOWS_1254),
        "ansi_1255" => Some(encoding_rs::WINDOWS_1255),
        "ansi_1256" => Some(encoding_rs::WINDOWS_1256),
        "ansi_1257" => Some(encoding_rs::WINDOWS_1257),
        "ansi_1258" => Some(encoding_rs::WINDOWS_1258),

        // ISO encodings
        "iso8859-1" | "iso_8859-1" => Some(encoding_rs::WINDOWS_1252),
        "iso8859-2" | "iso_8859-2" => Some(encoding_rs::ISO_8859_2),
        "iso8859-3" | "iso_8859-3" => Some(encoding_rs::ISO_8859_3),
        "iso8859-4" | "iso_8859-4" => Some(encoding_rs::ISO_8859_4),
        "iso8859-5" | "iso_8859-5" => Some(encoding_rs::ISO_8859_5),
        "iso8859-6" | "iso_8859-6" => Some(encoding_rs::ISO_8859_6),
        "iso8859-7" | "iso_8859-7" => Some(encoding_rs::ISO_8859_7),
        "iso8859-8" | "iso_8859-8" => Some(encoding_rs::ISO_8859_8),
        "iso8859-9" | "iso_8859-9" => Some(encoding_rs::WINDOWS_1254),
        "iso8859-10" | "iso_8859-10" => Some(encoding_rs::ISO_8859_10),
        "iso8859-13" | "iso_8859-13" => Some(encoding_rs::ISO_8859_13),
        "iso8859-14" | "iso_8859-14" => Some(encoding_rs::ISO_8859_14),
        "iso8859-15" | "iso_8859-15" => Some(encoding_rs::ISO_8859_15),

        // KOI8-R (Russian)
        "koi8-r" => Some(encoding_rs::KOI8_R),
        "koi8-u" => Some(encoding_rs::KOI8_U),

        // ASCII / UTF-8 / no fallback needed
        "ascii" | "utf-8" | "utf8" | "unicode" => None,

        // Default: Windows-1252 (most common DXF fallback)
        _ => Some(encoding_rs::WINDOWS_1252),
    }
}

/// Resolve the compact code-page index stored in DWG file metadata.
pub fn dwg_code_page_name(index: u16) -> &'static str {
    match index {
        1 => "ASCII",
        2 => "ISO8859-1",
        3 => "ISO8859-2",
        4 => "ISO8859-3",
        5 => "ISO8859-4",
        6 => "ISO8859-5",
        7 => "ISO8859-6",
        8 => "ISO8859-7",
        9 => "ISO8859-8",
        10 => "ISO8859-9",
        11 => "DOS437",
        12 => "DOS850",
        13 => "DOS852",
        14 => "DOS855",
        15 => "DOS857",
        16 => "DOS860",
        17 => "DOS861",
        18 => "DOS863",
        19 => "DOS864",
        20 => "DOS865",
        21 => "DOS869",
        22 | 38 => "ANSI_932",
        23 => "MAC-ROMAN",
        24 | 41 => "BIG5",
        25 | 40 => "KOREAN",
        26 | 42 => "JOHAB",
        27 => "DOS866",
        28 => "ANSI_1250",
        29 => "ANSI_1251",
        30 => "ANSI_1252",
        31 | 39 => "GB2312",
        32 => "ANSI_1253",
        33 => "ANSI_1254",
        34 => "ANSI_1255",
        35 => "ANSI_1256",
        36 => "ANSI_1257",
        37 => "ANSI_874",
        43 => "UTF-8",
        44 => "ANSI_1258",
        _ => "ANSI_1252",
    }
}

/// True when the document contains CJK text in a user-visible string.
///
/// Used to avoid writing Chinese content into an ANSI_1252 document: AutoCAD
/// 2006 cannot render those bytes and reports them as `\\U+` escapes, and a
/// font name damaged that way makes SHLOAD abort.
pub fn document_has_cjk(document: &crate::document::CadDocument) -> bool {
    use crate::io::dwg::legacy_text::has_cjk;
    if has_cjk(&document.header.code_page) {
        return false;
    }
    let style_strings = document
        .text_styles
        .iter()
        .any(|s| has_cjk(&s.name) || has_cjk(&s.font_file) || has_cjk(&s.big_font_file));
    if style_strings {
        return true;
    }
    if document.layers.iter().any(|l| has_cjk(&l.name)) {
        return true;
    }
    if document.block_records.iter().any(|b| has_cjk(&b.name)) {
        return true;
    }
    document.entities().any(|e| {
        let common = e.common();
        has_cjk(&common.layer)
            || match e {
                crate::entities::EntityType::Text(t) => has_cjk(&t.value) || has_cjk(&t.style),
                crate::entities::EntityType::MText(t) => has_cjk(&t.value) || has_cjk(&t.style),
                _ => false,
            }
    })
}

/// The code page a writer must use for legacy strings.
///
/// Normalises the DOS-era cp ids AutoCAD never writes (CP_BIG5 24, cp932 22,
/// cp949 25, johab 26, gb2312 31) to their ANSI equivalents, and promotes an
/// ANSI_1252 document to BIG5 when it actually contains Chinese text.
pub fn effective_code_page(document: &crate::document::CadDocument) -> (String, u16) {
    let mut name = document.header.code_page.clone();
    let mut index = document
        .dwg_code_page_index
        .unwrap_or_else(|| dwg_code_page_index(&name));
    index = match index {
        22 => 38,
        24 => 41,
        25 => 40,
        26 => 42,
        31 => 39,
        other => other,
    };
    if index == 41 && name.eq_ignore_ascii_case("ansi_1252") {
        name = "BIG5".to_string();
    }
    if (name.eq_ignore_ascii_case("ansi_1252") || index == 30) && document_has_cjk(document) {
        name = "BIG5".to_string();
        index = 41;
    }
    (name, index)
}

/// Convert a DXF `$DWGCODEPAGE` name to the compact DWG metadata index.
pub fn dwg_code_page_index(code_page: &str) -> u16 {
    match code_page.to_ascii_lowercase().as_str() {
        "ascii" => 1,
        "iso8859-1" | "iso_8859-1" => 2,
        "iso8859-2" | "iso_8859-2" => 3,
        "iso8859-3" | "iso_8859-3" => 4,
        "iso8859-4" | "iso_8859-4" => 5,
        "iso8859-5" | "iso_8859-5" => 6,
        "iso8859-6" | "iso_8859-6" => 7,
        "iso8859-7" | "iso_8859-7" => 8,
        "iso8859-8" | "iso_8859-8" => 9,
        "iso8859-9" | "iso_8859-9" => 10,
        "dos437" => 11,
        "dos850" => 12,
        "dos852" => 13,
        "dos855" => 14,
        "dos857" => 15,
        "dos860" => 16,
        "dos861" => 17,
        "dos863" => 18,
        "dos864" => 19,
        "dos865" => 20,
        "dos869" => 21,
        "dos932" => 22,
        // ANSI_932/936/949/950 are distinct DWG ids from the DOS-era
        // CP932/GB2312/CP949/CP_BIG5 entries. AutoCAD writes the ANSI ids.
        "ansi_932" => 38,
        "mac-roman" => 23,
        "big5" | "dos950" | "cp_big5" => 24,
        "ansi_950" => 41,
        "korean" => 25,
        "ansi_949" => 40,
        "johab" => 26,
        "dos866" => 27,
        "ansi_1250" | "ansi1250" => 28,
        "ansi_1251" | "ansi1251" => 29,
        "ansi_1252" | "ansi1252" => 30,
        "gb2312" => 31,
        "ansi_936" => 39,
        "ansi_1253" | "ansi1253" => 32,
        "ansi_1254" | "ansi1254" => 33,
        "ansi_1255" | "ansi1255" => 34,
        "ansi_1256" | "ansi1256" => 35,
        "ansi_1257" | "ansi1257" => 36,
        "ansi_874" => 37,
        "utf-8" | "utf8" | "unicode" => 43,
        "ansi_1258" | "ansi1258" => 44,
        _ => 30,
    }
}

pub fn encoding_from_dwg_code_page(index: u16) -> &'static Encoding {
    encoding_from_code_page(dwg_code_page_name(index))
        .unwrap_or(encoding_rs::WINDOWS_1252)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_1252() {
        let enc = encoding_from_code_page("ANSI_1252");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(
            encoding_from_code_page("ansi_1251"),
            encoding_from_code_page("ANSI_1251")
        );
    }

    #[test]
    fn test_ascii_returns_none() {
        assert_eq!(encoding_from_code_page("ASCII"), None);
    }

    #[test]
    fn test_utf8_returns_none() {
        assert_eq!(encoding_from_code_page("UTF-8"), None);
    }

    #[test]
    fn test_unknown_returns_windows1252() {
        let enc = encoding_from_code_page("SOMETHING_UNKNOWN");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_asian_encodings() {
        assert_eq!(encoding_from_code_page("GB2312"), Some(encoding_rs::GBK));
        assert_eq!(encoding_from_code_page("BIG5"), Some(encoding_rs::BIG5));
        assert_eq!(encoding_from_code_page("ANSI_932"), Some(encoding_rs::SHIFT_JIS));
        assert_eq!(encoding_from_code_page("KOREAN"), Some(encoding_rs::EUC_KR));
    }
}


/// Single source of truth for every string conversion in one write.
///
/// Phase 17 review: the writer used to recompute the "effective" code page in
/// the file header, the header section, the classes section and the object
/// writer.  A single context makes the source encoding, target encoding, DWG
/// code page id and file version explicit and impossible to disagree.
#[derive(Debug, Clone)]
pub struct EncodingContext {
    /// DXF version being written (AC1015/AC1018/...).
    pub version: crate::types::DxfVersion,
    /// DWG code page id stored at file-header 0x13.
    pub codepage_id: u16,
    /// Code page name used for diagnostics.
    pub codepage_name: String,
    /// Encoding the source drawing was decoded with (None for new documents).
    pub source_encoding: Option<&'static encoding_rs::Encoding>,
    /// Encoding every legacy string must be written with.
    pub target_encoding: &'static encoding_rs::Encoding,
    /// True when source and target code page differ, so raw replay of any
    /// string-bearing record is unsafe (it would carry source bytes).
    pub transcoding: bool,
    /// Source DWG code page id (0 when unknown/new document).
    pub source_codepage_id: u16,
}

impl EncodingContext {
    pub fn new(document: &crate::document::CadDocument) -> Self {
        let (name, id) = effective_code_page(document);
        let source_id = document.dwg_code_page_index.unwrap_or(0);
        let source_encoding = if source_id != 0 {
            encoding_from_code_page(&crate::io::dxf::code_page::dwg_code_page_name(source_id))
        } else {
            encoding_from_code_page(&document.header.code_page)
        };
        let target_encoding = encoding_from_code_page(&name)
            .unwrap_or(encoding_rs::WINDOWS_1252);
        // Phase 18: `transcoding` must mean "the byte representation changes",
        // not "the numeric id changes".  DOS-era ids and their ANSI successors
        // decode to the SAME encoding (24/41 -> BIG5, 22/38 -> ANSI_932,
        // 25/40 -> KOREAN, 26/42 -> JOHAB, 31/39 -> GB2312), so replaying raw
        // source bytes is byte-identical and safe.  Comparing ids alone made a
        // no-op save of a cp24 drawing fail closed ("raw-only object cannot be
        // transcoded 24 -> 41") even though nothing needed re-encoding.
        let source_matches_target = match source_encoding {
            Some(src) => src.name() == target_encoding.name(),
            None => false,
        };
        let transcoding = source_id != 0 && source_id != id && !source_matches_target;
        Self {
            version: document.version,
            codepage_id: id,
            codepage_name: name,
            source_encoding,
            target_encoding,
            transcoding,
            source_codepage_id: source_id,
        }
    }
}
