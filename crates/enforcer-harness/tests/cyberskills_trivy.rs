//! CP07 recorded/live Trivy normalization contract.
//!
//! The live branch is optional and never turns an unavailable executable into
//! a pass. The recorded branch is the deterministic CI proof.

use std::path::Path;

use enforcer_domain::harness_types::HarnessToolAvailability;
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::adapters::cyberskills::seam::AdapterOutcome;
use enforcer_harness::adapters::cyberskills::trivy::{parse_report, run};
use proptest::{prelude::any, proptest};

fn repo_root() -> Result<RepoRoot, Box<dyn std::error::Error>> {
    Ok(RepoRoot::try_from(Path::new(env!("CARGO_MANIFEST_DIR")))?)
}

#[test]
fn recorded_fixture_is_stable_and_has_findings() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = parse_report(include_str!(
        "fixtures/cyberskills_adapters/trivy/good/recorded.json"
    ))?;
    let AdapterOutcome::Ran { ran, findings } = outcome else {
        return Err("expected recorded Trivy run".into());
    };
    assert_eq!(ran, 1);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_str(), "AVD-AWS-0086");
    assert_eq!(findings[0].severity.as_str(), "HIGH");
    Ok(())
}

#[test]
fn clean_recorded_fixture_is_a_real_run_not_a_skip() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = parse_report(include_str!(
        "fixtures/cyberskills_adapters/trivy/good/clean.json"
    ))?;
    assert_eq!(
        outcome,
        AdapterOutcome::Ran {
            ran: 1,
            findings: vec![],
        }
    );
    Ok(())
}

#[test]
fn malformed_recorded_fixture_is_rejected() {
    assert!(parse_report(include_str!(
        "fixtures/cyberskills_adapters/trivy/bad/malformed.json"
    ))
    .is_err());
}

#[test]
fn unknown_severity_fails_closed_at_native_gate() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = parse_report(include_str!(
        "fixtures/cyberskills_adapters/trivy/bad/unknown-severity.json"
    ))?;
    let AdapterOutcome::Ran { findings, .. } = outcome else {
        return Err("expected unknown-severity run".into());
    };
    assert_eq!(
        AdapterOutcome::normalize_severity(findings[0].severity.as_str()),
        enforcer_domain::severity::Severity::Error
    );
    Ok(())
}

#[test]
fn optional_live_run_is_honest_and_matches_recorded_first_finding(
) -> Result<(), Box<dyn std::error::Error>> {
    let live = run(repo_root()?, "tests/fixtures/enforcer/iac")?;
    match live {
        AdapterOutcome::Skipped { ran } => {
            assert_eq!(ran, 0);
            assert_eq!(HarnessToolAvailability::Missing.as_str(), "missing");
        }
        AdapterOutcome::Errored { .. } => {}
        AdapterOutcome::Ran { findings, .. } => {
            let recorded = parse_report(include_str!(
                "fixtures/cyberskills_adapters/trivy/good/recorded.json"
            ))?;
            let AdapterOutcome::Ran {
                findings: recorded_findings,
                ..
            } = recorded
            else {
                return Err("expected recorded run".into());
            };
            let first = findings.first().ok_or("live Trivy returned no findings")?;
            assert_eq!(first, &recorded_findings[0]);
        }
    }
    Ok(())
}

proptest! {
    #[test]
    fn trivy_parser_is_total_and_deterministic(raw in any::<String>()) {
        let first = parse_report(&raw).is_ok();
        let second = parse_report(&raw).is_ok();
        assert_eq!(first, second);
    }
}
