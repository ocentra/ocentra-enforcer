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

use crate::boundary::log_schema::{
    ObservationLogEntryDto, ProceduralRecordDto, RouteTraceDto, SCHEMA_VERSION,
};
use crate::error::{MemoryError, Result};
use crate::graph::MemoryGraph;
use crate::owned_boundary::Retained;
use crate::store::Store;
use enforcer_domain::memory_types::{
    MemoryObservationReplayCount, MemoryObservationSearchText, MemoryObservationTimestamp,
    ProceduralDetail, ProceduralLessonReference, ProceduralOutcome, ProceduralRecordId,
    ProceduralSuccessRate, RetrievalRoute, RouteConfidence, RouteTraceId, RouteTraceQuery,
};

/// One procedural-memory record: the outcome of attempting to apply a
/// lesson's fix or retrieval guidance. Both success AND failure are
/// first-class -- a procedural memory that only ever records success
/// cannot distinguish "this always works" from "this was only tried
/// once and got lucky".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralRecord {
    pub id: ProceduralRecordId,
    pub lesson_id: ProceduralLessonReference,
    pub outcome: ProceduralOutcome,
    /// Free-text detail: what was attempted (e.g. "applied fix from
    /// mem-a-0001: return existing identity on re-init").
    pub detail: ProceduralDetail,
    pub ts: MemoryObservationTimestamp,
}

/// Whether applying a lesson's guidance succeeded or failed this time.
impl ProceduralRecord {
    pub fn searchable_text(&self) -> MemoryObservationSearchText {
        format!(
            "{} {} {}",
            self.lesson_id,
            self.outcome.as_str(),
            self.detail
        )
        .into()
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
    pub id: RouteTraceId,
    pub query: RouteTraceQuery,
    /// Which retrieval route answered this query, e.g. `"recall"`,
    /// `"evidence"`, `"code_graph"` -- free text naming the module/query
    /// path actually taken, not a closed enum, because the set of
    /// routes grows as later x06 subpacks (X06.4 retriever, X06.3 graph
    /// algorithms) add more query surfaces this crate cannot enumerate
    /// today.
    pub route: RetrievalRoute,
    /// Confidence in `[0.0, 1.0]`. Not a probability calibrated against
    /// any model -- this slice has no learned scorer -- but a
    /// deterministic signal the caller supplies (e.g. "1.0 if recall
    /// returned a non-empty hit set, 0.0 otherwise") so route-choice
    /// quality is at least comparable across queries.
    pub confidence: RouteConfidence,
    pub ts: MemoryObservationTimestamp,
}

impl RouteTrace {
    pub fn searchable_text(&self) -> MemoryObservationSearchText {
        format!("{} {}", self.query, self.route).into()
    }
}

/// Preserve the route-confidence domain at every ingestion and replay
/// boundary. Public record fields and historical NDJSON can otherwise carry
/// non-finite values, for which `f64::clamp` alone is not sufficient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProceduralStoreInput {
    pub lesson_id: ProceduralLessonReference,
    pub outcome: ProceduralOutcome,
    pub detail: ProceduralDetail,
    pub ts: MemoryObservationTimestamp,
}

impl ProceduralStoreInput {
    pub fn new(
        lesson_id: impl Into<ProceduralLessonReference>,
        outcome: ProceduralOutcome,
        detail: impl Into<ProceduralDetail>,
        ts: impl Into<MemoryObservationTimestamp>,
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
    pub query: RouteTraceQuery,
    pub route: RetrievalRoute,
    pub confidence: RouteConfidence,
    pub ts: MemoryObservationTimestamp,
}

impl RouteChoiceStoreInput {
    pub fn new(
        query: impl Into<RouteTraceQuery>,
        route: impl Into<RetrievalRoute>,
        confidence: impl Into<RouteConfidence>,
        ts: impl Into<MemoryObservationTimestamp>,
    ) -> Self {
        Self {
            query: query.into(),
            route: route.into(),
            confidence: confidence.into(),
            ts: ts.into(),
        }
    }
}

/// Record one procedural-memory outcome into `graph`. Returns the
/// assigned id.
pub fn record_procedural(
    graph: &mut MemoryGraph,
    lesson_id: impl Into<ProceduralLessonReference>,
    outcome: ProceduralOutcome,
    detail: impl Into<ProceduralDetail>,
    ts: impl Into<MemoryObservationTimestamp>,
) -> ProceduralRecordId {
    let id: ProceduralRecordId = format!("proc-{:04}", graph.procedural_records().len()).into();
    let record = ProceduralRecord {
        // CLONE-JUSTIFICATION: the record is consumed by graph ingestion while the caller receives its id.
        id: id.retained(),
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
) -> Result<ProceduralRecordId> {
    let mut assigned_id = ProceduralRecordId::default();
    let mut assigned_record: Option<ProceduralRecord> = None;
    store.append_observation_entry(|seq| {
        let id = format!("proc-{seq:04}");
        // CLONE-JUSTIFICATION: the append closure retains the returned id while the observation entry owns its id.
        assigned_id = id.retained().into();
        let record = ProceduralRecord {
            // CLONE-JUSTIFICATION: the procedural record and observation entry are independent durable records.
            id: id.retained().into(),
            // CLONE-JUSTIFICATION: native procedural persistence and the observation envelope each own this field.
            lesson_id: input.lesson_id.retained(),
            outcome: input.outcome,
            // CLONE-JUSTIFICATION: native procedural persistence and the observation envelope each own this field.
            detail: input.detail.retained(),
            // CLONE-JUSTIFICATION: native procedural persistence and the observation envelope each own this field.
            ts: input.ts.retained(),
        };
        let payload = serde_json::json!({
            "id": &record.id,
            "lesson_id": &record.lesson_id,
            "outcome": record.outcome,
            "detail": &record.detail,
            "ts": &record.ts,
        });
        assigned_record = Some(record);
        ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq,
            id: id.into(),
            // CLONE-JUSTIFICATION: the native procedural record remains owned for subsequent append and graph ingestion.
            lesson_id: input.lesson_id.as_str().into(),
            rule_id: None,
            fault_class: Some(input.outcome.as_str().into()),
            // CLONE-JUSTIFICATION: the native procedural record remains owned for subsequent append and graph ingestion.
            repo_context: input.detail.as_str().into(),
            clean: input.outcome.is_success().into(),
            source_surface: "procedural-memory".into(),
            // CLONE-JUSTIFICATION: the native procedural record remains owned for subsequent append and graph ingestion.
            ts: input.ts.as_str().into(),
            supersedes_seq: None,
            payload_kind: Some("procedural-memory".into()),
            payload: Some(payload.into()),
        }
    })?;
    let Some(record) = assigned_record else {
        return Err(MemoryError::InternalInvariant {
            operation: "record_procedural_in_store".into(),
            reason: "append did not assign a procedural record"
                .retained()
                .into(),
        });
    };
    // CLONE-JUSTIFICATION: store append consumes one record while graph ingestion consumes the other.
    store.append_procedural(record.retained())?;
    graph.ingest_procedural(record);
    Ok(assigned_id)
}

/// Record one meta-memory route-choice trace into `graph`. `confidence`
/// is clamped into `[0.0, 1.0]` -- a caller-supplied value outside that
/// range is a caller bug, not grounds to silently store a
/// nonsensical confidence.
pub fn record_route_choice(
    graph: &mut MemoryGraph,
    query: impl Into<RouteTraceQuery>,
    route: impl Into<RetrievalRoute>,
    confidence: impl Into<RouteConfidence>,
    ts: impl Into<MemoryObservationTimestamp>,
) -> RouteTraceId {
    let id: RouteTraceId = format!("route-{:04}", graph.route_traces().len()).into();
    let trace = RouteTrace {
        // CLONE-JUSTIFICATION: the trace is consumed by graph ingestion while the caller receives its id.
        id: id.retained(),
        query: query.into(),
        route: route.into(),
        confidence: confidence.into(),
        ts: ts.into(),
    };
    graph.ingest_route_trace(trace);
    id
}

pub fn record_route_choice_in_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    input: &RouteChoiceStoreInput,
) -> Result<RouteTraceId> {
    let mut assigned_id = RouteTraceId::default();
    let mut assigned_trace: Option<RouteTrace> = None;
    store.append_observation_entry(|seq| {
        let id = format!("route-{seq:04}");
        // CLONE-JUSTIFICATION: the append closure retains the returned id while the observation entry owns its id.
        assigned_id = id.retained().into();
        let trace = RouteTrace {
            // CLONE-JUSTIFICATION: the route trace and observation entry are independent durable records.
            id: id.retained().into(),
            // CLONE-JUSTIFICATION: native route persistence and the observation envelope each own this field.
            query: input.query.retained(),
            // CLONE-JUSTIFICATION: native route persistence and the observation envelope each own this field.
            route: input.route.retained(),
            confidence: input.confidence,
            // CLONE-JUSTIFICATION: native route persistence and the observation envelope each own this field.
            ts: input.ts.retained(),
        };
        let payload = serde_json::json!({
            "id": &trace.id,
            "query": &trace.query,
            "route": &trace.route,
            "confidence": trace.confidence,
            "ts": &trace.ts,
        });
        assigned_trace = Some(trace);
        ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq,
            id: id.into(),
            lesson_id: "".into(),
            rule_id: None,
            fault_class: Some("route-choice".into()),
            // CLONE-JUSTIFICATION: the native route trace remains owned for subsequent append and graph ingestion.
            repo_context: input.query.as_str().into(),
            clean: true.into(),
            source_surface: "route-choice".into(),
            // CLONE-JUSTIFICATION: the native route trace remains owned for subsequent append and graph ingestion.
            ts: input.ts.as_str().into(),
            supersedes_seq: None,
            payload_kind: Some("route-choice".into()),
            payload: Some(payload.into()),
        }
    })?;
    let Some(trace) = assigned_trace else {
        return Err(MemoryError::InternalInvariant {
            operation: "record_route_choice_in_store".into(),
            reason: "append did not assign a route trace".retained().into(),
        });
    };
    // CLONE-JUSTIFICATION: store append consumes one trace while graph ingestion consumes the other.
    store.append_route_trace(trace.retained())?;
    graph.ingest_route_trace(trace);
    Ok(assigned_id)
}

pub fn replay_procedural_and_routes_from_store(
    store: &Store,
    graph: &mut MemoryGraph,
) -> Result<MemoryObservationReplayCount> {
    let mut count = replay_procedural_from_native_log(store, graph)?;
    count += replay_route_traces_from_native_log(store, graph)?.get();
    count += replay_procedural_and_routes_from_legacy_observation_log(store, graph)?.get();
    Ok(count)
}

fn replay_procedural_from_native_log(
    store: &Store,
    graph: &mut MemoryGraph,
) -> Result<MemoryObservationReplayCount> {
    let outcome = store.read_procedural_entries()?;
    let mut count = MemoryObservationReplayCount::default();
    for entry in outcome.entries {
        let record = ProceduralRecord {
            id: entry.id,
            lesson_id: entry.lesson_id,
            outcome: entry.outcome,
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

fn replay_route_traces_from_native_log(
    store: &Store,
    graph: &mut MemoryGraph,
) -> Result<MemoryObservationReplayCount> {
    let outcome = store.read_route_trace_entries()?;
    let mut count = MemoryObservationReplayCount::default();
    for entry in outcome.entries {
        let trace = RouteTrace {
            id: entry.id,
            query: entry.query,
            route: entry.route,
            confidence: entry.confidence,
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
) -> Result<MemoryObservationReplayCount> {
    let outcome = store.read_observation_entries()?;
    let mut count = MemoryObservationReplayCount::default();
    for entry in outcome.entries {
        match (entry.payload_kind.as_deref(), entry.payload) {
            (Some("procedural-memory"), Some(payload)) => {
                let record: ProceduralRecord =
                    serde_json::from_value::<ProceduralRecordDto>(payload.into())?.into();
                if !graph.procedural_records().iter().any(|r| r.id == record.id) {
                    graph.ingest_procedural(record);
                    count += 1;
                }
            }
            (Some("route-choice"), Some(payload)) => {
                let mut trace: RouteTrace =
                    serde_json::from_value::<RouteTraceDto>(payload.into())?.into();
                trace.confidence = RouteConfidence::normalized(trace.confidence.get());
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
pub fn procedural_success_rate(
    graph: &MemoryGraph,
    lesson_id: impl Into<ProceduralLessonReference>,
) -> Option<ProceduralSuccessRate> {
    let lesson_id = lesson_id.into();
    let records: Vec<&ProceduralRecord> = graph
        .procedural_records()
        .iter()
        .filter(|r| r.lesson_id == lesson_id)
        .collect();
    if records.is_empty() {
        return None;
    }
    let successes = records.iter().filter(|r| r.outcome.is_success()).count();
    ProceduralSuccessRate::from_counts(successes, records.len())
}
