//! External fixture-parity coverage for the command-injection validator.

use std::num::NonZeroU32;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_lang_security::rules::cyberskills::cmd_injection::CommandInjectionValidator;
mod support;
use enforcer_validator::validator::{ValidationInput, Validator};
use support::assert_fixture_parity;

#[test]
fn command_injection_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = CommandInjectionValidator::new()?;
    assert_fixture_parity(
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
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            "subprocess.run(command, shell=1)\n",
        ),
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(
        findings[0].line.source_line(),
        Some(SourceLine::try_new(NonZeroU32::MIN))
    );
    Ok(())
}
