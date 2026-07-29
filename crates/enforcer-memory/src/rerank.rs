//! X06.4 reranking layer: the `Reranker` trait plus a deterministic
//! default implementation. Per D-03, the real cross-encoder/Qwen3-class
//! reranker path lives behind the local-runtime seam: llama.cpp/GGUF is
//! first-class, ONNX/ORT remains optional behind `ort-models`, and no
//! real model backend is exercised in default gates. The default build ships
//! [`FusionScoreReranker`], a deterministic reranker that recomputes a
//! lexical-overlap boost on top of the fusion score so the pipeline
//! (rank fuse -> rerank -> context) is exercisable end-to-end with zero
//! model weights, honestly reporting
//! `LoadState::Degraded(DegradedState::ProviderUnavailable)`.

use crate::error::Result;
use crate::fulltext::tokenize;
use crate::owned_boundary::Retained;
use crate::ranking::RankedHit;
use enforcer_domain::memory_types::{DegradedState, LoadState, ParserSourceText, RankingScore};

/// The reranking capability seam.
pub trait Reranker: Send + Sync {
    /// Re-score and re-order `candidates` for `query`, most relevant
    /// first. Implementations own their own scoring; the returned
    /// `Vec<RankedHit>` has each hit's `score` field overwritten with the
    /// reranker's own score (never the caller's fusion score).
    fn rerank(
        &self,
        query: ParserSourceText<'_>,
        candidates: &[RankedHit],
    ) -> Result<Vec<RankedHit>>;

    /// Current capability state, mirroring [`crate::embed::Embedder::state`].
    fn state(&self) -> LoadState;
}

/// Deterministic default reranker: re-scores each candidate by lexical
/// term overlap between the (code-aware tokenized) query and the
/// candidate's snippet, blended with the fusion score it received as
/// input so a candidate that scored well on both fulltext+vector but has
/// low literal overlap with this exact query phrasing does not get
/// discarded outright. Always reports
/// `LoadState::Degraded(DegradedState::ProviderUnavailable)` -- this is
/// a deterministic stand-in, never claimed as a real cross-encoder.
#[derive(Debug, Default, Clone, Copy)]
pub struct FusionScoreReranker;

impl FusionScoreReranker {
    pub fn new() -> Self {
        Self
    }

    fn overlap_score(
        query_terms: &[enforcer_domain::memory_types::MemoryFullTextToken],
        candidate_text: ParserSourceText<'_>,
    ) -> RankingScore {
        if query_terms.is_empty() {
            return 0.0.into();
        }
        let candidate_terms = tokenize(&enforcer_domain::memory_types::MemoryFullTextInput::from(
            candidate_text.as_str(),
        ))
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
        let hits = query_terms
            .iter()
            .filter(|term| candidate_terms.contains(*term))
            .count();
        (crate::owned_boundary::usize_to_f64(hits)
            / crate::owned_boundary::usize_to_f64(query_terms.len()))
        .into()
    }
}

impl Reranker for FusionScoreReranker {
    fn rerank(
        &self,
        query: ParserSourceText<'_>,
        candidates: &[RankedHit],
    ) -> Result<Vec<RankedHit>> {
        let query_terms = tokenize(&enforcer_domain::memory_types::MemoryFullTextInput::from(
            query.as_str(),
        ));
        let mut reranked: Vec<RankedHit> = candidates
            .iter()
            .map(|hit| {
                let overlap =
                    Self::overlap_score(&query_terms, ParserSourceText::from(hit.snippet.as_str()));
                // Blend: lexical overlap dominates (this reranker's only
                // real signal), fusion score breaks ties among equal
                // overlap so upstream ranking is not discarded entirely.
                let score = overlap.get() * 1000.0 + hit.score.get();
                RankedHit {
                    doc_id: hit.doc_id.retained(),
                    kind: hit.kind,
                    snippet: hit.snippet.retained(),
                    source_path: hit.source_path.retained(),
                    score: score.into(),
                }
            })
            .collect();
        reranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.doc_id.cmp(&b.doc_id))
        });
        Ok(reranked)
    }

    fn state(&self) -> LoadState {
        LoadState::Degraded(DegradedState::ProviderUnavailable)
    }
}
