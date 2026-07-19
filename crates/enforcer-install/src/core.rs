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
//! after `plan` and render an [`crate::report::InstallReportDto`] without any
//! adapter ever needing a separate "would I write" code path that can
//! drift from what `apply` actually does.

use crate::distribution::Downloader;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::ids::HarnessId;
use enforcer_domain::install_types::{
    ApplyResult, CheckStatus, CheckSubject, DoctorCommand, InstallCommand, InstallReport,
    InstallReportText, InstallRequestContext, InstallVerifyCheck, InstallVerifyReport,
    KnownHarnesses, ResolvedBinary, SkillAssetManifest, UninstallCommand, UpdateCommand,
};

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
fn known_adapter_keys(adapters: &[&dyn HarnessAdapter]) -> KnownHarnesses {
    let mut keys: Vec<HarnessId> = adapters
        .iter()
        .map(|adapter| adapter.harness_key())
        .collect();
    keys.sort_unstable();
    KnownHarnesses::from_sorted(keys)
}

/// One per-harness adapter. Implemented by each Track C pack (c02 detect
/// feeds the registry of which adapters are active; c03/c06/c07/c08/c09
/// implement one adapter each; x03 layers a legacy-name migration on top
/// of an existing adapter rather than being an adapter itself).
pub trait HarnessAdapter {
    /// This adapter's registration key (e.g. `"claude"`, `"codex"`) —
    /// matches [`crate::report::HarnessKey`] values reported back.
    fn harness_key(&self) -> HarnessId;

    /// Compute what this adapter would change for `ctx`, without touching
    /// disk. Called for both a real `install`/`uninstall` and a
    /// `--dry-run` — a dry run is exactly this call rendered, never a
    /// separate path.
    ///
    /// # Errors
    /// Returns an [`crate::error::InstallError`] if planning itself fails
    /// (e.g. a malformed existing config the adapter cannot safely reason
    /// about — detected, not silently skipped).
    fn plan(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport>;

    /// Execute a previously computed [`InstallReportDto`]. Never called for
    /// `--dry-run`. An adapter backs up each target file
    /// ([`crate::backup::backup_before_write`]) before mutating it.
    ///
    /// # Errors
    /// Returns an [`crate::error::InstallError`] if any individual change
    /// fails to apply; the returned [`ApplyResultDto`] still reports the
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
    fn verify(&self, ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport>;
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
    request: &InstallCommand,
) -> InstallResult<Vec<(HarnessId, InstallReport, Option<ApplyResult>)>> {
    let selected = select_adapters(adapters, &request.only_harnesses)?;
    let mut outcomes = Vec::with_capacity(selected.len());
    for adapter in selected {
        let plan = adapter.plan(&request.context)?;
        let applied = if matches!(
            request.context.dry_run,
            enforcer_domain::install_types::DryRun::Enabled
        ) {
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
/// [`crate::report::PlannedChangeDto`]s that describe removing its
/// registration when handed a request whose intent is uninstall (the
/// adapter distinguishes install-vs-uninstall planning internally; this
/// core function stays agnostic of that distinction so both verbs share
/// one orchestration path).
///
/// # Errors
/// See [`install`] — identical fail-fast contract.
pub fn uninstall(
    adapters: &[&dyn HarnessAdapter],
    request: &UninstallCommand,
) -> InstallResult<Vec<(HarnessId, InstallReport, Option<ApplyResult>)>> {
    let selected = select_adapters(adapters, &request.only_harnesses)?;
    let mut outcomes = Vec::with_capacity(selected.len());
    for adapter in selected {
        let plan = adapter.plan(&request.context)?;
        let applied = if matches!(
            request.context.dry_run,
            enforcer_domain::install_types::DryRun::Enabled
        ) {
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
/// host's [`enforcer_domain::install_types::TargetPlatform`], checks the release
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
    version: &enforcer_domain::install_types::ReleaseVersion,
    request: &UpdateCommand,
) -> InstallResult<Option<ResolvedBinary>> {
    let platform = crate::distribution::detect_host()?;
    if matches!(
        request.dry_run,
        enforcer_domain::install_types::DryRun::Enabled
    ) {
        return Ok(None);
    }
    let resolved = downloader.fetch(platform, version, install_path)?;
    Ok(Some(resolved))
}

/// `enforcer doctor` — read-only health check across every adapter, never
/// writes. Aggregates each adapter's [`VerifyReportDto`] under its harness
/// key.
///
/// # Errors
/// Returns an [`crate::error::InstallError`] if any adapter's `verify`
/// call itself cannot run.
pub fn doctor(
    adapters: &[&dyn HarnessAdapter],
    ctx: &InstallRequestContext,
    _request: &DoctorCommand,
) -> InstallResult<Vec<(HarnessId, InstallVerifyReport)>> {
    let mut outcomes = Vec::with_capacity(adapters.len());
    for adapter in adapters {
        let checks = adapter.verify(ctx)?;
        outcomes.push((adapter.harness_key(), checks));
    }
    Ok(outcomes)
}

/// The skill-asset doctor/verify check (RUST_ARCHITECTURE.md skill-asset
/// VALIDATOR fold-in, WAVE 6 re-home of `scripts/validate-codex-assets.mjs`).
/// Fail-closed: every declared [`crate::report::SkillAssetDto`] must exist on
/// disk AND every [`crate::report::PluginPublishContractDto`] field must equal
/// its expected value, or the corresponding [`VerifyCheckDto`] reports
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
/// reports `passed: false` inside the returned [`VerifyReportDto`], per the
/// "detected, not silently skipped" contract shared with
/// [`HarnessAdapter::verify`].
pub fn run_skill_asset_checks(
    manifest: &SkillAssetManifest,
    root: &std::path::Path,
) -> InstallResult<InstallVerifyReport> {
    let mut checks = Vec::with_capacity(manifest.assets.len() + manifest.plugin_contracts.len());

    for asset in &manifest.assets {
        let full_path = root.join(asset.path.as_path());
        let exists = full_path.is_file();
        checks.push(InstallVerifyCheck {
            // CLONE-JUSTIFICATION: the owned typed value must be retained independently by the returned report.
            subject: CheckSubject::SkillAsset(asset.name.clone()),
            name: InstallReportText::try_from(format!(
                "skill-asset-exists:{}",
                asset.path.as_path().display()
            ))?,
            status: if exists {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            detail: InstallReportText::try_from(if exists {
                // BRAND-INVARIANT: the empty detail is immediately validated by InstallReportText.
                String::new()
            } else {
                format!("missing skill asset at `{}`", full_path.display())
            })?,
        });
    }

    for contract in &manifest.plugin_contracts {
        let full_path = root.join(contract.manifest_path.as_path());
        let (passed, detail) = match std::fs::read_to_string(&full_path) {
            Err(e) => (
                false,
                format!("cannot read plugin manifest `{}`: {e}", full_path.display()),
            ),
            Ok(raw) => match crate::boundary::json_wire::decode_value(&raw) {
                Err(e) => (
                    false,
                    format!(
                        "plugin manifest `{}` is not valid JSON: {e}",
                        full_path.display()
                    ),
                ),
                Ok(value) => {
                    let actual = value.get(contract.field.as_str()).and_then(|v| v.as_str());
                    match actual {
                        Some(actual) if actual == contract.expected_value.as_str() => {
                            (true, String::new())
                        }
                        Some(actual) => (
                            false,
                            format!(
                                "`{}`.{} = \"{actual}\", expected \"{}\"",
                                full_path.display(),
                                contract.field.as_str(),
                                contract.expected_value.as_str()
                            ),
                        ),
                        None => (
                            false,
                            format!(
                                "`{}` has no field `{}`",
                                full_path.display(),
                                contract.field.as_str()
                            ),
                        ),
                    }
                }
            },
        };
        checks.push(InstallVerifyCheck {
            subject: CheckSubject::SkillAsset(InstallReportText::try_from(
                // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                "plugin-publish-contract".to_owned(),
            )?),
            name: InstallReportText::try_from(format!(
                "plugin-skills-path:{}",
                contract.manifest_path.as_path().display()
            ))?,
            status: if passed {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            detail: InstallReportText::try_from(detail)?,
        });
    }

    Ok(InstallVerifyReport { checks })
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
    only: &[HarnessId],
) -> InstallResult<Vec<&'a dyn HarnessAdapter>> {
    if only.is_empty() {
        return Ok(adapters.to_vec());
    }
    for key in only {
        if !adapters.iter().any(|adapter| adapter.harness_key() == *key) {
            return Err(InstallError::UnknownAdapter {
                // CLONE-JUSTIFICATION: the owned typed value must be retained independently by the returned report.
                id: key.clone(),
                known: known_adapter_keys(adapters),
            });
        }
    }
    Ok(adapters
        .iter()
        .copied()
        .filter(|adapter| only.iter().any(|key| key == &adapter.harness_key()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        doctor, install, run_skill_asset_checks, select_adapters, uninstall, update, HarnessAdapter,
    };
    use crate::distribution::Downloader;
    use crate::error::{InstallError, InstallResult};
    use enforcer_domain::install_types::{
        AppliedInstallChange, ApplyResult, ArtifactKind, ChangeDisposition, CheckStatus,
        CheckSubject, DoctorCommand, InstallCommand, InstallReport, InstallReportText,
        InstallRequestContext, InstallVerifyCheck, InstallVerifyReport, PlannedInstallChange,
        PluginPublishContract, SkillAsset, SkillAssetManifest, SkillAssetPath, UninstallCommand,
        UpdateCommand,
    };
    use enforcer_domain::install_types::{InstallBinaryPath, ResolvedBinary};
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::{ids::HarnessId, install_types::TargetPlatform};
    use std::path::{Path, PathBuf};

    /// A fixture adapter whose plan/apply/verify behavior is entirely
    /// caller-configured, so tests can assert on fail AND pass fixtures
    /// without a real harness config on disk.
    enum FixturePlan {
        ChangePending,
        NoChange,
        Fail,
    }

    enum FixtureApply {
        Pass,
        Fail,
    }

    struct FixtureAdapter {
        key: HarnessId,
        plan_behavior: FixturePlan,
        apply_behavior: FixtureApply,
        verify_status: CheckStatus,
    }

    impl HarnessAdapter for FixtureAdapter {
        fn harness_key(&self) -> HarnessId {
            self.key.clone()
        }

        fn plan(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
            if matches!(self.plan_behavior, FixturePlan::Fail) {
                return Err(InstallError::MalformedConfig {
                    path: format!("/fixtures/{}.json", self.key.as_str()),
                    reason: "simulated malformed existing config".to_owned(),
                });
            }
            let planned_changes = if matches!(self.plan_behavior, FixturePlan::ChangePending) {
                let path: RepoRoot = format!("/fixtures/{}.json", self.key.as_str())
                    .try_into()
                    .map_err(|e: enforcer_domain::boundary::decode_error::DecodeError| {
                        InstallError::MalformedConfig {
                            path: format!("/fixtures/{}.json", self.key.as_str()),
                            reason: e.to_string(),
                        }
                    })?;
                vec![PlannedInstallChange {
                    harness: self.key.clone(),
                    kind: ArtifactKind::McpRegistration,
                    path,
                    description: InstallReportText::try_from("register enforcer".to_owned())?,
                    disposition: ChangeDisposition::Create,
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
            if matches!(self.apply_behavior, FixtureApply::Fail) {
                return Err(InstallError::Io {
                    path: format!("/fixtures/{}.json", self.key.as_str()),
                    reason: "simulated write failure".to_owned(),
                });
            }
            let applied = report
                .planned_changes
                .iter()
                .cloned()
                .map(|change| AppliedInstallChange {
                    change,
                    status: CheckStatus::Passed,
                    backup_path: None,
                })
                .collect();
            Ok(ApplyResult { applied })
        }

        fn verify(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
            Ok(InstallVerifyReport {
                checks: vec![InstallVerifyCheck {
                    subject: CheckSubject::Harness(self.key.clone()),
                    name: InstallReportText::try_from("mcp-registration-present".to_owned())?,
                    status: match self.verify_status {
                        CheckStatus::Passed => CheckStatus::Passed,
                        CheckStatus::Failed => CheckStatus::Failed,
                    },
                    detail: InstallReportText::try_from(match self.verify_status {
                        // BRAND-INVARIANT: the empty detail is immediately validated by InstallReportText.
                        CheckStatus::Passed => String::new(),
                        CheckStatus::Failed => "missing registration".to_owned(),
                    })?,
                }],
            })
        }
    }

    fn passing_adapter(key: HarnessId, plan_behavior: FixturePlan) -> FixtureAdapter {
        FixtureAdapter {
            key,
            plan_behavior,
            apply_behavior: FixtureApply::Pass,
            verify_status: CheckStatus::Passed,
        }
    }

    #[test]
    fn install_plans_and_applies_a_fresh_registration() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        assert_eq!(outcomes.len(), 1);
        let (key, plan, applied) = &outcomes[0];
        assert_eq!(key.as_str(), "claude");
        assert_eq!(plan.planned_changes.len(), 1);
        let applied = applied
            .as_ref()
            .ok_or("expected an apply result for a non-dry-run")?;
        assert!(applied.applied.iter().all(|change| matches!(
            change.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        Ok(())
    }

    #[test]
    fn install_dry_run_never_calls_apply_and_writes_nothing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter(
            HarnessId::try_from("codex".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let mut context =
            InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        context.dry_run = enforcer_domain::install_types::DryRun::Enabled;
        let request = InstallCommand {
            context,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert_eq!(plan.planned_changes.len(), 1);
        assert!(
            applied.is_none(),
            "dry-run must never produce an ApplyResultDto"
        );
        Ok(())
    }

    #[test]
    fn idempotent_reinstall_yields_a_noop_plan() -> Result<(), Box<dyn std::error::Error>> {
        // change_pending=false models an adapter whose plan(), on a second
        // run against an already-registered harness, has nothing left to
        // do -- the idempotent re-install fixture the workpack requires.
        let adapter = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::NoChange,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert!(plan.planned_changes.is_empty());
        let applied = applied
            .as_ref()
            .ok_or("expected an apply result for a non-dry-run")?;
        assert!(applied.applied.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_existing_config_is_a_detected_plan_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = FixtureAdapter {
            key: HarnessId::try_from("gemini".to_owned())?,
            plan_behavior: FixturePlan::Fail,
            apply_behavior: FixtureApply::Pass,
            verify_status: CheckStatus::Failed,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn failing_apply_surfaces_the_error_and_stops() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = FixtureAdapter {
            key: HarnessId::try_from("cursor".to_owned())?,
            plan_behavior: FixturePlan::ChangePending,
            apply_behavior: FixtureApply::Fail,
            verify_status: CheckStatus::Failed,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::Io { .. })));
        Ok(())
    }

    #[test]
    fn only_harnesses_filter_narrows_the_adapter_set() -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let codex = passing_adapter(
            HarnessId::try_from("codex".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &[HarnessId::try_from("codex".to_owned())?])?;
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].harness_key().as_str(), "codex");
        Ok(())
    }

    #[test]
    fn empty_only_harnesses_selects_every_adapter() -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let codex = passing_adapter(
            HarnessId::try_from("codex".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &[])?;
        assert_eq!(selected.len(), 2);
        Ok(())
    }

    #[test]
    fn unknown_adapter_id_is_a_typed_error_not_a_silent_skip(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude];
        let result = select_adapters(
            &adapters,
            &[HarnessId::try_from("not-a-real-harness".to_owned())?],
        );
        assert!(matches!(
            result,
            Err(InstallError::UnknownAdapter { ref id, .. }) if id.as_str() == "not-a-real-harness"
        ));
        Ok(())
    }

    #[test]
    fn install_rejects_an_unknown_only_harnesses_entry() -> Result<(), Box<dyn std::error::Error>> {
        let claude = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude];
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![HarnessId::try_from("ghost-harness".to_owned())?],
        };
        let result = install(&adapters, &request);
        assert!(matches!(result, Err(InstallError::UnknownAdapter { .. })));
        Ok(())
    }

    #[test]
    fn parallel_execution_doctrine_cites_the_lessons_ledger_as_its_source() {
        assert_eq!(
            super::PARALLEL_EXECUTION_DOCTRINE
                .match_indices("refs/orchestration-lessons.md")
                .count(),
            1
        );
        assert_eq!(
            super::PARALLEL_EXECUTION_DOCTRINE
                .match_indices("coordination lane new")
                .count(),
            1
        );
        assert_eq!(
            super::PARALLEL_EXECUTION_DOCTRINE
                .match_indices("Cargo.lock")
                .count(),
            1
        );
    }

    #[test]
    fn uninstall_follows_the_same_plan_apply_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let request = UninstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let outcomes = uninstall(&adapters, &request)?;
        assert_eq!(outcomes.len(), 1);
        Ok(())
    }

    #[test]
    fn doctor_aggregates_verify_across_adapters() -> Result<(), Box<dyn std::error::Error>> {
        let healthy = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::NoChange,
        );
        let broken = FixtureAdapter {
            key: HarnessId::try_from("codex".to_owned())?,
            plan_behavior: FixturePlan::NoChange,
            apply_behavior: FixtureApply::Pass,
            verify_status: CheckStatus::Failed,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&healthy, &broken];
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let outcomes = doctor(&adapters, &ctx, &DoctorCommand::default())?;
        assert_eq!(outcomes.len(), 2);
        let (_, healthy_report) = &outcomes[0];
        assert!(healthy_report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        let (_, broken_report) = &outcomes[1];
        assert!(!broken_report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        Ok(())
    }

    struct FakeDownloader {
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        will_fail: bool,
    }

    impl Downloader for FakeDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            version: &enforcer_domain::install_types::ReleaseVersion,
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
                version: version.clone(),
                install_path: InstallBinaryPath::try_from(install_path.to_path_buf())?,
            })
        }
    }

    #[test]
    fn update_dry_run_never_calls_the_downloader() -> Result<(), Box<dyn std::error::Error>> {
        let downloader = FakeDownloader { will_fail: true };
        let request = UpdateCommand {
            dry_run: enforcer_domain::install_types::DryRun::Enabled,
            output: enforcer_domain::install_types::InstallOutputMode::Human,
        };
        let result = update(
            &downloader,
            &std::env::temp_dir().join("enforcer"),
            &enforcer_domain::install_types::ReleaseVersion::try_from("0.2.0".to_owned())
                .map_err(|error| format!("release version fixture is invalid: {error:?}"))?,
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
        let install_path = std::env::temp_dir().join("enforcer");
        let request = UpdateCommand {
            dry_run: enforcer_domain::install_types::DryRun::Disabled,
            output: enforcer_domain::install_types::InstallOutputMode::Human,
        };
        let version = enforcer_domain::install_types::ReleaseVersion::try_from("0.2.0".to_owned())
            .map_err(|error| format!("release version fixture is invalid: {error:?}"))?;
        let resolved = update(&downloader, &install_path, &version, &request)?
            .ok_or("expected a resolved binary for a non-dry-run update")?;
        assert_eq!(resolved.install_path.as_path(), install_path);
        assert_eq!(resolved.version.as_str(), "0.2.0");
        Ok(())
    }

    #[test]
    fn update_surfaces_a_release_channel_failure() -> Result<(), Box<dyn std::error::Error>> {
        let downloader = FakeDownloader { will_fail: true };
        let request = UpdateCommand {
            dry_run: enforcer_domain::install_types::DryRun::Disabled,
            output: enforcer_domain::install_types::InstallOutputMode::Human,
        };
        let version = enforcer_domain::install_types::ReleaseVersion::try_from("0.2.0".to_owned())
            .map_err(|error| format!("release version fixture is invalid: {error:?}"))?;
        let result = update(
            &downloader,
            &std::env::temp_dir().join("enforcer"),
            &version,
            &request,
        );
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
        Ok(())
    }

    /// Root of the checked-in skill-asset fixtures
    /// (`tests/fixtures/install_core/**`, workpack c01 acceptance row).
    #[derive(Clone, Copy)]
    enum SkillAssetFixture {
        Pass,
        MissingAsset,
        BadPluginPath,
    }

    fn fixture_root(
        fixture: SkillAssetFixture,
    ) -> Result<
        enforcer_domain::install_types::InstallRootPath,
        enforcer_domain::boundary::decode_error::DecodeError,
    > {
        let name = match fixture {
            SkillAssetFixture::Pass => "skill_asset_pass",
            SkillAssetFixture::MissingAsset => "skill_asset_fail_missing_asset",
            SkillAssetFixture::BadPluginPath => "skill_asset_fail_bad_plugin_path",
        };
        enforcer_domain::install_types::InstallRootPath::try_from(
            // BRAND-INVARIANT: the assembled fixture path is validated by InstallRootPath before use.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/install_core")
                .join(name),
        )
    }

    fn ocentra_enforcer_manifest() -> Result<SkillAssetManifest, Box<dyn std::error::Error>> {
        let skill_path = SkillAssetPath::from(PathBuf::from("skills/ocentra-enforcer/SKILL.md"));
        let plugin_manifest_path = SkillAssetPath::from(PathBuf::from(".codex-plugin/plugin.json"));
        Ok(SkillAssetManifest {
            assets: vec![SkillAsset {
                name: InstallReportText::try_from("ocentra-enforcer".to_owned())?,
                path: skill_path,
            }],
            plugin_contracts: vec![PluginPublishContract {
                manifest_path: plugin_manifest_path,
                field: InstallReportText::try_from("skills".to_owned())?,
                expected_value: InstallReportText::try_from("./skills/".to_owned())?,
            }],
        })
    }

    #[test]
    fn skill_asset_pass_fixture_resolves_clean() -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(
            &ocentra_enforcer_manifest()?,
            fixture_root(SkillAssetFixture::Pass)?.as_path(),
        )?;
        assert!(
            report.checks.iter().all(|check| matches!(
                check.status,
                enforcer_domain::install_types::CheckStatus::Passed
            )),
            "expected every check to pass, got {report:?}"
        );
        assert_eq!(report.checks.len(), 2);
        Ok(())
    }

    #[test]
    fn skill_asset_fail_fixture_missing_asset_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(
            &ocentra_enforcer_manifest()?,
            fixture_root(SkillAssetFixture::MissingAsset)?.as_path(),
        )?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        let asset_check = report
            .checks
            .iter()
            .find(|c| c.name.as_str().starts_with("skill-asset-exists"))
            .ok_or("expected a skill-asset-exists check")?;
        assert!(!matches!(
            asset_check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        ));
        assert!(asset_check.detail.as_str().contains("missing skill asset"));
        Ok(())
    }

    #[test]
    fn skill_asset_fail_fixture_bad_plugin_path_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = run_skill_asset_checks(
            &ocentra_enforcer_manifest()?,
            fixture_root(SkillAssetFixture::BadPluginPath)?.as_path(),
        )?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        let plugin_check = report
            .checks
            .iter()
            .find(|c| c.name.as_str().starts_with("plugin-skills-path"))
            .ok_or("expected a plugin-skills-path check")?;
        assert!(!matches!(
            plugin_check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        ));
        assert!(plugin_check.detail.as_str().contains("expected"));
        Ok(())
    }

    #[test]
    fn missing_plugin_manifest_is_a_failed_check_not_a_panic(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // BRAND-INVARIANT: the deliberately missing fixture path is immediately wrapped by SkillAssetPath.
        let missing_manifest_path =
            SkillAssetPath::from(PathBuf::from("does/not/exist/plugin.json"));
        let manifest = SkillAssetManifest {
            assets: vec![],
            plugin_contracts: vec![PluginPublishContract {
                manifest_path: missing_manifest_path,
                field: InstallReportText::try_from("skills".to_owned())?,
                expected_value: InstallReportText::try_from("./skills/".to_owned())?,
            }],
        };
        let report =
            run_skill_asset_checks(&manifest, fixture_root(SkillAssetFixture::Pass)?.as_path())?;
        assert!(!report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
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

        let before = crate::backup::snapshot_for_test(&target)?;

        struct WriterAdapter {
            // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
            target: PathBuf,
            key: HarnessId,
        }
        impl HarnessAdapter for WriterAdapter {
            fn harness_key(&self) -> HarnessId {
                self.key.clone()
            }
            fn plan(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
                let path: RepoRoot = self.target.display().to_string().try_into().map_err(
                    |e: enforcer_domain::boundary::decode_error::DecodeError| {
                        InstallError::MalformedConfig {
                            path: self.target.display().to_string(),
                            reason: e.to_string(),
                        }
                    },
                )?;
                Ok(InstallReport {
                    planned_changes: vec![PlannedInstallChange {
                        harness: self.key.clone(),
                        kind: ArtifactKind::McpRegistration,
                        path,
                        description: InstallReportText::try_from(
                            "would add enforcer to mcpServers".to_owned(),
                        )?,
                        disposition: ChangeDisposition::Update,
                    }],
                    warnings: vec![],
                })
            }
            fn apply(&self, _report: &InstallReport) -> InstallResult<ApplyResult> {
                // If a dry-run ever reached `apply`, this would actually
                // write -- the test asserts `applied` stays `None` so this
                // branch is provably unreached for `--dry-run`.
                std::fs::write(&self.target, "{\"mcpServers\":{\"enforcer\":{}}}").map_err(
                    |e| InstallError::Io {
                        path: self.target.display().to_string(),
                        reason: e.to_string(),
                    },
                )?;
                Ok(ApplyResult::default())
            }
            fn verify(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
                Ok(InstallVerifyReport::default())
            }
        }

        let adapter = WriterAdapter {
            target: target.clone(),
            key: HarnessId::try_from("claude".to_owned())?,
        };
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let mut context =
            InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        context.dry_run = enforcer_domain::install_types::DryRun::Enabled;
        let request = InstallCommand {
            context,
            only_harnesses: vec![],
        };
        let outcomes = install(&adapters, &request)?;
        let (_, plan, applied) = &outcomes[0];
        assert_eq!(plan.planned_changes.len(), 1);
        assert!(applied.is_none());

        let after = crate::backup::snapshot_for_test(&target)?;
        assert_eq!(
            before, after,
            "dry-run must leave the filesystem byte-identical"
        );
        Ok(())
    }

    /// The dry-run report must be byte-identical in SHAPE to the applied
    /// report minus the `applied` flag -- i.e. the same `InstallReportDto`
    /// serde shape is produced whether or not `--dry-run` was set, and only
    /// the presence of an [`ApplyResultDto`] differs (workpack c01 acceptance
    /// row: serde round-trip asserted).
    #[test]
    fn dry_run_report_shape_matches_applied_report_shape() -> Result<(), Box<dyn std::error::Error>>
    {
        let real_adapter = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let dry_adapter = passing_adapter(
            HarnessId::try_from("claude".to_owned())?,
            FixturePlan::ChangePending,
        );
        let adapters_real: Vec<&dyn HarnessAdapter> = vec![&real_adapter];
        let adapters_dry: Vec<&dyn HarnessAdapter> = vec![&dry_adapter];

        let real_request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(
                std::env::temp_dir().join("enforcer"),
            )?,
            only_harnesses: vec![],
        };
        let mut dry_context =
            InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        dry_context.dry_run = enforcer_domain::install_types::DryRun::Enabled;
        let dry_request = InstallCommand {
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
        let real_wire =
            serde_json::to_string(&crate::report::InstallReportDto::from(real_plan.clone()))?;
        let dry_wire =
            serde_json::to_string(&crate::report::InstallReportDto::from(dry_plan.clone()))?;
        assert_eq!(real_wire, dry_wire);

        // The ONLY difference is the presence of an apply result.
        let _real_applied = real_applied.as_ref().ok_or(std::io::Error::other(
            "real install must produce an apply result",
        ))?;
        assert!(dry_applied.is_none());
        Ok(())
    }
}
