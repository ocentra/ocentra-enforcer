//! External fixture-parity coverage for the Kubernetes pod-security validator.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::k8s_pod_security::K8sPodSecurityValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

#[test]
fn k8s_pod_security_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = K8sPodSecurityValidator::new()?;
    run_fixture_parity(
        &validator,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "tests/fixtures/cyberskills/k8s.pod.security-hardening/bad/privileged.yaml",
        "tests/fixtures/cyberskills/k8s.pod.security-hardening/good/hardened.yaml",
    )?;
    Ok(())
}

#[test]
fn privileged_ephemeral_container_is_checked_like_other_containers(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
apiVersion: v1
kind: Pod
metadata:
  name: debug-target
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
  containers:
  - name: app
    securityContext:
      privileged: false
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop: ["ALL"]
  ephemeralContainers:
  - name: debugger
    securityContext:
      privileged: true
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop: ["ALL"]
"#;
    let file: RelPath = "ephemeral-container.yaml".parse()?;
    let findings = K8sPodSecurityValidator::new()?.validate(ValidationInput {
        file: &file,
        source,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].detail,
        "container `debugger` runs `privileged: true` (full host access). Fix: set `securityContext.privileged: false`."
    );
    Ok(())
}
