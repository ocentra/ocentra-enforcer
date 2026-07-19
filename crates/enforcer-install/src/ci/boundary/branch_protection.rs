//! GitHub branch-protection read boundary.
//!
//! This module owns the raw JSON response shape from `gh api`. Callers must
//! convert it into [`ObservedBranchProtection`] before applying policy.
//!
//! ROUNDTRIP-TEST: `github_branch_protection_dtos_round_trip` exercises the
//! JSON contracts and the canonical verification conversion below.

//! BOUNDARY-INVARIANT: branch-protection reports convert through canonical domain policy.
//!
use serde::{Deserialize, Serialize};

use enforcer_domain::install_types::{
    ContextRequirement, DesiredProtection, ObservedBranchProtection, RefusalReason, Verification,
};

/// JSON-friendly result consumed by installer and CI callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtectionReportDto {
    /// Protected branch named by this fixed policy.
    pub branch: String,
    /// Required contexts normalized from workflow declarations.
    pub expected_contexts: Vec<String>,
    /// Required contexts observed from the GitHub response.
    pub observed_contexts: Vec<String>,
    /// Whether the policy attested the observation.
    pub attested: bool,
    /// Process-compatible status code.
    pub exit_code: i32,
    /// Stable machine-readable refusal codes.
    pub refusal_codes: Vec<String>,
}

/// Render a typed verification result at the serialization boundary.
#[must_use]
pub fn report(
    desired: &DesiredProtection,
    observed: &ObservedBranchProtection,
    verification: &Verification,
) -> BranchProtectionReportDto {
    let expected_contexts = desired
        .required_contexts()
        .iter()
        .map(ToString::to_string)
        .collect();
    let observed_contexts = desired
        .required_contexts()
        .iter()
        .filter(|context| observed.context_requirement(context) == ContextRequirement::Present)
        .map(ToString::to_string)
        .collect();
    match verification {
        Verification::Attested => BranchProtectionReportDto {
            branch: "main".to_owned(),
            expected_contexts,
            observed_contexts,
            attested: true,
            exit_code: 0,
            refusal_codes: Vec::new(),
        },
        Verification::Refused(reasons) => BranchProtectionReportDto {
            branch: "main".to_owned(),
            expected_contexts,
            observed_contexts,
            attested: false,
            exit_code: i32::try_from(reasons.len()).unwrap_or(i32::MAX).max(1),
            refusal_codes: reasons.iter().map(refusal_code).collect(),
        },
    }
}

fn refusal_code(reason: &RefusalReason) -> String {
    match reason {
        RefusalReason::NoRequiredChecks => "no_required_checks",
        RefusalReason::MissingRequiredContext => "missing_required_context",
        RefusalReason::AdministratorBypassAllowed => "admin_override_allowed",
        RefusalReason::ForcePushAllowed => "force_push_allowed",
        RefusalReason::DeletionAllowed => "deletion_allowed",
        RefusalReason::UpToDateNotRequired => "not_required_up_to_date",
        RefusalReason::PullRequestNotRequired => "pull_request_not_required",
        RefusalReason::RequiredChecksNotPassing => "required_checks_not_passing",
    }
    .to_owned()
}

impl TryFrom<BranchProtectionReportDto> for Verification {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(value: BranchProtectionReportDto) -> Result<Self, Self::Error> {
        if value.attested {
            if value.exit_code != 0 || !value.refusal_codes.is_empty() {
                return Err(Self::Error::new(
                    "verification",
                    "attested reports cannot carry refusal codes or a non-zero exit code",
                ));
            }
            return Ok(Self::Attested);
        }

        if value.exit_code <= 0 || value.refusal_codes.is_empty() {
            return Err(Self::Error::new(
                "verification",
                "refused reports require at least one refusal code and a positive exit code",
            ));
        }

        let reasons = value
            .refusal_codes
            .into_iter()
            .map(|code| match code.as_str() {
                "no_required_checks" => Ok(RefusalReason::NoRequiredChecks),
                "missing_required_context" => Ok(RefusalReason::MissingRequiredContext),
                "admin_override_allowed" => Ok(RefusalReason::AdministratorBypassAllowed),
                "force_push_allowed" => Ok(RefusalReason::ForcePushAllowed),
                "deletion_allowed" => Ok(RefusalReason::DeletionAllowed),
                "not_required_up_to_date" => Ok(RefusalReason::UpToDateNotRequired),
                "pull_request_not_required" => Ok(RefusalReason::PullRequestNotRequired),
                "required_checks_not_passing" => Ok(RefusalReason::RequiredChecksNotPassing),
                _ => Err(Self::Error::new(
                    "refusalCode",
                    "unknown branch-protection refusal code",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::Refused(reasons))
    }
}

#[cfg(test)]
mod tests {
    use enforcer_domain::{
        ids::GitHubCheckContext,
        install_types::{
            BypassAllowance, ContextRequirement, ObservedBranchProtection, PullRequestRequirement,
            RequiredChecksHealth, UpToDateRequirement, Verification,
        },
    };

    use super::BranchProtectionReportDto;
    use crate::ci::boundary::{
        branch_protection_payload::{
            BranchProtectionWriteDto, LiveProtectionStateDto, RequiredStatusChecksDto,
        },
        branch_protection_workflow::WorkflowJobDeclaration,
    };
    fn pass_dto() -> LiveProtectionStateDto {
        LiveProtectionStateDto {
            required_status_checks: Some(RequiredStatusChecksDto {
                strict: true,
                contexts: vec!["Rust CI / rust-ci".to_owned()],
            }),
            enforce_admins: true,
            required_pull_request: true,
            allow_force_pushes: false,
            allow_deletions: false,
            required_checks_passing: Some(true),
        }
    }

    #[test]
    fn github_read_dto_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let wire = serde_json::to_string(&pass_dto())?;
        let dto: LiveProtectionStateDto = serde_json::from_str(&wire)?;
        let observed = ObservedBranchProtection::try_from(dto)?;
        let context = GitHubCheckContext::try_from("Rust CI / rust-ci".to_owned())?;

        assert_eq!(
            observed.context_requirement(&context),
            ContextRequirement::Present
        );
        assert_eq!(observed.up_to_date(), UpToDateRequirement::Required);
        assert_eq!(observed.pull_requests(), PullRequestRequirement::Required);
        assert_eq!(observed.administrator_bypass(), BypassAllowance::Denied);
        assert_eq!(observed.force_push(), BypassAllowance::Denied);
        assert_eq!(observed.deletion(), BypassAllowance::Denied);
        assert_eq!(observed.required_checks(), RequiredChecksHealth::Passing);
        Ok(())
    }

    #[test]
    fn github_read_dto_rejects() -> Result<(), Box<dyn std::error::Error>> {
        let dto = LiveProtectionStateDto {
            required_status_checks: Some(RequiredStatusChecksDto {
                strict: true,
                contexts: vec!["Rust CI / rust-ci\nspoof".to_owned()],
            }),
            enforce_admins: true,
            required_pull_request: true,
            allow_force_pushes: false,
            allow_deletions: false,
            required_checks_passing: Some(true),
        };

        let error = ObservedBranchProtection::try_from(dto)
            .err()
            .ok_or("newline-spoofed check context must be rejected")?;
        assert_eq!(error.path, "githubCheckContext");
        assert_eq!(
            error.reason,
            "expected 1..=512 printable characters without line breaks"
        );
        Ok(())
    }

    #[test]
    fn workflow_job_declaration_rejects_invalid_context() -> Result<(), Box<dyn std::error::Error>>
    {
        let invalid = WorkflowJobDeclaration {
            workflow_name: "Rust CI\nspoof".to_owned(),
            job_id: "rust-ci".to_owned(),
            matrix: Vec::new(),
        };

        let error = std::collections::BTreeSet::<GitHubCheckContext>::try_from(invalid)
            .err()
            .ok_or("newline-spoofed workflow name must be rejected")?;
        assert_eq!(error.path, "githubCheckContext");
        assert_eq!(
            error.reason,
            "expected 1..=512 printable characters without line breaks"
        );
        Ok(())
    }

    #[test]
    /// Serializes each GitHub boundary shape in a round-trip test of its actual
    /// JSON contract.
    fn github_branch_protection_dtos_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let write = BranchProtectionWriteDto {
            required_status_checks: RequiredStatusChecksDto {
                strict: true,
                contexts: vec!["Rust CI / rust-ci".to_owned()],
            },
            enforce_admins: true,
            required_pull_request: true,
            allow_force_pushes: false,
            allow_deletions: false,
        };
        let write_json = serde_json::to_string(&write)?;
        assert_eq!(
            serde_json::from_str::<BranchProtectionWriteDto>(&write_json)?,
            write
        );

        let live = pass_dto();
        let live_json = serde_json::to_string(&live)?;
        assert_eq!(
            serde_json::from_str::<LiveProtectionStateDto>(&live_json)?,
            live
        );

        let report = BranchProtectionReportDto {
            branch: "main".to_owned(),
            expected_contexts: vec!["Rust CI / rust-ci".to_owned()],
            observed_contexts: vec!["Rust CI / rust-ci".to_owned()],
            attested: true,
            exit_code: 0,
            refusal_codes: Vec::new(),
        };
        let report_json = serde_json::to_string(&report)?;
        assert_eq!(
            serde_json::from_str::<BranchProtectionReportDto>(&report_json)?,
            report
        );
        assert_eq!(Verification::try_from(report)?, Verification::Attested);
        Ok(())
    }

    #[test]
    fn report_rejects_unknown_refusal_code() -> Result<(), Box<dyn std::error::Error>> {
        let report = BranchProtectionReportDto {
            branch: "main".to_owned(),
            expected_contexts: Vec::new(),
            observed_contexts: Vec::new(),
            attested: false,
            exit_code: 1,
            refusal_codes: vec!["unknown".to_owned()],
        };
        let error = Verification::try_from(report)
            .err()
            .ok_or("unknown refusal code must be rejected")?;
        assert_eq!(error.path, "refusalCode");
        Ok(())
    }
}
