//! Typed policy for GitHub main-branch protection.
//!
//! Wire JSON and workflow text are decoded in `ci::boundary::branch_protection`.
//! This module only decides whether a normalized observation satisfies the
//! repository's fail-closed protection policy.

use std::collections::BTreeSet;

use enforcer_domain::ids::GitHubCheckContext;

use super::branch_protection_domain::{
    BypassAllowance, ContextRequirement, ObservedBranchProtection, PullRequestRequirement,
    RequiredChecksHealth, UpToDateRequirement,
};

/// The normalized policy required for the protected branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredProtection {
    required_contexts: BTreeSet<GitHubCheckContext>,
}

impl DesiredProtection {
    /// Build the baseline policy from validated GitHub check contexts.
    #[must_use]
    pub fn baseline(required_contexts: BTreeSet<GitHubCheckContext>) -> Self {
        Self { required_contexts }
    }

    /// The validated contexts that must be required by branch protection.
    #[must_use]
    pub fn required_contexts(&self) -> &BTreeSet<GitHubCheckContext> {
        &self.required_contexts
    }
}

/// A concrete reason the normalized observation does not satisfy policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// No required GitHub check is configured.
    NoRequiredChecks,
    /// One required context is absent from GitHub's protection settings.
    MissingRequiredContext,
    /// Administrators can bypass required checks.
    AdministratorBypassAllowed,
    /// Force pushes remain available.
    ForcePushAllowed,
    /// Branch deletion remains available.
    DeletionAllowed,
    /// Branches are not required to be current before merge.
    UpToDateNotRequired,
    /// Pull requests are not required.
    PullRequestNotRequired,
    /// Required checks are red or pending.
    RequiredChecksNotPassing,
}

/// Fail-closed result of evaluating branch protection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    /// Every required property was observed.
    Attested,
    /// One or more required properties were absent or unsafe.
    Refused(Vec<RefusalReason>),
}

/// Evaluate a normalized GitHub observation against the repository policy.
#[must_use]
pub fn verify(desired: &DesiredProtection, observed: &ObservedBranchProtection) -> Verification {
    let mut reasons = Vec::new();

    if desired.required_contexts.is_empty() {
        reasons.push(RefusalReason::NoRequiredChecks);
    }
    for context in &desired.required_contexts {
        if observed.context_requirement(context) == ContextRequirement::Missing {
            reasons.push(RefusalReason::MissingRequiredContext);
        }
    }
    if observed.up_to_date() == UpToDateRequirement::NotRequired {
        reasons.push(RefusalReason::UpToDateNotRequired);
    }
    if observed.pull_requests() == PullRequestRequirement::NotRequired {
        reasons.push(RefusalReason::PullRequestNotRequired);
    }
    if observed.administrator_bypass() == BypassAllowance::Allowed {
        reasons.push(RefusalReason::AdministratorBypassAllowed);
    }
    if observed.force_push() == BypassAllowance::Allowed {
        reasons.push(RefusalReason::ForcePushAllowed);
    }
    if observed.deletion() == BypassAllowance::Allowed {
        reasons.push(RefusalReason::DeletionAllowed);
    }
    if observed.required_checks() == RequiredChecksHealth::RedOrPending {
        reasons.push(RefusalReason::RequiredChecksNotPassing);
    }

    if reasons.is_empty() {
        Verification::Attested
    } else {
        Verification::Refused(reasons)
    }
}
