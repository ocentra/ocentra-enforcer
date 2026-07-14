//! External fixture-parity coverage for the Kubernetes pod-security validator.

use std::path::PathBuf;

use enforcer_lang_security::rules::cyberskills::k8s_pod_security::K8sPodSecurityValidator;
use enforcer_validator::harness::run_fixture_parity;

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
