//! The in-process memory graph: nodes ingested from NDJSON memory
//! records and lesson-ledger rows, held in memory for the lifetime of a
//! query session. This is intentionally the smallest useful graph shape
//! for this slice — a flat node store plus `observedIn` edges recorded by
//! [`crate::ingest::ingest_observation`] — not the full KG/RAG surface
//! the workpack's long-range acceptance block describes (code symbols,
//! HNSW sidecars, weaver workers, embeddings). Those are out of scope
//! for this pass; see `src/lib.rs` module docs for the seam boundary.

use crate::lesson::LessonRow;
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
            MemoryNode::Record(record) => &record.id,
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
#[derive(Debug, Clone, Default)]
pub struct MemoryGraph {
    nodes: Vec<MemoryNode>,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest_record(&mut self, record: MemoryRecord) {
        self.nodes.push(MemoryNode::Record(Box::new(record)));
    }

    pub fn ingest_lesson_row(&mut self, row: LessonRow) {
        self.nodes.push(MemoryNode::Lesson(row));
    }

    pub fn ingest_incident(&mut self, incident: crate::ingest::Incident) {
        self.nodes.push(MemoryNode::Incident(incident));
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
            MemoryNode::Record(record) => matches!(record.kind, RecordKind::Lesson),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Provenance, RecordDomain};

    fn sample_record(id: &str) -> MemoryRecord {
        MemoryRecord {
            schema_version: 1,
            id: id.to_string(),
            ts: "2026-07-04T00:00:00Z".to_string(),
            kind: RecordKind::Lesson,
            domain: RecordDomain::Harness,
            statement: "sample statement".to_string(),
            why: None,
            how_to_apply: None,
            applies_to: vec![],
            evidence: None,
            routes: vec![],
            landed_at: vec![],
            supersedes: None,
            provenance: Provenance {
                writer: "primary".to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn ingest_and_lookup_by_id() {
        let mut graph = MemoryGraph::new();
        graph.ingest_record(sample_record("mem-primary-0001"));
        assert_eq!(graph.len(), 1);
        assert_eq!(graph.nodes()[0].id(), "mem-primary-0001");
    }

    #[test]
    fn empty_graph_reports_empty() {
        let graph = MemoryGraph::new();
        assert!(graph.is_empty());
    }
}
