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

#[derive(Debug, Default, serde::Deserialize)]
struct Manifest {
    // DEFAULT-JUSTIFICATION: a document without kind is outside this RBAC-only rule.
    #[serde(default)]
    // BRAND-INVARIANT: raw manifest kind gates all subsequent RBAC inspection.
    kind: Option<String>,
    // DEFAULT-JUSTIFICATION: bindings do not have rules and an absent rules list grants nothing.
    #[serde(default)]
    rules: Vec<PolicyRule>,
    // DEFAULT-JUSTIFICATION: roles do not have roleRef and cannot bind cluster-admin.
    #[serde(default, rename = "roleRef")]
    role_ref: Option<RoleRef>,
    // DEFAULT-JUSTIFICATION: roles have no subjects and an absent binding subject grants nothing.
    #[serde(default)]
    subjects: Vec<Subject>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct PolicyRule {
    // DEFAULT-JUSTIFICATION: an absent verb list grants no operation.
    #[serde(default)]
    // BRAND-INVARIANT: raw verbs are compared only with the narrow hazardous-verb set.
    verbs: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent resource list grants no resource access.
    #[serde(default)]
    // BRAND-INVARIANT: raw resources are compared only with wildcard and secret resource names.
    resources: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent apiGroups list cannot grant a wildcard group.
    #[serde(default, rename = "apiGroups")]
    // BRAND-INVARIANT: raw API groups are compared only for a wildcard grant.
    api_groups: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent non-resource URL list grants no API-path access.
    #[serde(default, rename = "nonResourceURLs")]
    // BRAND-INVARIANT: raw API paths are compared only for wildcard access.
    non_resource_urls: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RoleRef {
    // DEFAULT-JUSTIFICATION: missing kind is conventionally ClusterRole for a ClusterRoleBinding.
    #[serde(default)]
    // BRAND-INVARIANT: this raw reference kind is used only to qualify cluster-admin bindings.
    kind: Option<String>,
    // DEFAULT-JUSTIFICATION: missing name cannot identify the cluster-admin role.
    #[serde(default)]
    // BRAND-INVARIANT: this raw role name is compared only with the canonical cluster-admin name.
    name: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Subject {
    // DEFAULT-JUSTIFICATION: an omitted kind cannot identify a privileged group subject.
    #[serde(default)]
    // BRAND-INVARIANT: only the Kubernetes Group kind can be system:masters.
    kind: Option<String>,
    // DEFAULT-JUSTIFICATION: an omitted name cannot grant system:masters membership.
    #[serde(default)]
    // BRAND-INVARIANT: compared only with the built-in superuser group name.
    name: Option<String>,
}

#[derive(Debug)]
/// `CYBER-K8S-RBAC.1` — RBAC privilege-escalation hardening manifest gate.
pub struct K8sRbacValidator {
    rule_id: RuleId,
}

impl K8sRbacValidator {
    /// Builds the validator with its canonical, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-K8S-RBAC.1".parse()?,
        })
    }

    fn finding(&self, input: &ValidationInput<'_>, severity: Severity, detail: String) -> Finding {
        // ALLOC-JUSTIFICATION: findings are durable report records and therefore own their static title.
        Finding {
            // CLONE-JUSTIFICATION: each emitted finding owns its stable rule identity after validation returns.
            rule_id: self.rule_id.clone(),
            severity,
            title: "Kubernetes RBAC manifest grants excessive privilege".to_owned(),
            detail,
            // CLONE-JUSTIFICATION: each emitted finding owns its source path after the borrowed input expires.
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }
    }
}

fn has(list: &[String], value: &str) -> bool {
    list.iter().any(|v| v == value)
}

fn any_matches(list: &[String], candidates: &[&str]) -> bool {
    list.iter()
        .any(|v| candidates.iter().any(|c| v.eq_ignore_ascii_case(c)))
}

impl Validator for K8sRbacValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(manifest) = serde_yaml::from_str::<Manifest>(input.source) else {
            return Vec::new();
        };
        let Some(kind) = manifest.kind.as_deref() else {
            return Vec::new();
        };
        if !RBAC_KINDS.contains(&kind) {
            return Vec::new();
        }

        let mut findings = Vec::new();

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
                    findings.push(self.finding(
                        &input,
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
                    findings.push(self.finding(
                        &input,
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
                findings.push(self.finding(
                    &input,
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
                findings.push(self.finding(
                    &input,
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
