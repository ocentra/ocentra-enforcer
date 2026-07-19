//! Kubernetes RBAC manifest decoding boundary.
//!
//! BOUNDARY-INVARIANT: malformed YAML is rejected before any grant, role, or
//! subject value reaches RBAC policy evaluation.

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Manifest {
    // DEFAULT-JUSTIFICATION: a document without kind is outside this RBAC-only rule.
    #[serde(default)]
    // BRAND-INVARIANT: raw manifest kind gates all subsequent RBAC inspection.
    pub(crate) kind: Option<String>,
    // DEFAULT-JUSTIFICATION: bindings do not have rules and an absent rules list grants nothing.
    #[serde(default)]
    pub(crate) rules: Vec<PolicyRule>,
    // DEFAULT-JUSTIFICATION: roles do not have roleRef and cannot bind cluster-admin.
    #[serde(default, rename = "roleRef")]
    pub(crate) role_ref: Option<RoleRef>,
    // DEFAULT-JUSTIFICATION: roles have no subjects and an absent binding subject grants nothing.
    #[serde(default)]
    pub(crate) subjects: Vec<Subject>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct PolicyRule {
    // DEFAULT-JUSTIFICATION: an absent verb list grants no operation.
    #[serde(default)]
    // BRAND-INVARIANT: raw verbs are compared only with the narrow hazardous-verb set.
    pub(crate) verbs: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent resource list grants no resource access.
    #[serde(default)]
    // BRAND-INVARIANT: raw resources are compared only with wildcard and secret resource names.
    pub(crate) resources: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent apiGroups list cannot grant a wildcard group.
    #[serde(default, rename = "apiGroups")]
    // BRAND-INVARIANT: raw API groups are compared only for a wildcard grant.
    pub(crate) api_groups: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent non-resource URL list grants no API-path access.
    #[serde(default, rename = "nonResourceURLs")]
    // BRAND-INVARIANT: raw API paths are compared only for wildcard access.
    pub(crate) non_resource_urls: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct RoleRef {
    // DEFAULT-JUSTIFICATION: missing kind is conventionally ClusterRole for a ClusterRoleBinding.
    #[serde(default)]
    // BRAND-INVARIANT: this raw reference kind is used only to qualify cluster-admin bindings.
    pub(crate) kind: Option<String>,
    // DEFAULT-JUSTIFICATION: missing name cannot identify the cluster-admin role.
    #[serde(default)]
    // BRAND-INVARIANT: this raw role name is compared only with the canonical cluster-admin name.
    pub(crate) name: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Subject {
    // DEFAULT-JUSTIFICATION: an omitted kind cannot identify a privileged group subject.
    #[serde(default)]
    // BRAND-INVARIANT: only the Kubernetes Group kind can be system:masters.
    pub(crate) kind: Option<String>,
    // DEFAULT-JUSTIFICATION: an omitted name cannot grant system:masters membership.
    #[serde(default)]
    // BRAND-INVARIANT: compared only with the built-in superuser group name.
    pub(crate) name: Option<String>,
}

pub(crate) fn parse_manifest(source: &str) -> Option<Manifest> {
    serde_yaml::from_str(source).ok()
}

pub(crate) fn has(list: &[String], value: &str) -> bool {
    list.iter().any(|candidate| candidate == value)
}

pub(crate) fn any_matches(list: &[String], candidates: &[&str]) -> bool {
    list.iter().any(|value| {
        candidates
            .iter()
            .any(|candidate| value.eq_ignore_ascii_case(candidate))
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_rbac_manifest_is_rejected() {
        assert!(super::parse_manifest("rules: [").is_none());
    }
}
