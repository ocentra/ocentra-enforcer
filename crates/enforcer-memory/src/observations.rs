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

use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};
use crate::graph::MemoryGraph;
use crate::schema::{ObservationLogEntry, SCHEMA_VERSION};
use crate::store::Store;

/// One procedural-memory record: the outcome of attempting to apply a
/// lesson's fix or retrieval guidance. Both success AND failure are
/// first-class -- a procedural memory that only ever records success
/// cannot distinguish "this always works" from "this was only tried
/// once and got lucky".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Preserve the route-confidence domain at every ingestion and replay
/// boundary. Public record fields and historical NDJSON can otherwise carry
/// non-finite values, for which `f64::clamp` alone is not sufficient.
fn normalize_confidence(confidence: f64) -> f64 {
    if confidence.is_finite() {
        confidence.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralStoreInput {
    pub lesson_id: String,
    pub outcome: ProceduralOutcome,
    pub detail: String,
    pub ts: String,
}

impl ProceduralStoreInput {
    pub fn new(
        lesson_id: impl Into<String>,
        outcome: ProceduralOutcome,
        detail: impl Into<String>,
        ts: impl Into<String>,
    ) -> Self {
        Self {
            lesson_id: lesson_id.into(),
            outcome,
            detail: detail.into(),
            ts: ts.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteChoiceStoreInput {
    pub query: String,
    pub route: String,
    pub confidence: f64,
    pub ts: String,
}

impl RouteChoiceStoreInput {
    pub fn new(
        query: impl Into<String>,
        route: impl Into<String>,
        confidence: f64,
        ts: impl Into<String>,
    ) -> Self {
        Self {
            query: query.into(),
            route: route.into(),
            confidence,
            ts: ts.into(),
        }
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

pub fn record_procedural_in_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    input: &ProceduralStoreInput,
) -> Result<String> {
    let mut assigned_id = String::new();
    let mut assigned_record: Option<ProceduralRecord> = None;
    store.append_observation_entry(|seq| {
        let id = format!("proc-{seq:04}");
        assigned_id = id.clone();
        let record = ProceduralRecord {
            id: id.clone(),
            lesson_id: input.lesson_id.clone(),
            outcome: input.outcome,
            detail: input.detail.clone(),
            ts: input.ts.clone(),
        };
        let payload = serde_json::json!({
            "id": &record.id,
            "lesson_id": &record.lesson_id,
            "outcome": record.outcome,
            "detail": &record.detail,
            "ts": &record.ts,
        });
        assigned_record = Some(record);
        ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id,
            lesson_id: input.lesson_id.clone(),
            rule_id: None,
            fault_class: Some(input.outcome.as_str().to_owned()),
            repo_context: input.detail.clone(),
            clean: input.outcome.is_success(),
            source_surface: "procedural-memory".to_owned(),
            ts: input.ts.clone(),
            supersedes_seq: None,
            payload_kind: Some("procedural-memory".to_owned()),
            payload: Some(payload),
        }
    })?;
    let Some(record) = assigned_record else {
        return Err(MemoryError::InternalInvariant {
            operation: "record_procedural_in_store",
            reason: "append did not assign a procedural record".to_owned(),
        });
    };
    store.append_procedural(record.clone())?;
    graph.ingest_procedural(record);
    Ok(assigned_id)
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
        confidence: normalize_confidence(confidence),
        ts: ts.into(),
    };
    graph.ingest_route_trace(trace);
    id
}

pub fn record_route_choice_in_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    input: &RouteChoiceStoreInput,
) -> Result<String> {
    let mut assigned_id = String::new();
    let mut assigned_trace: Option<RouteTrace> = None;
    store.append_observation_entry(|seq| {
        let id = format!("route-{seq:04}");
        assigned_id = id.clone();
        let trace = RouteTrace {
            id: id.clone(),
            query: input.query.clone(),
            route: input.route.clone(),
            confidence: normalize_confidence(input.confidence),
            ts: input.ts.clone(),
        };
        let payload = serde_json::json!({
            "id": &trace.id,
            "query": &trace.query,
            "route": &trace.route,
            "confidence": trace.confidence,
            "ts": &trace.ts,
        });
        assigned_trace = Some(trace);
        ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id,
            lesson_id: String::new(),
            rule_id: None,
            fault_class: Some("route-choice".to_owned()),
            repo_context: input.query.clone(),
            clean: true,
            source_surface: "route-choice".to_owned(),
            ts: input.ts.clone(),
            supersedes_seq: None,
            payload_kind: Some("route-choice".to_owned()),
            payload: Some(payload),
        }
    })?;
    let Some(trace) = assigned_trace else {
        return Err(MemoryError::InternalInvariant {
            operation: "record_route_choice_in_store",
            reason: "append did not assign a route trace".to_owned(),
        });
    };
    store.append_route_trace(trace.clone())?;
    graph.ingest_route_trace(trace);
    Ok(assigned_id)
}

pub fn replay_procedural_and_routes_from_store(
    store: &Store,
    graph: &mut MemoryGraph,
) -> Result<usize> {
    let mut count = replay_procedural_from_native_log(store, graph)?;
    count += replay_route_traces_from_native_log(store, graph)?;
    count += replay_procedural_and_routes_from_legacy_observation_log(store, graph)?;
    Ok(count)
}

fn replay_procedural_from_native_log(store: &Store, graph: &mut MemoryGraph) -> Result<usize> {
    let outcome = store.read_procedural_entries()?;
    let mut count = 0;
    for entry in outcome.entries {
        let record = ProceduralRecord {
            id: entry.id,
            lesson_id: entry.lesson_id,
            outcome: match entry.outcome {
                crate::schema::ProceduralOutcomeWire::RetrievalSuccess => {
                    ProceduralOutcome::RetrievalSuccess
                }
                crate::schema::ProceduralOutcomeWire::RetrievalFailure => {
                    ProceduralOutcome::RetrievalFailure
                }
                crate::schema::ProceduralOutcomeWire::FixSuccess => ProceduralOutcome::FixSuccess,
                crate::schema::ProceduralOutcomeWire::FixFailure => ProceduralOutcome::FixFailure,
            },
            detail: entry.detail,
            ts: entry.ts,
        };
        if !graph
            .procedural_records()
            .iter()
            .any(|existing| existing.id == record.id)
        {
            graph.ingest_procedural(record);
            count += 1;
        }
    }
    Ok(count)
}

fn replay_route_traces_from_native_log(store: &Store, graph: &mut MemoryGraph) -> Result<usize> {
    let outcome = store.read_route_trace_entries()?;
    let mut count = 0;
    for entry in outcome.entries {
        let trace = RouteTrace {
            id: entry.id,
            query: entry.query,
            route: entry.route,
            confidence: normalize_confidence(entry.confidence),
            ts: entry.ts,
        };
        if !graph
            .route_traces()
            .iter()
            .any(|existing| existing.id == trace.id)
        {
            graph.ingest_route_trace(trace);
            count += 1;
        }
    }
    Ok(count)
}

fn replay_procedural_and_routes_from_legacy_observation_log(
    store: &Store,
    graph: &mut MemoryGraph,
) -> Result<usize> {
    let outcome = store.read_observation_entries()?;
    let mut count = 0;
    for entry in outcome.entries {
        match (entry.payload_kind.as_deref(), entry.payload) {
            (Some("procedural-memory"), Some(payload)) => {
                let record: ProceduralRecord = serde_json::from_value(payload)?;
                if !graph.procedural_records().iter().any(|r| r.id == record.id) {
                    graph.ingest_procedural(record);
                    count += 1;
                }
            }
            (Some("route-choice"), Some(payload)) => {
                let mut trace: RouteTrace = serde_json::from_value(payload)?;
                trace.confidence = normalize_confidence(trace.confidence);
                if !graph.route_traces().iter().any(|r| r.id == trace.id) {
                    graph.ingest_route_trace(trace);
                    count += 1;
                }
            }
            _ => {}
        }
    }
    Ok(count)
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
