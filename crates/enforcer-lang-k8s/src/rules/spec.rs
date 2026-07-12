//! Data-driven rule spec for the K8S family: one [`K8sRuleSpec`] per
//! `K8S-*` rule id, each built into an
//! [`enforcer_lang_common::pattern::PatternValidator`] — the shared
//! `generic-scanner` engine arc-09 owns. Kubernetes manifests are YAML, so
//! (mirroring the TS/`generic_scanner` slice's own note that
//! `rules.json`'s `triggers[0]` restates the rule TITLE rather than a
//! literal keyword) every marker below is a bespoke, semantically-derived
//! YAML key/value literal for the rule's insecure shape, not a copy of any
//! catalog `triggers` string.
//!
//! Detection posture: literal-substring line scan (the `PatternValidator`
//! shape), same as the TS/common families' `generic-scanner` slices. This
//! is deliberately NOT a YAML parser — Kubernetes manifests vary in key
//! ordering/formatting far less than general YAML, and a literal-key scan
//! keeps every rule a single, auditable line rather than a parser
//! dependency. Every marker is anchored on the YAML key spelling
//! (`key: value`) so a rule does not fire on a mention of the same word in
//! an unrelated context (e.g. a `name: privileged-role` metadata label does
//! not trip `K8S-1.1`, which anchors on the `privileged:` key specifically).

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_lang_common::pattern::PatternValidator;

/// One rule's static detection spec: the id, title, and the literal YAML
/// markers whose presence anywhere in a manifest's text flags it.
#[derive(Debug, Clone, Copy)]
pub struct K8sRuleSpec {
    /// The rule id this spec proves, e.g. `K8S-1.1`.
    // BRAND-INVARIANT: static catalog literals are validated into RuleId by
    // build(); malformed ids are rejected instead of reaching a validator.
    rule_id: &'static str,
    /// Human title, mirrored into every `Finding::title`.
    // BRAND-INVARIANT: title is catalog-owned static finding text.
    title: &'static str,
    /// Literal YAML key/value markers; any hit fires (OR'd).
    // BRAND-INVARIANT: markers are catalog-owned static YAML literals.
    markers: &'static [&'static str],
}

impl K8sRuleSpec {
    /// Parse the catalog literal into the branded rule identifier used by
    /// validators and registry consumers.
    pub fn rule_id(&self) -> Result<RuleId, enforcer_core::error::DecodeError> {
        self.rule_id.parse()
    }

    /// Build the [`PatternValidator`] this spec describes. Fails closed
    /// (propagates the parse error) rather than panicking when
    /// `self.rule_id` is not a well-formed [`RuleId`] literal.
    pub fn build(&self) -> Result<PatternValidator, enforcer_core::error::DecodeError> {
        let rule_id = self.rule_id()?;
        Ok(PatternValidator::new(
            rule_id,
            self.title,
            Severity::Error,
            self.markers.to_vec(),
        ))
    }

}

/// Every `K8S-*` rule's static spec, grouped by family:
/// - `K8S-1` pod/container security-context shapes.
/// - `K8S-2` RBAC shapes.
/// - `K8S-3` resource-limit shapes.
/// - `K8S-4` host-namespace/network shapes.
pub const SPECS: &[K8sRuleSpec] = &[
    // --- K8S-1 pod/container security-context rules ------------------------
    K8sRuleSpec {
        rule_id: "K8S-1.1",
        title: "Privileged containers are forbidden",
        markers: &["privileged: true"],
    },
    K8sRuleSpec {
        rule_id: "K8S-1.2",
        title: "Containers must not run as root",
        markers: &["runAsNonRoot: false"],
    },
    K8sRuleSpec {
        rule_id: "K8S-1.3",
        title: "Privilege escalation must be disabled",
        markers: &["allowPrivilegeEscalation: true"],
    },
    K8sRuleSpec {
        rule_id: "K8S-1.4",
        title: "Root filesystem must be read-only",
        markers: &["readOnlyRootFilesystem: false"],
    },
    // --- K8S-2 RBAC rules ---------------------------------------------------
    K8sRuleSpec {
        rule_id: "K8S-2.1",
        title: "Wildcard RBAC verbs are forbidden",
        markers: &["- \"*\" # verbs", "verbs: [\"*\"]", "verbs: ['*']"],
    },
    K8sRuleSpec {
        rule_id: "K8S-2.2",
        title: "Wildcard RBAC resources are forbidden",
        markers: &[
            "- \"*\" # resources",
            "resources: [\"*\"]",
            "resources: ['*']",
        ],
    },
    // --- K8S-3 resource-limit rules ------------------------------------------
    K8sRuleSpec {
        rule_id: "K8S-3.1",
        title: "Containers must declare resource limits",
        markers: &["resources: {}"],
    },
    K8sRuleSpec {
        rule_id: "K8S-3.2",
        title: "Containers must declare memory requests",
        markers: &["requests: {}"],
    },
    // --- K8S-4 host-namespace/network rules ----------------------------------
    K8sRuleSpec {
        rule_id: "K8S-4.1",
        title: "Host network access is forbidden",
        markers: &["hostNetwork: true"],
    },
    K8sRuleSpec {
        rule_id: "K8S-4.2",
        title: "Host PID/IPC namespace access is forbidden",
        markers: &["hostPID: true", "hostIPC: true"],
    },
];

#[cfg(test)]
mod tests {
    use super::SPECS;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    #[test]
    fn every_k8s_spec_fires_on_fail_and_stays_silent_on_pass(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for spec in SPECS {
            let validator = spec.build()?;
            // BRAND-INVARIANT: Cargo supplies this compile-time manifest-root
            // literal; it is converted immediately into a filesystem PathBuf.
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            // BRAND-INVARIANT: fixture paths are derived only from the static,
            // validated catalog literal paired with this exact spec.
            let slug = spec.rule_id.to_lowercase().replace('.', "-");
            let fail = format!("fixtures/generic-scanner/{slug}/fail.yaml");
            let pass = format!("fixtures/generic-scanner/{slug}/pass.yaml");
            run_fixture_parity(&validator, &manifest_dir, &fail, &pass)?;
        }
        Ok(())
    }
}
