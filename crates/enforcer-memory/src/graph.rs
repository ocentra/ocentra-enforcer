//! The in-process memory graph: nodes ingested from NDJSON memory
//! records and lesson-ledger rows, held in memory for the lifetime of a
//! query session. This is intentionally the smallest useful graph shape
//! for this slice — a flat node store plus `observedIn` edges recorded by
//! [`crate::ingest::ingest_observation`] — not the full KG/RAG surface
//! the workpack's long-range acceptance block describes (code symbols,
//! HNSW sidecars, weaver workers, embeddings). Those are out of scope
//! for this pass; see `src/lib.rs` module docs for the seam boundary.

use crate::lesson::LessonRow;
use crate::observations::{ProceduralRecord, RouteTrace};
use crate::record::{MemoryRecord, RecordKind};

/// One node in the memory graph: either an ingested memory record, a
/// lesson-ledger row, or an observation recorded through the
/// usage-ingestion seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryNode {
    Record(Box<MemoryRecord>),
    Lesson(LessonRow),
    Incident(crate::ingest::Incident),
}

impl MemoryNode {
    /// Stable node id, used for dedup and `landedAt`/evidence lookups.
    pub fn id(&self) -> &str {
        match self {
            MemoryNode::Record(record) => record.id(),
            MemoryNode::Lesson(lesson) => &lesson.id,
            MemoryNode::Incident(incident) => &incident.id,
        }
    }

    /// Text exposed to the deterministic keyword recall matcher.
    pub fn searchable_text(&self) -> String {
        match self {
            MemoryNode::Record(record) => record.searchable_text(),
            MemoryNode::Lesson(lesson) => lesson.searchable_text(),
            MemoryNode::Incident(incident) => incident.searchable_text(),
        }
    }
}

/// The in-process graph. Append-only by convention: nodes are added via
/// [`MemoryGraph::ingest_record`] / [`MemoryGraph::ingest_lesson_row`] /
/// [`MemoryGraph::ingest_incident`] and never mutated in place.
///
/// Procedural-memory ([`ProceduralRecord`]) and meta-memory
/// ([`RouteTrace`]) entries (X06.6) are kept in their own append-only
/// vecs rather than folded into the `MemoryNode` enum: they are not
/// recall-searchable text nodes (nothing in `crate::recall` should ever
/// match a route-choice trace as if it were a lesson), they are a
/// separate specialized-memory kind per the owner intent's MIA-derived
/// hierarchy (D-10) -- kept alongside the node store so a single graph
/// value is still the one thing callers construct and query.
#[derive(Debug, Clone, Default)]
pub struct MemoryGraph {
    nodes: Vec<MemoryNode>,
    procedural_records: Vec<ProceduralRecord>,
    route_traces: Vec<RouteTrace>,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest_record(&mut self, record: impl Into<MemoryRecord>) {
        self.nodes.push(MemoryNode::Record(Box::new(record.into())));
    }

    pub fn ingest_lesson_row(&mut self, row: LessonRow) {
        self.nodes.push(MemoryNode::Lesson(row));
    }

    pub fn ingest_incident(&mut self, incident: crate::ingest::Incident) {
        self.nodes.push(MemoryNode::Incident(incident));
    }

    /// Record one procedural-memory outcome (X06.6). See
    /// [`crate::observations::record_procedural`] for the ergonomic
    /// entry point; this is the raw storage primitive it calls.
    pub fn ingest_procedural(&mut self, record: ProceduralRecord) {
        self.procedural_records.push(record);
    }

    /// Record one meta-memory route-choice trace (X06.6). See
    /// [`crate::observations::record_route_choice`] for the ergonomic
    /// entry point; this is the raw storage primitive it calls.
    pub fn ingest_route_trace(&mut self, trace: RouteTrace) {
        self.route_traces.push(trace);
    }

    /// All procedural-memory records recorded so far, in insertion order.
    pub fn procedural_records(&self) -> &[ProceduralRecord] {
        &self.procedural_records
    }

    /// All meta-memory route-choice traces recorded so far, in
    /// insertion order.
    pub fn route_traces(&self) -> &[RouteTrace] {
        &self.route_traces
    }

    pub fn nodes(&self) -> &[MemoryNode] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Every `lesson`-kind memory record plus every ledger row — the
    /// domain [`crate::recall::evidence`] walks.
    pub fn lesson_like_nodes(&self) -> impl Iterator<Item = &MemoryNode> {
        self.nodes.iter().filter(|node| match node {
            MemoryNode::Lesson(_) => true,
            MemoryNode::Record(record) => matches!(record.kind(), RecordKind::Lesson),
            MemoryNode::Incident(_) => false,
        })
    }

    /// All incidents whose `landed_at` or free text references `lesson_id`.
    pub fn incidents_for_lesson(&self, lesson_id: &str) -> Vec<&crate::ingest::Incident> {
        self.nodes
            .iter()
            .filter_map(|node| match node {
                MemoryNode::Incident(incident) if incident.lesson_id == lesson_id => Some(incident),
                _ => None,
            })
            .collect()
    }
}
