//! Integration-level detection test for `LIT-2.1` (the universal
//! literal-scan T2 advisory bridge), exercised through the crate's public
//! surface only (`enforcer_literal_scan::LiteralScanBridgeValidator`),
//! mirroring the same `cargo test -p enforcer-literal-scan` entry point
//! named in `TEST_PROOF_EXPECTATIONS.md` (`literal-scan-universal-threshold`,
//! `literal-scan-graceful-skip`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{ScanScope, Violation};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_literal_scan::bridge::LiteralScanBridgeValidator;
use enforcer_validator::harness::run_fixture_parity as assert_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_root() -> Result<RepoRoot, DecodeError> {
    RepoRoot::try_from(env!("CARGO_MANIFEST_DIR").to_owned())
}

/// Named proof row `literal-scan-universal-threshold`: a high-literal-risk
/// Dart source crosses the advisory score threshold and yields a T2
/// finding; a clean equivalent stays under threshold and yields none.
#[test]
fn dart_threshold_crossing_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = LiteralScanBridgeValidator::new()?;
    let root = manifest_root()?;
    let fail: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
    let pass: RelPath = "tests/fixtures/universal/pass/dart-clean.dart".parse()?;
    assert_fixture_parity(&validator, &root, &fail, &pass)?;
    Ok(())
}

/// Same threshold-crossing proof for the newly-registered CFML
/// (`.cfm`/`.cfc`) entry — proves `.dart`/`.cfc`/`.cfm` are recognized by
/// the language registry and covered by the universal floor.
#[test]
fn cfml_threshold_crossing_parity() -> Result<(), Box<dyn std::error::Error>> {
    let validator = LiteralScanBridgeValidator::new()?;
    let root = manifest_root()?;
    let fail: RelPath = "tests/fixtures/universal/fail/cfml-secret.cfm".parse()?;
    let pass: RelPath = "tests/fixtures/universal/pass/cfml-clean.cfm".parse()?;
    assert_fixture_parity(&validator, &root, &fail, &pass)?;
    Ok(())
}

/// Named proof row `literal-scan-graceful-skip` (the workpack's
/// "advisory-nonblocking" requirement): an advisory finding never
/// promotes to a blocking `Violation`, and a target the registry cannot
/// classify degrades to silence (graceful skip) rather than erroring —
/// either way the run stays exit-0-capable from this validator's output
/// alone.
#[test]
fn advisory_findings_are_nonblocking_and_unknown_targets_skip_gracefully(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = LiteralScanBridgeValidator::new()?;
    let root = manifest_root()?;

    let fail_file: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
    let fail_source = std::fs::read_to_string(root.resolve(&fail_file))?;
    let findings = validator.validate(ValidationInput {
        file: &fail_file,
        source: ValidationSource::from_text(&fail_source),
        scope: ScanScope::Files,
    });
    assert!(!findings.is_empty(), "fail fixture must cross threshold");
    for finding in findings {
        assert_ne!(finding.severity, Severity::Error);
        let error = match Violation::try_from(finding) {
            Ok(_) => return Err("advisory finding converted into a blocking violation".into()),
            Err(error) => error,
        };
        assert_eq!(error.path, "violation.severity");
        assert_eq!(error.reason, "a violation must carry severity `error`");
    }

    let unknown_file: RelPath = "tests/fixtures/universal/fail/no-extension".parse()?;
    let graceful = validator.validate(ValidationInput {
        file: &unknown_file,
        source: ValidationSource::from_text("arbitrary text, no registered language for this path"),
        scope: ScanScope::Files,
    });
    assert!(graceful.is_empty(), "unrecognized target must skip cleanly");

    Ok(())
}
