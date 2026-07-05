use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::model_observations::{
    ModelRuntimeObservationCandidate, ModelRuntimeObservationRecord, RerankerLiftProof,
    RetrievalQualityProof, RouteChoiceImprovement, TokenReductionProof,
};
use enforcer_memory::ranking::HardFilter;
use enforcer_memory::rerank::FusionScoreReranker;
use enforcer_memory::search::{DocumentKind, HybridSearcher, SearchDocument};
use enforcer_memory::vector::{embed_documents, VectorIndex};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture_corpus() -> Vec<SearchDocument> {
    vec![
        SearchDocument::new(
            "runtime:llama-cache",
            DocumentKind::Function,
            "llama cpp local runtime loads gguf model files from local cache with vulkan cpu cuda acceleration",
        ),
        SearchDocument::new(
            "runtime:onnx-ort",
            DocumentKind::Function,
            "onnx runtime optional ort backend loads onnx embeddings only when feature and local cache are available",
        ),
        SearchDocument::new(
            "learning:observations",
            DocumentKind::Function,
            "model load failures provider downgrade hash mismatch tokenizer mismatch degraded fallback successful local load are learning observation signals",
        ),
        SearchDocument::new(
            "proof:retrieval-quality",
            DocumentKind::File,
            "retrieval quality proof records recall reranker lift token reduction route choice improvement recurrence negative evidence",
        ),
        SearchDocument::new(
            "unrelated:network",
            DocumentKind::Function,
            "tcp socket retry timeout dns connection pooling network request budget",
        ),
    ]
}

#[test]
fn x06_retrieval_quality_metrics_emit_observation_ready_records() -> TestResult {
    let corpus = fixture_corpus();
    let fulltext = FullTextIndex::build(&corpus)?;
    let embedder = HashingEmbedder::new();
    let doc_texts: Vec<(String, String)> = corpus
        .iter()
        .map(|doc| (doc.id.clone(), doc.text.clone()))
        .collect();
    let entries = embed_documents(&embedder, &doc_texts)?;
    let vector = VectorIndex::build(&entries, embedder.model_info());
    let reranker = FusionScoreReranker::new();
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);
    let filters = vec![HardFilter::new("exclude-network", |doc_id| {
        doc_id != "unrelated:network"
    })];

    let result = searcher.search(
        "local llama gguf cache provider fallback",
        &corpus,
        &filters,
    )?;
    let returned: Vec<String> = result
        .context
        .iter()
        .map(|hit| hit.doc_id.clone())
        .collect();
    let expected = ["runtime:llama-cache", "learning:observations"];
    let relevant_returned = expected
        .iter()
        .filter(|expected_id| returned.iter().any(|actual| actual == *expected_id))
        .count();
    let recall_at_five = relevant_returned as f64 / expected.len() as f64;

    assert!(recall_at_five >= 0.5, "returned {returned:?}");
    assert!(
        !returned.iter().any(|doc_id| doc_id == "unrelated:network"),
        "hard-filtered network document must not enter proof context"
    );
    assert!(result.token_reduction_estimate.ratio() > 0.0);
    assert!(result.reranker_lift.is_finite());

    let records = vec![
        ModelRuntimeObservationRecord::new(
            "2026-07-05T00:00:00Z",
            "x06-retrieval-quality-fixture",
            "x06-retrieval-quality",
            ModelRuntimeObservationCandidate::RetrievalQualityProof(RetrievalQualityProof {
                query_id: "x06-q-local-runtime".to_string(),
                query: "local llama gguf cache provider fallback".to_string(),
                route: "hybrid-fulltext-vector-rerank".to_string(),
                recall_at_five,
                recall_at_ten: recall_at_five,
                precision_at_five: relevant_returned as f64 / returned.len().max(1) as f64,
                expected_top_k: expected.len(),
                returned_top_k: returned.len(),
            }),
        ),
        ModelRuntimeObservationRecord::new(
            "2026-07-05T00:00:00Z",
            "x06-retrieval-quality-fixture",
            "x06-retrieval-quality",
            ModelRuntimeObservationCandidate::RerankerLiftProof(RerankerLiftProof {
                query_id: "x06-q-local-runtime".to_string(),
                query: "local llama gguf cache provider fallback".to_string(),
                pre_rerank_top_k: result
                    .pre_rerank_pool
                    .iter()
                    .map(|trace| trace.doc_id.clone())
                    .collect(),
                post_rerank_top_k: returned,
                lift_score: result.reranker_lift,
                improved: result.reranker_lift >= 0.0,
            }),
        ),
        ModelRuntimeObservationRecord::new(
            "2026-07-05T00:00:00Z",
            "x06-retrieval-quality-fixture",
            "x06-retrieval-quality",
            ModelRuntimeObservationCandidate::TokenReductionProof(TokenReductionProof {
                query_id: "x06-q-local-runtime".to_string(),
                query: "local llama gguf cache provider fallback".to_string(),
                naive_tokens: result.token_reduction_estimate.naive_tokens,
                context_tokens: result.token_reduction_estimate.context_tokens,
            }),
        ),
        ModelRuntimeObservationRecord::new(
            "2026-07-05T00:00:00Z",
            "x06-retrieval-quality-fixture",
            "x06-retrieval-quality",
            ModelRuntimeObservationCandidate::RouteChoiceImprovement(RouteChoiceImprovement {
                query_id: "x06-q-local-runtime".to_string(),
                query: "local llama gguf cache provider fallback".to_string(),
                chosen_route: "hybrid-fulltext-vector-rerank".to_string(),
                alternative_route: "fulltext-only".to_string(),
                chosen_route_score: recall_at_five,
                alternative_route_score: 0.5,
                improvement_delta: recall_at_five - 0.5,
            }),
        ),
    ];

    let serialized = serde_json::to_value(&records)?;
    assert_eq!(serialized.as_array().map(Vec::len), Some(4));
    assert_eq!(
        serialized[0]["candidate"]["observationKind"],
        "retrieval-quality-proof"
    );
    Ok(())
}
