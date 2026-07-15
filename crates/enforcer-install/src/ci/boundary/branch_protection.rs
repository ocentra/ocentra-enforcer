//! GitHub branch-protection read boundary.
//!
//! This module owns the raw JSON response shape from `gh api`. Callers must
//! convert it into [`ObservedBranchProtection`] before applying policy.

use std::collections::BTreeSet;

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::GitHubCheckContext;
use serde::{Deserialize, Serialize};

use super::super::branch_protection_domain::{
    BypassAllowance, ObservedBranchProtection, PullRequestRequirement,
    RequiredChecksHealth, UpToDateRequirement,
};

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
            Some(checks) => {
                checks
                    .contexts
                    .iter()
                    .cloned()
                    .map(GitHubCheckContext::try_from)
                    .collect::<Result<BTreeSet<_>, _>>()?
            }
            None => BTreeSet::new(),
        };
        let up_to_date = match dto.required_status_checks.as_ref().map(|checks| checks.strict) {
            Some(true) => UpToDateRequirement::Required,
            Some(false) | None => UpToDateRequirement::NotRequired,
        };
        let required_checks = if dto.required_status_checks.is_some()
            && dto.required_checks_passing == Some(true)
        {
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

    use super::{LiveProtectionStateDto, RequiredStatusChecksDto};
    use crate::ci::branch_protection_domain::{
        BypassAllowance, ContextRequirement, ObservedBranchProtection,
        PullRequestRequirement, RequiredChecksHealth, UpToDateRequirement,
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
    fn github_read_dto_round_trip(
    ) -> Result<(), Box<dyn std::error::Error>> {
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
}
