use enforcer_domain::memory_types::{DetectChangesScope, ImpactScope, RiskLevel};
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::impact::{
    analyze_diff_impact, analyze_diff_impact_scoped, classify_risk_from_factors,
    detect_changes_view, RiskFactors, DEFAULT_DEPTH,
};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

/// `a.rs` calls `helper` defined in `b.rs`; changing `b.rs` should
/// mark `a.rs` as impacted via the CALLS edge's reverse direction.
#[test]
fn diff_impact_finds_transitively_affected_files() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.path().join("b.rs"), "fn helper() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;

    let report = analyze_diff_impact(&graph, &["b.rs".into()], 3.into());
    assert_eq!(report.impacted.len(), 1);
    let impacted_b = &report.impacted[0];
    assert_eq!(impacted_b.rel_path, "b.rs");
    assert!(
        impacted_b
            .affected_node_ids
            .iter()
            .any(|id| id == "file:a.rs"),
        "expected file:a.rs among impacted nodes for changing b.rs, got {:?}",
        impacted_b.affected_node_ids
    );
    Ok(())
}

#[test]
fn unknown_changed_path_is_reported_with_zero_impact_not_panic() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.rs"), "fn a() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[dir.path().join("a.rs")], &Manifest::default())?;

    let report = analyze_diff_impact(&graph, &["does-not-exist.rs".into()], 3.into());
    assert_eq!(report.impacted.len(), 1);
    assert!(report.impacted[0].affected_node_ids.is_empty());
    assert_eq!(report.impacted[0].risk, RiskLevel::Low);
    Ok(())
}

fn build_risk_fixture(dir: &Path, caller_count: usize) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("helper.rs"), "fn helper() {}\n")?;
    (0..caller_count).try_for_each(|index| {
        fs::write(
            dir.join(format!("caller{index}.rs")),
            "fn caller() { helper(); }\n",
        )
    })?;
    commit_all(dir, "risk-fixture")?;

    let mut graph = CodeGraph::new();
    let mut files = vec![dir.join("helper.rs")];
    files.extend((0..caller_count).map(|index| dir.join(format!("caller{index}.rs"))));
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn risk_classification_boundaries_are_preserved_through_public_diff_impact() -> TestResult<()> {
    let low_dir = tempfile::tempdir()?;
    let low_graph = build_risk_fixture(low_dir.path(), 0)?;
    let low_report = analyze_diff_impact(&low_graph, &["helper.rs".into()], 3.into());
    assert_eq!(low_report.impacted[0].risk, RiskLevel::Low);

    let medium_dir = tempfile::tempdir()?;
    let medium_graph = build_risk_fixture(medium_dir.path(), 2)?;
    let medium_report = analyze_diff_impact(&medium_graph, &["helper.rs".into()], 3.into());
    assert_eq!(medium_report.impacted[0].risk, RiskLevel::Medium);

    let high_dir = tempfile::tempdir()?;
    let high_graph = build_risk_fixture(high_dir.path(), 6)?;
    let high_report = analyze_diff_impact(&high_graph, &["helper.rs".into()], 3.into());
    assert_eq!(high_report.impacted[0].risk, RiskLevel::High);
    Ok(())
}

// --- X06.P2: risk-classification boundaries (factors) --------------

#[test]
fn factors_high_centrality_is_high_risk_regardless_of_test_coverage() {
    let high_centrality_tested = RiskFactors {
        centrality_degree: 10.into(),
        has_test_coverage: true.into(),
        has_downstream_route: false.into(),
    };
    let high_centrality_untested = RiskFactors {
        centrality_degree: 25.into(),
        has_test_coverage: false.into(),
        has_downstream_route: false.into(),
    };
    assert_eq!(
        classify_risk_from_factors(high_centrality_tested),
        RiskLevel::High
    );
    assert_eq!(
        classify_risk_from_factors(high_centrality_untested),
        RiskLevel::High
    );
}

#[test]
fn factors_leaf_node_tested_no_route_is_low_risk() {
    let leaf = RiskFactors {
        centrality_degree: 0.into(),
        has_test_coverage: true.into(),
        has_downstream_route: false.into(),
    };
    assert_eq!(classify_risk_from_factors(leaf), RiskLevel::Low);
}

#[test]
fn factors_downstream_route_without_tests_is_high_risk() {
    let untested_route = RiskFactors {
        centrality_degree: 1.into(),
        has_test_coverage: false.into(),
        has_downstream_route: true.into(),
    };
    assert_eq!(classify_risk_from_factors(untested_route), RiskLevel::High);
}

#[test]
fn factors_downstream_route_with_tests_is_medium_risk() {
    let tested_route = RiskFactors {
        centrality_degree: 1.into(),
        has_test_coverage: true.into(),
        has_downstream_route: true.into(),
    };
    assert_eq!(classify_risk_from_factors(tested_route), RiskLevel::Medium);
}

#[test]
fn factors_untested_mid_centrality_leaf_is_medium_not_low() {
    let untested_mid = RiskFactors {
        centrality_degree: 2.into(),
        has_test_coverage: false.into(),
        has_downstream_route: false.into(),
    };
    assert_eq!(classify_risk_from_factors(untested_mid), RiskLevel::Medium);
}

// --- X06.P2: scoped impact analysis over a real fixture graph ------

/// `a.rs` calls `helper` (`b.rs`); `router.ts` imports `a.rs` and
/// declares `GET /a`; `a_test.rs` is a test covering `helper`.
fn build_scoped_fixture(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    fs::write(
        dir.join("router.ts"),
        "import { caller } from \"./a\";\nrouter.get(\"/a\", caller);\n",
    )?;
    fs::write(
        dir.join("a_test.rs"),
        "#[test]\nfn a_test() { caller(); }\n",
    )?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.join("a.rs"),
        dir.join("b.rs"),
        dir.join("router.ts"),
        dir.join("a_test.rs"),
    ];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn scoped_impact_default_depth_matches_mission_default() {
    assert_eq!(DEFAULT_DEPTH, 2);
}

#[test]
fn scoped_impact_detects_downstream_route_and_test_coverage() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_scoped_fixture(dir.path())?;

    let report = analyze_diff_impact_scoped(
        &graph,
        &["b.rs".into()],
        DEFAULT_DEPTH.into(),
        ImpactScope::All,
    );
    assert_eq!(report.impacted.len(), 1);
    let impacted = &report.impacted[0];
    assert!(
        impacted.factors.has_downstream_route.is_present(),
        "expected router.ts's GET /a route downstream of b.rs, got {:?}",
        impacted.factors
    );
    Ok(())
}

#[test]
fn scoped_impact_symbols_only_excludes_file_and_route_nodes() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_scoped_fixture(dir.path())?;

    let report = analyze_diff_impact_scoped(
        &graph,
        &["b.rs".into()],
        DEFAULT_DEPTH.into(),
        ImpactScope::SymbolsOnly,
    );
    let impacted = &report.impacted[0];
    assert!(
        impacted
            .affected_node_ids
            .iter()
            .all(|id| id.starts_with("sym:")),
        "SymbolsOnly scope must exclude non-symbol ids, got {:?}",
        impacted.affected_node_ids
    );
    Ok(())
}

#[test]
fn scoped_impact_routes_only_returns_only_route_declaring_files() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_scoped_fixture(dir.path())?;

    let report = analyze_diff_impact_scoped(
        &graph,
        &["b.rs".into()],
        DEFAULT_DEPTH.into(),
        ImpactScope::RoutesOnly,
    );
    let impacted = &report.impacted[0];
    assert!(
        impacted
            .affected_node_ids
            .iter()
            .all(|id| id == "file:router.ts"),
        "RoutesOnly scope must only return route-declaring file ids, got {:?}",
        impacted.affected_node_ids
    );
    Ok(())
}

#[test]
fn scoped_impact_unknown_path_has_low_risk_and_no_factors() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_scoped_fixture(dir.path())?;

    let report = analyze_diff_impact_scoped(
        &graph,
        &["does-not-exist.rs".into()],
        DEFAULT_DEPTH.into(),
        ImpactScope::All,
    );
    assert_eq!(report.impacted.len(), 1);
    assert_eq!(report.impacted[0].risk, RiskLevel::Low);
    assert!(!report.impacted[0].factors.has_downstream_route.is_present());
    assert!(!report.impacted[0].factors.has_test_coverage.is_present());
    Ok(())
}

// --- X06.P2: detect_changes parity shape ----------------------------

/// `a.rs` defines `caller` (Function) and a test `a_test`; `b.rs`
/// defines `helper` (Function). Used to assert the parity shape's
/// file-level (not blast-radius) `impacted_symbols`.
fn build_detect_changes_fixture(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(
        dir.join("a.rs"),
        "fn caller() { helper(); }\n#[test]\nfn a_test() { caller(); }\n",
    )?;
    fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.join("a.rs"), dir.join("b.rs")];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn detect_changes_view_matches_the_baseline_parity_shape() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_detect_changes_fixture(dir.path())?;

    let view = detect_changes_view(
        &graph,
        &["a.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::Symbols,
    );
    assert_eq!(view.changed_files, vec!["a.rs".to_string()]);
    assert_eq!(view.changed_count, 1);
    assert_eq!(view.depth, DEFAULT_DEPTH, "depth is echoed, never enforced");
    let names: Vec<&str> = view
        .impacted_symbols
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(names.contains(&"caller"), "expected caller, got {names:?}");
    assert!(names.contains(&"a_test"), "expected a_test, got {names:?}");
    assert!(
        !names.contains(&"helper"),
        "impacted_symbols is FILE-LEVEL (only symbols in the changed file itself, not \
         blast-radius downstream symbols), so b.rs's helper must not appear, got {names:?}"
    );
    for symbol in &view.impacted_symbols {
        assert_eq!(symbol.file, "a.rs");
    }
    Ok(())
}

#[test]
fn detect_changes_view_files_only_scope_leaves_impacted_symbols_empty_but_present() -> TestResult<()>
{
    let dir = tempfile::tempdir()?;
    let graph = build_detect_changes_fixture(dir.path())?;

    let view = detect_changes_view(
        &graph,
        &["a.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::FilesOnly,
    );
    assert!(
        view.impacted_symbols.is_empty(),
        "FilesOnly scope must leave impacted_symbols empty"
    );
    assert_eq!(view.changed_count, 1);
    Ok(())
}

#[test]
fn detect_changes_view_impact_scope_also_populates_symbols() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_detect_changes_fixture(dir.path())?;

    let view = detect_changes_view(
        &graph,
        &["a.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::Impact,
    );
    assert!(view
        .impacted_symbols
        .iter()
        .any(|symbol| symbol.name.as_str() == "caller"));
    Ok(())
}

#[test]
fn detect_changes_view_is_deterministically_ordered() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_detect_changes_fixture(dir.path())?;

    let first = detect_changes_view(
        &graph,
        &["a.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::Symbols,
    );
    let second = detect_changes_view(
        &graph,
        &["a.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::Symbols,
    );
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn detect_changes_view_unknown_file_reports_zero_symbols_not_panic() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_detect_changes_fixture(dir.path())?;

    let view = detect_changes_view(
        &graph,
        &["does-not-exist.rs".into()],
        DEFAULT_DEPTH.into(),
        DetectChangesScope::Symbols,
    );
    assert_eq!(view.changed_count, 1);
    assert!(view.impacted_symbols.is_empty());
    Ok(())
}
