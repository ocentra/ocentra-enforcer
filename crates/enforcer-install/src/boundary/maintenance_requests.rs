//! Update and doctor request commands at the CLI boundary.
//!
//! BOUNDARY-INVARIANT: maintenance request wire values are decoded before orchestration.
//!
use enforcer_domain::install_types::{DoctorCommand, DryRun, InstallOutputMode, UpdateCommand};

/// Typed request to inspect or apply a binary update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRequest {
    /// Whether the command must only report its plan.
    pub dry_run: DryRun,
    /// Rendering mode for the resulting report.
    pub output: InstallOutputMode,
}

impl From<UpdateRequest> for UpdateCommand {
    fn from(value: UpdateRequest) -> Self {
        Self {
            dry_run: value.dry_run,
            output: value.output,
        }
    }
}

/// Typed request for a read-only health check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoctorRequest {
    /// Rendering mode for the resulting report.
    pub output: InstallOutputMode,
}

impl From<DoctorRequest> for DoctorCommand {
    fn from(value: DoctorRequest) -> Self {
        Self {
            output: value.output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DoctorRequest, UpdateRequest};
    use enforcer_domain::install_types::{DoctorCommand, DryRun, InstallOutputMode, UpdateCommand};

    fn parse_output_mode(raw: &str) -> Option<InstallOutputMode> {
        match raw {
            "human" => Some(InstallOutputMode::Human),
            "json" => Some(InstallOutputMode::Json),
            _ => None,
        }
    }

    #[test]
    fn request_conversions_preserve_canonical_options() {
        let update = UpdateRequest {
            dry_run: DryRun::Enabled,
            output: InstallOutputMode::Json,
        };
        let command: UpdateCommand = update.into();
        assert_eq!(command.dry_run, DryRun::Enabled);
        assert_eq!(command.output, InstallOutputMode::Json);

        let doctor = DoctorRequest {
            output: InstallOutputMode::Human,
        };
        let command: DoctorCommand = doctor.into();
        assert_eq!(command.output, InstallOutputMode::Human);
    }

    #[test]
    fn maintenance_request_negative_case_rejects_unknown_output_mode() {
        assert_eq!(
            parse_output_mode("yaml"),
            None,
            "negative input must be rejected"
        );
    }
}
