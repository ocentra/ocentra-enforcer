//! `CYBER-K8S-RBAC.1` (T1) — Wave-1 cyberskills: Kubernetes RBAC
//! privilege-escalation hardening, a native Rust reimplementation of the
//! inline manifest predicates harvested 1:1 from
//! `vendor/anthropic-cybersecurity-skills/skills/{auditing-kubernetes-rbac-privilege-escalation,
//! implementing-rbac-hardening-for-kubernetes}/scripts/agent.py`.
//!
//! `auditing-kubernetes-rbac-privilege-escalation`'s `agent.py` is a
//! `kubectl auth can-i` / `kubectl get clusterrolebindings` CLI wrapper
//! against a *live* cluster — no CLI subprocess is introduced here.
//! `implementing-rbac-hardening-for-kubernetes`'s `agent.py`, however, also
//! shells out to `kubectl get ... -o json` but its `audit_cluster_roles`
//! and `audit_cluster_role_bindings` functions embed the actual manifest
//! predicates inline (wildcard verbs/resources, secrets read access,
//! `roleRef.name == "cluster-admin"`); those predicates are ported here
//! 1:1 against a manifest parsed directly from source instead of from a
//! `kubectl get -o json` dump, so the check runs offline on a YAML/JSON
//! manifest without ever invoking `kubectl`.
//!
//! It parses an RBAC manifest (YAML or JSON — JSON is a YAML subset, so one
//! `serde_yaml` pass deserializes both) and, for `Role` / `ClusterRole` /
//! `RoleBinding` / `ClusterRoleBinding` kinds, emits one `Finding` per
//! violated check. Any other `kind` (or a document with no recognized
//! kind) is out of scope and yields no findings.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// RBAC kinds this rule inspects. A manifest of any other `kind` (or a
/// non-manifest document) is not this rule's concern.
const RBAC_KINDS: &[&str] = &["Role", "ClusterRole", "RoleBinding", "ClusterRoleBinding"];

/// Verbs that grant read access to secret contents when paired with the
/// `secrets` resource — per the vendor's `dangerous_verbs`/secrets-access
/// check (`get`/`list`/`watch`, or a wildcard).
const SECRET_READ_VERBS: &[&str] = &["get", "list", "watch", "*"];

use crate::boundary::k8s_rbac::{any_matches, has, parse_manifest};

/// `CYBER-K8S-RBAC.1` — RBAC privilege-escalation hardening manifest gate.
#[derive(Debug)]
pub struct K8sRbacValidator {
    rule_id: RuleId,
}

impl K8sRbacValidator {
    /// Builds the validator with its canonical, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberK8sRbac.id(),
        })
    }
}

impl Validator for K8sRbacValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(manifest) = parse_manifest(input.source.as_str()) else {
            return Vec::new();
        };
        let Some(kind) = manifest.kind.as_deref() else {
            return Vec::new();
        };
        if !RBAC_KINDS.contains(&kind) {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let Some(emit) = crate::boundary::finding::ValidationFindingFactory::new(
            &self.rule_id,
            "Kubernetes RBAC manifest grants excessive privilege",
        ) else {
            return findings;
        };

        // Role/ClusterRole `rules:` checks — wildcard grants and secrets
        // read access.
        if kind == "Role" || kind == "ClusterRole" {
            for rule in &manifest.rules {
                let wildcard_verbs = has(&rule.verbs, "*");
                let wildcard_resources = has(&rule.resources, "*");
                let wildcard_api_groups = has(&rule.api_groups, "*");
                let wildcard_non_resource_urls = has(&rule.non_resource_urls, "*");

                if wildcard_verbs
                    || wildcard_resources
                    || wildcard_api_groups
                    || wildcard_non_resource_urls
                {
                    let severity =
                        if wildcard_verbs && (wildcard_resources || wildcard_non_resource_urls) {
                            Severity::Error
                        } else {
                            Severity::Warning
                        };
                    findings.extend(emit.finding(
                        &input,
                        1,
                        severity,
                        format!(
                            "{kind} rule grants a wildcard permission (verbs: {:?}, resources: \
                             {:?}, apiGroups: {:?}, nonResourceURLs: {:?}). Fix: replace `*` \
                             with the specific verbs, resources, API groups, and non-resource \
                             URLs actually required.",
                            rule.verbs, rule.resources, rule.api_groups, rule.non_resource_urls
                        ),
                    ));
                }

                let reads_secrets = any_matches(&rule.resources, &["secrets", "*"])
                    && any_matches(&rule.verbs, SECRET_READ_VERBS);
                if reads_secrets {
                    findings.extend(emit.finding(
                        &input,
                        1,
                        Severity::Error,
                        format!(
                            "{kind} rule grants read access to `secrets` (verbs: {:?}). Fix: \
                             scope this rule to the specific secret names required, or remove \
                             `secrets` from `resources`.",
                            rule.verbs
                        ),
                    ));
                }
            }
        }

        // ClusterRoleBinding `roleRef` check — a binding to `cluster-admin`
        // grants full cluster privilege to its subjects. `roleRef.kind` for
        // a ClusterRoleBinding is conventionally `ClusterRole`; treat a
        // missing `kind` the same way rather than requiring it verbatim.
        if kind == "ClusterRoleBinding" || kind == "RoleBinding" {
            let binds_cluster_admin = manifest.role_ref.as_ref().is_some_and(|role_ref| {
                role_ref.name.as_deref() == Some("cluster-admin")
                    && role_ref
                        .kind
                        .as_deref()
                        .is_none_or(|kind| kind == "ClusterRole")
            });
            if binds_cluster_admin {
                let scope = if kind == "ClusterRoleBinding" {
                    "full cluster privilege"
                } else {
                    "full privilege within the binding namespace"
                };
                findings.extend(emit.finding(
                    &input,
                    1,
                    Severity::Error,
                    format!(
                        "{kind} binds subjects to the `cluster-admin` ClusterRole ({scope}). \
                             Fix: bind to a narrowly scoped ClusterRole or Role instead."
                    ),
                ));
            }

            let binds_system_masters = manifest.subjects.iter().any(|subject| {
                subject.kind.as_deref() == Some("Group")
                    && subject.name.as_deref() == Some("system:masters")
            });
            if binds_system_masters {
                findings.extend(emit.finding(
                    &input,
                    1,
                    Severity::Error,
                    format!(
                        "{kind} binds a subject to the built-in `system:masters` group, whose \
                         members bypass normal RBAC authorization. Fix: remove this subject and \
                         bind named identities to a narrowly scoped Role or ClusterRole instead."
                    ),
                ));
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;

    #[test]
    fn parser_handles_empty_and_rejects_invalid_and_malformed_documents() {
        assert!(
            matches!(parse_manifest(""), Some(manifest) if manifest.kind.is_none()),
            "empty documents have no RBAC kind"
        );
        assert!(
            parse_manifest("kind: [").is_none(),
            "malformed YAML is rejected"
        );
        assert!(
            parse_manifest("kind: Role\nrules: [").is_none(),
            "invalid YAML is rejected"
        );
    }

    #[test]
    fn parser_handles_oversized_metadata_without_panicking() {
        let oversized_name = "x".repeat(4096);
        let document = format!("kind: Role\nmetadata:\n  name: {oversized_name}\n");
        assert_eq!(
            parse_manifest(&document)
                .and_then(|manifest| manifest.kind)
                .as_deref(),
            Some("Role")
        );
    }
}
