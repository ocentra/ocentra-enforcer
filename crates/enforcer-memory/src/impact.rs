//! X06.3: impact analysis from a git diff.
//!
//! Answers the workpack's "impact analysis from git diff" hard
//! requirement -- this module takes an already-computed list of
//! changed repo-relative paths (the caller's job: `git diff
//! --name-only base...HEAD` or [`crate::git`] once it grows a
//! diff-listing helper) and walks [`crate::analysis::CodeAdjacency`] to
//! find every node transitively impacted.
//!
//! # X06.P2: risk classification is an ENFORCER EXTENSION, not baseline parity
//!
//! Baseline-source-verified correction (orchestrator, post-scout-digest
//! extraction of the actual C source; see
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §13.3): the baseline's `detect_changes` carries NO risk concept
//! whatsoever -- its response is exactly `{changed_files,
//! changed_count, impacted_symbols, depth}` with `impacted_symbols`
//! being FILE-LEVEL (every symbol in a changed file, not a downstream
//! blast-radius walk) and `depth` parsed but unused (§13.4). The
//! baseline's ONLY risk labels live in `trace_path` behind a
//! `risk_labels` flag (pure BFS hop-distance; see
//! [`crate::analysis::trace`]'s module docs). [`RiskFactors`] /
//! [`classify_risk_from_factors`] / [`analyze_diff_impact_scoped`]
//! below therefore have **no baseline counterpart to match** -- they
//! are a documented enforcer-native extension (centrality/degree, test
//! coverage, downstream routes), kept because the mission calls for
//! richer risk signal than the baseline offers, not because parity
//! requires it. [`classify_risk`] (blast-radius count only) is kept
//! as-is -- existing callers/tests are unaffected. [`detect_changes_view`]
//! is the PARITY-SHAPED response builder: exactly the baseline's four
//! fields, file-level `impacted_symbols`, `depth` echoed not enforced --
//! it never exposes `risk`/`RiskFactors` inline; a caller that wants
//! both attaches [`analyze_diff_impact_scoped`]'s richer report
//! alongside, never merged into the parity fields.

use crate::analysis::{test_node_ids, CodeAdjacency};
use crate::code_graph::{CodeGraph, CodeNode};
use std::collections::{BTreeSet, HashMap};

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

/// Combine [`RiskFactors`] into a [`RiskLevel`]. Deliberately simple
/// and documented (same posture as [`classify_risk`]'s own doc comment
/// -- not the baseline's exact closed-source classifier, which does
/// not exist for this tool at all, see module docs):
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
/// [`ImpactedFile`] (kept unchanged for the original
/// [`analyze_diff_impact`]) so this lane's additions never alter that
/// struct's shape.
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
/// from centrality + test coverage + downstream routes/events). This
/// is an ENFORCER EXTENSION -- see module docs -- never fed into
/// [`detect_changes_view`]'s parity-shaped response.
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
    // Computed once for the whole call (not once per seed per changed
    // path): `hotspots` scores every node in the graph, so re-deriving
    // it inside the per-seed loop below would be O(changed_paths *
    // seeds * V log V) for no benefit -- the scores are a pure function
    // of `graph` and never change within this call.
    let degree_by_node: HashMap<String, usize> = adjacency
        .hotspots(usize::MAX)
        .into_iter()
        .map(|h| {
            let degree = h.total_degree();
            (h.node_id, degree)
        })
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
            .map(|seed| degree_by_node.get(seed).copied().unwrap_or(0))
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
        ImpactScope::RoutesOnly => graph.routes().iter().any(|r| r.from_file_id == node_id),
    }
}

/// One entry in [`DetectChangesView::impacted_symbols`]: the baseline's
/// exact per-symbol shape (`{name, label, file}`, §13.5 of the baseline
/// tool-schemas ref) -- `label` is `"Function"`/`"Type"`/`"Test"`
/// mirroring [`CodeNode`]'s own variant names (the baseline's `label`
/// values are its own KG's node-label strings; enforcer's closest
/// honest equivalent is its own [`CodeNode`] variant name, not a
/// fabricated mapping to the baseline's exact label vocabulary, which
/// this crate's graph model does not otherwise track).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedSymbol {
    pub name: String,
    pub label: String,
    pub file: String,
}

/// The baseline-parity `detect_changes` response shape (§13.5): exactly
/// `{changed_files, changed_count, impacted_symbols, depth}`, nothing
/// more. `impacted_symbols` is FILE-LEVEL -- every symbol defined in a
/// changed file, per §13.4's "no BFS/graph-traversal step at all in
/// this handler" finding -- never a downstream blast-radius walk (that
/// richer analysis is [`analyze_diff_impact_scoped`], an explicitly
/// separate, non-parity extension). `depth` is echoed back exactly as
/// passed in, never used to bound anything, matching the baseline's own
/// documented "dead/cosmetic parameter" behavior (§13.4) -- this
/// module does not "fix" that by making `depth` do something the
/// baseline never did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectChangesView {
    pub changed_files: Vec<String>,
    pub changed_count: usize,
    pub impacted_symbols: Vec<ChangedSymbol>,
    pub depth: usize,
}

/// Whether [`DetectChangesView::impacted_symbols`] should be populated
/// (baseline's `want_symbols` gate, mcp.c:5224/5352-5354): `"symbols"`
/// or `"impact"` include it; any other scope value (including an
/// unrecognized string) leaves it empty -- the key is always present,
/// per §13.5's "the key itself is not omitted" note, so this only gates
/// population, not presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectChangesScope {
    Symbols,
    Impact,
    FilesOnly,
}

impl DetectChangesScope {
    fn wants_symbols(self) -> bool {
        matches!(self, Self::Symbols | Self::Impact)
    }
}

/// Build the baseline-parity `detect_changes` response over an
/// already-computed list of `changed_files` (repo-relative,
/// forward-slash-normalized paths -- the caller's job to produce via
/// `git diff`/`git status`, per §13.2's three-merged-git-sources
/// mechanism; this library layer does not shell out to git itself, same
/// posture as [`analyze_diff_impact`]). `depth` is echoed verbatim into
/// the response and never used to bound traversal (see
/// [`DetectChangesView`] docs); `scope` gates whether
/// `impacted_symbols` is populated.
pub fn detect_changes_view(
    graph: &CodeGraph,
    changed_files: &[String],
    depth: usize,
    scope: DetectChangesScope,
) -> DetectChangesView {
    let mut impacted_symbols = Vec::new();
    if scope.wants_symbols() {
        for file in changed_files {
            let file_id = format!("file:{file}");
            for node in graph.nodes() {
                let symbol = match node {
                    CodeNode::Function(s) if s.file_id == file_id => Some((s, "Function")),
                    CodeNode::Type(s) if s.file_id == file_id => Some((s, "Type")),
                    CodeNode::Test(s) if s.file_id == file_id => Some((s, "Test")),
                    _ => None,
                };
                if let Some((symbol, label)) = symbol {
                    impacted_symbols.push(ChangedSymbol {
                        name: symbol.name.clone(),
                        label: label.to_string(),
                        file: file.clone(),
                    });
                }
            }
        }
        // Deterministic ordering: by file, then by symbol name --
        // `graph.nodes()` iteration order is insertion order, not
        // sorted, so this is not merely cosmetic.
        impacted_symbols.sort_by(|a, b| {
            (a.file.as_str(), a.name.as_str()).cmp(&(b.file.as_str(), b.name.as_str()))
        });
    }

    DetectChangesView {
        changed_files: changed_files.to_vec(),
        changed_count: changed_files.len(),
        impacted_symbols,
        depth,
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
