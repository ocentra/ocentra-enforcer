use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_security::rules::boundary::BoundaryValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
    Ok(path.parse()?)
}

#[test]
fn detects_unauthenticated_internal_route_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let validator = BoundaryValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir(),
        "tests/fixtures/money_critical_mechanics/boundary/bad/unauthed_internal.ts",
        "tests/fixtures/money_critical_mechanics/boundary/good/authed_internal.ts",
    )?;
    Ok(())
}

#[test]
fn detects_trusted_internal_header_without_verification() -> Result<(), Box<dyn std::error::Error>> {
    let validator = BoundaryValidator::new()?;
    let file = rel("src/svc.ts")?;
    let findings = validator.validate(ValidationInput {
        file: &file,
        source: "if (req.headers['x-internal'] === 'true') { grantAccess(); }\n",
        scope: ScanScope::Files,
    });

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].title, "internal header trusted without verification (T1)");
    Ok(())
}

#[test]
fn ignores_source_without_internal_boundary_signal() -> Result<(), Box<dyn std::error::Error>> {
    let validator = BoundaryValidator::new()?;
    let file = rel("src/util.ts")?;
    let findings = validator.validate(ValidationInput {
        file: &file,
        source: "function add(a: number, b: number) { return a + b; }\n",
        scope: ScanScope::Files,
    });

    assert!(findings.is_empty());
    Ok(())
}
