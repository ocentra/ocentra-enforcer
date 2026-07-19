//! X06 core parity: `SIMILAR_TO` / `SEMANTICALLY_RELATED` edge
//! materialization, mirroring the codebase-memory-mcp C baseline's two
//! post-index passes:
//!
//! - `SIMILAR_TO` -- `codebase-memory-mcp/src/pipeline/pass_similarity.c`
//!   (`cbm_pipeline_pass_similarity`): builds an LSH index over each
//!   Function/Method node's 64-permutation MinHash fingerprint
//!   (decoded from a `"fp"` property written at extraction time,
//!   `pass_similarity.c:48-70`) and emits a `SIMILAR_TO` edge for every
//!   pair whose Jaccard similarity is `>= CBM_MINHASH_JACCARD_THRESHOLD`
//!   (0.95, `simhash/minhash.h:33`), same file extension required
//!   (`pass_similarity.c:203`), capped at `CBM_MINHASH_MAX_EDGES_PER_NODE`
//!   (10, `simhash/minhash.h:36`) edges per node, `source_id < target_id`
//!   so each pair is emitted once (`pass_similarity.c:206`). Edge
//!   properties are `{"jaccard": <f64>, "same_file": <bool>}`
//!   (`pass_similarity.c:237-240`).
//! - `SEMANTICALLY_RELATED` --
//!   `codebase-memory-mcp/src/pipeline/pass_semantic_edges.c` +
//!   `src/semantic/semantic.c` (`cbm_sem_combined_score`,
//!   `semantic.c:1601-1660`): a combined score over 11 signals (TF-IDF
//!   token overlap, Random-Indexing-with-co-occurrence dense vectors
//!   seeded from a 768-d pretrained nomic-embed-code table, decoded
//!   MinHash Jaccard, API/type/decorator vectors, an AST structural
//!   profile, approximate data flow, and a module-proximity multiplier
//!   -- weights `w_tfidf=0.20, w_ri=0.25, w_minhash=0.10, w_api=0.15,
//!   w_type=0.10, w_decorator=0.05, w_struct_profile=0.10,
//!   w_dataflow=0.05`, `semantic.c:42-49`), gated at
//!   `CBM_SEM_EDGE_THRESHOLD` (0.75, `semantic.h:56`), capped at
//!   `CBM_SEM_MAX_EDGES` (10, `semantic.h:59`) edges per node, with an
//!   early-exit to 0 when the pair's MinHash Jaccard already clears the
//!   `SIMILAR_TO` threshold (`semantic.c:1607-1618`, so the two edge
//!   kinds partition rather than double-cover near-duplicate pairs).
//!   Edge properties are `{"score": <f32>, "same_file": <bool>}`
//!   (`pass_semantic_edges.c:941-944`).
//!
//! # Honest scope reduction
//!
//! The baseline's 11-signal pipeline depends on inputs this crate's
//! [`crate::code_graph::SymbolNode`] does not carry: no stored source
//! text, no `"fp"` MinHash fingerprint, no signature/param-type/
//! decorator strings, and no pretrained embedding table (the baseline's
//! nomic-embed-code vectors are a vendored binary blob this crate does
//! not ship, matching this crate's zero-network/zero-model-download
//! mandate -- see `src/lib.rs` module docs). Reproducing the baseline's
//! exact scores is therefore out of reach without those inputs; this
//! module instead computes an honestly-reduced, deterministic analog
//! from what *is* available on every indexed symbol (name, file path,
//! [`crate::complexity::ComplexityMetrics`], and the resolved call
//! graph), documented signal-by-signal below rather than silently
//! presented as byte-parity. This mirrors [`crate::resolution`]'s
//! "honest limitations" precedent for the same reason (a real
//! LSP/type-checker/embedding-model input this crate does not have).
//!
//! ## `SIMILAR_TO` contract
//!
//! [`similar_to`] now follows the baseline's actual contract closely:
//! it reads persisted 64-slot MinHash fingerprint evidence (`fp` hex,
//! `k=64`) from [`crate::code_graph::SymbolNode`]s, requires same file
//! extension, uses the baseline's 0.95 signature-agreement threshold,
//! caps emission at 10 edges per node, and emits each pair once with
//! `source_id < target_id`.
//!
//! Two older analog signals remain available explicitly rather than
//! silently masquerading as baseline parity:
//!
//! - [`similar_to_body_shingles`] — exact Jaccard over persisted
//!   body-token 5-shingles.
//! - [`similar_to_identifier_tokens`] — Rust-only identifier-token
//!   overlap, kept as an additive local heuristic.
//!
//! ## `SEMANTICALLY_RELATED` reduction
//!
//! [`semantically_related`] combines three of the baseline's signals
//! with available substitutes for the rest, re-weighted to sum to 1.0:
//!
//! - **name-token Jaccard** (`weight 0.40`) -- a lexical-overlap stand-in
//!   for the baseline's TF-IDF + Random Indexing signals (both of which
//!   need corpus-wide token statistics and/or a pretrained embedding
//!   table this module does not build).
//! - **shared-callee overlap** (`weight 0.30`) -- Jaccard over each
//!   symbol's resolved callee id set from
//!   [`crate::code_graph::CodeGraph::resolved_calls`], the direct analog
//!   of the baseline's API Signature vector signal ("same callees ->
//!   related").
//! - **complexity-profile similarity** (`weight 0.30`) -- 1 minus the
//!   normalized Euclidean distance between each symbol's
//!   `(complexity, cognitive, loop_count, param_count)` tuple (from
//!   [`crate::complexity::ComplexityMetrics`]), a coarse stand-in for
//!   the baseline's AST Structural Profile signal.
//!
//! The baseline's Type Signature, Decorator, and approximate-data-flow
//! signals have no equivalent input in this crate and are omitted
//! rather than faked with a constant. The module-proximity multiplier
//! (same-file/same-dir boost, `semantic.c:1519-1553`) is reproduced
//! exactly via [`proximity_multiplier`]. The early-exit-when-already-
//! `SIMILAR_TO` rule and the 0.75 threshold / 10-edges-per-node cap are
//! also reproduced exactly.

use std::collections::{BTreeSet, HashMap};

use crate::code_graph::{CodeGraph, CodeNode, SymbolNode};
use crate::complexity::ComplexityMetrics;
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::ResolutionConfidence;
use enforcer_domain::memory_types::{
    MemoryFingerprintHashCount, MemoryFingerprintValue, SimilarityEdgeBudgetAvailable,
    SimilarityEdgeCount, SimilarityIdentifierAtom, SimilarityIdentifierLexemes,
    SimilarityIdentifierName, SimilarityMinHashValues, SimilarityMode, SimilarityNodeId,
    SimilarityNodePair, SimilarityPath, SimilaritySameFile, SimilarityScore,
};

/// Baseline parity: `CBM_MINHASH_JACCARD_THRESHOLD`
/// (`simhash/minhash.h:33`).
pub const SIMILAR_TO_THRESHOLD: f64 = 0.95;

/// Baseline parity: `CBM_MINHASH_MAX_EDGES_PER_NODE`
/// (`simhash/minhash.h:36`).
pub const SIMILAR_TO_MAX_EDGES_PER_NODE: usize = 10;

/// Baseline parity: `CBM_SEM_EDGE_THRESHOLD` (`semantic.h:56`).
pub const SEMANTICALLY_RELATED_THRESHOLD: f64 = 0.75;

/// Baseline parity: `CBM_SEM_MAX_EDGES` (`semantic.h:59`).
pub const SEMANTICALLY_RELATED_MAX_EDGES_PER_NODE: usize = 10;

/// Baseline parity: `CBM_SEM_PROX_MAX_BOOST` (`semantic.c:67`) -- same
/// file/near-directory pairs score up to 10% higher.
const PROXIMITY_MAX_BOOST: f64 = 0.10;

/// Re-weighted signal weights for [`semantically_related`], summing to
/// 1.0 -- see module docs, "`SEMANTICALLY_RELATED` reduction", for why
/// these three signals stand in for the baseline's eight.
const WEIGHT_NAME_TOKENS: f64 = 0.40;
const WEIGHT_SHARED_CALLEES: f64 = 0.30;
const WEIGHT_COMPLEXITY_PROFILE: f64 = 0.30;

/// One materialized `SIMILAR_TO` edge: `source_id` and `target_id` are
/// [`SymbolNode::id`]s with `source_id < target_id` (baseline parity --
/// see module docs), `jaccard` is the name-token Jaccard similarity that
/// triggered the edge, and `same_file` mirrors the baseline's
/// `same_file` edge property.
#[derive(Debug, Clone, PartialEq)]
pub struct SimilarToEdge {
    pub source_id: SimilarityNodeId,
    pub target_id: SimilarityNodeId,
    pub mode: SimilarityMode,
    pub jaccard: SimilarityScore,
    pub same_file: SimilaritySameFile,
}

const MINHASH_K: usize = 64;

/// One materialized `SEMANTICALLY_RELATED` edge: `source_id` and
/// `target_id` are [`SymbolNode::id`]s with `source_id < target_id`,
/// `score` is the combined signal score that triggered the edge (after
/// the proximity multiplier and the early-exit-vs-`SIMILAR_TO` rule),
/// and `same_file` mirrors the baseline's `same_file` edge property.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticallyRelatedEdge {
    pub source_id: SimilarityNodeId,
    pub target_id: SimilarityNodeId,
    pub score: SimilarityScore,
    pub same_file: SimilaritySameFile,
}

/// Split an identifier into lowercase tokens on `camelCase`, `snake_case`,
/// and `.`/`-` boundaries -- the same delimiter set the baseline's
/// `cbm_sem_tokenize` uses (`semantic.c:142-188`), minus its abbreviation
/// expansion table (out of scope here; the delimiter-based split alone
/// is the load-bearing structural signal for [`similar_to`]).
pub fn tokenize_identifier(
    name: impl Into<SimilarityIdentifierName>,
) -> SimilarityIdentifierLexemes {
    let name = name.into();
    let mut tokens = SimilarityIdentifierLexemes::new();
    let mut current = String::new();
    let mut previous = None;
    for c in name.as_str().chars() {
        let is_delim = matches!(c, '.' | '/' | '_' | '-' | ' ' | '(' | ')' | ',' | ':');
        let is_camel_break = c.is_ascii_uppercase()
            && previous.is_some_and(|previous: char| previous.is_ascii_lowercase());
        if is_delim || is_camel_break {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current).into());
            }
            if is_delim {
                continue;
            }
        }
        if c.is_alphanumeric() {
            current.push(c.to_ascii_lowercase());
        }
        previous = Some(c);
    }
    if !current.is_empty() {
        tokens.push(current.into());
    }
    tokens
}

/// Jaccard similarity between two token multisets treated as sets
/// (`|A ∩ B| / |A ∪ B|`); `0.0` when both sides are empty (no evidence
/// of similarity, not vacuous certainty).
fn jaccard<T: Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        f64::from(u32::try_from(intersection).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(union).unwrap_or(u32::MAX))
    }
}

/// The file extension (including the leading dot), matching the
/// baseline's `file_ext` helper (`pass_similarity.c:38-44`); empty
/// string when `rel_path` has no dot.
fn file_ext(rel_path: &SimilarityPath) -> SimilarityPath {
    let rel_path = rel_path.as_str();
    match rel_path.rfind('.') {
        Some(idx) => rel_path.get(idx..).map_or_else(|| "".into(), Into::into),
        None => "".into(),
    }
}

/// Baseline parity: `cbm_sem_proximity` (`semantic.c:1519-1553`). Counts
/// shared leading path components between two forward-slash-normalized
/// relative paths and scales [`PROXIMITY_MAX_BOOST`] by
/// `shared / max(components_a, components_b)`; returns `1.0` (no boost)
/// when either path has no directory components.
pub fn proximity_multiplier(
    path_a: impl Into<SimilarityPath>,
    path_b: impl Into<SimilarityPath>,
) -> SimilarityScore {
    let path_a = path_a.into();
    let path_b = path_b.into();
    let path_a = path_a.as_str();
    let path_b = path_b.as_str();
    let shared_slashes = path_a
        .bytes()
        .zip(path_b.bytes())
        .take_while(|(left, right)| left == right)
        .filter(|(byte, _)| *byte == b'/')
        .count();
    let total_a = path_a.matches('/').count();
    let total_b = path_b.matches('/').count();
    let max_total = total_a.max(total_b);
    if max_total == 0 {
        return 1.0.into();
    }
    let ratio = crate::owned_boundary::usize_to_f64(shared_slashes)
        / crate::owned_boundary::usize_to_f64(max_total);
    (1.0 + (ratio * PROXIMITY_MAX_BOOST)).into()
}

/// One symbol's precomputed similarity inputs, gathered once per
/// [`similar_to`]/[`semantically_related`] call rather than
/// re-tokenizing/re-walking the call graph per pair.
struct SymbolProfile<'g> {
    id: SimilarityNodeId,
    rel_path: SimilarityPath,
    ext: SimilarityPath,
    name_tokens: BTreeSet<SimilarityIdentifierAtom>,
    fingerprint: Option<MinHashSignature>,
    body_shingles: Option<&'g BTreeSet<enforcer_domain::memory_types::MemoryFingerprintBodyGram>>,
    callees: BTreeSet<SimilarityNodeId>,
    metrics: Option<ComplexityMetrics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MinHashSignature {
    values: SimilarityMinHashValues,
}

/// Whether `node` is one of the callable kinds the baseline's
/// `pass_similarity`/`pass_semantic_edges` scan (`labels[] = {"Function",
/// "Method", NULL}` at `pass_similarity.c:100` and
/// `pass_semantic_edges.c:870`) -- widened here to also include
/// [`CodeNode::Test`] and [`CodeNode::Lambda`], this crate's other two
/// callable [`crate::code_graph::SymbolNode`]-carrying variants (the
/// baseline's C extractor has no distinct Test/Lambda node kind, so
/// there is no baseline scope to narrow against; excluding them would
/// silently drop callable symbols this crate does index).
fn is_callable(node: &CodeNode) -> Option<&SymbolNode> {
    match node {
        CodeNode::Function(sym)
        | CodeNode::Method(sym)
        | CodeNode::Test(sym)
        | CodeNode::Lambda(sym) => Some(sym),
        _ => None,
    }
}

/// Build the file-id -> rel_path lookup [`SymbolProfile`] needs, from
/// every [`CodeNode::File`]/[`CodeNode::TextOnly`] node in `graph`.
fn file_paths(graph: &CodeGraph) -> HashMap<SimilarityNodeId, SimilarityPath> {
    let mut paths = HashMap::new();
    for node in graph.nodes() {
        match node {
            CodeNode::File(file) | CodeNode::TextOnly(file) => {
                paths.insert(file.id.as_str().into(), file.rel_path.as_str().into());
            }
            _ => {}
        }
    }
    paths
}

/// Build each callable symbol's resolved-callee id set from
/// [`CodeGraph::resolved_calls`], keyed by the calling symbol's id.
/// Ambiguous resolutions contribute every candidate (matching
/// [`crate::resolution::ResolvedCall`]'s "never silently narrowed"
/// contract); unresolved calls contribute nothing.
fn callee_sets(graph: &CodeGraph) -> HashMap<SimilarityNodeId, BTreeSet<SimilarityNodeId>> {
    let mut callees: HashMap<SimilarityNodeId, BTreeSet<SimilarityNodeId>> = HashMap::new();
    for resolved in graph.resolved_calls() {
        if resolved.confidence == ResolutionConfidence::Unresolved {
            continue;
        }
        let Some(from_id) = resolved.from_symbol_id.as_deref() else {
            continue;
        };
        let entry = callees.entry(from_id.into()).or_default();
        for candidate in &resolved.candidates {
            entry.insert(candidate.as_str().into());
        }
    }
    callees
}

/// Gather every callable symbol's [`SymbolProfile`] in graph order.
fn build_profiles<'g>(graph: &'g CodeGraph) -> Vec<SymbolProfile<'g>> {
    let paths = file_paths(graph);
    let mut callees_by_symbol = callee_sets(graph);
    let mut profiles = Vec::new();
    for node in graph.nodes() {
        let Some(sym) = is_callable(node) else {
            continue;
        };
        let rel_path = match paths.get(&SimilarityNodeId::from(sym.file_id.as_str())) {
            Some(path) => path.as_str().into(),
            None => SimilarityPath::default(),
        };
        let ext = file_ext(&rel_path);
        let name_tokens = tokenize_identifier(&sym.name).into_iter().collect();
        let symbol_id = SimilarityNodeId::from(sym.id.as_str());
        let callees = callees_by_symbol
            .remove(&symbol_id)
            .map_or_else(BTreeSet::new, std::convert::identity);
        profiles.push(SymbolProfile {
            id: symbol_id,
            rel_path,
            ext,
            name_tokens,
            fingerprint: sym
                .source_body_fingerprint
                .as_ref()
                .and_then(|fingerprint| {
                    fingerprint
                        .fp
                        .as_deref()
                        .zip(fingerprint.k)
                        .and_then(|(fp, k)| {
                            let hex = MemoryFingerprintValue::from(fp);
                            decode_minhash_hex(&hex, k)
                        })
                }),
            body_shingles: sym
                .source_body_fingerprint
                .as_ref()
                .map(|fingerprint| &fingerprint.body_grams),
            callees,
            metrics: sym.metrics,
        });
    }
    profiles
}

/// Complexity-profile similarity: `1.0 - normalized_euclidean_distance`
/// over `(complexity, cognitive, loop_count, param_count)`, clamped to
/// `[0.0, 1.0]`. Returns `0.0` (no evidence of similarity) when either
/// side has no [`ComplexityMetrics`] -- never a fabricated mid-range
/// score for missing data.
fn complexity_similarity(
    a: Option<ComplexityMetrics>,
    b: Option<ComplexityMetrics>,
) -> SimilarityScore {
    let (Some(a), Some(b)) = (a, b) else {
        return 0.0.into();
    };
    let diffs = [
        (a.complexity, b.complexity),
        (a.cognitive, b.cognitive),
        (a.loop_count, b.loop_count),
        (a.param_count, b.param_count),
    ];
    let sum_sq: f64 = diffs
        .iter()
        .map(|&(x, y)| {
            let d = f64::from(x.get()) - f64::from(y.get());
            d * d
        })
        .sum();
    let distance = sum_sq.sqrt();
    // Normalize by a fixed scale so one wildly complex outlier pair
    // doesn't need per-corpus min/max bookkeeping: a distance of 20
    // (e.g. complexity differing by 20) already reads as "not similar".
    const DISTANCE_SCALE: f64 = 20.0;
    let normalized = (distance / DISTANCE_SCALE).min(1.0);
    (1.0 - normalized).into()
}

/// Push `(source_id, target_id)` onto `edge_counts` bookkeeping and
/// report whether both endpoints still have edge budget under `cap`.
fn has_budget(
    edge_counts: &HashMap<SimilarityNodeId, SimilarityEdgeCount>,
    id: &SimilarityNodeId,
    cap: SimilarityEdgeCount,
) -> SimilarityEdgeBudgetAvailable {
    let count = match edge_counts.get(id).copied() {
        Some(count) => count,
        None => SimilarityEdgeCount::from(0),
    };
    (count.get() < cap.get()).into()
}

/// Compute every `SIMILAR_TO` edge over `graph`'s callable symbols
/// (Function/Method/Test/Lambda). Deterministic: graph node order in,
/// `(source_id, target_id)` order out (both already-sorted since
/// `source_id < target_id` is enforced per pair, and pairs are emitted
/// in profile-index order) -- two calls against the same graph produce
/// byte-identical output. See module docs for the full baseline-parity
/// contract (threshold, same-extension gate, max-edges-per-node cap,
/// edge properties).
pub fn similar_to(graph: &CodeGraph) -> Vec<SimilarToEdge> {
    let profiles = build_profiles(graph);
    let mut edge_counts: HashMap<SimilarityNodeId, SimilarityEdgeCount> = HashMap::new();
    let mut edges = Vec::new();

    for (index, a) in profiles.iter().enumerate() {
        for b in profiles.iter().skip(index + 1) {
            if a.ext != b.ext {
                continue;
            }
            if !has_budget(&edge_counts, &a.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into()).has_budget()
                || !has_budget(&edge_counts, &b.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into())
                    .has_budget()
            {
                continue;
            }
            let (Some(a_fp), Some(b_fp)) = (a.fingerprint, b.fingerprint) else {
                continue;
            };
            let j_score = minhash_jaccard(&a_fp, &b_fp);
            if j_score < SIMILAR_TO_THRESHOLD {
                continue;
            }
            let pair = SimilarityNodePair::ordered(a.id.retained(), b.id.retained());
            let same_file = !a.rel_path.is_empty() && a.rel_path == b.rel_path;
            edges.push(SimilarToEdge {
                source_id: pair.source_id,
                target_id: pair.target_id,
                mode: SimilarityMode::MinHashFingerprint,
                jaccard: j_score,
                same_file: same_file.into(),
            });
            *edge_counts.entry(a.id.retained()).or_default() += 1;
            *edge_counts.entry(b.id.retained()).or_default() += 1;
        }
    }
    edges
}

pub fn similar_to_identifier_tokens(graph: &CodeGraph) -> Vec<SimilarToEdge> {
    let profiles = build_profiles(graph);
    let mut edge_counts: HashMap<SimilarityNodeId, SimilarityEdgeCount> = HashMap::new();
    let mut edges = Vec::new();

    for (index, a) in profiles.iter().enumerate() {
        for b in profiles.iter().skip(index + 1) {
            if a.ext != ".rs" || b.ext != ".rs" {
                continue;
            }
            if !has_budget(&edge_counts, &a.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into()).has_budget()
                || !has_budget(&edge_counts, &b.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into())
                    .has_budget()
            {
                continue;
            }
            let j_score = jaccard(&a.name_tokens, &b.name_tokens);
            if j_score < SIMILAR_TO_THRESHOLD {
                continue;
            }
            let pair = SimilarityNodePair::ordered(a.id.retained(), b.id.retained());
            let same_file = !a.rel_path.is_empty() && a.rel_path == b.rel_path;
            edges.push(SimilarToEdge {
                source_id: pair.source_id,
                target_id: pair.target_id,
                mode: SimilarityMode::IdentifierToken,
                jaccard: j_score.into(),
                same_file: same_file.into(),
            });
            *edge_counts.entry(a.id.retained()).or_default() += 1;
            *edge_counts.entry(b.id.retained()).or_default() += 1;
        }
    }
    edges
}

pub fn similar_to_body_shingles(graph: &CodeGraph) -> Vec<SimilarToEdge> {
    let profiles = build_profiles(graph);
    let mut edge_counts: HashMap<SimilarityNodeId, SimilarityEdgeCount> = HashMap::new();
    let mut edges = Vec::new();

    for (index, a) in profiles.iter().enumerate() {
        for b in profiles.iter().skip(index + 1) {
            if a.ext != b.ext {
                continue;
            }
            if !has_budget(&edge_counts, &a.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into()).has_budget()
                || !has_budget(&edge_counts, &b.id, SIMILAR_TO_MAX_EDGES_PER_NODE.into())
                    .has_budget()
            {
                continue;
            }
            let (Some(a_shingles), Some(b_shingles)) = (a.body_shingles, b.body_shingles) else {
                continue;
            };
            let j_score = jaccard(a_shingles, b_shingles);
            if j_score < SIMILAR_TO_THRESHOLD {
                continue;
            }
            let pair = SimilarityNodePair::ordered(a.id.retained(), b.id.retained());
            let same_file = !a.rel_path.is_empty() && a.rel_path == b.rel_path;
            edges.push(SimilarToEdge {
                source_id: pair.source_id,
                target_id: pair.target_id,
                mode: SimilarityMode::BodyShingle,
                jaccard: j_score.into(),
                same_file: same_file.into(),
            });
            *edge_counts.entry(a.id.retained()).or_default() += 1;
            *edge_counts.entry(b.id.retained()).or_default() += 1;
        }
    }
    edges
}

/// Compute every `SEMANTICALLY_RELATED` edge over `graph`'s callable
/// symbols, skipping any pair whose persisted MinHash signatures
/// already clear [`SIMILAR_TO_THRESHOLD`] -- the same
/// early-exit rule as the baseline's `cbm_sem_combined_score`
/// (`semantic.c:1607-1618`), adapted to this module's substitute
/// fingerprint signal so the two edge kinds still partition rather than
/// double-cover near-duplicates. See module docs for the full signal
/// breakdown and honest-scope-reduction rationale.
pub fn semantically_related(graph: &CodeGraph) -> Vec<SemanticallyRelatedEdge> {
    let profiles = build_profiles(graph);
    let mut edge_counts: HashMap<SimilarityNodeId, SimilarityEdgeCount> = HashMap::new();
    let mut edges = Vec::new();

    for (index, a) in profiles.iter().enumerate() {
        for b in profiles.iter().skip(index + 1) {
            if a.ext != b.ext {
                continue;
            }
            if !has_budget(
                &edge_counts,
                &a.id,
                SEMANTICALLY_RELATED_MAX_EDGES_PER_NODE.into(),
            )
            .has_budget()
                || !has_budget(
                    &edge_counts,
                    &b.id,
                    SEMANTICALLY_RELATED_MAX_EDGES_PER_NODE.into(),
                )
                .has_budget()
            {
                continue;
            }
            let callee_overlap = jaccard(&a.callees, &b.callees);
            let complexity_score = complexity_similarity(a.metrics, b.metrics);

            let fp_jaccard = match (a.fingerprint, b.fingerprint) {
                (Some(a_fp), Some(b_fp)) => minhash_jaccard(&a_fp, &b_fp),
                _ => 0.0.into(),
            };
            if fp_jaccard >= SIMILAR_TO_THRESHOLD {
                continue;
            }

            let name_jaccard = jaccard(&a.name_tokens, &b.name_tokens);

            let mut score = WEIGHT_NAME_TOKENS * name_jaccard
                + WEIGHT_SHARED_CALLEES * callee_overlap
                + WEIGHT_COMPLEXITY_PROFILE * complexity_score.get();
            score *= proximity_multiplier(a.rel_path.retained(), b.rel_path.retained()).get();
            score = score.clamp(0.0, 1.0);

            if score < SEMANTICALLY_RELATED_THRESHOLD {
                continue;
            }

            let pair = SimilarityNodePair::ordered(a.id.retained(), b.id.retained());
            let same_file = !a.rel_path.is_empty() && a.rel_path == b.rel_path;
            edges.push(SemanticallyRelatedEdge {
                source_id: pair.source_id,
                target_id: pair.target_id,
                score: score.into(),
                same_file: same_file.into(),
            });
            *edge_counts.entry(a.id.retained()).or_default() += 1;
            *edge_counts.entry(b.id.retained()).or_default() += 1;
        }
    }
    edges
}

fn decode_minhash_hex(
    hex: &MemoryFingerprintValue,
    k: MemoryFingerprintHashCount,
) -> Option<MinHashSignature> {
    if k.get() != MINHASH_K || hex.as_str().len() != MINHASH_K * 8 {
        return None;
    }
    let mut values = [0u32; MINHASH_K];
    for (idx, chunk) in hex.as_str().as_bytes().chunks_exact(8).enumerate() {
        let Ok(raw) = std::str::from_utf8(chunk) else {
            return None;
        };
        let value = values.get_mut(idx)?;
        let Ok(decoded) = u32::from_str_radix(raw, 16) else {
            return None;
        };
        *value = decoded;
    }
    Some(MinHashSignature {
        values: SimilarityMinHashValues::from_array(values),
    })
}

fn minhash_jaccard(a: &MinHashSignature, b: &MinHashSignature) -> SimilarityScore {
    let matches = a
        .values
        .as_array()
        .iter()
        .zip(b.values.as_array().iter())
        .filter(|(lhs, rhs)| lhs == rhs)
        .count();
    (crate::owned_boundary::usize_to_f64(matches) / crate::owned_boundary::usize_to_f64(MINHASH_K))
        .into()
}
