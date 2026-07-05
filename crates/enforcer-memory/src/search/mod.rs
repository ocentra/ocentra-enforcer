//! X06.4: the full-text/vector/rerank retrieval stack.
//!
//! This module wires four independently testable pieces
//! ([`crate::fulltext`], [`crate::vector`], [`crate::embed`],
//! [`crate::rerank`], [`crate::ranking`]) into one query surface:
//! [`HybridSearcher`]. It extends the existing feature-gated embedding
//! seam ([`crate::retriever::EmbeddingRetriever`]) rather than forking a
//! second retrieval path -- the deterministic [`crate::embed::Embedder`]
//! default satisfies that seam with zero model downloads and zero
//! network calls (see `embed.rs` module docs for the degraded-mode
//! contract), and a real `ort`-backed implementation may be dropped in
//! later behind the `ort-models` feature without changing this module's
//! shape.
//!
//! # Pipeline (owner-set model philosophy: "never run expensive models
//! on the entire corpus")
//!
//! ```text
//! query
//!   -> fulltext (BM25-ish, code-aware tokenization)   \
//!   -> vector (HNSW, dense embedding similarity)       } RRF fuse, k=60
//!   -> candidate pool (100-200, hard filters excluded) /
//!   -> rerank (20-40 survivors)
//!   -> context pack (5-10 final, token-reduction estimate recorded)
//! ```
//!
//! D-08 (LOCKED) governs the pool/rerank/context sizes and the
//! hard-filter-before-soft-boost ordering; see [`crate::ranking`].

pub mod document;

pub use document::{DocumentKind, SearchDocument};

use crate::embed::{DegradedState, Embedder, LoadState};
use crate::error::Result;
use crate::fulltext::FullTextIndex;
use crate::ranking::{fuse_rrf, CandidateTrace, HardFilter, RankFusionResult, RankedHit};
use crate::rerank::Reranker;
use crate::vector::VectorIndex;

/// D-08: candidate pool pulled from each retriever before fusion.
pub const CANDIDATE_POOL_MIN: usize = 100;
pub const CANDIDATE_POOL_MAX: usize = 200;
/// D-08: survivors handed to the reranker.
pub const RERANK_POOL_MIN: usize = 20;
pub const RERANK_POOL_MAX: usize = 40;
/// D-08: final context-pack size.
pub const CONTEXT_MIN: usize = 5;
pub const CONTEXT_MAX: usize = 10;
/// Rag-Guide RRF constant (D-08).
pub const RRF_K: f64 = 60.0;

/// The result of one hybrid query: the final context pack plus the
/// measurement hooks the QA/parity harness (X06.9) and the token-
/// reduction proof (`proof/memory/x06-token-reduction.json`) read.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Final context pack, `CONTEXT_MIN..=CONTEXT_MAX` items (fewer if
    /// the corpus itself is smaller than that).
    pub context: Vec<RankedHit>,
    /// Full fusion trace (score families per candidate) before rerank
    /// truncation -- the "Recall@100-pre-rerank" measurement hook.
    pub pre_rerank_pool: Vec<CandidateTrace>,
    /// Reranker-lift measurement: pre-rerank vs post-rerank ordering
    /// delta, see [`crate::ranking::reranker_lift`].
    pub reranker_lift: f64,
    /// Estimated token cost of the context pack vs. a naive "hand
    /// Claude the top N whole files" baseline. See
    /// [`token_reduction_estimate`].
    pub token_reduction_estimate: TokenReductionEstimate,
    /// Capability state the embedder/reranker ran under. Never silently
    /// upgraded to "loaded" if the run was actually degraded.
    pub embedder_state: LoadState,
    pub reranker_state: LoadState,
}

/// A crude but honest token-cost estimate: `naive_tokens` is what
/// handing over `naive_file_count` whole files at `avg_tokens_per_file`
/// would have cost; `context_tokens` is the sum of the actual context
/// pack's estimated token lengths. This is intentionally a *ratio*
/// measurement hook, not a claim about any specific LLM's tokenizer --
/// callers with a real tokenizer can substitute exact counts using the
/// same shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenReductionEstimate {
    pub naive_tokens: usize,
    pub context_tokens: usize,
}

impl TokenReductionEstimate {
    /// `naive_tokens / context_tokens`, saturating at `f64::MAX` if
    /// `context_tokens` is zero (an empty context pack is not a
    /// reduction claim -- treat as "no data" by returning 0.0 instead of
    /// dividing by zero).
    pub fn ratio(&self) -> f64 {
        if self.context_tokens == 0 {
            0.0
        } else {
            self.naive_tokens as f64 / self.context_tokens as f64
        }
    }
}

/// Very rough token estimate: ~4 bytes/token, matching common English
/// tokenizer heuristics closely enough for a *relative* reduction
/// measurement (this is never used as an exact-billing number).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Compute the token-reduction estimate for a context pack against a
/// naive baseline of handing over `naive_file_count` whole documents at
/// `naive_avg_len` bytes each.
pub fn token_reduction_estimate(
    context: &[RankedHit],
    naive_file_count: usize,
    naive_avg_len: usize,
) -> TokenReductionEstimate {
    let context_tokens: usize = context
        .iter()
        .map(|hit| estimate_tokens(&hit.snippet))
        .sum();
    let naive_tokens = estimate_tokens(&"x".repeat(naive_avg_len)) * naive_file_count.max(1);
    TokenReductionEstimate {
        naive_tokens,
        context_tokens,
    }
}

/// The hybrid searcher: owns a full-text index, a vector index, an
/// embedder, and a reranker, and answers queries per the D-08 pipeline.
pub struct HybridSearcher<'a> {
    pub fulltext: &'a FullTextIndex,
    pub vector: &'a VectorIndex,
    pub embedder: &'a dyn Embedder,
    pub reranker: &'a dyn Reranker,
}

impl<'a> HybridSearcher<'a> {
    pub fn new(
        fulltext: &'a FullTextIndex,
        vector: &'a VectorIndex,
        embedder: &'a dyn Embedder,
        reranker: &'a dyn Reranker,
    ) -> Self {
        Self {
            fulltext,
            vector,
            embedder,
            reranker,
        }
    }

    /// Run the full hybrid pipeline for `query` against `corpus`,
    /// applying `hard_filters` before rerank (D-08: hard filters EXCLUDE
    /// before rerank, soft signals only boost -- see
    /// [`crate::ranking::fuse_rrf`]).
    pub fn search(
        &self,
        query: &str,
        corpus: &[SearchDocument],
        hard_filters: &[HardFilter],
    ) -> Result<SearchResult> {
        let pool_size = CANDIDATE_POOL_MAX.min(corpus.len().max(1));
        let fulltext_ranked = self.fulltext.search(query, pool_size)?;
        let embedding_state = self.embedder.state();
        let query_vec = self.embedder.embed(query)?;
        let vector_ranked = self.vector.search(&query_vec, pool_size);

        let fused = fuse_rrf(
            &fulltext_ranked,
            &vector_ranked,
            corpus,
            hard_filters,
            RRF_K,
        );
        let RankFusionResult {
            pre_rerank_pool,
            candidates,
        } = fused;

        let rerank_take = RERANK_POOL_MAX.min(candidates.len());
        let to_rerank = &candidates[..rerank_take];
        let reranker_state = self.reranker.state();
        let reranked = self.reranker.rerank(query, to_rerank)?;

        let context_take = CONTEXT_MAX
            .min(reranked.len())
            .max(CONTEXT_MIN.min(reranked.len()));
        let context: Vec<RankedHit> = reranked.into_iter().take(context_take).collect();

        let lift = crate::ranking::reranker_lift(&pre_rerank_pool, &context);
        let token_estimate = token_reduction_estimate(&context, corpus.len(), 2000);

        Ok(SearchResult {
            context,
            pre_rerank_pool,
            reranker_lift: lift,
            token_reduction_estimate: token_estimate,
            embedder_state: embedding_state,
            reranker_state,
        })
    }
}

/// Whether either capability ran in a state the workpack forbids
/// claiming as feature parity (D-03/OWNER_INTENT: "degraded mode is
/// labeled and never claimed as parity").
pub fn is_degraded(result: &SearchResult) -> bool {
    matches!(result.embedder_state, LoadState::Degraded(_))
        || matches!(result.reranker_state, LoadState::Degraded(_))
        || matches!(result.embedder_state, LoadState::Failed)
        || matches!(result.reranker_state, LoadState::Failed)
}

/// Convenience: the reason for a degraded capability state, if any, for
/// diagnostics/proof-artifact reporting. `DegradedState` is `Copy`, so
/// this returns an owned value rather than borrowing from `state`.
pub fn degraded_reason(state: &LoadState) -> Option<DegradedState> {
    match state {
        LoadState::Degraded(reason) => Some(*reason),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_reduction_ratio_is_zero_for_empty_context() {
        let estimate = TokenReductionEstimate {
            naive_tokens: 1000,
            context_tokens: 0,
        };
        assert_eq!(estimate.ratio(), 0.0);
    }

    #[test]
    fn token_reduction_ratio_reflects_savings() {
        let estimate = TokenReductionEstimate {
            naive_tokens: 10_000,
            context_tokens: 500,
        };
        assert_eq!(estimate.ratio(), 20.0);
    }

    #[test]
    fn estimate_tokens_is_never_zero_for_nonempty_text() {
        assert!(estimate_tokens("a") >= 1);
        assert!(estimate_tokens("") >= 1);
    }
}
