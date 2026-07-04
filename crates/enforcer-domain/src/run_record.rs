//! `RunRecord` — the versioned per-run telemetry DTO (d04). DTO shape ONLY:
//! this record RIDES the `enforcer-core` NDJSON sink / hash-chain /
//! redaction mechanisms (arc-01) via the `enforcer-core::telemetry` sink.
//!
//! One `RunRecord` is appended per enforcer run to `proof/telemetry/runs.ndjson`
//! (mechanism owned by `enforcer-core::telemetry`, per the d04 workpack).
//! Every field is either a primitive, a branded newtype, or a closed enum —
//! never a bare `String` for an id. camelCase wire casing (locked decision).

use crate::ids::RuleId;
use crate::severity::Severity;

/// Current schema version stamped on new `RunRecord`s.
pub const RUN_RECORD_SCHEMA_VERSION: u32 = 1;

/// The fixed `eventType` tag for every `RunRecord` line (kept as an explicit
/// field, not an enum tag, because this NDJSON stream carries exactly one
/// record shape — unlike the mixed `EnforcerEvent` union in `records.rs`).
pub const RUN_RECORD_EVENT_TYPE: &str = "run";

/// How the enforcer run terminated.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
pub enum ExitStatus {
    /// Run completed and found no blocking violations.
    Clean,
    /// Run completed and found at least one blocking violation.
    Violations,
    /// Run aborted before completing (crash, panic-free bail, signal).
    Aborted,
}

/// Findings-by-severity counters for one run.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
pub struct FindingCounts {
    /// Count of `Severity::Error` findings.
    pub error: u32,
    /// Count of `Severity::Warning` findings.
    pub warning: u32,
    /// Count of `Severity::Info` findings.
    pub info: u32,
}

impl FindingCounts {
    /// Total findings across all severities.
    pub fn total(&self) -> u32 {
        self.error + self.warning + self.info
    }

    /// Fold one finding's severity into the running counts.
    pub fn record(&mut self, severity: Severity) {
        match severity {
            Severity::Error => self.error += 1,
            Severity::Warning => self.warning += 1,
            Severity::Info => self.info += 1,
        }
    }
}

/// One enforcer run's telemetry record — exactly one NDJSON line per run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    /// Record schema version.
    pub schema_version: u32,
    /// Fixed tag identifying this record shape (`"run"`).
    pub event_type: String,
    /// Milliseconds since the Unix epoch when the run started.
    pub epoch_ms: u64,
    /// The command/subcommand invoked (e.g. `check`, `scan`).
    pub command: String,
    /// Rule ids in scope for this run (deduplicated, order-independent
    /// identity; only the count is telemetered — the DTO stores the count,
    /// not the list, to keep the line small).
    pub rule_ids_in_scope: u32,
    /// Findings grouped by severity.
    pub findings: FindingCounts,
    /// Wall-clock duration of the run.
    pub duration_ms: u64,
    /// How the run terminated.
    pub exit_status: ExitStatus,
}

/// Constructor parameters for [`RunRecord::new`], grouped into one struct
/// (rather than a long positional argument list) so call sites stay
/// self-describing and clippy's arity lint stays honest, not suppressed.
pub struct RunRecordParams<'a> {
    /// Milliseconds since the Unix epoch when the run started.
    pub epoch_ms: u64,
    /// The command/subcommand invoked (e.g. `check`, `scan`).
    pub command: String,
    /// Rule ids in scope for this run; only the count is telemetered.
    pub rule_ids_in_scope: &'a [RuleId],
    /// Findings grouped by severity.
    pub findings: FindingCounts,
    /// Wall-clock duration of the run.
    pub duration_ms: u64,
    /// How the run terminated.
    pub exit_status: ExitStatus,
}

impl RunRecord {
    /// Build a `RunRecord`, computing `ruleIdsInScope` from the given slice
    /// so callers pass the actual scope rather than a bare count (keeps the
    /// call site honest — the count field exists only on the wire).
    pub fn new(params: RunRecordParams<'_>) -> Self {
        Self {
            schema_version: RUN_RECORD_SCHEMA_VERSION,
            event_type: RUN_RECORD_EVENT_TYPE.to_owned(),
            epoch_ms: params.epoch_ms,
            command: params.command,
            rule_ids_in_scope: params.rule_ids_in_scope.len() as u32,
            findings: params.findings,
            duration_ms: params.duration_ms,
            exit_status: params.exit_status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExitStatus, FindingCounts, RunRecord, RunRecordParams, RUN_RECORD_EVENT_TYPE,
        RUN_RECORD_SCHEMA_VERSION,
    };
    use crate::ids::RuleId;
    use crate::severity::Severity;
    use enforcer_core::error::DecodeError;

    fn rule(id: &str) -> Result<RuleId, DecodeError> {
        id.parse()
    }

    #[test]
    fn new_computes_scope_count_from_slice() -> Result<(), DecodeError> {
        let scope = vec![rule("RR-6.1")?, rule("DEP-1.1")?];
        let mut findings = FindingCounts::default();
        findings.record(Severity::Error);
        findings.record(Severity::Warning);
        let record = RunRecord::new(RunRecordParams {
            epoch_ms: 1_700_000_000_000,
            command: "check".to_owned(),
            rule_ids_in_scope: &scope,
            findings,
            duration_ms: 42,
            exit_status: ExitStatus::Violations,
        });
        assert_eq!(record.rule_ids_in_scope, 2);
        assert_eq!(record.schema_version, RUN_RECORD_SCHEMA_VERSION);
        assert_eq!(record.event_type, RUN_RECORD_EVENT_TYPE);
        Ok(())
    }

    #[test]
    fn wire_form_is_camel_case_and_round_trips() -> Result<(), DecodeError> {
        let scope = vec![rule("RR-6.1")?];
        let record = RunRecord::new(RunRecordParams {
            epoch_ms: 1_700_000_000_000,
            command: "scan".to_owned(),
            rule_ids_in_scope: &scope,
            findings: FindingCounts::default(),
            duration_ms: 10,
            exit_status: ExitStatus::Clean,
        });
        let wire = serde_json::to_value(&record)
            .map_err(|e| DecodeError::new("runRecord", e.to_string()))?;
        assert_eq!(wire["schemaVersion"], 1);
        assert_eq!(wire["eventType"], "run");
        assert_eq!(wire["ruleIdsInScope"], 1);
        assert_eq!(wire["durationMs"], 10);
        assert_eq!(wire["exitStatus"], "clean");
        assert!(wire.get("rule_ids_in_scope").is_none());
        let back: RunRecord = serde_json::from_value(wire)
            .map_err(|e| DecodeError::new("runRecord", e.to_string()))?;
        assert_eq!(back, record);
        Ok(())
    }

    #[test]
    fn boundary_rejects_unknown_exit_status() {
        let bad = serde_json::json!({
            "schemaVersion": 1,
            "eventType": "run",
            "epochMs": 0,
            "command": "check",
            "ruleIdsInScope": 0,
            "findings": { "error": 0, "warning": 0, "info": 0 },
            "durationMs": 1,
            "exitStatus": "mystery"
        });
        assert!(serde_json::from_value::<RunRecord>(bad).is_err());
    }

    #[test]
    fn finding_counts_total_sums_all_severities() {
        let mut counts = FindingCounts::default();
        counts.record(Severity::Error);
        counts.record(Severity::Error);
        counts.record(Severity::Warning);
        counts.record(Severity::Info);
        assert_eq!(counts.total(), 4);
        assert_eq!(counts.error, 2);
        assert_eq!(counts.warning, 1);
        assert_eq!(counts.info, 1);
    }
}
