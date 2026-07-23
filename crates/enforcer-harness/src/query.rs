//! Half B — query surface (backs 6 MCP tools + `runs` CLI) [G1, WAVE 4].
//!
//! `list_runs`/`run_summary`/`run_diagnostics`/`last_failure`/
//! `read_artifact` with query filters (runId/status/tool/limit), reading
//! across BOTH the authoritative and legacy storage roots (dedupe by
//! runId — see [`crate::legacy`]). Ported from the query half of
//! `src/harness.mjs`.

use std::path::Path;

use enforcer_core::error::Result;
use serde_json::Value;

use crate::legacy::{candidate_storage_roots, normalize_rel};
use crate::storage::read_ndjson;
use enforcer_domain::config_types::{
    CrateName, HarnessArtifactByteLimit, HarnessConfig, HarnessRunLimit,
};
use enforcer_domain::harness_types::{
    HarnessArtifactKind, HarnessDomainName, HarnessPackageName, HarnessRunId, HarnessRunStatus,
    HarnessTag, HarnessToolName,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

/// Optional filters shared by every query entry point.
#[derive(Debug, Clone, Default)]
pub struct RunQuery {
    pub run_id: Option<HarnessRunId>,
    pub status: Option<HarnessRunStatus>,
    pub tool: Option<HarnessToolName>,
    pub crate_name: Option<CrateName>,
    pub package_name: Option<HarnessPackageName>,
    pub domain: Option<HarnessDomainName>,
    pub tag: Option<HarnessTag>,
    pub limit: Option<HarnessRunLimit>,
}

/// Read one run's `summary.json` (with a `storage.root` default backfilled)
/// from a specific storage root, or `None` if absent.
fn read_summary_from(
    repo_root: &Path,
    storage_root: &Path,
    run_id: &str,
    config: &HarnessConfig,
) -> Result<Option<Value>> {
    let summary_path = storage_root.join("runs").join(run_id).join("summary.json");
    if !summary_path.exists() {
        return Ok(None);
    }
    let mut summary: Value = serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
    if summary.get("storage").is_none() {
        let storage = serde_json::json!({
            "root": normalize_rel(repo_root, storage_root),
            "retention": crate::storage::retention_summary_json(config),
        });
        let object = summary.as_object_mut().ok_or_else(|| {
            enforcer_core::error::Error::InvalidConfig(
                "summary.json root must be an object".to_owned(),
            )
        })?;
        object.insert("storage".to_owned(), storage);
    }
    Ok(Some(summary))
}

/// Every run across the authoritative + legacy roots, deduped by `runId`
/// (authoritative wins on duplicate). Mirrors `allRuns` in
/// `src/harness.mjs`.
pub fn all_runs(repo_root: &Path, config: &HarnessConfig) -> Result<Vec<Value>> {
    let mut runs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for storage_root in candidate_storage_roots(repo_root, config)? {
        let runs_dir = storage_root.join("runs");
        if !runs_dir.exists() {
            continue;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&runs_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let run_id = entry.file_name().to_string_lossy().into_owned();
            if seen.contains(&run_id) {
                continue;
            }
            if let Some(summary) = read_summary_from(repo_root, &storage_root, &run_id, config)? {
                seen.insert(run_id);
                runs.push(summary);
            }
        }
    }
    Ok(runs)
}

/// Read one run's summary by id, searching authoritative then legacy roots.
pub fn read_summary(
    repo_root: &Path,
    run_id: &str,
    config: &HarnessConfig,
) -> Result<Option<Value>> {
    for storage_root in candidate_storage_roots(repo_root, config)? {
        if let Some(summary) = read_summary_from(repo_root, &storage_root, run_id, config)? {
            return Ok(Some(summary));
        }
    }
    Ok(None)
}

fn matches(run: &Value, query: &RunQuery) -> bool {
    if query
        .run_id
        .as_ref()
        .is_some_and(|want| run.get("runId").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if query
        .status
        .is_some_and(|want| run.get("status").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if query
        .tool
        .as_ref()
        .is_some_and(|want| run.get("tool").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if query
        .crate_name
        .as_ref()
        .is_some_and(|want| run.get("crateName").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if query
        .package_name
        .as_ref()
        .is_some_and(|want| run.get("packageName").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if query
        .domain
        .as_ref()
        .is_some_and(|want| run.get("domain").and_then(Value::as_str) != Some(want.as_str()))
    {
        return false;
    }
    if let Some(tag) = &query.tag {
        let has_tag = run
            .get("tags")
            .and_then(Value::as_array)
            .is_some_and(|tags| tags.iter().any(|t| t.as_str() == Some(tag.as_str())));
        if !has_tag {
            return false;
        }
    }
    true
}

/// List runs newest-first (by `startedAt`), filtered + limited.
pub fn list_runs(repo_root: &Path, config: &HarnessConfig, query: &RunQuery) -> Result<Vec<Value>> {
    let mut runs: Vec<Value> = all_runs(repo_root, config)?
        .into_iter()
        .filter(|run| matches(run, query))
        .collect();
    runs.sort_by(|a, b| {
        let sa = a
            .get("startedAt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let sb = b
            .get("startedAt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        sb.cmp(sa)
    });
    let limit = query.limit.map_or(20, HarnessRunLimit::get);
    runs.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    Ok(runs)
}

/// The single run matching a query (`runId` if given, else the most recent
/// run).
pub fn run_summary(
    repo_root: &Path,
    config: &HarnessConfig,
    query: &RunQuery,
) -> Result<Option<Value>> {
    if let Some(run_id) = &query.run_id {
        return read_summary(repo_root, run_id.as_str(), config);
    }
    let mut one_query = query.clone();
    one_query.limit = Some(HarnessRunLimit::from_value(1));
    Ok(list_runs(repo_root, config, &one_query)?.into_iter().next())
}

/// Filters applied to a single run's diagnostics list (as opposed to
/// [`RunQuery`], which selects WHICH run).
#[derive(Debug, Clone, Default)]
pub struct DiagnosticsFilter {
    pub severity: Option<Severity>,
    pub file: Option<RelPath>,
    pub limit: Option<HarnessRunLimit>,
}

/// A run's parsed `diagnostics.ndjson`, filtered by `severity`/`file`/
/// `limit`.
pub fn run_diagnostics(
    repo_root: &Path,
    config: &HarnessConfig,
    query: &RunQuery,
    filter: &DiagnosticsFilter,
) -> Result<(bool, Option<String>, Vec<Value>)> {
    let Some(run) = run_summary(repo_root, config, query)? else {
        return Ok((false, None, Vec::new()));
    };
    let run_id = run
        .get("runId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let diagnostics_path = run
        .get("artifacts")
        .and_then(|a| a.get("diagnostics"))
        .and_then(Value::as_str)
        .map(|rel| repo_root.join(rel))
        .unwrap_or_else(|| {
            let storage_root = run
                .get("storage")
                .and_then(|s| s.get("root"))
                .and_then(Value::as_str)
                .unwrap_or(".enforce");
            repo_root
                .join(storage_root)
                .join("runs")
                .join(&run_id)
                .join("diagnostics.ndjson")
        });
    let all = read_ndjson(&diagnostics_path)?;
    let filtered: Vec<Value> = all
        .into_iter()
        .filter(|d| {
            filter
                .severity
                .is_none_or(|s| d.get("severity").and_then(Value::as_str) == Some(s.wire_name()))
        })
        .filter(|d| {
            filter
                .file
                .as_ref()
                .is_none_or(|f| d.get("file").and_then(Value::as_str) == Some(f.as_str()))
        })
        .take(usize::try_from(filter.limit.map_or(50, HarnessRunLimit::get)).unwrap_or(usize::MAX))
        .collect();
    Ok((true, Some(run_id), filtered))
}

/// The most recent `status == "failed"` run + its top-N diagnostics.
pub fn last_failure(
    repo_root: &Path,
    config: &HarnessConfig,
    query: &RunQuery,
    diagnostic_limit: Option<HarnessRunLimit>,
) -> Result<(bool, Option<Value>, Vec<Value>)> {
    let mut search = query.clone();
    search.limit = Some(query.limit.unwrap_or(HarnessRunLimit::from_value(50)));
    let runs = list_runs(repo_root, config, &search)?;
    let Some(failed) = runs
        .into_iter()
        .find(|r| r.get("status").and_then(Value::as_str) == Some("failed"))
    else {
        return Ok((false, None, Vec::new()));
    };
    let run_id = failed
        .get("runId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut diag_query = RunQuery {
        run_id: Some(HarnessRunId::from_adapter(&run_id)),
        ..RunQuery::default()
    };
    diag_query.limit = Some(HarnessRunLimit::from_value(1));
    let filter = DiagnosticsFilter {
        limit: diagnostic_limit.or(Some(HarnessRunLimit::from_value(10))),
        ..DiagnosticsFilter::default()
    };
    let (_, _, diagnostics) = run_diagnostics(repo_root, config, &diag_query, &filter)?;
    Ok((true, Some(failed), diagnostics))
}

/// Read + redact + byte-cap an artifact (`stdout`/`stderr`/`diagnostics`/
/// `events`) for a matched run. Rejects paths that escape `repo_root`.
pub fn read_artifact(
    repo_root: &Path,
    config: &HarnessConfig,
    query: &RunQuery,
    artifact: HarnessArtifactKind,
    limit_bytes: Option<HarnessArtifactByteLimit>,
) -> Result<(bool, Option<String>, String, Option<String>)> {
    let Some(run) = run_summary(repo_root, config, query)? else {
        return Ok((
            false,
            None,
            String::new(),
            Some("No harness run found.".to_owned()),
        ));
    };
    let run_id = run
        .get("runId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let Some(artifact_rel) = run
        .get("artifacts")
        .and_then(|a| a.get(artifact.as_str()))
        .and_then(Value::as_str)
    else {
        return Ok((
            false,
            Some(run_id),
            String::new(),
            Some(format!("Unknown artifact: {}", artifact.as_str())),
        ));
    };
    let absolute = repo_root.join(artifact_rel);
    if !is_inside_root(repo_root, &absolute) {
        return Ok((
            false,
            Some(run_id),
            String::new(),
            Some(format!(
                "Artifact path escapes harness root: {artifact_rel}"
            )),
        ));
    }
    let text = if absolute.exists() {
        std::fs::read_to_string(&absolute)?
    } else {
        String::new()
    };
    let redacted = crate::config::redact_text(&text)?;
    let configured_cap = usize::try_from(config.max_artifact_bytes.get()).unwrap_or(usize::MAX);
    let cap = limit_bytes
        .map(HarnessArtifactByteLimit::get)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(configured_cap);
    let capped: String = redacted.chars().take(cap).collect();
    Ok((true, Some(run_id), capped, None))
}

fn is_inside_root(root: &Path, candidate: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let candidate_parent = candidate.parent().unwrap_or(candidate);
    let candidate_resolved = candidate_parent
        .canonicalize()
        .map(|p| p.join(candidate.file_name().unwrap_or_default()))
        .unwrap_or_else(|_| candidate.to_path_buf());
    candidate_resolved.starts_with(&root)
}

#[cfg(test)]
mod tests {
    use super::{last_failure, list_runs, read_artifact, RunQuery};
    use crate::storage::{record_run, RunInput};
    use enforcer_core::error::{Error, Result};
    use enforcer_domain::config_types::HarnessConfig;
    use enforcer_domain::harness_types::{
        HarnessCapturedOutput, HarnessCommandArgument, HarnessPinned, HarnessRunId,
        HarnessTimestamp, HarnessToolName,
    };
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::telemetry_types::ProcessExitCode;
    use serde_json::Value;
    use std::path::Path;

    fn missing(what: &str) -> Error {
        Error::InvalidConfig(format!("test fixture: expected {what}"))
    }

    fn record_sample(
        repo_root: &Path,
        run_id: &str,
        tool: &str,
        exit_code: i32,
        started_at: &str,
    ) -> Result<()> {
        let config = HarnessConfig::default();
        let repo_root = RepoRoot::try_from(repo_root)?;
        record_run(
            &RunInput {
                repo_root: &repo_root,
                run_id: HarnessRunId::try_new(run_id.to_owned())?,
                tool: HarnessToolName::try_new(tool.to_owned())?,
                language: None,
                command: vec![HarnessCommandArgument::try_new(tool.to_owned())?],
                stdout: HarnessCapturedOutput::default(),
                stderr: HarnessCapturedOutput::default(),
                exit_code: ProcessExitCode::new(exit_code),
                crate_name: None,
                package_name: None,
                domain: None,
                tags: vec![],
                pinned: HarnessPinned::Unpinned,
                started_at: HarnessTimestamp::try_new(started_at.to_owned())?,
                ended_at: HarnessTimestamp::try_new(started_at.to_owned())?,
            },
            &config,
        )?;
        Ok(())
    }

    #[test]
    fn list_runs_returns_newest_first() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        record_sample(dir.path(), "run-a", "cargo", 0, "2026-01-01T00:00:00Z")?;
        record_sample(dir.path(), "run-b", "cargo", 0, "2026-01-02T00:00:00Z")?;
        let runs = list_runs(dir.path(), &HarnessConfig::default(), &RunQuery::default())?;
        assert_eq!(runs[0]["runId"], "run-b");
        assert_eq!(runs[1]["runId"], "run-a");
        Ok(())
    }

    #[test]
    fn last_failure_returns_most_recent_failed_run_with_diagnostics() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        record_sample(dir.path(), "run-ok", "cargo", 0, "2026-01-01T00:00:00Z")?;
        record_sample(dir.path(), "run-bad", "cargo", 1, "2026-01-02T00:00:00Z")?;
        let (found, run, diagnostics) = last_failure(
            dir.path(),
            &HarnessConfig::default(),
            &RunQuery::default(),
            None,
        )?;
        assert!(found);
        assert_eq!(
            run.ok_or_else(|| missing("a failed run"))?["runId"],
            "run-bad"
        );
        assert_eq!(diagnostics.len(), 1);
        Ok(())
    }

    #[test]
    fn read_artifact_redacts_and_caps() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig::default();
        let repo_root = RepoRoot::try_from(dir.path())?;
        record_run(
            &RunInput {
                repo_root: &repo_root,
                run_id: HarnessRunId::try_new("run-secret".to_owned())?,
                tool: HarnessToolName::try_new("cargo".to_owned())?,
                language: None,
                command: vec![HarnessCommandArgument::try_new("cargo".to_owned())?],
                stdout: HarnessCapturedOutput::from_owned(format!(
                    "token {}{} leaked",
                    "AKIAIOSF", "ODNN7EXAMPLE"
                )),
                stderr: HarnessCapturedOutput::default(),
                exit_code: ProcessExitCode::new(0),
                crate_name: None,
                package_name: None,
                domain: None,
                tags: vec![],
                pinned: HarnessPinned::Unpinned,
                started_at: HarnessTimestamp::try_new("2026-01-01T00:00:00Z".to_owned())?,
                ended_at: HarnessTimestamp::try_new("2026-01-01T00:00:01Z".to_owned())?,
            },
            &config,
        )?;
        let query = RunQuery {
            run_id: Some(HarnessRunId::try_new("run-secret".to_owned())?),
            ..RunQuery::default()
        };
        let (ok, _run_id, text, _err) = read_artifact(
            dir.path(),
            &config,
            &query,
            enforcer_domain::harness_types::HarnessArtifactKind::Stdout,
            None,
        )?;
        assert!(ok);
        assert_eq!(text, "token [REDACTED] leaked");
        Ok(())
    }

    #[test]
    fn read_artifact_rejects_path_escape() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig::default();
        record_sample(dir.path(), "run-x", "cargo", 0, "2026-01-01T00:00:00Z")?;
        // Simulate a tampered summary pointing outside the repo root.
        let storage_root = crate::config::storage_root(&config, dir.path())?;
        let summary_path = storage_root.join("runs").join("run-x").join("summary.json");
        let mut summary: Value = serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
        summary["artifacts"]["stdout"] = Value::String("../../outside.log".to_owned());
        std::fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)?;
        let query = RunQuery {
            run_id: Some(HarnessRunId::try_new("run-x".to_owned())?),
            ..RunQuery::default()
        };
        let (ok, _run_id, _text, err) = read_artifact(
            dir.path(),
            &config,
            &query,
            enforcer_domain::harness_types::HarnessArtifactKind::Stdout,
            None,
        )?;
        assert!(!ok);
        assert!(err
            .ok_or_else(|| missing("an escape error message"))?
            .contains("escapes"));
        Ok(())
    }
}
