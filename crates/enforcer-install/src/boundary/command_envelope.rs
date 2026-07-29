//! Stable non-TTY command-result transport owned by the CLI boundary.
//!
//! Round-trip JSON coverage is provided by `command_envelope_dto_round_trip`.

//! BOUNDARY-INVARIANT: command wire values are decoded before domain dispatch.
//!
use enforcer_domain::{
    boundary::decode_error::DecodeError,
    install_types::{CommandName, InstallVerifyCheck, InstallVerifyReport},
};

/// The stable non-TTY JSON envelope rendered by install commands.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandEnvelopeDto {
    /// Which verb produced this envelope.
    #[serde(with = "crate::boundary::install_type_wire::command_name")]
    pub command: CommandName,
    /// Whether every check passed.
    pub ok: bool,
    /// Health checks collected by the command.
    pub checks: Vec<crate::report::VerifyCheckDto>,
}

impl CommandEnvelopeDto {
    /// Build an envelope and derive its aggregate status from the checks.
    #[must_use]
    pub fn new(command: CommandName, checks: Vec<crate::report::VerifyCheckDto>) -> Self {
        let ok = checks.iter().all(|check| check.passed);
        Self {
            command,
            ok,
            checks,
        }
    }

    /// Convert a typed verification report at the CLI/JSON boundary.
    #[must_use]
    pub fn from_verify_report(command: CommandName, report: InstallVerifyReport) -> Self {
        let checks = report
            .checks
            .into_iter()
            .map(crate::report::VerifyCheckDto::from)
            .collect();
        Self::new(command, checks)
    }
}

/// Convert the serialized command envelope back into the canonical install
/// report before it re-enters domain orchestration.  The command discriminant
/// remains a transport concern, while the checks are validated at the same
/// boundary as every other install report DTO.
impl TryFrom<CommandEnvelopeDto> for InstallVerifyReport {
    type Error = DecodeError;

    fn try_from(value: CommandEnvelopeDto) -> Result<Self, Self::Error> {
        let checks = value
            .checks
            .into_iter()
            .map(crate::report::VerifyCheckDto::try_into)
            .collect::<Result<Vec<InstallVerifyCheck>, DecodeError>>()?;
        let actual_ok = checks.iter().all(|check| {
            matches!(
                check.status,
                enforcer_domain::install_types::CheckStatus::Passed
            )
        });
        if actual_ok != value.ok {
            return Err(DecodeError::new(
                "ok",
                "aggregate status must match every decoded verification check",
            ));
        }
        Ok(InstallVerifyReport { checks })
    }
}

#[cfg(test)]
mod tests {
    use super::CommandEnvelopeDto;
    use crate::report::VerifyCheckDto;
    use enforcer_domain::install_types::{CommandName, InstallVerifyReport};

    fn check(harness: &str, passed: bool) -> VerifyCheckDto {
        VerifyCheckDto {
            harness: harness.to_owned(),
            name: "mcp-registration-present".to_owned(),
            passed,
            detail: String::new(),
        }
    }

    #[test]
    fn command_envelope_dto_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let envelope = CommandEnvelopeDto::new(CommandName::Doctor, vec![check("claude", true)]);
        let wire = serde_json::to_string(&envelope)?;
        let back: CommandEnvelopeDto = serde_json::from_str(&wire)?;
        assert_eq!(back, envelope);
        Ok(())
    }

    #[test]
    fn any_failed_check_makes_envelope_fail() {
        let envelope = CommandEnvelopeDto::new(CommandName::Install, vec![check("codex", false)]);
        assert!(!envelope.ok);
    }

    #[test]
    fn no_checks_is_vacuously_successful() {
        let envelope = CommandEnvelopeDto::new(CommandName::Update, vec![]);
        assert!(envelope.ok);
    }

    #[test]
    fn invalid_envelope_json_is_rejected() {
        let outcome = serde_json::from_str::<CommandEnvelopeDto>(
            r#"{"command":"doctor","ok":"yes","checks":[]}"#,
        );
        assert!(
            outcome.is_err(),
            "invalid input with a non-boolean status must not decode"
        );
        if let Err(error) = outcome {
            assert!(error.is_data());
        }
    }

    #[test]
    fn envelope_converts_back_to_domain_and_rejects_inconsistent_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let envelope = CommandEnvelopeDto::new(CommandName::Doctor, vec![check("claude", true)]);
        let report = InstallVerifyReport::try_from(envelope)?;
        assert_eq!(report.checks.len(), 1);

        let mut inconsistent = CommandEnvelopeDto::new(CommandName::Doctor, vec![]);
        inconsistent.ok = false;
        let error = InstallVerifyReport::try_from(inconsistent)
            .err()
            .ok_or("status mismatch must be rejected")?;
        assert_eq!(error.path, "ok");
        Ok(())
    }
}
