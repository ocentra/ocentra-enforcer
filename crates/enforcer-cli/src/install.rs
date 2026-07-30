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

fn doctor_reports(
    adapters: &[&dyn HarnessAdapter],
    context: &InstallRequestContext,
) -> Result<ExitCode, InstallCommandFailure> {
    let reports = enforcer_install::core::doctor(adapters, context, &DoctorCommand::default())
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

fn doctor_with_current_environment() -> Result<ExitCode, InstallCommandFailure> {
    let binary = std::env::current_exe().map_err(|error| InstallCommandFailure {
        class: FailureClass::Internal,
        message: format!("cannot resolve the running enforcer binary: {error}"),
    })?;
    let context = InstallRequestContext::try_with_defaults(binary.clone())
        .map_err(InstallError::from)
        .map_err(InstallCommandFailure::from)?;
    let home = home_dir()?;
    let home = enforcer_domain::install_types::InstallRootPath::try_from(home)
        .map_err(InstallError::from)
        .map_err(InstallCommandFailure::from)?;
    let registry = enforcer_install::factory::adapter_registry(&home, &context.binary_path)
        .map_err(InstallCommandFailure::from)?;
    let adapters: Vec<&dyn HarnessAdapter> =
        registry.iter().map(std::convert::AsRef::as_ref).collect();

    doctor_reports(&adapters, &context)
}

/// Verify every native user-level harness registration without changing files.
pub(crate) fn doctor() -> Result<ExitCode, InstallCommandFailure> {
    doctor_with_current_environment()
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
    let home = enforcer_domain::install_types::InstallRootPath::try_from(home)
        .map_err(InstallError::from)
        .map_err(InstallCommandFailure::from)?;
    let registry = enforcer_install::factory::adapter_registry(&home, &context.binary_path)
        .map_err(InstallCommandFailure::from)?;
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
    doctor_reports(&adapters, &context)
}
