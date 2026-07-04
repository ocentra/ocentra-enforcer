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

#[cfg(test)]
mod tests {
    use super::SPECS;
    use crate::rules::spec::SpecValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn spec_for(rule_id: &str) -> Result<crate::rules::spec::RuleSpec, String> {
        SPECS
            .iter()
            .find(|spec| spec.rule_id == rule_id)
            .copied()
            .ok_or_else(|| format!("no kubernetes spec for {rule_id}"))
    }

    #[test]
    fn iac_1_8_fires_on_privileged_container_and_stays_silent_when_unprivileged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.8")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/kubernetes/iac-1-8/fail.k8s.yaml",
            "fixtures/kubernetes/iac-1-8/pass.k8s.yaml",
        )?;
        Ok(())
    }
}
