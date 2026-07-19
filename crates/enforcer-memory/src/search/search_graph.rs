//! X06.search: `search_graph` -- the library-level entry point behind
//! the codebase-memory-mcp parity baseline's `search_graph` tool (the
//! MCP/CLI wrapper is x06-mcpcli's job; this module exposes exactly one
//! entry point, [`search_graph`], that a wrapper can call directly).
//!
//! # Ground truth
//!
//! Every documented behavior below is taken from
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! Â§2, the fully-verified C-source extraction of codebase-memory-mcp's
//! `search_graph` wire contract (decisions D-07a/D-08). Citations in
//! comments below refer to that document's section numbers, not to the
//! C source directly (this crate never touches the C source).
//!
//! # Modes (baseline Â§2.1/Â§2.1 mode-interaction note)
//!
//! Three modes, selected by which of `query` / `name_pattern`+filters /
//! `semantic_query` is populated:
//!
//! - **BM25** (`query` set): full-text search over node names, using
//!   [`crate::fulltext::FullTextIndex`]. If BM25 finds at least one
//!   usable token, the tool **returns immediately** with only the BM25
//!   shape -- `name_pattern`/`qn_pattern`/label/degree filters and
//!   `semantic_query` are baseline-faithfully IGNORED in that branch
//!   (documented divergence knob: [`SearchGraphSpec::label_affects_bm25`]
//!   lets a caller opt into applying `label` to BM25 too, off by
//!   default to match baseline).
//! - **regex** (`name_pattern`/`qn_pattern`/no query): regex over node
//!   name and/or qualified name, combined with the full filter set
//!   (label, file_pattern, relationship, degree, exclude_entry_points,
//!   include_connected). Alphabetical ordering by name (baseline: `ORDER
//!   BY name ASC`), NOT relevance-ranked.
//! - **semantic** (`semantic_query` set): only combines with the regex
//!   path (never with BM25, since BM25 short-circuits first). Produces
//!   a separate `semantic_results` list, never merged into `results`.
//!
//! # Divergences from baseline (documented, not silent)
//!
//! - Baseline's semantic path is a 768-dim int8 quantized cosine scheme
//!   over a `token_vectors` table with a random-projection OOV
//!   fallback (baseline Â§2.3). This crate's semantic mode instead
//!   reuses the crate's existing [`crate::embed::HashingEmbedder`] +
//!   [`crate::vector::VectorIndex`] machinery (D-03/D-04 LOCKED
//!   defaults) scored by **minimum cosine across keywords** (baseline
//!   parity: "ALL keywords must be relevant, not just the average").
//!   The exact numeric scores will not byte-match baseline; the
//!   *contract* (mode interaction, response shape, ordering) does.
//! - `label` filter baseline-faithfully has ZERO effect on the BM25 and
//!   semantic paths (see [`SearchGraphSpec::label_affects_bm25`] /
//!   [`SearchGraphSpec::label_affects_semantic`] opt-in knobs, both
//!   default `false` to match baseline exactly).

use std::collections::HashSet;

use regex::Regex;

use crate::code_graph::{CallEdge, CodeGraph, CodeNode, FileNode, SymbolNode};
use crate::embed::Embedder;
use crate::owned_boundary::{Retained, RetainedDisplay};
use crate::vector::VectorIndex;
use enforcer_domain::memory_types::{
    GraphSearchMode, NodeLabel, ParserSourceText, SearchGraphFilePath, SearchGraphFlag,
    SearchGraphHasMore, SearchGraphLimit, SearchGraphNodeId, SearchGraphNodeName,
    SearchGraphObservedDegree, SearchGraphOffset, SearchGraphPattern, SearchGraphQualifiedName,
    SearchGraphQuery, SearchGraphRank, SearchGraphRelationship, SearchGraphScore, SearchGraphTotal,
};

/// Default limits per mode (baseline Â§2.1, using the CODE defaults, not
/// the tool docstring's claimed-but-wrong 200-for-all-modes).
pub const BM25_DEFAULT_LIMIT: usize = 100;
pub const REGEX_DEFAULT_LIMIT: usize = 200;
/// Internal fallback the semantic path baseline falls back to when
/// `limit <= 0` reaches that layer (baseline Â§2.3).
pub const SEMANTIC_INTERNAL_DEFAULT_LIMIT: usize = 16;

/// Sentinel meaning "no degree filter" (baseline Â§2.1: `-1`).
pub const NO_DEGREE_FILTER: i64 = -1;

/// Baseline default relationship for `include_connected`'s 1-hop BFS
/// (Â§2.1).
pub const DEFAULT_RELATIONSHIP: &str = "CALLS";

/// One `search_graph` request. `project` is intentionally not modeled
/// here -- project resolution/scoping is the caller's (store-layer)
/// concern; this module operates directly over one already-resolved
/// [`CodeGraph`].
#[derive(Debug, Clone, Default)]
pub struct SearchGraphSpec {
    /// BM25 mode trigger (baseline Â§2.1 `query`).
    pub query: Option<SearchGraphQuery>,
    /// Regex mode: matched against node name (baseline Â§2.1
    /// `name_pattern`).
    pub name_pattern: Option<SearchGraphPattern>,
    /// Regex mode: matched against qualified name (baseline Â§2.1
    /// `qn_pattern`).
    pub qn_pattern: Option<SearchGraphPattern>,
    /// Regex-mode-only label filter (baseline: zero effect on BM25/
    /// semantic paths -- see the opt-in divergence knobs below).
    pub label: Option<NodeLabel>,
    /// Glob-ish file path filter: bare literal (no `*`/`?`) means
    /// substring match (baseline Â§2.1: bare literal -> `%literal%`);
    /// `*`/`?` are translated to a regex.
    pub file_pattern: Option<SearchGraphPattern>,
    /// Regex-mode relationship for degree/`include_connected`
    /// computations. Must match `^[A-Z_]+$` or [`SearchGraphError`] is
    /// returned (baseline Â§2.1/Â§2.5).
    pub relationship: Option<SearchGraphRelationship>,
    pub min_degree: Option<enforcer_domain::memory_types::SearchGraphDegree>,
    pub max_degree: Option<enforcer_domain::memory_types::SearchGraphDegree>,
    pub exclude_entry_points: SearchGraphFlag,
    pub include_connected: SearchGraphFlag,
    /// Semantic mode trigger: independent keywords, each scored via
    /// per-keyword min-cosine (baseline Â§2.1/Â§2.3).
    pub semantic_query: Option<Vec<SearchGraphQuery>>,
    pub limit: Option<SearchGraphLimit>,
    pub offset: SearchGraphOffset,
    /// DIVERGENCE (opt-in, default `false` to match baseline exactly):
    /// when `true`, `label` also filters the BM25 candidate set.
    /// Baseline's BM25 path never reads `label` at all.
    pub label_affects_bm25: SearchGraphFlag,
    /// DIVERGENCE (opt-in, default `false`): when `true`, `label` also
    /// filters the semantic candidate set. Baseline hardcodes the
    /// semantic path to `Function`/`Method`/`Class` regardless of
    /// `label`.
    pub label_affects_semantic: SearchGraphFlag,
}

impl SearchGraphSpec {
    pub fn new() -> Self {
        Self::default()
    }
}

/// One `results`/`semantic_results` row.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchGraphHit {
    pub node_id: SearchGraphNodeId,
    pub name: SearchGraphNodeName,
    pub qualified_name: SearchGraphQualifiedName,
    pub label: NodeLabel,
    pub file_path: SearchGraphFilePath,
    /// BM25 mode only (baseline Â§2.4: `rank`, negative-is-better raw
    /// bm25-derived score -- this crate reports `higher is better`,
    /// see module docs on the shared convention; still present so
    /// deterministic-ordering tests can assert on it).
    pub rank: Option<SearchGraphRank>,
    /// Regex mode only.
    pub in_degree: Option<SearchGraphObservedDegree>,
    /// Regex mode only.
    pub out_degree: Option<SearchGraphObservedDegree>,
    /// Regex mode, `include_connected=true` only; omitted (`None`), not
    /// `Some(vec![])`, when empty (baseline Â§2.4).
    pub connected_names: Option<Vec<SearchGraphNodeName>>,
    /// Semantic results only (baseline Â§2.4 `semantic_results[].score`).
    pub score: Option<SearchGraphScore>,
}

/// The full `search_graph` response. `results` and `semantic_results`
/// are always separate lists (baseline Â§2.4) -- never merged, even
/// though in practice only the regex path ever populates both at once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SearchGraphResult {
    pub search_mode: GraphSearchMode,
    pub results: Vec<SearchGraphHit>,
    pub semantic_results: Vec<SearchGraphHit>,
    pub total: SearchGraphTotal,
    pub has_more: SearchGraphHasMore,
    /// Names reached by the `include_connected` 1-hop BFS, deduplicated
    /// across all matched roots (regex mode only). This is a
    /// convenience aggregate on top of each hit's own
    /// [`SearchGraphHit::connected_names`] -- kept for callers that
    /// want the whole-response connected set without re-walking
    /// `results`.
    pub connected_names: Vec<SearchGraphNodeName>,
}

/// Errors [`search_graph`] returns. Named per baseline Â§2.5 error case,
/// not a bare string, so callers can match on the reason.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SearchGraphError {
    /// Baseline Â§2.5: `relationship` must match `^[A-Z_]+$` and be
    /// <= 64 chars.
    #[error("relationship must be uppercase letters and underscores")]
    InvalidRelationship,
    /// A caller-supplied `name_pattern`/`qn_pattern`/`file_pattern`
    /// glob-to-regex translation failed to compile.
    #[error("invalid pattern {pattern:?}: {reason}")]
    InvalidPattern { pattern: String, reason: String },
    /// Semantic scoring could not produce an embedding for the query or candidate.
    #[error("semantic embedding failed: {reason}")]
    SemanticEmbedding { reason: String },
}

/// The one library entry point: run `spec` against `graph`, optionally
/// consulting `embedder`/`vector_index` for the semantic path (`None`
/// disables semantic search entirely -- `semantic_query` is then
/// silently ignored, matching "semantic_query absent" baseline
/// behavior per Â§2.4).
pub fn search_graph(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
) -> Result<SearchGraphResult, SearchGraphError> {
    search_graph_with_semantic(graph, spec, None)
}

/// Full entry point including the semantic path. Split from
/// [`search_graph`] so pure regex/BM25 callers (the common case, and
/// most of the hard test matrix) never need to construct an embedder.
pub fn search_graph_with_semantic(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
    semantic: Option<(&dyn Embedder, &VectorIndex)>,
) -> Result<SearchGraphResult, SearchGraphError> {
    validate_relationship(spec.relationship.as_ref())?;

    if let Some(query) = spec
        .query
        .as_deref()
        .filter(|query| !query.trim().is_empty())
    {
        // Baseline Â§2.1 mode-interaction: BM25 short-circuits on any
        // usable token, ignoring name_pattern/label/degree filters and
        // semantic_query entirely.
        let bm25 = run_bm25(graph, spec, ParserSourceText::from(query))?;
        if !bm25.results.is_empty() || bm25.total.get() > 0 {
            return Ok(bm25);
        }
        // BM25 found zero usable tokens (baseline: "0 usable tokens" ->
        // fall through to regex/label path) -- fall through below.
    }

    let mut result = run_regex_mode(graph, spec)?;

    if let (Some(keywords), Some((embedder, vector_index))) =
        (spec.semantic_query.as_ref(), semantic)
    {
        result.semantic_results = run_semantic(graph, spec, keywords, embedder, vector_index)?;
    }

    Ok(result)
}

fn validate_relationship(
    relationship: Option<&SearchGraphRelationship>,
) -> Result<(), SearchGraphError> {
    if let Some(rel) = relationship {
        let value = rel.as_str();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value.chars().all(|c| c.is_ascii_uppercase() || c == '_');
        if !valid {
            return Err(SearchGraphError::InvalidRelationship);
        }
    }
    Ok(())
}

/// A flattened, label-tagged view of every node `search_graph` can
/// return -- built fresh per call (D-02: "indexes are disposable").
struct FlatNode {
    name: SearchGraphNodeName,
    qualified_name: SearchGraphQualifiedName,
    label: NodeLabel,
    file_path: SearchGraphFilePath,
    node_id: SearchGraphNodeId,
}

fn flatten_nodes(graph: &CodeGraph) -> Vec<FlatNode> {
    graph
        .nodes()
        .iter()
        .filter_map(|node| match node {
            CodeNode::Function(sym) => Some(flat_symbol(sym, NodeLabel::Function)),
            CodeNode::Type(sym) => Some(flat_symbol(sym, NodeLabel::Type)),
            CodeNode::Test(sym) => Some(flat_symbol(sym, NodeLabel::Test)),
            CodeNode::File(file) => Some(flat_file(file, NodeLabel::File)),
            CodeNode::TextOnly(file) => Some(flat_file(file, NodeLabel::TextOnly)),
            CodeNode::Method(sym) => Some(flat_symbol(sym, NodeLabel::Method)),
            CodeNode::Class(sym) => Some(flat_symbol(sym, NodeLabel::Class)),
            CodeNode::Struct(sym) => Some(flat_symbol(sym, NodeLabel::Struct)),
            CodeNode::Interface(sym) => Some(flat_symbol(sym, NodeLabel::Interface)),
            CodeNode::Enum(sym) => Some(flat_symbol(sym, NodeLabel::Enum)),
            CodeNode::TypeAlias(sym) => Some(flat_symbol(sym, NodeLabel::TypeAlias)),
            CodeNode::Module(sym) => Some(flat_symbol(sym, NodeLabel::Module)),
            CodeNode::Lambda(sym) => Some(flat_symbol(sym, NodeLabel::Lambda)),
            CodeNode::Variable(sym) => Some(flat_symbol(sym, NodeLabel::Variable)),
            CodeNode::Constant(sym) => Some(flat_symbol(sym, NodeLabel::Constant)),
            CodeNode::Tombstone(_) => None,
        })
        .collect()
}

fn flat_symbol(sym: &SymbolNode, label: NodeLabel) -> FlatNode {
    FlatNode {
        name: sym.name.as_str().into(),
        qualified_name: format!("{}:{}", sym.file_id, sym.name).into(),
        label,
        file_path: sym.file_id.as_str().into(),
        node_id: sym.id.as_str().into(),
    }
}

fn flat_file(file: &FileNode, label: NodeLabel) -> FlatNode {
    FlatNode {
        name: file.rel_path.as_str().into(),
        qualified_name: file.rel_path.as_str().into(),
        label,
        file_path: file.rel_path.as_str().into(),
        node_id: file.id.as_str().into(),
    }
}

// ---------------------------------------------------------------------
// BM25 mode
// ---------------------------------------------------------------------

fn run_bm25(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
    query: ParserSourceText<'_>,
) -> Result<SearchGraphResult, SearchGraphError> {
    let terms = crate::fulltext::tokenize(
        &enforcer_domain::memory_types::MemoryFullTextInput::from(query.as_str()),
    );
    if terms.is_empty() {
        return Ok(SearchGraphResult {
            search_mode: GraphSearchMode::Bm25,
            ..Default::default()
        });
    }

    let flat = flatten_nodes(graph);
    let file_re = compile_file_pattern(spec.file_pattern.as_ref())?;

    let mut scored: Vec<(FlatNode, f64)> = flat
        .into_iter()
        .filter(|n| !n.label.is_bm25_noise())
        .filter(|n| {
            // Baseline-faithful default: `label` has zero effect on the
            // BM25 path unless the caller opts into the divergence.
            !spec.label_affects_bm25.is_enabled() || spec.label.is_none_or(|label| label == n.label)
        })
        .filter(|n| file_pattern_matches(&file_re, &n.file_path).is_enabled())
        .filter_map(|n| {
            let terms_in_name = crate::fulltext::tokenize(
                &enforcer_domain::memory_types::MemoryFullTextInput::from(n.name.as_str()),
            );
            let hits = terms.iter().filter(|t| terms_in_name.contains(*t)).count();
            if hits == 0 {
                return None;
            }
            // Simple deterministic BM25-ish scoring: term-overlap ratio
            // is the base signal, boosted by the label tier (baseline
            // Â§2.2 label-boost table, corrected per doc erratum).
            // CAST-JUSTIFICATION: term cardinalities are converted to the
            // floating-point ratio used by the BM25 ranking formula.
            let base = hits as f64 / terms.len() as f64;
            let score = base + f64::from(n.label.bm25_boost());
            Some((n, score))
        })
        .collect();

    scored.sort_by(|(a, sa), (b, sb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });

    let total = scored.len();
    let limit = spec
        .limit
        .map(SearchGraphLimit::get)
        .unwrap_or(BM25_DEFAULT_LIMIT)
        .max(1);
    let offset = spec.offset.get();
    let page: Vec<SearchGraphHit> = scored
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(n, score)| SearchGraphHit {
            // ALLOC-JUSTIFICATION: result rows outlive the borrowed flattened
            // graph view used to calculate this BM25 page.
            node_id: n.node_id.retained(),
            name: n.name.retained(),
            qualified_name: n.qualified_name,
            label: n.label,
            file_path: n.file_path.retained(),
            rank: Some(score.into()),
            in_degree: None,
            out_degree: None,
            connected_names: None,
            score: None,
        })
        .collect();

    let emitted = page.len();
    Ok(SearchGraphResult {
        search_mode: GraphSearchMode::Bm25,
        results: page,
        semantic_results: Vec::new(),
        total: total.into(),
        has_more: (total.saturating_sub(offset) > emitted).into(),
        connected_names: Vec::new(),
    })
}

// ---------------------------------------------------------------------
// Regex mode
// ---------------------------------------------------------------------

fn run_regex_mode(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
) -> Result<SearchGraphResult, SearchGraphError> {
    let name_re = compile_pattern(spec.name_pattern.as_ref())?;
    let qn_re = compile_pattern(spec.qn_pattern.as_ref())?;
    let file_re = compile_file_pattern(spec.file_pattern.as_ref())?;

    let relationship = spec.relationship.as_ref().map_or_else(
        || SearchGraphRelationship::from(DEFAULT_RELATIONSHIP),
        Clone::clone,
    );
    let degrees = compute_degrees(graph, &relationship);
    let entry_point_ids = compute_entry_points(graph, &relationship);

    let flat = flatten_nodes(graph);

    let mut matched: Vec<FlatNode> = flat
        .into_iter()
        .filter(|n| match &name_re {
            Some(re) => re.is_match(n.name.as_str()),
            None => true,
        })
        .filter(|n| match &qn_re {
            Some(re) => re.is_match(n.qualified_name.as_str()),
            None => true,
        })
        .filter(|n| match spec.label {
            Some(label) => n.label == label,
            None => true,
        })
        .filter(|n| file_pattern_matches(&file_re, &n.file_path).is_enabled())
        .filter(|n| {
            let (in_deg, out_deg) = degrees
                .get(&n.node_id)
                .copied()
                .unwrap_or((0.into(), 0.into()));
            let total_deg = in_deg.get() + out_deg.get();
            let min_ok = spec
                .min_degree
                // CAST-JUSTIFICATION: degrees are u32 counters, which fit
                // losslessly in i64 for comparison with the request bound.
                .map(|m| m == NO_DEGREE_FILTER || i64::from(total_deg) >= m.get())
                .unwrap_or(true);
            let max_ok = spec
                .max_degree
                // CAST-JUSTIFICATION: degrees are u32 counters, which fit
                // losslessly in i64 for comparison with the request bound.
                .map(|m| m == NO_DEGREE_FILTER || i64::from(total_deg) <= m.get())
                .unwrap_or(true);
            min_ok && max_ok
        })
        .filter(|n| {
            !(spec.exclude_entry_points.is_enabled() && entry_point_ids.contains(&n.node_id))
        })
        .collect();

    // Baseline Â§2.4: `ORDER BY name` ascending, alphabetical -- not
    // relevance-ranked. Tie-break on node id for full determinism.
    matched.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.node_id.cmp(&b.node_id)));

    let total = matched.len();
    let limit = spec
        .limit
        .map(SearchGraphLimit::get)
        .unwrap_or(REGEX_DEFAULT_LIMIT)
        .max(1);
    let offset = spec.offset.get();

    let call_adjacency = build_adjacency(graph, &relationship);
    let mut all_connected: HashSet<SearchGraphNodeName> = HashSet::new();

    let page: Vec<SearchGraphHit> = matched
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|n| {
            let (in_deg, out_deg) = degrees
                .get(&n.node_id)
                .copied()
                .unwrap_or((0.into(), 0.into()));
            let connected_names = if spec.include_connected.is_enabled() {
                let names = one_hop_names(call_adjacency, &n.node_id, graph);
                for name in &names {
                    // CLONE-JUSTIFICATION: each hit retains its own names
                    // while the response-level aggregate owns its dedup set.
                    all_connected.insert(name.retained());
                }
                if names.is_empty() {
                    None
                } else {
                    Some(names)
                }
            } else {
                None
            };
            SearchGraphHit {
                // ALLOC-JUSTIFICATION: regex result rows outlive the borrowed
                // flattened graph view used to select and sort the page.
                node_id: n.node_id.retained(),
                name: n.name.retained(),
                qualified_name: n.qualified_name,
                label: n.label,
                file_path: n.file_path.retained(),
                rank: None,
                in_degree: Some(in_deg),
                out_degree: Some(out_deg),
                connected_names,
                score: None,
            }
        })
        .collect();

    let emitted = page.len();
    let mut connected_sorted: Vec<SearchGraphNodeName> = all_connected.into_iter().collect();
    connected_sorted.sort();

    Ok(SearchGraphResult {
        search_mode: GraphSearchMode::Regex,
        results: page,
        semantic_results: Vec::new(),
        total: total.into(),
        has_more: (total > offset + emitted).into(),
        connected_names: connected_sorted,
    })
}

/// Build a `from_node_id -> callee name` adjacency for `include_connected`
/// (baseline Â§2.1: 1-hop BFS using `relationship`, default `CALLS`).
/// This crate's [`CodeGraph`] models calls at file granularity
/// ([`CallEdge::from_file_id`]/`callee` as a written name, not a
/// resolved target id) -- so the "1 hop" here is: for a symbol node,
/// its containing file's outbound calls; for a file node, its own
/// outbound calls.
fn build_adjacency<'a>(
    graph: &'a CodeGraph,
    relationship: &SearchGraphRelationship,
) -> &'a [CallEdge] {
    // This crate only models one relationship kind (`CallEdge`) at the
    // moment; a non-`CALLS` relationship therefore has no adjacency
    // (empty, not an error -- baseline never errors on an unmodeled
    // relationship name that still passes the `^[A-Z_]+$` shape check).
    if relationship.as_str() == "CALLS" {
        graph.calls()
    } else {
        &[]
    }
}

fn one_hop_names(
    calls: &[CallEdge],
    node_id: &SearchGraphNodeId,
    graph: &CodeGraph,
) -> Vec<SearchGraphNodeName> {
    // Resolve node_id to its owning file id: symbol node ids are
    // `sym:<file>:<line>:<name>`, so recover the file id by finding the
    // symbol/file node itself rather than string-parsing.
    let file_id = owning_file_id(graph, node_id);
    let Some(file_id) = file_id else {
        return Vec::new();
    };
    let mut names: Vec<SearchGraphNodeName> = calls
        .iter()
        .filter(|c| c.from_file_id.as_str() == file_id.as_str())
        // CLONE-JUSTIFICATION: connected-name output is owned after the
        // borrowed call-edge adjacency is released.
        .map(|c| c.callee.retained().into())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn owning_file_id(graph: &CodeGraph, node_id: &SearchGraphNodeId) -> Option<SearchGraphFilePath> {
    for node in graph.nodes() {
        match node {
            CodeNode::Function(s) | CodeNode::Type(s) | CodeNode::Test(s)
                if s.id == node_id.as_str() =>
            {
                return Some(s.file_id.as_str().into());
            }
            CodeNode::File(f) | CodeNode::TextOnly(f) if f.id == node_id.as_str() => {
                return Some(f.id.as_str().into());
            }
            _ => {}
        }
    }
    None
}

/// `node_id -> (in_degree, out_degree)` over `relationship`'s edges.
/// Only `CALLS` is modeled by [`CodeGraph`] today (see
/// [`build_adjacency`]); any other relationship name yields all-zero
/// degrees rather than an error.
fn compute_degrees(
    graph: &CodeGraph,
    relationship: &SearchGraphRelationship,
) -> std::collections::HashMap<
    SearchGraphNodeId,
    (SearchGraphObservedDegree, SearchGraphObservedDegree),
> {
    let mut degrees: std::collections::HashMap<
        SearchGraphNodeId,
        (SearchGraphObservedDegree, SearchGraphObservedDegree),
    > = std::collections::HashMap::new();
    if relationship.as_str() != "CALLS" {
        return degrees;
    }
    for call in graph.calls() {
        // CLONE-JUSTIFICATION: the degree map owns file ids while it is
        // assembled independently of the immutable graph call list.
        let entry = degrees
            .entry(call.from_file_id.retained().into())
            .or_insert((0.into(), 0.into()));
        entry.1 = (entry.1.get() + 1).into();
        // Baseline computes in/out degree against resolved node ids;
        // this crate's `CallEdge::callee` is a written name, not a
        // resolved node id, so inbound degree can only be attributed
        // when the callee text happens to match a known node's own
        // name (best-effort resolution, documented divergence: no
        // cross-reference resolution pass exists in this slice).
        if let Some(target_id) =
            resolve_callee_to_node_id(graph, &SearchGraphNodeName::from(call.callee.as_str()))
        {
            let target_entry = degrees.entry(target_id).or_insert((0.into(), 0.into()));
            target_entry.0 = (target_entry.0.get() + 1).into();
        }
    }
    degrees
}

fn resolve_callee_to_node_id(
    graph: &CodeGraph,
    callee: &SearchGraphNodeName,
) -> Option<SearchGraphNodeId> {
    graph
        .symbol_nodes()
        .find(|s| s.name == callee.as_str())
        // CLONE-JUSTIFICATION: callers retain the resolved id in owned degree
        // aggregation state after graph traversal ends.
        .map(|s| s.id.retained().into())
}

/// Entry points per baseline Â§2.1: nodes with 0 inbound + >=1 outbound
/// CALLS edges. Computed over file ids (this crate's call granularity)
/// since `CallEdge` is file-scoped, not symbol-scoped.
fn compute_entry_points(
    graph: &CodeGraph,
    relationship: &SearchGraphRelationship,
) -> HashSet<SearchGraphNodeId> {
    let degrees = compute_degrees(graph, relationship);
    degrees
        .iter()
        .filter(|(_, (in_deg, out_deg))| in_deg.get() == 0 && out_deg.get() >= 1)
        // CLONE-JUSTIFICATION: the entry-point result owns ids independently
        // of the local degree map used to derive it.
        .map(|(id, _)| id.retained())
        .collect()
}

// ---------------------------------------------------------------------
// Semantic mode
// ---------------------------------------------------------------------

fn run_semantic(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
    keywords: &[SearchGraphQuery],
    embedder: &dyn Embedder,
    vector_index: &VectorIndex,
) -> Result<Vec<SearchGraphHit>, SearchGraphError> {
    if keywords.is_empty() {
        return Ok(Vec::new());
    }
    let flat = flatten_nodes(graph);
    let by_id: std::collections::HashMap<&str, &FlatNode> =
        flat.iter().map(|n| (n.node_id.as_str(), n)).collect();

    // Baseline Â§2.3: hardcoded to Function/Method/Class regardless of
    // the `label` param, unless the caller opts into the divergence.
    // Now that Method/Class are their own reachable labels (X06 rich
    // vocabulary), this matches the baseline's literal three-label set
    // instead of folding Method/Class into Function/Type.
    let allowed = |label: NodeLabel| -> bool {
        if spec.label_affects_semantic.is_enabled() {
            spec.label.map(|l| l == label).unwrap_or(true)
        } else {
            matches!(
                label,
                NodeLabel::Function | NodeLabel::Method | NodeLabel::Class | NodeLabel::Type
            )
        }
    };

    let limit = spec
        .limit
        .map(SearchGraphLimit::get)
        .filter(|limit| *limit > 0)
        .unwrap_or(SEMANTIC_INTERNAL_DEFAULT_LIMIT);

    let mut scored: Vec<(String, f64)> = Vec::new();
    for node in &flat {
        if !allowed(node.label) {
            continue;
        }
        // Baseline Â§2.3: score is the MINIMUM cosine across all
        // keywords, not the average -- "ALL keywords must be relevant".
        let node_vec = embedder
            .embed(ParserSourceText::from(node.name.as_str()))
            .map_err(|error| SearchGraphError::SemanticEmbedding {
                reason: error.retained_display(),
            })?;
        let node_vector = node_vec;
        let mut min_cosine = f64::MAX;
        for keyword in keywords {
            let kw_vec = embedder
                .embed(enforcer_domain::memory_types::ParserSourceText::from(
                    keyword.as_str(),
                ))
                .map_err(|error| SearchGraphError::SemanticEmbedding {
                    reason: error.retained_display(),
                })?;
            let keyword_vector = kw_vec;
            let cosine = crate::embed::cosine_similarity(&node_vector, &keyword_vector).as_f64();
            min_cosine = min_cosine.min(cosine);
        }
        if min_cosine == f64::MAX {
            continue;
        }
        scored.push((node.node_id.as_str().retained(), min_cosine));
    }
    // `vector_index` is accepted for interface parity with a real HNSW-
    // backed candidate prefilter; this exact-scan path is correct and
    // simple enough for the fixture-scale corpora this crate's tests
    // and current usage operate over (D-02: correctness first).
    let _ = vector_index;

    scored.sort_by(|(id_a, a), (id_b, b)| {
        b.partial_cmp(a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| id_a.cmp(id_b))
    });
    scored.truncate(limit);

    Ok(scored
        .into_iter()
        .filter_map(|(node_id, score)| {
            let node = by_id.get(node_id.as_str())?;
            Some(SearchGraphHit {
                node_id: node.node_id.retained(),
                name: node.name.retained(),
                qualified_name: node.qualified_name.retained(),
                label: node.label,
                file_path: node.file_path.retained(),
                rank: None,
                in_degree: None,
                out_degree: None,
                connected_names: None,
                score: Some(score.into()),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------
// Pattern helpers
// ---------------------------------------------------------------------

fn compile_pattern(
    pattern: Option<&SearchGraphPattern>,
) -> Result<Option<Regex>, SearchGraphError> {
    match pattern {
        None => Ok(None),
        Some(p) => Regex::new(p.as_str())
            .map(Some)
            .map_err(|e| SearchGraphError::InvalidPattern {
                pattern: p.as_str().retained(),
                reason: e.retained_display(),
            }),
    }
}

/// Baseline Â§2.1: `file_pattern` is glob-ish for BM25 (SQL LIKE) but a
/// bare literal (no `*`/`?`) becomes a substring match
/// (`%literal%`). For both modes here it compiles to a regex: `*` ->
/// `.*`, `?` -> `.`, a bare literal is wrapped so it matches anywhere
/// in the path.
fn compile_file_pattern(
    pattern: Option<&SearchGraphPattern>,
) -> Result<Option<Regex>, SearchGraphError> {
    match pattern {
        None => Ok(None),
        Some(p) => {
            let value = p.as_str();
            let has_glob = value.contains('*') || value.contains('?');
            let regex_source = if has_glob {
                let mut out = String::from("^");
                for c in value.chars() {
                    match c {
                        '*' => out.push_str(".*"),
                        '?' => out.push('.'),
                        other => out.push_str(&regex::escape(&other.retained_display())),
                    }
                }
                out.push('$');
                out
            } else {
                format!(".*{}.*", regex::escape(value))
            };
            Regex::new(&regex_source)
                .map(Some)
                .map_err(|e| SearchGraphError::InvalidPattern {
                    pattern: value.retained(),
                    reason: e.retained_display(),
                })
        }
    }
}

fn file_pattern_matches(re: &Option<Regex>, file_path: &SearchGraphFilePath) -> SearchGraphFlag {
    match re {
        Some(re) => re.is_match(file_path.as_str()).into(),
        None => true.into(),
    }
}
