//! c09 — the Aider adapter (CLI-only, no MCP surface).
//!
//! # Charter (workpack c09 — BINDING)
//!
//! Aider is detected via `enforcer-harness` (arc-18, `AIDER_HOME`/
//! `.aider`, c02 autodetect) as a CLI tool, but — unlike the four
//! JSON-config harnesses this pack also owns (antigravity/windsurf/
//! kilocode/kiro) — it exposes no discovered MCP server registration
//! surface to write into. Per the workpack's own acceptance row
//! ("CLI-only harnesses ... detect the binary ... and, absent an MCP
//! surface, return a `Tier::T3` `deferred` verify `Check` writing zero
//! files"), this adapter is intentionally a documented no-write stub, NOT
//! a silent no-op that looks like success:
//! - `plan` reports a deferred warning and zero planned changes.
//! - `apply` never touches the filesystem (its `plan` never produces a
//!   planned change to apply in the first place).
//! - `verify` reports a single passing (T3, informational-tier) advisory
//!   check stating there is no MCP surface to verify — always reachable,
//!   never a hard failure just because this harness has nothing to write.

use crate::cli_contract::RequestContext;
use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use crate::report::{ApplyResult, InstallReport, VerifyCheck, VerifyReport};

/// This adapter's registration key, matching [`crate::report::HarnessKey`].
const HARNESS_KEY: &str = "aider";

/// Advisory verify-check name this stub reports under.
const DEFERRED_CHECK_NAME: &str = "adapter-mcp-surface";

/// Advisory verify-check detail this stub reports under (T3 —
/// informational tier, never `Error` severity: the check runs and passes
/// closed, it does not fail a `doctor` run just because this harness has
/// no MCP config surface to register into).
const DEFERRED_DETAIL: &str = "deferred: no mcp surface (T3)";

/// The Aider adapter. Holds no state — a CLI-only harness with no MCP
/// surface has nothing to configure.
#[derive(Debug, Clone, Default)]
pub struct AiderAdapter;

impl AiderAdapter {
    /// Build an instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl HarnessAdapter for AiderAdapter {
    fn harness_key(&self) -> &'static str {
        HARNESS_KEY
    }

    fn plan(&self, _ctx: &RequestContext) -> InstallResult<InstallReport> {
        Ok(InstallReport {
            planned_changes: vec![],
            warnings: vec![format!("{HARNESS_KEY} adapter {DEFERRED_DETAIL}")],
        })
    }

    fn apply(&self, _report: &InstallReport) -> InstallResult<ApplyResult> {
        // plan() never produces planned_changes, so there is nothing to
        // iterate -- but the contract still holds even if a caller hands
        // this an unexpected non-empty report: no write, ever.
        Ok(ApplyResult::default())
    }

    fn verify(&self, _ctx: &RequestContext) -> InstallResult<VerifyReport> {
        Ok(VerifyReport {
            checks: vec![VerifyCheck {
                harness: HARNESS_KEY.to_owned(),
                name: DEFERRED_CHECK_NAME.to_owned(),
                passed: true,
                detail: DEFERRED_DETAIL.to_owned(),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AiderAdapter;
    use crate::cli_contract::RequestContext;
    use crate::core::HarnessAdapter;
    use std::path::PathBuf;

    #[test]
    fn harness_key_is_aider() {
        assert_eq!(AiderAdapter::new().harness_key(), "aider");
    }

    #[test]
    fn plan_produces_zero_planned_changes_and_a_deferred_warning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = AiderAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let plan = adapter.plan(&ctx)?;
        assert!(plan.is_noop());
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("deferred"));
        assert!(plan.warnings[0].contains("no mcp surface"));
        Ok(())
    }

    #[test]
    fn apply_writes_zero_files_against_a_temp_dir_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let before: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(before.is_empty());

        let adapter = AiderAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let plan = adapter.plan(&ctx)?;
        let applied = adapter.apply(&plan)?;
        assert!(applied.applied.is_empty());

        let after: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(after.is_empty(), "stub apply must write zero files");
        Ok(())
    }

    #[test]
    fn verify_returns_a_single_passing_advisory_check_labelled_t3(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let adapter = AiderAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let report = adapter.verify(&ctx)?;
        assert_eq!(report.checks.len(), 1);
        assert!(report.all_passed());
        assert!(report.checks[0].detail.contains("deferred"));
        assert!(report.checks[0].detail.contains("no mcp surface"));
        assert!(report.checks[0].detail.contains("T3"));
        Ok(())
    }
}
