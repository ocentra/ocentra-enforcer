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
use crate::telemetry_types::{
    DurationMillis, EpochMillis, FindingCount, RecordSchemaVersion, RuleCount, RunCommandName,
    RunRecordKind,
};

/// Current schema version stamped on new `RunRecord`s.
pub const RUN_RECORD_SCHEMA_VERSION: RecordSchemaVersion = RecordSchemaVersion::V1;

/// The fixed `eventType` tag for every `RunRecord` line (kept as an explicit
/// field, not an enum tag, because this NDJSON stream carries exactly one
/// record shape — unlike the mixed `EnforcerEvent` union in `records.rs`).
pub const RUN_RECORD_EVENT_TYPE: RunRecordKind = RunRecordKind::Run;

/// How the enforcer run terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ts_rs::TS)]
#[doc = "Canonical domain representation for ExitStatus."]
pub enum ExitStatus {
    /// Run completed and found no blocking violations.
    Clean,
    /// Run completed and found at least one blocking violation.
    Violations,
    /// Run aborted before completing (crash, panic-free bail, signal).
    Aborted,
}

impl serde::Serialize for ExitStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Clean => "clean",
            Self::Violations => "violations",
            Self::Aborted => "aborted",
        })
    }
}

impl<'de> serde::Deserialize<'de> for ExitStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "clean" => Ok(Self::Clean),
            "violations" => Ok(Self::Violations),
            "aborted" => Ok(Self::Aborted),
            _ => Err(serde::de::Error::custom("invalid exit status")),
        }
    }
}

/// Findings-by-severity counters for one run.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for FindingCounts."]
pub struct FindingCounts {
    /// Count of `Severity::Error` findings.
    pub error: FindingCount,
    /// Count of `Severity::Warning` findings.
    pub warning: FindingCount,
    /// Count of `Severity::Info` findings.
    pub info: FindingCount,
}

impl FindingCounts {
    /// Total findings across all severities.
    pub fn total(&self) -> FindingCount {
        FindingCount::new(self.error.get() + self.warning.get() + self.info.get())
    }

    /// Fold one finding's severity into the running counts.
    pub fn record(&mut self, severity: Severity) {
        match severity {
            Severity::Error => self.error = FindingCount::new(self.error.get().saturating_add(1)),
            Severity::Warning => {
                self.warning = FindingCount::new(self.warning.get().saturating_add(1));
            }
            Severity::Info => self.info = FindingCount::new(self.info.get().saturating_add(1)),
        }
    }
}

/// One enforcer run's telemetry record — exactly one NDJSON line per run.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for RunRecord."]
pub struct RunRecord {
    /// Record schema version.
    pub schema_version: RecordSchemaVersion,
    /// Fixed tag identifying this record shape (`"run"`).
    pub event_type: RunRecordKind,
    /// Milliseconds since the Unix epoch when the run started.
    pub epoch_ms: EpochMillis,
    /// The command/subcommand invoked (e.g. `check`, `scan`).
    pub command: RunCommandName,
    /// Rule ids in scope for this run (deduplicated, order-independent
    /// identity; only the count is telemetered — the DTO stores the count,
    /// not the list, to keep the line small).
    pub rule_ids_in_scope: RuleCount,
    /// Findings grouped by severity.
    pub findings: FindingCounts,
    /// Wall-clock duration of the run.
    pub duration_ms: DurationMillis,
    /// How the run terminated.
    pub exit_status: ExitStatus,
}

/// Constructor parameters for [`RunRecord::new`], grouped into one struct
/// (rather than a long positional argument list) so call sites stay
/// self-describing and clippy's arity lint stays honest, not suppressed.
pub struct RunRecordParams<'a> {
    /// Milliseconds since the Unix epoch when the run started.
    pub epoch_ms: EpochMillis,
    /// The command/subcommand invoked (e.g. `check`, `scan`).
    pub command: RunCommandName,
    /// Rule ids in scope for this run; only the count is telemetered.
    pub rule_ids_in_scope: &'a [RuleId],
    /// Findings grouped by severity.
    pub findings: FindingCounts,
    /// Wall-clock duration of the run.
    pub duration_ms: DurationMillis,
    /// How the run terminated.
    pub exit_status: ExitStatus,
}

impl std::fmt::Debug for RunRecordParams<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RunRecordParams")
            .field("epoch_ms", &self.epoch_ms)
            .field("command", &"[REDACTED]")
            .field("rule_ids_in_scope_count", &self.rule_ids_in_scope.len())
            .field("findings", &self.findings)
            .field("duration_ms", &self.duration_ms)
            .field("exit_status", &self.exit_status)
            .finish()
    }
}

impl RunRecord {
    /// Build a `RunRecord`, computing `ruleIdsInScope` from the given slice
    /// so callers pass the actual scope rather than a bare count (keeps the
    /// call site honest — the count field exists only on the wire).
    pub fn new(params: RunRecordParams<'_>) -> Self {
        Self {
            schema_version: RUN_RECORD_SCHEMA_VERSION,
            event_type: RUN_RECORD_EVENT_TYPE,
            epoch_ms: params.epoch_ms,
            command: params.command,
            // BRAND-INVARIANT: the checked collection length is wrapped as RuleCount immediately.
            rule_ids_in_scope: RuleCount::new(
                u32::try_from(params.rule_ids_in_scope.len()).unwrap_or(u32::MAX),
            ),
            findings: params.findings,
            duration_ms: params.duration_ms,
            exit_status: params.exit_status,
        }
    }
}
