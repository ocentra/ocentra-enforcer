//! Combined integration tests for the X06.4 hybrid search stack
//! (`fulltext`, `vector`, `embed`, `rerank`, `ranking`) and the X06.5
//! weaver enrichment worker pool (`enrichment`), migrated out of each
//! module's inline `#[cfg(test)]` block into one integration test file.

use enforcer_domain::memory_types::{DegradedState, LoadState, ParserSourceText};
use enforcer_domain::memory_types::{
    DocumentKind, EmbeddingGenerationId, EmbeddingVector, MemoryPriority, MemoryQueueLength,
    RetryAttemptCount, TaskOutcome, VectorIndexEntries, VectorIndexEntry, VectorStaleReason,
    WorkerConcurrency,
};
use enforcer_memory::embed::{cosine_similarity, Embedder, EmbeddingModelInfo, HashingEmbedder};
use enforcer_memory::enrichment::{
    process_event, EnrichmentContext, FlakyEmbedder, NullEmbedder, WorkerPool, WorkerPoolConfig,
    WorkerPoolOutcomeChannel,
};
use enforcer_memory::error::Result;
use enforcer_memory::fulltext::{tokenize, FullTextIndex};
use enforcer_memory::queue::{RetryPolicy, WeaverEvent, WeaverQueue};
use enforcer_memory::ranking::{
    fuse_rrf, reranker_lift, CandidateTrace, HardFilter, RankedHit, ScoredCandidate,
};
use enforcer_memory::rerank::{FusionScoreReranker, Reranker};
use enforcer_memory::search::document::SearchDocument;
use enforcer_memory::vector::{embed_documents, VectorIndex, VectorManifest};
use std::error::Error;
use std::sync::Arc;

// ---------------------------------------------------------------------
// fulltext.rs
// ---------------------------------------------------------------------

fn tokenize_strings(text: &str) -> Vec<String> {
    tokenize(&text.into()).into_iter().map(Into::into).collect()
}

#[test]
fn fulltext_tokenize_splits_camel_case() {
    let terms = tokenize_strings("parseConfigFile");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
    assert!(terms.contains(&"file".to_string()));
    assert!(terms.contains(&"parseconfigfile".to_string()));
}

#[test]
fn fulltext_tokenize_splits_snake_case() {
    let terms = tokenize_strings("parse_config_file");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
    assert!(terms.contains(&"file".to_string()));
}

#[test]
fn fulltext_tokenize_splits_kebab_case() {
    let terms = tokenize_strings("parse-config-file");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
}

#[test]
fn fulltext_tokenize_splits_path_separators() {
    let terms = tokenize_strings("crates/enforcer-memory/src/fulltext.rs");
    assert!(terms.contains(&"enforcer".to_string()));
    assert!(terms.contains(&"memory".to_string()));
    assert!(terms.contains(&"fulltext".to_string()));
}

#[test]
fn fulltext_tokenize_keeps_version_digits_attached() {
    let terms = tokenize_strings("schemaV2Migration");
    assert!(terms.iter().any(|t| t.contains("v2") || t == "v2"));
}

#[test]
fn fulltext_exact_query_finds_exact_document() -> Result<()> {
    let docs = vec![
        SearchDocument::new(
            "sym:a.rs:1:parseConfigFile",
            DocumentKind::Function,
            "fn parseConfigFile() { read the config file }",
        ),
        SearchDocument::new(
            "sym:b.rs:1:writeLog",
            DocumentKind::Function,
            "fn writeLog() { append to the log }",
        ),
    ];
    let index = FullTextIndex::build(&docs)?;
    let hits = index.search(&"parseConfigFile".into(), 10.into())?;
    assert_eq!(hits[0].doc_id, "sym:a.rs:1:parseConfigFile");
    Ok(())
}

#[test]
fn fulltext_split_component_query_also_matches() -> Result<()> {
    let docs = vec![SearchDocument::new(
        "sym:a.rs:1:parseConfigFile",
        DocumentKind::Function,
        "fn parseConfigFile() { read the config file }",
    )];
    let index = FullTextIndex::build(&docs)?;
    let hits = index.search(&"config".into(), 10.into())?;
    assert_eq!(hits.len(), 1);
    Ok(())
}

#[test]
fn fulltext_structural_label_boost_orders_function_above_file_for_equal_relevance() -> Result<()> {
    let docs = vec![
        SearchDocument::new("file:a.rs", DocumentKind::File, "widget widget widget"),
        SearchDocument::new("sym:a.rs:1:widget", DocumentKind::Function, "widget"),
    ];
    let index = FullTextIndex::build(&docs)?;
    let hits = index.search(&"widget".into(), 10.into())?;
    assert_eq!(hits.len(), 2);
    assert_eq!(
        hits[0].doc_id, "sym:a.rs:1:widget",
        "Function boost should outrank a lexically-denser File match"
    );
    Ok(())
}

#[test]
fn fulltext_empty_index_returns_no_hits() -> Result<()> {
    let index = FullTextIndex::build(&[])?;
    assert!(index.is_empty());
    let hits = index.search(&"anything".into(), 10.into())?;
    assert!(hits.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------
// vector.rs
// ---------------------------------------------------------------------

fn vector_model_info() -> EmbeddingModelInfo {
    HashingEmbedder::new().model_info()
}

#[test]
fn vector_exact_vector_query_returns_the_matching_document_first() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let mut entries = VectorIndexEntries::new();
    entries.push(VectorIndexEntry {
        doc_id: "a".into(),
        vector: embedder.embed(ParserSourceText::from("parse config file"))?,
    });
    entries.push(VectorIndexEntry {
        doc_id: "b".into(),
        vector: embedder.embed(ParserSourceText::from("write log entry"))?,
    });
    let index = VectorIndex::build(entries, vector_model_info());
    let query_vec = embedder.embed(ParserSourceText::from("parse config file"))?;
    let hits = index.search(query_vec, 2);
    assert_eq!(hits[0].doc_id, "a");
    Ok(())
}

#[test]
fn vector_empty_index_returns_no_hits() {
    let index = VectorIndex::build(&[], vector_model_info());
    assert!(index.is_empty().is_enabled());
    let hits = index.search(&[0.0, 1.0], 5);
    assert!(hits.is_empty());
}

#[test]
fn vector_manifest_matches_identical_model_info() {
    let manifest = VectorManifest::new(vector_model_info());
    assert!(bool::from(manifest.matches(&vector_model_info())));
}

#[test]
fn vector_manifest_detects_dimension_mismatch() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.dimension += 1;
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, VectorStaleReason::Dimension { .. })));
    assert!(!bool::from(manifest.matches(&other)));
}

#[test]
fn vector_manifest_detects_embedding_model_name_mismatch() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.embedding_model = "some-other-model".into();
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, VectorStaleReason::EmbeddingModel { .. })));
}

#[test]
fn vector_manifest_reports_every_mismatched_field_not_just_the_first() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.dimension += 1;
    other.dtype = "f16".into();
    let diff = manifest.diff(&other);
    assert!(diff.len() >= 2);
}

#[test]
fn vector_embed_documents_dedups_repeated_doc_ids() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let docs = vec![
        ("a".to_owned(), "first".to_owned()),
        ("a".to_owned(), "second".to_owned()),
    ];
    let entries = embed_documents(&embedder, &docs)?;
    assert_eq!(entries.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------
// embed.rs
// ---------------------------------------------------------------------

fn embed_l2_normalize(vector: &mut [f32]) {
    let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector.iter_mut() {
            *value /= norm;
        }
    }
}

#[test]
fn embed_hashing_embedder_is_deterministic_across_calls() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let a = embedder.embed(ParserSourceText::from("parseConfigFile"))?;
    let b = embedder.embed(ParserSourceText::from("parseConfigFile"))?;
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn embed_hashing_embedder_reports_degraded_state() {
    let embedder = HashingEmbedder::new();
    assert_eq!(
        embedder.state(),
        LoadState::Degraded(DegradedState::ProviderUnavailable)
    );
}

#[test]
fn embed_shared_vocabulary_queries_are_more_similar_than_disjoint_ones() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let a = embedder.embed(ParserSourceText::from(
        "parse config file for the widget loader",
    ))?;
    let b = embedder.embed(ParserSourceText::from(
        "parse config file for the widget reader",
    ))?;
    let c = embedder.embed(ParserSourceText::from(
        "unrelated network socket timeout retry logic",
    ))?;
    let sim_ab = cosine_similarity(&a, &b);
    let sim_ac = cosine_similarity(&a, &c);
    assert!(
        sim_ab > sim_ac,
        "shared-vocabulary texts should be closer than disjoint-vocabulary ones: {sim_ab} vs {sim_ac}"
    );
    Ok(())
}

#[test]
fn embed_cosine_similarity_is_zero_for_mismatched_lengths() {
    let left = EmbeddingVector::from(vec![1.0, 0.0]);
    let right = EmbeddingVector::from(vec![1.0, 0.0, 0.0]);
    assert_eq!(cosine_similarity(&left, &right), 0.0);
}

#[test]
fn embed_cosine_similarity_is_one_for_identical_normalized_vectors() {
    let mut v = vec![3.0f32, 4.0];
    embed_l2_normalize(&mut v);
    let v = EmbeddingVector::from(v);
    let sim = cosine_similarity(&v, &v);
    assert!((sim.get() - 1.0).abs() < 1e-6);
}

#[test]
fn embed_model_info_reports_stable_version_vector_fields() {
    let embedder = HashingEmbedder::new();
    let info = embedder.model_info();
    assert_eq!(
        info.dimension,
        enforcer_memory::embed::HASHING_EMBEDDER_DIMENSION
    );
    assert_eq!(info.similarity_metric, "cosine");
}

// ---------------------------------------------------------------------
// rerank.rs
// ---------------------------------------------------------------------

fn rerank_hit(doc_id: &str, snippet: &str, score: f64) -> RankedHit {
    RankedHit {
        doc_id: doc_id.into(),
        kind: DocumentKind::Function,
        snippet: snippet.into(),
        source_path: None,
        score: score.into(),
    }
}

#[test]
fn rerank_prefers_higher_lexical_overlap_with_query() -> Result<()> {
    let reranker = FusionScoreReranker::new();
    let candidates = vec![
        rerank_hit("low", "totally unrelated network socket code", 0.9),
        rerank_hit("high", "parse the config file for widgets", 0.1),
    ];
    let reranked = reranker.rerank(
        enforcer_domain::memory_types::ParserSourceText::from("parse config file"),
        &candidates,
    )?;
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
    assert!(reranker
        .rerank(
            enforcer_domain::memory_types::ParserSourceText::from("anything"),
            &[],
        )?
        .is_empty());
    Ok(())
}

#[test]
fn rerank_overwrites_score_field_not_just_reorders() -> Result<()> {
    let reranker = FusionScoreReranker::new();
    let candidates = vec![rerank_hit("a", "parse config file", 0.1)];
    let reranked = reranker.rerank(
        enforcer_domain::memory_types::ParserSourceText::from("parse config file"),
        &candidates,
    )?;
    assert!(
        reranked[0].score > 0.1,
        "score should reflect the reranker's own blended score"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// ranking.rs
// ---------------------------------------------------------------------

fn ranking_doc(id: &str) -> SearchDocument {
    SearchDocument::new(id, DocumentKind::Function, format!("body of {id}"))
}

#[test]
fn ranking_fuse_rrf_ranks_documents_present_in_both_retrievers_highest() {
    let corpus = vec![ranking_doc("a"), ranking_doc("b"), ranking_doc("c")];
    let fulltext = vec![
        ScoredCandidate {
            doc_id: "a".into(),
            score: 10.0.into(),
        },
        ScoredCandidate {
            doc_id: "b".into(),
            score: 5.0.into(),
        },
    ];
    let vector = vec![
        ScoredCandidate {
            doc_id: "a".into(),
            score: 0.9.into(),
        },
        ScoredCandidate {
            doc_id: "c".into(),
            score: 0.5.into(),
        },
    ];
    let result = fuse_rrf(&fulltext, &vector, &corpus, &[], 60.0.into());
    assert_eq!(
        result.candidates[0].doc_id, "a",
        "present in both retrievers at rank 1 each"
    );
}

#[test]
fn ranking_hard_filters_exclude_before_fusion() {
    let corpus = vec![ranking_doc("a"), ranking_doc("blocked")];
    let fulltext = vec![
        ScoredCandidate {
            doc_id: "blocked".into(),
            score: 100.0.into(),
        },
        ScoredCandidate {
            doc_id: "a".into(),
            score: 1.0.into(),
        },
    ];
    let filters = vec![HardFilter::from_predicate("no-blocked".into(), |id| {
        (id != "blocked").into()
    })];
    let result = fuse_rrf(&fulltext, &[], &corpus, &filters, 60.0.into());
    assert!(
        result.candidates.iter().all(|hit| hit.doc_id != "blocked"),
        "hard-filtered doc must never enter the pre-rerank pool"
    );
    assert!(result.pre_rerank_pool.iter().all(|t| t.doc_id != "blocked"));
}

#[test]
fn ranking_candidate_not_in_corpus_is_skipped_not_panicking() {
    let corpus = vec![ranking_doc("a")];
    let fulltext = vec![ScoredCandidate {
        doc_id: "ghost".into(),
        score: 1.0.into(),
    }];
    let result = fuse_rrf(&fulltext, &[], &corpus, &[], 60.0.into());
    assert!(result.candidates.is_empty());
}

#[test]
fn ranking_reranker_lift_is_zero_when_order_is_unchanged() {
    let pre = vec![
        CandidateTrace {
            doc_id: "a".into(),
            fulltext_rank: Some(1.into()),
            vector_rank: None,
            rrf_score: 1.0.into(),
        },
        CandidateTrace {
            doc_id: "b".into(),
            fulltext_rank: Some(2.into()),
            vector_rank: None,
            rrf_score: 0.5.into(),
        },
    ];
    let context = vec![
        RankedHit {
            doc_id: "a".into(),
            kind: DocumentKind::Function,
            snippet: String::new().into(),
            source_path: None,
            score: 1.0.into(),
        },
        RankedHit {
            doc_id: "b".into(),
            kind: DocumentKind::Function,
            snippet: String::new().into(),
            source_path: None,
            score: 0.5.into(),
        },
    ];
    assert_eq!(reranker_lift(&pre, &context), 0.0);
}

#[test]
fn ranking_reranker_lift_is_positive_when_order_changes() {
    let pre = vec![
        CandidateTrace {
            doc_id: "a".into(),
            fulltext_rank: Some(1.into()),
            vector_rank: None,
            rrf_score: 1.0.into(),
        },
        CandidateTrace {
            doc_id: "b".into(),
            fulltext_rank: Some(2.into()),
            vector_rank: None,
            rrf_score: 0.5.into(),
        },
    ];
    let context = vec![
        RankedHit {
            doc_id: "b".into(),
            kind: DocumentKind::Function,
            snippet: String::new().into(),
            source_path: None,
            score: 0.9.into(),
        },
        RankedHit {
            doc_id: "a".into(),
            kind: DocumentKind::Function,
            snippet: String::new().into(),
            source_path: None,
            score: 0.8.into(),
        },
    ];
    assert!(reranker_lift(&pre, &context) > 0.0);
}

#[test]
fn ranking_reranker_lift_is_zero_for_empty_inputs() {
    assert_eq!(reranker_lift(&[], &[]), 0.0);
}

// ---------------------------------------------------------------------
// enrichment.rs
// ---------------------------------------------------------------------

type EnrichmentTestResult = std::result::Result<(), Box<dyn Error>>;

fn enrichment_test_ctx(
    embedder: Arc<dyn enforcer_memory::enrichment::Embedder>,
) -> Arc<EnrichmentContext> {
    Arc::new(EnrichmentContext::new(
        embedder,
        EmbeddingGenerationId::INITIAL,
    ))
}

#[tokio::test]
async fn enrichment_node_changed_event_produces_an_embedding_task() -> EnrichmentTestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let ctx = enrichment_test_ctx(
        Arc::clone(&embedder) as Arc<dyn enforcer_memory::enrichment::Embedder>
    );
    let event = WeaverEvent::NodeChanged {
        node_id: "sym:src/lib.rs:1:foo".to_owned().into(),
        rel_path: "src/lib.rs".to_owned().into(),
        content_hash: "1111111111111111111111111111111111111111111111111111111111111111"
            .to_owned()
            .into(),
    };

    process_event(&ctx, &event).await?;

    let calls = embedder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].node_id.as_str(), "sym:src/lib.rs:1:foo");
    assert_eq!(
        calls[0].content_hash.as_str(),
        "1111111111111111111111111111111111111111111111111111111111111111"
    );
    Ok(())
}

#[tokio::test]
async fn enrichment_file_changed_event_invalidates_summary() -> EnrichmentTestResult {
    let embedder = Arc::new(NullEmbedder::new()) as Arc<dyn enforcer_memory::enrichment::Embedder>;
    let ctx = enrichment_test_ctx(embedder);
    ctx.with_summaries(|store| store.set_summary("src/lib.rs", "old summary"));
    let event = WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned().into(),
        content_hash: "hash-2".to_owned().into(),
    };

    process_event(&ctx, &event).await?;

    let is_stale = ctx.with_summaries(|store| store.is_stale("src/lib.rs").is_stale());
    assert!(is_stale);
    Ok(())
}

#[tokio::test]
async fn enrichment_retry_succeeds_after_transient_failure() -> EnrichmentTestResult {
    let flaky = Arc::new(FlakyEmbedder::fail_first_n(2));
    let ctx =
        enrichment_test_ctx(Arc::clone(&flaky) as Arc<dyn enforcer_memory::enrichment::Embedder>);
    let queue = WeaverQueue::new();
    let handle = queue.handle();
    let WorkerPoolOutcomeChannel {
        config,
        mut outcomes,
    } = WorkerPoolConfig {
        max_concurrency: WorkerConcurrency::from_nonzero(
            std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN),
        ),
        retry: RetryPolicy::bounded_default(),
        embedding_version: EmbeddingGenerationId::INITIAL,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    handle.send(
        WeaverEvent::NodeChanged {
            node_id: "sym:src/lib.rs:1:n1".to_owned().into(),
            rel_path: "src/lib.rs".to_owned().into(),
            content_hash: "3333333333333333333333333333333333333333333333333333333333333333"
                .to_owned()
                .into(),
        },
        MemoryPriority::Hot,
    )?;

    // Deterministic synchronization: wait on the outcome channel
    // for exactly two `RetryScheduled` outcomes followed by one
    // `Succeeded` -- no sleep-as-synchronization.
    let mut retries_seen = 0;
    let mut succeeded = false;
    let mut dead_lettered = false;
    while let Some(outcome) = outcomes.recv().await {
        match outcome {
            TaskOutcome::RetryScheduled { .. } => retries_seen += 1,
            TaskOutcome::Succeeded { .. } => {
                succeeded = true;
                break;
            }
            TaskOutcome::DeadLettered { .. } => {
                dead_lettered = true;
                break;
            }
        }
    }

    let dead_letters_len = pool.with_dead_letters(|dead_letters| dead_letters.len());
    drop(handle);
    pool.shutdown().await;

    assert!(
        !dead_lettered,
        "task must not dead-letter: it succeeds on the 3rd attempt"
    );
    assert_eq!(
        retries_seen, 2,
        "expected exactly 2 retry-scheduled outcomes"
    );
    assert!(succeeded, "expected the 3rd attempt to succeed");
    assert_eq!(
        flaky.attempts_seen().get(),
        3,
        "expected 2 failures + 1 success"
    );
    assert_eq!(
        dead_letters_len, 0,
        "a task that eventually succeeds must never reach the dead-letter queue"
    );
    Ok(())
}

#[tokio::test]
async fn enrichment_task_failing_every_retry_lands_in_dead_letter_queue() -> EnrichmentTestResult {
    let flaky = Arc::new(FlakyEmbedder::fail_first_n(1_000));
    let ctx =
        enrichment_test_ctx(Arc::clone(&flaky) as Arc<dyn enforcer_memory::enrichment::Embedder>);
    let queue = WeaverQueue::new();
    let handle = queue.handle();
    let retry = RetryPolicy {
        max_attempts: RetryAttemptCount::ZERO.next().next(),
        base_delay: std::time::Duration::from_millis(5).into(),
        max_delay: std::time::Duration::from_millis(20).into(),
    };
    let WorkerPoolOutcomeChannel {
        config,
        mut outcomes,
    } = WorkerPoolConfig {
        max_concurrency: WorkerConcurrency::from_nonzero(
            std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN),
        ),
        retry,
        embedding_version: EmbeddingGenerationId::INITIAL,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    let event = WeaverEvent::NodeChanged {
        node_id: "sym:src/dlq.rs:1:n_dlq".to_owned().into(),
        rel_path: "src/dlq.rs".to_owned().into(),
        content_hash: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
            .to_owned()
            .into(),
    };
    handle.send(event.clone(), MemoryPriority::Hot)?;

    // Deterministic synchronization: wait for the `DeadLettered`
    // outcome instead of polling the dead-letter queue on a sleep.
    let mut dead_lettered = false;
    while let Some(outcome) = outcomes.recv().await {
        if let TaskOutcome::DeadLettered { .. } = outcome {
            dead_lettered = true;
            break;
        }
    }
    assert!(
        dead_lettered,
        "expected the task to reach the dead-letter queue"
    );

    let dead_letter_state = pool.with_dead_letters(|dead_letters| {
        let found = dead_letters.find(&event.task_key());
        (
            dead_letters.len(),
            found.len(),
            found.first().map(|task| u32::from(task.attempts)),
        )
    });
    drop(handle);
    pool.shutdown().await;

    assert_eq!(dead_letter_state, (MemoryQueueLength::from(1), 1, Some(2)));
    Ok(())
}
