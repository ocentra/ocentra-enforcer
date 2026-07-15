//! Proof row `scan-modes-select` (f01,
//! `docs/plans/enforcer-selfhost-plan/TEST_PROOF_EXPECTATIONS.md`):
//!
//! - a `full`-only violation seeded OUTSIDE the scoped path ->
//!   `scoped`/`quick` must NOT report it (scope honored); the same
//!   violation INSIDE scope -> `scoped` reports it and `full` always
//!   reports it.
//! - an invalid mode string is rejected at the serde boundary
//!   (non-zero/error), never silently defaulted.
//! - `scoped` is the no-arg default.

use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::ScopeRequest;
use enforcer_domain::severity::Tier;
use enforcer_scan::engine::{build_family_validators, run};
use enforcer_scan::modes::{ScanMode, ScanModeError, ScanRequest, TierFilter};
use enforcer_scan::scope::resolve;
use enforcer_scan::walk::{walk, IgnoreRules};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/modes")
        .join(name)
}

/// Rule-tier classifier for this test: both wired Rust validators
/// (`T1-NOREEXPORT.1`, `T1-RUSTERR.1`, see `engine::build_family_validators`)
/// carry a literal `T1-` prefix, matching `enforcer-rules`' real registry
/// tagging for these same rule ids (arc-04). `modes.rs`'s production code
/// takes no dependency on a classifier — this is the shape a future
/// MCP/CLI caller (or a real `enforcer-rules` registry lookup, already a
/// dev-dependency of this crate) would supply.
fn classify_tier(rule_id: &str) -> Tier {
    if rule_id.starts_with("T1-") {
        Tier::T1
    } else {
        Tier::T2
    }
}

/// Walk + run the fan-out engine rooted at `walk_root`, then apply
/// `tier_filter` to the resulting violations the way a mode-aware caller
/// would: filtering the fan-out's findings down to the requested tier
/// subset.
fn scan_with_mode(
    walk_root: &std::path::Path,
    tier_filter: &TierFilter,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let root: RepoRoot = walk_root.to_string_lossy().parse()?;
    let resolved = resolve(&ScopeRequest::All, &root)?;
    let files = walk(walk_root, &IgnoreRules::default())?;
    let validators = build_family_validators()?;
    let report = run(&resolved, &files, &validators);
    let rule_ids: Vec<String> = report
        .violations
        .iter()
        .map(|v| v.finding().rule_id.as_str().to_owned())
        .filter(|rule_id| tier_filter.allows(classify_tier(rule_id)))
        .collect();
    Ok(rule_ids)
}

#[test]
fn scoped_mode_does_not_report_a_violation_outside_the_scoped_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = fixture_root("violation_outside_scope");
    let request = ScanRequest {
        mode: ScanMode::Scoped,
        scope: Some("crates/scoped_crate".to_owned()),
        ..ScanRequest::default()
    };
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    // Narrow the physical walk root to the resolved scope path — the
    // scoped/quick narrowing this workpack proves.
    let scoped_root = repo_root.join("crates/scoped_crate");
    let found = scan_with_mode(&scoped_root, &resolved.tier_filter)?;
    assert!(
        found.is_empty(),
        "scoped mode must not see the violation planted outside its scope, found: {found:?}"
    );
    Ok(())
}

#[test]
fn quick_mode_does_not_report_a_violation_outside_the_scoped_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = fixture_root("violation_outside_scope");
    let request = ScanRequest {
        mode: ScanMode::Quick,
        scope: Some("crates/scoped_crate".to_owned()),
        ..ScanRequest::default()
    };
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    let scoped_root = repo_root.join("crates/scoped_crate");
    let found = scan_with_mode(&scoped_root, &resolved.tier_filter)?;
    assert!(
        found.is_empty(),
        "quick mode must not see the violation planted outside its scope, found: {found:?}"
    );
    Ok(())
}

#[test]
fn full_mode_always_reports_the_violation_regardless_of_where_it_lives(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = fixture_root("violation_outside_scope");
    let request = ScanRequest {
        mode: ScanMode::Full,
        ..ScanRequest::default()
    };
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    // `full` resolves to `ScopeRequest::All`: walk the whole fixture repo.
    let found = scan_with_mode(&repo_root, &resolved.tier_filter)?;
    assert!(
        found.iter().any(|id| id == "T1-RUSTERR.1"),
        "full mode must always report the violation, found: {found:?}"
    );
    Ok(())
}

#[test]
fn scoped_mode_reports_the_same_violation_when_it_lives_inside_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = fixture_root("violation_inside_scope");
    let request = ScanRequest {
        mode: ScanMode::Scoped,
        scope: Some("crates/scoped_crate".to_owned()),
        ..ScanRequest::default()
    };
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    let scoped_root = repo_root.join("crates/scoped_crate");
    let found = scan_with_mode(&scoped_root, &resolved.tier_filter)?;
    assert!(
        found.iter().any(|id| id == "T1-RUSTERR.1"),
        "scoped mode must report a violation planted inside its own scope, found: {found:?}"
    );
    Ok(())
}

#[test]
fn full_mode_reports_the_same_violation_inside_scope_too() -> Result<(), Box<dyn std::error::Error>>
{
    let repo_root = fixture_root("violation_inside_scope");
    let request = ScanRequest {
        mode: ScanMode::Full,
        ..ScanRequest::default()
    };
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    let found = scan_with_mode(&repo_root, &resolved.tier_filter)?;
    assert!(
        found.iter().any(|id| id == "T1-RUSTERR.1"),
        "full mode must report the violation, found: {found:?}"
    );
    Ok(())
}

#[test]
fn invalid_mode_string_is_rejected_at_the_serde_boundary() {
    let outcome: Result<ScanMode, _> = serde_json::from_str("\"not-a-real-mode\"");
    assert!(
        outcome.is_err(),
        "a malformed mode string must be a typed decode error, never a silent default"
    );
}

#[test]
fn invalid_mode_string_is_rejected_by_from_str_too() {
    let outcome = "not-a-real-mode".parse::<ScanMode>();
    assert!(matches!(outcome, Err(ScanModeError::UnknownMode { .. })));
}

#[test]
fn no_arg_request_defaults_to_scoped_and_resolves_to_paths_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = fixture_root("violation_outside_scope");
    let request = ScanRequest::default();
    assert_eq!(request.mode, ScanMode::Scoped);
    let resolved = request.resolve(
        &repo_root.to_string_lossy().parse()?,
        &"crates/scoped_crate".parse()?,
    )?;
    assert_eq!(resolved.mode, ScanMode::Scoped);
    assert!(
        matches!(resolved.scope_request, ScopeRequest::Paths(_)),
        "no-arg default must resolve to a narrow Paths scope, never ScopeRequest::All"
    );
    assert_ne!(
        resolved.scope_request,
        ScopeRequest::All,
        "the no-arg default must never resolve to whole-repo"
    );
    Ok(())
}

#[test]
fn each_named_mode_resolves_to_its_expected_scope_and_tier_shape(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root: RepoRoot = fixture_root("violation_outside_scope")
        .to_string_lossy()
        .parse()?;
    let cwd = "crates/scoped_crate".parse()?;

    let quick = ScanRequest {
        mode: ScanMode::Quick,
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert!(matches!(quick.scope_request, ScopeRequest::Paths(_)));
    assert_eq!(quick.tier_filter, TierFilter::Only(vec![Tier::T1]));

    let full = ScanRequest {
        mode: ScanMode::Full,
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert_eq!(full.scope_request, ScopeRequest::All);
    assert_eq!(full.tier_filter, TierFilter::All);

    let repo = ScanRequest {
        mode: ScanMode::Repo,
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert_eq!(repo.scope_request, ScopeRequest::All);

    let workspace = ScanRequest {
        mode: ScanMode::Workspace,
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert_eq!(workspace.scope_request, ScopeRequest::All);

    let scoped = ScanRequest {
        mode: ScanMode::Scoped,
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert!(matches!(scoped.scope_request, ScopeRequest::Paths(_)));

    let diff = ScanRequest {
        mode: ScanMode::Diff,
        base: Some("main".to_owned()),
        head: Some("HEAD".to_owned()),
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert!(matches!(diff.scope_request, ScopeRequest::Diff { .. }));

    let plan_scan = ScanRequest {
        mode: ScanMode::PlanScan,
        scope: Some("docs/plans/enforcer-selfhost-plan".to_owned()),
        ..ScanRequest::default()
    }
    .resolve(&repo_root, &cwd)?;
    assert!(matches!(plan_scan.scope_request, ScopeRequest::Paths(_)));

    Ok(())
}

#[test]
fn scan_mode_and_request_round_trip_through_the_external_wire_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = ScanMode::Quick;
    let mode_wire = serde_json::to_string(&mode)?;
    assert_eq!(mode_wire, r#"{"kind":"quick"}"#);
    assert_eq!(serde_json::from_str::<ScanMode>(&mode_wire)?, mode);

    let request = ScanRequest {
        mode: ScanMode::Scoped,
        scope: Some("crates/enforcer-scan".to_owned()),
        base: None,
        head: None,
    };
    let request_wire = serde_json::to_string(&request)?;
    assert_eq!(serde_json::from_str::<ScanRequest>(&request_wire)?, request);
    Ok(())
}
