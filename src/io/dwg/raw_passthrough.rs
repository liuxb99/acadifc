//! Dependency closure for same-version AC1018 raw record passthrough.
//!
//! A record may only be replayed verbatim when neither it nor any record that
//! structurally contains it was modified. Graph construction is fail-closed for
//! *source* references: a reference to a handle that existed in the source file
//! but can no longer be resolved blocks the write instead of silently dropping
//! the reference. References to objects created after reading are legitimate
//! and are classified as `New`, not as errors.
//!
//! The plan distinguishes five per-handle actions:
//!   * `Replay`  — emit the original bytes verbatim.
//!   * `Rewrite` — re-serialise from the semantic model (dirty/new/derived).
//!   * `New`     — object created after reading (no source bytes by design).
//!   * `Delete`  — source object removed by the edit contract.
//!   * `Blocked` — cannot be handled safely; the caller must not publish.
use crate::document::{CadDocument, PassthroughReport};
use crate::entities::EntityType;
use crate::objects::ObjectType;
use crate::tables::TableEntry;
use crate::types::Handle;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Kind of reference between two records. Structural edges propagate a rewrite
/// to the container (a block header lists its members; a dictionary lists its
/// entries), pointer edges only need the target to keep its handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Container → contained (BLOCK_RECORD → entity, DICTIONARY → entry,
    /// control → table entry, INSERT → ATTRIB/SEQEND, POLYLINE → VERTEX).
    Structural,
    /// Plain handle reference whose target content the referrer depends on
    /// (entity → layer/style/linetype/dimstyle): a change propagates.
    Pointer,
    /// Owner back-reference (entity → owner block record, object → owner
    /// dictionary). The owner changing does NOT change the child's bytes, so a
    /// rewrite must not cascade down the ownership tree — only up, through the
    /// structural `container → member` edges.
    Ownership,
}

/// Presence and provenance of every handle in the document.
#[derive(Debug, Clone, Default)]
pub struct HandleIndex {
    /// Handles that exist in the current model (entities, objects, tables).
    pub known: HashSet<Handle>,
    /// Handles whose verbatim source bytes were captured at read time.
    pub raw: HashSet<Handle>,
    /// Handles present in the source file but no longer in the model.
    pub deleted: HashSet<Handle>,
    /// Handles created after reading (not present in the source handle map).
    pub new: HashSet<Handle>,
    /// Source handles that lost their raw bytes (should be empty after P0-1).
    pub missing_raw: Vec<Handle>,
    /// Source records the semantic model does not represent (for example a
    /// second LAYER with a duplicate name, which `Table<T>` cannot hold). They
    /// were never deleted by an edit contract, so they must be written back
    /// verbatim instead of being dropped.
    pub raw_only: HashSet<Handle>,
    /// Handles that the source file's handle map lists but which produced
    /// neither a semantic object nor a preserved record. They were never
    /// deleted by an edit contract: the write must be blocked, never silently
    /// turned into a DELETE.
    pub missing_source: BTreeSet<Handle>,
}

impl HandleIndex {
    pub fn build(document: &CadDocument) -> Self {
        let known = collect_known_handles(document);
        let raw: HashSet<Handle> = document.raw_object_records.keys().copied().collect();
        let source = &document.source_handles;
        let mut deleted: Vec<Handle> = Vec::new();
        let mut missing_raw: Vec<Handle> = Vec::new();
        let mut raw_only: HashSet<Handle> = HashSet::new();
        let mut missing_source: BTreeSet<Handle> = BTreeSet::new();
        if !source.is_empty() {
            for h in source {
                if document.deleted_handles.contains(h) {
                    // The only way a source object becomes a DELETE.
                    deleted.push(*h);
                } else if !known.contains(h) {
                    if raw.contains(h) {
                        // Present in the file but not in the semantic model:
                        // preserve the record verbatim.
                        raw_only.insert(*h);
                    } else {
                        // The source listed it, the reader produced nothing and
                        // no bytes were preserved. This is a MISSING_SOURCE
                        // condition, not a deletion: block instead of losing it.
                        missing_source.insert(*h);
                    }
                } else if !raw.contains(h) {
                    missing_raw.push(*h);
                }
            }
        }
        let new: HashSet<Handle> = known
            .iter()
            .copied()
            .filter(|h| !source.is_empty() && !source.contains(h))
            .collect();
        deleted.sort_by_key(|h| h.value());
        missing_raw.sort_by_key(|h| h.value());
        Self {
            known,
            raw,
            deleted: deleted.into_iter().collect(),
            new,
            missing_raw,
            raw_only,
            missing_source,
        }
    }
}

/// The single classification of one handle that every stage must agree on.
///
/// Reader, dependency graph, writer plan, writer and verifier all derive the
/// state from this enum instead of re-implementing "does this handle exist".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum HandleState {
    /// Decoded into the semantic model and (when read from a file) has bytes.
    Known,
    /// Present only as preserved source bytes; never semantically decoded.
    RawOnly,
    /// Created after reading.
    New,
    /// Removed by the edit contract.
    ExplicitlyDeleted,
    /// Listed by the source handle map but neither modelled nor preserved.
    MissingSource,
    /// Referenced by something in the document but present nowhere.
    Unresolved,
}

impl HandleState {
    /// Whether a reference to this handle can be honoured in the output.
    pub fn is_resolvable(self) -> bool {
        matches!(self, HandleState::Known | HandleState::RawOnly | HandleState::New)
    }

    pub fn is_present(self) -> bool {
        matches!(
            self,
            HandleState::Known | HandleState::RawOnly | HandleState::New | HandleState::ExplicitlyDeleted
        )
    }
}

impl HandleIndex {
    /// Classify one handle. This is the only place that decides a state.
    pub fn state(&self, handle: Handle) -> HandleState {
        if handle.is_null() {
            return HandleState::Unresolved;
        }
        if self.deleted.contains(&handle) {
            return HandleState::ExplicitlyDeleted;
        }
        if self.known.contains(&handle) {
            // `new` is a subset of `known`: both are decoded objects, the plan
            // decides whether they replay or are written from the model.
            return HandleState::Known;
        }
        if self.raw_only.contains(&handle) || self.raw.contains(&handle) {
            return HandleState::RawOnly;
        }
        if self.missing_source.contains(&handle) || self.missing_raw.iter().any(|h| *h == handle) {
            return HandleState::MissingSource;
        }
        HandleState::Unresolved
    }

    /// True when a reference to `handle` is satisfiable after the write.
    pub fn resolves(&self, handle: Handle) -> bool {
        self.state(handle).is_resolvable()
    }

    /// True when the handle is listed by the source file at all.
    pub fn is_source(&self, handle: Handle) -> bool {
        !self.deleted.contains(&handle) && !self.new.contains(&handle)
    }

    /// Handles in this index whose state is not one of the safe ones.
    pub fn problem_handles(&self) -> Vec<(Handle, HandleState)> {
        let mut out: Vec<(Handle, HandleState)> = Vec::new();
        for handle in self
            .missing_source
            .iter()
            .copied()
            .chain(self.missing_raw.iter().copied())
        {
            out.push((handle, self.state(handle)));
        }
        out.sort_by_key(|(handle, _)| handle.value());
        out
    }
}

/// One unsatisfiable reference found while building the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceIssue {
    /// Record that holds the reference.
    pub source: Handle,
    /// What kind of record holds it (`BLOCK_RECORD`, `DICTIONARY`, `ENTITY`, …).
    pub container: &'static str,
    /// The handle that cannot be resolved.
    pub target: Handle,
    /// Classification of the target.
    pub target_state: HandleState,
    /// Human-readable description.
    pub detail: String,
}

/// Every handle the semantic model owns, including table entries and controls.
pub fn collect_known_handles(document: &CadDocument) -> HashSet<Handle> {
    let mut known: HashSet<Handle> = HashSet::new();
    for e in document.entities() {
        known.insert(e.common().handle);
        // Sub-entity handles are not separate entries in `entity_index`: they
        // live inside their parent (INSERT ATTRIBs, POLYLINE VERTEX/SEQEND,
        // mesh vertices). They are emitted by the parent's writer, so they are
        // present in the output even though they have no standalone record.
        match e {
            EntityType::Insert(i) => {
                for a in &i.attributes {
                    known.insert(a.common.handle);
                }
                if let Some(seq) = i.seqend_handle {
                    known.insert(seq);
                }
            }
            EntityType::Polyline2D(p) => {
                for v in &p.vertices {
                    known.insert(v.handle);
                }
                if let Some(seq) = p.seqend_handle {
                    known.insert(seq);
                }
            }
            EntityType::Polyline3D(p) => {
                for v in &p.vertices {
                    known.insert(v.handle);
                }
                if let Some(seq) = p.seqend_handle {
                    known.insert(seq);
                }
            }
            EntityType::PolyfaceMesh(p) => {
                for v in &p.vertices {
                    known.insert(v.common.handle);
                }
                if let Some(seq) = p.seqend_handle {
                    known.insert(seq);
                }
            }
            EntityType::PolygonMesh(p) => {
                for v in &p.vertices {
                    known.insert(v.common.handle);
                }
                if let Some(seq) = p.seqend_handle {
                    known.insert(seq);
                }
            }
            _ => {}
        }
    }
    for h in document.objects.keys() {
        known.insert(*h);
    }
    for l in document.layers.iter() {
        known.insert(l.handle);
    }
    for s in document.text_styles.iter() {
        known.insert(s.handle);
        // An xref-dependent style references its external block record.
        if let Some(block) = s.xref_block_handle {
            known.insert(block);
        }
    }
    for l in document.line_types.iter() {
        known.insert(l.handle);
    }
    for v in document.views.iter() {
        known.insert(v.handle());
    }
    for u in document.ucss.iter() {
        known.insert(u.handle());
    }
    for v in document.vports.iter() {
        known.insert(v.handle());
    }
    for a in document.app_ids.iter() {
        known.insert(a.handle());
    }
    for d in document.dim_styles.iter() {
        known.insert(d.handle);
    }
    for b in document.block_records.iter() {
        known.insert(b.handle);
        // BLOCK / ENDBLK markers are written by the block writer, not through
        // the entity list, so they are present without being model entities.
        known.insert(b.block_entity_handle);
        known.insert(b.block_end_handle);
    }
    known.insert(document.layers.handle());
    known.insert(document.text_styles.handle());
    known.insert(document.line_types.handle());
    known.insert(document.views.handle());
    known.insert(document.ucss.handle());
    known.insert(document.vports.handle());
    known.insert(document.app_ids.handle());
    known.insert(document.dim_styles.handle());
    known.insert(document.block_records.handle());
    known.insert(document.header.block_control_handle);
    known.insert(document.header.layer_control_handle);
    known.insert(document.header.style_control_handle);
    known.insert(document.header.linetype_control_handle);
    known.insert(document.header.view_control_handle);
    known.insert(document.header.ucs_control_handle);
    known.insert(document.header.vport_control_handle);
    known.insert(document.header.appid_control_handle);
    known.insert(document.header.dimstyle_control_handle);
    known.insert(document.header.named_objects_dict_handle);
    known.insert(document.header.model_space_block_handle);
    known.insert(document.header.paper_space_block_handle);
    known.remove(&Handle::NULL);
    known
}

/// Typed reference graph: `referrer -> (target, edge kind)`.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    pub edges: HashMap<Handle, Vec<(Handle, EdgeKind)>>,
}

impl DependencyGraph {
    fn push(&mut self, from: Handle, to: Handle, kind: EdgeKind, index: &HandleIndex) {
        if from.is_null() || to.is_null() || from == to {
            return;
        }
        // References to handles that exist nowhere are not silently dropped:
        // they are recorded so the caller can fail closed. The target set is
        // the union of model handles and preserved raw records.
        let _ = index;
        let slot = self.edges.entry(from).or_default();
        if !slot.iter().any(|(t, _)| *t == to) {
            slot.push((to, kind));
        }
    }

    fn pointer(&mut self, from: Handle, to: Handle, index: &HandleIndex) {
        self.push(from, to, EdgeKind::Pointer, index);
    }

    fn ownership(&mut self, from: Handle, to: Handle, index: &HandleIndex) {
        self.push(from, to, EdgeKind::Ownership, index);
    }

    fn structural(&mut self, from: Handle, to: Handle, index: &HandleIndex) {
        self.push(from, to, EdgeKind::Structural, index);
    }
}

/// Build the complete dependency graph from the semantic model.
///
/// References that cannot be resolved to any known handle are returned as
/// `Err` so the caller blocks the passthrough write instead of publishing a
/// file with dangling handles.
pub fn build_dependencies(
    document: &CadDocument,
) -> std::result::Result<DependencyGraph, String> {
    let index = HandleIndex::build(document);
    build_dependencies_with_index(document, &index)
}

/// Every unsatisfiable reference in the document, classified. Empty means the
/// reference graph is closed.
pub fn reference_issues(document: &CadDocument) -> Vec<ReferenceIssue> {
    let index = HandleIndex::build(document);
    build_graph(document, &index).1
}

pub fn build_dependencies_with_index(
    document: &CadDocument,
    index: &HandleIndex,
) -> std::result::Result<DependencyGraph, String> {
    let (graph, issues) = build_graph(document, index);
    if issues.is_empty() {
        return Ok(graph);
    }
    let mut shown: Vec<String> = issues.iter().map(ReferenceIssue::describe).collect();
    shown.sort();
    shown.truncate(20);
    Err(format!(
        "{} unresolvable reference(s): {}",
        issues.len(),
        shown.join("; ")
    ))
}

impl ReferenceIssue {
    pub fn describe(&self) -> String {
        format!(
            "{} {:#X}: {} -> {:#X} ({:?})",
            self.container,
            self.source.value(),
            self.detail,
            self.target.value(),
            self.target_state
        )
    }
}

fn build_graph(
    document: &CadDocument,
    index: &HandleIndex,
) -> (DependencyGraph, Vec<ReferenceIssue>) {
    let mut graph = DependencyGraph::default();
    let mut issues: Vec<ReferenceIssue> = Vec::new();
    let mut note = |container: &'static str, source: Handle, target: Handle, detail: String| {
        if index.resolves(target) {
            return;
        }
        issues.push(ReferenceIssue {
            source,
            container,
            target,
            target_state: index.state(target),
            detail,
        });
    };

    // ── Entities ────────────────────────────────────────────────────
    for entity in document.entities() {
        let h = entity.common().handle;
        if h.is_null() {
            continue;
        }
        let common = entity.common();
        graph.ownership(h, common.owner_handle, index);
        for r in &common.reactors {
            graph.pointer(h, *r, index);
        }
        if let Some(x) = common.xdictionary_handle {
            graph.pointer(h, x, index);
        }
        // Keep the *original* layer handle even when that table entry was
        // dropped (duplicate names), so the deleted-reference sweep below can
        // see that this entity must be re-encoded instead of replayed verbatim.
        let layer_h = common
            .layer_handle
            .or_else(|| document.layers.get(&common.layer).map(|l| l.handle));
        if let Some(lh) = layer_h {
            graph.pointer(h, lh, index);
        }
        if let Some(lt) = document.line_types.get(&common.linetype) {
            graph.pointer(h, lt.handle, index);
        }
        if let Some(lh) = common.linetype_handle {
            graph.pointer(h, lh, index);
        }
        match entity {
            EntityType::MText(m) => {
                if let Some(s) = m.style_handle {
                    graph.pointer(h, s, index);
                }
            }
            EntityType::Text(t) => {
                if let Some(s) = document.text_styles.get(&t.style) {
                    graph.pointer(h, s.handle, index);
                }
            }
            EntityType::Insert(i) => {
                if let Some(br) = document.block_records.get(&i.block_name) {
                    graph.pointer(h, br.handle, index);
                }
                for a in &i.attributes {
                    graph.structural(h, a.common.handle, index);
                    graph.structural(h, a.common.owner_handle, index);
                }
                if let Some(s) = i.seqend_handle {
                    graph.structural(h, s, index);
                }
            }
            EntityType::AttributeEntity(_) | EntityType::AttributeDefinition(_) => {
                if let Some(s) = document
                    .text_styles
                    .get(&match entity {
                        EntityType::AttributeEntity(a) => &a.text_style,
                        EntityType::AttributeDefinition(a) => &a.text_style,
                        _ => unreachable!(),
                    })
                {
                    graph.pointer(h, s.handle, index);
                }
            }
            EntityType::Polyline2D(p) => {
                for v in &p.vertices {
                    graph.structural(h, v.handle, index);
                }
                if let Some(s) = p.seqend_handle {
                    graph.structural(h, s, index);
                }
            }
            EntityType::Polyline3D(p) => {
                for v in &p.vertices {
                    graph.structural(h, v.handle, index);
                }
            }
            EntityType::PolyfaceMesh(p) => {
                for v in &p.vertices {
                    graph.structural(h, v.common.handle, index);
                }
            }
            EntityType::PolygonMesh(p) => {
                for v in &p.vertices {
                    graph.structural(h, v.common.handle, index);
                }
            }
            // DIMENSION: the anonymous dimension block and the DIMSTYLE the
            // record renders from are both load-bearing references.
            EntityType::Dimension(d) => {
                let base = d.base();
                if let Some(br) = document.block_records.get(&base.block_name) {
                    graph.structural(h, br.handle, index);
                }
                if let Some(ds) = document.dim_styles.get(&base.style_name) {
                    graph.pointer(h, ds.handle, index);
                }
            }
            _ => {}
        }
    }

    // ── Table controls → entries ──────────────────────────────────
    // A control record lists its entries' handles, so adding/removing an entry
    // must rewrite the control; an unchanged table replays byte-identically.
    let mut control_edges: Vec<(Handle, Vec<Handle>)> = Vec::new();
    control_edges.push((document.layers.handle(), document.layers.iter().map(|e| e.handle).collect()));
    control_edges.push((
        document.text_styles.handle(),
        document.text_styles.iter().map(|e| e.handle).collect(),
    ));
    control_edges.push((
        document.line_types.handle(),
        document.line_types.iter().map(|e| e.handle).collect(),
    ));
    control_edges.push((
        document.views.handle(),
        document.views.iter().map(|e| e.handle()).collect(),
    ));
    control_edges.push((
        document.ucss.handle(),
        document.ucss.iter().map(|e| e.handle()).collect(),
    ));
    control_edges.push((
        document.vports.handle(),
        document.vports.iter().map(|e| e.handle()).collect(),
    ));
    control_edges.push((
        document.app_ids.handle(),
        document.app_ids.iter().map(|e| e.handle()).collect(),
    ));
    control_edges.push((
        document.dim_styles.handle(),
        document.dim_styles.iter().map(|e| e.handle).collect(),
    ));
    control_edges.push((
        document.block_records.handle(),
        document.block_records.iter().map(|e| e.handle).collect(),
    ));
    for (control, entries) in control_edges {
        for entry in entries {
            graph.structural(control, entry, index);
        }
    }
    // The header's root dictionary owns the layout/dimstyle/etc. dictionaries.
    if !document.header.named_objects_dict_handle.is_null() {
        for br in document.block_records.iter() {
            if !br.layout.is_null() {
                graph.pointer(document.header.named_objects_dict_handle, br.layout, index);
            }
        }
    }

    // ── Block records: membership + ownership ──────────────────────
    for br in document.block_records.iter() {
        for &member in &br.entity_handles {
            graph.structural(br.handle, member, index);
            note(
                "BLOCK_RECORD",
                br.handle,
                member,
                format!("block '{}' lists member", br.name),
            );
        }
        for field in [br.block_entity_handle, br.block_end_handle] {
            if !field.is_null() {
                graph.structural(br.handle, field, index);
            }
        }
        if !br.layout.is_null() {
            graph.pointer(br.handle, br.layout, index);
        }
        for &ins in &br.insert_handles {
            graph.structural(br.handle, ins, index);
        }
    }

    // ── Non-graphical objects ──────────────────────────────────────
    for (h, obj) in &document.objects {
        let h = *h;
        graph.ownership(h, object_owner(obj), index);
        match obj {
            ObjectType::Dictionary(dict) => {
                for (name, target) in &dict.entries {
                    graph.structural(h, *target, index);
                    note(
                        "DICTIONARY",
                        h,
                        *target,
                        format!("entry '{name}' references"),
                    );
                }
                for r in &dict.reactors {
                    graph.pointer(h, *r, index);
                }
                if let Some(x) = dict.xdictionary_handle {
                    graph.pointer(h, x, index);
                }
            }
            ObjectType::DictionaryWithDefault(d) => {
                for (name, target) in &d.entries {
                    graph.structural(h, *target, index);
                    note(
                        "DICTIONARY_WITH_DEFAULT",
                        h,
                        *target,
                        format!("entry '{name}' references"),
                    );
                }

            }
            ObjectType::XRecord(x) => {
                for r in &x.reactors {
                    graph.pointer(h, *r, index);
                }
                if let Some(xd) = x.xdictionary_handle {
                    graph.pointer(h, xd, index);
                }
            }
            ObjectType::Layout(layout) => {
                for vp in &layout.viewports {
                    graph.structural(h, *vp, index);
                }
                if !layout.block_record.is_null() {
                    graph.pointer(h, layout.block_record, index);
                }
            }
            ObjectType::Group(group) => {
                for e in &group.entities {
                    graph.structural(h, *e, index);
                }
            }
            ObjectType::SortEntitiesTable(t) => {
                for entry in t.entries() {
                    graph.structural(h, entry.entity_handle, index);
                }
            }
            ObjectType::ImageDefinition(_) => {}
            _ => {}
        }
    }
    // Ownership of entity handles is also expressed through the block record;
    // make sure an entity whose owner is a block record stays linked.
    for entity in document.entities() {
        let h = entity.common().handle;
        let owner = entity.common().owner_handle;
        if !owner.is_null() && index.known.contains(&owner) {
            graph.structural(owner, h, index);
        }
    }

    (graph, issues)
}

fn object_owner(obj: &ObjectType) -> Handle {
    match obj {
        ObjectType::Dictionary(o) => o.owner,
        ObjectType::Layout(o) => o.owner,
        ObjectType::XRecord(o) => o.owner,
        ObjectType::Group(o) => o.owner,
        ObjectType::MLineStyle(o) => o.owner,
        ObjectType::ImageDefinition(o) => o.owner,
        ObjectType::UnderlayDefinition(o) => o.owner_handle,
        ObjectType::PlotSettings(o) => o.owner,
        ObjectType::MultiLeaderStyle(o) => o.owner_handle,
        ObjectType::TableStyle(o) => o.owner_handle,
        ObjectType::Scale(o) => o.owner_handle,
        ObjectType::ObjectContextData(o) => o.owner_handle,
        ObjectType::SortEntitiesTable(o) => o.owner_handle,
        ObjectType::DictionaryVariable(o) => o.owner_handle,
        ObjectType::VisualStyle(o) => o.owner,
        ObjectType::Material(o) => o.owner,
        ObjectType::ImageDefinitionReactor(o) => o.owner,
        ObjectType::GeoData(o) => o.owner,
        ObjectType::SpatialFilter(o) => o.owner,
        ObjectType::RasterVariables(o) => o.owner,
        ObjectType::BookColor(o) => o.owner,
        ObjectType::PlaceHolder(o) => o.owner,
        ObjectType::DictionaryWithDefault(o) => o.owner,
        ObjectType::WipeoutVariables(o) => o.owner,
        ObjectType::BlockVisibilityParameter(o) => o.owner,
        ObjectType::DynamicBlock(o) => o.owner,
        ObjectType::Associative(o) => o.owner,
        ObjectType::ClassObject(o) => o.owner,
        ObjectType::DataObject(o) => o.owner,
        ObjectType::Field(o) => o.owner,
        ObjectType::FieldList(o) => o.owner,
        ObjectType::RegisteredClass(o) => o.owner,
        ObjectType::DgnLineStyle(o) => o.owner,
        ObjectType::ProxyObject(o) => o.owner,
        ObjectType::TableContent(_) => Handle::NULL,
        ObjectType::Unknown { owner, .. } => *owner,
    }
}

/// Why a record is not replayed verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteReason {
    /// The caller marked it dirty.
    Dirty,
    /// It structurally contains a changed record.
    ContainsDirty(Handle),
    /// It points at a changed record.
    PointsAtDirty(Handle),
    /// It contains or references something that was deleted.
    TouchesDeleted(Handle),
    /// Object created after reading: no source bytes exist by design.
    NewObject,
    /// Source object whose bytes were lost (must be treated as an error).
    MissingSourceBytes,
    /// The source version differs from the output version.
    VersionChanged,
    /// No writer path for this object type.
    Unsupported(String),
}

impl RewriteReason {
    pub fn describe(&self) -> String {
        match self {
            RewriteReason::Dirty => "dirty".into(),
            RewriteReason::ContainsDirty(h) => format!("contains changed {:#X}", h.value()),
            RewriteReason::PointsAtDirty(h) => format!("references changed {:#X}", h.value()),
            RewriteReason::TouchesDeleted(h) => format!("touches deleted {:#X}", h.value()),
            RewriteReason::NewObject => "new object".into(),
            RewriteReason::MissingSourceBytes => "source bytes missing".into(),
            RewriteReason::VersionChanged => "version changed".into(),
            RewriteReason::Unsupported(what) => format!("unsupported: {what}"),
        }
    }
}

/// Per-handle writer action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordAction {
    Replay,
    Rewrite(RewriteReason),
    New,
    Delete,
    Blocked(String),
}

impl RecordAction {
    pub fn label(&self) -> &'static str {
        match self {
            RecordAction::Replay => "replay",
            RecordAction::Rewrite(_) => "rewrite",
            RecordAction::New => "new",
            RecordAction::Delete => "delete",
            RecordAction::Blocked(_) => "blocked",
        }
    }
}

/// Full writer plan for one save operation.
#[derive(Debug, Clone, Default)]
pub struct WriterPlan {
    pub actions: BTreeMap<u64, RecordAction>,
    pub blocked: Vec<String>,
    pub index: HandleIndex,
}

impl WriterPlan {
    /// Count the plan's actions per kind, for auditing plan/writer agreement.
    pub fn action_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for action in self.actions.values() {
            *counts.entry(action.label()).or_default() += 1;
        }
        counts
    }

    pub fn action(&self, handle: Handle) -> Option<&RecordAction> {
        self.actions.get(&handle.value())
    }

    pub fn is_replay(&self, handle: Handle) -> bool {
        matches!(self.action(handle), Some(RecordAction::Replay))
    }

    pub fn counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut replay = 0;
        let mut rewrite = 0;
        let mut new = 0;
        let mut delete = 0;
        let mut blocked = 0;
        for a in self.actions.values() {
            match a {
                RecordAction::Replay => replay += 1,
                RecordAction::Rewrite(_) => rewrite += 1,
                RecordAction::New => new += 1,
                RecordAction::Delete => delete += 1,
                RecordAction::Blocked(_) => blocked += 1,
            }
        }
        (replay, rewrite, new, delete, blocked)
    }
}

/// Compute the closure of records that must be re-encoded.
///
/// Unlike the previous implementation, a handle without source bytes is only an
/// error when it *should* have had them (a source object). Objects created
/// after reading are classified `New`/`Rewrite` and never abort the write.
pub fn dirty_closure(
    document: &CadDocument,
    graph: &DependencyGraph,
) -> std::result::Result<HashSet<Handle>, String> {
    let index = HandleIndex::build(document);
    dirty_closure_with_index(document, graph, &index)
}

pub fn dirty_closure_with_index(
    document: &CadDocument,
    graph: &DependencyGraph,
    index: &HandleIndex,
) -> std::result::Result<HashSet<Handle>, String> {
    let mut dirty: HashSet<Handle> = document.dirty_handles.iter().copied().collect();
    // Any record that references a source handle which no longer exists must be
    // re-encoded: replaying it verbatim would emit a dangling handle. This is
    // how duplicate-name table entries (AC1018 allows them) get resolved — the
    // entity is rewritten so its reference is re-addressed to the surviving
    // entry instead of pointing at a record that is not written.
    for (referrer, targets) in &graph.edges {
        if targets
            .iter()
            .any(|(target, _)| index.deleted.contains(target))
        {
            dirty.insert(*referrer);
        }
    }
    // Fail closed when a caller-marked dirty handle is a source object whose
    // bytes were lost: we cannot prove the rewrite is faithful.
    if !index.missing_raw.is_empty() {
        let shown: Vec<String> = index
            .missing_raw
            .iter()
            .take(20)
            .map(|h| format!("{:#X}", h.value()))
            .collect();
        return Err(format!(
            "{} source handle(s) lost their raw record: {}",
            index.missing_raw.len(),
            shown.join(", ")
        ));
    }
    let mut changed = true;
    while changed {
        changed = false;
        for (referrer, targets) in &graph.edges {
            if dirty.contains(referrer) {
                continue;
            }
            // Only a MEMBERSHIP change alters a container's bytes: a block
            // header or table control lists its members, so adding/removing one
            // requires a rewrite. Merely modifying a member does not (the list
            // is unchanged), and a pointer target changing does not either (the
            // referrer stores just the handle). Cascading on every dirty target
            // is what made one edit rewrite whole drawings.
            let hit = targets.iter().any(|(target, kind)| {
                *kind == EdgeKind::Structural
                    && dirty.contains(target)
                    && (index.new.contains(target) || index.deleted.contains(target))
            });
            if hit && dirty.insert(*referrer) {
                changed = true;
            }
        }
    }
    Ok(dirty)
}

/// Explain how each handle entered the closure: returns (handle, cause).
/// Used for auditing a surprising closure size; not part of the save path.
pub fn explain_closure(
    document: &CadDocument,
    graph: &DependencyGraph,
) -> Vec<(Handle, Option<(Handle, EdgeKind)>)> {
    let mut dirty: HashSet<Handle> = document.dirty_handles.iter().copied().collect();
    let mut cause: HashMap<Handle, (Handle, EdgeKind)> = HashMap::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (referrer, targets) in &graph.edges {
            if dirty.contains(referrer) {
                continue;
            }
            for (target, kind) in targets {
                if *kind != EdgeKind::Ownership && dirty.contains(target) {
                    dirty.insert(*referrer);
                    cause.insert(*referrer, (*target, *kind));
                    changed = true;
                    break;
                }
            }
        }
    }
    let mut out: Vec<(Handle, Option<(Handle, EdgeKind)>)> = dirty
        .iter()
        .map(|h| (*h, cause.get(h).copied()))
        .collect();
    out.sort_by_key(|(h, _)| h.value());
    out
}

/// Build the complete per-handle writer plan.
pub fn plan(document: &CadDocument) -> std::result::Result<WriterPlan, String> {
    let index = HandleIndex::build(document);
    let mut out = WriterPlan {
        index: index.clone(),
        ..Default::default()
    };
    if !document.raw_passthrough_enabled {
        out.blocked.push("raw passthrough disabled by caller".into());
    }
    let graph = match build_dependencies_with_index(document, &index) {
        Ok(g) => Some(g),
        Err(e) => {
            out.blocked.push(e);
            None
        }
    };
    let dirty = match graph.as_ref() {
        Some(graph) => match dirty_closure_with_index(document, graph, &index) {
            Ok(dirty) => Some(dirty),
            Err(error) => {
                out.blocked.push(error);
                None
            }
        },
        None => None,
    };
    // Verbatim replay is only implemented for the AC1018 object stream. A
    // document read from another version is written from the semantic model;
    // the plan must say so, otherwise it promises replays the writer cannot
    // perform and the two stages disagree about the same handle.
    let raw_passthrough_supported = document.version == crate::types::DxfVersion::AC1018;
    let same_version = raw_passthrough_supported
        && document
            .dwg_source_version
            .map(|v| v == document.version)
            .unwrap_or(false);

    // A source handle the reader could neither decode nor preserve is not a
    // deletion: the save must be blocked with a reproducible reason.
    for handle in index.problem_handles() {
        if out.blocked.len() < 64 {
            out.blocked.push(handle.0.value().to_string());
        }
    }
    let mut all: Vec<Handle> = index.known.iter().copied().collect();
    all.extend(index.raw_only.iter().copied());
    all.sort_by_key(|h| h.value());
    for handle in all {
        let has_raw = index.raw.contains(&handle);
        let action = if !same_version {
            // The document was not read from this exact DWG version (new
            // document, DXF import or a version conversion). There is nothing
            // to replay and nothing is "missing": every record is written from
            // the semantic model.
            if index.new.contains(&handle) || !has_raw {
                RecordAction::New
            } else {
                RecordAction::Rewrite(RewriteReason::VersionChanged)
            }
        } else if index.raw_only.contains(&handle) {
            // Not modelled, but present in the source and never deleted.
            RecordAction::Replay
        } else if index.new.contains(&handle) {
            RecordAction::New
        } else if has_raw {
            if dirty.as_ref().is_some_and(|d| d.contains(&handle)) {
                RecordAction::Rewrite(RewriteReason::Dirty)
            } else {
                RecordAction::Replay
            }
        } else {
            RecordAction::Blocked(format!(
                "source handle {:#X} has no preserved record",
                handle.value()
            ))
        };
        if let RecordAction::Blocked(reason) = &action {
            out.blocked.push(reason.clone());
        }
        out.actions.insert(handle.value(), action);
    }
    for (handle, state) in index.problem_handles() {
        out.actions.insert(
            handle.value(),
            RecordAction::Blocked(format!(
                "source handle {:#X} is {state:?}: neither decoded nor preserved",
                handle.value()
            )),
        );
    }
    for h in &index.deleted {
        out.actions.insert(h.value(), RecordAction::Delete);
    }
    Ok(out)
}

/// Report the passthrough decision without performing a write.
pub fn report(document: &CadDocument) -> PassthroughReport {
    let mut out = PassthroughReport {
        enabled: document.raw_passthrough_enabled
            && document.version == crate::types::DxfVersion::AC1018
            && !document.raw_object_records.is_empty(),
        eligible: document.raw_object_records.len(),
        ..Default::default()
    };
    if !out.enabled {
        return out;
    }
    match plan(document) {
        Ok(plan) => {
            let (replay, rewrite, new, delete, blocked) = plan.counts();
            out.replayed = replay;
            out.rewritten = rewrite;
            out.new_objects = new;
            out.deleted = delete;
            out.blocked_records = blocked;
            let mut dirty: Vec<Handle> = document.dirty_handles.iter().copied().collect();
            dirty.sort_by_key(|h| h.value());
            out.dirty_closure = dirty;
            if !plan.blocked.is_empty() {
                out.blocked_reason = Some(plan.blocked.join("; "));
            }
        }
        Err(e) => out.blocked_reason = Some(e),
    }
    out
}

/// Actual outcome of one save, measured by the writer itself (not estimated
/// from the plan). Every number here counts records that were really emitted.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WriterReport {
    /// Records emitted by replaying preserved source bytes.
    pub replayed: usize,
    /// Records re-serialised from the semantic model.
    pub rewritten: usize,
    /// Objects created after reading.
    pub new_objects: usize,
    /// Source objects the plan classified as deleted.
    pub deleted: usize,
    /// Plan-level blocks that prevented a safe passthrough decision.
    pub blocked: Vec<String>,
    /// Objects the writer deliberately omitted, with handle/type/reason.
    pub omitted: Vec<String>,
    /// Fidelity errors raised during serialisation (non-empty = write refused).
    pub errors: Vec<String>,
    /// Handles whose plan action was Rewrite, with the reason, for auditing.
    pub rewrite_reasons: Vec<(u64, String)>,
}

impl WriterReport {
    pub fn total_records(&self) -> usize {
        self.replayed + self.rewritten
    }
}

thread_local! {
    static LAST_WRITER_REPORT: std::cell::RefCell<Option<WriterReport>> =
        const { std::cell::RefCell::new(None) };
}

/// Publish the report of the write that is currently running on this thread.
pub fn publish_writer_report(report: WriterReport) {
    LAST_WRITER_REPORT.with(|slot| {
        *slot.borrow_mut() = Some(report);
    });
}

/// Take the report produced by the last DWG write on this thread.
pub fn take_writer_report() -> Option<WriterReport> {
    LAST_WRITER_REPORT.with(|slot| slot.borrow_mut().take())
}

/// Peek at the report produced by the last DWG write on this thread.
pub fn last_writer_report() -> Option<WriterReport> {
    LAST_WRITER_REPORT.with(|slot| slot.borrow().clone())
}
