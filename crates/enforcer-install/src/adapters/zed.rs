//! Zed user-scope MCP registration.
//!
//! Zed stores custom local servers in the user settings file under
//! `context_servers`. The file location is platform-specific: `%APPDATA%\
//! Zed\settings.json` on Windows, `~/Library/Application Support/Zed\
//! settings.json` on macOS, and `~/.config/zed/settings.json` elsewhere.

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

/// Zed adapter rooted at the platform's user configuration directory.
#[derive(Debug, Clone)]
pub struct ZedAdapter {
    config_root: InstallRootPath,
    inner: GenericAdapter,
}

impl ZedAdapter {
    /// Build a Zed adapter with an explicit user configuration root.
    pub fn try_new(config_root: PathBuf, binary_path: PathBuf) -> Result<Self, DecodeError> {
        let config_root = InstallRootPath::try_from(config_root)?;
        let target = InstallTargetPath::try_from(config_root.as_path().join("settings.json"))?;
        let inner = GenericAdapter::new(GenericAdapterConfig::new_with_server_map(
            BuiltInHarness::Zed.id(),
            target,
            InstallBinaryPath::try_from(binary_path)?,
            "context_servers",
        ));
        Ok(Self { config_root, inner })
    }

    /// Native Zed user settings path beneath the resolved config root.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.config_root.as_path().join("settings.json")
    }
}

impl HarnessAdapter for ZedAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        BuiltInHarness::Zed.id()
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
    use super::ZedAdapter;
    use crate::core::HarnessAdapter;
    use enforcer_domain::install_types::{CheckStatus, InstallRequestContext};

    #[test]
    fn native_settings_round_trip_uses_context_servers_and_preserves_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config_root = tempfile::tempdir()?;
        let binary = config_root.path().join("bin").join("enforcer");
        let adapter = ZedAdapter::try_new(config_root.path().to_path_buf(), binary.clone())?;
        std::fs::write(
            adapter.config_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "theme": "One Dark",
                "context_servers": {
                    "other": { "command": "/abs/other", "args": [], "env": {} }
                }
            }))?,
        )?;
        let ctx = InstallRequestContext::try_with_defaults(binary.clone())?;

        let plan = adapter.plan(&ctx)?;
        assert_eq!(plan.planned_changes.len(), 1);
        adapter.apply(&plan)?;
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(adapter.config_path())?)?;
        assert_eq!(written["theme"], "One Dark");
        assert_eq!(written["context_servers"]["other"]["command"], "/abs/other");
        assert_eq!(
            written["context_servers"]["enforcer"]["command"],
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
        let config_root = tempfile::tempdir()?;
        let binary = config_root.path().join("enforcer");
        let adapter = ZedAdapter::try_new(config_root.path().to_path_buf(), binary.clone())?;
        std::fs::write(adapter.config_path(), "{ invalid json")?;
        let ctx = InstallRequestContext::try_with_defaults(binary)?;
        assert!(matches!(
            adapter.plan(&ctx),
            Err(crate::error::InstallError::MalformedConfig { .. })
        ));
        Ok(())
    }
}
