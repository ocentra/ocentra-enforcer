//! X06.4 rank fusion (D-08 LOCKED): hybrid dense+BM25 via Reciprocal
//! Rank Fusion (RRF, kâ‰ˆ60), hard filters EXCLUDE before rerank, soft
//! signals only boost, and the Recall@100-pre-rerank / reranker-lift
//! measurement hooks the QA/parity harness (X06.9) reads.
//!
//! RRF combines RANKS, not raw scores (Rag-Guide doctrine, D-08) --
//! full-text BM25 scores and dense cosine similarities are not on
//! comparable scales, so fusing them by rank position rather than score
//! magnitude is the documented-correct approach.

use std::collections::HashMap;

use crate::owned_boundary::Retained;
use crate::search::document::SearchDocument;
use enforcer_domain::memory_types::{
    DocumentKind, RankingDocumentId, RankingFilterDecision, RankingFilterName,
    RankingHardFilterPredicate, RankingPosition, RankingScore, RankingSnippet, RankingSourcePath,
};

/// One retriever's scored hit for one document. Full-text
/// ([`crate::fulltext::FullTextIndex::search`]) and vector
/// ([`crate::vector::VectorIndex::search`]) both produce this same
/// shape so [`fuse_rrf`] can treat them uniformly.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCandidate {
    pub doc_id: RankingDocumentId,
    pub score: RankingScore,
}

/// A hard, binary inclusion/exclusion filter (permission/trust/repo
/// scoping, D-08: "hard filters EXCLUDE before rerank"). `allow` returns
/// `true` if `doc_id` may appear in the candidate pool at all.
pub struct HardFilter {
    name: RankingFilterName,
    predicate: RankingHardFilterPredicate,
}

impl std::fmt::Debug for HardFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HardFilter")
            .field("name", &self.name)
            .field("predicate", &self.predicate)
            .finish()
    }
}

impl HardFilter {
    pub fn from_predicate(
        name: RankingFilterName,
        predicate: impl Fn(&RankingDocumentId) -> RankingFilterDecision + Send + Sync + 'static,
    ) -> Self {
        Self {
            name,
            predicate: RankingHardFilterPredicate::from_predicate(predicate),
        }
    }

    pub fn name(&self) -> &RankingFilterName {
        &self.name
    }

    fn allows(&self, doc_id: &RankingDocumentId) -> RankingFilterDecision {
        self.predicate.is_allowed(doc_id).into()
    }
}

/// One candidate's full score-family trace before rerank truncation --
/// the Recall@100-pre-rerank measurement hook (D-08 / Rag-Guide: "measure
/// Recall@100 pre-rerank; the reranker cannot fix a missing candidate").
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateTrace {
    pub doc_id: RankingDocumentId,
    pub fulltext_rank: Option<RankingPosition>,
    pub vector_rank: Option<RankingPosition>,
    pub rrf_score: RankingScore,
}

/// One ranked hit surviving into the reranked/context stage, carrying
/// enough of the source document forward that the context pack never
/// needs a second corpus lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedHit {
    pub doc_id: RankingDocumentId,
    pub kind: DocumentKind,
    pub snippet: RankingSnippet,
    pub source_path: Option<RankingSourcePath>,
    /// Fusion-stage score (RRF). Overwritten by the reranker's own score
    /// once reranking runs, so this always reflects "the score this hit
    /// carried at the point it was last ranked".
    pub score: RankingScore,
}

/// The output of [`fuse_rrf`]: the full pre-rerank trace (for
/// Recall@100) plus the fused candidate pool in rank order, ready to be
/// truncated to the rerank pool size by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct RankFusionResult {
    pub pre_rerank_pool: Vec<CandidateTrace>,
    pub candidates: Vec<RankedHit>,
}

/// Reciprocal Rank Fusion: `score(d) = sum over retrievers of 1 /
/// (k + rank(d))`, rank is 1-based. D-08 LOCKED: `k` is ~60
/// (Rag-Guide's RRF constant; see [`crate::search::RRF_K`]).
///
/// Hard filters are applied BEFORE fusion (D-08: EXCLUDE before rerank)
/// -- a document any filter rejects never enters `pre_rerank_pool` at
/// all, so it can never leak into the context pack even if it scored
/// well on either retriever.
pub fn fuse_rrf(
    fulltext_ranked: &[ScoredCandidate],
    vector_ranked: &[ScoredCandidate],
    corpus: &[SearchDocument],
    hard_filters: &[HardFilter],
    k: RankingScore,
) -> RankFusionResult {
    let k = k.get();
    let corpus_index: HashMap<&str, &SearchDocument> =
        corpus.iter().map(|doc| (doc.id.as_str(), doc)).collect();

    let passes_filters = |doc_id: &RankingDocumentId| {
        hard_filters
            .iter()
            .all(|filter| filter.allows(doc_id).is_allowed())
    };

    let fulltext_rank_of: HashMap<&str, usize> = fulltext_ranked
        .iter()
        .filter(|hit| passes_filters(&hit.doc_id))
        .enumerate()
        .map(|(index, hit)| (hit.doc_id.as_str(), index + 1))
        .collect();
    let vector_rank_of: HashMap<&str, usize> = vector_ranked
        .iter()
        .filter(|hit| passes_filters(&hit.doc_id))
        .enumerate()
        .map(|(index, hit)| (hit.doc_id.as_str(), index + 1))
        .collect();

    let mut doc_ids: Vec<&str> = fulltext_rank_of
        .keys()
        .chain(vector_rank_of.keys())
        .copied()
        .collect();
    doc_ids.sort_unstable();
    doc_ids.dedup();

    let mut traces: Vec<CandidateTrace> = doc_ids
        .into_iter()
        .filter_map(|doc_id| {
            // A candidate not present in the corpus projection cannot be
            // resolved to a document to return -- skip it rather than
            // panicking on a caller/corpus mismatch.
            corpus_index.get(doc_id)?;
            let fulltext_rank = fulltext_rank_of.get(doc_id).copied();
            let vector_rank = vector_rank_of.get(doc_id).copied();
            let mut rrf_score = 0.0;
            if let Some(rank) = fulltext_rank {
                rrf_score += 1.0 / (k + crate::owned_boundary::usize_to_f64(rank));
            }
            if let Some(rank) = vector_rank {
                rrf_score += 1.0 / (k + crate::owned_boundary::usize_to_f64(rank));
            }
            Some(CandidateTrace {
                doc_id: doc_id.retained().into(),
                fulltext_rank: fulltext_rank.map(Into::into),
                vector_rank: vector_rank.map(Into::into),
                rrf_score: rrf_score.into(),
            })
        })
        .collect();

    traces.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.doc_id.cmp(&b.doc_id))
    });

    let candidates: Vec<RankedHit> = traces
        .iter()
        .filter_map(|trace| {
            let doc = corpus_index.get(trace.doc_id.as_str())?;
            Some(RankedHit {
                doc_id: trace.doc_id.retained(),
                kind: doc.kind,
                snippet: doc.snippet.as_str().into(),
                source_path: doc.source_path.as_ref().map(|path| path.as_str().into()),
                score: trace.rrf_score,
            })
        })
        .collect();

    RankFusionResult {
        pre_rerank_pool: traces,
        candidates,
    }
}

/// Reranker-lift: how much the reranker's final ordering diverged from
/// (improved on) the pre-rerank fusion ordering, measured as the mean
/// absolute rank-position change of every document that survived into
/// `context`, normalized to `[0, 1]` by the pre-rerank pool size. `0.0`
/// means the reranker did not move anything (or there is nothing to
/// measure); values approaching `1.0` mean large reordering.
///
/// This is a *lift magnitude*, not a quality judgment by itself -- the
/// QA/parity harness (X06.9) pairs it with relevance grades to decide
/// whether the movement was an improvement. Recording the raw magnitude
/// here keeps this module's contract simple and testable.
pub fn reranker_lift(pre_rerank_pool: &[CandidateTrace], context: &[RankedHit]) -> RankingScore {
    if pre_rerank_pool.is_empty() || context.is_empty() {
        return 0.0.into();
    }
    let pre_rank_of: HashMap<&str, usize> = pre_rerank_pool
        .iter()
        .enumerate()
        .map(|(index, trace)| (trace.doc_id.as_str(), index + 1))
        .collect();

    let mut total_shift = 0.0;
    let mut counted = 0usize;
    for (post_index, hit) in context.iter().enumerate() {
        if let Some(&pre_rank) = pre_rank_of.get(hit.doc_id.as_str()) {
            let post_rank = post_index + 1;
            total_shift += (crate::owned_boundary::usize_to_f64(pre_rank)
                - crate::owned_boundary::usize_to_f64(post_rank))
            .abs();
            counted += 1;
        }
    }
    if counted == 0 {
        return 0.0.into();
    }
    let pool_size = crate::owned_boundary::usize_to_f64(pre_rerank_pool.len());
    (total_shift / crate::owned_boundary::usize_to_f64(counted) / pool_size)
        .min(1.0)
        .into()
}
