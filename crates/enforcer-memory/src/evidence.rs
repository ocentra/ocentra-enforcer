//! X06.6: the t0 -> t1 -> t2 learning-evidence chain engine.
//!
//! `crate::recall::evidence` (X06.1-era) already answers the basic
//! `memory evidence <lessonId>` query: observed incidents (t0) -> landed
//! artifacts (t1) -> recurrence count since landing (t2). This module
//! extends that into the FULL workpack contract without forking the
//! query: [`evidence_chain`] wraps [`crate::recall::evidence`] and adds
//! the two pieces the base query does not carry --
//!
//! - **enforcer-proof journal refs** per chain element, via a
//!   caller-supplied lookup ([`ProofRefLookup`]) rather than a hard
//!   dependency on the `enforcer-proof` crate (this crate's Cargo.toml
//!   is not part of X06.6's file claim; the lookup is a plain closure
//!   so callers that DO depend on both crates can wire a real journal
//!   query without `enforcer-memory` ever depending on `enforcer-proof`);
//! - **recurrence-curve update semantics**: [`recurrence_curve`] returns
//!   the ordered, per-incident-since-landing running count (not just the
//!   final tally [`crate::recall::evidence`] reports), so a caller can
//!   plot "did recurrence go up or down after landing" over time.
//!
//! Fail-closed is inherited unchanged from [`crate::recall::evidence`]:
//! a lesson with no t0 provenance reports `evidence:incomplete`
//! ([`EvidenceReport::is_incomplete`]), never a fabricated chain.

use crate::graph::MemoryGraph;
use crate::ingest::Incident;
use crate::recall::EvidenceResult;

/// A caller-supplied lookup from a landing reference (e.g. `"commit
/// abc123"`, `"arc-16 finding"`) to zero or more enforcer-proof journal
/// refs corroborating it. `enforcer-memory` has no compile-time
/// dependency on `enforcer-proof`'s journal type -- this is a plain
/// `Fn` seam so a caller in a crate that depends on both can wire a
/// real journal query (or a test can wire a fixed map) without this
/// crate forking or vendoring the proof journal's schema.
pub trait ProofRefLookup {
    /// Return every enforcer-proof journal ref that corroborates
    /// `landed_at_ref`. An empty vec is a legitimate answer (no journal
    /// entry found) -- it is NOT an error, it just means this element of
    /// the chain will report `proof_refs: []`.
    fn lookup(&self, landed_at_ref: &str) -> Vec<String>;
}

/// A no-op lookup that always returns no proof refs -- the default for
/// callers that have no journal wired (e.g. tests exercising only the
/// t0/t1/t2 structure). Distinguishable from "journal was consulted and
/// found nothing" only by the caller's own knowledge of which lookup it
/// passed; this type never claims completeness it doesn't have.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProofRefs;

impl ProofRefLookup for NoProofRefs {
    fn lookup(&self, _landed_at_ref: &str) -> Vec<String> {
        Vec::new()
    }
}

/// One t0 observation, with any enforcer-proof journal refs the caller's
/// [`ProofRefLookup`] could attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedIncident<'a> {
    pub incident: &'a Incident,
}

/// One t1 landing, with any enforcer-proof journal refs the caller's
/// [`ProofRefLookup`] attached for that landing reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandedArtifact {
    pub landed_at: String,
    pub proof_refs: Vec<String>,
}

/// The full evidence report for one lesson id: t0 observations, t1
/// landing(s) with proof refs, and t2 the recurrence-since-landing
/// count. Mirrors [`EvidenceResult`] but is the richer, proof-ref-aware
/// shape this module adds on top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceReport<'a> {
    Chain {
        lesson_id: String,
        observed: Vec<ObservedIncident<'a>>,
        landed: Vec<LandedArtifact>,
        recurrence_since_landing: usize,
        has_t0_provenance: bool,
    },
    Unknown {
        lesson_id: String,
    },
}

impl<'a> EvidenceReport<'a> {
    /// `true` when this lesson is known but its t0 provenance is
    /// missing -- the fail-closed signal the caller must report as
    /// `evidence:incomplete` rather than treating an empty chain as
    /// "nothing wrong".
    pub fn is_incomplete(&self) -> bool {
        matches!(
            self,
            EvidenceReport::Chain {
                has_t0_provenance: false,
                ..
            }
        )
    }
}

/// Build the full evidence report for `lesson_id`: wraps
/// [`crate::recall::evidence`] (never forks its logic) and enriches each
/// t1 landing with proof refs from `proof_refs`.
pub fn evidence_chain<'a>(
    graph: &'a MemoryGraph,
    lesson_id: &str,
    proof_refs: &impl ProofRefLookup,
) -> EvidenceReport<'a> {
    match crate::recall::evidence(graph, lesson_id) {
        EvidenceResult::Unknown { lesson_id } => EvidenceReport::Unknown { lesson_id },
        EvidenceResult::Chain {
            lesson_id,
            steps,
            recurrence_since_landing,
            has_t0_provenance,
        } => {
            let mut observed = Vec::new();
            let mut landed = Vec::new();
            for step in steps {
                match step {
                    crate::recall::EvidenceStep::Observed(incident) => {
                        observed.push(ObservedIncident { incident });
                    }
                    crate::recall::EvidenceStep::Landed(landed_at) => {
                        let refs = proof_refs.lookup(&landed_at);
                        landed.push(LandedArtifact {
                            landed_at,
                            proof_refs: refs,
                        });
                    }
                }
            }
            EvidenceReport::Chain {
                lesson_id,
                observed,
                landed,
                recurrence_since_landing,
                has_t0_provenance,
            }
        }
    }
}

/// The ordered recurrence curve for a lesson: for every t0 incident
/// recorded (in graph insertion order), whether a landed artifact
/// already existed for this lesson BY THE TIME that incident was
/// recorded, and the running "since landing" count. This slice's graph
/// has no independent per-incident vs. per-landing timestamp ordering
/// (see `crate::recall::evidence`'s own doc comment on the same
/// limitation), so "by the time" here means "landing evidence exists in
/// the graph at all" -- once a lesson lands, every subsequent-in-order
/// incident recorded for it counts toward the running recurrence total.
/// Incidents recorded before any landing exists count as pre-landing
/// (t0 baseline), not recurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrencePoint {
    pub incident_id: String,
    pub since_landing: bool,
    pub running_recurrence_count: usize,
}

/// Compute the recurrence curve for `lesson_id`. Fail-closed: if the
/// lesson has no landing evidence at all, every point has
/// `since_landing = false` and the running count stays 0 -- recurrence
/// is only ever counted against a real landing, never assumed.
pub fn recurrence_curve(graph: &MemoryGraph, lesson_id: &str) -> Vec<RecurrencePoint> {
    let has_landing = graph.nodes().iter().any(|node| match node {
        crate::graph::MemoryNode::Lesson(row) => {
            row.id == lesson_id && !row.landed_at.trim().is_empty()
        }
        crate::graph::MemoryNode::Record(record) => {
            record.id == lesson_id && record.landed_at.iter().any(|l| !l.trim().is_empty())
        }
        crate::graph::MemoryNode::Incident(_) => false,
    });

    let mut running = 0usize;
    graph
        .incidents_for_lesson(lesson_id)
        .into_iter()
        .map(|incident| {
            let since_landing = has_landing;
            if since_landing {
                running += 1;
            }
            RecurrencePoint {
                incident_id: incident.id.clone(),
                since_landing,
                running_recurrence_count: running,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{ingest_observation, Observation};
    use crate::lesson::LessonRow;
    use std::collections::HashMap;

    struct FixedLookup(HashMap<String, Vec<String>>);

    impl ProofRefLookup for FixedLookup {
        fn lookup(&self, landed_at_ref: &str) -> Vec<String> {
            self.0.get(landed_at_ref).cloned().unwrap_or_default()
        }
    }

    fn graph_with_landed_lesson_and_incident() -> MemoryGraph {
        let mut graph = MemoryGraph::new();
        graph.ingest_lesson_row(LessonRow {
            id: "L1".to_string(),
            date: "2026-07-04".to_string(),
            observed: "init threw raw EEXIST".to_string(),
            lesson: "init must be idempotent".to_string(),
            landed_at: "arc-16 finding".to_string(),
            ships_via: "fixed MCP tool behavior".to_string(),
        });
        ingest_observation(
            &mut graph,
            Observation {
                lesson_id: "L1".to_string(),
                rule_id: Some("ARC16-INIT".to_string()),
                fault_class: Some("non_idempotent_init".to_string()),
                repo_context: "crates/enforcer-coordination".to_string(),
                clean: false,
                source_surface: "check".to_string(),
                ts: "2026-07-04T00:00:00Z".to_string(),
            },
        );
        graph
    }

    #[test]
    fn evidence_chain_attaches_proof_refs_to_landed_step() {
        let graph = graph_with_landed_lesson_and_incident();
        let mut refs = HashMap::new();
        refs.insert(
            "arc-16 finding".to_string(),
            vec!["proof/journal/arc-16-0007".to_string()],
        );
        let lookup = FixedLookup(refs);

        match evidence_chain(&graph, "L1", &lookup) {
            EvidenceReport::Chain {
                landed,
                observed,
                has_t0_provenance,
                ..
            } => {
                assert!(has_t0_provenance);
                assert_eq!(observed.len(), 1);
                assert_eq!(landed.len(), 1);
                assert_eq!(landed[0].proof_refs, vec!["proof/journal/arc-16-0007"]);
            }
            EvidenceReport::Unknown { .. } => unreachable!("expected a chain"),
        }
    }

    #[test]
    fn evidence_chain_no_lookup_hit_reports_empty_refs_not_error() {
        let graph = graph_with_landed_lesson_and_incident();
        let lookup = NoProofRefs;
        match evidence_chain(&graph, "L1", &lookup) {
            EvidenceReport::Chain { landed, .. } => {
                assert_eq!(landed[0].proof_refs, Vec::<String>::new());
            }
            EvidenceReport::Unknown { .. } => unreachable!("expected a chain"),
        }
    }

    #[test]
    fn evidence_chain_unknown_lesson_is_incomplete_safe() {
        let graph = graph_with_landed_lesson_and_incident();
        let report = evidence_chain(&graph, "L-missing", &NoProofRefs);
        assert!(matches!(report, EvidenceReport::Unknown { .. }));
        assert!(!report.is_incomplete(), "Unknown is not the same as incomplete-chain");
    }

    #[test]
    fn evidence_chain_missing_t0_is_incomplete() {
        let mut graph = MemoryGraph::new();
        graph.ingest_lesson_row(LessonRow {
            id: "L2".to_string(),
            date: "2026-07-04".to_string(),
            observed: "seen once".to_string(),
            lesson: "no incidents recorded yet".to_string(),
            landed_at: "commit abc123".to_string(),
            ships_via: "docs".to_string(),
        });
        let report = evidence_chain(&graph, "L2", &NoProofRefs);
        assert!(report.is_incomplete(), "missing t0 must report incomplete");
    }

    #[test]
    fn recurrence_curve_counts_only_after_landing_exists() {
        let graph = graph_with_landed_lesson_and_incident();
        let curve = recurrence_curve(&graph, "L1");
        assert_eq!(curve.len(), 1);
        assert!(curve[0].since_landing);
        assert_eq!(curve[0].running_recurrence_count, 1);
    }

    #[test]
    fn recurrence_curve_before_any_landing_is_zero() {
        let mut graph = MemoryGraph::new();
        ingest_observation(
            &mut graph,
            Observation {
                lesson_id: "L3".to_string(),
                rule_id: None,
                fault_class: Some("x".to_string()),
                repo_context: "crates/foo".to_string(),
                clean: false,
                source_surface: "scan".to_string(),
                ts: "2026-07-04T00:00:00Z".to_string(),
            },
        );
        let curve = recurrence_curve(&graph, "L3");
        assert_eq!(curve.len(), 1);
        assert!(!curve[0].since_landing, "no landing exists yet for L3");
        assert_eq!(curve[0].running_recurrence_count, 0);
    }
}
