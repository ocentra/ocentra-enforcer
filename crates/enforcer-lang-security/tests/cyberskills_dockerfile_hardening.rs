//! Public-API coverage for the Dockerfile hardening CyberSkills validator.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::dockerfile_hardening::DockerfileHardeningValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn dockerfile_hardening_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerfileHardeningValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/cyberskills/container.dockerfile-hardening/bad/Dockerfile",
        "tests/fixtures/cyberskills/container.dockerfile-hardening/good/Dockerfile",
    )?;
    Ok(())
}

#[test]
fn platform_qualified_from_uses_the_actual_image_reference(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerfileHardeningValidator::new()?;
    let file: RelPath = "Dockerfile".parse()?;
    let source = "FROM --platform=$BUILDPLATFORM rust:1.88 AS builder\nUSER 10001\nFROM builder AS runtime\nUSER 10001\n";
    let findings = validator.validate(ValidationInput {
        source,
        file: &file,
        scope: ScanScope::Files,
    });

    assert!(
        findings.is_empty(),
        "a platform-qualified, tagged build stage should not be treated as an untagged image: {findings:?}"
    );
    Ok(())
}

#[test]
fn json_array_add_with_remote_url_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerfileHardeningValidator::new()?;
    let file: RelPath = "Dockerfile".parse()?;
    let findings = validator.validate(ValidationInput {
        source: "FROM alpine:3.20\nADD [\"https://example.com/tool\", \"/usr/local/bin/tool\"]\nUSER 10001\n",
        file: &file,
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 2);
    assert_eq!(
        findings[0].severity,
        enforcer_domain::severity::Severity::Error
    );
    Ok(())
}
