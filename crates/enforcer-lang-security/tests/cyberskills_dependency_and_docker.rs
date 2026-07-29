//! Fixture-parity coverage for the dependency-confusion and Docker-daemon
//! CyberSkills validators. These integration tests exercise only their public
//! validator APIs, keeping test-only harness concerns outside rule source.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::dependency_confusion::DependencyConfusionClaimableValidator;
use enforcer_lang_security::rules::cyberskills::docker_daemon::DockerDaemonHardeningValidator;
mod support;
use enforcer_validator::validator::{ValidationInput, Validator};
use support::assert_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn dependency_confusion_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DependencyConfusionClaimableValidator::new()?;
    assert_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/bad/package.json",
        "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/good/package.json",
    )?;
    Ok(())
}

#[test]
fn dependency_confusion_reports_an_internal_npm_alias_target_once(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = DependencyConfusionClaimableValidator::new()?;
    let file: RelPath = "package.json".parse()?;
    let findings = validator.validate(ValidationInput {
        file: &file,
        source: enforcer_domain::boundary::validation::ValidationSource::from_text(
            r#"{
            "dependencies": {
                "internal-api": "^1.0.0",
                "public-alias": "npm:internal-api@^2.0.0"
            }
        }"#,
        ),
        scope: ScanScope::Files,
    });

    assert_eq!(
        findings.len(),
        1,
        "one resolved package should yield one finding"
    );
    assert_eq!(
        findings[0].detail.as_str(),
        "dependency `internal-api` is unscoped and matches an internal-looking naming \
         convention, so it is a CANDIDATE for a dependency-confusion takeover. This is a \
         naming heuristic, not a registry-verified verdict (see h12 for the registry-probe \
         adapter). Fix: publish it under an org scope (`@your-org/internal-api`), or confirm \
         the public-registry name is claimed/reserved by your org."
    );
    Ok(())
}

#[test]
fn docker_daemon_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerDaemonHardeningValidator::new()?;
    assert_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/cyberskills/container.docker-daemon/bad/daemon.json",
        "tests/fixtures/cyberskills/container.docker-daemon/good/daemon.json",
    )?;
    Ok(())
}
