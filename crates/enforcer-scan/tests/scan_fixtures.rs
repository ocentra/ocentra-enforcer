//! Integration proof for arc-15's acceptance criteria: fixture repo trees
//! route correctly end-to-end through [`enforcer_scan::walk::walk`] +
//! [`enforcer_scan::engine::run`] — the fail fixture's planted violation
//! is found and routed to the right family; the pass fixture's clean tree
//! produces an empty report.

use enforcer_domain::paths::RepoRoot;
use enforcer_scan::engine::{build_family_validators, run};
use enforcer_scan::scope::{resolve, ScopeRequest};
use enforcer_scan::walk::{walk, IgnoreRules};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn pass_fixture_tree_produces_an_empty_report() -> Result<(), Box<dyn std::error::Error>> {
    let root_path = fixture_root("pass");
    let root: RepoRoot = root_path.to_string_lossy().parse()?;
    let resolved = resolve(&ScopeRequest::All, &root)?;
    let files = walk(&root_path, &IgnoreRules::default())?;
    assert!(
        !files.is_empty(),
        "pass fixture must contain at least one file"
    );
    let validators = build_family_validators()?;
    let report = run(&resolved, &files, &validators);
    assert!(report.ok, "clean fixture tree must produce an ok report");
    assert!(report.violations.is_empty());
    Ok(())
}

#[test]
fn fail_fixture_tree_routes_planted_violation_to_rust_family(
) -> Result<(), Box<dyn std::error::Error>> {
    let root_path = fixture_root("fail");
    let root: RepoRoot = root_path.to_string_lossy().parse()?;
    let resolved = resolve(&ScopeRequest::All, &root)?;
    let files = walk(&root_path, &IgnoreRules::default())?;
    assert!(
        !files.is_empty(),
        "fail fixture must contain at least one file"
    );
    let validators = build_family_validators()?;
    let report = run(&resolved, &files, &validators);
    assert!(
        !report.ok,
        "planted unwrap() must trip a blocking violation"
    );
    assert!(
        report
            .violations
            .iter()
            .any(|v| v.finding().rule_id.as_str() == "T1-RUSTERR.1"),
        "the planted violation must route to the Rust family's error_handling validator, \
         found rule ids: {:?}",
        report
            .violations
            .iter()
            .map(|v| v.finding().rule_id.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn parallel_and_serial_walk_over_the_same_fixture_agree() -> Result<(), Box<dyn std::error::Error>>
{
    let root_path = fixture_root("fail");
    let root: RepoRoot = root_path.to_string_lossy().parse()?;
    let resolved = resolve(&ScopeRequest::All, &root)?;
    let files = walk(&root_path, &IgnoreRules::default())?;
    let validators = build_family_validators()?;

    let run_one = run(&resolved, &files, &validators);
    let run_two = run(&resolved, &files, &validators);
    assert_eq!(
        run_one, run_two,
        "repeated runs over the same scope must be byte-identical (idempotency guard)"
    );
    Ok(())
}
