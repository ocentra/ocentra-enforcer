//! Typed process boundary for `enforcer install`.
//!
//! The CLI resolves platform user-config roots once, constructs every native
//! harness adapter with the absolute running binary path, delegates the
//! plan/apply cycle to `enforcer-install`, and then runs the read-only doctor
//! fold before returning a stable CLI exit class. Terminal rendering remains
//! in `main.rs`/`enforcer_cli::output`; this module owns no output sink.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use enforcer_domain::core_types::ExitCode;
use enforcer_domain::install_types::{
    CheckStatus, DoctorCommand, InstallCommand, InstallRequestContext,
};
use enforcer_install::adapters::{
    aider::AiderAdapter, antigravity::AntigravityAdapter, claude::ClaudeAdapter,
    codex::CodexAdapter, cursor::CursorAdapter, gemini::GeminiAdapter, kilocode::KiloCodeAdapter,
    kiro::KiroAdapter, opencode::OpenCodeAdapter, windsurf::WindsurfAdapter, zed::ZedAdapter,
};
use enforcer_install::core::HarnessAdapter;
use enforcer_install::error::InstallError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureClass {
    Config,
    Internal,
}

/// A typed failure at the CLI-to-installer boundary.
#[derive(Debug)]
pub(crate) struct InstallCommandFailure {
    class: FailureClass,
    message: String,
}

impl InstallCommandFailure {
    fn environment(variable: &'static str) -> Self {
        Self {
            class: FailureClass::Config,
            message: format!(
                "cannot resolve the user installation root: set `{variable}` to an absolute path"
            ),
        }
    }

    pub(crate) fn exit_code(&self) -> ExitCode {
        match self.class {
            FailureClass::Config => ExitCode::ConfigError,
            FailureClass::Internal => ExitCode::InternalError,
        }
    }
}

impl From<InstallError> for InstallCommandFailure {
    fn from(error: InstallError) -> Self {
        let class = match error {
            InstallError::InvalidDomain(_)
            | InstallError::MalformedConfig { .. }
            | InstallError::ManagedBlockInvalid { .. }
            | InstallError::UnknownAdapter { .. }
            | InstallError::InvalidCiCommand { .. } => FailureClass::Config,
            InstallError::Io { .. }
            | InstallError::BackupFailed { .. }
            | InstallError::UnsupportedTarget { .. }
            | InstallError::DistributionFailed { .. }
            | InstallError::VerificationFailed(_)
            | InstallError::SkillAssetInvalid { .. } => FailureClass::Internal,
        };
        Self {
            class,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for InstallCommandFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn non_empty_env(variable: &str) -> Option<OsString> {
    std::env::var_os(variable).filter(|value| !value.is_empty())
}

fn home_dir() -> Result<PathBuf, InstallCommandFailure> {
    #[cfg(windows)]
    let value = non_empty_env("USERPROFILE").or_else(|| non_empty_env("HOME"));
    #[cfg(not(windows))]
    let value = non_empty_env("HOME");

    value
        .map(PathBuf::from)
        .ok_or_else(|| InstallCommandFailure::environment("HOME"))
}

fn codex_home(home: &std::path::Path) -> PathBuf {
    non_empty_env("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"))
}

fn platform_config_root(home: &std::path::Path) -> PathBuf {
    #[cfg(windows)]
    {
        non_empty_env("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
    }
    #[cfg(target_os = "macos")]
    {
        return home.join("Library").join("Application Support");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        non_empty_env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
    }
}

fn adapter_registry(
    home: &std::path::Path,
    binary: &std::path::Path,
) -> Result<Vec<Box<dyn HarnessAdapter>>, InstallCommandFailure> {
    let config_root = platform_config_root(home);
    let binary = binary.to_path_buf();
    let adapters: Vec<Box<dyn HarnessAdapter>> = vec![
        Box::new(
            AntigravityAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            ClaudeAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            CodexAdapter::try_new(codex_home(home), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            CursorAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            GeminiAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            KiloCodeAdapter::try_new(config_root.join("Code").join("User"), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            KiroAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            WindsurfAdapter::try_new(home.to_path_buf(), binary.clone())
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(
            ZedAdapter::try_new(config_root.join("Zed"), binary)
                .map_err(InstallError::from)
                .map_err(InstallCommandFailure::from)?,
        ),
        Box::new(AiderAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
    ];
    Ok(adapters)
}

/// Apply every native user-level harness registration and verify the result.
pub(crate) fn run() -> Result<ExitCode, InstallCommandFailure> {
    let binary = std::env::current_exe().map_err(|error| InstallCommandFailure {
        class: FailureClass::Internal,
        message: format!("cannot resolve the running enforcer binary: {error}"),
    })?;
    let context = InstallRequestContext::try_with_defaults(binary.clone())
        .map_err(InstallError::from)
        .map_err(InstallCommandFailure::from)?;
    let home = home_dir()?;
    let registry = adapter_registry(&home, &binary)?;
    let adapters: Vec<&dyn HarnessAdapter> =
        registry.iter().map(std::convert::AsRef::as_ref).collect();

    enforcer_install::core::install(
        &adapters,
        &InstallCommand {
            context: context.clone(),
            only_harnesses: Vec::new(),
        },
    )
    .map_err(InstallCommandFailure::from)?;

    let reports = enforcer_install::core::doctor(&adapters, &context, &DoctorCommand::default())
        .map_err(InstallCommandFailure::from)?;
    let verified = reports.iter().all(|(_, report)| {
        report
            .checks
            .iter()
            .all(|check| check.status == CheckStatus::Passed)
    });
    Ok(if verified {
        ExitCode::Success
    } else {
        ExitCode::Violations
    })
}
