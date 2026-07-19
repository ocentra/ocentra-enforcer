//! Serde decode boundary for durable domain records.
//!
//! Domain modules own the validated shapes and serialization contract. This
//! boundary owns untrusted wire decoding and constructs those shapes only
//! after each branded field has passed its own `Deserialize` implementation.
//! NEGATIVE-TEST: `records` tests reject unknown event tags and malformed
//! branded identifiers before durable domain events are constructed.

use crate::findings::{
    Finding, FindingDetail, FindingLine, FindingTitle, Report, ReportOutcome, ScanScope, Violation,
};
use crate::hashes::Sha256;
use crate::ids::{CausationId, CorrelationId, RuleId};
use crate::memory_types::{
    GraphQueryResultRow, GraphQueryVariable, MemoryAnalysisNodeId, MemoryStorePath,
};
use crate::paths::RelPath;
use crate::records::{
    ArtifactKind, ArtifactRef, DiagnosticMessage, DiagnosticRecord, EnforcerEvent, RunEvent,
    ScanEvent, ToolName,
};
use crate::run_record::{ExitStatus, FindingCounts, RunRecord};
use crate::severity::Severity;
use crate::telemetry_types::{
    DurationMillis, EpochMillis, FileCount, FindingCount, ProcessExitCode, RecordSchemaVersion,
    RuleCount, RunCommandName, RunRecordKind, SourceLine,
};
use serde::{Deserialize, Serialize};

macro_rules! deserialize_via_wire {
    ($domain:ty, $wire:ty) => {
        impl<'de> Deserialize<'de> for $domain {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                <$wire>::deserialize(deserializer).map(Into::into)
            }
        }
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindingWire {
    rule_id: RuleId,
    severity: Severity,
    title: FindingTitle,
    detail: FindingDetail,
    file: RelPath,
    line: FindingLine,
    snippet: Option<crate::findings::FindingSnippet>,
}

impl From<FindingWire> for Finding {
    fn from(wire: FindingWire) -> Self {
        Self {
            rule_id: wire.rule_id,
            severity: wire.severity,
            title: wire.title,
            detail: wire.detail,
            file: wire.file,
            line: wire.line,
            snippet: wire.snippet,
        }
    }
}

deserialize_via_wire!(Finding, FindingWire);

impl<'de> Deserialize<'de> for Violation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let finding = Finding::deserialize(deserializer)?;
        Self::try_from(finding).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportWire {
    ok: ReportOutcome,
    scope: ScanScope,
    violations: Vec<Violation>,
    warnings: Vec<Finding>,
    waived: Vec<Finding>,
    findings: Vec<Finding>,
}

impl From<ReportWire> for Report {
    fn from(wire: ReportWire) -> Self {
        Self {
            ok: wire.ok,
            scope: wire.scope,
            violations: wire.violations,
            warnings: wire.warnings,
            waived: wire.waived,
            findings: wire.findings,
        }
    }
}

deserialize_via_wire!(Report, ReportWire);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunEventWire {
    schema_version: RecordSchemaVersion,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    epoch_ms: EpochMillis,
    tool: ToolName,
    exit_code: ProcessExitCode,
    duration_ms: DurationMillis,
}

impl From<RunEventWire> for RunEvent {
    fn from(wire: RunEventWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            correlation_id: wire.correlation_id,
            causation_id: wire.causation_id,
            epoch_ms: wire.epoch_ms,
            tool: wire.tool,
            exit_code: wire.exit_code,
            duration_ms: wire.duration_ms,
        }
    }
}

deserialize_via_wire!(RunEvent, RunEventWire);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticRecordWire {
    schema_version: RecordSchemaVersion,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    rule_id: RuleId,
    severity: Severity,
    file: RelPath,
    line: SourceLine,
    message: DiagnosticMessage,
}

impl From<DiagnosticRecordWire> for DiagnosticRecord {
    fn from(wire: DiagnosticRecordWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            correlation_id: wire.correlation_id,
            causation_id: wire.causation_id,
            rule_id: wire.rule_id,
            severity: wire.severity,
            file: wire.file,
            line: wire.line,
            message: wire.message,
        }
    }
}

deserialize_via_wire!(DiagnosticRecord, DiagnosticRecordWire);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactRefWire {
    schema_version: RecordSchemaVersion,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    path: RelPath,
    sha256: Sha256,
    kind: ArtifactKind,
}

impl From<ArtifactRefWire> for ArtifactRef {
    fn from(wire: ArtifactRefWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            correlation_id: wire.correlation_id,
            causation_id: wire.causation_id,
            path: wire.path,
            sha256: wire.sha256,
            kind: wire.kind,
        }
    }
}

deserialize_via_wire!(ArtifactRef, ArtifactRefWire);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanEventWire {
    schema_version: RecordSchemaVersion,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    scope: ScanScope,
    files_scanned: FileCount,
    findings: FindingCount,
    duration_ms: DurationMillis,
}

impl From<ScanEventWire> for ScanEvent {
    fn from(wire: ScanEventWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            correlation_id: wire.correlation_id,
            causation_id: wire.causation_id,
            scope: wire.scope,
            files_scanned: wire.files_scanned,
            findings: wire.findings,
            duration_ms: wire.duration_ms,
        }
    }
}

deserialize_via_wire!(ScanEvent, ScanEventWire);

#[derive(Deserialize)]
#[serde(tag = "eventType", rename_all = "camelCase")]
enum EnforcerEventWire {
    Run(RunEvent),
    Diagnostic(DiagnosticRecord),
    Artifact(ArtifactRef),
    Scan(ScanEvent),
}

impl From<EnforcerEventWire> for EnforcerEvent {
    fn from(wire: EnforcerEventWire) -> Self {
        match wire {
            EnforcerEventWire::Run(event) => Self::Run(event),
            EnforcerEventWire::Diagnostic(event) => Self::Diagnostic(event),
            EnforcerEventWire::Artifact(event) => Self::Artifact(event),
            EnforcerEventWire::Scan(event) => Self::Scan(event),
        }
    }
}

deserialize_via_wire!(EnforcerEvent, EnforcerEventWire);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRecordWire {
    schema_version: RecordSchemaVersion,
    event_type: RunRecordKind,
    epoch_ms: EpochMillis,
    command: RunCommandName,
    rule_ids_in_scope: RuleCount,
    findings: FindingCounts,
    duration_ms: DurationMillis,
    exit_status: ExitStatus,
}

impl From<RunRecordWire> for RunRecord {
    fn from(wire: RunRecordWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            event_type: wire.event_type,
            epoch_ms: wire.epoch_ms,
            command: wire.command,
            rule_ids_in_scope: wire.rule_ids_in_scope,
            findings: wire.findings,
            duration_ms: wire.duration_ms,
            exit_status: wire.exit_status,
        }
    }
}

deserialize_via_wire!(RunRecord, RunRecordWire);

impl Serialize for GraphQueryResultRow {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.iter())
    }
}

impl<'de> Deserialize<'de> for GraphQueryResultRow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        std::collections::BTreeMap::<GraphQueryVariable, MemoryAnalysisNodeId>::deserialize(
            deserializer,
        )
        .map(Into::into)
    }
}

impl Serialize for MemoryStorePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_path().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MemoryStorePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        std::path::PathBuf::deserialize(deserializer).map(Into::into)
    }
}
