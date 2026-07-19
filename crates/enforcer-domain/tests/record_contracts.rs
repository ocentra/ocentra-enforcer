use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::records::{
    ArtifactKind, ArtifactRef, DiagnosticMessage, DiagnosticRecord, EnforcerEvent, RunEvent,
    ScanEvent, ToolName, SCHEMA_VERSION,
};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::{
    DurationMillis, EpochMillis, FileCount, FindingCount, ProcessExitCode, SourceLine,
};

fn json_err(e: &serde_json::Error) -> DecodeError {
    DecodeError::new("records", e.to_string())
}

fn sample_run() -> Result<RunEvent, DecodeError> {
    Ok(RunEvent {
        schema_version: SCHEMA_VERSION,
        correlation_id: "run-001".parse()?,
        causation_id: None,
        epoch_ms: EpochMillis::new(1_700_000_000_000),
        tool: ToolName::new("cargo".to_owned())?,
        exit_code: ProcessExitCode::new(0),
        duration_ms: DurationMillis::new(1234),
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
        line: SourceLine::try_new(
            std::num::NonZeroU32::new(7)
                .ok_or_else(|| DecodeError::new("sourceLine", "expected positive test line"))?,
        ),
        message: DiagnosticMessage::new("raw string in signature".to_owned())?,
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
    let digest = enforcer_domain::boundary::hash::validate(b"artifact-bytes");
    let event = EnforcerEvent::Artifact(ArtifactRef {
        schema_version: SCHEMA_VERSION,
        correlation_id: "run-003".parse()?,
        causation_id: None,
        path: "proof/cargo/arc-02.txt".parse()?,
        sha256: digest.clone(),
        kind: ArtifactKind::new("proof".to_owned())?,
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
        files_scanned: FileCount::new(420),
        findings: FindingCount::new(3),
        duration_ms: DurationMillis::new(900),
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
    assert_eq!(
        serde_json::from_value::<EnforcerEvent>(unknown)
            .as_ref()
            .err()
            .map(serde_json::Error::classify),
        Some(serde_json::error::Category::Data)
    );

    let bad_id = serde_json::json!({
        "eventType": "run",
        "schemaVersion": 1,
        "correlationId": "has space",
        "epochMs": 0,
        "tool": "cargo",
        "exitCode": 0,
        "durationMs": 1
    });
    assert_eq!(
        serde_json::from_value::<EnforcerEvent>(bad_id)
            .as_ref()
            .err()
            .map(serde_json::Error::classify),
        Some(serde_json::error::Category::Data)
    );
    Ok(())
}
