//! BOUNDARY-INVARIANT: this integration test exercises only supplied JSON
//! evidence and never contacts Kubernetes, a registry, a runtime, or Falco.
//! NEGATIVE-TEST: malformed envelope, sensitive audit facts, drift, and
//! escape-risk configuration each have explicit assertions.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::k8s_container_security::K8sContainerSecurityValidator;
use enforcer_lang_security::rules::cyberskills::k8s_pod_security::K8sPodSecurityValidator;
use enforcer_lang_security::rules::cyberskills::k8s_rbac::K8sRbacValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

fn fixture(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cyberskills/k8s-container-security-b01")
        .join(name);
    Ok(std::fs::read_to_string(path)?)
}

fn validate_container(
    source: &str,
    path: &str,
) -> Result<Vec<enforcer_domain::findings::Finding>, Box<dyn std::error::Error>> {
    let validator = K8sContainerSecurityValidator::new()?;
    let file: RelPath = path.parse()?;
    Ok(validator.validate(ValidationInput {
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
        file: &file,
        scope: ScanScope::Files,
    }))
}

#[test]
fn hardened_supplied_container_evidence_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let findings = validate_container(
        &fixture("pass.json")?,
        "k8s-container-security-b01/pass.json",
    )?;
    assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    Ok(())
}

#[test]
fn static_audit_drift_and_escape_facts_are_reported() -> Result<(), Box<dyn std::error::Error>> {
    let findings = validate_container(
        &fixture("fail.json")?,
        "k8s-container-security-b01/fail.json",
    )?;
    assert_eq!(findings.len(), 4, "unexpected findings: {findings:?}");
    assert!(findings
        .iter()
        .any(|finding| finding.detail.as_str().contains("pod exec/attach")));
    assert!(findings.iter().any(|finding| finding
        .detail
        .as_str()
        .contains("differs from its approved")));
    assert!(findings.iter().any(|finding| finding
        .detail
        .as_str()
        .contains("escape-risk configuration")));
    Ok(())
}

#[test]
fn malformed_envelope_is_rejected_without_external_fallback(
) -> Result<(), Box<dyn std::error::Error>> {
    let findings = validate_container(
        &fixture("malformed.json")?,
        "k8s-container-security-b01/malformed.json",
    )?;
    assert_eq!(findings.len(), 1);
    assert!(findings[0].detail.as_str().contains("could not be decoded"));
    Ok(())
}

#[test]
fn authorization_boundary_fixture_has_no_live_authority_effect(
) -> Result<(), Box<dyn std::error::Error>> {
    let findings = validate_container(
        &fixture("boundary.json")?,
        "k8s-container-security-b01/boundary.json",
    )?;
    assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    Ok(())
}

#[test]
fn existing_rbac_and_pod_validators_are_reused_for_adjacent_intents(
) -> Result<(), Box<dyn std::error::Error>> {
    let rbac_source = "kind: Role\nrules:\n- verbs: [get]\n  resources: [configmaps]\n";
    let rbac_file: RelPath = "k8s-container-security-b01/rbac.yaml".parse()?;
    let rbac_findings = K8sRbacValidator::new()?.validate(ValidationInput {
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(rbac_source),
        file: &rbac_file,
        scope: ScanScope::Files,
    });
    assert!(
        rbac_findings.is_empty(),
        "unexpected RBAC findings: {rbac_findings:?}"
    );

    let pod_source = r#"
kind: Pod
spec:
  hostNetwork: false
  hostPID: false
  hostIPC: false
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
  containers:
  - name: api
    securityContext:
      privileged: false
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop: ["ALL"]
"#;
    let pod_file: RelPath = "k8s-container-security-b01/pod.yaml".parse()?;
    let pod_findings = K8sPodSecurityValidator::new()?.validate(ValidationInput {
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(pod_source),
        file: &pod_file,
        scope: ScanScope::Files,
    });
    assert!(
        pod_findings.is_empty(),
        "unexpected pod findings: {pod_findings:?}"
    );
    Ok(())
}
