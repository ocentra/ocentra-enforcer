//! External fixture-parity coverage for the Kubernetes RBAC validator.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_lang_security::rules::cyberskills::k8s_rbac::K8sRbacValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

#[test]
fn k8s_rbac_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = K8sRbacValidator::new()?;
    run_fixture_parity(
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
        source: "apiVersion: rbac.authorization.k8s.io/v1\nkind: ClusterRole\nrules:\n- nonResourceURLs: [\"*\"]\n  verbs: [\"*\"]\n",
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(
        findings[0].detail,
        "ClusterRole rule grants a wildcard permission (verbs: [\"*\"], resources: [], apiGroups: [], nonResourceURLs: [\"*\"]). Fix: replace `*` with the specific verbs, resources, API groups, and non-resource URLs actually required."
    );
    Ok(())
}
