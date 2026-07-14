//! External fixture-parity coverage for the GCP resource hardening validator.

use std::path::PathBuf;

use enforcer_lang_security::rules::cyberskills::cloud_gcp::GcpResourceHardeningValidator;
use enforcer_validator::harness::run_fixture_parity;

#[test]
fn cloud_gcp_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = GcpResourceHardeningValidator::new()?;
    run_fixture_parity(
        &validator,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "tests/fixtures/cyberskills/cloud.gcp.resource-hardening/bad/public.tf",
        "tests/fixtures/cyberskills/cloud.gcp.resource-hardening/good/hardened.tf",
    )?;
    Ok(())
}
