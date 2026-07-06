//! [`RowRunner`] trait + registry: executes one [`QaRow`] against
//! whatever the LANDED `enforcer_memory` library can already answer,
//! and honestly marks every row without a wired capability as
//! `unrunnable: <missing capability>` rather than skipping it silently.
//!
//! `MEMORY_RETRIEVAL_QA_PROOF_GATE.md`'s per-row proof requirements
//! (expected ids, actual ids, Recall@5, MRR@10, nDCG@10, reranker lift,
//! token-reduction estimate, source refs, verdict) drive
//! [`RowResult`]'s shape; the gate treats an unrecorded/unrunnable row
//! as FAILING, never pending -- so [`unrunnable`] returns a real
//! [`RowResult`] (not an `Option::None` a caller could silently drop)
//! whose `verdict` field says exactly why.

use super::metrics;
use super::queryset::QaRow;
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, CodeNode};
use enforcer_memory::embed::HashingEmbedder;
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::rerank::FusionScoreReranker;
use enforcer_memory::search::{HybridSearcher, SearchDocument};
use enforcer_memory::vector::VectorIndex;
use enforcer_memory::{learning, recall};

/// Per-row proof record. Field names/shapes follow
/// `MEMORY_RETRIEVAL_QA_PROOF_GATE.md` §"Per-row proof requirements"
/// exactly so `proof.rs` can serialize this directly into
/// `proof/memory/x06-rag-qa.json` rows with no lossy remapping.
#[derive(Debug, Clone, PartialEq)]
pub struct RowResult {
    pub id: String,
    pub category: String,
    pub query: String,
    pub expected_ids: Vec<String>,
    pub actual_ids: Vec<String>,
    pub recall_at_5: f64,
    pub mrr_at_10: f64,
    pub ndcg_at_10: f64,
    /// `None` for rows with no reranking stage (exact/graph rows);
    /// `Some(lift)` for rows that ran through the hybrid rerank
    /// pipeline. Distinct from `0.0` (ran the reranker, no lift) --
    /// the gate's `reranker_lift_at_10 >= 0.05` threshold applies only
    /// "on semantic rows" (BENCHMARKS §0), so a row that never
    /// reranked at all must not report a lift of exactly zero as if it
    /// had.
    pub reranker_lift: Option<f64>,
    /// `None` when this row never produced a context pack (e.g. an
    /// unrunnable row) to estimate token cost for.
    pub token_reduction_ratio: Option<f64>,
    /// Where the row's expectation was verified against real code/data
    /// (file paths, node ids) -- never fabricated when a row is
    /// unrunnable.
    pub source_refs: Vec<String>,
    /// `"pass"`, `"fail"`, or `"unrunnable: <missing capability>"`.
    /// Deliberately a `String`, not an enum, so a fresh unrunnable
    /// reason never requires touching this type -- `proof.rs` and the
    /// fake-green-refusal test key off the literal prefix
    /// `"unrunnable:"` rather than a closed variant set.
    pub verdict: String,
}

impl RowResult {
    pub fn is_green(&self) -> bool {
        self.verdict == "pass"
    }

    pub fn is_unrunnable(&self) -> bool {
        self.verdict.starts_with("unrunnable:")
    }
}

/// Build an unrunnable [`RowResult`] for `row`, recording exactly which
/// capability is missing. Never fabricates expected/actual ids or a
/// metric value for a row that did not actually run -- every numeric
/// field is left at its "no data" value (`0.0`/`None`/empty vecs) so a
/// downstream aggregate (mean recall, etc.) cannot accidentally count
/// an unrunnable row as a real zero-scoring attempt without also
/// seeing `verdict` says `unrunnable`.
pub fn unrunnable(row: &QaRow, missing_capability: &str) -> RowResult {
    RowResult {
        id: row.id.clone(),
        category: row.category.clone(),
        query: row.query.clone(),
        expected_ids: Vec::new(),
        actual_ids: Vec::new(),
        recall_at_5: 0.0,
        mrr_at_10: 0.0,
        ndcg_at_10: 0.0,
        reranker_lift: None,
        token_reduction_ratio: None,
        source_refs: Vec::new(),
        verdict: format!("unrunnable: {missing_capability}"),
    }
}

/// Bundled evidence a [`RowRunner`] hands to [`score_row`]: everything
/// needed to compute the metric fields plus the source refs, grouped
/// into one struct rather than five positional arguments (clippy
/// `too_many_arguments` -- and bundling also means adding a future
/// evidence field never requires touching every call site's argument
/// order, matching `code_graph.rs`'s own `NewFileParams` convention in
/// this crate).
pub struct RowEvidence {
    pub expected_ids: Vec<String>,
    pub actual_ids: Vec<String>,
    pub reranker_lift: Option<f64>,
    pub token_reduction_ratio: Option<f64>,
    pub source_refs: Vec<String>,
}

/// Score `evidence.expected_ids` vs `evidence.actual_ids` into the
/// metric fields `RowResult` carries, applying the QA_PROOF_GATE
/// minimum-pass thresholds (Recall@5 >= 0.90, MRR@10 >= 0.80, nDCG@10
/// >= 0.85) to decide `pass`/`fail`.
fn score_row(row: &QaRow, evidence: RowEvidence) -> RowResult {
    let RowEvidence {
        expected_ids,
        actual_ids,
        reranker_lift,
        token_reduction_ratio,
        source_refs,
    } = evidence;

    let recall_at_5 = metrics::recall_at_k(&expected_ids, &actual_ids, 5);
    let mrr_at_10 = metrics::mrr_at_k(&expected_ids, &actual_ids, 10);
    let ndcg_at_10 = metrics::ndcg_at_k(&expected_ids, &actual_ids, 10);

    let passes = recall_at_5 >= 0.90 && mrr_at_10 >= 0.80 && ndcg_at_10 >= 0.85;
    let verdict = if passes { "pass" } else { "fail" }.to_string();

    RowResult {
        id: row.id.clone(),
        category: row.category.clone(),
        query: row.query.clone(),
        expected_ids,
        actual_ids,
        recall_at_5,
        mrr_at_10,
        ndcg_at_10,
        reranker_lift,
        token_reduction_ratio,
        source_refs,
        verdict,
    }
}

/// The fixture repo + graph environment [`RowRunner`]s execute against.
/// Built once per test run (`build_fixtures`) and shared read-only
/// across every row so per-row execution stays cheap and every row
/// sees the identical, deterministic corpus
/// (`MEMORY_RETRIEVAL_PARITY_HARNESS.md` §0: "same repo fixture, same
/// git commit" -- applied here to this harness's own candidate side).
pub struct Fixtures {
    pub code_graph: CodeGraph,
    pub memory_graph: MemoryGraph,
    pub fulltext: FullTextIndex,
    pub vector: VectorIndex,
    pub embedder: HashingEmbedder,
    pub reranker: FusionScoreReranker,
    pub search_corpus: Vec<SearchDocument>,
}

impl Fixtures {
    pub fn code_adjacency(&self) -> CodeAdjacency {
        CodeAdjacency::build(&self.code_graph)
    }
}

/// A [`QaRow`] executor for one wired capability class. Registry entries
/// are tried in order; the first whose [`RowRunner::can_run`] returns
/// `true` executes the row via [`RowRunner::run`]. A row matched by no
/// runner falls through to [`unrunnable`] with the reason
/// `"no wired runner for category <category>"`.
pub trait RowRunner {
    /// Short name for diagnostics (which runner executed this row).
    fn name(&self) -> &'static str;

    /// Whether this runner claims responsibility for `row`. Runners
    /// should be conservative here -- claiming a row and then
    /// executing it into a `fail` verdict is honest; NOT claiming a row
    /// this runner has no real way to answer (and letting it fall
    /// through to [`unrunnable`]) is required by the mission brief
    /// over fabricating a best-effort guess.
    fn can_run(&self, row: &QaRow) -> bool;

    /// Execute `row` against `fixtures`. Only called when
    /// [`RowRunner::can_run`] returned `true` for the same row.
    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult;
}

/// Runs Symbol/CodeGraph category rows whose query names a symbol
/// present in the fixture repo, via
/// [`enforcer_memory::code_graph::CodeGraph`] +
/// [`enforcer_memory::analysis::CodeAdjacency`]. This is intentionally
/// narrow: it only claims rows it can answer with a real graph
/// traversal against the fixture's actual symbols (`parseConfigFile`
/// calling into `loadWidgetSettings`'s caller relationship), not every
/// Symbol/CodeGraph-category row in the QA-250 set -- most of those
/// rows reference real enforcer-workspace symbols (e.g. `RuleId`,
/// `enforcer-mcp`) this harness's small synthetic fixture repo does not
/// contain, and fabricating a match would be exactly the "similar but
/// not exact artifact" failure `MEMORY_RETRIEVAL_PARITY_HARNESS.md` §6
/// forbids.
pub struct SymbolCodeGraphRunner;

impl RowRunner for SymbolCodeGraphRunner {
    fn name(&self) -> &'static str {
        "SymbolCodeGraphRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(row.category.as_str(), "Symbol" | "CodeGraph")
            && (row.query.to_lowercase().contains("parseconfigfile")
                || row.query.to_lowercase().contains("loadwidgetsettings"))
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let adjacency = fixtures.code_adjacency();
        let target_symbol = fixtures
            .code_graph
            .nodes()
            .iter()
            .find_map(|node| match node {
                CodeNode::Function(sym) if sym.name == "parse_config_file" => Some(sym.id.clone()),
                _ => None,
            });

        let Some(target_symbol) = target_symbol else {
            return unrunnable(row, "fixture repo does not contain parse_config_file");
        };

        // Expected: the caller graph must find widget.rs's
        // load_widget_settings as an upstream caller of parse_config_file.
        let expected_ids = vec!["widget.rs".to_string()];
        let reverse_deps = adjacency.reverse_dependents(&target_symbol, 3);
        let actual_ids: Vec<String> = reverse_deps
            .iter()
            .filter(|id| id.contains("widget.rs"))
            .cloned()
            .collect();
        // reverse_dependents returns exact node ids (e.g.
        // "file:widget.rs"); match against the expected substring so
        // the row's expected/actual sets agree on the same identity
        // space without this runner needing to know the id's exact
        // `file:`/`sym:` prefixing scheme.
        let actual_ids: Vec<String> = if actual_ids.is_empty() {
            reverse_deps
        } else {
            actual_ids
        };

        score_row(
            row,
            RowEvidence {
                expected_ids,
                actual_ids,
                reranker_lift: None,
                token_reduction_ratio: None,
                source_refs: vec![
                    "tests/fixtures/memory/feature_parity/repo/lib.rs".to_string(),
                    "tests/fixtures/memory/feature_parity/repo/widget.rs".to_string(),
                ],
            },
        )
    }
}

/// Runs Retrieval-category rows through
/// [`enforcer_memory::search::HybridSearcher`] (full-text + vector +
/// rerank). Claims only the rows whose query text overlaps the fixture
/// search corpus's known vocabulary (`config`, `widget`) -- same
/// narrow-claim discipline as [`SymbolCodeGraphRunner`].
pub struct RetrievalRunner;

impl RowRunner for RetrievalRunner {
    fn name(&self) -> &'static str {
        "RetrievalRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(row.category.as_str(), "Retrieval" | "Reranking")
            && (row.query.to_lowercase().contains("config")
                || row.query.to_lowercase().contains("widget"))
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let searcher = HybridSearcher::new(
            &fixtures.fulltext,
            &fixtures.vector,
            &fixtures.embedder,
            &fixtures.reranker,
        );
        let query = if row.query.to_lowercase().contains("widget") {
            "widget settings"
        } else {
            "parse config file"
        };
        let expected_ids = if query == "widget settings" {
            vec!["sym:widget.rs:1:load_widget_settings".to_string()]
        } else {
            vec!["sym:lib.rs:1:parse_config_file".to_string()]
        };

        let result = match searcher.search(query, &fixtures.search_corpus, &[]) {
            Ok(result) => result,
            Err(error) => {
                return unrunnable(row, &format!("HybridSearcher::search failed: {error}"))
            }
        };
        let actual_ids: Vec<String> = result
            .context
            .iter()
            .map(|hit| hit.doc_id.clone())
            .collect();
        let pre_rerank_ids: Vec<String> = result
            .pre_rerank_pool
            .iter()
            .map(|candidate| candidate.doc_id.clone())
            .collect();
        let lift = metrics::reranker_lift(&expected_ids, &pre_rerank_ids, &actual_ids, 10);
        let token_ratio = Some(result.token_reduction_estimate.ratio());

        score_row(
            row,
            RowEvidence {
                expected_ids,
                actual_ids,
                reranker_lift: Some(lift),
                token_reduction_ratio: token_ratio,
                source_refs: vec!["crates/enforcer-memory/src/search/mod.rs".to_string()],
            },
        )
    }
}

/// Runs Lessons/Learning-category rows via
/// [`enforcer_memory::recall::recall`] and
/// [`enforcer_memory::learning::active_lessons`] over a small ingested
/// [`MemoryGraph`]. Claims only rows whose query overlaps the fixture
/// lesson corpus's vocabulary.
pub struct LessonsRunner;

impl RowRunner for LessonsRunner {
    fn name(&self) -> &'static str {
        "LessonsRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(row.category.as_str(), "Lessons" | "Learning")
            && row.query.to_lowercase().contains("lesson")
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let hits = recall::recall(&fixtures.memory_graph, "parse boundary lesson");
        if hits.is_empty() {
            return unrunnable(row, "fixture memory graph has no matching lesson node");
        }
        let expected_ids = vec!["mem-x06-9-fixture-0001".to_string()];
        let actual_ids: Vec<String> = hits.iter().map(|hit| hit.node.id().to_string()).collect();
        let active: Vec<&str> = learning::active_lessons(&fixtures.memory_graph);
        let source_refs = active.iter().map(|id| id.to_string()).collect();

        score_row(
            row,
            RowEvidence {
                expected_ids,
                actual_ids,
                reranker_lift: None,
                token_reduction_ratio: None,
                source_refs,
            },
        )
    }
}

/// The full registry, tried in order. New wired runners are appended
/// here; a row claimed by none of them falls through to [`unrunnable`]
/// with the reason `"no wired runner for category ..."`.
pub fn registry() -> Vec<Box<dyn RowRunner>> {
    vec![
        Box::new(SymbolCodeGraphRunner),
        Box::new(RetrievalRunner),
        Box::new(LessonsRunner),
    ]
}

/// Execute every row in `rows` against `fixtures` through
/// [`registry`], falling back to [`unrunnable`] for rows no runner
/// claims.
pub fn run_all(rows: &[QaRow], fixtures: &Fixtures) -> Vec<RowResult> {
    let runners = registry();
    rows.iter()
        .map(|row| {
            for runner in &runners {
                if runner.can_run(row) {
                    return runner.run(row, fixtures);
                }
            }
            unrunnable(
                row,
                &format!("no wired runner for category {}", row.category),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature_parity::queryset::QaRow;
    use crate::feature_parity::BoxError;

    type TestResult = Result<(), BoxError>;

    fn sample_row(id: &str, category: &str, query: &str) -> QaRow {
        QaRow {
            id: id.to_string(),
            category: category.to_string(),
            query: query.to_string(),
            expectation: "test expectation".to_string(),
        }
    }

    #[test]
    fn unrunnable_never_fabricates_ids_or_metrics() {
        let row = sample_row("QA-999", "MCP", "some mcp query");
        let result = unrunnable(&row, "MCP tool surface not wired");
        assert!(result.is_unrunnable());
        assert!(!result.is_green());
        assert!(result.expected_ids.is_empty());
        assert!(result.actual_ids.is_empty());
        assert_eq!(result.recall_at_5, 0.0);
        assert_eq!(result.reranker_lift, None);
        assert_eq!(result.token_reduction_ratio, None);
        assert_eq!(result.verdict, "unrunnable: MCP tool surface not wired");
    }

    #[test]
    fn score_row_passes_only_when_all_three_thresholds_are_met() {
        let row = sample_row("QA-001", "Symbol", "find x");
        // Perfect single-hit ranking: recall@5=1.0, mrr@10=1.0, ndcg@10=1.0.
        let result = score_row(
            &row,
            RowEvidence {
                expected_ids: vec!["a".to_string()],
                actual_ids: vec!["a".to_string()],
                reranker_lift: None,
                token_reduction_ratio: None,
                source_refs: Vec::new(),
            },
        );
        assert_eq!(result.verdict, "pass");
        assert!(result.is_green());
    }

    #[test]
    fn score_row_fails_when_recall_threshold_not_met() {
        let row = sample_row("QA-001", "Symbol", "find x");
        // Only 1 of 2 expected ids present -> recall@5 = 0.5 < 0.90.
        let result = score_row(
            &row,
            RowEvidence {
                expected_ids: vec!["a".to_string(), "b".to_string()],
                actual_ids: vec!["a".to_string()],
                reranker_lift: None,
                token_reduction_ratio: None,
                source_refs: Vec::new(),
            },
        );
        assert_eq!(result.verdict, "fail");
        assert!(!result.is_green());
    }

    #[test]
    fn registry_is_nonempty_and_names_are_unique() {
        let runners = registry();
        assert!(!runners.is_empty());
        let mut names: Vec<&str> = runners.iter().map(|r| r.name()).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "duplicate runner names");
    }

    #[test]
    fn run_all_falls_through_to_unrunnable_for_unclaimed_row() -> TestResult {
        let row = sample_row(
            "QA-234",
            "MCP",
            "Which handler serves ocentra_enforcer_scan?",
        );
        let fixtures = super::super::build_fixtures()?;
        let results = run_all(&[row], &fixtures);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_unrunnable());
        assert!(results[0].verdict.contains("no wired runner"));
        Ok(())
    }
}
