//! c09 — the OpenCode adapter (CLI-only, no MCP surface).
//!
//! # Charter (workpack c09 — BINDING)
//!
//! OpenCode is detected via `enforcer-harness` (arc-18, `OPENCODE_HOME`/
//! `.opencode`, c02 autodetect) as a CLI shim, but — unlike the four
//! JSON-config harnesses this pack also owns (antigravity/windsurf/
//! kilocode/kiro) — it exposes no discovered MCP server registration
//! surface to write into. Per the workpack's own acceptance row
//! ("CLI-only harnesses ... detect the binary ... and, absent an MCP
//! surface, return a `Tier::T3` `deferred` verify `Check` writing zero
//! files"), this adapter is intentionally a documented no-write adapter, NOT
//! a silent no-op that looks like success:
//! - `plan` reports a deferred warning and zero planned changes.
//! - `apply` never touches the filesystem (its `plan` never produces a
//!   planned change to apply in the first place).
//! - `verify` reports a single passing (T3, informational-tier) advisory
//!   check stating there is no MCP surface to verify — always reachable,
//!   never a hard failure just because this harness has nothing to write.

//!
//! BOUNDARY-INVARIANT: adapter configuration is normalized before install decisions.
//! Negative invalid inputs are rejected by adapter configuration tests.
//!
use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use enforcer_domain::install_types::InstallRequestContext;
use enforcer_domain::install_types::{
    ApplyResult, CheckStatus, CheckSubject, InstallReport, InstallReportText, InstallVerifyCheck,
    InstallVerifyReport,
};

/// This adapter's registration key, matching [`crate::report::HarnessKey`].
const HARNESS_KEY: &str = "opencode";

/// Advisory verify-check name this no-write adapter reports under.
const DEFERRED_CHECK_NAME: &str = "adapter-mcp-surface";

/// Advisory verify-check detail this no-write adapter reports under (T3 —
/// informational tier, never `Error` severity: the check runs and passes
/// closed, it does not fail a `doctor` run just because this harness has
/// no MCP config surface to register into).
const DEFERRED_DETAIL: &str = "deferred: no mcp surface (T3)";

/// The OpenCode adapter. Holds no state — a CLI-only harness with no MCP
/// surface has nothing to configure.
#[derive(Debug, Clone, Default)]
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    /// Build an instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl HarnessAdapter for OpenCodeAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        enforcer_domain::ids::BuiltInHarness::OpenCode.id()
    }

    fn plan(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        Ok(InstallReport {
            planned_changes: vec![],
            warnings: vec![InstallReportText::try_from(format!(
                "{HARNESS_KEY} adapter {DEFERRED_DETAIL}"
            ))?],
        })
    }

    fn apply(&self, _report: &InstallReport) -> InstallResult<ApplyResult> {
        // plan() never produces planned_changes, so there is nothing to
        // iterate -- but the contract still holds even if a caller hands
        // this an unexpected non-empty report: no write, ever.
        Ok(ApplyResult::default())
    }

    fn verify(&self, _ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
        Ok(InstallVerifyReport {
            checks: vec![InstallVerifyCheck {
                subject: CheckSubject::Harness(self.harness_key()),
                name: InstallReportText::try_from(DEFERRED_CHECK_NAME.to_owned())?,
                status: CheckStatus::Passed,
                detail: InstallReportText::try_from(DEFERRED_DETAIL.to_owned())?,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OpenCodeAdapter;
    use crate::core::HarnessAdapter;
    use enforcer_domain::install_types::InstallRequestContext;
    #[test]
    fn harness_key_is_opencode() {
        assert_eq!(OpenCodeAdapter::new().harness_key().as_str(), "opencode");
    }

    #[test]
    fn plan_produces_zero_planned_changes_and_a_deferred_warning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = OpenCodeAdapter::new();
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let plan = adapter.plan(&ctx)?;
        assert!(plan.planned_changes.is_empty());
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].as_str().contains("deferred"));
        assert!(plan.warnings[0].as_str().contains("no mcp surface"));
        Ok(())
    }

    #[test]
    fn apply_writes_zero_files_against_a_temp_dir_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let before: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(before.is_empty());

        let adapter = OpenCodeAdapter::new();
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let plan = adapter.plan(&ctx)?;
        let applied = adapter.apply(&plan)?;
        assert!(applied.applied.is_empty());

        let after: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(
            after.is_empty(),
            "no-write apply must leave the directory empty"
        );
        Ok(())
    }

    #[test]
    fn verify_returns_a_single_passing_advisory_check_labelled_t3(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = OpenCodeAdapter::new();
        let ctx = InstallRequestContext::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let report = adapter.verify(&ctx)?;
        assert_eq!(report.checks.len(), 1);
        assert!(report.checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));
        assert!(report.checks[0].detail.as_str().contains("deferred"));
        assert!(report.checks[0].detail.as_str().contains("no mcp surface"));
        assert!(report.checks[0].detail.as_str().contains("T3"));
        Ok(())
    }
}
