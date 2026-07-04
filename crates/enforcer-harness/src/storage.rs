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
use serde_json::{json, Value};

use crate::config::HarnessConfig;
use crate::duckdb_seam::write_duckdb_status;
use crate::legacy::normalize_rel;
use crate::parsers::{dedupe_diagnostics, parse_diagnostics, sort_diagnostics, HarnessDiagnostic};

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
    pub repo_root: &'a Path,
    pub run_id: String,
    pub tool: String,
    pub language: Option<String>,
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub crate_name: Option<String>,
    pub package_name: Option<String>,
    pub domain: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub started_at: String,
    pub ended_at: String,
}

/// Result of recording one run: whether it passed, plus the written
/// summary and parsed diagnostics.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub ok: bool,
    pub summary: Value,
    pub diagnostics: Vec<HarnessDiagnostic>,
}

/// Persist one run to `.enforce/runs/<runId>/`, apply redaction, write the
/// five required files, stamp the manifest + duckdb-status, run the prune
/// engine, and return the summary + diagnostics.
pub fn record_run(input: &RunInput<'_>, config: &HarnessConfig) -> Result<RunOutcome> {
    let storage_root = config.storage_root(input.repo_root)?;
    let run_dir = storage_root.join("runs").join(&input.run_id);
    std::fs::create_dir_all(run_dir.join("raw"))?;
    std::fs::create_dir_all(storage_root.join("db"))?;

    let stdout = crate::config::redact_text(&input.stdout)?;
    let stderr = crate::config::redact_text(&input.stderr)?;
    std::fs::write(run_dir.join("raw").join("stdout.log"), &stdout)?;
    std::fs::write(run_dir.join("raw").join("stderr.log"), &stderr)?;

    let language = input
        .language
        .clone()
        .unwrap_or_else(|| crate::parsers::infer_language(&input.tool));

    let mut diagnostics = parse_diagnostics(&input.run_id, &input.tool, &stdout, &stderr);
    if input.exit_code != 0 {
        let tail = if !stderr.trim().is_empty() {
            &stderr
        } else {
            &stdout
        };
        let excerpt: String = tail.trim().lines().take(8).collect::<Vec<_>>().join("\n");
        diagnostics.push(HarnessDiagnostic {
            run_id: input.run_id.clone(),
            tool: input.tool.clone(),
            language: language.clone(),
            severity: "error".to_owned(),
            rule_id: "HAR-1.1".to_owned(),
            file: ".".to_owned(),
            line: 1,
            message: format!("Command failed with exit code {}.", input.exit_code),
            source: if excerpt.is_empty() {
                None
            } else {
                Some(excerpt)
            },
            fingerprint: None,
        });
    }
    let diagnostics = sort_diagnostics(dedupe_diagnostics(diagnostics));

    let status = if input.exit_code == 0 {
        "passed"
    } else {
        "failed"
    };
    let mut by_severity = std::collections::BTreeMap::new();
    for d in &diagnostics {
        *by_severity.entry(d.severity.clone()).or_insert(0i64) += 1;
    }

    let duckdb_status = write_duckdb_status(input.repo_root, &storage_root)?;

    let storage_root_rel = normalize_rel(input.repo_root, &storage_root);
    let summary = json!({
        "runId": input.run_id,
        "root": input.repo_root.to_string_lossy(),
        "tool": input.tool,
        "language": language,
        "crateName": input.crate_name,
        "packageName": input.package_name,
        "domain": input.domain,
        "tags": input.tags,
        "command": input.command,
        "pinned": input.pinned,
        "status": status,
        "exitCode": input.exit_code,
        "startedAt": input.started_at,
        "endedAt": input.ended_at,
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
        "duckdb": serde_json::to_value(&duckdb_status)?,
    });

    write_ndjson(&run_dir.join("diagnostics.ndjson"), &diagnostics)?;
    let events = vec![
        json!({ "type": "run-started", "runId": input.run_id, "timestamp": input.started_at, "tool": input.tool, "command": input.command }),
        json!({ "type": "run-finished", "runId": input.run_id, "timestamp": input.ended_at, "status": status, "exitCode": input.exit_code, "diagnosticCount": diagnostics.len() }),
    ];
    write_ndjson(&run_dir.join("events.ndjson"), &events)?;
    let summary_path = run_dir.join("summary.json");
    std::fs::write(
        &summary_path,
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )?;

    let prune = crate::retention::prune_runs(input.repo_root, config)?;
    let mut summary = summary;
    summary["pruned"] = json!(prune.removed);
    std::fs::write(
        &summary_path,
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )?;

    write_manifest(&storage_root, &input.run_id, &summary)?;

    Ok(RunOutcome {
        ok: input.exit_code == 0,
        summary,
        diagnostics,
    })
}

pub(crate) fn retention_summary_json(config: &HarnessConfig) -> Value {
    json!({
        "maxRuns": config.max_runs,
        "maxRunsPerTool": config.max_runs_per_tool,
        "maxFailedRuns": config.max_failed_runs,
        "pruneAfterDays": config.prune_after_days,
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
    let runs = current["runs"].as_array_mut().ok_or_else(|| {
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
pub fn reset_runs(repo_root: &Path, config: &HarnessConfig) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    for root in crate::legacy::candidate_storage_roots(repo_root, config)? {
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
            removed.push(normalize_rel(repo_root, &root));
        }
    }
    Ok(removed)
}

/// Verify a run directory has exactly the five required files (used by the
/// fail fixture: a run missing any of the five is rejected/repaired).
pub fn verify_run_layout(run_dir: &Path) -> Vec<String> {
    RUN_FILES
        .iter()
        .filter(|rel| !run_dir.join(rel).exists())
        .map(|rel| (*rel).to_owned())
        .collect()
}
