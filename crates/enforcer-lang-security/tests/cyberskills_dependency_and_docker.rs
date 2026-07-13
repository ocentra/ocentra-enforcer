//! Fixture-parity coverage for the dependency-confusion and Docker-daemon
//! CyberSkills validators. These integration tests exercise only their public
//! validator APIs, keeping test-only harness concerns outside rule source.

use std::path::PathBuf;

use enforcer_lang_security::rules::cyberskills::dependency_confusion::DependencyConfusionClaimableValidator;
use enforcer_lang_security::rules::cyberskills::docker_daemon::DockerDaemonHardeningValidator;
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn dependency_confusion_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DependencyConfusionClaimableValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/bad/package.json",
        "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/good/package.json",
    )?;
    Ok(())
}

#[test]
fn docker_daemon_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = DockerDaemonHardeningValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/cyberskills/container.docker-daemon/bad/daemon.json",
        "tests/fixtures/cyberskills/container.docker-daemon/good/daemon.json",
    )?;
    Ok(())
}
