//! External fixture-parity coverage for the command-injection validator.

use std::path::PathBuf;

use enforcer_lang_security::rules::cyberskills::cmd_injection::CommandInjectionValidator;
use enforcer_validator::harness::run_fixture_parity;

#[test]
fn command_injection_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = CommandInjectionValidator::new()?;
    run_fixture_parity(
        &validator,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "tests/fixtures/cyberskills/web.command-injection/bad/inject.py",
        "tests/fixtures/cyberskills/web.command-injection/good/safe.py",
    )?;
    Ok(())
}
