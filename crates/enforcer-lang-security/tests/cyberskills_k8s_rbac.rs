//! External fixture-parity coverage for the Kubernetes RBAC validator.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_lang_security::rules::cyberskills::k8s_rbac::K8sRbacValidator;
mod support;
use enforcer_validator::validator::{ValidationInput, Validator};
use support::assert_fixture_parity;

#[test]
fn k8s_rbac_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = K8sRbacValidator::new()?;
    assert_fixture_parity(
        &validator,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "tests/fixtures/cyberskills/k8s.rbac-privilege-escalation/bad/wildcard.yaml",
        "tests/fixtures/cyberskills/k8s.rbac-privilege-escalation/good/scoped.yaml",
    )?;
    Ok(())
}

#[test]
fn cluster_role_wildcard_non_resource_url_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let validator = K8sRbacValidator::new()?;
    let file: RelPath = "cluster-role.yaml".parse()?;
    let findings = validator.validate(ValidationInput {
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            "apiVersion: rbac.authorization.k8s.io/v1\nkind: ClusterRole\nrules:\n- nonResourceURLs: [\"*\"]\n  verbs: [\"*\"]\n",
        ),
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(
        findings[0].detail.as_str(),
        "ClusterRole rule grants a wildcard permission (verbs: [\"*\"], resources: [], apiGroups: [], nonResourceURLs: [\"*\"]). Fix: replace `*` with the specific verbs, resources, API groups, and non-resource URLs actually required."
    );
    Ok(())
}

#[test]
fn binding_system_masters_group_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let validator = K8sRbacValidator::new()?;
    let file: RelPath = "cluster-role-binding.yaml".parse()?;
    let findings = validator.validate(ValidationInput {
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            "apiVersion: rbac.authorization.k8s.io/v1\nkind: ClusterRoleBinding\nroleRef:\n  apiGroup: rbac.authorization.k8s.io\n  kind: ClusterRole\n  name: view\nsubjects:\n- kind: Group\n  name: system:masters\n",
        ),
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(
        findings[0].detail.as_str(),
        "ClusterRoleBinding binds a subject to the built-in `system:masters` group, whose members bypass normal RBAC authorization. Fix: remove this subject and bind named identities to a narrowly scoped Role or ClusterRole instead."
    );
    Ok(())
}
