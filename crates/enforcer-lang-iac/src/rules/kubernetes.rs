//! `iac/k8s-*` — the Kubernetes-manifest slice of the IaC rule family:
//! IAC-1.8.

use super::spec::{RuleSpec, TriggerKind};

/// Every Kubernetes-manifest rule's static spec, in `rules/rules.json`
/// declaration order.
pub const SPECS: &[RuleSpec] = &[RuleSpec {
    rule_id: "IAC-1.8",
    title: "Kubernetes containers must not run privileged",
    kind: TriggerKind::ForbiddenPresent,
    needles: &["privileged: true"],
    comment_guard: true,
}];
