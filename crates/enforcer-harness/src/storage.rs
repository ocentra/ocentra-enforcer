//! Half B — run storage layout [G1].
//!
//! Persists each run under `.enforce/runs/<runId>/` with exactly
//! `raw/stdout.log`, `raw/stderr.log`, `diagnostics.ndjson`,
//! `events.ndjson`, `summary.json`, and maintains
//! `.enforce/db/ingest-manifest.json` (append-or-replace by `runId`).
//! Applies `enforcer_core`/`config::redact_text` redaction to stdout/
//! stderr bytes before write. Ported from the storage half of
//! `src/harness.mjs` (`runHarness`, `writeManifest`, `writeDuckDbStatus`).

use std::path::Path;

use enforcer_core::error::Result;
use enforcer_domain::config_types::CrateName;
use serde_json::{json, Value};

use crate::duckdb_seam::write_duckdb_status;
use crate::legacy::normalize_rel;
use crate::parsers::{
    dedupe_diagnostics, parse_diagnostics, sort_diagnostics, HarnessDiagnostic,
    HarnessDiagnosticDto,
};
use enforcer_domain::config_types::HarnessConfig;
use enforcer_domain::harness_types::{
    HarnessCapturedOutput, HarnessCommandArgument, HarnessDiagnosticMessage, HarnessDiagnosticPath,
    HarnessDomainName, HarnessExternalRuleId, HarnessLanguage, HarnessPinned, HarnessRunFile,
    HarnessRunId, HarnessRunStatus, HarnessSourceLine, HarnessTag, HarnessTimestamp,
    HarnessToolName,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::telemetry_types::ProcessExitCode;

/// Exactly the five files a completed run must have on disk.
pub const RUN_FILES: &[&str] = &[
    "raw/stdout.log",
    "raw/stderr.log",
    "diagnostics.ndjson",
    "events.ndjson",
    "summary.json",
];

/// Input to [`record_run`] — the outcome of having already shelled out to
/// a native tool (this crate does not own process-spawning policy; callers
/// supply the captured stdout/stderr/exit code).
#[derive(Debug, Clone)]
pub struct RunInput<'a> {
    pub repo_root: &'a RepoRoot,
    pub run_id: HarnessRunId,
    pub tool: HarnessToolName,
    pub language: Option<HarnessLanguage>,
    pub command: Vec<HarnessCommandArgument>,
    pub stdout: HarnessCapturedOutput,
    pub stderr: HarnessCapturedOutput,
    pub exit_code: ProcessExitCode,
    pub crate_name: Option<CrateName>,
    pub package_name: Option<CrateName>,
    pub domain: Option<HarnessDomainName>,
    pub tags: Vec<HarnessTag>,
    pub pinned: HarnessPinned,
    pub started_at: HarnessTimestamp,
    pub ended_at: HarnessTimestamp,
}

/// Result of recording one run: whether it passed, plus the written
/// summary and parsed diagnostics.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub status: HarnessRunStatus,
    pub summary: Value,
    pub diagnostics: Vec<HarnessDiagnostic>,
}

/// Persist one run to `.enforce/runs/<runId>/`, apply redaction, write the
/// five required files, stamp the manifest + duckdb-status, run the prune
/// engine, and return the summary + diagnostics.
pub fn record_run(input: &RunInput<'_>, config: &HarnessConfig) -> Result<RunOutcome> {
    let repo_root = Path::new(input.repo_root.as_str());
    let storage_root = crate::config::storage_root(config, repo_root)?;
    let run_dir = storage_root.join("runs").join(input.run_id.as_str());
    std::fs::create_dir_all(run_dir.join("raw"))?;
    std::fs::create_dir_all(storage_root.join("db"))?;

    let stdout = crate::config::redact_text(input.stdout.as_str())?;
    let stderr = crate::config::redact_text(input.stderr.as_str())?;
    std::fs::write(run_dir.join("raw").join("stdout.log"), &stdout)?;
    std::fs::write(run_dir.join("raw").join("stderr.log"), &stderr)?;

    let language = input
        .language
        .unwrap_or_else(|| crate::parsers::infer_language(input.tool.as_str()));

    let mut diagnostics =
        parse_diagnostics(input.run_id.as_str(), input.tool.as_str(), &stdout, &stderr);
    if input.exit_code.get() != 0 {
        let tail = if !stderr.trim().is_empty() {
            &stderr
        } else {
            &stdout
        };
        let excerpt: String = tail.trim().lines().take(8).collect::<Vec<_>>().join("\n");
        let mut failure = HarnessDiagnostic {
            run_id: input.run_id.clone(),
            tool: input.tool.clone(),
            language,
            severity: enforcer_domain::severity::Severity::Error,
            rule_id: HarnessExternalRuleId::from_adapter("HAR-1.1"),
            file: HarnessDiagnosticPath::from_adapter("."),
            line: HarnessSourceLine::from_external(1),
            message: HarnessDiagnosticMessage::from_adapter(&format!(
                "Command failed with exit code {}.",
                input.exit_code.get()
            )),
            source: None,
            fingerprint: None,
        };
        failure.source = (!excerpt.is_empty())
            .then(|| enforcer_domain::harness_types::HarnessCapturedOutput::from_owned(excerpt));
        diagnostics.push(failure);
    }
    let diagnostics = sort_diagnostics(dedupe_diagnostics(diagnostics));

    let status = HarnessRunStatus::from_exit_code(input.exit_code);
    let mut by_severity = std::collections::BTreeMap::new();
    for d in &diagnostics {
        *by_severity
            .entry(format!("{:?}", d.severity).to_ascii_lowercase())
            .or_insert(0i64) += 1;
    }

    let duckdb_status = write_duckdb_status(repo_root, &storage_root)?;

    let storage_root_rel = normalize_rel(repo_root, &storage_root);
    let summary = json!({
        "runId": input.run_id.as_str(),
        "root": input.repo_root.as_str(),
        "tool": input.tool.as_str(),
        "language": language.as_str(),
        "crateName": input.crate_name.as_ref().map(CrateName::as_str),
        "packageName": input.package_name.as_ref().map(CrateName::as_str),
        "domain": input.domain.as_ref().map(HarnessDomainName::as_str),
        "tags": input.tags.iter().map(HarnessTag::as_str).collect::<Vec<_>>(),
        "command": input.command.iter().map(HarnessCommandArgument::as_str).collect::<Vec<_>>(),
        "pinned": input.pinned.as_bool(),
        "status": status.as_str(),
        "exitCode": input.exit_code.get(),
        "startedAt": input.started_at.as_str(),
        "endedAt": input.ended_at.as_str(),
        "diagnosticCount": diagnostics.len(),
        "bySeverity": by_severity,
        "artifacts": {
            "stdout": format!("{storage_root_rel}/runs/{}/raw/stdout.log", input.run_id),
            "stderr": format!("{storage_root_rel}/runs/{}/raw/stderr.log", input.run_id),
            "diagnostics": format!("{storage_root_rel}/runs/{}/diagnostics.ndjson", input.run_id),
            "events": format!("{storage_root_rel}/runs/{}/events.ndjson", input.run_id),
        },
        "storage": {
            "root": storage_root_rel,
            "retention": retention_summary_json(config),
        },
        "duckdb": crate::duckdb_seam::status_wire_value(&duckdb_status)?,
    });

    let diagnostic_rows: Vec<_> = diagnostics.iter().map(HarnessDiagnosticDto::from).collect();
    write_ndjson(&run_dir.join("diagnostics.ndjson"), &diagnostic_rows)?;
    let events = vec![
        json!({ "type": "run-started", "runId": input.run_id.as_str(), "timestamp": input.started_at.as_str(), "tool": input.tool.as_str(), "command": input.command.iter().map(HarnessCommandArgument::as_str).collect::<Vec<_>>() }),
        json!({ "type": "run-finished", "runId": input.run_id.as_str(), "timestamp": input.ended_at.as_str(), "status": status.as_str(), "exitCode": input.exit_code.get(), "diagnosticCount": diagnostics.len() }),
    ];
    write_ndjson(&run_dir.join("events.ndjson"), &events)?;
    let summary_path = run_dir.join("summary.json");
    std::fs::write(
        &summary_path,
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )?;

    let prune = crate::retention::prune_runs(repo_root, config)?;
    let mut summary = summary;
    let summary_object = summary.as_object_mut().ok_or_else(|| {
        enforcer_core::error::Error::InvalidConfig("summary root must be an object".to_owned())
    })?;
    summary_object.insert("pruned".to_owned(), json!(prune.removed));
    std::fs::write(
        &summary_path,
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )?;

    write_manifest(&storage_root, input.run_id.as_str(), &summary)?;

    Ok(RunOutcome {
        status,
        summary,
        diagnostics,
    })
}

pub(crate) fn retention_summary_json(config: &HarnessConfig) -> Value {
    json!({
        "maxRuns": config.max_runs.map(enforcer_domain::config_types::HarnessRunLimit::get),
        "maxRunsPerTool": config.max_runs_per_tool.map(enforcer_domain::config_types::HarnessRunLimit::get),
        "maxFailedRuns": config.max_failed_runs.map(enforcer_domain::config_types::HarnessRunLimit::get),
        "pruneAfterDays": config.prune_after_days.map(enforcer_domain::config_types::HarnessRetentionDays::get),
    })
}

pub(crate) fn write_ndjson<T: serde::Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row)?);
        body.push('\n');
    }
    std::fs::write(path, body)?;
    Ok(())
}

pub(crate) fn read_ndjson(path: &Path) -> Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(line)?);
    }
    Ok(out)
}

/// Append-or-replace this `runId`'s entry in
/// `<storageRoot>/db/ingest-manifest.json`.
pub(crate) fn write_manifest(storage_root: &Path, run_id: &str, summary: &Value) -> Result<()> {
    let manifest_path = storage_root.join("db").join("ingest-manifest.json");
    let mut current: Value = if manifest_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?
    } else {
        json!({ "runs": [] })
    };
    let runs = current
        .get_mut("runs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            enforcer_core::error::Error::InvalidConfig(
                "ingest-manifest.json: `runs` is not an array".to_owned(),
            )
        })?;
    runs.retain(|entry| entry.get("runId").and_then(Value::as_str) != Some(run_id));
    runs.push(json!({
        "runId": run_id,
        "summaryPath": format!("{}/runs/{run_id}/summary.json", normalize_rel_ancestor(storage_root)),
        "ingestedAt": now_iso(),
        "tool": summary.get("tool"),
        "status": summary.get("status"),
        "crateName": summary.get("crateName"),
        "packageName": summary.get("packageName"),
        "domain": summary.get("domain"),
        "tags": summary.get("tags"),
        "duckdb": summary.get("duckdb"),
    }));
    std::fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&current)?),
    )?;
    Ok(())
}

/// Rebuild the manifest from scratch by scanning every run under
/// `storage_root` — used after a prune pass removes run directories so
/// stale manifest entries don't linger.
pub(crate) fn rewrite_manifest(repo_root: &Path, storage_root: &Path) -> Result<()> {
    if !storage_root.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(storage_root.join("db"))?;
    let runs_dir = storage_root.join("runs");
    let mut entries = Vec::new();
    if runs_dir.exists() {
        for entry in std::fs::read_dir(&runs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let run_id = entry.file_name().to_string_lossy().into_owned();
            let summary_path = entry.path().join("summary.json");
            if !summary_path.exists() {
                continue;
            }
            let summary: Value = serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
            entries.push(json!({
                "runId": run_id,
                "summaryPath": format!("{}/runs/{run_id}/summary.json", normalize_rel(repo_root, storage_root)),
                "ingestedAt": now_iso(),
                "tool": summary.get("tool"),
                "status": summary.get("status"),
                "crateName": summary.get("crateName"),
                "packageName": summary.get("packageName"),
                "domain": summary.get("domain"),
                "tags": summary.get("tags").cloned().unwrap_or_else(|| json!([])),
                "duckdb": summary.get("duckdb"),
            }));
        }
    }
    let manifest = json!({ "runs": entries });
    std::fs::write(
        storage_root.join("db").join("ingest-manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}

fn normalize_rel_ancestor(storage_root: &Path) -> String {
    storage_root
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".enforce".to_owned())
}

fn now_iso() -> String {
    // RFC3339-ish timestamp without pulling in a time crate: seconds-since-
    // epoch is sufficient for manifest bookkeeping (not compared/parsed as
    // a date anywhere in this crate).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{now}")
}

/// Clear the entire run store: removes every candidate storage root.
pub fn reset_runs(repo_root: &Path, config: &HarnessConfig) -> Result<Vec<HarnessDiagnosticPath>> {
    let mut removed = Vec::new();
    for root in crate::legacy::candidate_storage_roots(repo_root, config)? {
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
            removed.push(HarnessDiagnosticPath::from_adapter(&normalize_rel(
                repo_root, &root,
            )));
        }
    }
    Ok(removed)
}

/// Verify a run directory has exactly the five required files (used by the
/// fail fixture: a run missing any of the five is rejected/repaired).
pub fn verify_run_layout(run_dir: &Path) -> Vec<HarnessRunFile> {
    RUN_FILES
        .iter()
        .filter(|rel| !run_dir.join(rel).exists())
        .filter_map(|rel| HarnessRunFile::try_new((*rel).to_owned()).ok())
        .collect()
}
