//! Kubernetes workload manifest decoding boundary.
//! Malformed YAML is rejected, with negative coverage in this module's tests.

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Manifest {
    // DEFAULT-JUSTIFICATION: a document without `kind` is outside this workload-only rule.
    #[serde(default)]
    // BRAND-INVARIANT: raw manifest kind is compared only against WORKLOAD_KINDS before inspection.
    pub(crate) kind: Option<String>,
    // DEFAULT-JUSTIFICATION: missing `spec` means there is no pod surface to validate.
    #[serde(default)]
    pub(crate) spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct PodSpec {
    // DEFAULT-JUSTIFICATION: an omitted hostNetwork field is Kubernetes' safe false default.
    #[serde(default, rename = "hostNetwork")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-network finding.
    pub(crate) host_network: bool,
    // DEFAULT-JUSTIFICATION: an omitted hostPID field is Kubernetes' safe false default.
    #[serde(default, rename = "hostPID")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-PID finding.
    pub(crate) host_pid: bool,
    // DEFAULT-JUSTIFICATION: an omitted hostIPC field is Kubernetes' safe false default.
    #[serde(default, rename = "hostIPC")]
    // BRAND-INVARIANT: this raw transport flag is emitted only as the host-IPC finding.
    pub(crate) host_ipc: bool,
    // DEFAULT-JUSTIFICATION: workloads without containers have no container security context to check.
    #[serde(default)]
    pub(crate) containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: workloads without init containers require no init-container findings.
    #[serde(default, rename = "initContainers")]
    pub(crate) init_containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: workloads without ephemeral containers have no debug-container security context to check.
    #[serde(default, rename = "ephemeralContainers")]
    pub(crate) ephemeral_containers: Vec<Container>,
    // DEFAULT-JUSTIFICATION: an absent pod context intentionally delegates to container-level checks.
    #[serde(default, rename = "securityContext")]
    pub(crate) security_context: Option<SecurityContext>,
    /// Present on Deployment/DaemonSet/StatefulSet/ReplicaSet/Job — the pod
    /// template whose `.spec` is the real pod spec.
    // DEFAULT-JUSTIFICATION: a bare Pod has no template and is validated through its direct spec.
    #[serde(default)]
    pub(crate) template: Option<Box<PodTemplate>>,
    /// Present on CronJob — the Job template ultimately carries the pod
    /// template whose spec must receive the same restricted-profile checks.
    // DEFAULT-JUSTIFICATION: non-CronJob workloads have no job template.
    #[serde(default, rename = "jobTemplate")]
    pub(crate) job_template: Option<Box<JobTemplate>>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct PodTemplate {
    // DEFAULT-JUSTIFICATION: an incomplete template has no pod spec to inspect.
    #[serde(default)]
    pub(crate) spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct JobTemplate {
    // DEFAULT-JUSTIFICATION: an incomplete job template has no pod template to inspect.
    #[serde(default)]
    pub(crate) spec: Option<PodSpec>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Container {
    // DEFAULT-JUSTIFICATION: an omitted container name is reported as <unnamed> without blocking validation.
    #[serde(default)]
    // BRAND-INVARIANT: this transport name is used solely to identify a finding to the manifest author.
    pub(crate) name: String,
    // DEFAULT-JUSTIFICATION: an absent container context inherits applicable pod context or triggers a finding.
    #[serde(default, rename = "securityContext")]
    pub(crate) security_context: Option<SecurityContext>,
    // DEFAULT-JUSTIFICATION: an absent ports list cannot bind a host port.
    #[serde(default)]
    pub(crate) ports: Vec<Port>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Port {
    // DEFAULT-JUSTIFICATION: an omitted hostPort cannot expose a node port.
    #[serde(default, rename = "hostPort")]
    // BRAND-INVARIANT: the raw port number is reduced to presence because any host port is unsafe here.
    pub(crate) host_port: Option<i64>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct SecurityContext {
    // DEFAULT-JUSTIFICATION: absence is distinct from false because Kubernetes defaults privileged to false.
    #[serde(default)]
    // BRAND-INVARIANT: this raw tri-state preserves Kubernetes omission semantics for policy evaluation.
    pub(crate) privileged: Option<bool>,
    // DEFAULT-JUSTIFICATION: absence is unsafe because Kubernetes defaults privilege escalation to true.
    #[serde(default, rename = "allowPrivilegeEscalation")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit-false enforcement requirement.
    pub(crate) allow_privilege_escalation: Option<bool>,
    // DEFAULT-JUSTIFICATION: an omitted UID is evaluated with runAsNonRoot rather than coerced.
    #[serde(default, rename = "runAsUser")]
    // BRAND-INVARIANT: this raw UID is checked only for root (zero) execution.
    pub(crate) run_as_user: Option<i64>,
    // DEFAULT-JUSTIFICATION: absence must remain distinct from true for the restricted profile requirement.
    #[serde(default, rename = "runAsNonRoot")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit run-as-non-root requirement.
    pub(crate) run_as_non_root: Option<bool>,
    // DEFAULT-JUSTIFICATION: absence must remain distinct from true for the restricted profile requirement.
    #[serde(default, rename = "readOnlyRootFilesystem")]
    // BRAND-INVARIANT: this raw tri-state preserves the explicit read-only-root requirement.
    pub(crate) read_only_root_filesystem: Option<bool>,
    // DEFAULT-JUSTIFICATION: an absent capabilities block is reported as missing drop ALL.
    #[serde(default)]
    pub(crate) capabilities: Option<Capabilities>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Capabilities {
    // DEFAULT-JUSTIFICATION: an absent add list grants no additional capabilities.
    #[serde(default)]
    // BRAND-INVARIANT: raw capability names are compared case-insensitively to the restricted allowlist.
    pub(crate) add: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent drop list must produce the missing drop ALL finding.
    #[serde(default)]
    // BRAND-INVARIANT: raw capability names are compared case-insensitively for the ALL drop.
    pub(crate) drop: Vec<String>,
}

pub(crate) fn parse_manifest(source: &str) -> Option<Manifest> {
    serde_yaml::from_str(source).ok()
}

pub(crate) fn pod_spec(manifest: &Manifest) -> Option<&PodSpec> {
    let spec = manifest.spec.as_ref()?;
    if manifest.kind.as_deref() == Some("CronJob") {
        return spec
            .job_template
            .as_ref()
            .and_then(|job| job.spec.as_ref())
            .and_then(|job_spec| job_spec.template.as_ref())
            .and_then(|template| template.spec.as_ref());
    }
    match spec
        .template
        .as_ref()
        .and_then(|template| template.spec.as_ref())
    {
        Some(template_spec) => Some(template_spec),
        None => Some(spec),
    }
}

pub(crate) fn run_as_non_root_ok(
    pod_context: Option<&SecurityContext>,
    container_context: Option<&SecurityContext>,
) -> bool {
    container_context
        .and_then(|context| context.run_as_non_root)
        .or_else(|| pod_context.and_then(|context| context.run_as_non_root))
        == Some(true)
}

pub(crate) fn effective_run_as_user(
    pod_context: Option<&SecurityContext>,
    container_context: Option<&SecurityContext>,
) -> Option<i64> {
    container_context
        .and_then(|context| context.run_as_user)
        .or_else(|| pod_context.and_then(|context| context.run_as_user))
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_workload_manifest_is_rejected() {
        assert!(super::parse_manifest("spec: [").is_none());
    }
}
