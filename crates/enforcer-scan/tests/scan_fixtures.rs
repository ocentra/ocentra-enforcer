//! Integration proof for arc-15's acceptance criteria: fixture repo trees
//! route correctly end-to-end through [`enforcer_scan::walk::walk`] +
//! [`enforcer_scan::engine::run`] — the fail fixture's planted violation
//! is found and routed to the right family; the pass fixture's clean tree
//! produces an empty report.

use enforcer_domain::config_types::InlineTestPolicy;
use enforcer_domain::findings::{Finding, Report};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::ScopeRequest;
use enforcer_domain::severity::Severity;
use enforcer_scan::engine::{build_family_validators, run, run_with_inline_test_policy};
use enforcer_scan::scope::resolve;
use enforcer_scan::walk::{walk, IgnoreRules};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn inline_test_findings(report: &Report) -> Vec<&Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.rule_id.as_str() == "TEST-2.2")
        .collect()
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

#[test]
fn inline_test_policy_is_configurable_and_external_tests_are_exempt(
) -> Result<(), Box<dyn std::error::Error>> {
    let root_path = fixture_root("inline_test_policy");
    let root: RepoRoot = root_path.to_string_lossy().parse()?;
    let resolved = resolve(&ScopeRequest::All, &root)?;
    let files = walk(&root_path, &IgnoreRules::default())?;
    let validators = build_family_validators()?;

    let forbid =
        run_with_inline_test_policy(&resolved, &files, &validators, InlineTestPolicy::Forbid);
    let warn = run_with_inline_test_policy(&resolved, &files, &validators, InlineTestPolicy::Warn);
    let allow =
        run_with_inline_test_policy(&resolved, &files, &validators, InlineTestPolicy::Allow);

    let forbid_findings = inline_test_findings(&forbid);
    assert_eq!(forbid_findings.len(), 1);
    assert_eq!(forbid_findings[0].severity, Severity::Error);
    assert!(!forbid.ok, "forbid must make inline tests blocking");
    assert_eq!(forbid_findings[0].file.as_str(), "src/lib.rs");

    let warn_findings = inline_test_findings(&warn);
    assert_eq!(warn_findings.len(), 1);
    assert_eq!(warn_findings[0].severity, Severity::Warning);
    assert!(warn.ok, "warn must remain advisory-only");

    assert!(
        inline_test_findings(&allow).is_empty(),
        "allow must emit no TEST-2.2 finding"
    );
    assert!(
        forbid_findings
            .iter()
            .all(|finding| !finding.file.as_str().starts_with("tests/")),
        "tests/ files are organized external tests and must never be reported"
    );
    Ok(())
}
