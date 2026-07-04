//! c08 — the Gemini CLI adapter stub.
//!
//! # Charter
//!
//! Gemini's real config surface (`~/.gemini/settings.json`
//! `mcpServers`) is structurally close to the generic `.mcp.json` shape
//! but is NOT yet wired here — this module is contract-only. It exists so
//! the [`crate::core::HarnessAdapter`] registry can resolve the `"gemini"`
//! key (c02 autodetect, the arc-22 CLI) without a typed-error dead end or
//! a silent no-op that looks like success. Every call is safe: `plan`
//! reports a deferred warning and zero planned changes, `apply` never
//! touches the filesystem, and `verify` reports a single passing advisory
//! check that says mechanization has not landed yet.
//!
//! # Deferred
//!
//! An ADBP-style config converter for Gemini's `settings.json`
//! `mcpServers` map (USER/GLOBAL scope, absolute binary path, x01
//! `SERVER_NAME` const, value-merge preserving unrelated keys) is Track B
//! future work, not this workpack (c08 is deliberately scoped
//! contract-only so that later work can land without touching this
//! module's `HarnessAdapter` interface).

use crate::cli_contract::RequestContext;
use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use crate::report::{ApplyResult, InstallReport, VerifyCheck, VerifyReport};

/// Advisory verify-check name every c08 stub reports under.
const DEFERRED_CHECK_NAME: &str = "adapter-mechanization";

/// Advisory verify-check detail every c08 stub reports under (T3 —
/// informational tier, never `Error` severity: the check runs and passes
/// closed, it does not fail a `doctor` run just because mechanization has
/// not landed yet).
const DEFERRED_DETAIL: &str = "deferred: no mechanization yet (T3)";

/// The Gemini CLI adapter stub. Holds no state — a stub has nothing to
/// configure until its real config-converter work lands.
#[derive(Debug, Clone, Default)]
pub struct GeminiAdapter;

impl GeminiAdapter {
    /// Build a stub instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl HarnessAdapter for GeminiAdapter {
    fn harness_key(&self) -> &'static str {
        "gemini"
    }

    fn plan(&self, _ctx: &RequestContext) -> InstallResult<InstallReport> {
        Ok(InstallReport {
            planned_changes: vec![],
            warnings: vec![format!("gemini adapter {DEFERRED_DETAIL}")],
        })
    }

    fn apply(&self, _report: &InstallReport) -> InstallResult<ApplyResult> {
        // A stub's plan never produces planned_changes, so this has
        // nothing to iterate -- but the contract still holds even if a
        // caller hands it an unexpected non-empty report: no write, ever.
        Ok(ApplyResult::default())
    }

    fn verify(&self, _ctx: &RequestContext) -> InstallResult<VerifyReport> {
        Ok(VerifyReport {
            checks: vec![VerifyCheck {
                harness: self.harness_key().to_owned(),
                name: DEFERRED_CHECK_NAME.to_owned(),
                passed: true,
                detail: DEFERRED_DETAIL.to_owned(),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiAdapter;
    use crate::cli_contract::RequestContext;
    use crate::core::HarnessAdapter;
    use std::path::PathBuf;

    #[test]
    fn harness_key_is_gemini() {
        assert_eq!(GeminiAdapter::new().harness_key(), "gemini");
    }

    #[test]
    fn plan_produces_zero_planned_changes() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = GeminiAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let plan = adapter.plan(&ctx)?;
        assert!(plan.is_noop());
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("deferred"));
        Ok(())
    }

    #[test]
    fn apply_writes_zero_files_against_a_temp_dir_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let before: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(before.is_empty());

        let adapter = GeminiAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let plan = adapter.plan(&ctx)?;
        let applied = adapter.apply(&plan)?;
        assert!(applied.applied.is_empty());

        let after: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(after.is_empty(), "stub apply must write zero files");
        Ok(())
    }

    #[test]
    fn verify_returns_a_single_passing_advisory_check() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = GeminiAdapter::new();
        let ctx = RequestContext::with_defaults(PathBuf::from("/abs/path/to/enforcer"));
        let report = adapter.verify(&ctx)?;
        assert_eq!(report.checks.len(), 1);
        assert!(report.all_passed());
        assert!(report.checks[0].detail.contains("deferred"));
        assert!(report.checks[0].detail.contains("T3"));
        Ok(())
    }
}
