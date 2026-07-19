//! Versioned telemetry/audit RECORDS (OcentraParent "Logging = structured
//! data" borrow). DTO shape ONLY: these records RIDE the `enforcer-core`
//! NDJSON sink / hash-chain / redaction mechanisms (arc-01) and the
//! `enforcer-events` envelope (arc-25).
//!
//! Every record carries `schemaVersion`; the [`EnforcerEvent`] union is
//! internally tagged on `eventType`. camelCase wire casing (locked
//! decision).

use crate::findings::ScanScope;
use crate::hashes::Sha256;
use crate::ids::{CausationId, CorrelationId, RuleId};
use crate::paths::RelPath;
use crate::severity::Severity;
use crate::telemetry_types::{
    DurationMillis, EpochMillis, FileCount, FindingCount, ProcessExitCode, RecordSchemaVersion,
    SourceLine,
};

/// Current schema version stamped on new records.
pub const SCHEMA_VERSION: RecordSchemaVersion = RecordSchemaVersion::V1;

macro_rules! record_text {
    ($(#[$doc:meta])* $name:ident, $field:literal) => {
        $(#[$doc])*
        // SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
        #[serde(transparent)]
        #[ts(type = "string")]
        pub struct $name(String);

        impl $name {
            /// Validate record text, rejecting invalid blank or control-bearing input.
            pub fn new(value: String) -> Result<Self, crate::boundary::decode_error::DecodeError> {
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(crate::boundary::decode_error::DecodeError::new(
                        $field,
                        "must be non-empty printable text",
                    ));
                }
                Ok(Self(value))
            }

            #[must_use]
            #[doc = "The as_str operation for this canonical domain value."]
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl TryFrom<String> for $name {
            type Error = crate::boundary::decode_error::DecodeError;
            fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) }
        }

        impl std::str::FromStr for $name {
            type Err = crate::boundary::decode_error::DecodeError;
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            fn from_str(value: &str) -> Result<Self, Self::Err> { Self::new(value.to_owned()) }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

record_text!(
    #[doc = "Validated name of a tool recorded in a run event."]
    ToolName,
    "tool"
);
record_text!(
    #[doc = "Validated, pre-redacted diagnostic text recorded at the boundary."]
    DiagnosticMessage,
    "diagnosticMessage"
);
record_text!(
    #[doc = "Validated classification of a content-addressed artifact."]
    ArtifactKind,
    "artifactKind"
);

/// A tool/run execution record (d04 run-telemetry rides this).
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for RunEvent."]
pub struct RunEvent {
    /// Record schema version.
    pub schema_version: RecordSchemaVersion,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// Milliseconds since the Unix epoch.
    pub epoch_ms: EpochMillis,
    /// Tool that ran (e.g. `cargo`, `tsc`).
    pub tool: ToolName,
    /// Process exit code.
    pub exit_code: ProcessExitCode,
    /// Wall-clock duration.
    pub duration_ms: DurationMillis,
}

/// One diagnostic occurrence in structured form.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for DiagnosticRecord."]
pub struct DiagnosticRecord {
    /// Record schema version.
    pub schema_version: RecordSchemaVersion,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// Rule that produced the diagnostic.
    pub rule_id: RuleId,
    /// Severity of the diagnostic.
    pub severity: Severity,
    /// Repo-relative file.
    pub file: RelPath,
    /// 1-based line number.
    pub line: SourceLine,
    /// Human message (already redacted upstream).
    pub message: DiagnosticMessage,
}

/// Reference to a produced artifact, content-addressed.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for ArtifactRef."]
pub struct ArtifactRef {
    /// Record schema version.
    pub schema_version: RecordSchemaVersion,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// Repo-relative artifact path.
    pub path: RelPath,
    /// Content digest.
    pub sha256: Sha256,
    /// Artifact kind (e.g. `proof`, `report`, `export`).
    pub kind: ArtifactKind,
}

/// Scan lifecycle summary record.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[doc = "Canonical domain representation for ScanEvent."]
pub struct ScanEvent {
    /// Record schema version.
    pub schema_version: RecordSchemaVersion,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// What the scan covered.
    pub scope: ScanScope,
    /// Files scanned.
    pub files_scanned: FileCount,
    /// Findings produced.
    pub findings: FindingCount,
    /// Wall-clock duration.
    pub duration_ms: DurationMillis,
}

/// The tagged union of all enforcer records, internally tagged on
/// `eventType` so one NDJSON stream can carry mixed record kinds and
/// consumers route on the tag.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(tag = "eventType", rename_all = "camelCase")]
#[doc = "Canonical domain representation for EnforcerEvent."]
pub enum EnforcerEvent {
    /// Tool/run execution record.
    Run(RunEvent),
    /// Structured diagnostic occurrence.
    Diagnostic(DiagnosticRecord),
    /// Content-addressed artifact reference.
    Artifact(ArtifactRef),
    /// Scan lifecycle summary.
    Scan(ScanEvent),
}
