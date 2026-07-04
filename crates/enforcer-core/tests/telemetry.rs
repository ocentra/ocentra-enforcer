//! d04 run-telemetry integration proof: exercises
//! `enforcer_core::telemetry::RunTelemetrySink` against the concrete
//! `enforcer_domain::run_record::RunRecord` shape.
//!
//! `enforcer-domain` is a dev-dependency of `enforcer-core` here ONLY (see
//! the note in `Cargo.toml`); the library build graph stays acyclic since
//! `enforcer-domain` depends on `enforcer-core`, not the reverse.

use enforcer_core::error::Result;
use enforcer_core::telemetry::{verify_file_chain, RunTelemetrySink, DEFAULT_RUN_TELEMETRY_PATH};
use enforcer_domain::run_record::{ExitStatus, FindingCounts, RunRecord, RunRecordParams};

fn temp_path(name: &str) -> std::path::PathBuf {
    let unique = format!(
        "enforcer-core-telemetry-it-{}-{}-{name}.ndjson",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn sample_record(seq: u32, exit_status: ExitStatus) -> RunRecord {
    let findings = FindingCounts {
        error: seq,
        ..FindingCounts::default()
    };
    RunRecord::new(RunRecordParams {
        epoch_ms: 1_700_000_000_000 + u64::from(seq),
        command: "check".to_owned(),
        rule_ids_in_scope: &[],
        findings,
        duration_ms: 100 + u64::from(seq),
        exit_status,
    })
}

#[test]
fn default_path_matches_the_workpack_specified_location() {
    assert_eq!(DEFAULT_RUN_TELEMETRY_PATH, "proof/telemetry/runs.ndjson");
}

#[test]
fn a_scripted_run_appends_exactly_one_valid_ndjson_line() -> Result<()> {
    let path = temp_path("single-run");
    {
        let mut sink: RunTelemetrySink<RunRecord> = RunTelemetrySink::open(&path)?;
        sink.append(&sample_record(1, ExitStatus::Clean))?;
    }
    let raw = std::fs::read_to_string(&path)?;
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one NDJSON line per run");
    let decoded: RunRecord = serde_json::from_str(lines[0])?;
    assert_eq!(decoded, sample_record(1, ExitStatus::Clean));
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn two_runs_append_two_independently_parseable_lines_and_hash_chain_verifies_on_replay(
) -> Result<()> {
    let path = temp_path("two-runs");
    {
        let mut sink: RunTelemetrySink<RunRecord> = RunTelemetrySink::open(&path)?;
        sink.append(&sample_record(1, ExitStatus::Clean))?;
        sink.append(&sample_record(2, ExitStatus::Violations))?;
    }
    let raw = std::fs::read_to_string(&path)?;
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2);
    // Each line is independently parseable (no shared state required).
    let first: RunRecord = serde_json::from_str(lines[0])?;
    let second: RunRecord = serde_json::from_str(lines[1])?;
    assert_eq!(first.exit_status, ExitStatus::Clean);
    assert_eq!(second.exit_status, ExitStatus::Violations);

    let outcome = verify_file_chain(&path)?;
    assert_eq!(outcome, Ok(2), "hash-chain must verify cleanly on replay");
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn a_forced_schema_violation_fixture_is_rejected_on_decode() -> Result<()> {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/telemetry");
    let valid_raw = std::fs::read_to_string(fixtures.join("valid_run_record.json"))?;
    let invalid_raw =
        std::fs::read_to_string(fixtures.join("invalid_run_record_bad_exit_status.json"))?;

    let valid: std::result::Result<RunRecord, _> = serde_json::from_str(&valid_raw);
    assert!(valid.is_ok(), "the valid fixture must decode: {valid:?}");

    let invalid: std::result::Result<RunRecord, _> = serde_json::from_str(&invalid_raw);
    assert!(
        invalid.is_err(),
        "the schema-violation fixture (bad exitStatus) must be rejected on decode"
    );
    Ok(())
}

#[test]
fn telemetry_never_influences_findings_or_a_would_be_exit_code() -> Result<()> {
    // Telemetry emission is an observer: appending a record about a run with
    // violations does not itself produce, mutate, or clear any finding, and
    // the sink API has no exit-code-shaped return value to smuggle one
    // through.
    let path = temp_path("observer");
    let record = sample_record(3, ExitStatus::Violations);
    let findings_before = record.findings;
    {
        let mut sink: RunTelemetrySink<RunRecord> = RunTelemetrySink::open(&path)?;
        sink.append(&record)?;
    }
    // The record we hold is untouched by the append call (value semantics),
    // and re-reading it back off disk reproduces the same findings.
    assert_eq!(record.findings, findings_before);
    let raw = std::fs::read_to_string(&path)?;
    let decoded: RunRecord = serde_json::from_str(raw.lines().next().unwrap_or_default())?;
    assert_eq!(decoded.findings, findings_before);
    std::fs::remove_file(&path)?;
    Ok(())
}
