//! GitHub branch-protection request and response payloads.
//!
//! BOUNDARY-INVARIANT: GitHub payloads convert to canonical protection types.
//!
use std::collections::BTreeSet;

use enforcer_domain::{
    boundary::decode_error::DecodeError,
    ids::GitHubCheckContext,
    install_types::{
        BranchProtectionBypassPolicy, BranchProtectionRequirements, BypassAllowance,
        DesiredProtection, ObservedBranchProtection, PullRequestRequirement, RequiredChecksHealth,
        UpToDateRequirement,
    },
};
use serde::{Deserialize, Serialize};

/// Raw GitHub PUT payload emitted from the typed policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtectionWriteDto {
    /// Required status-check settings.
    pub required_status_checks: RequiredStatusChecksDto,
    /// Whether administrators are subject to protection.
    pub enforce_admins: bool,
    /// Whether pull requests are required.
    pub required_pull_request: bool,
    /// Whether force pushes are permitted.
    pub allow_force_pushes: bool,
    /// Whether branch deletion is permitted.
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

/// Raw `required_status_checks` object returned by GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredStatusChecksDto {
    /// GitHub's stale-branch requirement flag.
    pub strict: bool,
    /// Raw check-run contexts, validated during domain conversion.
    pub contexts: Vec<String>,
}

/// Raw GitHub branch-protection response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveProtectionStateDto {
    /// Required-check configuration, when configured.
    pub required_status_checks: Option<RequiredStatusChecksDto>,
    /// Whether administrators are subject to protection.
    pub enforce_admins: bool,
    /// Whether pull requests are required.
    pub required_pull_request: bool,
    /// Whether force pushes are permitted.
    pub allow_force_pushes: bool,
    /// Whether branch deletion is permitted.
    pub allow_deletions: bool,
    /// Whether required checks currently pass.
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
            BranchProtectionRequirements::new(
                up_to_date,
                if dto.required_pull_request {
                    PullRequestRequirement::Required
                } else {
                    PullRequestRequirement::NotRequired
                },
                required_checks,
            ),
            BranchProtectionBypassPolicy::new(
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
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{LiveProtectionStateDto, RequiredStatusChecksDto};
    use enforcer_domain::install_types::ObservedBranchProtection;

    #[test]
    fn invalid_check_context_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
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
            .ok_or("invalid newline-spoofed check context must fail")?;
        assert_eq!(error.path, "githubCheckContext");
        Ok(())
    }
}
