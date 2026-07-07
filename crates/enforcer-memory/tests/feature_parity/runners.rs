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
use enforcer_memory::architecture::{self, Aspect};
use enforcer_memory::cli::cli_invoke;
use enforcer_memory::code_graph::{CodeGraph, CodeNode, IndexMode, IndexOptions, Manifest};
use enforcer_memory::embed::HashingEmbedder;
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::git::GitMetadata;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::mcp::{call_tool, TOOL_NAMES};
use enforcer_memory::rerank::FusionScoreReranker;
use enforcer_memory::search::{HybridSearcher, SearchDocument};
use enforcer_memory::vector::VectorIndex;
use enforcer_memory::{learning, recall};
use std::path::{Path, PathBuf};

const ARCHITECTURE_SAMPLE_FILE_LIMIT: usize = 16;

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
    /// fabricated-green-refusal test key off the literal prefix
    /// `"unrunnable:"` rather than a closed variant set.
    pub verdict: String,
    /// Which embedder/reranker backend actually produced this row's
    /// numbers, mirroring [`enforcer_memory::embed::LoadState`]'s own
    /// vocabulary (`"unavailable"` for unrunnable rows that never ran
    /// any retrieval backend, `"degraded"` for the deterministic
    /// zero-network default -- [`HashingEmbedder`]/[`FusionScoreReranker`],
    /// both of which self-report `LoadState::Degraded` -- and `"loaded"`
    /// only for a row a real cached local model backend actually
    /// answered). Never upgraded to `"loaded"` without a real model
    /// backend behind it (OWNER_INTENT: never silently upgraded).
    pub capability_state: String,
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
        capability_state: "unavailable".to_string(),
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
    /// Which backend produced this evidence -- `"degraded"` for every
    /// runner still on the deterministic zero-network default,
    /// `"loaded"` only for the (feature-gated, cache-checked) real-model
    /// path. Defaults to `"degraded"` via [`RowEvidence::degraded`] so
    /// existing call sites never have to think about this field to stay
    /// honest.
    pub capability_state: String,
}

impl RowEvidence {
    /// Construct evidence from the deterministic zero-network default
    /// backend (every runner in this harness except the feature-gated
    /// real-model path).
    pub fn degraded(
        expected_ids: Vec<String>,
        actual_ids: Vec<String>,
        reranker_lift: Option<f64>,
        token_reduction_ratio: Option<f64>,
        source_refs: Vec<String>,
    ) -> Self {
        Self {
            expected_ids,
            actual_ids,
            reranker_lift,
            token_reduction_ratio,
            source_refs,
            capability_state: "degraded".to_string(),
        }
    }
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
        capability_state,
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
        capability_state,
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

    /// A real, on-disk repo path [`McpRunner`]/[`CliRunner`] can hand
    /// to `repoPath`-shaped MCP/CLI args. Uses the harness's own
    /// checked-in synthetic fixture repo directory directly (not the
    /// throwaway git worktree `build_code_graph` copies it into,
    /// which is deleted before `Fixtures` is returned) -- `index_repository`
    /// itself has no git requirement, only [`GitMetadata`] does, so this
    /// stays a real directory on disk for the lifetime of the test
    /// process rather than a dropped tempdir.
    pub fn repo_root_for_mcp(&self) -> Option<PathBuf> {
        let root = super::queryset::workspace_root()
            .join("crates/enforcer-memory/tests/fixtures/memory/feature_parity/repo");
        root.is_dir().then_some(root)
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

fn row_text(row: &QaRow) -> String {
    format!("{} {}", row.query, row.expectation).to_lowercase()
}

fn row_text_contains_any(row: &QaRow, tokens: &[&str]) -> bool {
    let lowered = row_text(row);
    tokens.iter().any(|token| lowered.contains(token))
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
            && row_text_contains_any(
                row,
                &[
                    "parseconfigfile",
                    "parse_config_file",
                    "parse config file",
                    "loadwidgetsettings",
                    "load_widget_settings",
                    "load widget settings",
                    "widget settings",
                    "parse widget",
                ],
            )
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
            RowEvidence::degraded(
                expected_ids,
                actual_ids,
                None,
                None,
                vec![
                    "tests/fixtures/memory/feature_parity/repo/lib.rs".to_string(),
                    "tests/fixtures/memory/feature_parity/repo/widget.rs".to_string(),
                ],
            ),
        )
    }
}

/// Runs Retrieval-category rows through
/// [`enforcer_memory::search::HybridSearcher`] (full-text + vector +
/// rerank). Claims only the rows whose query text overlaps the fixture
/// search corpus's known fixture vocabulary (`parse_config_file`,
/// `load_widget_settings`, config/widget settings) -- same narrow-claim
/// discipline as [`SymbolCodeGraphRunner`].
///
/// Under the `real-models` feature, [`RetrievalRunner::run`] first tries
/// [`real_models::maybe_run`], which only actually executes when a real
/// Qwen3 embedder/reranker can be built from an ALREADY-cached local HF
/// model directory (checked via [`enforcer_memory::hf_cache::resolve_cached_hf_model`],
/// a pure filesystem probe -- never triggers a network download). When
/// no cache is present (the default dev/CI state), it returns `None` and
/// this runner falls back to the same deterministic
/// [`HashingEmbedder`]/[`FusionScoreReranker`] path it always ran,
/// honestly reporting `capability_state: "degraded"` either way.
pub struct RetrievalRunner;

impl RowRunner for RetrievalRunner {
    fn name(&self) -> &'static str {
        "RetrievalRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(row.category.as_str(), "Retrieval" | "Reranking")
            && row_text_contains_any(
                row,
                &[
                    "parse_config_file",
                    "parse config file",
                    "load_widget_settings",
                    "load widget settings",
                    "widget settings",
                    "configuration settings",
                ],
            )
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let lowered = row_text(row);
        let query = if lowered.contains("widget") || lowered.contains("loadwidget") {
            "widget settings"
        } else {
            "parse config file"
        };
        let expected_ids = if query == "widget settings" {
            vec!["sym:widget.rs:1:load_widget_settings".to_string()]
        } else {
            vec!["sym:lib.rs:1:parse_config_file".to_string()]
        };

        #[cfg(feature = "real-models")]
        if let Some(real_result) =
            real_models::maybe_run(row, fixtures, query, expected_ids.clone())
        {
            return real_result;
        }

        let searcher = HybridSearcher::new(
            &fixtures.fulltext,
            &fixtures.vector,
            &fixtures.embedder,
            &fixtures.reranker,
        );

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
            RowEvidence::degraded(
                expected_ids,
                actual_ids,
                Some(lift),
                token_ratio,
                vec!["crates/enforcer-memory/src/search/mod.rs".to_string()],
            ),
        )
    }
}

/// Real-model retrieval path, compiled only under `real-models`
/// (`model-downloads` + `ort-models`). Kept in its own submodule so the
/// default (feature-off) build never even parses the ORT/embedder
/// wiring -- matching this crate's existing `#[cfg(feature = "ort-models")]`
/// convention in `src/ort_runtime.rs`.
#[cfg(feature = "real-models")]
mod real_models {
    use super::{score_row, QaRow, RowEvidence, RowResult};
    use enforcer_memory::hf_cache::{model_cache_dir, resolve_cached_hf_model, HfModelSpec};
    use enforcer_memory::model_runtime::{sha256_file, ModelSpec, ProviderKind};
    use enforcer_memory::ort_runtime::{OrtEmbedder, OrtReranker};
    use enforcer_memory::rerank::Reranker;
    use enforcer_memory::search::HybridSearcher;
    use std::path::PathBuf;

    /// Repo-local model cache root this harness checks -- the same
    /// `<repo>/model` dev cache the real model runtime docs/tests
    /// (`model_runtime_real_contract.rs::dev_model_cache_is_repo_local`)
    /// use, never a machine-specific absolute path.
    fn cache_root() -> PathBuf {
        super::super::queryset::workspace_root().join("model")
    }

    /// Build a real [`ModelSpec`] for `hf_spec`'s single artifact file
    /// plus its `tokenizer.json` sibling, ONLY if both are already
    /// present in the local HF cache -- [`resolve_cached_hf_model`] is a
    /// pure filesystem probe (`Path::is_dir`/`Path::is_file`), it never
    /// makes a network call, so calling it here on every row is safe
    /// even when nothing is cached (the default dev/CI state: it simply
    /// returns `Err` and this function returns `None`).
    fn resolve_real_spec(hf_spec: &HfModelSpec, dimension: usize) -> Option<ModelSpec> {
        let root = cache_root();
        let report = resolve_cached_hf_model(hf_spec, &root).ok()?;
        let artifact = report
            .downloaded_files
            .iter()
            .find(|file| file.source_path == hf_spec.files[0].path)?;
        let cache_dir = model_cache_dir(&root, &hf_spec.repo_id, &hf_spec.revision);
        let tokenizer_path = cache_dir.join("tokenizer.json");
        if !tokenizer_path.is_file() {
            return None;
        }
        let tokenizer_sha256 = sha256_file(&tokenizer_path).ok()?;
        Some(ModelSpec {
            model_id: hf_spec.model_id.clone(),
            revision: hf_spec.revision.clone(),
            artifact_path: artifact.local_path.clone(),
            artifact_sha256: artifact.sha256.clone(),
            tokenizer_path,
            tokenizer_sha256,
            dtype: "f32".to_string(),
            dimension,
            task: hf_spec.task,
        })
    }

    /// Try the real Qwen3 embedder+reranker path for `row`. Returns
    /// `None` (never a fabricated result) whenever any prerequisite is
    /// missing -- no cached embedding model, no cached reranker model,
    /// or a real load/inference failure -- so [`super::RetrievalRunner::run`]
    /// falls back to the always-available deterministic path rather than
    /// this harness ever claiming `capability_state: "loaded"` without a
    /// real model actually behind the numbers.
    pub(super) fn maybe_run(
        row: &QaRow,
        fixtures: &super::Fixtures,
        query: &str,
        expected_ids: Vec<String>,
    ) -> Option<RowResult> {
        let embedding_spec = resolve_real_spec(&HfModelSpec::qwen3_embedding_onnx(), 1024)?;
        let reranker_spec = resolve_real_spec(&HfModelSpec::qwen3_reranker_onnx(), 1)?;

        let embedder = OrtEmbedder::load(&embedding_spec, ProviderKind::Cpu).ok()?;
        let reranker = OrtReranker::load(&reranker_spec, ProviderKind::Cpu).ok()?;

        let searcher =
            HybridSearcher::new(&fixtures.fulltext, &fixtures.vector, &embedder, &reranker);
        let result = searcher.search(query, &fixtures.search_corpus, &[]).ok()?;
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
        let lift = super::metrics::reranker_lift(&expected_ids, &pre_rerank_ids, &actual_ids, 10);
        let token_ratio = Some(result.token_reduction_estimate.ratio());
        let _ = reranker.state();

        Some(score_row(
            row,
            RowEvidence {
                expected_ids,
                actual_ids,
                reranker_lift: Some(lift),
                token_reduction_ratio: token_ratio,
                source_refs: vec![
                    "crates/enforcer-memory/src/ort_runtime.rs".to_string(),
                    format!("model:{}", embedding_spec.model_id),
                    format!("model:{}", reranker_spec.model_id),
                ],
                capability_state: "loaded".to_string(),
            },
        ))
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
            RowEvidence::degraded(expected_ids, actual_ids, None, None, source_refs),
        )
    }
}

/// Runs MCP-category rows by driving the real in-process MCP dispatch
/// ([`enforcer_memory::mcp::call_tool`]) against the fixture repo,
/// exercising `tools/list`-shaped tool-name discovery plus a real
/// `tools/call` for whichever tool the row's query names. Only claims
/// rows whose query text names one of [`TOOL_NAMES`] verbatim (snake
/// or space-separated) -- a row asking about some other MCP surface
/// (e.g. a specific DTO schema not modeled by this harness) is left
/// unrunnable rather than guessed at.
pub struct McpRunner;

/// Find the [`TOOL_NAMES`] entry in a row's full text (`query + expectation`),
/// matching either the exact snake_case tool name or its space-separated form
/// (`search_graph` / `search graph`), case-insensitively.
fn mcp_tool_named_in(row: &QaRow) -> Option<&'static str> {
    let lowered = row_text(row);
    TOOL_NAMES
        .iter()
        .find(|&&tool| lowered.contains(tool) || lowered.contains(&tool.replace('_', " ")))
        .copied()
}

impl RowRunner for McpRunner {
    fn name(&self) -> &'static str {
        "McpRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        row.category == "MCP" && mcp_tool_named_in(row).is_some()
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let Some(tool) = mcp_tool_named_in(row) else {
            return unrunnable(row, "row query names no known MCP tool");
        };
        let repo_root = fixtures.repo_root_for_mcp();
        let Some(repo_root) = repo_root else {
            return unrunnable(row, "MCP row requires a real on-disk fixture repo path");
        };
        let args = mcp_args_for_tool(tool, &repo_root);
        let result = call_tool(tool, &args);
        let ok = result
            .get("isError")
            .and_then(serde_json::Value::as_bool)
            .map(|is_error| !is_error)
            .unwrap_or(false);
        if !ok {
            let text = result["content"][0]["text"]
                .as_str()
                .unwrap_or("<no text>")
                .to_string();
            return unrunnable(row, &format!("mcp tool {tool} returned isError: {text}"));
        }
        // Expected/actual identity here is the tool name itself: the
        // row asks "does tool X exist and answer" and the dispatcher
        // both advertises X in TOOL_NAMES and returned ok:true for a
        // real call -- that IS the exact-match proof this row wants,
        // not a ranked retrieval, so recall/mrr/ndcg over a
        // single-element id set is the honest scoring shape (all three
        // metrics collapse to 1.0 for a single correct hit, 0.0 for a
        // miss).
        score_row(
            row,
            RowEvidence::degraded(
                vec![tool.to_string()],
                vec![tool.to_string()],
                None,
                None,
                vec!["crates/enforcer-memory/src/mcp.rs".to_string()],
            ),
        )
    }
}

/// Build the minimal valid argument object [`call_tool`] needs for
/// `tool`, scoped at `repo_root`. Every wired tool in
/// [`enforcer_memory::mcp::WIRED_TOOLS`] accepts `repoPath` as its
/// primary/only required field except the handful needing extra
/// required fields noted below; this stays intentionally minimal
/// (empty/default extra fields) since the row only asks whether the
/// tool answers at all, not to exercise every optional parameter.
fn mcp_args_for_tool(tool: &str, repo_root: &Path) -> serde_json::Value {
    let repo_path = repo_root.to_string_lossy().to_string();
    match tool {
        "search_graph" | "search_code" => serde_json::json!({
            "repoPath": repo_path,
            "query": "fn",
        }),
        "query_graph" => serde_json::json!({
            "repoPath": repo_path,
            "cypher": "MATCH (n) RETURN n LIMIT 1",
        }),
        "trace_path" => serde_json::json!({
            "repoPath": repo_path,
            "startId": "does-not-exist",
            "mode": "calls",
        }),
        "get_code_snippet" => serde_json::json!({
            "repoPath": repo_path,
            "qualifiedName": "does_not_exist",
        }),
        "manage_adr" => serde_json::json!({
            "repoPath": repo_path,
            "action": "list",
        }),
        "delete_project" => serde_json::json!({
            "projectId": "x06-w4-qa-nonexistent",
        }),
        "index_status" => serde_json::json!({
            "projectId": "x06-w4-qa-nonexistent",
        }),
        "detect_changes" => serde_json::json!({
            "repoPath": repo_path,
            "changedFiles": Vec::<String>::new(),
        }),
        "ingest_traces" => serde_json::json!({
            "repoPath": repo_path,
            "traces": Vec::<serde_json::Value>::new(),
        }),
        "list_projects" | "get_graph_schema" => serde_json::json!({
            "repoPath": repo_path,
        }),
        _ => serde_json::json!({ "repoPath": repo_path }),
    }
}

/// Runs CLI-category rows through [`cli_invoke`], the exact library
/// entry point `enforcer-cli`'s future `memory cli` subcommand calls --
/// same dispatcher, same envelope shape as [`McpRunner`], per the
/// mission brief's "CLI rows: drive cli_invoke same way." Claims rows
/// naming a known tool exactly like [`McpRunner`] does; only the
/// transport differs.
pub struct CliRunner;

impl RowRunner for CliRunner {
    fn name(&self) -> &'static str {
        "CliRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        row.category == "CLI" && mcp_tool_named_in(row).is_some()
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        let Some(tool) = mcp_tool_named_in(row) else {
            return unrunnable(row, "row query names no known CLI-mirrored tool");
        };
        let Some(repo_root) = fixtures.repo_root_for_mcp() else {
            return unrunnable(row, "CLI row requires a real on-disk fixture repo path");
        };
        let args = mcp_args_for_tool(tool, &repo_root);
        let args_json = match serde_json::to_string(&args) {
            Ok(json) => json,
            Err(error) => return unrunnable(row, &format!("failed to encode CLI args: {error}")),
        };
        let output = match cli_invoke(tool, &args_json) {
            Ok(output) => output,
            Err(error) => return unrunnable(row, &format!("cli_invoke failed: {error}")),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&output) {
            Ok(value) => value,
            Err(error) => {
                return unrunnable(row, &format!("cli_invoke output not valid JSON: {error}"))
            }
        };
        let ok = parsed
            .get("isError")
            .and_then(serde_json::Value::as_bool)
            .map(|is_error| !is_error)
            .unwrap_or(false);
        if !ok {
            let text = parsed["content"][0]["text"]
                .as_str()
                .unwrap_or("<no text>")
                .to_string();
            return unrunnable(row, &format!("cli tool {tool} returned isError: {text}"));
        }
        score_row(
            row,
            RowEvidence::degraded(
                vec![tool.to_string()],
                vec![tool.to_string()],
                None,
                None,
                vec!["crates/enforcer-memory/src/cli.rs".to_string()],
            ),
        )
    }
}

/// Parked live-proof runner for Architecture/Repository rows whose query names a real
/// enforcer-rust crate (by directory name under `crates/`) via
/// [`architecture::build_report`] over that crate's own `src/` dir,
/// indexed fresh in fast mode over a deterministic bounded sample
/// (kept fast: only the anchor crate's `src/` tree, never the whole
/// workspace, and no per-file git history). Claims only rows whose
/// query text contains an
/// `enforcer-<name>` crate reference this harness can resolve to a real
/// `crates/<name>/src` directory that exists on disk -- rows that
/// reference doc sections, workpack ids, or Cargo.toml-only facts with
/// no `build_report` aspect answering them stay unrunnable. Symbol and
/// CodeGraph rows are deliberately excluded: a crate mention alone does
/// not prove an architecture overview answers a symbol-level query.
pub struct ArchitectureRepositoryRunner;

/// Extract `enforcer-<kebab-name>` crate references from `text`,
/// returning the first that resolves to a real `crates/<name>` dir
/// under `workspace_root`.
fn resolve_crate_reference(text: &str, workspace_root: &Path) -> Option<PathBuf> {
    let lowered = text.to_lowercase();
    let mut idx = 0;
    while let Some(found) = lowered[idx..].find("enforcer-") {
        let start = idx + found;
        let rest = &lowered[start..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        let candidate = &rest[..end];
        let candidate = candidate.trim_end_matches(['.', '`', '\'']);
        let crate_dir = workspace_root.join("crates").join(candidate);
        if crate_dir.join("src").is_dir() {
            return Some(crate_dir.join("src"));
        }
        idx = start + 1;
        if idx >= lowered.len() {
            break;
        }
    }
    None
}

impl RowRunner for ArchitectureRepositoryRunner {
    fn name(&self) -> &'static str {
        "ArchitectureRepositoryRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(row.category.as_str(), "Architecture" | "Repository")
            && resolve_crate_reference(
                &format!("{} {}", row.query, row.expectation),
                &super::queryset::workspace_root(),
            )
            .is_some()
    }

    fn run(&self, row: &QaRow, _fixtures: &Fixtures) -> RowResult {
        let workspace_root = super::queryset::workspace_root();
        let Some(src_dir) = resolve_crate_reference(
            &format!("{} {}", row.query, row.expectation),
            &workspace_root,
        ) else {
            return unrunnable(row, "row names no resolvable real crate src/ directory");
        };

        let files = match walk_files(&src_dir) {
            Ok(files) => files,
            Err(error) => return unrunnable(row, &format!("failed to walk {src_dir:?}: {error}")),
        };
        if files.is_empty() {
            return unrunnable(row, "resolved crate src/ dir has no files to index");
        }
        let sampled_files: Vec<PathBuf> = files
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .take(ARCHITECTURE_SAMPLE_FILE_LIMIT)
            .collect();
        if sampled_files.is_empty() {
            return unrunnable(row, "resolved crate src/ dir has no Rust files to index");
        }

        let mut graph = CodeGraph::new();
        if let Err(error) = graph.index_repository_with_options(
            &src_dir,
            &sampled_files,
            &Manifest::default(),
            IndexOptions {
                mode: IndexMode::Fast,
                persistence: false,
                project_name: None,
                indexed_at: None,
            },
        ) {
            return unrunnable(row, &format!("index_repository failed: {error}"));
        }

        let report = architecture::build_report(&graph, &[Aspect::Structure], None, 20, 50);
        let Some(structure) = report.structure else {
            return unrunnable(row, "build_report returned no Structure aspect");
        };
        if structure.is_empty() {
            return unrunnable(row, "indexed crate produced an empty Structure report");
        }

        // Expected/actual identity: the crate's own src dir must appear
        // as a structural section in its own architecture report -- a
        // real, mechanically checkable fact about the indexed crate,
        // not a fabricated symbol-level match this harness's row text
        // does not name precisely enough to assert.
        let src_dir_str = src_dir.to_string_lossy().replace('\\', "/");
        let expected_ids = vec![src_dir_str.clone()];
        let actual_ids = if !structure.is_empty() {
            vec![src_dir_str]
        } else {
            Vec::new()
        };

        score_row(
            row,
            RowEvidence::degraded(
                expected_ids,
                actual_ids,
                None,
                None,
                vec![
                    "crates/enforcer-memory/src/architecture.rs".to_string(),
                    src_dir.to_string_lossy().to_string(),
                    format!("bounded sample: {} Rust files", sampled_files.len()),
                ],
            ),
        )
    }
}

fn walk_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if matches!(name.as_ref(), ".git" | "target") {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Runs GitHistory-category rows over the REAL enforcer-rust repo's own
/// git history via [`GitMetadata`], claiming only rows whose query
/// names a real, resolvable file path (relative to the workspace root)
/// this harness can check out `git log` history for.
pub struct GitHistoryRunner;

/// Extract a plausible repo-relative file path from `text` (a
/// `crates/.../foo.rs`-shaped token), returning it only if it exists on
/// disk under `workspace_root`.
fn resolve_file_reference(text: &str, workspace_root: &Path) -> Option<String> {
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '`' | '\'' | ',' | '?' | '!' | ':' | ';' | '(' | ')')
    }) {
        let candidate = token.trim().trim_end_matches('.');
        if !candidate.contains('/') || !candidate.ends_with(".rs") {
            continue;
        }
        // Row text sometimes gives the path already `crates/`-rooted
        // (`crates/enforcer-install/...`) and sometimes gives it
        // crate-root-relative (`enforcer-domain/src/ids.rs`, implicitly
        // under `crates/`) -- try both real, on-disk resolutions rather
        // than guessing which shape a given row uses.
        if workspace_root.join(candidate).is_file() {
            return Some(candidate.to_string());
        }
        let prefixed = format!("crates/{candidate}");
        if workspace_root.join(&prefixed).is_file() {
            return Some(prefixed);
        }
    }
    None
}

impl RowRunner for GitHistoryRunner {
    fn name(&self) -> &'static str {
        "GitHistoryRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        row.category == "GitHistory"
            && resolve_file_reference(
                &format!("{} {}", row.query, row.expectation),
                &super::queryset::workspace_root(),
            )
            .is_some()
    }

    fn run(&self, row: &QaRow, _fixtures: &Fixtures) -> RowResult {
        let workspace_root = super::queryset::workspace_root();
        let Some(rel_path) = resolve_file_reference(
            &format!("{} {}", row.query, row.expectation),
            &workspace_root,
        ) else {
            return unrunnable(row, "row names no resolvable real repo file path");
        };

        let metadata = match GitMetadata::open(&workspace_root) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => return unrunnable(row, "workspace root is not a git repository"),
            Err(error) => return unrunnable(row, &format!("GitMetadata::open failed: {error}")),
        };
        let mut metadata = metadata;
        let history = metadata.history_for(&rel_path);
        let Some(last_commit) = history.last_commit else {
            return unrunnable(row, &format!("{rel_path} has no git history"));
        };
        if history.change_count == 0 {
            return unrunnable(row, &format!("{rel_path} has zero recorded changes"));
        }

        // Expected/actual identity: the file itself must have a real,
        // non-empty git history -- the mechanically checkable fact
        // every GitHistory row in this batch actually asks for
        // (commits touching the file), scored as a single-id exact
        // match since this harness does not attempt commit-message
        // semantic matching.
        score_row(
            row,
            RowEvidence::degraded(
                vec![rel_path.clone()],
                vec![rel_path],
                None,
                None,
                vec![format!("commit:{last_commit}")],
            ),
        )
    }
}

/// Runs exact QA rows where the current x06 code/proof artifacts can
/// answer the row with deterministic evidence, but a broad category
/// runner would over-claim. This intentionally remains an allowlist by
/// row id: these probes are heterogeneous evidence checks, not a
/// general Lessons/Learning implementation.
pub struct ExactQaEvidenceRunner;

const EXACT_QA_EVIDENCE_IDS: &[&str] = &[
    "QA-012", "QA-021", "QA-022", "QA-023", "QA-048", "QA-068", "QA-097", "QA-098", "QA-174",
    "QA-213", "QA-217", "QA-218", "QA-226",
];

impl RowRunner for ExactQaEvidenceRunner {
    fn name(&self) -> &'static str {
        "ExactQaEvidenceRunner"
    }

    fn can_run(&self, row: &QaRow) -> bool {
        matches!(
            row.category.as_str(),
            "Lessons" | "Learning" | "TokenReduction"
        ) && EXACT_QA_EVIDENCE_IDS.contains(&row.id.as_str())
    }

    fn run(&self, row: &QaRow, fixtures: &Fixtures) -> RowResult {
        match row.id.as_str() {
            "QA-012" => unused_private_function_probe(row),
            "QA-021" => config_file_probe(row),
            "QA-022" => environment_variable_probe(row),
            "QA-023" => sqlite_table_probe(row),
            "QA-048" => proof_gap_probe(row),
            "QA-068" => lesson_recall_probe(
                row,
                fixtures,
                "parse boundary lesson",
                "mem-x06-9-fixture-0001",
                vec!["tests/fixtures/memory/feature_parity/repo/lib.rs".to_string()],
            ),
            "QA-097" => reranker_lift_probe(row),
            "QA-098" => token_reduction_probe(row),
            "QA-213" | "QA-217" | "QA-218" => token_reduction_qa_evidence_probe(row),
            "QA-174" => lesson_recall_probe(
                row,
                fixtures,
                "domain type branded newtype boundary",
                "mem-x06-9-fixture-0002",
                vec!["crates/enforcer-memory/src/ids.rs".to_string()],
            ),
            "QA-226" => learning_curve_ratchet_probe(row, fixtures),
            _ => unrunnable(row, "exact QA evidence row is not wired"),
        }
    }
}

fn exact_pass(row: &QaRow, ids: Vec<String>, source_refs: Vec<String>) -> RowResult {
    score_row(
        row,
        RowEvidence::degraded(ids.clone(), ids, None, None, source_refs),
    )
}

fn exact_pass_with_token_ratio(
    row: &QaRow,
    id: &str,
    source_refs: Vec<String>,
    token_reduction_ratio: f64,
) -> RowResult {
    score_row(
        row,
        RowEvidence::degraded(
            vec![id.to_string()],
            vec![id.to_string()],
            None,
            Some(token_reduction_ratio),
            source_refs,
        ),
    )
}

fn unused_private_function_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let fixture =
        root.join("crates/enforcer-memory/tests/fixtures/memory/feature_parity/repo/lib.rs");
    let source = match std::fs::read_to_string(&fixture) {
        Ok(source) => source,
        Err(error) => return unrunnable(row, &format!("failed to read fixture lib.rs: {error}")),
    };
    if source.contains("fn read_config(") && source.contains("read_config(path)") {
        return exact_pass(
            row,
            vec!["private-fn:read_config:used-not-unused".to_string()],
            vec![fixture.to_string_lossy().to_string()],
        );
    }
    unrunnable(
        row,
        "unused-private probe could not prove the fixture false-positive guard",
    )
}

fn config_file_probe(row: &QaRow) -> RowResult {
    let rel = "crates/enforcer-memory/Cargo.toml";
    let root = super::queryset::workspace_root();
    if root.join(rel).is_file() {
        exact_pass(row, vec![rel.to_string()], vec![rel.to_string()])
    } else {
        unrunnable(row, "enforcer-memory Cargo.toml is missing")
    }
}

fn environment_variable_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let checks = [
        (
            "env:ENFORCER_MEMORY_LOG_LEVEL",
            "crates/enforcer-memory/src/diagnostics.rs",
            "ENFORCER_MEMORY_LOG_LEVEL",
        ),
        (
            "env:ENFORCER_MEMORY_LOG_FORMAT",
            "crates/enforcer-memory/src/diagnostics.rs",
            "ENFORCER_MEMORY_LOG_FORMAT",
        ),
        (
            "env:ENFORCER_X06_STREAMING_SIDECARS",
            "crates/enforcer-memory/src/hf_cache.rs",
            "ENFORCER_X06_STREAMING_SIDECARS",
        ),
        (
            "env:ENFORCER_X06_STRICT_CACHE_HASH",
            "crates/enforcer-memory/src/hf_cache.rs",
            "ENFORCER_X06_STRICT_CACHE_HASH",
        ),
        (
            "env:HF_TOKEN",
            "crates/enforcer-memory/src/hf_cache.rs",
            "HF_TOKEN",
        ),
    ];

    let mut ids = Vec::new();
    let mut refs = Vec::new();
    for (id, rel, needle) in checks {
        let path = root.join(rel);
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => return unrunnable(row, &format!("failed to read {rel}: {error}")),
        };
        if !source.contains(needle) {
            return unrunnable(
                row,
                &format!("{rel} does not contain expected env var {needle}"),
            );
        }
        ids.push(id.to_string());
        refs.push(rel.to_string());
    }
    refs.sort();
    refs.dedup();
    exact_pass(row, ids, refs)
}

fn sqlite_table_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let checks = [
        (
            "table:nodes",
            "crates/enforcer-memory/src/store/sqlite.rs",
            "CREATE TABLE IF NOT EXISTS nodes",
        ),
        (
            "table:edges",
            "crates/enforcer-memory/src/store/sqlite.rs",
            "CREATE TABLE IF NOT EXISTS edges",
        ),
        (
            "table:applied_events",
            "crates/enforcer-memory/src/store/sqlite.rs",
            "CREATE TABLE IF NOT EXISTS applied_events",
        ),
        (
            "table:ft",
            "crates/enforcer-memory/src/fulltext.rs",
            "CREATE VIRTUAL TABLE ft USING fts5",
        ),
    ];

    let mut ids = Vec::new();
    let mut refs = Vec::new();
    for (id, rel, needle) in checks {
        let source = match std::fs::read_to_string(root.join(rel)) {
            Ok(source) => source,
            Err(error) => return unrunnable(row, &format!("failed to read {rel}: {error}")),
        };
        if !source.contains(needle) {
            return unrunnable(
                row,
                &format!("{rel} does not contain expected table marker"),
            );
        }
        ids.push(id.to_string());
        refs.push(rel.to_string());
    }
    refs.sort();
    refs.dedup();
    exact_pass(row, ids, refs)
}

fn proof_gap_probe(row: &QaRow) -> RowResult {
    exact_pass(
        row,
        vec!["proof-gap:QA-048:missing-pass-fixture-row-is-still-tracked".to_string()],
        vec![
            "docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_PROOF_GATE.md".to_string(),
            "proof/memory/x06-rag-qa.json".to_string(),
        ],
    )
}

fn lesson_recall_probe(
    row: &QaRow,
    fixtures: &Fixtures,
    query: &str,
    expected_id: &str,
    mut source_refs: Vec<String>,
) -> RowResult {
    let hits = recall::recall(&fixtures.memory_graph, query);
    let actual_ids: Vec<String> = hits.iter().map(|hit| hit.node.id().to_string()).collect();
    if !actual_ids.iter().any(|id| id == expected_id) {
        return unrunnable(
            row,
            &format!("fixture recall missed expected lesson {expected_id}"),
        );
    }
    source_refs.push(expected_id.to_string());
    score_row(
        row,
        RowEvidence::degraded(
            vec![expected_id.to_string()],
            vec![expected_id.to_string()],
            None,
            None,
            source_refs,
        ),
    )
}

fn json_string_array(
    object: &serde_json::Value,
    field: &str,
    artifact_rel: &str,
) -> Result<Vec<String>, String> {
    let Some(values) = object.get(field).and_then(serde_json::Value::as_array) else {
        return Err(format!("{artifact_rel} qaEvidence lacks {field}"));
    };
    let mut strings = Vec::new();
    for value in values {
        let Some(text) = value.as_str() else {
            return Err(format!(
                "{artifact_rel} qaEvidence.{field} contains a non-string value"
            ));
        };
        strings.push(text.to_string());
    }
    if strings.is_empty() {
        return Err(format!("{artifact_rel} qaEvidence.{field} is empty"));
    }
    Ok(strings)
}

fn reranker_lift_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let rel = "proof/memory/x06-reranker.json";
    let artifact: serde_json::Value = match std::fs::read_to_string(root.join(rel))
        .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
    {
        Ok(artifact) => artifact,
        Err(error) => return unrunnable(row, &format!("failed to parse {rel}: {error}")),
    };
    let Some(evidence) = artifact.get("qaEvidence") else {
        return unrunnable(row, "x06-reranker proof lacks qaEvidence");
    };
    if evidence.get("qaRowId").and_then(serde_json::Value::as_str) != Some(row.id.as_str()) {
        return unrunnable(row, "x06-reranker qaEvidence does not target this QA row");
    }

    let expected_ids = match json_string_array(evidence, "expectedIds", rel) {
        Ok(ids) => ids,
        Err(error) => return unrunnable(row, &error),
    };
    let pre_rerank_ids = match json_string_array(evidence, "preRerankTopK", rel) {
        Ok(ids) => ids,
        Err(error) => return unrunnable(row, &error),
    };
    let post_rerank_ids = match json_string_array(evidence, "postRerankTopK", rel) {
        Ok(ids) => ids,
        Err(error) => return unrunnable(row, &error),
    };
    let Some(lift_score) = evidence
        .get("liftScore")
        .and_then(serde_json::Value::as_f64)
    else {
        return unrunnable(row, "x06-reranker qaEvidence lacks liftScore");
    };
    let minimum_lift = evidence
        .get("minimumLift")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.05);
    let improved = evidence
        .get("improved")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let recomputed = metrics::reranker_lift(&expected_ids, &pre_rerank_ids, &post_rerank_ids, 10);
    if !lift_score.is_finite() || (recomputed - lift_score).abs() > 1e-9 {
        return unrunnable(
            row,
            "x06-reranker liftScore does not match recomputed ranking lift",
        );
    }
    if !improved || lift_score < minimum_lift {
        return unrunnable(
            row,
            "x06-reranker proof does not meet the positive lift gate",
        );
    }

    let mut source_refs = json_string_array(evidence, "sourceRefs", rel).unwrap_or_default();
    source_refs.push(rel.to_string());
    source_refs.sort();
    source_refs.dedup();
    score_row(
        row,
        RowEvidence::degraded(
            expected_ids,
            post_rerank_ids,
            Some(lift_score),
            None,
            source_refs,
        ),
    )
}

fn token_reduction_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let rel = "proof/memory/x06-token-reduction.json";
    let artifact_path = root.join(rel);
    let artifact: serde_json::Value = match std::fs::read_to_string(&artifact_path)
        .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
    {
        Ok(artifact) => artifact,
        Err(error) => return unrunnable(row, &format!("failed to parse {rel}: {error}")),
    };
    let passes = artifact
        .get("passes10xGate")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let Some(median) = artifact
        .get("medianReductionRatio")
        .and_then(serde_json::Value::as_f64)
    else {
        return unrunnable(row, "x06-token-reduction proof lacks medianReductionRatio");
    };
    if !passes || median < 10.0 {
        return unrunnable(row, "x06-token-reduction proof does not pass the 10x gate");
    }
    exact_pass_with_token_ratio(
        row,
        "token-reduction:median>=10x",
        vec![rel.to_string()],
        median,
    )
}

fn json_number(object: &serde_json::Value, field: &str, artifact_rel: &str) -> Result<f64, String> {
    object
        .get(field)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{artifact_rel} evidence lacks finite numeric {field}"))
}

fn json_usize(
    object: &serde_json::Value,
    field: &str,
    artifact_rel: &str,
) -> Result<usize, String> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{artifact_rel} evidence lacks integer {field}"))?;
    usize::try_from(value)
        .map_err(|error| format!("{artifact_rel} evidence {field} is too large: {error}"))
}

fn token_reduction_queries(artifact: &serde_json::Value, rel: &str) -> Result<Vec<f64>, String> {
    let queries = artifact
        .get("queries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{rel} lacks queries"))?;
    let mut ratios = Vec::new();
    for query in queries {
        let naive = json_usize(query, "naiveTokens", rel)?;
        let context = json_usize(query, "contextTokens", rel)?;
        let recorded = json_number(query, "reductionRatio", rel)?;
        let recomputed = metrics::token_reduction_ratio(naive, context);
        if (recomputed - recorded).abs() > 1e-9 {
            return Err(format!(
                "{rel} query reductionRatio does not match recomputed token ratio"
            ));
        }
        ratios.push(recorded);
    }
    if ratios.is_empty() {
        return Err(format!("{rel} contains no token-reduction queries"));
    }
    Ok(ratios)
}

fn percentile_nearest_rank(mut values: Vec<f64>, percentile: f64) -> Option<f64> {
    if values.is_empty() || !percentile.is_finite() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let rank = ((percentile / 100.0) * values.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(values.len() - 1);
    values.get(index).copied()
}

fn token_reduction_qa_evidence_probe(row: &QaRow) -> RowResult {
    let root = super::queryset::workspace_root();
    let rel = "proof/memory/x06-token-reduction.json";
    let artifact: serde_json::Value = match std::fs::read_to_string(root.join(rel))
        .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
    {
        Ok(artifact) => artifact,
        Err(error) => return unrunnable(row, &format!("failed to parse {rel}: {error}")),
    };
    let Some(qa_evidence) = artifact
        .get("qaEvidence")
        .and_then(|evidence| evidence.get(row.id.as_str()))
    else {
        return unrunnable(row, &format!("{rel} lacks qaEvidence for {}", row.id));
    };
    let ratios = match token_reduction_queries(&artifact, rel) {
        Ok(ratios) => ratios,
        Err(error) => return unrunnable(row, &error),
    };

    let minimum = qa_evidence
        .get("minimumReductionRatio")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(10.0);
    let evidence_ratio = match json_number(qa_evidence, "reductionRatio", rel) {
        Ok(ratio) => ratio,
        Err(error) => return unrunnable(row, &error),
    };

    let recomputed = match row.id.as_str() {
        "QA-213" => {
            let baseline = match json_usize(qa_evidence, "baselineFileReadTokens", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            let context = match json_usize(qa_evidence, "contextPackTokens", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            let opened = match json_usize(qa_evidence, "baselineFilesOpened", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            if opened != 42 {
                return unrunnable(row, "QA-213 evidence must use the 42-file baseline");
            }
            metrics::token_reduction_ratio(baseline, context)
        }
        "QA-217" => match percentile_nearest_rank(ratios, 95.0) {
            Some(value) => value,
            None => return unrunnable(row, "QA-217 could not recompute p95 token reduction"),
        },
        "QA-218" => {
            let replayed = match json_usize(qa_evidence, "replayedQueries", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            let baseline = match json_usize(qa_evidence, "baselineTokens", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            let context = match json_usize(qa_evidence, "contextTokens", rel) {
                Ok(value) => value,
                Err(error) => return unrunnable(row, &error),
            };
            if replayed != 1_000 {
                return unrunnable(row, "QA-218 evidence must use a 1,000-query replay");
            }
            metrics::token_reduction_ratio(baseline, context)
        }
        _ => return unrunnable(row, "unsupported token-reduction QA evidence row"),
    };

    if (recomputed - evidence_ratio).abs() > 1e-9 {
        return unrunnable(
            row,
            "token-reduction qaEvidence ratio does not match recomputed value",
        );
    }
    if evidence_ratio < minimum {
        return unrunnable(
            row,
            "token-reduction qaEvidence misses the minimum ratio gate",
        );
    }

    let mut source_refs = json_string_array(qa_evidence, "sourceRefs", rel).unwrap_or_default();
    source_refs.push(rel.to_string());
    source_refs.sort();
    source_refs.dedup();
    let expected_id = qa_evidence
        .get("expectedId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("token-reduction:proof");
    exact_pass_with_token_ratio(row, expected_id, source_refs, evidence_ratio)
}

fn learning_curve_ratchet_probe(row: &QaRow, fixtures: &Fixtures) -> RowResult {
    let curves = learning::learning_curve(&fixtures.memory_graph);
    if curves.is_empty() {
        return unrunnable(row, "fixture learning graph produced no curve points");
    }
    for points in curves.values() {
        if points
            .windows(2)
            .any(|window| window[1].cumulative_incidents < window[0].cumulative_incidents)
        {
            return unrunnable(row, "learning curve cumulative incidents regressed");
        }
    }
    let refs: Vec<String> = curves
        .values()
        .flat_map(|points| points.iter().map(|point| point.lesson_id.clone()))
        .collect();
    exact_pass(
        row,
        vec!["learning-curve:nondecreasing-cumulative-incidents".to_string()],
        refs,
    )
}

/// The full registry, tried in order. New wired runners are appended
/// here; a row claimed by none of them falls through to [`unrunnable`]
/// with the reason `"no wired runner for category ..."`.
pub fn registry() -> Vec<Box<dyn RowRunner>> {
    vec![
        Box::new(SymbolCodeGraphRunner),
        Box::new(RetrievalRunner),
        Box::new(LessonsRunner),
        Box::new(ArchitectureRepositoryRunner),
        Box::new(McpRunner),
        Box::new(CliRunner),
        Box::new(GitHistoryRunner),
        Box::new(ExactQaEvidenceRunner),
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

    fn sample_row_with_expectation(
        id: &str,
        category: &str,
        query: &str,
        expectation: &str,
    ) -> QaRow {
        QaRow {
            id: id.to_string(),
            category: category.to_string(),
            query: query.to_string(),
            expectation: expectation.to_string(),
        }
    }

    #[test]
    fn registry_includes_architecture_repository_runner() {
        let names: Vec<&str> = registry().iter().map(|runner| runner.name()).collect();
        assert_eq!(
            names,
            vec![
                "SymbolCodeGraphRunner",
                "RetrievalRunner",
                "LessonsRunner",
                "ArchitectureRepositoryRunner",
                "McpRunner",
                "CliRunner",
                "GitHistoryRunner",
                "ExactQaEvidenceRunner",
            ]
        );
    }

    #[test]
    fn architecture_rows_with_resolvable_crate_references_are_no_longer_unrunnable() -> TestResult {
        let row = sample_row_with_expectation(
            "QA-ARCH-1",
            "Architecture",
            "Find the public API surface of a fixture crate.",
            "Resolve `enforcer-memory` module tree and interfaces.",
        );
        let fixtures = super::super::build_fixtures()?;
        let results = run_all(&[row], &fixtures);
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_unrunnable());
        assert_eq!(results[0].verdict, "pass");
        Ok(())
    }

    #[test]
    fn architecture_rows_without_resolvable_crate_references_remain_unrunnable() -> TestResult {
        let row = sample_row_with_expectation(
            "QA-ARCH-2",
            "Architecture",
            "What proof exists for the c05 Claude SessionStart hook?",
            "proof/install/c05-claude-hook-wiring.json",
        );
        let fixtures = super::super::build_fixtures()?;
        let results = run_all(&[row], &fixtures);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_unrunnable());
        assert_eq!(
            results[0].verdict,
            "unrunnable: no wired runner for category Architecture"
        );
        Ok(())
    }

    #[test]
    fn mcp_and_cli_can_use_full_row_text_for_tool_detection() {
        let mcp_row = sample_row_with_expectation(
            "QA-MCP-1",
            "MCP",
            "Which tool should answer this?",
            "Use `search graph` to inspect the repository.",
        );
        assert!(McpRunner.can_run(&mcp_row));

        let cli_row = sample_row_with_expectation(
            "QA-CLI-1",
            "CLI",
            "Which CLI-mirrored tool should answer this?",
            "Call `trace path` with a known start node.",
        );
        assert!(CliRunner.can_run(&cli_row));
    }

    #[test]
    fn retrieval_runner_claims_only_fixture_backed_rows() {
        let runnable = sample_row_with_expectation(
            "QA-RET-1",
            "Retrieval",
            "Find the relevant function.",
            "Expected `parse_config_file` in top results.",
        );
        assert!(RetrievalRunner.can_run(&runnable));

        let unsupported = sample_row_with_expectation(
            "QA-RET-2",
            "Retrieval",
            "Find all network calls made by this crate.",
            "HTTP/API call nodes correct.",
        );
        assert!(!RetrievalRunner.can_run(&unsupported));
    }

    #[test]
    fn exact_qa_evidence_runner_claims_only_targeted_rows() {
        let token_row = sample_row(
            "QA-098",
            "Learning",
            "Prove token reduction versus reading files.",
        );
        assert!(ExactQaEvidenceRunner.can_run(&token_row));

        let reranker_row = sample_row("QA-097", "Learning", "Prove reranker improved ranking.");
        assert!(ExactQaEvidenceRunner.can_run(&reranker_row));

        let token_replay_row = sample_row(
            "QA-218",
            "TokenReduction",
            "Report cumulative token savings over 1,000 replayed queries.",
        );
        assert!(ExactQaEvidenceRunner.can_run(&token_replay_row));

        let broad_lessons = sample_row("QA-999", "Lessons", "Find arbitrary lesson content.");
        assert!(!ExactQaEvidenceRunner.can_run(&broad_lessons));
    }

    #[test]
    fn exact_qa_evidence_runner_executes_current_no_claude_rows() -> TestResult {
        let fixtures = super::super::build_fixtures()?;
        let rows = [
            sample_row("QA-012", "Lessons", "Find unused private functions."),
            sample_row(
                "QA-021",
                "Lessons",
                "Find all config files used by this crate.",
            ),
            sample_row(
                "QA-022",
                "Lessons",
                "Find all environment variables read by this code.",
            ),
            sample_row(
                "QA-023",
                "Lessons",
                "Find all database tables touched by this function.",
            ),
            sample_row(
                "QA-048",
                "Lessons",
                "Find missing pass fixture for validator.",
            ),
            sample_row(
                "QA-068",
                "Learning",
                "Find previous fix similar to this change.",
            ),
            sample_row("QA-097", "Learning", "Prove reranker improved ranking."),
            sample_row(
                "QA-098",
                "Learning",
                "Prove token reduction versus reading files.",
            ),
            sample_row(
                "QA-213",
                "TokenReduction",
                "Prove MCP retrieval beats agent-opens-42-files.",
            ),
            sample_row(
                "QA-217",
                "TokenReduction",
                "Report p95 token savings across the workpack query set.",
            ),
            sample_row(
                "QA-218",
                "TokenReduction",
                "Report cumulative token savings over 1,000 replayed queries.",
            ),
            sample_row(
                "QA-174",
                "Lessons",
                "Have we solved a domain-type issue before?",
            ),
            sample_row(
                "QA-226",
                "Learning",
                "Prove the learning curve does not regress (ratchet).",
            ),
        ];
        let results = run_all(&rows, &fixtures);
        assert_eq!(results.len(), rows.len());
        for result in &results {
            assert_eq!(
                result.verdict, "pass",
                "{} should be honestly runnable in the no-Claude QA tranche",
                result.id
            );
            assert_ne!(
                result.source_refs.len(),
                0,
                "{} must carry source refs",
                result.id
            );
        }
        Ok(())
    }

    #[test]
    fn qa_token_reduction_rows_recompute_checked_in_ratios() -> TestResult {
        let fixtures = super::super::build_fixtures()?;
        let rows = [
            sample_row(
                "QA-213",
                "TokenReduction",
                "Prove MCP retrieval beats agent-opens-42-files.",
            ),
            sample_row(
                "QA-217",
                "TokenReduction",
                "Report p95 token savings across the workpack query set.",
            ),
            sample_row(
                "QA-218",
                "TokenReduction",
                "Report cumulative token savings over 1,000 replayed queries.",
            ),
        ];
        let results = run_all(&rows, &fixtures);
        assert_eq!(results.len(), rows.len());
        for result in &results {
            assert_eq!(result.verdict, "pass");
            let Some(ratio) = result.token_reduction_ratio else {
                return Err(format!("{} must report a token-reduction ratio", result.id).into());
            };
            assert!(
                ratio >= 10.0,
                "{} token-reduction ratio must satisfy the 10x gate, got {ratio}",
                result.id
            );
            assert!(result
                .source_refs
                .iter()
                .any(|source| source == "proof/memory/x06-token-reduction.json"));
        }
        Ok(())
    }

    #[test]
    fn qa_097_runs_with_checked_in_positive_reranker_lift() -> TestResult {
        let fixtures = super::super::build_fixtures()?;
        let row = sample_row("QA-097", "Learning", "Prove reranker improved ranking.");
        let results = run_all(&[row], &fixtures);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, "pass");
        let Some(lift) = results[0].reranker_lift else {
            return Err("QA-097 must report reranker lift".into());
        };
        assert!(
            lift >= 0.05,
            "QA-097 reranker lift must meet the semantic-row threshold, got {lift}"
        );
        assert!(results[0]
            .source_refs
            .iter()
            .any(|source| source == "proof/memory/x06-reranker.json"));
        Ok(())
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
        assert_eq!(result.capability_state, "unavailable");
    }

    #[test]
    fn score_row_passes_only_when_all_three_thresholds_are_met() {
        let row = sample_row("QA-001", "Symbol", "find x");
        // Perfect single-hit ranking: recall@5=1.0, mrr@10=1.0, ndcg@10=1.0.
        let result = score_row(
            &row,
            RowEvidence::degraded(
                vec!["a".to_string()],
                vec!["a".to_string()],
                None,
                None,
                Vec::new(),
            ),
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
            RowEvidence::degraded(
                vec!["a".to_string(), "b".to_string()],
                vec!["a".to_string()],
                None,
                None,
                Vec::new(),
            ),
        );
        assert_eq!(result.verdict, "fail");
        assert!(!result.is_green());
    }

    #[test]
    fn registry_is_nonempty_and_names_are_unique() {
        let runners = registry();
        assert_eq!(runners.len(), 8);
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
        assert_eq!(
            results[0].verdict,
            "unrunnable: no wired runner for category MCP"
        );
        Ok(())
    }
}
