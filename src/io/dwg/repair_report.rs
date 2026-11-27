//! Phase 17 item 3: per-document, explicit RepairReport.
//!
//! UTF-8-in-legacy-field repair is a *repair mode*, not a silent default. Every
//! repair records the original bytes, the produced text and whether the change
//! is reversible (the original bytes are always kept in the entry, so a caller
//! can veto or reverse it). `begin_document()` isolates reports between files;
//! `snapshot()` is what reporters/doctor expose.

use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RepairEntry {
    pub raw_bytes_hex: String,
    pub repaired_text: String,
    pub reversible: bool,
}

static REPORT: Mutex<Vec<RepairEntry>> = Mutex::new(Vec::new());

pub fn begin_document() {
    if let Ok(mut entries) = REPORT.lock() {
        entries.clear();
    }
}

pub fn record(bytes: &[u8], text: &str) {
    if let Ok(mut entries) = REPORT.lock() {
        entries.push(RepairEntry {
            raw_bytes_hex: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            repaired_text: text.to_string(),
            // The original bytes are stored verbatim: the caller can always
            // write them back, so the repair is reversible by construction.
            reversible: true,
        });
    }
}

pub fn snapshot() -> Vec<RepairEntry> {
    REPORT.lock().map(|e| e.clone()).unwrap_or_default()
}
