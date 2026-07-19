//! Shared install request context decoded at the CLI boundary.

//! BOUNDARY-INVARIANT: request context wire values are decoded before domain use.
//!
use enforcer_domain::{
    boundary::decode_error::DecodeError,
    install_types::{DryRun, InstallOutputMode, InstallRequestContext, InstallScope},
};

/// Shared fields carried by every install command.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContextDto {
    /// Install scope selected by the caller.
    #[serde(with = "crate::boundary::install_type_wire::install_scope")]
    pub scope: InstallScope,
    /// Whether planning must avoid all writes.
    #[serde(with = "crate::boundary::install_type_wire::dry_run")]
    pub dry_run: DryRun,
    /// Rendering mode for the resulting report.
    #[serde(with = "crate::boundary::install_type_wire::install_output_mode")]
    pub output: InstallOutputMode,
    /// Absolute path of the binary being installed.
    pub binary_path: std::path::PathBuf,
}

impl RequestContextDto {
    /// Build the release-default user-scope context.
    pub fn try_with_defaults(binary_path: std::path::PathBuf) -> Result<Self, DecodeError> {
        let context = InstallRequestContext::try_with_defaults(binary_path)?;
        Ok(Self {
            scope: context.scope,
            dry_run: context.dry_run,
            output: context.output,
            binary_path: context.binary_path.as_path().to_path_buf(),
        })
    }
}

impl TryFrom<RequestContextDto> for InstallRequestContext {
    type Error = DecodeError;

    fn try_from(value: RequestContextDto) -> Result<Self, Self::Error> {
        Ok(Self {
            scope: value.scope,
            dry_run: value.dry_run,
            output: value.output,
            binary_path: value.binary_path.try_into()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RequestContextDto;
    use enforcer_domain::install_types::{
        DryRun, InstallOutputMode, InstallRequestContext, InstallScope,
    };

    #[test]
    fn defaults_are_user_scope_human_output_and_writes_enabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = RequestContextDto::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        assert_eq!(context.scope, InstallScope::User);
        assert_eq!(context.dry_run, DryRun::Disabled);
        assert_eq!(context.output, InstallOutputMode::Human);
        Ok(())
    }

    #[test]
    fn request_context_dto_round_trip_preserves_the_absolute_binary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dto = RequestContextDto::try_with_defaults(std::env::temp_dir().join("enforcer"))?;
        let wire = serde_json::to_string(&dto)?;
        let round_trip: RequestContextDto = serde_json::from_str(&wire)?;
        assert_eq!(round_trip, dto);
        Ok(())
    }

    #[test]
    fn relative_binary_path_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let dto = RequestContextDto {
            scope: InstallScope::User,
            dry_run: DryRun::Disabled,
            output: InstallOutputMode::Human,
            binary_path: "relative/enforcer".into(),
        };
        let error = InstallRequestContext::try_from(dto)
            .err()
            .ok_or("invalid relative binary path must fail")?;
        assert_eq!(error.path, "installBinaryPath");
        assert_eq!(error.reason, "must be an absolute filesystem path");
        Ok(())
    }
}
