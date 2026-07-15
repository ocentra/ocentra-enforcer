//! GitHub branch-protection read boundary.
//!
//! This module owns the raw JSON response shape from `gh api`. Callers must
//! convert it into [`ObservedBranchProtection`] before applying policy.

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::GitHubCheckContext;
use serde::{Deserialize, Serialize};

use super::super::branch_protection::{DesiredProtection, RefusalReason, Verification};
use super::super::branch_protection_domain::{
    BypassAllowance, ContextRequirement, ObservedBranchProtection, PullRequestRequirement,
    RequiredChecksHealth, UpToDateRequirement,
};

/// Raw workflow declaration used only to derive GitHub check contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowJobDeclaration {
    /// Workflow `name:` value.
    pub workflow_name: String,
    /// Workflow job identifier.
    pub job_id: String,
    /// Matrix values in GitHub's rendered order.
    pub matrix: Vec<String>,
}

impl TryFrom<WorkflowJobDeclaration> for BTreeSet<GitHubCheckContext> {
    type Error = DecodeError;

    fn try_from(dto: WorkflowJobDeclaration) -> Result<Self, Self::Error> {
        let values = if dto.matrix.is_empty() {
            vec![format!("{} / {}", dto.workflow_name, dto.job_id)]
        } else {
            dto.matrix
                .into_iter()
                .map(|value| format!("{} / {} ({value})", dto.workflow_name, dto.job_id))
                .collect()
        };
        values
            .into_iter()
            .map(GitHubCheckContext::try_from)
            .collect()
    }
}

/// Raw GitHub PUT payload emitted from the typed policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtectionWriteDto {
    /// Required status check settings.
    pub required_status_checks: RequiredStatusChecksDto,
    /// GitHub setting that blocks administrator bypass when true.
    pub enforce_admins: bool,
    /// GitHub setting requiring pull requests when true.
    pub required_pull_request: bool,
    /// GitHub setting permitting force pushes when true.
    pub allow_force_pushes: bool,
    /// GitHub setting permitting branch deletion when true.
    pub allow_deletions: bool,
}

impl From<&DesiredProtection> for BranchProtectionWriteDto {
    fn from(desired: &DesiredProtection) -> Self {
        Self {
            required_status_checks: RequiredStatusChecksDto {
                strict: true,
                contexts: desired
                    .required_contexts()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
            enforce_admins: true,
            required_pull_request: true,
            allow_force_pushes: false,
            allow_deletions: false,
        }
    }
}

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

/// Raw `required_status_checks` object returned by GitHub's branch-protection
/// endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredStatusChecksDto {
    /// GitHub's stale-branch requirement flag.
    pub strict: bool,
    /// Raw GitHub check-run contexts; validated during domain conversion.
    pub contexts: Vec<String>,
}

/// Raw GitHub branch-protection response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveProtectionStateDto {
    /// GitHub's required-check configuration, when configured.
    pub required_status_checks: Option<RequiredStatusChecksDto>,
    /// GitHub flag: `true` prevents administrator bypass.
    pub enforce_admins: bool,
    /// GitHub flag: `true` requires pull requests.
    pub required_pull_request: bool,
    /// GitHub flag: `true` allows force pushes.
    pub allow_force_pushes: bool,
    /// GitHub flag: `true` allows branch deletion.
    pub allow_deletions: bool,
    /// GitHub check health: `Some(true)` is green; false or absent is unsafe.
    pub required_checks_passing: Option<bool>,
}

impl TryFrom<LiveProtectionStateDto> for ObservedBranchProtection {
    type Error = DecodeError;

    fn try_from(dto: LiveProtectionStateDto) -> Result<Self, Self::Error> {
        let contexts = match dto.required_status_checks.as_ref() {
            Some(checks) => checks
                .contexts
                .iter()
                .cloned()
                .map(GitHubCheckContext::try_from)
                .collect::<Result<BTreeSet<_>, _>>()?,
            None => BTreeSet::new(),
        };
        let up_to_date = match dto
            .required_status_checks
            .as_ref()
            .map(|checks| checks.strict)
        {
            Some(true) => UpToDateRequirement::Required,
            Some(false) | None => UpToDateRequirement::NotRequired,
        };
        let required_checks =
            if dto.required_status_checks.is_some() && dto.required_checks_passing == Some(true) {
                RequiredChecksHealth::Passing
            } else {
                RequiredChecksHealth::RedOrPending
            };

        Ok(ObservedBranchProtection::new(
            contexts,
            up_to_date,
            if dto.required_pull_request {
                PullRequestRequirement::Required
            } else {
                PullRequestRequirement::NotRequired
            },
            if dto.enforce_admins {
                BypassAllowance::Denied
            } else {
                BypassAllowance::Allowed
            },
            if dto.allow_force_pushes {
                BypassAllowance::Allowed
            } else {
                BypassAllowance::Denied
            },
            if dto.allow_deletions {
                BypassAllowance::Allowed
            } else {
                BypassAllowance::Denied
            },
            required_checks,
        ))
    }
}

#[cfg(test)]
mod tests {
    use enforcer_domain::ids::GitHubCheckContext;

    use super::{
        BranchProtectionReportDto, BranchProtectionWriteDto, LiveProtectionStateDto,
        RequiredStatusChecksDto, WorkflowJobDeclaration,
    };
    use crate::ci::branch_protection_domain::{
        BypassAllowance, ContextRequirement, ObservedBranchProtection, PullRequestRequirement,
        RequiredChecksHealth, UpToDateRequirement,
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
    fn github_read_dto_rejects() {
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

        assert!(ObservedBranchProtection::try_from(dto).is_err());
    }

    #[test]
    fn workflow_job_declaration_rejects_invalid_context() {
        let invalid = WorkflowJobDeclaration {
            workflow_name: "Rust CI\nspoof".to_owned(),
            job_id: "rust-ci".to_owned(),
            matrix: Vec::new(),
        };

        assert!(std::collections::BTreeSet::<GitHubCheckContext>::try_from(invalid).is_err());
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
        Ok(())
    }
}
