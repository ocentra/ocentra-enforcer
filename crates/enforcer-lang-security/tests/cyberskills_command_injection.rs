//! External fixture-parity coverage for the command-injection validator.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_lang_security::rules::cyberskills::cmd_injection::CommandInjectionValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

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

#[test]
fn subprocess_truthy_numeric_shell_value_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let validator = CommandInjectionValidator::new()?;
    let file: RelPath = "command.py".parse()?;
    let findings = validator.validate(ValidationInput {
        source: "subprocess.run(command, shell=1)\n",
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(findings[0].line, 1);
    Ok(())
}
