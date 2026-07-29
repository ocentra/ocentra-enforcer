//! Kubernetes-manifest built-in IaC detection specifications.

use enforcer_domain::ids::BuiltInIacRule;

use super::spec::{CommentPolicy, RuleSpec, TriggerKind};
use crate::boundary::source_text::IacPattern;

const PRIVILEGED_CONTAINER: &[IacPattern] = &[IacPattern::PrivilegedContainer];

pub(crate) const SPECS: &[RuleSpec] = &[RuleSpec {
    rule: BuiltInIacRule::KubernetesPrivilegedContainer,
    kind: TriggerKind::ForbiddenPresent,
    patterns: PRIVILEGED_CONTAINER,
    comments: CommentPolicy::Ignore,
}];
