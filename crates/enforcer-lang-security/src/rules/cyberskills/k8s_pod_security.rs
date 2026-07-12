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

use enforcer_core::error::DecodeError;
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
];

/// Capabilities that escalate to full host compromise — a Critical add.
const CRITICAL_CAPS: &[&str] = &["ALL", "SYS_ADMIN"];
/// The single capability the restricted profile tolerates being added.
const ALLOWED_CAP: &str = "NET_BIND_SERVICE";

#[derive(Debug, Default, serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct PodSpec {
    #[serde(default, rename = "hostNetwork")]
    host_network: bool,
    #[serde(default, rename = "hostPID")]
    host_pid: bool,
    #[serde(default, rename = "hostIPC")]
    host_ipc: bool,
    #[serde(default)]
    containers: Vec<Container>,
    #[serde(default, rename = "initContainers")]
    init_containers: Vec<Container>,
    #[serde(default, rename = "securityContext")]
    security_context: Option<SecurityContext>,
    /// Present on Deployment/DaemonSet/StatefulSet/ReplicaSet/Job — the pod
    /// template whose `.spec` is the real pod spec.
    #[serde(default)]
    template: Option<Box<PodTemplate>>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct PodTemplate {
    #[serde(default)]
    spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Container {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "securityContext")]
    security_context: Option<SecurityContext>,
    #[serde(default)]
    ports: Vec<Port>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Port {
    #[serde(default, rename = "hostPort")]
    host_port: Option<i64>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct SecurityContext {
    #[serde(default)]
    privileged: Option<bool>,
    #[serde(default, rename = "allowPrivilegeEscalation")]
    allow_privilege_escalation: Option<bool>,
    #[serde(default, rename = "runAsUser")]
    run_as_user: Option<i64>,
    #[serde(default, rename = "runAsNonRoot")]
    run_as_non_root: Option<bool>,
    #[serde(default, rename = "readOnlyRootFilesystem")]
    read_only_root_filesystem: Option<bool>,
    #[serde(default)]
    capabilities: Option<Capabilities>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Capabilities {
    #[serde(default)]
    add: Vec<String>,
    #[serde(default)]
    drop: Vec<String>,
}

/// `CYBER-K8S-POD.1` — pod-security-standards (restricted) manifest gate.
pub struct K8sPodSecurityValidator {
    rule_id: RuleId,
}

impl K8sPodSecurityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-K8S-POD.1".parse()?,
        })
    }

    fn finding(&self, input: &ValidationInput<'_>, severity: Severity, detail: String) -> Finding {
        Finding {
            rule_id: self.rule_id.clone(),
            severity,
            title: "Kubernetes pod spec violates pod-security-standards hardening".to_owned(),
            detail,
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }
    }
}

/// Resolve the effective pod spec: a Deployment/etc carries it under
/// `spec.template.spec`; a bare Pod carries it under `spec` directly.
fn pod_spec(manifest: &Manifest) -> Option<&PodSpec> {
    let spec = manifest.spec.as_ref()?;
    match spec.template.as_ref().and_then(|t| t.spec.as_ref()) {
        Some(template_spec) => Some(template_spec),
        None => Some(spec),
    }
}

/// Whether the container's effective `runAsNonRoot` is satisfied, honoring
/// the pod-level securityContext as the default when the container does not
/// override it (standard Kubernetes inheritance).
fn run_as_non_root_ok(pod_sc: Option<&SecurityContext>, ctr_sc: Option<&SecurityContext>) -> bool {
    let effective = ctr_sc
        .and_then(|sc| sc.run_as_non_root)
        .or_else(|| pod_sc.and_then(|sc| sc.run_as_non_root));
    effective == Some(true)
}

fn effective_run_as_user(
    pod_sc: Option<&SecurityContext>,
    ctr_sc: Option<&SecurityContext>,
) -> Option<i64> {
    ctr_sc
        .and_then(|sc| sc.run_as_user)
        .or_else(|| pod_sc.and_then(|sc| sc.run_as_user))
}

impl Validator for K8sPodSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(manifest) = serde_yaml::from_str::<Manifest>(input.source) else {
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

        // Pod-level host-namespace sharing.
        if spec.host_network {
            findings.push(self.finding(
                &input,
                Severity::Error,
                "pod sets `hostNetwork: true` (shares the node network namespace). Fix: remove \
                 `hostNetwork` or set it to false."
                    .to_owned(),
            ));
        }
        if spec.host_pid {
            findings.push(
                self.finding(
                    &input,
                    Severity::Error,
                    "pod sets `hostPID: true` (shares the node process namespace). Fix: remove \
                 `hostPID` or set it to false."
                        .to_owned(),
                ),
            );
        }
        if spec.host_ipc {
            findings.push(self.finding(
                &input,
                Severity::Error,
                "pod sets `hostIPC: true` (shares the node IPC namespace). Fix: remove `hostIPC` \
                 or set it to false."
                    .to_owned(),
            ));
        }

        let pod_sc = spec.security_context.as_ref();
        for container in spec.containers.iter().chain(spec.init_containers.iter()) {
            let name = if container.name.is_empty() {
                "<unnamed>"
            } else {
                &container.name
            };
            let sc = container.security_context.as_ref();

            if sc.and_then(|s| s.privileged) == Some(true) {
                findings.push(self.finding(
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
                findings.push(self.finding(
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
                findings.push(self.finding(
                    &input,
                    Severity::Error,
                    format!(
                        "container `{name}` sets `runAsUser: 0` (runs as root). Fix: run as a \
                         non-zero UID."
                    ),
                ));
            }

            if !run_as_non_root_ok(pod_sc, sc) {
                findings.push(self.finding(
                    &input,
                    Severity::Warning,
                    format!(
                        "container `{name}` does not set `runAsNonRoot: true`. Fix: set \
                         `securityContext.runAsNonRoot: true`."
                    ),
                ));
            }

            if sc.and_then(|s| s.read_only_root_filesystem) != Some(true) {
                findings.push(self.finding(
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
                    findings.push(self.finding(
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
                    findings.push(self.finding(
                        &input,
                        Severity::Warning,
                        format!(
                            "container `{name}` does not `drop: [\"ALL\"]` capabilities. Fix: drop \
                             ALL and add back only what is required."
                        ),
                    ));
                }
            } else {
                findings.push(self.finding(
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
                    findings.push(self.finding(
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
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::K8sPodSecurityValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_k8s_pod_security() -> Result<(), Box<dyn std::error::Error>> {
        let validator = K8sPodSecurityValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/k8s.pod.security-hardening/bad/privileged.yaml",
            "tests/fixtures/cyberskills/k8s.pod.security-hardening/good/hardened.yaml",
        )?;
        Ok(())
    }
}
