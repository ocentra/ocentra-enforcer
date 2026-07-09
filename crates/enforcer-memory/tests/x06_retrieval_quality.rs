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
use std::collections::{BTreeMap, BTreeSet};

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
    let candidates = serialized
        .as_array()
        .ok_or("serialized retrieval-quality records must be an array")?;
    let mut candidates_by_kind = BTreeMap::new();
    for record in candidates {
        let candidate = &record["candidate"];
        let kind = candidate["observationKind"]
            .as_str()
            .ok_or("serialized candidate missing observationKind")?;
        candidates_by_kind.insert(kind.to_owned(), candidate);
    }

    let retrieval_quality = candidates_by_kind
        .get("retrieval-quality-proof")
        .ok_or("missing retrieval-quality-proof observation")?;
    assert_eq!(retrieval_quality["queryId"], "x06-q-local-runtime");
    assert_eq!(retrieval_quality["route"], "hybrid-fulltext-vector-rerank");
    assert_eq!(retrieval_quality["expectedTopK"], expected.len());
    assert_eq!(retrieval_quality["returnedTopK"], result.context.len());
    assert_json_number_close(&retrieval_quality["recallAtFive"], recall_at_five)?;

    let reranker_lift = candidates_by_kind
        .get("reranker-lift-proof")
        .ok_or("missing reranker-lift-proof observation")?;
    assert_eq!(reranker_lift["queryId"], "x06-q-local-runtime");
    assert_eq!(
        reranker_lift["postRerankTopK"].as_array().map(Vec::len),
        Some(result.context.len())
    );
    assert_eq!(reranker_lift["improved"], result.reranker_lift >= 0.0);
    assert_json_number_close(&reranker_lift["liftScore"], result.reranker_lift)?;

    let token_reduction = candidates_by_kind
        .get("token-reduction-proof")
        .ok_or("missing token-reduction-proof observation")?;
    assert_eq!(token_reduction["queryId"], "x06-q-local-runtime");
    assert_eq!(
        token_reduction["naiveTokens"],
        result.token_reduction_estimate.naive_tokens
    );
    assert_eq!(
        token_reduction["contextTokens"],
        result.token_reduction_estimate.context_tokens
    );

    let route_choice = candidates_by_kind
        .get("route-choice-improvement")
        .ok_or("missing route-choice-improvement observation")?;
    assert_eq!(route_choice["queryId"], "x06-q-local-runtime");
    assert_eq!(route_choice["chosenRoute"], "hybrid-fulltext-vector-rerank");
    assert_eq!(route_choice["alternativeRoute"], "fulltext-only");
    assert_json_number_close(&route_choice["chosenRouteScore"], recall_at_five)?;
    assert_json_number_close(&route_choice["alternativeRouteScore"], 0.5)?;
    assert_json_number_close(&route_choice["improvementDelta"], recall_at_five - 0.5)?;
    Ok(())
}

#[test]
fn checked_in_token_reduction_rollup_is_derived_from_retrieval_observations() -> TestResult {
    let retrieval: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-retrieval-quality.json"
    ))?;
    let token_rollup: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-token-reduction.json"
    ))?;

    let observations = retrieval["observations"]
        .as_array()
        .ok_or("x06-retrieval-quality observations must be an array")?;
    let mut ratios = Vec::new();
    for observation in observations {
        let candidate = &observation["candidate"];
        if candidate["observationKind"] != "token-reduction-proof" {
            continue;
        }
        let naive = candidate["naiveTokens"]
            .as_f64()
            .ok_or("token-reduction observation missing naiveTokens")?;
        let context = candidate["contextTokens"]
            .as_f64()
            .ok_or("token-reduction observation missing contextTokens")?;
        if context <= 0.0 {
            return Err("token-reduction observation must have positive contextTokens".into());
        }
        ratios.push(naive / context);
    }
    ratios.sort_by(|a, b| a.total_cmp(b));

    assert_eq!(
        token_rollup["derivedFrom"],
        "proof/memory/x06-retrieval-quality.json#/observations[token-reduction-proof]"
    );
    assert_eq!(token_rollup["distribution"]["queryCount"], ratios.len());
    assert!(token_rollup["passes10xGate"].as_bool().unwrap_or(false));

    let median = percentile_nearest_rank(&ratios, 50.0)?;
    let p95 = percentile_nearest_rank(&ratios, 95.0)?;
    let minimum = *ratios.first().ok_or("no token-reduction observations")?;
    assert_json_number_close(&token_rollup["medianReductionRatio"], median)?;
    assert_json_number_close(
        &token_rollup["distribution"]["minimumReductionRatio"],
        minimum,
    )?;
    assert_json_number_close(&token_rollup["distribution"]["p50ReductionRatio"], median)?;
    assert_json_number_close(&token_rollup["distribution"]["p95ReductionRatio"], p95)?;
    Ok(())
}

#[test]
fn checked_in_retrieval_quality_proof_has_complete_observation_triplets() -> TestResult {
    let retrieval: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-retrieval-quality.json"
    ))?;

    assert_eq!(retrieval["artifact"], "x06-retrieval-quality");
    assert_eq!(retrieval["workpack"], "x06-models-harvest");
    assert_eq!(
        retrieval["evidence"]["degradedDefaultState"]["acceptedAsFeatureParity"], false,
        "degraded default model state must not be accepted as feature parity"
    );
    assert_eq!(
        retrieval["evidence"]["modelCapability"]["embedderLoadState"],
        "degraded-provider-unavailable"
    );
    assert_eq!(
        retrieval["evidence"]["modelCapability"]["rerankerLoadState"],
        "degraded-provider-unavailable"
    );

    let observations = retrieval["observations"]
        .as_array()
        .ok_or("x06-retrieval-quality observations must be an array")?;
    let declared_query_count = retrieval["queries"]
        .as_u64()
        .ok_or("x06-retrieval-quality queries must be a number")?
        as usize;

    let mut kinds_by_query: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for observation in observations {
        let candidate = &observation["candidate"];
        let query_id = candidate["queryId"]
            .as_str()
            .ok_or("retrieval observation missing queryId")?;
        let kind = candidate["observationKind"]
            .as_str()
            .ok_or("retrieval observation missing observationKind")?;
        kinds_by_query
            .entry(query_id.to_owned())
            .or_default()
            .insert(kind.to_owned());
    }

    assert_eq!(
        kinds_by_query.len(),
        declared_query_count,
        "declared query count must match distinct observed query ids"
    );
    for (query_id, kinds) in &kinds_by_query {
        for required in [
            "retrieval-quality-proof",
            "reranker-lift-proof",
            "token-reduction-proof",
        ] {
            assert!(
                kinds.contains(required),
                "query {query_id} missing required observation kind {required}; kinds={kinds:?}"
            );
        }
    }
    Ok(())
}

fn percentile_nearest_rank(values: &[f64], percentile: f64) -> Result<f64, &'static str> {
    if values.is_empty() {
        return Err("cannot compute percentile over empty values");
    }
    let rank = ((percentile / 100.0) * values.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(values.len() - 1);
    values
        .get(index)
        .copied()
        .ok_or("percentile index must resolve")
}

fn assert_json_number_close(value: &serde_json::Value, expected: f64) -> TestResult {
    let actual = value.as_f64().ok_or("expected JSON number")?;
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
    Ok(())
}
