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

use crate::graph::MemoryGraph;
use crate::record::MemoryRecord;
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
        let record: MemoryRecord =
            serde_json::from_str(trimmed).map_err(|source| IngestError::InvalidJson {
                line: idx + 1,
                source,
            })?;
        records.push(record);
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
        format!(
            "{} {} {}",
            self.repo_context,
            self.fault_class.clone().unwrap_or_default(),
            self.rule_id.clone().unwrap_or_default()
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
    let incident = Incident {
        id: id.clone(),
        lesson_id: observation.lesson_id,
        rule_id: observation.rule_id,
        fault_class: observation.fault_class,
        repo_context: observation.repo_context,
        clean: observation.clean,
        source_surface: observation.source_surface,
        ts: observation.ts,
    };
    graph.ingest_incident(incident);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_lines_and_skips_blanks() -> Result<(), Box<dyn std::error::Error>> {
        let text = "\n{\"schemaVersion\":1,\"id\":\"mem-primary-0001\",\"ts\":\"2026-07-04T00:00:00Z\",\"kind\":\"lesson\",\"domain\":\"harness\",\"statement\":\"a\",\"provenance\":{\"writer\":\"primary\"}}\n\n{\"schemaVersion\":1,\"id\":\"mem-primary-0002\",\"ts\":\"2026-07-04T00:00:01Z\",\"kind\":\"decision\",\"domain\":\"code\",\"statement\":\"b\",\"provenance\":{\"writer\":\"primary\"}}\n";
        let records = parse_ndjson(text)?;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "mem-primary-0001");
        assert_eq!(records[1].id, "mem-primary-0002");
        Ok(())
    }

    #[test]
    fn rejects_malformed_line() {
        let text = "{not json}\n";
        let result = parse_ndjson(text);
        match result {
            Err(IngestError::InvalidJson { line, .. }) => assert_eq!(line, 1),
            Ok(_) => unreachable!("malformed line must not parse as valid ndjson"),
        }
    }

    #[test]
    fn observation_seam_records_clean_run_as_negative_evidence() {
        let mut graph = MemoryGraph::new();
        let id = ingest_observation(
            &mut graph,
            Observation {
                lesson_id: "L1".to_string(),
                rule_id: None,
                fault_class: None,
                repo_context: "crates/enforcer-memory".to_string(),
                clean: true,
                source_surface: "scan".to_string(),
                ts: "2026-07-04T00:00:00Z".to_string(),
            },
        );
        assert_eq!(graph.len(), 1);
        assert_eq!(graph.incidents_for_lesson("L1").len(), 1);
        assert!(graph.incidents_for_lesson("L1")[0].clean);
        assert!(id.starts_with("obs-scan-"));
    }
}
