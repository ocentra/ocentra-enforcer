//! Half B — retention/prune engine [G1].
//!
//! Honors `maxRuns`/`maxRunsPerTool`/`maxFailedRuns`/`pruneAfterDays` with
//! keep/pin logic (a `None` limit means unlimited). Runs on every run write
//! (via [`crate::storage::record_run`]) AND via explicit [`prune_runs`].
//! Ported from `pruneRuns` in `src/harness.mjs`.

use std::path::Path;

use enforcer_core::error::Result;
use enforcer_domain::harness_types::HarnessRunId;
use serde_json::Value;

use crate::query::all_runs;
use enforcer_domain::config_types::HarnessConfig;

/// Outcome of a prune pass: the `runId`s that were removed.
#[derive(Debug, Clone, Default)]
pub struct PruneOutcome {
    /// Identifiers of authoritative storage runs removed by this pass.
    pub removed: Vec<HarnessRunId>,
}

/// Run the retention/prune engine over every run under both storage roots
/// (authoritative writes only ever remove from the authoritative root —
/// legacy runs are read-only and never pruned by this engine).
pub fn prune_runs(repo_root: &Path, config: &HarnessConfig) -> Result<PruneOutcome> {
    prune_selected_runs(repo_root, config, true)
}

fn prune_selected_runs(
    repo_root: &Path,
    config: &HarnessConfig,
    authoritative_only: bool,
) -> Result<PruneOutcome> {
    let storage_root = crate::config::storage_root(config, repo_root)?;
    let mut runs = all_runs(repo_root, config)?;
    // Only consider runs actually stored under the authoritative root for
    // removal — legacy-root runs are immutable history.
    let storage_root_rel = crate::legacy::normalize_rel(repo_root, &storage_root);
    if authoritative_only {
        runs.retain(|r| {
            r.get("storage")
                .and_then(|s| s.get("root"))
                .and_then(Value::as_str)
                == Some(storage_root_rel.as_str())
        });
    }

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

    let mut remove: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    let now_ms = epoch_millis();

    for (index, run) in runs.iter().enumerate() {
        let run_id = run
            .get("runId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if let Some(max_runs) = config.max_runs {
            if u64::try_from(index).unwrap_or(u64::MAX) >= max_runs.get() {
                remove.insert(run_id.clone());
            }
        }
        if let Some(prune_after_days) = config.prune_after_days {
            let started_at = run
                .get("startedAt")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if let Some(age_ms) = age_millis(started_at, now_ms) {
                let retention_ms = i64::try_from(prune_after_days.get())
                    .unwrap_or(i64::MAX)
                    .saturating_mul(24 * 60 * 60 * 1000);
                if age_ms > retention_ms {
                    remove.insert(run_id);
                }
            }
        }
    }

    for run in runs
        .iter()
        .filter(|r| r.get("pinned").and_then(Value::as_bool) == Some(true))
    {
        keep.insert(
            run.get("runId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
    }

    let max_failed = config
        .max_failed_runs
        .map(|limit| usize::try_from(limit.get()).unwrap_or(usize::MAX))
        .unwrap_or(usize::MAX);
    for run in runs
        .iter()
        .filter(|r| r.get("status").and_then(Value::as_str) == Some("failed"))
        .take(max_failed)
    {
        keep.insert(
            run.get("runId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
    }

    let max_per_tool = config
        .max_runs_per_tool
        .map(|limit| usize::try_from(limit.get()).unwrap_or(usize::MAX))
        .unwrap_or(usize::MAX);
    let mut by_tool: std::collections::HashMap<String, Vec<&Value>> =
        std::collections::HashMap::new();
    for run in &runs {
        let tool = run
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        by_tool.entry(tool).or_default().push(run);
    }
    for tool_runs in by_tool.values() {
        for run in tool_runs.iter().take(max_per_tool) {
            keep.insert(
                run.get("runId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }

    let mut removed = Vec::new();
    for run in &runs {
        let run_id = run
            .get("runId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if !remove.contains(&run_id) || keep.contains(&run_id) {
            continue;
        }
        let run_root = run
            .get("storage")
            .and_then(|storage| storage.get("root"))
            .and_then(Value::as_str)
            .map(|relative| repo_root.join(relative))
            .unwrap_or_else(|| storage_root.clone());
        let run_dir = run_root.join("runs").join(&run_id);
        if run_dir.exists() {
            std::fs::remove_dir_all(&run_dir)?;
            removed.push(HarnessRunId::from_adapter(&run_id));
        }
    }

    crate::storage::rewrite_manifest(repo_root, &storage_root)?;
    Ok(PruneOutcome { removed })
}

/// Frozen-MJS compatibility only: applies the historical prune algorithm to
/// both readable roots. Native callers must use [`prune_runs`], which keeps
/// legacy history read-only as the safer default.
pub fn prune_runs_frozen_compat(repo_root: &Path, config: &HarnessConfig) -> Result<PruneOutcome> {
    prune_selected_runs(repo_root, config, false)
}

fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Parse an RFC3339 `startedAt` timestamp and return its age in
/// milliseconds relative to `now_ms`. Returns `None` if unparseable
/// (never prunes on a bad timestamp — fail safe).
fn age_millis(started_at: &str, now_ms: i64) -> Option<i64> {
    let epoch_ms = parse_rfc3339_millis(started_at)?;
    Some(now_ms - epoch_ms)
}

/// Minimal RFC3339 `YYYY-MM-DDTHH:MM:SS(.fff)?Z` parser (UTC-only, no
/// external time crate) — sufficient for retention-age comparisons.
fn parse_rfc3339_millis(value: &str) -> Option<i64> {
    let value = value.strip_suffix('Z')?;
    let (date, time) = value.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    let (time_main, millis) = match time.split_once('.') {
        Some((main, frac)) => (
            main,
            frac.get(0..3).unwrap_or(frac).parse::<i64>().unwrap_or(0),
        ),
        None => (time, 0),
    };
    let mut time_parts = time_main.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;

    let days = days_from_civil(year, month, day);
    let millis_total = ((days * 24 + hour) * 60 + minute) * 60 * 1000 + second * 1000 + millis;
    Some(millis_total)
}

/// Days since the Unix epoch for a proleptic-Gregorian civil date
/// (Howard Hinnant's `days_from_civil` algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::storage::{record_run, RunInput};
    use enforcer_core::error::{Error, Result};
    use enforcer_domain::config_types::{HarnessConfig, HarnessRetentionDays, HarnessRunLimit};
    use enforcer_domain::harness_types::{
        HarnessCapturedOutput, HarnessCommandArgument, HarnessPinned, HarnessRunId,
        HarnessTimestamp, HarnessToolName,
    };
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::telemetry_types::ProcessExitCode;

    fn missing(what: &str) -> Error {
        Error::InvalidConfig(format!("test fixture: expected {what}"))
    }

    fn record(
        repo_root: &Path,
        run_id: &str,
        exit_code: i32,
        started_at: &str,
        config: &HarnessConfig,
    ) -> Result<()> {
        let repo_root = RepoRoot::try_from(repo_root)?;
        record_run(
            &RunInput {
                repo_root: &repo_root,
                run_id: HarnessRunId::try_new(run_id.to_owned())?,
                tool: HarnessToolName::try_new("cargo".to_owned())?,
                language: None,
                command: vec![HarnessCommandArgument::try_new("cargo".to_owned())?],
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
            config,
        )?;
        Ok(())
    }

    #[test]
    fn max_runs_prunes_oldest_and_lists_it_in_summary_pruned() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig {
            max_runs: Some(HarnessRunLimit::from_value(2)),
            max_runs_per_tool: Some(HarnessRunLimit::from_value(2)),
            max_failed_runs: Some(HarnessRunLimit::from_value(0)),
            prune_after_days: None,
            ..HarnessConfig::default()
        };
        record(dir.path(), "run-1", 0, "2026-01-01T00:00:00Z", &config)?;
        record(dir.path(), "run-2", 0, "2026-01-02T00:00:00Z", &config)?;
        record(dir.path(), "run-3", 0, "2026-01-03T00:00:00Z", &config)?;

        let runs =
            crate::query::list_runs(dir.path(), &config, &crate::query::RunQuery::default())?;
        let run_ids: Vec<&str> = runs.iter().filter_map(|r| r["runId"].as_str()).collect();
        assert_eq!(
            run_ids,
            vec!["run-3", "run-2"],
            "oldest run-1 must be pruned"
        );

        let latest = crate::query::read_summary(dir.path(), "run-3", &config)?
            .ok_or_else(|| missing("run-3 summary"))?;
        let pruned = latest["pruned"]
            .as_array()
            .ok_or_else(|| missing("pruned array"))?;
        assert!(pruned.iter().any(|v| v.as_str() == Some("run-1")));
        Ok(())
    }

    #[test]
    fn pinned_and_recent_failed_runs_survive_prune() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig {
            max_runs: Some(HarnessRunLimit::from_value(1)),
            max_runs_per_tool: None,
            max_failed_runs: Some(HarnessRunLimit::from_value(5)),
            prune_after_days: None,
            ..HarnessConfig::default()
        };
        let repo_root = RepoRoot::try_from(dir.path())?;
        record_run(
            &RunInput {
                repo_root: &repo_root,
                run_id: HarnessRunId::try_new("run-pinned".to_owned())?,
                tool: HarnessToolName::try_new("cargo".to_owned())?,
                language: None,
                command: vec![HarnessCommandArgument::try_new("cargo".to_owned())?],
                stdout: HarnessCapturedOutput::default(),
                stderr: HarnessCapturedOutput::default(),
                exit_code: ProcessExitCode::new(0),
                crate_name: None,
                package_name: None,
                domain: None,
                tags: vec![],
                pinned: HarnessPinned::Pinned,
                started_at: HarnessTimestamp::try_new("2026-01-01T00:00:00Z".to_owned())?,
                ended_at: HarnessTimestamp::try_new("2026-01-01T00:00:00Z".to_owned())?,
            },
            &config,
        )?;
        record(dir.path(), "run-failed", 1, "2026-01-02T00:00:00Z", &config)?;
        record(dir.path(), "run-latest", 0, "2026-01-03T00:00:00Z", &config)?;

        let run_ids: Vec<String> =
            crate::query::list_runs(dir.path(), &config, &crate::query::RunQuery::default())?
                .iter()
                .filter_map(|r| r["runId"].as_str().map(str::to_owned))
                .collect();
        assert!(
            run_ids.contains(&"run-pinned".to_owned()),
            "pinned run must survive: {run_ids:?}"
        );
        assert!(
            run_ids.contains(&"run-failed".to_owned()),
            "failed run within maxFailedRuns must survive: {run_ids:?}"
        );
        Ok(())
    }

    #[test]
    fn prune_after_days_removes_old_run_keeps_fresh_one() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig {
            max_runs: None,
            max_runs_per_tool: Some(HarnessRunLimit::from_value(1)),
            max_failed_runs: Some(HarnessRunLimit::from_value(0)),
            prune_after_days: Some(HarnessRetentionDays::from_value(14)),
            ..HarnessConfig::default()
        };
        record(dir.path(), "run-old", 0, "2020-01-01T00:00:00Z", &config)?;
        // A fixed recent date, far newer than the 14-day cutoff but still
        // parseable by `parse_rfc3339_millis`.
        record(dir.path(), "run-fresh", 0, "2026-07-04T00:00:00Z", &config)?;

        let run_ids: Vec<String> =
            crate::query::list_runs(dir.path(), &config, &crate::query::RunQuery::default())?
                .iter()
                .filter_map(|r| r["runId"].as_str().map(str::to_owned))
                .collect();
        assert!(
            !run_ids.contains(&"run-old".to_owned()),
            "old run must be pruned: {run_ids:?}"
        );
        assert!(
            run_ids.contains(&"run-fresh".to_owned()),
            "fresh run must survive: {run_ids:?}"
        );
        Ok(())
    }

    #[test]
    fn reset_runs_clears_the_store() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let config = HarnessConfig::default();
        record(dir.path(), "run-1", 0, "2026-01-01T00:00:00Z", &config)?;
        crate::storage::reset_runs(dir.path(), &config)?;
        let runs =
            crate::query::list_runs(dir.path(), &config, &crate::query::RunQuery::default())?;
        assert!(runs.is_empty());
        assert!(!crate::config::storage_root(&config, dir.path())?.exists());
        Ok(())
    }

    #[test]
    fn frozen_compat_prunes_eligible_legacy_runs_without_weakening_native_default() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let permissive = HarnessConfig::default();
        record(
            dir.path(),
            "legacy-old",
            0,
            "2020-01-01T00:00:00Z",
            &permissive,
        )?;
        let authoritative = crate::config::storage_root(&permissive, dir.path())?
            .join("runs")
            .join("legacy-old");
        let legacy_root = crate::config::legacy_storage_root(dir.path());
        std::fs::create_dir_all(legacy_root.join("runs"))?;
        let legacy = legacy_root.join("runs").join("legacy-old");
        std::fs::rename(&authoritative, &legacy)?;
        let summary_path = legacy.join("summary.json");
        let mut summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&summary_path)?)?;
        summary["storage"]["root"] = serde_json::json!(".ocentra-enforcer");
        std::fs::write(
            &summary_path,
            format!("{}\n", serde_json::to_string_pretty(&summary)?),
        )?;
        let restrictive = HarnessConfig {
            max_runs: Some(HarnessRunLimit::from_value(0)),
            max_runs_per_tool: Some(HarnessRunLimit::from_value(0)),
            max_failed_runs: Some(HarnessRunLimit::from_value(0)),
            prune_after_days: None,
            ..HarnessConfig::default()
        };

        let native = super::prune_runs(dir.path(), &restrictive)?;
        assert!(
            native.removed.is_empty(),
            "native retention must leave legacy history read-only"
        );
        assert!(
            legacy.exists(),
            "native retention must not delete the legacy run"
        );
        assert_eq!(crate::query::all_runs(dir.path(), &restrictive)?.len(), 1);

        let frozen = super::prune_runs_frozen_compat(dir.path(), &restrictive)?;
        assert_eq!(
            frozen
                .removed
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>(),
            vec!["legacy-old"]
        );
        assert!(
            !legacy.exists(),
            "frozen compatibility prune must delete an eligible legacy run"
        );
        Ok(())
    }
}
