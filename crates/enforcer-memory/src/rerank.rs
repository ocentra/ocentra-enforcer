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

use crate::embed::{DegradedState, LoadState};
use crate::error::Result;
use crate::fulltext::tokenize;
use crate::ranking::RankedHit;

/// The reranking capability seam.
pub trait Reranker: Send + Sync {
    /// Re-score and re-order `candidates` for `query`, most relevant
    /// first. Implementations own their own scoring; the returned
    /// `Vec<RankedHit>` has each hit's `score` field overwritten with the
    /// reranker's own score (never the caller's fusion score).
    fn rerank(&self, query: &str, candidates: &[RankedHit]) -> Result<Vec<RankedHit>>;

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

    fn overlap_score(query_terms: &[String], candidate_text: &str) -> f64 {
        if query_terms.is_empty() {
            return 0.0;
        }
        let candidate_terms: std::collections::HashSet<String> =
            tokenize(candidate_text).into_iter().collect();
        let hits = query_terms
            .iter()
            .filter(|term| candidate_terms.contains(*term))
            .count();
        hits as f64 / query_terms.len() as f64
    }
}

impl Reranker for FusionScoreReranker {
    fn rerank(&self, query: &str, candidates: &[RankedHit]) -> Result<Vec<RankedHit>> {
        let query_terms = tokenize(query);
        let mut reranked: Vec<RankedHit> = candidates
            .iter()
            .map(|hit| {
                let overlap = Self::overlap_score(&query_terms, &hit.snippet);
                // Blend: lexical overlap dominates (this reranker's only
                // real signal), fusion score breaks ties among equal
                // overlap so upstream ranking is not discarded entirely.
                let score = overlap * 1000.0 + hit.score;
                RankedHit {
                    doc_id: hit.doc_id.clone(),
                    kind: hit.kind,
                    snippet: hit.snippet.clone(),
                    source_path: hit.source_path.clone(),
                    score,
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::document::DocumentKind;

    fn hit(doc_id: &str, snippet: &str, score: f64) -> RankedHit {
        RankedHit {
            doc_id: doc_id.to_owned(),
            kind: DocumentKind::Function,
            snippet: snippet.to_owned(),
            source_path: None,
            score,
        }
    }

    #[test]
    fn rerank_prefers_higher_lexical_overlap_with_query() -> Result<()> {
        let reranker = FusionScoreReranker::new();
        let candidates = vec![
            hit("low", "totally unrelated network socket code", 0.9),
            hit("high", "parse the config file for widgets", 0.1),
        ];
        let reranked = reranker.rerank("parse config file", &candidates)?;
        assert_eq!(reranked[0].doc_id, "high");
        Ok(())
    }

    #[test]
    fn rerank_reports_degraded_state() {
        let reranker = FusionScoreReranker::new();
        assert_eq!(
            reranker.state(),
            LoadState::Degraded(DegradedState::ProviderUnavailable)
        );
    }

    #[test]
    fn rerank_of_empty_candidates_is_empty() -> Result<()> {
        let reranker = FusionScoreReranker::new();
        assert!(reranker.rerank("anything", &[])?.is_empty());
        Ok(())
    }

    #[test]
    fn rerank_overwrites_score_field_not_just_reorders() -> Result<()> {
        let reranker = FusionScoreReranker::new();
        let candidates = vec![hit("a", "parse config file", 0.1)];
        let reranked = reranker.rerank("parse config file", &candidates)?;
        assert!(
            reranked[0].score > 0.1,
            "score should reflect the reranker's own blended score"
        );
        Ok(())
    }
}
