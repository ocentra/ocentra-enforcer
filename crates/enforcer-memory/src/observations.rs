//! X06.6: procedural memory and meta-memory observation records.
//!
//! [`crate::ingest::ingest_observation`] already covers the workpack's
//! "every scan/check/run/doctor/closeout writes an observation" and
//! "clean scans write negative evidence" requirements via [`Incident`]
//! nodes. This module adds the other two hard requirements the owner
//! intent's MIA-derived memory hierarchy (D-10) calls for:
//!
//! - **procedural memory** ([`ProceduralOutcome`]/[`record_procedural`]):
//!   did applying a lesson's fix/retrieval actually work THIS time --
//!   success and failure both recorded, because a memory system that
//!   only logs successes cannot tell "this fix reliably works" from
//!   "this fix has never been tried again";
//! - **meta-memory** ([`RouteTrace`]/[`record_route_choice`]): which
//!   retrieval route a query took and how confident that choice was --
//!   the "did retrieval improve" self-evaluation the owner intent
//!   describes, kept as plain structured data here (no learned
//!   scoring model in this slice) so it is deterministic and testable.
//!
//! Both record types live alongside [`Incident`] in [`MemoryGraph`]
//! rather than forking a second graph, following the same append-only,
//! never-mutate-in-place discipline: outcomes and route traces are
//! FACTS ABOUT PAST EVENTS, never edited once recorded.

use crate::graph::MemoryGraph;

/// One procedural-memory record: the outcome of attempting to apply a
/// lesson's fix or retrieval guidance. Both success AND failure are
/// first-class -- a procedural memory that only ever records success
/// cannot distinguish "this always works" from "this was only tried
/// once and got lucky".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralRecord {
    pub id: String,
    pub lesson_id: String,
    pub outcome: ProceduralOutcome,
    /// Free-text detail: what was attempted (e.g. "applied fix from
    /// mem-a-0001: return existing identity on re-init").
    pub detail: String,
    pub ts: String,
}

/// Whether applying a lesson's guidance succeeded or failed this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralOutcome {
    RetrievalSuccess,
    RetrievalFailure,
    FixSuccess,
    FixFailure,
}

impl ProceduralOutcome {
    pub fn is_success(self) -> bool {
        matches!(
            self,
            ProceduralOutcome::RetrievalSuccess | ProceduralOutcome::FixSuccess
        )
    }

    fn as_str(self) -> &'static str {
        match self {
            ProceduralOutcome::RetrievalSuccess => "retrieval-success",
            ProceduralOutcome::RetrievalFailure => "retrieval-failure",
            ProceduralOutcome::FixSuccess => "fix-success",
            ProceduralOutcome::FixFailure => "fix-failure",
        }
    }
}

impl ProceduralRecord {
    pub fn searchable_text(&self) -> String {
        format!(
            "{} {} {}",
            self.lesson_id,
            self.outcome.as_str(),
            self.detail
        )
    }
}

/// One meta-memory record: which retrieval route a query took, and how
/// confident that route selection was. This is the "did the router pick
/// the right memory" self-evaluation signal -- kept as plain recorded
/// data (never inferred after the fact) so a later audit of "should
/// this query have used a different route" has ground truth to compare
/// against.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteTrace {
    pub id: String,
    pub query: String,
    /// Which retrieval route answered this query, e.g. `"recall"`,
    /// `"evidence"`, `"code_graph"` -- free text naming the module/query
    /// path actually taken, not a closed enum, because the set of
    /// routes grows as later x06 subpacks (X06.4 retriever, X06.3 graph
    /// algorithms) add more query surfaces this crate cannot enumerate
    /// today.
    pub route: String,
    /// Confidence in `[0.0, 1.0]`. Not a probability calibrated against
    /// any model -- this slice has no learned scorer -- but a
    /// deterministic signal the caller supplies (e.g. "1.0 if recall
    /// returned a non-empty hit set, 0.0 otherwise") so route-choice
    /// quality is at least comparable across queries.
    pub confidence: f64,
    pub ts: String,
}

impl RouteTrace {
    pub fn searchable_text(&self) -> String {
        format!("{} {}", self.query, self.route)
    }
}

/// Record one procedural-memory outcome into `graph`. Returns the
/// assigned id.
pub fn record_procedural(
    graph: &mut MemoryGraph,
    lesson_id: impl Into<String>,
    outcome: ProceduralOutcome,
    detail: impl Into<String>,
    ts: impl Into<String>,
) -> String {
    let id = format!("proc-{:04}", graph.procedural_records().len());
    let record = ProceduralRecord {
        id: id.clone(),
        lesson_id: lesson_id.into(),
        outcome,
        detail: detail.into(),
        ts: ts.into(),
    };
    graph.ingest_procedural(record);
    id
}

/// Record one meta-memory route-choice trace into `graph`. `confidence`
/// is clamped into `[0.0, 1.0]` -- a caller-supplied value outside that
/// range is a caller bug, not grounds to silently store a
/// nonsensical confidence.
pub fn record_route_choice(
    graph: &mut MemoryGraph,
    query: impl Into<String>,
    route: impl Into<String>,
    confidence: f64,
    ts: impl Into<String>,
) -> String {
    let id = format!("route-{:04}", graph.route_traces().len());
    let trace = RouteTrace {
        id: id.clone(),
        query: query.into(),
        route: route.into(),
        confidence: confidence.clamp(0.0, 1.0),
        ts: ts.into(),
    };
    graph.ingest_route_trace(trace);
    id
}

/// Success rate (successes / total) for a lesson's procedural history.
/// `None` when no procedural record exists yet for this lesson --
/// distinct from `Some(0.0)` (tried and always failed).
pub fn procedural_success_rate(graph: &MemoryGraph, lesson_id: &str) -> Option<f64> {
    let records: Vec<&ProceduralRecord> = graph
        .procedural_records()
        .iter()
        .filter(|r| r.lesson_id == lesson_id)
        .collect();
    if records.is_empty() {
        return None;
    }
    let successes = records.iter().filter(|r| r.outcome.is_success()).count();
    Some(successes as f64 / records.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_both_success_and_failure_outcomes() {
        let mut graph = MemoryGraph::new();
        record_procedural(
            &mut graph,
            "L1",
            ProceduralOutcome::FixSuccess,
            "applied idempotent-init fix",
            "2026-07-04T00:00:00Z",
        );
        record_procedural(
            &mut graph,
            "L1",
            ProceduralOutcome::FixFailure,
            "fix regressed on retry",
            "2026-07-04T00:01:00Z",
        );
        assert_eq!(graph.procedural_records().len(), 2);
        assert_eq!(procedural_success_rate(&graph, "L1"), Some(0.5));
    }

    #[test]
    fn success_rate_is_none_when_no_history() {
        let graph = MemoryGraph::new();
        assert_eq!(procedural_success_rate(&graph, "L-never-tried"), None);
    }

    #[test]
    fn records_route_choice_with_confidence() {
        let mut graph = MemoryGraph::new();
        let id = record_route_choice(
            &mut graph,
            "idempotent init",
            "recall",
            0.9,
            "2026-07-04T00:00:00Z",
        );
        assert!(id.starts_with("route-"));
        assert_eq!(graph.route_traces().len(), 1);
        assert_eq!(graph.route_traces()[0].confidence, 0.9);
    }

    #[test]
    fn confidence_is_clamped_not_stored_out_of_range() {
        let mut graph = MemoryGraph::new();
        record_route_choice(&mut graph, "q", "recall", 5.0, "2026-07-04T00:00:00Z");
        record_route_choice(&mut graph, "q2", "recall", -1.0, "2026-07-04T00:00:00Z");
        assert_eq!(graph.route_traces()[0].confidence, 1.0);
        assert_eq!(graph.route_traces()[1].confidence, 0.0);
    }
}
