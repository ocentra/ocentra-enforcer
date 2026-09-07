//! Native construction of the complete user-level harness adapter registry.
//!
//! This module converts already validated installation paths into the native
//! adapters. It is the one shared construction boundary for install and the
//! separate harness-registration doctor.

use crate::adapters::{
    aider::AiderAdapter, antigravity::AntigravityAdapter, claude::ClaudeAdapter,
    codex::CodexAdapter, cursor::CursorAdapter, gemini::GeminiAdapter, kilocode::KiloCodeAdapter,
    kiro::KiroAdapter, opencode::OpenCodeAdapter, windsurf::WindsurfAdapter, zed::ZedAdapter,
};
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use enforcer_domain::install_types::{InstallBinaryPath, InstallRootPath};

#[cfg(all(unix, not(target_os = "macos")))]
const ZED_CONFIG_DIRECTORY: &str = "zed";
#[cfg(any(windows, target_os = "macos"))]
const ZED_CONFIG_DIRECTORY: &str = "Zed";

/// Build every native user-level harness adapter from validated process paths.
///
/// The registry is complete and ordered deterministically. Callers must not
/// recreate a divergent adapter list.
pub fn adapter_registry(
    home: &InstallRootPath,
    binary: &InstallBinaryPath,
) -> InstallResult<Vec<Box<dyn HarnessAdapter>>> {
    let home_path = home.as_path();
    let config_root = {
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA")
                .filter(|value| !value.is_empty())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| home_path.join("AppData").join("Roaming"))
        }
        #[cfg(target_os = "macos")]
        {
            home_path.join("Library").join("Application Support")
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|value| !value.is_empty())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| home_path.join(".config"))
        }
    };
    // CLONE-JUSTIFICATION: every adapter owns its binary path for later plan/apply/verify calls.
    let binary = binary.as_path().to_path_buf();
    let adapters: Vec<Box<dyn HarnessAdapter>> = vec![
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            AntigravityAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            ClaudeAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            CodexAdapter::try_new(
                std::env::var_os("CODEX_HOME")
                    .filter(|value| !value.is_empty())
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| home_path.join(".codex")),
                // CLONE-JUSTIFICATION: Codex retains its own binary path.
                binary.clone(),
            )
            .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            CursorAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            GeminiAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            KiloCodeAdapter::try_new(config_root.join("Code").join("User"), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            KiroAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        // CLONE-JUSTIFICATION: this adapter retains its own binary path.
        Box::new(
            WindsurfAdapter::try_new(home_path.to_path_buf(), binary.clone())
                .map_err(InstallError::from)?,
        ),
        Box::new(
            ZedAdapter::try_new(config_root.join(ZED_CONFIG_DIRECTORY), binary)
                .map_err(InstallError::from)?,
        ),
        Box::new(AiderAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
    ];
    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::adapter_registry;
    #[cfg(all(unix, not(target_os = "macos")))]
    use super::ZED_CONFIG_DIRECTORY;
    use enforcer_domain::install_types::{InstallBinaryPath, InstallRootPath};

    #[test]
    fn registry_has_each_native_harness_once() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let binary = home.path().join("enforcer");
        let home = InstallRootPath::try_from(home.path().to_path_buf())?;
        let binary = InstallBinaryPath::try_from(binary)?;
        let registry = adapter_registry(&home, &binary)?;
        let mut keys = registry
            .iter()
            .map(|adapter| adapter.harness_key().as_str().to_owned())
            .collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "aider",
                "antigravity",
                "claude",
                "codex",
                "cursor",
                "gemini",
                "kilocode",
                "kiro",
                "opencode",
                "windsurf",
                "zed"
            ]
        );
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn zed_uses_the_lowercase_linux_config_directory() {
        assert_eq!(ZED_CONFIG_DIRECTORY, "zed");
    }
}
