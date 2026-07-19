//! Cursor user-scope MCP registration.
//!
//! Cursor reads global custom servers from `~/.cursor/mcp.json` under the
//! top-level `mcpServers` object. The shared generic JSON adapter performs
//! an idempotent value merge and preserves unrelated settings and servers.

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

/// Cursor adapter rooted at the user's home directory.
#[derive(Debug, Clone)]
pub struct CursorAdapter {
    home: InstallRootPath,
    inner: GenericAdapter,
}

impl CursorAdapter {
    /// Build a global Cursor adapter.
    pub fn try_new(home: PathBuf, binary_path: PathBuf) -> Result<Self, DecodeError> {
        let home = InstallRootPath::try_from(home)?;
        let target = InstallTargetPath::try_from(home.as_path().join(".cursor").join("mcp.json"))?;
        let inner = GenericAdapter::new(GenericAdapterConfig::new(
            BuiltInHarness::Cursor.id(),
            target,
            InstallBinaryPath::try_from(binary_path)?,
        ));
        Ok(Self { home, inner })
    }

    /// Native Cursor global MCP path.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.home.as_path().join(".cursor").join("mcp.json")
    }
}

impl HarnessAdapter for CursorAdapter {
    fn harness_key(&self) -> enforcer_domain::ids::HarnessId {
        BuiltInHarness::Cursor.id()
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
    use super::CursorAdapter;
    use crate::core::HarnessAdapter;
    use enforcer_domain::install_types::{CheckStatus, InstallRequestContext};

    #[test]
    fn native_mcp_file_round_trip_is_idempotent_and_preserves_unrelated_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = CursorAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        std::fs::create_dir_all(adapter.config_path().parent().ok_or("missing parent")?)?;
        std::fs::write(
            adapter.config_path(),
            serde_json::to_string_pretty(&serde_json::json!({
                "unrelated": true,
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
        assert_eq!(written["unrelated"], true);
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
    fn malformed_mcp_file_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("enforcer");
        let adapter = CursorAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
        std::fs::create_dir_all(adapter.config_path().parent().ok_or("missing parent")?)?;
        std::fs::write(adapter.config_path(), "{ invalid json")?;
        let ctx = InstallRequestContext::try_with_defaults(binary)?;
        assert!(matches!(
            adapter.verify(&ctx),
            Err(crate::error::InstallError::MalformedConfig { .. })
        ));
        Ok(())
    }
}
