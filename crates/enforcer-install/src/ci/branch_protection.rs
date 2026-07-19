//! Typed policy for GitHub main-branch protection.
//!
//! Wire JSON and workflow text are decoded in `ci::boundary::branch_protection`.
//! This module only decides whether a normalized observation satisfies the
//! repository's fail-closed protection policy.

use enforcer_domain::install_types::{
    BypassAllowance, ContextRequirement, DesiredProtection, ObservedBranchProtection,
    PullRequestRequirement, RefusalReason, RequiredChecksHealth, UpToDateRequirement, Verification,
};

/// Evaluate a normalized GitHub observation against the repository policy.
#[must_use]
pub fn verify(desired: &DesiredProtection, observed: &ObservedBranchProtection) -> Verification {
    let mut reasons = Vec::new();

    if desired.required_contexts().is_empty() {
        reasons.push(RefusalReason::NoRequiredChecks);
    }
    for context in desired.required_contexts() {
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
