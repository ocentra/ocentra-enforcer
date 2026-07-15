//! Typed branch-protection policy values used after the GitHub boundary has
//! decoded its wire DTOs.

use std::collections::BTreeSet;

use enforcer_domain::ids::GitHubCheckContext;

/// Whether the protected branch requires a pull request before merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestRequirement {
    /// Pull requests are mandatory.
    Required,
    /// Direct pushes remain possible.
    NotRequired,
}

/// Whether branches must include the protected branch's latest commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpToDateRequirement {
    /// Required checks must be re-run against the latest protected branch.
    Required,
    /// Stale checks can satisfy the protection rule.
    NotRequired,
}

/// Whether a bypass mechanism is available for the protected branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassAllowance {
    /// The operation is blocked by branch protection.
    Denied,
    /// The operation remains available and therefore fails closed verification.
    Allowed,
}

/// Health of the required checks for the candidate merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredChecksHealth {
    /// Every required check is green.
    Passing,
    /// One or more required checks are red or their state is not known yet.
    RedOrPending,
}

/// Whether a required GitHub check context is configured on the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRequirement {
    /// The context is present in the live branch-protection response.
    Present,
    /// The context is absent from the live branch-protection response.
    Missing,
}

/// Typed observation produced from a GitHub branch-protection response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedBranchProtection {
    required_contexts: BTreeSet<GitHubCheckContext>,
    up_to_date: UpToDateRequirement,
    pull_requests: PullRequestRequirement,
    administrator_bypass: BypassAllowance,
    force_push: BypassAllowance,
    deletion: BypassAllowance,
    required_checks: RequiredChecksHealth,
}

impl ObservedBranchProtection {
    /// Construct a normalized observation after a boundary DTO has been
    /// validated into branded check contexts and explicit policy states.
    #[must_use]
    pub fn new(
        required_contexts: BTreeSet<GitHubCheckContext>,
        up_to_date: UpToDateRequirement,
        pull_requests: PullRequestRequirement,
        administrator_bypass: BypassAllowance,
        force_push: BypassAllowance,
        deletion: BypassAllowance,
        required_checks: RequiredChecksHealth,
    ) -> Self {
        Self {
            required_contexts,
            up_to_date,
            pull_requests,
            administrator_bypass,
            force_push,
            deletion,
            required_checks,
        }
    }

    /// Return whether the live API response includes a required context.
    #[must_use]
    pub fn context_requirement(&self, context: &GitHubCheckContext) -> ContextRequirement {
        if self.required_contexts.contains(context) {
            ContextRequirement::Present
        } else {
            ContextRequirement::Missing
        }
    }

    /// Return the observed branch freshness posture.
    #[must_use]
    pub fn up_to_date(&self) -> UpToDateRequirement {
        self.up_to_date
    }

    /// Return the observed pull-request requirement.
    #[must_use]
    pub fn pull_requests(&self) -> PullRequestRequirement {
        self.pull_requests
    }

    /// Return the observed administrator bypass posture.
    #[must_use]
    pub fn administrator_bypass(&self) -> BypassAllowance {
        self.administrator_bypass
    }

    /// Return the observed force-push posture.
    #[must_use]
    pub fn force_push(&self) -> BypassAllowance {
        self.force_push
    }

    /// Return the observed deletion posture.
    #[must_use]
    pub fn deletion(&self) -> BypassAllowance {
        self.deletion
    }

    /// Return the observed required-check health.
    #[must_use]
    pub fn required_checks(&self) -> RequiredChecksHealth {
        self.required_checks
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use enforcer_domain::ids::GitHubCheckContext;

    use super::{
        BypassAllowance, ContextRequirement, ObservedBranchProtection, PullRequestRequirement,
        RequiredChecksHealth, UpToDateRequirement,
    };

    #[test]
    fn observation_keeps_typed_contexts_and_explicit_states() -> Result<(), Box<dyn std::error::Error>> {
        let context = GitHubCheckContext::try_from("Rust CI / rust-ci".to_owned())?;
        let observation = ObservedBranchProtection::new(
            BTreeSet::from([context.clone()]),
            UpToDateRequirement::Required,
            PullRequestRequirement::Required,
            BypassAllowance::Denied,
            BypassAllowance::Denied,
            BypassAllowance::Denied,
            RequiredChecksHealth::Passing,
        );

        assert_eq!(
            observation.context_requirement(&context),
            ContextRequirement::Present
        );
        assert_eq!(observation.required_checks(), RequiredChecksHealth::Passing);
        Ok(())
    }
}
