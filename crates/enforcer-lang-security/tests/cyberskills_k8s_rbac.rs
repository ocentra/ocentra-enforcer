//! External fixture-parity coverage for the Kubernetes RBAC validator.

use std::path::PathBuf;

use enforcer_lang_security::rules::cyberskills::k8s_rbac::K8sRbacValidator;
use enforcer_validator::harness::run_fixture_parity;

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
