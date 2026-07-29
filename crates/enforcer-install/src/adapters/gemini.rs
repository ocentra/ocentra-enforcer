//! Gemini CLI user-scope MCP registration.
//!
//! Gemini CLI reads user settings from `~/.gemini/settings.json` and
//! registers stdio servers in its top-level `mcpServers` object. This
//! adapter delegates the value-preserving JSON merge to
//! [`crate::adapters::generic::GenericAdapter`] while fixing the native
//! path and typed harness identity.

//! BOUNDARY-INVARIANT: adapter configuration is normalized before install decisions.
//!
use std::path::PathBuf;

use crate::adapters::generic::{GenericAdapter, GenericAdapterConfig};
use crate::core::HarnessAdapter;
use crate::error::InstallResult;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::BuiltInHarness;
use enforcer_domain::install_types::{
    ApplyResult, InstallBinaryPath, InstallReport, InstallRequestContext, InstallRootPath,
    InstallTargetPath, InstallVerifyReport,
};

/// Gemini CLI adapter rooted at the user's home directory.
#[derive(Debug, Clone)]
pub struct GeminiAdapter {
    home: InstallRootPath,
    inner: GenericAdapter,
}

impl GeminiAdapter {
    /// Build a user-scope Gemini adapter.
    pub fn try_new(home: PathBuf, binary_path: PathBuf) -> Result<Self, DecodeError> {
        let home = InstallRootPath::try_from(home)?;
        let target =
            InstallTargetPath::try_from(home.as_path().join(".gemini").join("settings.json"))?;
        let inner = GenericAdapter::new(GenericAdapterConfig::new(
            BuiltInHarness::Gemini.id(),
            target,
            InstallBinaryPath::try_from(binary_path)?,
        ));
        Ok(Self { home, inner })
    }

    /// Native Gemini CLI user settings path.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.home.as_path().join(".gemini").join("settings.json")
    }
}

impl HarnessAdapter for GeminiAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        BuiltInHarness::Gemini.id()
    }

    fn plan(&self, ctx: &InstallRequestContext) -> InstallResult<InstallReport> {
        self.inner.plan(ctx)
    }

    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        self.inner.apply(report)
    }

    fn verify(&self, ctx: &InstallRequestContext) -> InstallResult<InstallVerifyReport> {
        self.inner.verify(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiAdapter;
    use crate::core::HarnessAdapter;
    use enforcer_domain::install_types::{CheckStatus, InstallRequestContext};

    #[test]
    fn native_settings_round_trip_is_idempotent_and_preserves_unrelated_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = GeminiAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        std::fs::create_dir_all(adapter.config_path().parent().ok_or("missing parent")?)?;
        std::fs::write(
            adapter.config_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "theme": "ANSI",
                "mcpServers": {
                    "other": { "command": "/abs/other" }
                }
            }))?,
        )?;
        let ctx = InstallRequestContext::try_with_defaults(binary.clone())?;

        let plan = adapter.plan(&ctx)?;
        assert_eq!(plan.planned_changes.len(), 1);
        adapter.apply(&plan)?;
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(adapter.config_path())?)?;
        assert_eq!(written["theme"], "ANSI");
        assert_eq!(written["mcpServers"]["other"]["command"], "/abs/other");
        assert_eq!(
            written["mcpServers"]["enforcer"]["command"],
            binary.display().to_string()
        );
        assert!(adapter
            .verify(&ctx)?
            .checks
            .iter()
            .all(|check| matches!(check.status, CheckStatus::Passed)));
        assert!(adapter.plan(&ctx)?.planned_changes.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_settings_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("enforcer");
        let adapter = GeminiAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        std::fs::create_dir_all(adapter.config_path().parent().ok_or("missing parent")?)?;
        std::fs::write(adapter.config_path(), "{ invalid json")?;
        let ctx = InstallRequestContext::try_with_defaults(binary)?;
        assert!(matches!(
            adapter.plan(&ctx),
            Err(crate::error::InstallError::MalformedConfig { .. })
        ));
        Ok(())
    }
}
