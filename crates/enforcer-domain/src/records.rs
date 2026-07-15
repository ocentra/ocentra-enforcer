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

/// Current schema version stamped on new records.
pub const SCHEMA_VERSION: u32 = 1;

/// A tool/run execution record (d04 run-telemetry rides this).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    /// Record schema version.
    pub schema_version: u32,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// Milliseconds since the Unix epoch.
    pub epoch_ms: u64,
    /// Tool that ran (e.g. `cargo`, `tsc`).
    pub tool: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration.
    pub duration_ms: u64,
}

/// One diagnostic occurrence in structured form.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRecord {
    /// Record schema version.
    pub schema_version: u32,
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
    pub line: u32,
    /// Human message (already redacted upstream).
    pub message: String,
}

/// Reference to a produced artifact, content-addressed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    /// Record schema version.
    pub schema_version: u32,
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
    pub kind: String,
}

/// Scan lifecycle summary record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanEvent {
    /// Record schema version.
    pub schema_version: u32,
    /// Flow correlation id.
    pub correlation_id: CorrelationId,
    /// Optional causing-event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<CausationId>,
    /// What the scan covered.
    pub scope: ScanScope,
    /// Files scanned.
    pub files_scanned: u64,
    /// Findings produced.
    pub findings: u64,
    /// Wall-clock duration.
    pub duration_ms: u64,
}

/// The tagged union of all enforcer records, internally tagged on
/// `eventType` so one NDJSON stream can carry mixed record kinds and
/// consumers route on the tag.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(tag = "eventType", rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
    use super::{
        ArtifactRef, DiagnosticRecord, EnforcerEvent, RunEvent, ScanEvent, SCHEMA_VERSION,
    };
    use crate::boundary::decode_error::DecodeError;
    use crate::findings::ScanScope;
    use crate::hashes::Sha256;
    use crate::severity::Severity;

    fn json_err(e: &serde_json::Error) -> DecodeError {
        DecodeError::new("records", e.to_string())
    }

    fn sample_run() -> Result<RunEvent, DecodeError> {
        Ok(RunEvent {
            schema_version: SCHEMA_VERSION,
            correlation_id: "run-001".parse()?,
            causation_id: None,
            epoch_ms: 1_700_000_000_000,
            tool: "cargo".to_owned(),
            exit_code: 0,
            duration_ms: 1234,
        })
    }

    #[test]
    fn run_event_round_trips_with_camel_case_and_version() -> Result<(), DecodeError> {
        let event = EnforcerEvent::Run(sample_run()?);
        let wire = serde_json::to_value(&event).map_err(|e| json_err(&e))?;
        assert_eq!(wire["eventType"], "run");
        assert_eq!(wire["schemaVersion"], 1);
        assert!(wire.get("schema_version").is_none());
        assert_eq!(wire["correlationId"], "run-001");
        assert_eq!(wire["durationMs"], 1234);
        let back: EnforcerEvent = serde_json::from_value(wire).map_err(|e| json_err(&e))?;
        assert_eq!(back, event);
        Ok(())
    }

    #[test]
    fn diagnostic_record_round_trips() -> Result<(), DecodeError> {
        let event = EnforcerEvent::Diagnostic(DiagnosticRecord {
            schema_version: SCHEMA_VERSION,
            correlation_id: "run-002".parse()?,
            causation_id: Some("run-001".parse()?),
            rule_id: "RR-6.1".parse()?,
            severity: Severity::Error,
            file: "src/lib.rs".parse()?,
            line: 7,
            message: "raw string in signature".to_owned(),
        });
        let wire = serde_json::to_value(&event).map_err(|e| json_err(&e))?;
        assert_eq!(wire["eventType"], "diagnostic");
        assert_eq!(wire["ruleId"], "RR-6.1");
        assert_eq!(wire["causationId"], "run-001");
        let back: EnforcerEvent = serde_json::from_value(wire).map_err(|e| json_err(&e))?;
        assert_eq!(back, event);
        Ok(())
    }

    #[test]
    fn artifact_ref_round_trips_with_branded_digest() -> Result<(), DecodeError> {
        let digest = Sha256::of(b"artifact-bytes");
        let event = EnforcerEvent::Artifact(ArtifactRef {
            schema_version: SCHEMA_VERSION,
            correlation_id: "run-003".parse()?,
            causation_id: None,
            path: "proof/cargo/arc-02.txt".parse()?,
            sha256: digest.clone(),
            kind: "proof".to_owned(),
        });
        let wire = serde_json::to_value(&event).map_err(|e| json_err(&e))?;
        assert_eq!(wire["eventType"], "artifact");
        assert_eq!(wire["sha256"], digest.as_str());
        let back: EnforcerEvent = serde_json::from_value(wire).map_err(|e| json_err(&e))?;
        assert_eq!(back, event);
        Ok(())
    }

    #[test]
    fn scan_event_round_trips() -> Result<(), DecodeError> {
        let event = EnforcerEvent::Scan(ScanEvent {
            schema_version: SCHEMA_VERSION,
            correlation_id: "run-004".parse()?,
            causation_id: None,
            scope: ScanScope::Workspace,
            files_scanned: 420,
            findings: 3,
            duration_ms: 900,
        });
        let wire = serde_json::to_value(&event).map_err(|e| json_err(&e))?;
        assert_eq!(wire["eventType"], "scan");
        assert_eq!(wire["scope"], "workspace");
        let back: EnforcerEvent = serde_json::from_value(wire).map_err(|e| json_err(&e))?;
        assert_eq!(back, event);
        Ok(())
    }

    #[test]
    fn boundary_rejects_unknown_event_type_and_bad_ids() -> Result<(), DecodeError> {
        let unknown = serde_json::json!({
            "eventType": "mystery",
            "schemaVersion": 1,
            "correlationId": "run-005"
        });
        assert!(serde_json::from_value::<EnforcerEvent>(unknown).is_err());

        let bad_id = serde_json::json!({
            "eventType": "run",
            "schemaVersion": 1,
            "correlationId": "has space",
            "epochMs": 0,
            "tool": "cargo",
            "exitCode": 0,
            "durationMs": 1
        });
        assert!(serde_json::from_value::<EnforcerEvent>(bad_id).is_err());
        Ok(())
    }
}
