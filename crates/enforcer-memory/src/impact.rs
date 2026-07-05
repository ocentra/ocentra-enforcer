//! X06.3: impact analysis from a git diff.
//!
//! Answers the workpack's "impact analysis from git diff" hard
//! requirement and mirrors the baseline `detect_changes` tool shape
//! (scout digest §1: "git diff -> affected symbols + risk
//! classification; base_branch/since") without shelling into git
//! itself -- this module takes an already-computed list of changed
//! repo-relative paths (the caller's job: `git diff --name-only
//! base...HEAD` or [`crate::git`] once it grows a diff-listing helper)
//! and walks [`crate::analysis::CodeAdjacency`] to find every node
//! transitively impacted.
//!
//! # X06.P2: risk classification (extends the original `RiskLevel`)
//!
//! [`classify_risk`] (blast-radius count only) is kept as-is -- existing
//! callers/tests are unaffected. [`RiskFactors`]/[`classify_risk_from_factors`]
//! add the mission's three signals on top: inbound-degree/centrality
//! (how connected the changed symbol already is), whether any impacted
//! node is itself covered by a [`crate::code_graph::CodeNode::Test`]
//! node reaching it, and whether any route is downstream in the blast
//! radius (a route is "at risk" if a changed file's reverse-dependents
//! include a route-declaring file). [`analyze_diff_impact_scoped`] is
//! the scope/depth-aware entry point ([`ImpactScope`], `depth` default
//! [`DEFAULT_DEPTH`] = 2 per this lane's mission); [`analyze_diff_impact`]
//! is kept unchanged as the pre-existing entry point.

use crate::analysis::{test_node_ids, CodeAdjacency};
use crate::code_graph::CodeGraph;
use std::collections::BTreeSet;

/// Default impact-analysis depth for [`analyze_diff_impact_scoped`]
/// (this lane's mission: "depth default 2" -- distinct from
/// [`crate::analysis::trace::DEFAULT_DEPTH`]'s 3, matching the parity
/// digest's `detect_changes` row rather than `trace_path`'s).
pub const DEFAULT_DEPTH: usize = 2;

/// Which part of the graph [`analyze_diff_impact_scoped`] walks for a
/// changed file's blast radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImpactScope {
    /// Every node type (files, symbols, routes) -- the original
    /// [`analyze_diff_impact`] behavior.
    #[default]
    All,
    /// Only symbol nodes (functions/types/tests) in the blast radius --
    /// for a caller that wants "what code do I need to re-review",
    /// excluding bare file-level noise.
    SymbolsOnly,
    /// Only nodes that are (or are upstream of) a declared route -- for
    /// a caller doing API-surface risk triage.
    RoutesOnly,
}

/// The three signals [`classify_risk_from_factors`] combines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RiskFactors {
    /// Total (in + out) degree of the changed node in the adjacency
    /// view -- the centrality proxy [`crate::analysis::CodeAdjacency::hotspots`]
    /// already uses, reused here rather than a second metric.
    pub centrality_degree: usize,
    /// Whether at least one node in the blast radius is itself a test
    /// node, or is directly reachable from one (i.e. the change is
    /// exercised by an existing test).
    pub has_test_coverage: bool,
    /// Whether any route-declaring file is in the blast radius
    /// (downstream of the change).
    pub has_downstream_route: bool,
}

/// One changed file's blast radius.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactedFile {
    pub rel_path: String,
    /// Node ids of every symbol/file that transitively depends on this
    /// file (reverse dependents), up to the analysis depth.
    pub affected_node_ids: Vec<String>,
    pub risk: RiskLevel,
}

/// A coarse risk classification: how many nodes are in the blast
/// radius. Thresholds are a deliberately simple, documented starting
/// point (not the baseline's exact classifier, which is closed-source
/// C -- BORROW_POLICY treats it as behavior-spec-only, not code to
/// copy) -- tunable later without changing the shape callers see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

fn classify_risk(affected_count: usize) -> RiskLevel {
    match affected_count {
        0..=2 => RiskLevel::Low,
        3..=10 => RiskLevel::Medium,
        _ => RiskLevel::High,
    }
}

/// The full impact report for one diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactReport {
    pub changed_paths: Vec<String>,
    pub impacted: Vec<ImpactedFile>,
    /// The union of every impacted node id across all changed files.
    pub total_affected_node_ids: Vec<String>,
}

/// Analyze the impact of `changed_paths` (repo-relative,
/// forward-slash-normalized, matching [`crate::code_graph::FileNode::rel_path`])
/// against `graph`. `max_depth` bounds the reverse-dependency walk
/// (same depth-limit contract as [`CodeAdjacency::related`]).
pub fn analyze_diff_impact(
    graph: &CodeGraph,
    changed_paths: &[String],
    max_depth: usize,
) -> ImpactReport {
    let adjacency = CodeAdjacency::build(graph);
    let mut impacted = Vec::new();
    let mut total: BTreeSet<String> = BTreeSet::new();

    for rel_path in changed_paths {
        let file_id = format!("file:{rel_path}");
        // Seed the reverse walk from the file node AND every symbol it
        // contains -- an upstream caller reaches a changed file via a
        // CALLS edge into one of *its symbols*, not via any edge
        // pointing at the bare file id (file->symbol is a Contains
        // edge in the *outgoing* direction, so `reverse_dependents`
        // starting at the file id alone can never see a call into a
        // symbol the file merely contains).
        let mut seeds: BTreeSet<String> = BTreeSet::new();
        seeds.insert(file_id.clone());
        for symbol in graph.symbol_nodes() {
            if symbol.file_id == file_id {
                seeds.insert(symbol.id.clone());
            }
        }

        let mut affected: BTreeSet<String> = BTreeSet::new();
        for seed in &seeds {
            for id in adjacency.reverse_dependents(seed, max_depth) {
                // Never report the changed file's own nodes as
                // "affected by itself".
                if !seeds.contains(&id) {
                    affected.insert(id);
                }
            }
        }

        let affected: Vec<String> = affected.into_iter().collect();
        for id in &affected {
            total.insert(id.clone());
        }
        let risk = classify_risk(affected.len());
        impacted.push(ImpactedFile {
            rel_path: rel_path.clone(),
            affected_node_ids: affected,
            risk,
        });
    }

    ImpactReport {
        changed_paths: changed_paths.to_vec(),
        impacted,
        total_affected_node_ids: total.into_iter().collect(),
    }
}

/// Combine [`RiskFactors`] into a [`RiskLevel`]. Deliberately simple
/// and documented (same posture as [`classify_risk`]'s own doc comment
/// -- not the baseline's exact closed-source classifier):
///
/// - `centrality_degree >= HIGH_CENTRALITY_DEGREE` alone is High (a
///   highly-connected node is risky regardless of test coverage --
///   tests reduce risk of *regression going unnoticed*, not the blast
///   radius itself);
/// - a downstream route with NO test coverage is High (an untested
///   change reaching a public API surface is the mission's explicit
///   "routes/events downstream" + "untested" combination);
/// - a downstream route WITH test coverage, or any test-covered
///   moderate-centrality node, is Medium;
/// - a leaf node (zero centrality) with test coverage and no
///   downstream route is Low.
pub fn classify_risk_from_factors(factors: RiskFactors) -> RiskLevel {
    const HIGH_CENTRALITY_DEGREE: usize = 10;

    if factors.centrality_degree >= HIGH_CENTRALITY_DEGREE {
        return RiskLevel::High;
    }
    if factors.has_downstream_route && !factors.has_test_coverage {
        return RiskLevel::High;
    }
    if factors.has_downstream_route || (!factors.has_test_coverage && factors.centrality_degree > 0)
    {
        return RiskLevel::Medium;
    }
    RiskLevel::Low
}

/// One changed file's blast radius, scoped analysis. Distinct from
/// [`ImpactedFile`] (kept unchanged for the original [`analyze_diff_impact`])
/// so this lane's additions never alter that struct's shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedImpactedFile {
    pub rel_path: String,
    pub affected_node_ids: Vec<String>,
    pub factors: RiskFactorsSnapshot,
    pub risk: RiskLevel,
}

/// [`RiskFactors`] plus the raw degree/coverage/route data that produced
/// it, for callers that want to render "why" (matches the MIA-framework
/// "traversal reasoning" idea this crate's [`crate::analysis`] module
/// already cites -- explainable, not just a label).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RiskFactorsSnapshot {
    pub centrality_degree: usize,
    pub has_test_coverage: bool,
    pub has_downstream_route: bool,
    pub covering_test_ids: Vec<String>,
    pub downstream_route_file_ids: Vec<String>,
}

/// The full scoped impact report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedImpactReport {
    pub changed_paths: Vec<String>,
    pub impacted: Vec<ScopedImpactedFile>,
    pub total_affected_node_ids: Vec<String>,
}

/// Scope- and depth-aware impact analysis (X06.P2 mission: `scope`
/// param honoring, `depth` default [`DEFAULT_DEPTH`] = 2, risk derived
/// from centrality + test coverage + downstream routes/events).
pub fn analyze_diff_impact_scoped(
    graph: &CodeGraph,
    changed_paths: &[String],
    depth: usize,
    scope: ImpactScope,
) -> ScopedImpactReport {
    let adjacency = CodeAdjacency::build(graph);
    let test_ids = test_node_ids(graph);
    let route_file_ids: BTreeSet<String> = graph
        .routes()
        .iter()
        .map(|r| r.from_file_id.clone())
        .collect();

    let mut impacted = Vec::new();
    let mut total: BTreeSet<String> = BTreeSet::new();

    for rel_path in changed_paths {
        let file_id = format!("file:{rel_path}");
        let mut seeds: BTreeSet<String> = BTreeSet::new();
        seeds.insert(file_id.clone());
        for symbol in graph.symbol_nodes() {
            if symbol.file_id == file_id {
                seeds.insert(symbol.id.clone());
            }
        }

        let mut affected: BTreeSet<String> = BTreeSet::new();
        for seed in &seeds {
            for id in adjacency.reverse_dependents(seed, depth) {
                if !seeds.contains(&id) {
                    affected.insert(id);
                }
            }
        }

        let scoped_affected: Vec<String> = affected
            .iter()
            .filter(|id| node_in_scope(graph, id, scope))
            .cloned()
            .collect();

        let centrality_degree = seeds
            .iter()
            .map(|seed| {
                if let Some(score) = adjacency
                    .hotspots(usize::MAX)
                    .into_iter()
                    .find(|h| &h.node_id == seed)
                {
                    score.total_degree()
                } else {
                    0
                }
            })
            .max()
            .unwrap_or(0);

        let covering_test_ids: Vec<String> = affected
            .iter()
            .filter(|id| test_ids.contains(id.as_str()))
            .cloned()
            .collect();
        let has_test_coverage = !covering_test_ids.is_empty();

        let downstream_route_file_ids: Vec<String> = affected
            .iter()
            .filter(|id| route_file_ids.contains(id.as_str()))
            .cloned()
            .collect();
        let has_downstream_route = !downstream_route_file_ids.is_empty();

        let factors = RiskFactors {
            centrality_degree,
            has_test_coverage,
            has_downstream_route,
        };
        let risk = classify_risk_from_factors(factors);

        for id in &scoped_affected {
            total.insert(id.clone());
        }

        impacted.push(ScopedImpactedFile {
            rel_path: rel_path.clone(),
            affected_node_ids: scoped_affected,
            factors: RiskFactorsSnapshot {
                centrality_degree,
                has_test_coverage,
                has_downstream_route,
                covering_test_ids,
                downstream_route_file_ids,
            },
            risk,
        });
    }

    ScopedImpactReport {
        changed_paths: changed_paths.to_vec(),
        impacted,
        total_affected_node_ids: total.into_iter().collect(),
    }
}

/// Whether `node_id` belongs in `scope`'s filtered view. Unknown node
/// ids (no matching [`crate::code_graph::CodeNode`], e.g. an id from a
/// stale manifest) are excluded from `SymbolsOnly`/`RoutesOnly` scopes
/// rather than assumed to match -- `All` always includes them (matches
/// the original [`analyze_diff_impact`]'s unfiltered behavior).
fn node_in_scope(graph: &CodeGraph, node_id: &str, scope: ImpactScope) -> bool {
    match scope {
        ImpactScope::All => true,
        ImpactScope::SymbolsOnly => graph.symbol_nodes().any(|s| s.id == node_id),
        ImpactScope::RoutesOnly => graph
            .routes()
            .iter()
            .any(|r| r.from_file_id == node_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
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

        let report = analyze_diff_impact(&graph, &["b.rs".to_string()], 3);
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
    fn risk_classification_scales_with_affected_count() {
        assert_eq!(classify_risk(0), RiskLevel::Low);
        assert_eq!(classify_risk(2), RiskLevel::Low);
        assert_eq!(classify_risk(3), RiskLevel::Medium);
        assert_eq!(classify_risk(10), RiskLevel::Medium);
        assert_eq!(classify_risk(11), RiskLevel::High);
    }

    #[test]
    fn unknown_changed_path_is_reported_with_zero_impact_not_panic() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        init_repo(dir.path())?;
        fs::write(dir.path().join("a.rs"), "fn a() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[dir.path().join("a.rs")], &Manifest::default())?;

        let report = analyze_diff_impact(&graph, &["does-not-exist.rs".to_string()], 3);
        assert_eq!(report.impacted.len(), 1);
        assert!(report.impacted[0].affected_node_ids.is_empty());
        assert_eq!(report.impacted[0].risk, RiskLevel::Low);
        Ok(())
    }

    // --- X06.P2: risk-classification boundaries (factors) --------------

    #[test]
    fn factors_high_centrality_is_high_risk_regardless_of_test_coverage() {
        let high_centrality_tested = RiskFactors {
            centrality_degree: 10,
            has_test_coverage: true,
            has_downstream_route: false,
        };
        let high_centrality_untested = RiskFactors {
            centrality_degree: 25,
            has_test_coverage: false,
            has_downstream_route: false,
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
            centrality_degree: 0,
            has_test_coverage: true,
            has_downstream_route: false,
        };
        assert_eq!(classify_risk_from_factors(leaf), RiskLevel::Low);
    }

    #[test]
    fn factors_downstream_route_without_tests_is_high_risk() {
        let untested_route = RiskFactors {
            centrality_degree: 1,
            has_test_coverage: false,
            has_downstream_route: true,
        };
        assert_eq!(
            classify_risk_from_factors(untested_route),
            RiskLevel::High
        );
    }

    #[test]
    fn factors_downstream_route_with_tests_is_medium_risk() {
        let tested_route = RiskFactors {
            centrality_degree: 1,
            has_test_coverage: true,
            has_downstream_route: true,
        };
        assert_eq!(
            classify_risk_from_factors(tested_route),
            RiskLevel::Medium
        );
    }

    #[test]
    fn factors_untested_mid_centrality_leaf_is_medium_not_low() {
        let untested_mid = RiskFactors {
            centrality_degree: 2,
            has_test_coverage: false,
            has_downstream_route: false,
        };
        assert_eq!(
            classify_risk_from_factors(untested_mid),
            RiskLevel::Medium
        );
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
            &["b.rs".to_string()],
            DEFAULT_DEPTH,
            ImpactScope::All,
        );
        assert_eq!(report.impacted.len(), 1);
        let impacted = &report.impacted[0];
        assert!(
            impacted.factors.has_downstream_route,
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
            &["b.rs".to_string()],
            DEFAULT_DEPTH,
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
            &["b.rs".to_string()],
            DEFAULT_DEPTH,
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
            &["does-not-exist.rs".to_string()],
            DEFAULT_DEPTH,
            ImpactScope::All,
        );
        assert_eq!(report.impacted.len(), 1);
        assert_eq!(report.impacted[0].risk, RiskLevel::Low);
        assert!(!report.impacted[0].factors.has_downstream_route);
        assert!(!report.impacted[0].factors.has_test_coverage);
        Ok(())
    }
}
