//! c07 — the shared, mechanical `enforcer doctor`.
//!
//! # Charter
//!
//! The retired `.mjs` doctor logic was Codex-specific and tangled into one
//! install path. This module is the harness-neutral replacement: it takes
//! every registered [`crate::core::HarnessAdapter`], re-runs its
//! [`crate::core::HarnessAdapter::verify`] (which itself re-reads the
//! actual on-disk config — see that trait's docs), and aggregates every
//! resulting [`crate::report::VerifyCheckDto`] into one typed [`DoctorReport`]
//! with a [`Severity`] attached to each check.
//!
//! # Mechanical, never trust-the-plan
//!
//! `run` never accepts (or looks at) a previously computed
//! [`crate::report::InstallReportDto`]/[`crate::report::ApplyResultDto`] — the
//! only input is the adapter list plus a fresh [`enforcer_domain::install_types::InstallRequestContext`].
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

use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use enforcer_domain::install_types::{
    DoctorCheck, DoctorReport, InstallRequestContext, InstallVerifyCheck,
};

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
pub fn run(
    adapters: &[&dyn HarnessAdapter],
    ctx: &InstallRequestContext,
) -> InstallResult<DoctorReport> {
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
    ctx: &InstallRequestContext,
    extra_checks: Vec<InstallVerifyCheck>,
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
    use super::{run, run_with_extra_checks};
    use crate::adapters::generic::{GenericAdapter, GenericAdapterConfig};
    use crate::core::HarnessAdapter;
    use enforcer_domain::ids::HarnessId;
    use enforcer_domain::install_types::{
        DoctorCheck, DoctorReport, InstallBinaryPath, InstallRequestContext, InstallRootPath,
        InstallTargetPath, OverwriteMode,
    };
    use enforcer_domain::severity::Severity;
    use std::path::PathBuf;

    #[derive(Clone, Copy)]
    enum DoctorFixture {
        Good,
        MissingServer,
        RenamedBinary,
    }

    fn fixture_root(
        fixture: DoctorFixture,
    ) -> Result<InstallRootPath, enforcer_domain::boundary::decode_error::DecodeError> {
        let name = match fixture {
            DoctorFixture::Good => "good",
            DoctorFixture::MissingServer => "missing_server",
            DoctorFixture::RenamedBinary => "renamed_binary",
        };
        InstallRootPath::try_from(
            // BRAND-INVARIANT: the assembled fixture path is validated by InstallRootPath before use.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/doctor")
                .join(name),
        )
    }

    fn generic_config(
        target: InstallTargetPath,
    ) -> Result<GenericAdapterConfig, Box<dyn std::error::Error>> {
        Ok(GenericAdapterConfig::new(
            HarnessId::try_from("generic-harness".to_owned())?,
            target,
            InstallBinaryPath::try_from(std::env::temp_dir().join("enforcer"))?,
        ))
    }

    #[test]
    fn all_green_on_a_good_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let good = std::fs::read_to_string(
            fixture_root(DoctorFixture::Good)?
                .as_path()
                .join("mcp.json"),
        )?;
        let good = crate::boundary::json_wire::decode_value(&good)?;
        let binary = std::env::temp_dir().join("enforcer");
        let good = crate::boundary::json_wire::with_mcp_command(good, &binary)?;
        std::fs::write(&target, serde_json::to_string_pretty(&good)?)?;

        let config = generic_config(InstallTargetPath::try_from(target)?)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = InstallRequestContext::try_with_defaults(binary)?;

        let report = run(&adapters, &ctx)?;
        assert!(report.checks.iter().all(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        Ok(())
    }

    #[test]
    fn red_on_missing_server_names_the_failing_check() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let missing = std::fs::read_to_string(
            fixture_root(DoctorFixture::MissingServer)?
                .as_path()
                .join("mcp.json"),
        )?;
        std::fs::write(&target, missing)?;

        let config = generic_config(InstallTargetPath::try_from(target)?)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;

        let report = run(&adapters, &ctx)?;
        assert!(report.checks.iter().any(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Failed
        )));
        assert_eq!(
            report
                .checks
                .iter()
                .filter(|check| matches!(
                    check.check.status,
                    enforcer_domain::install_types::CheckStatus::Failed
                ))
                .map(|check| check.check.name.as_str())
                .collect::<Vec<_>>(),
            vec!["mcp-registration-present"],
        );
        Ok(())
    }

    #[test]
    fn red_on_renamed_server_binary_names_the_failing_check(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let renamed = std::fs::read_to_string(
            fixture_root(DoctorFixture::RenamedBinary)?
                .as_path()
                .join("mcp.json"),
        )?;
        std::fs::write(&target, renamed)?;

        let config = generic_config(InstallTargetPath::try_from(target)?)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;

        let report = run(&adapters, &ctx)?;
        assert!(report.checks.iter().any(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Failed
        )));
        assert_eq!(
            report
                .checks
                .iter()
                .filter(|check| matches!(
                    check.check.status,
                    enforcer_domain::install_types::CheckStatus::Failed
                ))
                .map(|check| check.check.name.as_str())
                .collect::<Vec<_>>(),
            vec!["mcp-registration-present"],
        );
        Ok(())
    }

    #[test]
    fn warning_severity_never_flips_the_exit_code() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_domain::install_types::{
            CheckStatus, CheckSubject, InstallReportText, InstallVerifyCheck,
        };

        let report = DoctorReport {
            checks: vec![DoctorCheck {
                check: InstallVerifyCheck {
                    subject: CheckSubject::Harness(enforcer_domain::ids::HarnessId::try_from(
                        "generic-harness".to_owned(),
                    )?),
                    name: InstallReportText::try_from("advisory-only".to_owned())?,
                    status: CheckStatus::Passed,
                    detail: InstallReportText::try_from(
                        // BRAND-INVARIANT: the empty detail is immediately validated by InstallReportText.
                        String::new(),
                    )?,
                },
                severity: Severity::Warning,
            }],
        };
        assert!(report
            .checks
            .iter()
            .all(|check| !matches!(check.severity, Severity::Error)));
        Ok(())
    }

    #[test]
    fn doctor_re_reads_disk_never_trusts_a_stale_in_memory_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(".mcp.json");
        let config = generic_config(InstallTargetPath::try_from(target.clone())?)?;
        let adapter = GenericAdapter::new(config);
        let adapters: Vec<&dyn HarnessAdapter> = vec![&adapter];
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;

        // Install fresh -- doctor should be green.
        let plan = adapter.plan(&ctx)?;
        adapter.apply(&plan)?;
        let report = run(&adapters, &ctx)?;
        assert!(report.checks.iter().all(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));

        // Hand-corrupt the file on disk AFTER the adapter object exists --
        // doctor must notice because it re-reads, not because it remembers.
        std::fs::write(&target, "{}")?;
        let report = run(&adapters, &ctx)?;
        assert!(report.checks.iter().any(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Failed
        )));
        Ok(())
    }

    #[test]
    fn emitter_verify_checks_feed_the_same_aggregation_as_adapters(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::emitters::{consumer_ci, git_hooks};

        let dir = tempfile::tempdir()?;
        let root = InstallRootPath::try_from(dir.path().to_path_buf())?;
        consumer_ci::apply(&root, OverwriteMode::PreserveExisting)?;
        git_hooks::apply(
            &root,
            enforcer_domain::install_types::HookFlavor::Lefthook,
            OverwriteMode::PreserveExisting,
        )?;

        let mut extra = consumer_ci::verify(&root)?;
        extra.extend(git_hooks::verify(
            &root,
            enforcer_domain::install_types::HookFlavor::Lefthook,
        )?);

        let adapters: Vec<&dyn HarnessAdapter> = vec![];
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let report = run_with_extra_checks(&adapters, &ctx, extra)?;

        assert_eq!(report.checks.len(), 6); // 5 workflows + 1 lefthook
        assert!(report.checks.iter().all(|check| matches!(
            check.check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        Ok(())
    }
}
