//! Combined integration tests for the X06.4 hybrid search stack
//! (`fulltext`, `vector`, `embed`, `rerank`, `ranking`) and the X06.5
//! weaver enrichment worker pool (`enrichment`), migrated out of each
//! module's inline `#[cfg(test)]` block into one integration test file.

use enforcer_memory::embed::{
    cosine_similarity, DegradedState, Embedder, EmbeddingModelInfo, HashingEmbedder, LoadState,
};
use enforcer_memory::enrichment::{
    process_event, EnrichmentContext, FlakyEmbedder, NullEmbedder, TaskOutcome, WorkerPool,
    WorkerPoolConfig,
};
use enforcer_memory::error::Result;
use enforcer_memory::fulltext::{tokenize, FullTextIndex};
use enforcer_memory::queue::{Priority, RetryPolicy, WeaverEvent, WeaverQueue};
use enforcer_memory::ranking::{
    fuse_rrf, reranker_lift, CandidateTrace, HardFilter, RankedHit, ScoredCandidate,
};
use enforcer_memory::rerank::{FusionScoreReranker, Reranker};
use enforcer_memory::search::document::{DocumentKind, SearchDocument};
use enforcer_memory::summaries::SummaryStore;
use enforcer_memory::vector::{embed_documents, StaleReason, VectorIndex, VectorManifest};
use std::error::Error;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------
// fulltext.rs
// ---------------------------------------------------------------------

#[test]
fn fulltext_tokenize_splits_camel_case() {
    let terms = tokenize("parseConfigFile");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
    assert!(terms.contains(&"file".to_string()));
    assert!(terms.contains(&"parseconfigfile".to_string()));
}

#[test]
fn fulltext_tokenize_splits_snake_case() {
    let terms = tokenize("parse_config_file");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
    assert!(terms.contains(&"file".to_string()));
}

#[test]
fn fulltext_tokenize_splits_kebab_case() {
    let terms = tokenize("parse-config-file");
    assert!(terms.contains(&"parse".to_string()));
    assert!(terms.contains(&"config".to_string()));
}

#[test]
fn fulltext_tokenize_splits_path_separators() {
    let terms = tokenize("crates/enforcer-memory/src/fulltext.rs");
    assert!(terms.contains(&"enforcer".to_string()));
    assert!(terms.contains(&"memory".to_string()));
    assert!(terms.contains(&"fulltext".to_string()));
}

#[test]
fn fulltext_tokenize_keeps_version_digits_attached() {
    let terms = tokenize("schemaV2Migration");
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
    let hits = index.search("parseConfigFile", 10)?;
    assert!(!hits.is_empty());
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
    let hits = index.search("config", 10)?;
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
    let hits = index.search("widget", 10)?;
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
    let hits = index.search("anything", 10)?;
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
    let entries = vec![
        ("a".to_owned(), embedder.embed("parse config file")?),
        ("b".to_owned(), embedder.embed("write log entry")?),
    ];
    let index = VectorIndex::build(&entries, vector_model_info());
    let query_vec = embedder.embed("parse config file")?;
    let hits = index.search(&query_vec, 2);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].doc_id, "a");
    Ok(())
}

#[test]
fn vector_empty_index_returns_no_hits() {
    let index = VectorIndex::build(&[], vector_model_info());
    assert!(index.is_empty());
    let hits = index.search(&[0.0, 1.0], 5);
    assert!(hits.is_empty());
}

#[test]
fn vector_manifest_matches_identical_model_info() {
    let manifest = VectorManifest::new(vector_model_info());
    assert!(manifest.matches(&vector_model_info()));
}

#[test]
fn vector_manifest_detects_dimension_mismatch() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.dimension += 1;
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, StaleReason::Dimension { .. })));
    assert!(!manifest.matches(&other));
}

#[test]
fn vector_manifest_detects_embedding_model_name_mismatch() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.embedding_model = "some-other-model".to_owned();
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, StaleReason::EmbeddingModel { .. })));
}

#[test]
fn vector_manifest_reports_every_mismatched_field_not_just_the_first() {
    let manifest = VectorManifest::new(vector_model_info());
    let mut other = vector_model_info();
    other.dimension += 1;
    other.dtype = "f16".to_owned();
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
    let a = embedder.embed("parseConfigFile")?;
    let b = embedder.embed("parseConfigFile")?;
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
    let a = embedder.embed("parse config file for the widget loader")?;
    let b = embedder.embed("parse config file for the widget reader")?;
    let c = embedder.embed("unrelated network socket timeout retry logic")?;
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
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
}

#[test]
fn embed_cosine_similarity_is_one_for_identical_normalized_vectors() {
    let mut v = vec![3.0f32, 4.0];
    embed_l2_normalize(&mut v);
    let sim = cosine_similarity(&v, &v);
    assert!((sim - 1.0).abs() < 1e-6);
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
        rerank_hit("low", "totally unrelated network socket code", 0.9),
        rerank_hit("high", "parse the config file for widgets", 0.1),
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
    let candidates = vec![rerank_hit("a", "parse config file", 0.1)];
    let reranked = reranker.rerank("parse config file", &candidates)?;
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
            score: 10.0,
        },
        ScoredCandidate {
            doc_id: "b".into(),
            score: 5.0,
        },
    ];
    let vector = vec![
        ScoredCandidate {
            doc_id: "a".into(),
            score: 0.9,
        },
        ScoredCandidate {
            doc_id: "c".into(),
            score: 0.5,
        },
    ];
    let result = fuse_rrf(&fulltext, &vector, &corpus, &[], 60.0);
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
            score: 100.0,
        },
        ScoredCandidate {
            doc_id: "a".into(),
            score: 1.0,
        },
    ];
    let filters = vec![HardFilter::new("no-blocked", |id: &str| id != "blocked")];
    let result = fuse_rrf(&fulltext, &[], &corpus, &filters, 60.0);
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
        score: 1.0,
    }];
    let result = fuse_rrf(&fulltext, &[], &corpus, &[], 60.0);
    assert!(result.candidates.is_empty());
}

#[test]
fn ranking_reranker_lift_is_zero_when_order_is_unchanged() {
    let pre = vec![
        CandidateTrace {
            doc_id: "a".into(),
            fulltext_rank: Some(1),
            vector_rank: None,
            rrf_score: 1.0,
        },
        CandidateTrace {
            doc_id: "b".into(),
            fulltext_rank: Some(2),
            vector_rank: None,
            rrf_score: 0.5,
        },
    ];
    let context = vec![
        RankedHit {
            doc_id: "a".into(),
            kind: DocumentKind::Function,
            snippet: String::new(),
            source_path: None,
            score: 1.0,
        },
        RankedHit {
            doc_id: "b".into(),
            kind: DocumentKind::Function,
            snippet: String::new(),
            source_path: None,
            score: 0.5,
        },
    ];
    assert_eq!(reranker_lift(&pre, &context), 0.0);
}

#[test]
fn ranking_reranker_lift_is_positive_when_order_changes() {
    let pre = vec![
        CandidateTrace {
            doc_id: "a".into(),
            fulltext_rank: Some(1),
            vector_rank: None,
            rrf_score: 1.0,
        },
        CandidateTrace {
            doc_id: "b".into(),
            fulltext_rank: Some(2),
            vector_rank: None,
            rrf_score: 0.5,
        },
    ];
    let context = vec![
        RankedHit {
            doc_id: "b".into(),
            kind: DocumentKind::Function,
            snippet: String::new(),
            source_path: None,
            score: 0.9,
        },
        RankedHit {
            doc_id: "a".into(),
            kind: DocumentKind::Function,
            snippet: String::new(),
            source_path: None,
            score: 0.8,
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

/// `std::sync::Mutex::lock`'s `PoisonError<MutexGuard<'_, T>>` is not
/// `'static` (it embeds the guard), so it cannot convert into
/// `Box<dyn Error>` via a bare `?` -- this maps it to an owned,
/// `'static` message first.
fn enrichment_lock_or_msg<T>(
    mutex: &Mutex<T>,
) -> std::result::Result<std::sync::MutexGuard<'_, T>, Box<dyn Error>> {
    mutex
        .lock()
        .map_err(|_poison_error| "mutex poisoned".into())
}

fn enrichment_test_ctx(
    embedder: Arc<dyn enforcer_memory::enrichment::Embedder>,
) -> Arc<EnrichmentContext> {
    Arc::new(EnrichmentContext {
        embedder,
        summaries: Arc::new(Mutex::new(SummaryStore::new())),
        embedding_version: 1,
    })
}

#[tokio::test]
async fn enrichment_node_changed_event_produces_an_embedding_task() -> EnrichmentTestResult {
    let embedder = Arc::new(NullEmbedder::new());
    let ctx = enrichment_test_ctx(
        Arc::clone(&embedder) as Arc<dyn enforcer_memory::enrichment::Embedder>
    );
    let event = WeaverEvent::NodeChanged {
        node_id: "sym:src/lib.rs:1:foo".to_owned(),
        rel_path: "src/lib.rs".to_owned(),
        content_hash: "hash-1".to_owned(),
    };

    process_event(&ctx, &event).await?;

    let calls = embedder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].node_id, "sym:src/lib.rs:1:foo");
    assert_eq!(calls[0].content_hash, "hash-1");
    Ok(())
}

#[tokio::test]
async fn enrichment_file_changed_event_invalidates_summary() -> EnrichmentTestResult {
    let embedder = Arc::new(NullEmbedder::new()) as Arc<dyn enforcer_memory::enrichment::Embedder>;
    let ctx = enrichment_test_ctx(embedder);
    {
        let mut store = enrichment_lock_or_msg(&ctx.summaries)?;
        store.set_summary("src/lib.rs", "old summary");
    }
    let event = WeaverEvent::FileChanged {
        rel_path: "src/lib.rs".to_owned(),
        content_hash: "hash-2".to_owned(),
    };

    process_event(&ctx, &event).await?;

    let is_stale = {
        let store = enrichment_lock_or_msg(&ctx.summaries)?;
        store.is_stale("src/lib.rs")
    };
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
    let (config, mut outcomes) = WorkerPoolConfig {
        max_concurrency: 2,
        retry: RetryPolicy::bounded_default(),
        embedding_version: 1,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    handle.send(
        WeaverEvent::NodeChanged {
            node_id: "n1".to_owned(),
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-3".to_owned(),
        },
        Priority::Hot,
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

    let dead_letters_len = pool.dead_letters.lock().map(|d| d.len()).unwrap_or(0);
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
    assert_eq!(flaky.attempts_seen(), 3, "expected 2 failures + 1 success");
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
        max_attempts: 2,
        base_delay: std::time::Duration::from_millis(5),
        max_delay: std::time::Duration::from_millis(20),
    };
    let (config, mut outcomes) = WorkerPoolConfig {
        max_concurrency: 2,
        retry,
        embedding_version: 1,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    let event = WeaverEvent::NodeChanged {
        node_id: "n-dlq".to_owned(),
        rel_path: "src/dlq.rs".to_owned(),
        content_hash: "hash-dlq".to_owned(),
    };
    handle.send(event.clone(), Priority::Hot)?;

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

    let dead_letters = Arc::clone(&pool.dead_letters);
    drop(handle);
    pool.shutdown().await;

    let dlq_guard = enrichment_lock_or_msg(&dead_letters)?;
    assert_eq!(dlq_guard.len(), 1);
    let found = dlq_guard.find(&event.task_key());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].attempts, 2);
    Ok(())
}
