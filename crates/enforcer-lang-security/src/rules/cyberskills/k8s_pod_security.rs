//! `CYBER-K8S-POD.1` (T1) — Wave-1 cyberskills: Kubernetes workload
//! pod-security hardening, a native Rust reimplementation of the
//! kubesec/pod-security-standards manifest checks harvested 1:1 from
//! `vendor/anthropic-cybersecurity-skills/skills/{scanning-kubernetes-manifests-with-kubesec,
//! implementing-kubernetes-pod-security-standards,
//! implementing-pod-security-admission-controller}/scripts/agent.py`.
//!
//! It parses a workload manifest (YAML or JSON — JSON is a YAML subset, so
//! one `serde_yaml` pass deserializes both a raw `*.yaml` and
//! `kubectl -o json` output) and, for the pod spec of a `Pod` or the pod
//! template of a `Deployment` / `DaemonSet` / `StatefulSet` / `ReplicaSet`
//! / `Job`, emits one `Finding` per violated pod-security-standards
//! (restricted-profile) check — the same shape a kubesec/kubescape scan
//! emits. No CLI subprocess is introduced; the kube-bench/kubesec binaries
//! are NOT invoked.
//!
//! Namespace-level Pod Security Admission label checks (the
//! `pod-security.kubernetes.io/{enforce,audit,warn}` labels) are a
//! different manifest kind and are tracked as a follow-up, not handled
//! here.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Workload kinds whose pod spec this rule inspects. A manifest of any
/// other `kind` (or a non-manifest document) is not this rule's concern.
const WORKLOAD_KINDS: &[&str] = &[
    "Pod",
    "Deployment",
    "DaemonSet",
    "StatefulSet",
    "ReplicaSet",
    "Job",
    "CronJob",
];

/// Capabilities that escalate to full host compromise — a Critical add.
const CRITICAL_CAPS: &[&str] = &["ALL", "SYS_ADMIN"];
/// The single capability the restricted profile tolerates being added.
const ALLOWED_CAP: &str = "NET_BIND_SERVICE";

use crate::boundary::k8s_pod::{
    effective_run_as_user, parse_manifest, pod_spec, run_as_non_root_ok,
};

/// `CYBER-K8S-POD.1` — pod-security-standards (restricted) manifest gate.
#[derive(Debug)]
pub struct K8sPodSecurityValidator {
    rule_id: RuleId,
}

impl K8sPodSecurityValidator {
    /// Builds the validator with its canonical, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberK8sPod.id(),
        })
    }
}

impl Validator for K8sPodSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(manifest) = parse_manifest(input.source.as_str()) else {
            return Vec::new();
        };
        // Only inspect known workload kinds; anything else (or a document
        // with no kind) is not this rule's concern.
        let is_workload = manifest
            .kind
            .as_deref()
            .is_some_and(|k| WORKLOAD_KINDS.contains(&k));
        if !is_workload {
            return Vec::new();
        }
        let Some(spec) = pod_spec(&manifest) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        let Some(emit) = crate::boundary::finding::ValidationFindingFactory::new(
            &self.rule_id,
            "Kubernetes pod spec violates pod-security-standards hardening",
        ) else {
            return findings;
        };

        // Pod-level host-namespace sharing.
        if spec.host_network {
            findings.extend(emit.at_start(
                &input,
                Severity::Error,
                "pod sets `hostNetwork: true` (shares the node network namespace). Fix: remove \
                 `hostNetwork` or set it to false."
                    // ALLOC-JUSTIFICATION: the finding detail must outlive this borrowed manifest input.
                    .to_owned(),
            ));
        }
        if spec.host_pid {
            findings.extend(
                emit.at_start(
                    &input,
                    Severity::Error,
                    "pod sets `hostPID: true` (shares the node process namespace). Fix: remove \
                 `hostPID` or set it to false."
                        // ALLOC-JUSTIFICATION: the finding detail must outlive this borrowed manifest input.
                        .to_owned(),
                ),
            );
        }
        if spec.host_ipc {
            findings.extend(emit.at_start(
                &input,
                Severity::Error,
                "pod sets `hostIPC: true` (shares the node IPC namespace). Fix: remove `hostIPC` \
                 or set it to false."
                    // ALLOC-JUSTIFICATION: the finding detail must outlive this borrowed manifest input.
                    .to_owned(),
            ));
        }

        let pod_sc = spec.security_context.as_ref();
        for container in spec
            .containers
            .iter()
            .chain(spec.init_containers.iter())
            .chain(spec.ephemeral_containers.iter())
        {
            let name = if container.name.is_empty() {
                "<unnamed>"
            } else {
                &container.name
            };
            let sc = container.security_context.as_ref();

            if sc.and_then(|s| s.privileged) == Some(true) {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Error,
                    format!(
                        "container `{name}` runs `privileged: true` (full host access). Fix: set \
                         `securityContext.privileged: false`."
                    ),
                ));
            }

            // allowPrivilegeEscalation must be explicitly false (missing
            // defaults to true, per the vendor `.get(..., True)`).
            if sc.and_then(|s| s.allow_privilege_escalation) != Some(false) {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Warning,
                    format!(
                        "container `{name}` does not set \
                         `securityContext.allowPrivilegeEscalation: false` (defaults to true). \
                         Fix: set it to false."
                    ),
                ));
            }

            if effective_run_as_user(pod_sc, sc) == Some(0) {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Error,
                    format!(
                        "container `{name}` sets `runAsUser: 0` (runs as root). Fix: run as a \
                         non-zero UID."
                    ),
                ));
            }

            if !run_as_non_root_ok(pod_sc, sc) {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Warning,
                    format!(
                        "container `{name}` does not set `runAsNonRoot: true`. Fix: set \
                         `securityContext.runAsNonRoot: true`."
                    ),
                ));
            }

            if sc.and_then(|s| s.read_only_root_filesystem) != Some(true) {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Warning,
                    format!(
                        "container `{name}` does not set `readOnlyRootFilesystem: true`. Fix: set \
                         `securityContext.readOnlyRootFilesystem: true`."
                    ),
                ));
            }

            if let Some(caps) = sc.and_then(|s| s.capabilities.as_ref()) {
                let dangerous: Vec<&str> = caps
                    .add
                    .iter()
                    .map(String::as_str)
                    .filter(|c| !c.eq_ignore_ascii_case(ALLOWED_CAP))
                    .collect();
                if !dangerous.is_empty() {
                    let critical = dangerous
                        .iter()
                        .any(|c| CRITICAL_CAPS.iter().any(|k| c.eq_ignore_ascii_case(k)));
                    findings.extend(emit.at_start(
                        &input,
                        if critical {
                            Severity::Error
                        } else {
                            Severity::Warning
                        },
                        format!(
                            "container `{name}` adds dangerous Linux capabilities: {}. Fix: drop \
                             all capabilities and add back only `NET_BIND_SERVICE` if required.",
                            dangerous.join(", ")
                        ),
                    ));
                }
                let drops_all = caps.drop.iter().any(|c| c.eq_ignore_ascii_case("ALL"));
                if !drops_all {
                    findings.extend(emit.at_start(
                        &input,
                        Severity::Warning,
                        format!(
                            "container `{name}` does not `drop: [\"ALL\"]` capabilities. Fix: drop \
                             ALL and add back only what is required."
                        ),
                    ));
                }
            } else {
                findings.extend(emit.at_start(
                    &input,
                    Severity::Warning,
                    format!(
                        "container `{name}` does not `drop: [\"ALL\"]` capabilities (no \
                         `capabilities` block). Fix: add `securityContext.capabilities.drop: \
                         [\"ALL\"]`."
                    ),
                ));
            }

            for port in &container.ports {
                if port.host_port.is_some() {
                    findings.extend(emit.at_start(
                        &input,
                        Severity::Error,
                        format!(
                            "container `{name}` binds a `hostPort` (exposes the node's network). \
                             Fix: remove `hostPort` and use a Service instead."
                        ),
                    ));
                    break;
                }
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
            "empty documents have no workload kind"
        );
        assert!(
            parse_manifest("kind: [").is_none(),
            "malformed YAML is rejected"
        );
        assert!(
            parse_manifest("kind: Pod\nspec: [").is_none(),
            "invalid YAML is rejected"
        );
    }

    #[test]
    fn parser_handles_oversized_metadata_without_panicking() {
        let oversized_name = "x".repeat(4096);
        let document = format!("kind: Pod\nmetadata:\n  name: {oversized_name}\n");
        assert!(matches!(parse_manifest(&document), Some(_)));
    }
}
