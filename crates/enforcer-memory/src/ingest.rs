//! Ingestion: parsing append-only NDJSON memory-record streams, and the
//! usage-ingestion seam that scan/check/run/closeout surfaces call on
//! every run so that enforcement usage automatically feeds the graph.
//!
//! The seam is a plain function contract ([`ingest_observation`]) rather
//! than a trait-object registry: callers in other crates (arc-15 scan/
//! check/run, arc-16 coordination closeout) depend on `enforcer-memory`
//! and call this function directly. Wiring that call from those crates
//! is explicitly OUT OF SCOPE for this lane (x06 owns only
//! `crates/enforcer-memory/**`) — see the final report for the deferred
//! follow-up.

use crate::boundary::log_schema::{ObservationLogEntryDto, SCHEMA_VERSION};
use crate::boundary::record::MemoryRecordDto;
use crate::error::Result as MemoryResult;
use crate::graph::MemoryGraph;
use crate::record::MemoryRecord;
use crate::store::Store;
use enforcer_domain::memory_types::{
    IngestClean, IngestFaultClass, IngestIncidentId, IngestIncidentSearchText, IngestLessonId,
    IngestLineNumber, IngestNdjsonDocument, IngestObservationPayload, IngestObservationPayloadKind,
    IngestRecordCount, IngestRepoContext, IngestRuleId, IngestSourceSurface, IngestTimestamp,
};
use thiserror::Error;

/// Errors from parsing an NDJSON memory-record stream.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error("line {line}: invalid JSON: {source}")]
    InvalidJson {
        line: IngestLineNumber,
        #[source]
        source: serde_json::Error,
    },
}

/// Parse a full NDJSON document (one [`MemoryRecord`] per non-blank
/// line) into records, in file order. A malformed line is a hard error
/// — this is an append-only audit log; a corrupt line must not be
/// silently skipped.
/// PROPERTY-TEST: crates/enforcer-memory/tests/property_parser_contracts.rs::every_registered_parser_is_total
pub fn parse_ndjson(
    text: impl Into<IngestNdjsonDocument>,
) -> Result<Vec<MemoryRecord>, IngestError> {
    let text = text.into();
    let mut records = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let dto: MemoryRecordDto =
            crate::boundary::json::decode(trimmed).map_err(|source| IngestError::InvalidJson {
                line: (idx + 1).into(),
                source,
            })?;
        records.push(MemoryRecord::from_dto(dto));
    }
    Ok(records)
}

/// Ingest an NDJSON document's records into `graph`. Returns the number
/// of records ingested.
pub fn ingest_ndjson_into(
    graph: &mut MemoryGraph,
    text: impl Into<IngestNdjsonDocument>,
) -> Result<IngestRecordCount, IngestError> {
    let records = parse_ndjson(text)?;
    let count = records.len();
    for record in records {
        graph.ingest_record(record);
    }
    Ok(count.into())
}

/// One fault occurrence: the "Incident node" the workpack's
/// usage-ingestion requirement describes — `finding/fault-class/ruleId/
/// repo-context -> Incident node + observedIn edges`. A clean run still
/// produces an `Incident` with `clean = true` (negative evidence): usage
/// is learning even when nothing is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incident {
    /// Stable id for this observation, e.g. `obs-<writer>-<n>`.
    pub id: IngestIncidentId,
    /// The rule/lesson this observation is evidence for or against.
    /// Empty string when the observation is not yet linked to a lesson.
    pub lesson_id: IngestLessonId,
    /// Rule id the finding fired on, if any (`ruleId` in the workpack's
    /// contract). `None` for a clean scan with no findings at all.
    pub rule_id: Option<IngestRuleId>,
    /// Fault class / finding category, free text (e.g. `"unwrap_used"`).
    pub fault_class: Option<IngestFaultClass>,
    /// Repo-relative path or module the observation concerns.
    pub repo_context: IngestRepoContext,
    /// `true` when this observation is a clean run recording the
    /// ABSENCE of the fault class (negative evidence), `false` when it
    /// is an actual finding.
    pub clean: IngestClean,
    /// Where this observation came from: `scan`, `check`, `run`,
    /// `doctor`, `closeout`, matching the workpack's named call sites.
    pub source_surface: IngestSourceSurface,
    /// Opaque timestamp, ISO-8601 string (no parsing needed by this
    /// crate — callers already have a clock; we just record it).
    pub ts: IngestTimestamp,
}

impl Incident {
    pub fn searchable_text(&self) -> IngestIncidentSearchText {
        let fault_class = self.fault_class.as_deref().unwrap_or("");
        let rule_id = self.rule_id.as_deref().unwrap_or("");
        format!("{} {} {}", self.repo_context, fault_class, rule_id).into()
    }
}

/// Parameters for one call into the usage-ingestion seam. Mirrors the
/// workpack contract literally: "finding/fault-class/ruleId/repo-context
/// -> Incident node + observedIn edges".
#[derive(Debug, Clone)]
pub struct Observation {
    pub lesson_id: IngestLessonId,
    pub rule_id: Option<IngestRuleId>,
    pub fault_class: Option<IngestFaultClass>,
    pub repo_context: IngestRepoContext,
    pub clean: IngestClean,
    pub source_surface: IngestSourceSurface,
    pub ts: IngestTimestamp,
}

/// The usage-ingestion seam: every enforcement operation (scan/check/run/
/// doctor/closeout) calls this on every run — no manual capture step.
/// Append-only: this always creates a new [`Incident`] node, it never
/// edits or removes an existing one. Returns the id of the incident
/// created so the caller can, e.g., surface it in a run's proof journal.
pub fn ingest_observation(graph: &mut MemoryGraph, observation: Observation) -> IngestIncidentId {
    let id = IngestIncidentId::from(format!(
        "obs-{}-{:04}",
        observation.source_surface,
        graph.len().get()
    ));
    // CLONE-JUSTIFICATION: the graph consumes the incident while the caller receives its id.
    let incident = incident_from_observation(id.clone(), observation);
    graph.ingest_incident(incident);
    id
}

pub fn append_observation_to_store(
    store: &mut Store,
    observation: Observation,
) -> MemoryResult<IngestIncidentId> {
    append_observation_payload_to_store(store, observation, None, None)
}

pub fn append_observation_payload_to_store(
    store: &mut Store,
    observation: Observation,
    payload_kind: Option<IngestObservationPayloadKind>,
    payload: Option<IngestObservationPayload>,
) -> MemoryResult<IngestIncidentId> {
    let mut assigned_id = IngestIncidentId::default();
    let payload = payload
        .map(|value| crate::boundary::json::decode(value.as_str()))
        .transpose()?;
    store.append_observation_entry(|seq| {
        let id = format!("obs-{}-{seq:04}", observation.source_surface);
        // CLONE-JUSTIFICATION: the durable log entry owns its id while the caller receives it after append.
        assigned_id = id.as_str().into();
        ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq,
            id: id.into(),
            lesson_id: observation.lesson_id,
            rule_id: observation.rule_id,
            fault_class: observation.fault_class,
            repo_context: observation.repo_context,
            clean: observation.clean,
            source_surface: observation.source_surface,
            ts: observation.ts,
            supersedes_seq: None,
            payload_kind,
            payload,
        }
    })?;
    Ok(assigned_id)
}

pub fn ingest_observation_into_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    observation: Observation,
) -> MemoryResult<IngestIncidentId> {
    ingest_observation_payload_into_store(store, graph, observation, None, None)
}

pub fn ingest_observation_payload_into_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    observation: Observation,
    payload_kind: Option<IngestObservationPayloadKind>,
    payload: Option<IngestObservationPayload>,
) -> MemoryResult<IngestIncidentId> {
    let id = append_observation_payload_to_store(store, observation, payload_kind, payload)?;
    let stored = store.read_observation_entries()?;
    if let Some(entry) = stored
        .entries
        .into_iter()
        .find(|entry| entry.id == id.as_str())
    {
        graph.ingest_incident(incident_from_entry(&entry));
    }
    Ok(id)
}

pub fn replay_incident_observations_from_store(
    store: &Store,
    graph: &mut MemoryGraph,
) -> MemoryResult<IngestRecordCount> {
    let outcome = store.read_observation_entries()?;
    let mut count = 0;
    for entry in outcome.entries {
        if matches!(
            entry.payload_kind.as_deref(),
            Some("procedural-memory") | Some("route-choice")
        ) {
            continue;
        }
        if graph
            .nodes()
            .iter()
            .any(|node| node.id().as_str() == entry.id.as_str())
        {
            continue;
        }
        graph.ingest_incident(incident_from_entry(&entry));
        count += 1;
    }
    Ok(count.into())
}

fn incident_from_observation(id: IngestIncidentId, observation: Observation) -> Incident {
    Incident {
        id,
        lesson_id: observation.lesson_id,
        rule_id: observation.rule_id,
        fault_class: observation.fault_class,
        repo_context: observation.repo_context,
        clean: observation.clean,
        source_surface: observation.source_surface,
        ts: observation.ts,
    }
}

fn incident_from_entry(entry: &ObservationLogEntryDto) -> Incident {
    Incident {
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        id: entry.id.as_str().into(),
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        lesson_id: entry.lesson_id.clone(),
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        rule_id: entry.rule_id.clone(),
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        fault_class: entry.fault_class.clone(),
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        repo_context: entry.repo_context.clone(),
        clean: entry.clean,
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        source_surface: entry.source_surface.clone(),
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        ts: entry.ts.clone(),
    }
}
