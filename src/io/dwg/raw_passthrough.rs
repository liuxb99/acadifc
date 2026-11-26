//! Dependency closure for same-version AC1018 raw record passthrough.
//!
//! A record may only be replayed verbatim when neither it nor any record
//! referencing it was modified. Because an incomplete dependency graph would
//! silently produce dangling handles, graph construction is fail-closed: any
//! unresolvable native reference returns `Err` and the caller must refuse the
//! passthrough write rather than fall back quietly.
use crate::document::{CadDocument, PassthroughReport};
use crate::entities::EntityType;
use crate::objects::ObjectType;
use crate::types::Handle;
use std::collections::{HashMap, HashSet};

/// Build `referrer -> referenced handles` from the semantic model.
pub fn build_dependencies(document: &CadDocument) -> std::result::Result<HashMap<Handle, Vec<Handle>>, String> {
    let mut deps: HashMap<Handle, Vec<Handle>> = HashMap::new();
    let has = |h: Handle| -> std::result::Result<Handle, String> {
        if h.is_null() { return Err("NULL reference".into()); }
        if !document.raw_object_records.contains_key(&h) {
            return Err(format!("handle {:#X} has no source record", h.value()));
        }
        Ok(h)
    };
    let mut push = |from: Handle, to: Handle| {
        if !to.is_null() && document.raw_object_records.contains_key(&to) {
            deps.entry(from).or_default().push(to);
        }
    };
    for entity in document.entities() {
        let h = entity.common().handle;
        if h.is_null() { continue; }
        push(h, entity.common().owner_handle);
        for r in &entity.common().reactors { push(h, *r); }
        if let Some(x) = entity.common().xdictionary_handle { push(h, x); }
        if let Some(l) = document.layers.get(&entity.common().layer) { push(h, l.handle); }
        if let Some(l) = document.line_types.get(&entity.common().linetype) { push(h, l.handle); }
        match entity {
            EntityType::MText(m) => {
                if let Some(s) = m.style_handle { push(h, s); }
            }
            EntityType::Text(t) => {
                if let Some(s) = document.text_styles.get(&t.style) { push(h, s.handle); }
            }
            EntityType::Insert(i) => {
                if let Some(br) = document.block_records.get(&i.block_name) { push(h, br.handle); }
                for a in &i.attributes { push(h, a.common.handle); }
                if let Some(s) = i.seqend_handle { push(h, s); }
            }
            EntityType::AttributeEntity(_) | EntityType::AttributeDefinition(_) => {}
            EntityType::Polyline2D(p) => {
                for v in &p.vertices { push(h, v.handle); }
                if let Some(s) = p.seqend_handle { push(h, s); }
            }
            _ => {}
        }
        // The owner may be a table/BLOCK record that is re-encoded
        // semantically with the same handle; requiring a raw record for it
        // would reject every real drawing. Only the dirty set needs a raw
        // record (checked in `dirty_closure`).
        let _ = &has;
    }
    for (h, obj) in &document.objects {
        match obj {
            ObjectType::Unknown { owner, .. } => {
                if !owner.is_null() { push(*h, *owner); }
            }
            _ => {}
        }
    }
    Ok(deps)
}

/// Expand the caller's dirty set through reverse dependencies: any record
/// that references a changed handle must be re-encoded as well.
pub fn dirty_closure(
    document: &CadDocument,
    deps: &HashMap<Handle, Vec<Handle>>,
) -> std::result::Result<HashSet<Handle>, String> {
    let mut dirty: HashSet<Handle> = document.dirty_handles.iter().copied().collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (referrer, targets) in deps {
            if targets.iter().any(|t| dirty.contains(t)) && dirty.insert(*referrer) {
                changed = true;
            }
        }
    }
    for h in &dirty {
        if !document.raw_object_records.contains_key(h) {
            return Err(format!("dirty handle {:#X} has no source record", h.value()));
        }
    }
    Ok(dirty)
}

/// Report the passthrough decision without performing a write. Useful for
/// callers that need to prove which records would be replayed verbatim.
pub fn report(document: &CadDocument) -> PassthroughReport {
    let mut out = PassthroughReport {
        enabled: document.raw_passthrough_enabled
            && document.version == crate::types::DxfVersion::AC1018
            && !document.raw_object_records.is_empty(),
        eligible: document.raw_object_records.len(),
        ..Default::default()
    };
    if !out.enabled { return out; }
    match build_dependencies(document)
        .and_then(|deps| dirty_closure(document, &deps))
    {
        Ok(dirty) => {
            out.dirty_closure = dirty.iter().map(|h| *h).collect();
            out.dirty_closure.sort_by_key(|h| h.value());
            out.rewritten = dirty.len();
            out.replayed = out.eligible.saturating_sub(out.rewritten);
        }
        Err(e) => out.blocked_reason = Some(e),
    }
    out
}
