//! The local-first, zero-network recall/evidence query surface.
//!
//! `recall` is a deterministic keyword matcher over node text — no
//! embeddings, no model download, no network call. It is the default
//! (and, in this slice, only) retriever; an embedding-backed retriever
//! can be added later behind the `embeddings` feature (see
//! [`crate::retriever`]) without changing this module's public contract.
//!
//! `evidence` answers the workpack's learning-evidence requirement: the
//! t0 (observed) -> t1 (landed) -> t2 (recurrence) chain for a lesson
//! id, fail-closed when provenance is missing rather than fabricating a
//! chain.

use crate::graph::{MemoryGraph, MemoryNode};
use crate::ingest::Incident;

/// One recall hit: the matched node plus which query tokens matched, so
/// callers can show a "why selected" trace without needing a ranking
/// score they can't audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallHit<'a> {
    pub node: &'a MemoryNode,
    pub matched_tokens: Vec<String>,
}

/// Lowercase, split on non-alphanumeric boundaries, drop empty tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

/// Deterministic keyword recall: a node matches when at least one query
/// token appears as a substring-tokenized match in the node's
/// searchable text. Returns hits in graph insertion order (stable,
/// reproducible — no similarity-score ties to break arbitrarily).
///
/// The anti-vacuous contract: a query whose tokens match nothing returns
/// an EMPTY vec, never "everything" — there is no fallback-to-all-nodes
/// path in this function.
pub fn recall<'a>(graph: &'a MemoryGraph, query: &str) -> Vec<RecallHit<'a>> {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    for node in graph.nodes() {
        let node_tokens = tokenize(&node.searchable_text());
        let matched: Vec<String> = query_tokens
            .iter()
            .filter(|q| node_tokens.iter().any(|n| n == *q))
            .cloned()
            .collect();
        if !matched.is_empty() {
            hits.push(RecallHit {
                node,
                matched_tokens: matched,
            });
        }
    }
    hits
}

/// One step of a learning-evidence chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceStep<'a> {
    /// t0: an incident observed for this lesson (with provenance —
    /// the source surface and timestamp that recorded it).
    Observed(&'a Incident),
    /// t1: a durable artifact this lesson has landed in
    /// (`landedAt`/ledger `landed-at` cell).
    Landed(String),
}

/// The evidence chain result for `memory evidence <lessonId>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceResult<'a> {
    /// Full or partial chain, with a flag for whether t0 provenance was
    /// actually found (fail-closed signal for the caller).
    Chain {
        lesson_id: String,
        steps: Vec<EvidenceStep<'a>>,
        /// t2: how many incidents recorded AFTER at least one landed
        /// artifact exists — the recurrence count since landing.
        recurrence_since_landing: usize,
        /// `false` when no t0 observation with provenance was found for
        /// this lesson — the caller must report `evidence:incomplete`,
        /// never fabricate a chain.
        has_t0_provenance: bool,
    },
    /// No node in the graph is known under this lesson id at all.
    Unknown { lesson_id: String },
}

/// Walk the t0 (observed) -> t1 (landed) -> t2 (recurrence) chain for
/// `lesson_id`. Fail-closed: a lesson with no matching node at all is
/// `Unknown`; a lesson found but with no incident provenance still
/// returns a `Chain` with `has_t0_provenance = false` so the caller can
/// report `evidence:incomplete` rather than treating an empty chain as
/// "nothing to report".
pub fn evidence<'a>(graph: &'a MemoryGraph, lesson_id: &str) -> EvidenceResult<'a> {
    let landed_at = graph.nodes().iter().find_map(|node| match node {
        MemoryNode::Lesson(row) if row.id == lesson_id => Some(row.landed_at.clone()),
        MemoryNode::Record(record) if record.id == lesson_id => record.landed_at.first().cloned(),
        _ => None,
    });

    let incidents = graph.incidents_for_lesson(lesson_id);

    if landed_at.is_none() && incidents.is_empty() {
        return EvidenceResult::Unknown {
            lesson_id: lesson_id.to_string(),
        };
    }

    let has_t0_provenance = !incidents.is_empty();
    let mut steps: Vec<EvidenceStep<'a>> = incidents
        .iter()
        .map(|inc| EvidenceStep::Observed(inc))
        .collect();

    let has_landed = landed_at.as_ref().is_some_and(|value| !value.is_empty());
    if let Some(landed) = landed_at {
        if !landed.is_empty() {
            steps.push(EvidenceStep::Landed(landed));
        }
    }

    // t2: recurrence since landing = incidents recorded once a landed
    // artifact exists. This slice has no independent landing timestamp
    // field to compare against per-incident timestamps, so recurrence is
    // counted only when both landing evidence AND at least one incident
    // exist; a lesson with no landing yet has zero "since landing" by
    // definition (fail-closed: we don't guess a landing time).
    let recurrence_since_landing = if has_landed { incidents.len() } else { 0 };

    EvidenceResult::Chain {
        lesson_id: lesson_id.to_string(),
        steps,
        recurrence_since_landing,
        has_t0_provenance,
    }
}
