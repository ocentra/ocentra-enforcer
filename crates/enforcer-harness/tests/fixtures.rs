//! Cross-cutting integration fixtures for arc-18 (`enforcer-harness`):
//! run-storage layout, manifest replace-not-duplicate, and legacy
//! `.ocentra-enforcer` dual-read/dedupe. Unit-level fixtures for parsing,
//! retention, and the query surface live alongside their modules under
//! `src/`.

use std::path::Path;

use enforcer_core::error::{Error, Result};
use enforcer_domain::config_types::HarnessConfig;
use enforcer_domain::harness_types::{
    HarnessCapturedOutput, HarnessCommandArgument, HarnessPinned, HarnessRunId, HarnessTimestamp,
    HarnessToolName,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::telemetry_types::ProcessExitCode;
use enforcer_harness::storage::{record_run, verify_run_layout, RunInput, RUN_FILES};

fn missing(what: &str) -> Error {
    Error::InvalidConfig(format!("test fixture: expected {what}"))
}

fn record(repo_root: &Path, run_id: &str, config: &HarnessConfig) -> Result<()> {
    let repo_root = RepoRoot::try_from(repo_root)?;
    record_run(
        &RunInput {
            repo_root: &repo_root,
            run_id: HarnessRunId::try_new(run_id.to_owned())?,
            tool: HarnessToolName::try_new("cargo".to_owned())?,
            language: None,
            command: vec![
                HarnessCommandArgument::try_new("cargo".to_owned())?,
                HarnessCommandArgument::try_new("test".to_owned())?,
            ],
            stdout: HarnessCapturedOutput::default(),
            stderr: HarnessCapturedOutput::default(),
            exit_code: ProcessExitCode::new(0),
            crate_name: Some("enforcer-harness".parse()?),
            package_name: None,
            domain: None,
            tags: vec![],
            pinned: HarnessPinned::Unpinned,
            started_at: HarnessTimestamp::try_new("2026-01-01T00:00:00Z".to_owned())?,
            ended_at: HarnessTimestamp::try_new("2026-01-01T00:00:01Z".to_owned())?,
        },
        config,
    )?;
    Ok(())
}

#[test]
fn recorded_run_has_all_five_required_files() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let config = HarnessConfig::default();
    record(dir.path(), "run-layout", &config)?;

    let run_dir = enforcer_harness::config::storage_root(&config, dir.path())?
        .join("runs")
        .join("run-layout");
    let missing_files = verify_run_layout(&run_dir);
    assert!(
        missing_files.is_empty(),
        "expected all five files present, missing: {missing_files:?}"
    );
    for rel in RUN_FILES {
        assert!(run_dir.join(rel).exists(), "missing required file: {rel}");
    }
    Ok(())
}

#[test]
fn a_run_missing_any_of_the_five_files_is_flagged_by_verify_run_layout() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let config = HarnessConfig::default();
    record(dir.path(), "run-broken", &config)?;
    let run_dir = enforcer_harness::config::storage_root(&config, dir.path())?
        .join("runs")
        .join("run-broken");
    std::fs::remove_file(run_dir.join("events.ndjson"))?;

    let missing_files = verify_run_layout(&run_dir);
    assert_eq!(missing_files.len(), 1);
    assert_eq!(missing_files[0], "events.ndjson");
    Ok(())
}

#[test]
fn manifest_replaces_not_duplicates_on_rerecord_and_two_runs_yield_two_entries() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let config = HarnessConfig::default();
    record(dir.path(), "run-a", &config)?;
    record(dir.path(), "run-b", &config)?;

    let manifest_path = enforcer_harness::config::storage_root(&config, dir.path())?
        .join("db")
        .join("ingest-manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let runs = manifest["runs"]
        .as_array()
        .ok_or_else(|| missing("runs array"))?;
    assert_eq!(
        runs.len(),
        2,
        "two distinct runIds must yield two manifest entries"
    );

    // Re-record run-a (same runId): must replace, not duplicate.
    record(dir.path(), "run-a", &config)?;
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let runs = manifest["runs"]
        .as_array()
        .ok_or_else(|| missing("runs array"))?;
    let run_a_count = runs.iter().filter(|e| e["runId"] == "run-a").count();
    assert_eq!(
        run_a_count, 1,
        "re-recording the same runId must replace its manifest entry, not duplicate it"
    );
    assert_eq!(
        runs.len(),
        2,
        "total entries must stay at two after replace"
    );
    Ok(())
}

#[test]
fn legacy_only_run_is_surfaced_by_list_runs_and_dedupes_when_present_in_both_roots() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let config = HarnessConfig::default();

    // Seed a run ONLY under the legacy `.ocentra-enforcer` root by writing
    // its layout directly (simulating a pre-migration install `arc-18`
    // must keep readable).
    let legacy_root = dir.path().join(".ocentra-enforcer");
    let legacy_run_dir = legacy_root.join("runs").join("run-legacy");
    std::fs::create_dir_all(legacy_run_dir.join("raw"))?;
    std::fs::write(legacy_run_dir.join("raw").join("stdout.log"), "")?;
    std::fs::write(legacy_run_dir.join("raw").join("stderr.log"), "")?;
    std::fs::write(legacy_run_dir.join("diagnostics.ndjson"), "")?;
    std::fs::write(legacy_run_dir.join("events.ndjson"), "")?;
    let legacy_summary = serde_json::json!({
        "runId": "run-legacy",
        "tool": "cargo",
        "status": "passed",
        "startedAt": "2020-01-01T00:00:00Z",
        "endedAt": "2020-01-01T00:00:01Z",
        "artifacts": {
            "stdout": ".ocentra-enforcer/runs/run-legacy/raw/stdout.log",
            "stderr": ".ocentra-enforcer/runs/run-legacy/raw/stderr.log",
            "diagnostics": ".ocentra-enforcer/runs/run-legacy/diagnostics.ndjson",
            "events": ".ocentra-enforcer/runs/run-legacy/events.ndjson"
        },
        "storage": { "root": ".ocentra-enforcer" }
    });
    std::fs::write(
        legacy_run_dir.join("summary.json"),
        serde_json::to_string_pretty(&legacy_summary)?,
    )?;

    // A run present under BOTH roots (write via the authoritative API,
    // then mirror an identical summary under legacy) must appear once.
    record(dir.path(), "run-both", &config)?;
    let authoritative_run_dir = enforcer_harness::config::storage_root(&config, dir.path())?
        .join("runs")
        .join("run-both");
    let both_summary_src = authoritative_run_dir.join("summary.json");
    let both_legacy_dir = legacy_root.join("runs").join("run-both");
    std::fs::create_dir_all(&both_legacy_dir)?;
    std::fs::copy(&both_summary_src, both_legacy_dir.join("summary.json"))?;

    let runs = enforcer_harness::query::list_runs(
        dir.path(),
        &config,
        &enforcer_harness::query::RunQuery::default(),
    )?;
    let run_ids: Vec<&str> = runs.iter().filter_map(|r| r["runId"].as_str()).collect();

    assert!(
        run_ids.contains(&"run-legacy"),
        "legacy-only run must be surfaced: {run_ids:?}"
    );
    let both_count = run_ids.iter().filter(|id| **id == "run-both").count();
    assert_eq!(
        both_count, 1,
        "run present in both roots must appear once (dedupe by runId)"
    );
    Ok(())
}

#[test]
fn dropping_the_legacy_root_would_lose_run_history_fail_fixture() -> Result<()> {
    // This fixture documents the contract: candidate_storage_roots MUST
    // include the legacy root. If a future change dropped it, this
    // assertion fails, catching the regression the workpack calls out.
    let dir = tempfile::TempDir::new()?;
    let config = HarnessConfig::default();
    let roots = enforcer_harness::legacy::candidate_storage_roots(dir.path(), &config)?;
    assert!(
        roots.iter().any(|r| r.ends_with(".ocentra-enforcer")),
        "legacy `.ocentra-enforcer` root must remain in the candidate list — dropping it loses run history"
    );
    Ok(())
}
