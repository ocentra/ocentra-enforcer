use enforcer_memory::embed::{DegradedState, LoadState};
use enforcer_memory::error::Result;
use enforcer_memory::ranking::RankedHit;
use enforcer_memory::rerank::{FusionScoreReranker, Reranker};
use enforcer_memory::search::document::DocumentKind;

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
