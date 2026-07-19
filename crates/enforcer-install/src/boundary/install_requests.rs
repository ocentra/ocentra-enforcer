//! Install and uninstall request commands at the CLI boundary.
//!
//! BOUNDARY-INVARIANT: install request wire values are decoded before orchestration.
//!
use crate::request_context::RequestContextDto;
use enforcer_domain::{
    ids::HarnessId,
    install_types::{InstallCommand, UninstallCommand},
};

/// Typed request to install one or more harness integrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    /// Shared scope, output, dry-run, and binary-path fields.
    pub context: RequestContextDto,
    /// Selected harnesses; empty selects all detected harnesses.
    pub only_harnesses: Vec<HarnessId>,
}

impl TryFrom<InstallRequest> for InstallCommand {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: InstallRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            context: value.context.try_into()?,
            only_harnesses: value.only_harnesses,
        })
    }
}

/// Typed request to uninstall one or more harness integrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallRequest {
    /// Shared scope, output, dry-run, and binary-path fields.
    pub context: RequestContextDto,
    /// Selected harnesses; empty selects all detected harnesses.
    pub only_harnesses: Vec<HarnessId>,
}

impl TryFrom<UninstallRequest> for UninstallCommand {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: UninstallRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            context: value.context.try_into()?,
            only_harnesses: value.only_harnesses,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{InstallRequest, RequestContextDto};
    use enforcer_domain::install_types::{DryRun, InstallCommand, InstallOutputMode, InstallScope};

    #[test]
    fn install_request_rejects_an_invalid_relative_binary_path(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = InstallRequest {
            context: RequestContextDto {
                scope: InstallScope::User,
                dry_run: DryRun::Disabled,
                output: InstallOutputMode::Human,
                binary_path: "relative/enforcer".into(),
            },
            only_harnesses: Vec::new(),
        };
        let error = InstallCommand::try_from(request)
            .err()
            .ok_or("invalid relative binary path must be rejected")?;
        assert_eq!(error.path, "installBinaryPath");
        Ok(())
    }
}
