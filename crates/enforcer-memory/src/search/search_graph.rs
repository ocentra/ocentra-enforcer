//! X06.search: `search_graph` -- the library-level entry point behind
//! the codebase-memory-mcp parity baseline's `search_graph` tool (the
//! MCP/CLI wrapper is x06-mcpcli's job; this module exposes exactly one
//! entry point, [`search_graph`], that a wrapper can call directly).
//!
//! # Ground truth
//!
//! Every documented behavior below is taken from
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §2, the fully-verified C-source extraction of codebase-memory-mcp's
//! `search_graph` wire contract (decisions D-07a/D-08). Citations in
//! comments below refer to that document's section numbers, not to the
//! C source directly (this crate never touches the C source).
//!
//! # Modes (baseline §2.1/§2.1 mode-interaction note)
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
//!   fallback (baseline §2.3). This crate's semantic mode instead
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
use crate::vector::VectorIndex;

/// Node label as exposed by `search_graph` (mirrors baseline's node
/// `label` values that are actually reachable from [`CodeGraph`] --
/// `File`/`Folder`/`Module`/`Section`/`Variable`/`Project` from the
/// baseline's fuller label set have no [`CodeGraph`] analogue in this
/// slice and are simply never produced).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    Function,
    Type,
    Test,
    File,
    TextOnly,
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeLabel::Function => "Function",
            NodeLabel::Type => "Type",
            NodeLabel::Test => "Test",
            NodeLabel::File => "File",
            NodeLabel::TextOnly => "TextOnly",
        }
    }

    /// Baseline §2.2 label-boost table, corrected per the doc's
    /// erratum: the +5.0 tier covers Class/Interface/Type/Enum (four
    /// labels). This crate's node set has no separate
    /// Class/Interface/Enum labels, so [`NodeLabel::Type`] alone
    /// occupies that tier; [`NodeLabel::Function`]/[`NodeLabel::Test`]
    /// occupy the +10.0 tier (baseline: Function, Method).
    fn bm25_boost(&self) -> f64 {
        match self {
            NodeLabel::Function | NodeLabel::Test => 10.0,
            NodeLabel::Type => 5.0,
            NodeLabel::File | NodeLabel::TextOnly => 0.0,
        }
    }

    /// Baseline §2.2: BM25 path excludes a fixed noise-label set
    /// (`File`,`Folder`,`Module`,`Section`,`Variable`,`Project`)
    /// independent of the `label` filter. This crate's analogue:
    /// `File`/`TextOnly` nodes are noise for BM25 purposes.
    fn is_bm25_noise(&self) -> bool {
        matches!(self, NodeLabel::File | NodeLabel::TextOnly)
    }
}

/// Default limits per mode (baseline §2.1, using the CODE defaults, not
/// the tool docstring's claimed-but-wrong 200-for-all-modes).
pub const BM25_DEFAULT_LIMIT: usize = 100;
pub const REGEX_DEFAULT_LIMIT: usize = 200;
/// Internal fallback the semantic path baseline falls back to when
/// `limit <= 0` reaches that layer (baseline §2.3).
pub const SEMANTIC_INTERNAL_DEFAULT_LIMIT: usize = 16;

/// Sentinel meaning "no degree filter" (baseline §2.1: `-1`).
pub const NO_DEGREE_FILTER: i64 = -1;

/// Baseline default relationship for `include_connected`'s 1-hop BFS
/// (§2.1).
pub const DEFAULT_RELATIONSHIP: &str = "CALLS";

/// One `search_graph` request. `project` is intentionally not modeled
/// here -- project resolution/scoping is the caller's (store-layer)
/// concern; this module operates directly over one already-resolved
/// [`CodeGraph`].
#[derive(Debug, Clone, Default)]
pub struct SearchGraphSpec {
    /// BM25 mode trigger (baseline §2.1 `query`).
    pub query: Option<String>,
    /// Regex mode: matched against node name (baseline §2.1
    /// `name_pattern`).
    pub name_pattern: Option<String>,
    /// Regex mode: matched against qualified name (baseline §2.1
    /// `qn_pattern`).
    pub qn_pattern: Option<String>,
    /// Regex-mode-only label filter (baseline: zero effect on BM25/
    /// semantic paths -- see the opt-in divergence knobs below).
    pub label: Option<NodeLabel>,
    /// Glob-ish file path filter: bare literal (no `*`/`?`) means
    /// substring match (baseline §2.1: bare literal -> `%literal%`);
    /// `*`/`?` are translated to a regex.
    pub file_pattern: Option<String>,
    /// Regex-mode relationship for degree/`include_connected`
    /// computations. Must match `^[A-Z_]+$` or [`SearchGraphError`] is
    /// returned (baseline §2.1/§2.5).
    pub relationship: Option<String>,
    pub min_degree: Option<i64>,
    pub max_degree: Option<i64>,
    pub exclude_entry_points: bool,
    pub include_connected: bool,
    /// Semantic mode trigger: independent keywords, each scored via
    /// per-keyword min-cosine (baseline §2.1/§2.3).
    pub semantic_query: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: usize,
    /// DIVERGENCE (opt-in, default `false` to match baseline exactly):
    /// when `true`, `label` also filters the BM25 candidate set.
    /// Baseline's BM25 path never reads `label` at all.
    pub label_affects_bm25: bool,
    /// DIVERGENCE (opt-in, default `false`): when `true`, `label` also
    /// filters the semantic candidate set. Baseline hardcodes the
    /// semantic path to `Function`/`Method`/`Class` regardless of
    /// `label`.
    pub label_affects_semantic: bool,
}

impl SearchGraphSpec {
    pub fn new() -> Self {
        Self::default()
    }
}

/// One `results`/`semantic_results` row.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchGraphHit {
    pub name: String,
    pub qualified_name: String,
    pub label: &'static str,
    pub file_path: String,
    /// BM25 mode only (baseline §2.4: `rank`, negative-is-better raw
    /// bm25-derived score -- this crate reports `higher is better`,
    /// see module docs on the shared convention; still present so
    /// deterministic-ordering tests can assert on it).
    pub rank: Option<f64>,
    /// Regex mode only.
    pub in_degree: Option<u32>,
    /// Regex mode only.
    pub out_degree: Option<u32>,
    /// Regex mode, `include_connected=true` only; omitted (`None`), not
    /// `Some(vec![])`, when empty (baseline §2.4).
    pub connected_names: Option<Vec<String>>,
    /// Semantic results only (baseline §2.4 `semantic_results[].score`).
    pub score: Option<f64>,
}

/// The full `search_graph` response. `results` and `semantic_results`
/// are always separate lists (baseline §2.4) -- never merged, even
/// though in practice only the regex path ever populates both at once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SearchGraphResult {
    pub search_mode: SearchMode,
    pub results: Vec<SearchGraphHit>,
    pub semantic_results: Vec<SearchGraphHit>,
    pub total: usize,
    pub has_more: bool,
    /// Names reached by the `include_connected` 1-hop BFS, deduplicated
    /// across all matched roots (regex mode only). This is a
    /// convenience aggregate on top of each hit's own
    /// [`SearchGraphHit::connected_names`] -- kept for callers that
    /// want the whole-response connected set without re-walking
    /// `results`.
    pub connected_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Bm25,
    Regex,
}

/// Errors [`search_graph`] returns. Named per baseline §2.5 error case,
/// not a bare string, so callers can match on the reason.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SearchGraphError {
    /// Baseline §2.5: `relationship` must match `^[A-Z_]+$` and be
    /// <= 64 chars.
    #[error("relationship must be uppercase letters and underscores")]
    InvalidRelationship,
    /// A caller-supplied `name_pattern`/`qn_pattern`/`file_pattern`
    /// glob-to-regex translation failed to compile.
    #[error("invalid pattern {pattern:?}: {reason}")]
    InvalidPattern { pattern: String, reason: String },
}

/// The one library entry point: run `spec` against `graph`, optionally
/// consulting `embedder`/`vector_index` for the semantic path (`None`
/// disables semantic search entirely -- `semantic_query` is then
/// silently ignored, matching "semantic_query absent" baseline
/// behavior per §2.4).
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
    validate_relationship(spec.relationship.as_deref())?;

    let has_query = spec
        .query
        .as_deref()
        .map(|q| !q.trim().is_empty())
        .unwrap_or(false);

    if has_query {
        // Baseline §2.1 mode-interaction: BM25 short-circuits on any
        // usable token, ignoring name_pattern/label/degree filters and
        // semantic_query entirely.
        let bm25 = run_bm25(graph, spec)?;
        if !bm25.results.is_empty() || bm25.total > 0 {
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

fn validate_relationship(relationship: Option<&str>) -> Result<(), SearchGraphError> {
    if let Some(rel) = relationship {
        let valid = !rel.is_empty()
            && rel.len() <= 64
            && rel.chars().all(|c| c.is_ascii_uppercase() || c == '_');
        if !valid {
            return Err(SearchGraphError::InvalidRelationship);
        }
    }
    Ok(())
}

/// A flattened, label-tagged view of every node `search_graph` can
/// return -- built fresh per call (D-02: "indexes are disposable").
struct FlatNode<'a> {
    name: &'a str,
    qualified_name: String,
    label: NodeLabel,
    file_path: &'a str,
    node_id: &'a str,
}

fn flatten_nodes(graph: &CodeGraph) -> Vec<FlatNode<'_>> {
    graph
        .nodes()
        .iter()
        .filter_map(|node| match node {
            CodeNode::Function(sym) => Some(flat_symbol(sym, NodeLabel::Function)),
            CodeNode::Type(sym) => Some(flat_symbol(sym, NodeLabel::Type)),
            CodeNode::Test(sym) => Some(flat_symbol(sym, NodeLabel::Test)),
            CodeNode::File(file) => Some(flat_file(file, NodeLabel::File)),
            CodeNode::TextOnly(file) => Some(flat_file(file, NodeLabel::TextOnly)),
            CodeNode::Tombstone(_) => None,
        })
        .collect()
}

fn flat_symbol(sym: &SymbolNode, label: NodeLabel) -> FlatNode<'_> {
    FlatNode {
        name: &sym.name,
        qualified_name: format!("{}:{}", sym.file_id, sym.name),
        label,
        file_path: &sym.file_id,
        node_id: &sym.id,
    }
}

fn flat_file(file: &FileNode, label: NodeLabel) -> FlatNode<'_> {
    FlatNode {
        name: &file.rel_path,
        qualified_name: file.rel_path.clone(),
        label,
        file_path: &file.rel_path,
        node_id: &file.id,
    }
}

// ---------------------------------------------------------------------
// BM25 mode
// ---------------------------------------------------------------------

fn run_bm25(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
) -> Result<SearchGraphResult, SearchGraphError> {
    let query = spec.query.as_deref().unwrap_or_default();
    let terms = crate::fulltext::tokenize(query);
    if terms.is_empty() {
        return Ok(SearchGraphResult {
            search_mode: SearchMode::Bm25,
            ..Default::default()
        });
    }

    let flat = flatten_nodes(graph);
    let file_re = compile_file_pattern(spec.file_pattern.as_deref())?;

    let mut scored: Vec<(FlatNode<'_>, f64)> = flat
        .into_iter()
        .filter(|n| !n.label.is_bm25_noise())
        .filter(|n| {
            // Baseline-faithful default: `label` has zero effect on the
            // BM25 path unless the caller opts into the divergence.
            !spec.label_affects_bm25 || spec.label.is_none_or(|label| label == n.label)
        })
        .filter(|n| file_pattern_matches(&file_re, n.file_path))
        .filter_map(|n| {
            let terms_in_name = crate::fulltext::tokenize(n.name);
            let hits = terms.iter().filter(|t| terms_in_name.contains(*t)).count();
            if hits == 0 {
                return None;
            }
            // Simple deterministic BM25-ish scoring: term-overlap ratio
            // is the base signal, boosted by the label tier (baseline
            // §2.2 label-boost table, corrected per doc erratum).
            let base = hits as f64 / terms.len() as f64;
            let score = base + n.label.bm25_boost();
            Some((n, score))
        })
        .collect();

    scored.sort_by(|(a, sa), (b, sb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(b.name))
            .then_with(|| a.node_id.cmp(b.node_id))
    });

    let total = scored.len();
    let limit = spec.limit.unwrap_or(BM25_DEFAULT_LIMIT).max(1);
    let offset = spec.offset;
    let page: Vec<SearchGraphHit> = scored
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(n, score)| SearchGraphHit {
            name: n.name.to_owned(),
            qualified_name: n.qualified_name,
            label: n.label.as_str(),
            file_path: n.file_path.to_owned(),
            rank: Some(score),
            in_degree: None,
            out_degree: None,
            connected_names: None,
            score: None,
        })
        .collect();

    let emitted = page.len();
    Ok(SearchGraphResult {
        search_mode: SearchMode::Bm25,
        results: page,
        semantic_results: Vec::new(),
        total,
        has_more: total > offset + emitted,
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
    let name_re = compile_pattern(spec.name_pattern.as_deref())?;
    let qn_re = compile_pattern(spec.qn_pattern.as_deref())?;
    let file_re = compile_file_pattern(spec.file_pattern.as_deref())?;

    let relationship = spec.relationship.as_deref().unwrap_or(DEFAULT_RELATIONSHIP);
    let degrees = compute_degrees(graph, relationship);
    let entry_point_ids = compute_entry_points(graph, relationship);

    let flat = flatten_nodes(graph);

    let mut matched: Vec<FlatNode<'_>> = flat
        .into_iter()
        .filter(|n| match &name_re {
            Some(re) => re.is_match(n.name),
            None => true,
        })
        .filter(|n| match &qn_re {
            Some(re) => re.is_match(&n.qualified_name),
            None => true,
        })
        .filter(|n| match spec.label {
            Some(label) => n.label == label,
            None => true,
        })
        .filter(|n| file_pattern_matches(&file_re, n.file_path))
        .filter(|n| {
            let (in_deg, out_deg) = degrees.get(n.node_id).copied().unwrap_or((0, 0));
            let total_deg = in_deg + out_deg;
            let min_ok = spec
                .min_degree
                .map(|m| m == NO_DEGREE_FILTER || total_deg as i64 >= m)
                .unwrap_or(true);
            let max_ok = spec
                .max_degree
                .map(|m| m == NO_DEGREE_FILTER || total_deg as i64 <= m)
                .unwrap_or(true);
            min_ok && max_ok
        })
        .filter(|n| !(spec.exclude_entry_points && entry_point_ids.contains(n.node_id)))
        .collect();

    // Baseline §2.4: `ORDER BY name` ascending, alphabetical -- not
    // relevance-ranked. Tie-break on node id for full determinism.
    matched.sort_by(|a, b| a.name.cmp(b.name).then_with(|| a.node_id.cmp(b.node_id)));

    let total = matched.len();
    let limit = spec.limit.unwrap_or(REGEX_DEFAULT_LIMIT).max(1);
    let offset = spec.offset;

    let call_adjacency = build_adjacency(graph, relationship);
    let mut all_connected: HashSet<String> = HashSet::new();

    let page: Vec<SearchGraphHit> = matched
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|n| {
            let (in_deg, out_deg) = degrees.get(n.node_id).copied().unwrap_or((0, 0));
            let connected_names = if spec.include_connected {
                let names = one_hop_names(call_adjacency, n.node_id, graph);
                for name in &names {
                    all_connected.insert(name.clone());
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
                name: n.name.to_owned(),
                qualified_name: n.qualified_name,
                label: n.label.as_str(),
                file_path: n.file_path.to_owned(),
                rank: None,
                in_degree: Some(in_deg),
                out_degree: Some(out_deg),
                connected_names,
                score: None,
            }
        })
        .collect();

    let emitted = page.len();
    let mut connected_sorted: Vec<String> = all_connected.into_iter().collect();
    connected_sorted.sort();

    Ok(SearchGraphResult {
        search_mode: SearchMode::Regex,
        results: page,
        semantic_results: Vec::new(),
        total,
        has_more: total > offset + emitted,
        connected_names: connected_sorted,
    })
}

/// Build a `from_node_id -> callee name` adjacency for `include_connected`
/// (baseline §2.1: 1-hop BFS using `relationship`, default `CALLS`).
/// This crate's [`CodeGraph`] models calls at file granularity
/// ([`CallEdge::from_file_id`]/`callee` as a written name, not a
/// resolved target id) -- so the "1 hop" here is: for a symbol node,
/// its containing file's outbound calls; for a file node, its own
/// outbound calls.
fn build_adjacency<'a>(graph: &'a CodeGraph, relationship: &str) -> &'a [CallEdge] {
    // This crate only models one relationship kind (`CallEdge`) at the
    // moment; a non-`CALLS` relationship therefore has no adjacency
    // (empty, not an error -- baseline never errors on an unmodeled
    // relationship name that still passes the `^[A-Z_]+$` shape check).
    if relationship == "CALLS" {
        graph.calls()
    } else {
        &[]
    }
}

fn one_hop_names(calls: &[CallEdge], node_id: &str, graph: &CodeGraph) -> Vec<String> {
    // Resolve node_id to its owning file id: symbol node ids are
    // `sym:<file>:<line>:<name>`, so recover the file id by finding the
    // symbol/file node itself rather than string-parsing.
    let file_id = owning_file_id(graph, node_id);
    let Some(file_id) = file_id else {
        return Vec::new();
    };
    let mut names: Vec<String> = calls
        .iter()
        .filter(|c| c.from_file_id == file_id)
        .map(|c| c.callee.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn owning_file_id<'a>(graph: &'a CodeGraph, node_id: &str) -> Option<&'a str> {
    for node in graph.nodes() {
        match node {
            CodeNode::Function(s) | CodeNode::Type(s) | CodeNode::Test(s) if s.id == node_id => {
                return Some(&s.file_id);
            }
            CodeNode::File(f) | CodeNode::TextOnly(f) if f.id == node_id => {
                return Some(&f.id);
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
    relationship: &str,
) -> std::collections::HashMap<String, (u32, u32)> {
    let mut degrees: std::collections::HashMap<String, (u32, u32)> =
        std::collections::HashMap::new();
    if relationship != "CALLS" {
        return degrees;
    }
    for call in graph.calls() {
        let entry = degrees.entry(call.from_file_id.clone()).or_insert((0, 0));
        entry.1 += 1;
        // Baseline computes in/out degree against resolved node ids;
        // this crate's `CallEdge::callee` is a written name, not a
        // resolved node id, so inbound degree can only be attributed
        // when the callee text happens to match a known node's own
        // name (best-effort resolution, documented divergence: no
        // cross-reference resolution pass exists in this slice).
        if let Some(target_id) = resolve_callee_to_node_id(graph, &call.callee) {
            let target_entry = degrees.entry(target_id).or_insert((0, 0));
            target_entry.0 += 1;
        }
    }
    degrees
}

fn resolve_callee_to_node_id(graph: &CodeGraph, callee: &str) -> Option<String> {
    graph
        .symbol_nodes()
        .find(|s| s.name == callee)
        .map(|s| s.id.clone())
}

/// Entry points per baseline §2.1: nodes with 0 inbound + >=1 outbound
/// CALLS edges. Computed over file ids (this crate's call granularity)
/// since `CallEdge` is file-scoped, not symbol-scoped.
fn compute_entry_points(graph: &CodeGraph, relationship: &str) -> HashSet<String> {
    let degrees = compute_degrees(graph, relationship);
    degrees
        .iter()
        .filter(|(_, (in_deg, out_deg))| *in_deg == 0 && *out_deg >= 1)
        .map(|(id, _)| id.clone())
        .collect()
}

// ---------------------------------------------------------------------
// Semantic mode
// ---------------------------------------------------------------------

fn run_semantic(
    graph: &CodeGraph,
    spec: &SearchGraphSpec,
    keywords: &[String],
    embedder: &dyn Embedder,
    vector_index: &VectorIndex,
) -> Result<Vec<SearchGraphHit>, SearchGraphError> {
    if keywords.is_empty() {
        return Ok(Vec::new());
    }
    let flat = flatten_nodes(graph);
    let by_id: std::collections::HashMap<&str, &FlatNode<'_>> =
        flat.iter().map(|n| (n.node_id, n)).collect();

    // Baseline §2.3: hardcoded to Function/Method/Class regardless of
    // the `label` param, unless the caller opts into the divergence.
    let allowed = |label: NodeLabel| -> bool {
        if spec.label_affects_semantic {
            spec.label.map(|l| l == label).unwrap_or(true)
        } else {
            matches!(label, NodeLabel::Function | NodeLabel::Type)
        }
    };

    let limit = spec
        .limit
        .filter(|l| *l > 0)
        .unwrap_or(SEMANTIC_INTERNAL_DEFAULT_LIMIT);

    let mut scored: Vec<(String, f64)> = Vec::new();
    for node in &flat {
        if !allowed(node.label) {
            continue;
        }
        // Baseline §2.3: score is the MINIMUM cosine across all
        // keywords, not the average -- "ALL keywords must be relevant".
        let node_vec = embedder.embed(node.name);
        let mut min_cosine = f64::MAX;
        for keyword in keywords {
            let kw_vec = embedder.embed(keyword);
            let cosine = crate::embed::cosine_similarity(&node_vec, &kw_vec) as f64;
            min_cosine = min_cosine.min(cosine);
        }
        if min_cosine == f64::MAX {
            continue;
        }
        scored.push((node.node_id.to_owned(), min_cosine));
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
                name: node.name.to_owned(),
                qualified_name: node.qualified_name.clone(),
                label: node.label.as_str(),
                file_path: node.file_path.to_owned(),
                rank: None,
                in_degree: None,
                out_degree: None,
                connected_names: None,
                score: Some(score),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------
// Pattern helpers
// ---------------------------------------------------------------------

fn compile_pattern(pattern: Option<&str>) -> Result<Option<Regex>, SearchGraphError> {
    match pattern {
        None => Ok(None),
        Some(p) => Regex::new(p)
            .map(Some)
            .map_err(|e| SearchGraphError::InvalidPattern {
                pattern: p.to_owned(),
                reason: e.to_string(),
            }),
    }
}

/// Baseline §2.1: `file_pattern` is glob-ish for BM25 (SQL LIKE) but a
/// bare literal (no `*`/`?`) becomes a substring match
/// (`%literal%`). For both modes here it compiles to a regex: `*` ->
/// `.*`, `?` -> `.`, a bare literal is wrapped so it matches anywhere
/// in the path.
fn compile_file_pattern(pattern: Option<&str>) -> Result<Option<Regex>, SearchGraphError> {
    match pattern {
        None => Ok(None),
        Some(p) => {
            let has_glob = p.contains('*') || p.contains('?');
            let regex_source = if has_glob {
                let mut out = String::from("^");
                for c in p.chars() {
                    match c {
                        '*' => out.push_str(".*"),
                        '?' => out.push('.'),
                        other => out.push_str(&regex::escape(&other.to_string())),
                    }
                }
                out.push('$');
                out
            } else {
                format!(".*{}.*", regex::escape(p))
            };
            Regex::new(&regex_source)
                .map(Some)
                .map_err(|e| SearchGraphError::InvalidPattern {
                    pattern: p.to_owned(),
                    reason: e.to_string(),
                })
        }
    }
}

fn file_pattern_matches(re: &Option<Regex>, file_path: &str) -> bool {
    match re {
        Some(re) => re.is_match(file_path),
        None => true,
    }
}
