//! DWG binary file format support.
//!
//! Read and write AutoCAD's native binary format.  DWG files use
//! bit-granularity encoding, version-specific data layouts, and LZ77
//! compression (R2004+).
//!
//! # Reading
//!
//! ```rust,ignore
//! use acadrust::DwgReader;
//!
//! let doc = DwgReader::from_file("drawing.dwg")?.read()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! use acadrust::DwgWriter;
//!
//! DwgWriter::write_to_file("output.dwg", &doc)?;
//! ```
//!
//! ## Supported versions
//!
//! | DWG Version | AutoCAD | File format  |
//! |-------------|---------|-------------|
//! | AC1012      | R13     | Linear      |
//! | AC1014      | R14     | Linear      |
//! | AC1015      | R2000   | Linear      |
//! | AC1018      | R2004   | Paged + LZ77 |
//! | AC1021      | R2007   | Paged + LZ77 |
//! | AC1024      | R2010   | Paged + LZ77 |
//! | AC1027      | R2013   | Paged + LZ77 |
//! | AC1032      | R2018   | Paged + LZ77 |

pub mod annotative_eed;
pub mod checksum;
pub mod compression;
pub mod compressor_ac21;
pub mod crc;
pub mod decompressor_ac18;
pub mod decompressor_ac21;
pub mod dwg21_metadata;
pub mod dwg_document_builder;
pub mod dwg_reader;
pub mod dwg_reference_type;
pub mod dwg_stream_readers;
pub mod dwg_stream_writers;
pub mod dwg_version;
pub mod dwg_writer;
pub mod eed_codec;
pub(crate) mod embedded_entity;
pub(crate) mod legacy_text;
pub(crate) mod cp950_fallback;
pub mod file_headers;
mod parallel;
pub mod preview;
pub mod reed_solomon;
pub mod raw_passthrough;

pub use dwg_reader::DwgReader;
pub use dwg_reader::DwgReadOptions;
pub use dwg_reference_type::DwgReferenceType;
pub use dwg_version::DwgVersion;
pub use dwg_writer::DwgWriter;

pub(crate) fn sab_fingerprint<'a>(
    entries: impl IntoIterator<Item = (crate::Handle, &'a [u8])>,
) -> Vec<(u64, usize, u64)> {
    use std::hash::{Hash, Hasher};

    let mut fingerprint = Vec::new();
    for (handle, bytes) in entries {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        fingerprint.push((handle.value(), bytes.len(), hasher.finish()));
    }
    fingerprint.sort_unstable_by_key(|entry| entry.0);
    fingerprint
}
pub use file_headers::{DwgFileHeaderWriterAC15, DwgFileHeaderWriterAC18};

/// Phase 14/15 diagnostic switches.
///
/// `CIVILCAD_P14_FIXES` is a comma separated allow-list of the AC1018 writer
/// fixes (`release_byte`, `app_version_bytes`, `codepage`, `aux_maintenance`,
/// `extents_preserve`, `appinfo_r18`). Unset means "all fixes on"; set to
/// `none` (or `CIVILCAD_P14_BASELINE=1`) to reproduce the pre-Phase-14 writer.
/// This exists only to build the single-variable compatibility matrix and is
/// never used by the product paths.
pub mod repair_report;

pub fn p14_fix_enabled(name: &str) -> bool {
    if std::env::var_os("CIVILCAD_P14_BASELINE").is_some() {
        return false;
    }
    match std::env::var("CIVILCAD_P14_FIXES") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            if value.is_empty() || value == "none" {
                return false;
            }
            if value == "all" {
                return true;
            }
            value.split(',').map(str::trim).any(|part| part == name)
        }
        Err(_) => true,
    }
}
