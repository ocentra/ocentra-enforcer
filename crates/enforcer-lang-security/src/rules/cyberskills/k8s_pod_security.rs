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
    "CronJob",
];

/// Capabilities that escalate to full host compromise — a Critical add.
const CRITICAL_CAPS: &[&str] = &["ALL", "SYS_ADMIN"];
/// The single capability the restricted profile tolerates being added.
const ALLOWED_CAP: &str = "NET_BIND_SERVICE";

#[derive(Debug, Default, serde::Deserialize)]
struct Manifest {
    // DEFAULT-JUSTIFICATION: a document without `kind` is outside this workload-only rule.
    #[serde(default)]
    // BRAND-INVARIANT: raw manifest kind is compared only against WORKLOAD_KINDS before inspection.
    kind: Option<String>,
    // DEFAULT-JUSTIFICATION: missing `spec` means there is no pod surface to validate.
    #[serde(default)]
    spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct PodSpec {
    // DEFAULT-JUSTIFICATION: an omitted hostNetwork field is Kubernetes' safe false default.
    #[serde(default, rename = "hostNetwork")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-network finding.
    host_network: bool,
    // DEFAULT-JUSTIFICATION: an omitted hostPID field is Kubernetes' safe false default.
    #[serde(default, rename = "hostPID")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-PID finding.
    host_pid: bool,
    // DEFAULT-JUSTIFICATION: an omitted hostIPC field is Kubernetes' safe false default.
    #[serde(default, rename = "hostIPC")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-IPC finding.
    host_ipc: bool,
    // DEFAULT-JUSTIFICATION: workloads without containers have no container security context to check.
    #[serde(default)]
    containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: workloads without init containers require no init-container findings.
    #[serde(default, rename = "initContainers")]
    init_containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: workloads without ephemeral containers have no debug-container security context to check.
    #[serde(default, rename = "ephemeralContainers")]
    ephemeral_containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: an absent pod context intentionally delegates to container-level checks.
    #[serde(default, rename = "securityContext")]
    security_context: Option<SecurityContext>,
    /// Present on Deployment/DaemonSet/StatefulSet/ReplicaSet/Job — the pod
    /// template whose `.spec` is the real pod spec.
    // DEFAULT-JUSTIFICATION: a bare Pod has no template and is validated through its direct spec.
    #[serde(default)]
    template: Option<Box<PodTemplate>>,
    /// Present on CronJob â€” the Job template ultimately carries the pod
    /// template whose spec must receive the same restricted-profile checks.
    #[serde(default, rename = "jobTemplate")]
    job_template: Option<Box<JobTemplate>>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct PodTemplate {
    // DEFAULT-JUSTIFICATION: an incomplete template has no pod spec to inspect.
    #[serde(default)]
    spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct JobTemplate {
    #[serde(default)]
    spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Container {
    // DEFAULT-JUSTIFICATION: an omitted container name is reported as <unnamed> without blocking validation.
    #[serde(default)]
    // BRAND-INVARIANT: this transport name is used solely to identify a finding to the manifest author.
    name: String,
    // DEFAULT-JUSTIFICATION: an absent container context inherits applicable pod context or triggers a finding.
    #[serde(default, rename = "securityContext")]
    security_context: Option<SecurityContext>,
    // DEFAULT-JUSTIFICATION: an absent ports list cannot bind a host port.
    #[serde(default)]
    ports: Vec<Port>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Port {
    // DEFAULT-JUSTIFICATION: an omitted hostPort cannot expose a node port.
    #[serde(default, rename = "hostPort")]
    // BRAND-INVARIANT: the raw port number is reduced to presence because any host port is unsafe here.
    host_port: Option<i64>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct SecurityContext {
    // DEFAULT-JUSTIFICATION: absence is distinct from false because Kubernetes defaults privileged to false.
    #[serde(default)]
    // BRAND-INVARIANT: this raw tri-state preserves Kubernetes omission semantics for policy evaluation.
    privileged: Option<bool>,
    // DEFAULT-JUSTIFICATION: absence is unsafe because Kubernetes defaults privilege escalation to true.
    #[serde(default, rename = "allowPrivilegeEscalation")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit-false enforcement requirement.
    allow_privilege_escalation: Option<bool>,
    // DEFAULT-JUSTIFICATION: an omitted UID is evaluated with runAsNonRoot rather than coerced.
    #[serde(default, rename = "runAsUser")]
    // BRAND-INVARIANT: this raw UID is checked only for root (zero) execution.
    run_as_user: Option<i64>,
    // DEFAULT-JUSTIFICATION: absence must remain distinct from true for the restricted profile requirement.
    #[serde(default, rename = "runAsNonRoot")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit run-as-non-root requirement.
    run_as_non_root: Option<bool>,
    // DEFAULT-JUSTIFICATION: absence must remain distinct from true for the restricted profile requirement.
    #[serde(default, rename = "readOnlyRootFilesystem")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit read-only-root requirement.
    read_only_root_filesystem: Option<bool>,
    // DEFAULT-JUSTIFICATION: an absent capabilities block is reported as missing drop ALL.
    #[serde(default)]
    capabilities: Option<Capabilities>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Capabilities {
    // DEFAULT-JUSTIFICATION: an absent add list grants no additional capabilities.
    #[serde(default)]
    // BRAND-INVARIANT: raw capability names are compared case-insensitively to the restricted allowlist.
    add: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent drop list must produce the missing drop ALL finding.
    #[serde(default)]
    // BRAND-INVARIANT: raw capability names are compared case-insensitively for the ALL drop.
    drop: Vec<String>,
}

#[derive(Debug)]
/// `CYBER-K8S-POD.1` — pod-security-standards (restricted) manifest gate.
pub struct K8sPodSecurityValidator {
    rule_id: RuleId,
}

impl K8sPodSecurityValidator {
    /// Builds the validator with its canonical, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-K8S-POD.1".parse()?,
        })
    }

    fn finding(&self, input: &ValidationInput<'_>, severity: Severity, detail: String) -> Finding {
        // ALLOC-JUSTIFICATION: findings are durable report records and therefore own their static title.
        Finding {
            // CLONE-JUSTIFICATION: each emitted finding owns its stable rule identity after validation returns.
            rule_id: self.rule_id.clone(),
            severity,
            title: "Kubernetes pod spec violates pod-security-standards hardening".to_owned(),
            detail,
            // CLONE-JUSTIFICATION: each emitted finding owns its source path after the borrowed input expires.
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
    if manifest.kind.as_deref() == Some("CronJob") {
        return spec
            .job_template
            .as_ref()
            .and_then(|job| job.spec.as_ref())
            .and_then(|job_spec| job_spec.template.as_ref())
            .and_then(|template| template.spec.as_ref());
    }
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
                    // ALLOC-JUSTIFICATION: the finding detail must outlive this borrowed manifest input.
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
                        // ALLOC-JUSTIFICATION: the finding detail must outlive this borrowed manifest input.
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
