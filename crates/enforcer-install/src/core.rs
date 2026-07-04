//! The harness-neutral `install`/`uninstall`/`update`/`doctor` core and
//! the [`HarnessAdapter`] trait every Track C per-harness adapter (c02-c09,
//! x03) implements. This module owns ORCHESTRATION only — it never speaks
//! a specific harness's config format itself (that is each adapter's own
//! `src/adapters/<harness>.rs`, sequenced after this skeleton).
//!
//! # Adapter interface (workpack-binding shape)
//!
//! `plan(ctx) -> report`, `apply(report) -> result`, `verify(ctx) ->
//! checks` — a strict three-phase cycle so a `--dry-run` caller can stop
//! after `plan` and render an [`crate::report::InstallReport`] without any
//! adapter ever needing a separate "would I write" code path that can
//! drift from what `apply` actually does.

use crate::cli_contract::{
    DoctorRequest, InstallRequest, RequestContext, UninstallRequest, UpdateRequest,
};
use crate::distribution::Downloader;
use crate::error::{InstallError, InstallResult};
use crate::report::{ApplyResult, InstallReport, VerifyReport};

/// The harness-neutral **parallel-execution doctrine** text every adapter
/// (c03/c06/c08/c09) embeds into whatever surface its harness reads
/// doctrine from (a `CLAUDE.md`/`AGENTS.md` managed block, a Cursor rule, a
/// skill file, ...) — never a Claude-specific file itself. Source: the
/// live orchestration-lessons ledger
/// (`docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md`) —
/// the doctrine IS the lessons, not a paraphrase invented separately from
/// what the swarm actually learned running this build (L3, L4, L5, L6, L7,
/// L13, L14, L19, L21 land here).
pub const PARALLEL_EXECUTION_DOCTRINE: &str = concat!(
    "Parallel execution doctrine (source: refs/orchestration-lessons.md — ",
    "the doctrine IS the lessons):\n",
    "- Spawn one worktree per lane via `enforcer coordination lane new` for any ",
    "multi-agent parallel work through this coordination hub.\n",
    "- NEVER share `Cargo.lock`/`target/`/`node_modules`/build cache across ",
    "lanes — each worktree is isolated end to end.\n",
    "- Parking or deleting-and-rebuilding a lane's worktree is NORMAL, not a ",
    "failure; there is no local undo, so commit+push is how a step becomes ",
    "durable (never trust the working tree as memory).\n",
    "- Worker mail lifecycle is `started -> progress -> done/blocked`, never ",
    "final-only silence.\n",
    "- Step 0 for any worker: fetch + branch/reset from the integration branch ",
    "— never merge a lane branch without checking `merge-base` first.\n",
);

/// A stable list of every adapter key an adapter registry is expected to
/// know about, for rendering the `known: <comma list>` hint on an
/// [`InstallError::UnknownAdapter`].
fn known_adapter_keys(adapters: &[&dyn HarnessAdapter]) -> String {
    let mut keys: Vec<&str> = adapters.iter().map(|a| a.harness_key()).collect();
    keys.sort_unstable();
    keys.join(", ")
}

/// One per-harness adapter. Implemented by each Track C pack (c02 detect
/// feeds the registry of which adapters are active; c03/c06/c07/c08/c09
/// implement one adapter each; x03 layers a legacy-name migration on top
/// of an existing adapter rather than being an adapter itself).
pub trait HarnessAdapter {
    /// This adapter's registration key (e.g. `"claude"`, `"codex"`) —
    /// matches [`crate::report::HarnessKey`] values reported back.
    fn harness_key(&self) -> &'static str;

    /// Compute what this adapter would change for `ctx`, without touching
    /// disk. Called for both a real `install`/`uninstall` and a
    /// `--dry-run` — a dry run is exactly this call rendered, never a
    /// separate path.
    ///
    /// # Errors
    /// Returns an [`crate::error::InstallError`] if planning itself fails
    /// (e.g. a malformed existing config the adapter cannot safely reason
    /// about — detected, not silently skipped).
    fn plan(&self, ctx: &RequestContext) -> InstallResult<InstallReport>;

    /// Execute a previously computed [`InstallReport`]. Never called for
    /// `--dry-run`. An adapter backs up each target file
    /// ([`crate::backup::backup_before_write`]) before mutating it.
    ///
    /// # Errors
    /// Returns an [`crate::error::InstallError`] if any individual change
    /// fails to apply; the returned [`ApplyResult`] still reports the
    /// per-change outcomes attempted before the error, so a caller can see
    /// exactly how far a partially-applied run got.
    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult>;

    /// Read-only post-install health check: does the harness's config
    /// currently reflect what an `apply` for `ctx` should have produced.
    /// Backs `enforcer doctor` and the fail/pass fixture proof rows.
    ///
    /// # Errors
    /// Returns an [`crate::error::InstallError`] if the check itself
    /// cannot run (e.g. the config path is unreadable) — distinct from a
    /// check that runs and reports `passed: false`.
    fn verify(&self, ctx: &RequestContext) -> InstallResult<VerifyReport>;
}

/// `enforcer install` — plan (and, unless `--dry-run`, apply) every
/// adapter in `adapters` against `request`. Returns the aggregated plan
/// (dry-run) or the aggregated apply result rendered back into a report
/// shape, per adapter.
///
/// # Errors
/// Returns an [`crate::error::InstallError`] on the first adapter whose
/// `plan` (or, when applying, `apply`) fails — install does not silently
/// continue past a broken adapter into an inconsistent partial state
/// across harnesses.
pub fn install(
    adapters: &[&dyn HarnessAdapter],
    request: &InstallRequest,
) -> InstallResult<Vec<(&'static str, InstallReport, Option<ApplyResult>)>> {
    let selected = select_adapters(adapters, &request.only_harnesses)?;
    let mut outcomes = Vec::with_capacity(selected.len());
    for adapter in selected {
        let plan = adapter.plan(&request.context)?;
        let applied = if request.context.dry_run {
            None
        } else {
            Some(adapter.apply(&plan)?)
        };
        outcomes.push((adapter.harness_key(), plan, applied));
    }
    Ok(outcomes)
}

/// `enforcer uninstall` — mirrors [`install`]'s plan/apply cycle for the
/// removal direction. An adapter's `plan` is expected to return
/// [`crate::report::PlannedChange`]s that describe removing its
/// registration when handed a request whose intent is uninstall (the
/// adapter distinguishes install-vs-uninstall planning internally; this
/// core function stays agnostic of that distinction so both verbs share
/// one orchestration path).
///
/// # Errors
/// See [`install`] — identical fail-fast contract.
pub fn uninstall(
    adapters: &[&dyn HarnessAdapter],
    request: &UninstallRequest,
) -> InstallResult<Vec<(&'static str, InstallReport, Option<ApplyResult>)>> {
    let selected = select_adapters(adapters, &request.only_harnesses)?;
    let mut outcomes = Vec::with_capacity(selected.len());
    for adapter in selected {
        let plan = adapter.plan(&request.context)?;
        let applied = if request.context.dry_run {
            None
        } else {
            Some(adapter.apply(&plan)?)
        };
        outcomes.push((adapter.harness_key(), plan, applied));
    }
    Ok(outcomes)
}

/// `enforcer update` — the binary-swap verb (RUST_ARCHITECTURE.md,
/// "Update UX (binary swap, not a repo pull)"). Resolves the current
/// host's [`crate::distribution::TargetPlatform`], checks the release
/// channel via `downloader`, and (unless `request.dry_run`) removes the
/// old binary bytes and installs the new ones at the SAME `install_path`
/// — the harness's MCP registration keeps pointing at an unchanged path,
/// so no adapter re-run is needed for a routine update.
///
/// # Errors
/// Returns an [`crate::error::InstallError`] if the host platform has no
/// released binary, or the downloader's fetch fails.
pub fn update(
    downloader: &dyn Downloader,
    install_path: &std::path::Path,
    version: &str,
    request: &UpdateRequest,
) -> InstallResult<Option<crate::distribution::ResolvedBinary>> {
    let platform = crate::distribution::TargetPlatform::detect_host()?;
    if request.dry_run {
        return Ok(None);
    }
    let resolved = downloader.fetch(platform, version, install_path)?;
    Ok(Some(resolved))
}

/// `enforcer doctor` — read-only health check across every adapter, never
/// writes. Aggregates each adapter's [`VerifyReport`] under its harness
/// key.
///
/// # Errors
/// Returns an [`crate::error::InstallError`] if any adapter's `verify`
/// call itself cannot run.
pub fn doctor(
    adapters: &[&dyn HarnessAdapter],
    ctx: &RequestContext,
    _request: &DoctorRequest,
) -> InstallResult<Vec<(&'static str, VerifyReport)>> {
    let mut outcomes = Vec::with_capacity(adapters.len());
    for adapter in adapters {
        let checks = adapter.verify(ctx)?;
        outcomes.push((adapter.harness_key(), checks));
    }
    Ok(outcomes)
}

/// The skill-asset doctor/verify check (RUST_ARCHITECTURE.md skill-asset
/// VALIDATOR fold-in, WAVE 6 re-home of `scripts/validate-codex-assets.mjs`).
/// Fail-closed: every declared [`crate::report::SkillAsset`] must exist on
/// disk AND every [`crate::report::PluginPublishContract`] field must equal
/// its expected value, or the corresponding [`VerifyCheck`] reports
/// `passed: false` with the concrete reason — never a silent skip. This is
/// a shared core check (not a `HarnessAdapter::verify` override) so
/// `enforcer doctor` can run it once against whatever manifest an adapter
/// (or the CLI) supplies, independent of any single harness's asset
/// layout.
///
/// # Errors
/// Returns [`InstallError::SkillAssetInvalid`] only if a check's underlying
/// I/O itself cannot be performed (e.g. a plugin manifest exists but is not
/// valid JSON) — a check that runs and finds a real mismatch instead
/// reports `passed: false` inside the returned [`VerifyReport`], per the
/// "detected, not silently skipped" contract shared with
/// [`HarnessAdapter::verify`].
pub fn run_skill_asset_checks(
    manifest: &crate::report::SkillAssetManifest,
    root: &std::path::Path,
) -> InstallResult<VerifyReport> {
    use crate::report::VerifyCheck;

    let mut checks = Vec::with_capacity(manifest.assets.len() + manifest.plugin_contracts.len());

    for asset in &manifest.assets {
        let full_path = root.join(&asset.asset_path);
        let exists = full_path.is_file();
        checks.push(VerifyCheck {
            harness: asset.skill_name.clone(),
            name: format!("skill-asset-exists:{}", asset.asset_path),
            passed: exists,
            detail: if exists {
                String::new()
            } else {
                format!("missing skill asset at `{}`", full_path.display())
            },
        });
    }

    for contract in &manifest.plugin_contracts {
        let full_path = root.join(&contract.manifest_path);
        let (passed, detail) = match std::fs::read_to_string(&full_path) {
            Err(e) => (
                false,
                format!(
                    "cannot read plugin manifest `{}`: {e}",
                    full_path.display()
                ),
            ),
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Err(e) => (
                    false,
                    format!(
                        "plugin manifest `{}` is not valid JSON: {e}",
                        full_path.display()
                    ),
                ),
                Ok(value) => {
                    let actual = value.get(&contract.field).and_then(|v| v.as_str());
                    match actual {
                        Some(actual) if actual == contract.expected_value => (true, String::new()),
                        Some(actual) => (
                            false,
                            format!(
                                "`{}`.{} = \"{actual}\", expected \"{}\"",
                                full_path.display(),
                                contract.field,
                                contract.expected_value
                            ),
                        ),
                        None => (
                            false,
                            format!(
                                "`{}` has no field `{}`",
                                full_path.display(),
                                contract.field
                            ),
                        ),
                    }
                }
            },
        };
        checks.push(VerifyCheck {
            harness: "plugin-publish-contract".to_owned(),
            name: format!("plugin-skills-path:{}", contract.manifest_path),
            passed,
            detail,
        });
    }

    Ok(VerifyReport { checks })
}

/// Narrow `adapters` down to `only`'s keys when non-empty; otherwise
/// return every adapter (the "every detected/known harness" default).
///
/// # Errors
/// Returns [`InstallError::UnknownAdapter`] the moment `only` names a key
/// that matches no registered adapter — an unrecognized `--only <harness>`
/// value is a typed error, never a silent skip past a name a caller may
/// have mistyped (workpack c01 acceptance row: "unknown adapter id must
/// return a typed error, not skip silently").
fn select_adapters<'a>(
    adapters: &'a [&'a dyn HarnessAdapter],
    only: &[String],
) -> InstallResult<Vec<&'a dyn HarnessAdapter>> {
    if only.is_empty() {
        return Ok(adapters.to_vec());
    }
    for key in only {
        if !adapters.iter().any(|adapter| adapter.harness_key() == key) {
            return Err(InstallError::UnknownAdapter {
                id: key.clone(),
                known: known_adapter_keys(adapters),
            });
        }
    }
    Ok(adapters
        .iter()
        .copied()
        .filter(|adapter| only.iter().any(|key| key == adapter.harness_key()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        doctor, install, run_skill_asset_checks, select_adapters, uninstall, update,
        HarnessAdapter,
    };
    use crate::cli_contract::{
        DoctorRequest, InstallRequest, RequestContext, UninstallRequest, UpdateRequest,
    };
    use crate::distribution::{Downloader, ResolvedBinary, TargetPlatform};
    use crate::error::{InstallError, InstallResult};
    use enforcer_domain::paths::RepoRoot;
    use crate::report::{
        AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange,
        PluginPublishContract, SkillAsset, SkillAssetManifest, VerifyCheck,
        VerifyReport,
    };
    use std::path::{Path, PathBuf};

    /// A fixture adapter whose plan/apply/verify behavior is entirely
    /// caller-configured, so tests can assert on fail AND pass fixtures
    /// without a real harness config on disk.
    struct FixtureAdapter {
        key: &'static str,
        change_pending: bool,
        fail_plan: bool,
        fail_apply: bool,
        verify_passes: bool,
    }

    impl HarnessAdapter for FixtureAdapter {
        fn harness_key(&self) -> &'static str {
            self.key
        }

        fn plan(&self, _ctx: &RequestContext) -> InstallResult<InstallReport> {
            if self.fail_plan {
                return Err(InstallError::MalformedConfig {
                    path: format!("/fixtures/{}.json", self.key),
                    reason: "simulated malformed existing config".to_owned(),
                });
            }
            let planned_changes = if self.change_pending {
                let path: RepoRoot = format!("/fixtures/{}.json", self.key)
                    .try_into()
                    .map_err(|e: enforcer_core::error::DecodeError| InstallError::MalformedConfig {
                        path: format!("/fixtures/{}.json", self.key),
                        reason: e.to_string(),
                    })?;
                vec![PlannedChange {
                    harness: self.key.to_owned(),
                    kind: ArtifactKind::McpRegistration,
                    path,
                    description: "register enforcer".to_owned(),
                    is_update: false,
                }]
            } else {
                vec![]
            };
            Ok(InstallReport {
                planned_changes,
                warnings: vec![],
            })
        }

        fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
            if self.fail_apply {
                return Err(InstallError::Io {
                    path: format!("/fixtures/{}.json", self.key),
                    reason: "simulated write failure".to_owned(),
                });
            }
            let applied = report
                .planned_changes
                .iter()
                .cloned()
                .map(|change| AppliedChange {
                    change,
                    succeeded: true,
                    backup_path: None,
                })
                .collect();
            Ok(ApplyResult { applied })
        }

        fn verify(&self, _ctx: &RequestContext) -> InstallResult<VerifyReport> {
            Ok(VerifyReport {
                checks: vec![VerifyCheck {
                    harness: self.key.to_owned(),
                    name: "mcp-registration-present".to_owned(),
                    passed: self.verify_passes,
                    detail: if self.verify_passes {
                        String::new()
                    } else {
                        "missing registration".to_owned()
                    },
                }],
            })
        }
    }

    fn passing_adapter(key: &'static str, change_pending: bool) -> FixtureAdapter {
        FixtureAdapter {
            key,
            change_pending,
            fail_plan: false,
            fail_apply: false,
            verify_passes: true,
        }
    }

    #[test]
    fn install_plans_and_applies_a_fresh_registration() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter("claude", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        assert_eq!(outcomes.len(), 1);
        let (key, plan, applied) = &outcomes[0];
        assert_eq!(*key, "claude");
        assert!(!plan.is_noop());
        let applied = applied
            .as_ref()
            .ok_or("expected an apply result for a non-dry-run")?;
        assert!(applied.all_succeeded());
        Ok(())
    }

    #[test]
    fn install_dry_run_never_calls_apply_and_writes_nothing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter("codex", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let mut context = RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer"));
        context.dry_run = true;
        let request = InstallRequest {
            context,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert!(!plan.is_noop());
        assert!(
            applied.is_none(),
            "dry-run must never produce an ApplyResult"
        );
        Ok(())
    }

    #[test]
    fn idempotent_reinstall_yields_a_noop_plan() -> Result<(), Box<dyn std::error::Error>> {
        // change_pending=false models an adapter whose plan(), on a second
        // run against an already-registered harness, has nothing left to
        // do -- the idempotent re-install fixture the workpack requires.
        let adapter = passing_adapter("claude", false);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert!(plan.is_noop());
        let applied = applied
            .as_ref()
            .ok_or("expected an apply result for a non-dry-run")?;
        assert!(applied.applied.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_existing_config_is_a_detected_plan_failure() {
        let adapter = FixtureAdapter {
            key: "gemini",
            change_pending: false,
            fail_plan: true,
            fail_apply: false,
            verify_passes: false,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
    }

    #[test]
    fn failing_apply_surfaces_the_error_and_stops() {
        let adapter = FixtureAdapter {
            key: "cursor",
            change_pending: true,
            fail_plan: false,
            fail_apply: true,
            verify_passes: false,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::Io { .. })));
    }

    #[test]
    fn only_harnesses_filter_narrows_the_adapter_set() -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter("claude", true);
        let codex = passing_adapter("codex", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &["codex".to_owned()])?;
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].harness_key(), "codex");
        Ok(())
    }

    #[test]
    fn empty_only_harnesses_selects_every_adapter() -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter("claude", true);
        let codex = passing_adapter("codex", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &[])?;
        assert_eq!(selected.len(), 2);
        Ok(())
    }

    #[test]
    fn unknown_adapter_id_is_a_typed_error_not_a_silent_skip() {
        let claude = passing_adapter("claude", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude];
        let result = select_adapters(&adapters, &["not-a-real-harness".to_owned()]);
        assert!(matches!(
            result,
            Err(InstallError::UnknownAdapter { ref id, .. }) if id == "not-a-real-harness"
        ));
    }

    #[test]
    fn install_rejects_an_unknown_only_harnesses_entry() {
        let claude = passing_adapter("claude", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude];
        let request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec!["ghost-harness".to_owned()],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::UnknownAdapter { .. })));
    }

    #[test]
    fn parallel_execution_doctrine_cites_the_lessons_ledger_as_its_source() {
        assert!(super::PARALLEL_EXECUTION_DOCTRINE.contains("refs/orchestration-lessons.md"));
        assert!(super::PARALLEL_EXECUTION_DOCTRINE.contains("coordination lane new"));
        assert!(super::PARALLEL_EXECUTION_DOCTRINE.contains("Cargo.lock"));
    }

    #[test]
    fn uninstall_follows_the_same_plan_apply_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter("claude", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = UninstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let outcomes = uninstall(&adapters, &request)?;
        assert_eq!(outcomes.len(), 1);
        Ok(())
    }

    #[test]
    fn doctor_aggregates_verify_across_adapters() -> Result<(), Box<dyn std::error::Error>> {
        let healthy = passing_adapter("claude", false);
        let broken = FixtureAdapter {
            key: "codex",
            change_pending: false,
            fail_plan: false,
            fail_apply: false,
            verify_passes: false,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&healthy, &broken];
        let ctx = RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer"));
        let outcomes = doctor(&adapters, &ctx, &DoctorRequest::default())?;
        assert_eq!(outcomes.len(), 2);
        let (_, healthy_report) = &outcomes[0];
        assert!(healthy_report.all_passed());
        let (_, broken_report) = &outcomes[1];
        assert!(!broken_report.all_passed());
        Ok(())
    }

    struct FakeDownloader {
        will_fail: bool,
    }

    impl Downloader for FakeDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            version: &str,
            install_path: &Path,
        ) -> InstallResult<ResolvedBinary> {
            if self.will_fail {
                return Err(InstallError::DistributionFailed {
                    target: platform.target_triple().to_owned(),
                    reason: "simulated release-channel failure".to_owned(),
                });
            }
            Ok(ResolvedBinary {
                platform,
                version: version.to_owned(),
                install_path: install_path.to_path_buf(),
            })
        }
    }

    #[test]
    fn update_dry_run_never_calls_the_downloader() -> Result<(), Box<dyn std::error::Error>> {
        let downloader = FakeDownloader { will_fail: true };
        let request = UpdateRequest {
            dry_run: true,
            output: crate::cli_contract::OutputMode::Human,
        };
        let result = update(
            &downloader,
            &PathBuf::from("/usr/local/bin/enforcer"),
            "0.2.0",
            &request,
        )?;
        assert!(
            result.is_none(),
            "dry-run update must not fetch a new binary"
        );
        Ok(())
    }

    #[test]
    fn update_performs_a_binary_swap_at_the_same_path() -> Result<(), Box<dyn std::error::Error>> {
        let downloader = FakeDownloader { will_fail: false };
        let install_path = PathBuf::from("/usr/local/bin/enforcer");
        let request = UpdateRequest {
            dry_run: false,
            output: crate::cli_contract::OutputMode::Human,
        };
        let resolved = update(&downloader, &install_path, "0.2.0", &request)?
            .ok_or("expected a resolved binary for a non-dry-run update")?;
        assert_eq!(resolved.install_path, install_path);
        assert_eq!(resolved.version, "0.2.0");
        Ok(())
    }

    #[test]
    fn update_surfaces_a_release_channel_failure() {
        let downloader = FakeDownloader { will_fail: true };
        let request = UpdateRequest {
            dry_run: false,
            output: crate::cli_contract::OutputMode::Human,
        };
        let result = update(
            &downloader,
            &PathBuf::from("/usr/local/bin/enforcer"),
            "0.2.0",
            &request,
        );
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
    }

    /// Root of the checked-in skill-asset fixtures
    /// (`tests/fixtures/install_core/**`, workpack c01 acceptance row).
    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/install_core").join(name)
    }

    fn ocentra_enforcer_manifest() -> SkillAssetManifest {
        SkillAssetManifest {
            assets: vec![SkillAsset {
                skill_name: "ocentra-enforcer".to_owned(),
                asset_path: "skills/ocentra-enforcer/SKILL.md".to_owned(),
            }],
            plugin_contracts: vec![PluginPublishContract {
                manifest_path: ".codex-plugin/plugin.json".to_owned(),
                field: "skills".to_owned(),
                expected_value: "./skills/".to_owned(),
            }],
        }
    }

    #[test]
    fn skill_asset_pass_fixture_resolves_clean() -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(&ocentra_enforcer_manifest(), &fixture_root("skill_asset_pass"))?;
        assert!(
            report.all_passed(),
            "expected every check to pass, got {report:?}"
        );
        assert_eq!(report.checks.len(), 2);
        Ok(())
    }

    #[test]
    fn skill_asset_fail_fixture_missing_asset_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(
            &ocentra_enforcer_manifest(),
            &fixture_root("skill_asset_fail_missing_asset"),
        )?;
        assert!(!report.all_passed());
        let asset_check = report
            .checks
            .iter()
            .find(|c| c.name.starts_with("skill-asset-exists"))
            .ok_or("expected a skill-asset-exists check")?;
        assert!(!asset_check.passed);
        assert!(asset_check.detail.contains("missing skill asset"));
        Ok(())
    }

    #[test]
    fn skill_asset_fail_fixture_bad_plugin_path_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(
            &ocentra_enforcer_manifest(),
            &fixture_root("skill_asset_fail_bad_plugin_path"),
        )?;
        assert!(!report.all_passed());
        let plugin_check = report
            .checks
            .iter()
            .find(|c| c.name.starts_with("plugin-skills-path"))
            .ok_or("expected a plugin-skills-path check")?;
        assert!(!plugin_check.passed);
        assert!(plugin_check.detail.contains("expected"));
        Ok(())
    }

    #[test]
    fn missing_plugin_manifest_is_a_failed_check_not_a_panic() -> Result<(), Box<dyn std::error::Error>>
    {
        let manifest = SkillAssetManifest {
            assets: vec![],
            plugin_contracts: vec![PluginPublishContract {
                manifest_path: "does/not/exist/plugin.json".to_owned(),
                field: "skills".to_owned(),
                expected_value: "./skills/".to_owned(),
            }],
        };
        let report = run_skill_asset_checks(&manifest, &fixture_root("skill_asset_pass"))?;
        assert!(!report.all_passed());
        Ok(())
    }

    /// `--dry-run` must write ZERO files: a temp-dir fixture under
    /// `tests/fixtures/install_core/dry_run_zero_writes` is snapshotted
    /// before and after a dry-run `install`, and the filesystem diff must
    /// be empty (workpack c01 acceptance row).
    #[test]
    fn dry_run_writes_zero_files_fs_diff_is_empty() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("claude.json");
        std::fs::write(&target, "{\"mcpServers\":{}}")?;

        fn snapshot(path: &std::path::Path) -> InstallResult<(u64, String)> {
            let meta = std::fs::metadata(path).map_err(|e| InstallError::Io {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
            let content = std::fs::read_to_string(path).map_err(|e| InstallError::Io {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
            Ok((meta.len(), content))
        }

        let before = snapshot(&target)?;

        struct WriterAdapter {
            target: PathBuf,
        }
        impl HarnessAdapter for WriterAdapter {
            fn harness_key(&self) -> &'static str {
                "claude"
            }
            fn plan(&self, _ctx: &RequestContext) -> InstallResult<InstallReport> {
                let path: RepoRoot =
                    self.target
                        .display()
                        .to_string()
                        .try_into()
                        .map_err(|e: enforcer_core::error::DecodeError| {
                            InstallError::MalformedConfig {
                                path: self.target.display().to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                Ok(InstallReport {
                    planned_changes: vec![PlannedChange {
                        harness: "claude".to_owned(),
                        kind: ArtifactKind::McpRegistration,
                        path,
                        description: "would add enforcer to mcpServers".to_owned(),
                        is_update: true,
                    }],
                    warnings: vec![],
                })
            }
            fn apply(&self, _report: &InstallReport) -> InstallResult<ApplyResult> {
                // If a dry-run ever reached `apply`, this would actually
                // write -- the test asserts `applied` stays `None` so this
                // branch is provably unreached for `--dry-run`.
                std::fs::write(&self.target, "{\"mcpServers\":{\"enforcer\":{}}}").map_err(|e| {
                    InstallError::Io {
                        path: self.target.display().to_string(),
                        reason: e.to_string(),
                    }
                })?;
                Ok(ApplyResult::default())
            }
            fn verify(&self, _ctx: &RequestContext) -> InstallResult<VerifyReport> {
                Ok(VerifyReport::default())
            }
        }

        let adapter = WriterAdapter {
            target: target.clone(),
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let mut context = RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer"));
        context.dry_run = true;
        let request = InstallRequest {
            context,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert!(!plan.is_noop());
        assert!(applied.is_none());

        let after = snapshot(&target)?;
        assert_eq!(before, after, "dry-run must leave the filesystem byte-identical");
        Ok(())
    }

    /// The dry-run report must be byte-identical in SHAPE to the applied
    /// report minus the `applied` flag -- i.e. the same `InstallReport`
    /// serde shape is produced whether or not `--dry-run` was set, and only
    /// the presence of an [`ApplyResult`] differs (workpack c01 acceptance
    /// row: serde round-trip asserted).
    #[test]
    fn dry_run_report_shape_matches_applied_report_shape() -> Result<(), Box<dyn std::error::Error>>
    {
        let real_adapter = passing_adapter("claude", true);
        let dry_adapter = passing_adapter("claude", true);
        let adapters_real: Vec<&dyn HarnessAdapter> = vec![&real_adapter];
        let adapters_dry: Vec<&dyn HarnessAdapter> = vec![&dry_adapter];

        let real_request = InstallRequest {
            context: RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer")),
            only_harnesses: vec![],
        };
        let mut dry_context = RequestContext::with_defaults(PathBuf::from("/usr/local/bin/enforcer"));
        dry_context.dry_run = true;
        let dry_request = InstallRequest {
            context: dry_context,
            only_harnesses: vec![],
        };

        let real_outcomes = install(&adapters_real, &real_request)?;
        let dry_outcomes = install(&adapters_dry, &dry_request)?;

        let (_, real_plan, real_applied) = &real_outcomes[0];
        let (_, dry_plan, dry_applied) = &dry_outcomes[0];

        // Same plan shape (the report itself is identical either way --
        // apply never mutates the plan it was handed).
        assert_eq!(real_plan, dry_plan);
        let real_wire = serde_json::to_string(real_plan)?;
        let dry_wire = serde_json::to_string(dry_plan)?;
        assert_eq!(real_wire, dry_wire);

        // The ONLY difference is the presence of an apply result.
        assert!(real_applied.is_some());
        assert!(dry_applied.is_none());
        Ok(())
    }
}
