//! c07 — the shared, mechanical `enforcer doctor`.
//!
//! # Charter
//!
//! The retired `.mjs` doctor logic was Codex-specific and tangled into one
//! install path. This module is the harness-neutral replacement: it takes
//! every registered [`crate::core::HarnessAdapter`], re-runs its
//! [`crate::core::HarnessAdapter::verify`] (which itself re-reads the
//! actual on-disk config — see that trait's docs), and aggregates every
//! resulting [`crate::report::VerifyCheck`] into one typed [`DoctorReport`]
//! with a [`Severity`] attached to each check.
//!
//! # Mechanical, never trust-the-plan
//!
//! `run` never accepts (or looks at) a previously computed
//! [`crate::report::InstallReport`]/[`crate::report::ApplyResult`] — the
//! only input is the adapter list plus a fresh [`crate::cli_contract::RequestContext`].
//! Every check an adapter's `verify` performs re-reads its target file (and,
//! where relevant, resolves the registered binary path) from disk at call
//! time, so a stale in-memory plan can never make doctor report green for a
//! config that has since drifted or been hand-edited.
//!
//! # Fail-closed severity
//!
//! Every aggregated check is classified [`Severity::Error`] (a failing
//! check) or [`Severity::Info`] (a passing check) — this module never
//! invents a soft "warning-only" reading for a failed mechanical check, so
//! [`DoctorReport::exit_is_nonzero`] is exactly "any check reports
//! `passed: false`". `enforcer-cli` (arc-22) reads
//! [`DoctorReport::exit_is_nonzero`] to decide the process exit code.

use crate::cli_contract::RequestContext;
use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use crate::report::VerifyCheck;
use enforcer_domain::severity::Severity;

/// One aggregated doctor check: the underlying [`VerifyCheck`] plus the
/// [`Severity`] doctor classifies it at (fail-closed: a failing check is
/// always [`Severity::Error`], never downgraded to [`Severity::Warning`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    /// The underlying per-adapter check.
    pub check: VerifyCheck,
    /// [`Severity::Error`] when `check.passed` is `false`, [`Severity::Info`]
    /// when it is `true`.
    pub severity: Severity,
}

impl DoctorCheck {
    fn from_verify_check(check: VerifyCheck) -> Self {
        let severity = if check.passed {
            Severity::Info
        } else {
            Severity::Error
        };
        Self { check, severity }
    }
}

/// The full aggregated doctor report across every adapter passed to
/// [`run`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoctorReport {
    /// Every check, across every adapter, in adapter-registration order.
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    /// True when every check passed (no [`Severity::Error`] present).
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.severity != Severity::Error)
    }

    /// The doctor exit-code contract (arc-22 CLI wiring reads this):
    /// fail-closed, so any [`Severity::Error`] check drives a non-zero
    /// process exit; [`Severity::Warning`]/[`Severity::Info`] checks never
    /// do. Today doctor only ever emits [`Severity::Error`] or
    /// [`Severity::Info`] (see [`DoctorCheck::from_verify_check`]), but the
    /// contract is phrased over [`Severity::Warning`] explicitly so a
    /// future adapter that reports a genuine non-blocking warning (a
    /// distinct check outcome from pass/fail) does not accidentally flip
    /// the exit code.
    #[must_use]
    pub fn exit_is_nonzero(&self) -> bool {
        self.checks.iter().any(|c| c.severity == Severity::Error)
    }

    /// Every failing check's name, for a `doctor` CLI render that must
    /// name the specific failing check (workpack acceptance: "doctor
    /// returns ... red (naming the failing check)").
    #[must_use]
    pub fn failing_check_names(&self) -> Vec<&str> {
        self.checks
            .iter()
            .filter(|c| c.severity == Severity::Error)
            .map(|c| c.check.name.as_str())
            .collect()
    }
}

/// Run every adapter's `verify(ctx)` and aggregate the results into one
/// [`DoctorReport`]. This is the shared core `enforcer doctor` (and the
/// doctor-shaped portion of `install`/`uninstall` post-apply checks) calls
/// into — never a single-harness-specific doctor path.
///
/// # Errors
/// Returns [`crate::error::InstallError`] if any adapter's `verify` call
/// itself cannot run (distinct from a check that runs and reports
/// `passed: false` — that is captured as a [`Severity::Error`]
/// [`DoctorCheck`], not a `Result::Err`).
pub fn run(adapters: &[&dyn HarnessAdapter], ctx: &RequestContext) -> InstallResult<DoctorReport> {
    run_with_extra_checks(adapters, ctx, Vec::new())
}

/// Same aggregation as [`run`], plus `extra_checks` folded in unchanged.
/// This is the seam [`crate::emitters::consumer_ci::verify`] and
/// [`crate::emitters::git_hooks::verify`] use to feed their own mechanical,
/// disk-re-reading checks into the same aggregated [`DoctorReport`] every
/// [`HarnessAdapter`] check lands in — the emitters are plain functions,
/// not [`HarnessAdapter`] implementations (they write consumer-repo-side
/// artifacts, not a harness's own home-directory config), so they cannot
/// be passed in the `adapters` slice itself.
///
/// # Errors
/// See [`run`] — identical contract for the `adapters` portion;
/// `extra_checks` is pre-computed by the caller and never itself
/// fallible here.
pub fn run_with_extra_checks(
    adapters: &[&dyn HarnessAdapter],
    ctx: &RequestContext,
    extra_checks: Vec<VerifyCheck>,
) -> InstallResult<DoctorReport> {
    let mut checks: Vec<DoctorCheck> = extra_checks
        .into_iter()
        .map(DoctorCheck::from_verify_check)
        .collect();
    for adapter in adapters {
        let report = adapter.verify(ctx)?;
        checks.extend(
            report
                .checks
                .into_iter()
                .map(DoctorCheck::from_verify_check),
        );
    }
    Ok(DoctorReport { checks })
}

#[cfg(test)]
mod tests {
    use super::{run, run_with_extra_checks, DoctorReport};
    use crate::adapters::generic::{GenericAdapter, GenericAdapterConfig};
    use crate::cli_contract::RequestContext;
    use crate::core::HarnessAdapter;
    use enforcer_domain::severity::Severity;
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/doctor")
            .join(name)
    }

    #[test]
    fn all_green_on_a_good_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let good = std::fs::read_to_string(fixture_root("good").join("mcp.json"))?;
        std::fs::write(&target, good)?;

        let config = GenericAdapterConfig::new("generic-harness", target)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let report = run(&adapters, &ctx)?;
        assert!(report.all_passed());
        assert!(!report.exit_is_nonzero());
        assert!(report.failing_check_names().is_empty());
        Ok(())
    }

    #[test]
    fn red_on_missing_server_names_the_failing_check() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let missing = std::fs::read_to_string(fixture_root("missing_server").join("mcp.json"))?;
        std::fs::write(&target, missing)?;

        let config = GenericAdapterConfig::new("generic-harness", target)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let report = run(&adapters, &ctx)?;
        assert!(!report.all_passed());
        assert!(report.exit_is_nonzero());
        assert_eq!(
            report.failing_check_names(),
            vec!["mcp-registration-present"]
        );
        Ok(())
    }

    #[test]
    fn red_on_renamed_server_binary_names_the_failing_check(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let renamed = std::fs::read_to_string(fixture_root("renamed_binary").join("mcp.json"))?;
        std::fs::write(&target, renamed)?;

        let config = GenericAdapterConfig::new("generic-harness", target)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        let report = run(&adapters, &ctx)?;
        assert!(!report.all_passed());
        assert_eq!(
            report.failing_check_names(),
            vec!["mcp-registration-present"]
        );
        Ok(())
    }

    #[test]
    fn warning_severity_never_flips_the_exit_code() {
        use crate::report::VerifyCheck;

        let report = DoctorReport {
            checks: vec![super::DoctorCheck {
                check: VerifyCheck {
                    harness: "generic-harness".to_owned(),
                    name: "advisory-only".to_owned(),
                    passed: true,
                    detail: String::new(),
                },
                severity: Severity::Warning,
            }],
        };
        assert!(!report.exit_is_nonzero());
    }

    #[test]
    fn doctor_re_reads_disk_never_trusts_a_stale_in_memory_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let config = GenericAdapterConfig::new("generic-harness", target.clone())?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));

        // Install fresh -- doctor should be green.
        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;
        let report = run(&adapters, &ctx)?;
        assert!(report.all_passed());

        // Hand-corrupt the file on disk AFTER the adapter object exists --
        // doctor must notice because it re-reads, not because it remembers.
        std::fs::write(&target, "{}")?;
        let report = run(&adapters, &ctx)?;
        assert!(!report.all_passed());
        Ok(())
    }

    #[test]
    fn emitter_verify_checks_feed_the_same_aggregation_as_adapters(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::emitters::{consumer_ci, git_hooks};

        let dir = tempfile::tempdir()?;
        consumer_ci::apply(dir.path(), false)?;
        git_hooks::apply(dir.path(), git_hooks::HookFlavor::Lefthook, false)?;

        let mut extra = consumer_ci::verify(dir.path());
        extra.extend(git_hooks::verify(
            dir.path(),
            git_hooks::HookFlavor::Lefthook,
        ));

        let adapters: Vec<&dyn HarnessAdapter> = vec![];
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let report = run_with_extra_checks(&adapters, &ctx, extra)?;

        assert_eq!(report.checks.len(), 6); // 5 workflows + 1 lefthook
        assert!(report.all_passed());
        Ok(())
    }
}
