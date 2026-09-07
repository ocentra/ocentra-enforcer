//! BOUNDARY-INVARIANT: this module decodes only supplied container-security
//! evidence and exposes typed values to the native predicate layer.
//! NEGATIVE-TEST: malformed JSON and malformed audit JSONL are rejected by
//! the focused CP09 B01 integration test.
//!
//! No Kubernetes API, runtime, syscall stream, Falco process, registry, image
//! store, or production authority is read from this boundary.

/// A supplied, versioned container-security evidence envelope.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(crate) struct ContainerSecurityManifest {
    // DEFAULT-JUSTIFICATION: version one is the only accepted wire contract.
    #[serde(default = "default_schema_version", rename = "schemaVersion")]
    pub(crate) schema_version: u32,
    // DEFAULT-JUSTIFICATION: callers may supply normalized events or JSONL evidence.
    #[serde(default, rename = "auditEvents")]
    pub(crate) audit_events: Vec<AuditEvent>,
    // DEFAULT-JUSTIFICATION: an absent audit stream means no supplied audit facts.
    #[serde(default, rename = "auditJsonl")]
    pub(crate) audit_jsonl: String,
    // DEFAULT-JUSTIFICATION: an absent approved set provides no drift baseline.
    #[serde(default, rename = "approvedContainers")]
    pub(crate) approved_containers: Vec<ContainerSnapshot>,
    // DEFAULT-JUSTIFICATION: an absent observed set provides no runtime comparison fact.
    #[serde(default, rename = "observedContainers")]
    pub(crate) observed_containers: Vec<ContainerSnapshot>,
    // DEFAULT-JUSTIFICATION: an absent pod set provides no supplied escape-risk fact.
    #[serde(default, rename = "podSnapshots")]
    pub(crate) pod_snapshots: Vec<PodSnapshot>,
}

/// A normalized Kubernetes audit event supplied by the caller.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(crate) struct AuditEvent {
    // DEFAULT-JUSTIFICATION: missing values cannot satisfy a sensitive-event predicate.
    #[serde(default)]
    pub(crate) verb: String,
    // DEFAULT-JUSTIFICATION: missing values cannot identify a sensitive resource.
    #[serde(default)]
    pub(crate) resource: String,
    // DEFAULT-JUSTIFICATION: namespace is retained as supplied evidence context.
    #[serde(default)]
    pub(crate) namespace: String,
    // DEFAULT-JUSTIFICATION: user is retained as supplied evidence context.
    #[serde(default)]
    pub(crate) user: String,
    // DEFAULT-JUSTIFICATION: request URI is retained as supplied evidence context.
    #[serde(default, rename = "requestUri")]
    pub(crate) request_uri: String,
}

/// A supplied approved or observed container snapshot used for deterministic drift comparison.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(crate) struct ContainerSnapshot {
    // DEFAULT-JUSTIFICATION: unnamed snapshots cannot be joined across the two supplied sets.
    #[serde(default)]
    pub(crate) name: String,
    // DEFAULT-JUSTIFICATION: an absent digest is distinct from a changed digest.
    #[serde(default, rename = "imageDigest")]
    pub(crate) image_digest: Option<String>,
    // DEFAULT-JUSTIFICATION: an absent executable list means no executable facts were supplied.
    #[serde(default)]
    pub(crate) executables: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent package list means no package facts were supplied.
    #[serde(default)]
    pub(crate) packages: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent filesystem list means no filesystem facts were supplied.
    #[serde(default)]
    pub(crate) filesystem: Vec<String>,
}

/// A supplied pod security-context snapshot used only for static escape-risk predicates.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(crate) struct PodSnapshot {
    // DEFAULT-JUSTIFICATION: the name identifies the supplied snapshot in a finding.
    #[serde(default)]
    pub(crate) name: String,
    // DEFAULT-JUSTIFICATION: absent booleans preserve the Kubernetes-safe false interpretation.
    #[serde(default)]
    pub(crate) privileged: bool,
    // DEFAULT-JUSTIFICATION: absent booleans preserve the supplied snapshot's default state.
    #[serde(default, rename = "hostPID")]
    pub(crate) host_pid: bool,
    // DEFAULT-JUSTIFICATION: absent booleans preserve the supplied snapshot's default state.
    #[serde(default, rename = "hostNetwork")]
    pub(crate) host_network: bool,
    // DEFAULT-JUSTIFICATION: absent booleans preserve the supplied snapshot's default state.
    #[serde(default, rename = "hostIPC")]
    pub(crate) host_ipc: bool,
    // DEFAULT-JUSTIFICATION: an absent mount list contains no supplied host-path fact.
    #[serde(default, rename = "hostPaths")]
    pub(crate) host_paths: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent capability list contains no supplied capability fact.
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
    // DEFAULT-JUSTIFICATION: absence remains distinct from an explicitly enabled escalation.
    #[serde(default, rename = "allowPrivilegeEscalation")]
    pub(crate) allow_privilege_escalation: Option<bool>,
}

fn default_schema_version() -> u32 {
    1
}

/// Decode the supplied JSON envelope without consulting any external authority.
pub(crate) fn parse(source: &str) -> Result<ContainerSecurityManifest, serde_json::Error> {
    serde_json::from_str(source)
}

/// Decode both normalized audit events and newline-delimited audit evidence.
pub(crate) fn audit_events(
    manifest: &ContainerSecurityManifest,
) -> Result<Vec<AuditEvent>, serde_json::Error> {
    let mut events = manifest.audit_events.clone();
    for line in manifest
        .audit_jsonl
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        events.push(serde_json::from_str(line)?);
    }
    Ok(events)
}

/// Classify a supplied audit event when its fields match a sensitive operation.
pub(crate) fn audit_reason(event: &AuditEvent) -> Option<&'static str> {
    let verb = event.verb.as_str();
    let resource = event.resource.as_str();
    [
        (
            matches!(resource, "pods/exec" | "pods/attach")
                && matches!(verb, "create" | "get" | "connect"),
            "pod exec/attach operation",
        ),
        (
            resource.eq_ignore_ascii_case("secrets") && matches!(verb, "get" | "list" | "watch"),
            "secret-read operation",
        ),
        (
            matches!(
                resource,
                "roles" | "clusterroles" | "rolebindings" | "clusterrolebindings"
            ) && matches!(verb, "create" | "update" | "patch"),
            "RBAC mutation operation",
        ),
    ]
    .into_iter()
    .find_map(|(matches, reason)| matches.then_some(reason))
}

fn same_values(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

/// Return the supplied container fields that differ from the approved snapshot.
pub(crate) fn drift_fields(
    approved: &ContainerSnapshot,
    observed: &ContainerSnapshot,
) -> Vec<&'static str> {
    [
        (
            "imageDigest",
            approved.image_digest != observed.image_digest,
        ),
        (
            "executables",
            !same_values(&approved.executables, &observed.executables),
        ),
        (
            "packages",
            !same_values(&approved.packages, &observed.packages),
        ),
        (
            "filesystem",
            !same_values(&approved.filesystem, &observed.filesystem),
        ),
    ]
    .into_iter()
    .filter_map(|(field, changed)| changed.then_some(field))
    .collect()
}

/// Return static escape-risk indicators present in a supplied pod snapshot.
pub(crate) fn escape_indicators(snapshot: &PodSnapshot) -> Vec<&'static str> {
    [
        ("privileged", snapshot.privileged),
        ("hostPID", snapshot.host_pid),
        ("hostNetwork", snapshot.host_network),
        ("hostIPC", snapshot.host_ipc),
        ("hostPaths", !snapshot.host_paths.is_empty()),
        (
            "dangerousCapabilities",
            snapshot.capabilities.iter().any(|capability| {
                matches!(
                    capability.as_str(),
                    "SYS_ADMIN" | "SYS_PTRACE" | "NET_ADMIN"
                )
            }),
        ),
        (
            "allowPrivilegeEscalation",
            snapshot.allow_privilege_escalation == Some(true),
        ),
    ]
    .into_iter()
    .filter_map(|(indicator, present)| present.then_some(indicator))
    .collect()
}
