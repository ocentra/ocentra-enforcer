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
use crate::error::InstallResult;
use crate::report::{ApplyResult, InstallReport, VerifyReport};

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
    let selected = select_adapters(adapters, &request.only_harnesses);
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
    let selected = select_adapters(adapters, &request.only_harnesses);
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

/// Narrow `adapters` down to `only`'s keys when non-empty; otherwise
/// return every adapter (the "every detected/known harness" default).
fn select_adapters<'a>(
    adapters: &'a [&'a dyn HarnessAdapter],
    only: &[String],
) -> Vec<&'a dyn HarnessAdapter> {
    if only.is_empty() {
        return adapters.to_vec();
    }
    adapters
        .iter()
        .copied()
        .filter(|adapter| only.iter().any(|key| key == adapter.harness_key()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{doctor, install, select_adapters, uninstall, update, HarnessAdapter};
    use crate::cli_contract::{
        DoctorRequest, InstallRequest, RequestContext, UninstallRequest, UpdateRequest,
    };
    use crate::distribution::{Downloader, ResolvedBinary, TargetPlatform};
    use crate::error::{InstallError, InstallResult};
    use crate::report::{
        AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange, VerifyCheck,
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
                vec![PlannedChange {
                    harness: self.key.to_owned(),
                    kind: ArtifactKind::McpRegistration,
                    path: format!("/fixtures/{}.json", self.key),
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
    fn only_harnesses_filter_narrows_the_adapter_set() {
        let claude = passing_adapter("claude", true);
        let codex = passing_adapter("codex", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &["codex".to_owned()]);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].harness_key(), "codex");
    }

    #[test]
    fn empty_only_harnesses_selects_every_adapter() {
        let claude = passing_adapter("claude", true);
        let codex = passing_adapter("codex", true);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&claude, &codex];
        let selected = select_adapters(&adapters, &[]);
        assert_eq!(selected.len(), 2);
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
}
