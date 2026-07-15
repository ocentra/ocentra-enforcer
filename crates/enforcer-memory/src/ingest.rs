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

use crate::error::Result as MemoryResult;
use crate::graph::MemoryGraph;
use crate::record::{MemoryRecord, MemoryRecordDto};
use crate::schema::{ObservationLogEntry, SCHEMA_VERSION};
use crate::store::Store;
use thiserror::Error;

/// Errors from parsing an NDJSON memory-record stream.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error("line {line}: invalid JSON: {source}")]
    InvalidJson {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}

/// Parse a full NDJSON document (one [`MemoryRecord`] per non-blank
/// line) into records, in file order. A malformed line is a hard error
/// — this is an append-only audit log; a corrupt line must not be
/// silently skipped.
pub fn parse_ndjson(text: &str) -> Result<Vec<MemoryRecord>, IngestError> {
    let mut records = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let dto: MemoryRecordDto =
            serde_json::from_str(trimmed).map_err(|source| IngestError::InvalidJson {
                line: idx + 1,
                source,
            })?;
        records.push(MemoryRecord::from_dto(dto));
    }
    Ok(records)
}

/// Ingest an NDJSON document's records into `graph`. Returns the number
/// of records ingested.
pub fn ingest_ndjson_into(graph: &mut MemoryGraph, text: &str) -> Result<usize, IngestError> {
    let records = parse_ndjson(text)?;
    let count = records.len();
    for record in records {
        graph.ingest_record(record);
    }
    Ok(count)
}

/// One fault occurrence: the "Incident node" the workpack's
/// usage-ingestion requirement describes — `finding/fault-class/ruleId/
/// repo-context -> Incident node + observedIn edges`. A clean run still
/// produces an `Incident` with `clean = true` (negative evidence): usage
/// is learning even when nothing is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incident {
    /// Stable id for this observation, e.g. `obs-<writer>-<n>`.
    pub id: String,
    /// The rule/lesson this observation is evidence for or against.
    /// Empty string when the observation is not yet linked to a lesson.
    pub lesson_id: String,
    /// Rule id the finding fired on, if any (`ruleId` in the workpack's
    /// contract). `None` for a clean scan with no findings at all.
    pub rule_id: Option<String>,
    /// Fault class / finding category, free text (e.g. `"unwrap_used"`).
    pub fault_class: Option<String>,
    /// Repo-relative path or module the observation concerns.
    pub repo_context: String,
    /// `true` when this observation is a clean run recording the
    /// ABSENCE of the fault class (negative evidence), `false` when it
    /// is an actual finding.
    pub clean: bool,
    /// Where this observation came from: `scan`, `check`, `run`,
    /// `doctor`, `closeout`, matching the workpack's named call sites.
    pub source_surface: String,
    /// Opaque timestamp, ISO-8601 string (no parsing needed by this
    /// crate — callers already have a clock; we just record it).
    pub ts: String,
}

impl Incident {
    pub fn searchable_text(&self) -> String {
        let fault_class = match self.fault_class.as_deref() {
            Some(value) => value,
            None => "",
        };
        let rule_id = match self.rule_id.as_deref() {
            Some(value) => value,
            None => "",
        };
        format!(
            "{} {} {}",
            self.repo_context,
            fault_class,
            rule_id
        )
    }
}

/// Parameters for one call into the usage-ingestion seam. Mirrors the
/// workpack contract literally: "finding/fault-class/ruleId/repo-context
/// -> Incident node + observedIn edges".
#[derive(Debug, Clone)]
pub struct Observation {
    pub lesson_id: String,
    pub rule_id: Option<String>,
    pub fault_class: Option<String>,
    pub repo_context: String,
    pub clean: bool,
    pub source_surface: String,
    pub ts: String,
}

/// The usage-ingestion seam: every enforcement operation (scan/check/run/
/// doctor/closeout) calls this on every run — no manual capture step.
/// Append-only: this always creates a new [`Incident`] node, it never
/// edits or removes an existing one. Returns the id of the incident
/// created so the caller can, e.g., surface it in a run's proof journal.
pub fn ingest_observation(graph: &mut MemoryGraph, observation: Observation) -> String {
    let id = format!("obs-{}-{:04}", observation.source_surface, graph.len());
    // CLONE-JUSTIFICATION: the graph consumes the incident while the caller receives its id.
    let incident = incident_from_observation(id.clone(), observation);
    graph.ingest_incident(incident);
    id
}

pub fn append_observation_to_store(
    store: &mut Store,
    observation: Observation,
) -> MemoryResult<String> {
    append_observation_payload_to_store(store, observation, None, None)
}

pub fn append_observation_payload_to_store(
    store: &mut Store,
    observation: Observation,
    payload_kind: Option<String>,
    payload: Option<serde_json::Value>,
) -> MemoryResult<String> {
    let mut assigned_id = String::new();
    store.append_observation_entry(|seq| {
        let id = format!("obs-{}-{seq:04}", observation.source_surface);
        // CLONE-JUSTIFICATION: the durable log entry owns its id while the caller receives it after append.
        assigned_id = id.clone();
        ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id,
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
) -> MemoryResult<String> {
    ingest_observation_payload_into_store(store, graph, observation, None, None)
}

pub fn ingest_observation_payload_into_store(
    store: &mut Store,
    graph: &mut MemoryGraph,
    observation: Observation,
    payload_kind: Option<String>,
    payload: Option<serde_json::Value>,
) -> MemoryResult<String> {
    let id = append_observation_payload_to_store(store, observation, payload_kind, payload)?;
    let stored = store.read_observation_entries()?;
    if let Some(entry) = stored.entries.into_iter().find(|entry| entry.id == id) {
        graph.ingest_incident(incident_from_entry(&entry));
    }
    Ok(id)
}

pub fn replay_incident_observations_from_store(
    store: &Store,
    graph: &mut MemoryGraph,
) -> MemoryResult<usize> {
    let outcome = store.read_observation_entries()?;
    let mut count = 0;
    for entry in outcome.entries {
        if matches!(
            entry.payload_kind.as_deref(),
            Some("procedural-memory") | Some("route-choice")
        ) {
            continue;
        }
        if graph.nodes().iter().any(|node| node.id() == entry.id) {
            continue;
        }
        graph.ingest_incident(incident_from_entry(&entry));
        count += 1;
    }
    Ok(count)
}

fn incident_from_observation(id: String, observation: Observation) -> Incident {
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

fn incident_from_entry(entry: &ObservationLogEntry) -> Incident {
    Incident {
        // CLONE-JUSTIFICATION: the graph-owned incident outlives this borrowed durable log entry.
        id: entry.id.clone(),
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
