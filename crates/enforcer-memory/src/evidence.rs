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
use crate::owned_boundary::Retained;
use crate::recall::EvidenceResult;
use enforcer_domain::memory_types::{
    IngestIncidentId, MemoryEvidenceHasT0Provenance, MemoryEvidenceIncomplete,
    MemoryEvidenceLandedAt, MemoryEvidenceProofRef, MemoryEvidenceRecurrenceCount,
    MemoryEvidenceSinceLanding, MemoryLessonId,
};

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
    fn lookup(&self, landed_at_ref: &MemoryEvidenceLandedAt) -> Vec<MemoryEvidenceProofRef>;
}

/// A no-op lookup that always returns no proof refs -- the default for
/// callers that have no journal wired (e.g. tests exercising only the
/// t0/t1/t2 structure). Distinguishable from "journal was consulted and
/// found nothing" only by the caller's own knowledge of which lookup it
/// passed; this type never claims completeness it doesn't have.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProofRefs;

impl ProofRefLookup for NoProofRefs {
    fn lookup(&self, _landed_at_ref: &MemoryEvidenceLandedAt) -> Vec<MemoryEvidenceProofRef> {
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
    pub landed_at: MemoryEvidenceLandedAt,
    pub proof_refs: Vec<MemoryEvidenceProofRef>,
}

/// The full evidence report for one lesson id: t0 observations, t1
/// landing(s) with proof refs, and t2 the recurrence-since-landing
/// count. Mirrors [`EvidenceResult`] but is the richer, proof-ref-aware
/// shape this module adds on top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceReport<'a> {
    Chain {
        lesson_id: MemoryLessonId,
        observed: Vec<ObservedIncident<'a>>,
        landed: Vec<LandedArtifact>,
        recurrence_since_landing: MemoryEvidenceRecurrenceCount,
        has_t0_provenance: MemoryEvidenceHasT0Provenance,
    },
    Unknown {
        lesson_id: MemoryLessonId,
    },
}

impl<'a> EvidenceReport<'a> {
    /// `true` when this lesson is known but its t0 provenance is
    /// missing -- the fail-closed signal the caller must report as
    /// `evidence:incomplete` rather than treating an empty chain as
    /// "nothing wrong".
    pub fn is_incomplete(&self) -> MemoryEvidenceIncomplete {
        matches!(
            self,
            EvidenceReport::Chain {
                has_t0_provenance,
                ..
            } if !has_t0_provenance.has_t0_provenance()
        )
        .into()
    }
}

/// Build the full evidence report for `lesson_id`: wraps
/// [`crate::recall::evidence`] (never forks its logic) and enriches each
/// t1 landing with proof refs from `proof_refs`.
pub fn evidence_chain<'a>(
    graph: &'a MemoryGraph,
    lesson_id: &MemoryLessonId,
    proof_refs: &impl ProofRefLookup,
) -> EvidenceReport<'a> {
    match crate::recall::evidence(graph, lesson_id) {
        EvidenceResult::Unknown { .. } => EvidenceReport::Unknown {
            lesson_id: lesson_id.retained(),
        },
        EvidenceResult::Chain {
            lesson_id: _,
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
                lesson_id: lesson_id.retained(),
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
    pub incident_id: IngestIncidentId,
    pub since_landing: MemoryEvidenceSinceLanding,
    pub running_recurrence_count: MemoryEvidenceRecurrenceCount,
}

/// Compute the recurrence curve for `lesson_id`. Fail-closed: if the
/// lesson has no landing evidence at all, every point has
/// `since_landing = false` and the running count stays 0 -- recurrence
/// is only ever counted against a real landing, never assumed.
pub fn recurrence_curve(graph: &MemoryGraph, lesson_id: &MemoryLessonId) -> Vec<RecurrencePoint> {
    let has_landing = graph.nodes().iter().any(|node| match node {
        crate::graph::MemoryNode::Lesson(row) => {
            row.id == lesson_id.as_str() && !row.landed_at.trim().is_empty()
        }
        crate::graph::MemoryNode::Record(record) => {
            record.id() == lesson_id.as_str()
                && record.landed_at().iter().any(|l| !l.trim().is_empty())
        }
        crate::graph::MemoryNode::Incident(_) => false,
    });

    let mut running = 0usize;
    graph
        .incidents_for_lesson(&lesson_id.as_str().into())
        .into_iter()
        .map(|incident| {
            let since_landing = has_landing;
            if since_landing {
                running += 1;
            }
            RecurrencePoint {
                incident_id: incident.id.retained(),
                since_landing: since_landing.into(),
                running_recurrence_count: running.into(),
            }
        })
        .collect()
}
