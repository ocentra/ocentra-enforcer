// contractHash: run_record_contracts.rs
// sourceOwner: enforcer-domain
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::run_record::{
    ExitStatus, FindingCounts, RunRecord, RunRecordParams, RUN_RECORD_EVENT_TYPE,
    RUN_RECORD_SCHEMA_VERSION,
};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::{DurationMillis, EpochMillis, RunCommandName};

fn rule(id: &str) -> Result<RuleId, DecodeError> {
    RuleId::try_from(id.to_owned())
}

#[test]
fn new_computes_scope_count_from_slice() -> Result<(), DecodeError> {
    let scope = vec![rule("RR-6.1")?, rule("DEP-1.1")?];
    let mut findings = FindingCounts::default();
    findings.record(Severity::Error);
    findings.record(Severity::Warning);
    let record = RunRecord::new(RunRecordParams {
        epoch_ms: EpochMillis::new(1_700_000_000_000),
        command: RunCommandName::try_new("check".to_owned())?,
        rule_ids_in_scope: &scope,
        findings,
        duration_ms: DurationMillis::new(42),
        exit_status: ExitStatus::Violations,
    });
    assert_eq!(record.rule_ids_in_scope.get(), 2);
    assert_eq!(record.schema_version, RUN_RECORD_SCHEMA_VERSION);
    assert_eq!(record.event_type, RUN_RECORD_EVENT_TYPE);
    Ok(())
}

#[test]
fn wire_form_is_camel_case_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let scope = vec![rule("RR-6.1")?];
    let record = RunRecord::new(RunRecordParams {
        epoch_ms: EpochMillis::new(1_700_000_000_000),
        command: RunCommandName::try_new("scan".to_owned())?,
        rule_ids_in_scope: &scope,
        findings: FindingCounts::default(),
        duration_ms: DurationMillis::new(10),
        exit_status: ExitStatus::Clean,
    });
    let wire = enforcer_domain::boundary::json::to_value(&record)?;
    assert_eq!(wire["schemaVersion"], 1);
    assert_eq!(wire["eventType"], "run");
    assert_eq!(wire["ruleIdsInScope"], 1);
    assert_eq!(wire["durationMs"], 10);
    assert_eq!(wire["exitStatus"], "clean");
    assert!(wire.get("rule_ids_in_scope").is_none());
    let back: RunRecord = enforcer_domain::boundary::json::from_value(wire)?;
    assert_eq!(back, record);
    Ok(())
}

#[test]
fn boundary_rejects_unknown_exit_status() -> Result<(), Box<dyn std::error::Error>> {
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
    let rejection = enforcer_domain::boundary::json::from_value::<RunRecord>(bad)
        .err()
        .ok_or("unknown exit status must be rejected")?;
    assert_eq!(rejection.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn finding_counts_total_sums_all_severities() {
    let mut counts = FindingCounts::default();
    counts.record(Severity::Error);
    counts.record(Severity::Error);
    counts.record(Severity::Warning);
    counts.record(Severity::Info);
    assert_eq!(counts.total().get(), 4);
    assert_eq!(counts.error.get(), 2);
    assert_eq!(counts.warning.get(), 1);
    assert_eq!(counts.info.get(), 1);
}
