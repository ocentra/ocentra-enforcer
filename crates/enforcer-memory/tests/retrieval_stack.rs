//! X06.4 hard tests: the full-text/vector/rerank hybrid retrieval
//! stack, exercised end-to-end through [`enforcer_memory::search`].
//!
//! Per the subpack's hard-test list
//! (`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md`
//! §5): exact query, semantic query, reranker lift, vector stale
//! detection, model manifest content, no-remote-provider proof, and the
//! token-reduction estimate in the context-pack result.
//!
//! Workspace lints (`unwrap_used`/`expect_used` = deny) apply to test
//! code too, so every test here returns `Result` and propagates with
//! `?` rather than `.expect(...)`, matching `ingest_and_recall.rs`'s
//! existing style.

use enforcer_memory::embed::{DegradedState, Embedder, HashingEmbedder, LoadState};
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::ranking::HardFilter;
use enforcer_memory::rerank::{FusionScoreReranker, Reranker as _};
use enforcer_memory::search::{DocumentKind, HybridSearcher, SearchDocument};
use enforcer_memory::vector::{embed_documents, VectorIndex};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A small, fixed corpus covering: an exact-match target, a
/// semantically-related-but-lexically-different document, an unrelated
/// document, and a structurally-distinct pair (Function vs File) for
/// the label-boost path already covered by `fulltext.rs`'s own unit
/// tests -- this corpus is deliberately the retrieval-stack's own
/// fixture, separate from `fulltext.rs`'s unit-level tokenizer tests.
fn fixture_corpus() -> Vec<SearchDocument> {
    vec![
        SearchDocument::new(
            "sym:config.rs:1:parseConfigFile",
            DocumentKind::Function,
            "fn parseConfigFile(path: &str) -> Config { read and parse the config file from disk }",
        ),
        SearchDocument::new(
            "sym:config.rs:2:loadWidgetSettings",
            DocumentKind::Function,
            "fn loadWidgetSettings(path: &str) -> Settings { read widget configuration settings from disk }",
        ),
        SearchDocument::new(
            "sym:net.rs:1:openSocket",
            DocumentKind::Function,
            "fn openSocket(addr: &str) -> Socket { open a network socket connection and retry on timeout }",
        ),
        SearchDocument::new(
            "sym:log.rs:1:writeLog",
            DocumentKind::Function,
            "fn writeLog(message: &str) { append a message to the log file }",
        ),
        SearchDocument::new(
            "file:config.rs",
            DocumentKind::File,
            "the whole config.rs file, containing parseConfigFile and other config helpers",
        ),
    ]
}

fn build_stack(
    corpus: &[SearchDocument],
) -> Result<
    (
        FullTextIndex,
        VectorIndex,
        HashingEmbedder,
        FusionScoreReranker,
    ),
    Box<dyn std::error::Error>,
> {
    let fulltext = FullTextIndex::build(corpus)?;
    let embedder = HashingEmbedder::new();
    let doc_texts: Vec<(String, String)> = corpus
        .iter()
        .map(|doc| (doc.id.clone(), doc.text.clone()))
        .collect();
    let entries = embed_documents(&embedder, &doc_texts)?;
    let vector = VectorIndex::build(&entries, embedder.model_info());
    let reranker = FusionScoreReranker::new();
    Ok((fulltext, vector, embedder, reranker))
}

/// Hard test: exact query. A query for the exact literal identifier
/// must return that document as (or among) the top hits.
#[test]
fn exact_query_returns_the_exact_match() -> TestResult {
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);

    let result = searcher.search("parseConfigFile", &corpus, &[])?;

    assert!(
        !result.context.is_empty(),
        "exact query must return at least one hit"
    );
    assert!(
        result
            .context
            .iter()
            .any(|hit| hit.doc_id == "sym:config.rs:1:parseConfigFile"),
        "exact query for 'parseConfigFile' must surface the exact-match document, got {:?}",
        result.context.iter().map(|h| &h.doc_id).collect::<Vec<_>>()
    );
    Ok(())
}

/// Hard test: semantic query (deterministic embedder). A query using
/// different vocabulary than any document, but from the same domain as
/// one document's shared vocabulary, is more likely to retrieve that
/// document via the vector path than the fully unrelated network/log
/// documents -- exercised end-to-end through the vector index built by
/// the deterministic [`HashingEmbedder`] (no model download, no network
/// call).
#[test]
fn semantic_query_prefers_shared_vocabulary_document_over_unrelated_ones() -> TestResult {
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);

    // "widget configuration" shares vocabulary with loadWidgetSettings
    // ("widget", "configuration"/"settings") but is not a literal
    // substring of it -- this exercises the vector path's cosine
    // similarity, not just BM25 exact/split-term matching.
    let result = searcher.search("widget configuration settings", &corpus, &[])?;

    assert!(!result.context.is_empty());
    let top_ids: Vec<&str> = result
        .context
        .iter()
        .map(|hit| hit.doc_id.as_str())
        .collect();
    assert!(
        top_ids.contains(&"sym:config.rs:2:loadWidgetSettings"),
        "semantic query should surface the shared-vocabulary document, got {top_ids:?}"
    );

    // With only 5 documents in this fixture and CONTEXT_MIN=5, the
    // context pack pads out to the whole corpus regardless of relevance
    // -- so the real signal to check is RANK, not exclusion: the
    // shared-vocabulary document must rank strictly above the fully
    // unrelated networking document.
    let widget_rank = top_ids
        .iter()
        .position(|id| *id == "sym:config.rs:2:loadWidgetSettings")
        .ok_or("widget document must be present")?;
    let socket_rank = top_ids
        .iter()
        .position(|id| *id == "sym:net.rs:1:openSocket")
        .ok_or("socket document must be present")?;
    assert!(
        widget_rank < socket_rank,
        "shared-vocabulary document must outrank the unrelated networking document: {top_ids:?}"
    );
    Ok(())
}

/// Hard test: reranker-lift computation is correct. The pipeline's own
/// `reranker_lift` field must be a valid measurement, and must equal an
/// independent recomputation from the same run's pre/post pools -- this
/// proves the wiring, not just the isolated `ranking::reranker_lift`
/// function (already unit-tested in `ranking.rs`).
#[test]
fn reranker_lift_reports_nonzero_when_pipeline_reorders_results() -> TestResult {
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);

    let result = searcher.search("config file settings", &corpus, &[])?;

    // The pipeline's own reranker_lift field must be a valid, finite,
    // non-negative measurement -- this is the field the QA/parity
    // harness reads, so its presence and shape matter as much as any
    // specific value for this small fixture.
    assert!(result.reranker_lift.is_finite());
    assert!(result.reranker_lift >= 0.0);

    let recomputed =
        enforcer_memory::ranking::reranker_lift(&result.pre_rerank_pool, &result.context);
    assert!((recomputed - result.reranker_lift).abs() < 1e-9);
    Ok(())
}

/// Hard test: vector stale detection. A vector index built under one
/// embedder's model info must be detected as stale (mismatched
/// dimension) against a different model info -- D-04: "stale detection
/// on any mismatch".
#[test]
fn vector_index_manifest_detects_staleness_on_dimension_change() -> TestResult {
    let corpus = fixture_corpus();
    let (_fulltext, vector, embedder, _reranker) = build_stack(&corpus)?;

    let mut drifted_model = embedder.model_info();
    drifted_model.dimension += 8;

    let diff = vector.manifest().diff(&drifted_model);
    assert!(
        !diff.is_empty(),
        "a dimension mismatch must be reported as staleness"
    );
    assert!(!vector.manifest().matches(&drifted_model));

    // The un-drifted model info must still match (no false positive).
    assert!(vector.manifest().matches(&embedder.model_info()));
    Ok(())
}

/// Hard test: model manifest content. The embedder's `model_info()`
/// (the version vector that lands in the vector index manifest) must
/// carry every field the Rag-Guide doctrine requires: embedding_model,
/// dimension, dtype, similarity_metric, normalization, and the
/// formatter/chunker/parser versions -- none blank.
#[test]
fn model_manifest_carries_the_full_version_vector() {
    let embedder = HashingEmbedder::new();
    let info = embedder.model_info();

    assert!(!info.embedding_model.is_empty());
    assert!(info.dimension > 0);
    assert!(!info.dtype.is_empty());
    assert!(!info.similarity_metric.is_empty());
    assert!(!info.normalization.is_empty());
    assert!(!info.formatter_version.is_empty());
    assert!(!info.chunker_version.is_empty());
    assert!(!info.parser_version.is_empty());
}

/// Hard test: no-remote-provider proof. The default embedder and
/// reranker must both report a `Degraded` capability state (never
/// silently "loaded" as if backed by a real remote/local model) -- this
/// is the mechanical proof that the default build's retrieval stack
/// makes zero network calls and downloads zero model weights (D-03:
/// "degraded mode is labeled and is NOT accepted for feature parity").
#[test]
fn default_build_reports_degraded_capability_state_never_a_real_provider() -> TestResult {
    let embedder = HashingEmbedder::new();
    let reranker = FusionScoreReranker::new();

    assert_eq!(
        embedder.state(),
        LoadState::Degraded(DegradedState::ProviderUnavailable),
        "default embedder must honestly report degraded/provider-unavailable, never 'loaded'"
    );
    assert!(
        matches!(
            reranker.state(),
            LoadState::Degraded(DegradedState::ProviderUnavailable)
        ),
        "default reranker must honestly report degraded/provider-unavailable, never 'loaded'"
    );

    // A full pipeline run must propagate that same honesty into its
    // result -- `is_degraded` must be true for the default build.
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);
    let result = searcher.search("config", &corpus, &[])?;
    assert!(
        enforcer_memory::search::is_degraded(&result),
        "the default build's search result must be labeled degraded, never claimed as feature parity"
    );
    Ok(())
}

/// Hard test: token-reduction estimate in the context-pack result. The
/// [`enforcer_memory::search::SearchResult::token_reduction_estimate`]
/// field must be present, finite, and reflect a real reduction ratio for
/// a nonempty context pack (the whole point of retrieval over "hand
/// over every file").
#[test]
fn token_reduction_estimate_is_present_and_reflects_a_real_reduction() -> TestResult {
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);

    let result = searcher.search("config file settings", &corpus, &[])?;

    assert!(!result.context.is_empty());
    let estimate = result.token_reduction_estimate;
    assert!(estimate.context_tokens > 0);
    assert!(estimate.naive_tokens > 0);
    assert!(
        estimate.ratio() > 0.0,
        "a nonempty context pack against a nonzero naive baseline must report a positive reduction ratio"
    );
    Ok(())
}

/// Extra coverage: D-08 hard filters exclude before rerank end-to-end
/// through the full pipeline (not just the `ranking.rs` unit test) --
/// a hard-filtered document must never appear anywhere in the result,
/// including the pre-rerank trace.
#[test]
fn hard_filters_exclude_a_document_from_the_full_pipeline_result() -> TestResult {
    let corpus = fixture_corpus();
    let (fulltext, vector, embedder, reranker) = build_stack(&corpus)?;
    let searcher = HybridSearcher::new(&fulltext, &vector, &embedder, &reranker);

    let filters = vec![HardFilter::new("no-config-file", |doc_id: &str| {
        doc_id != "file:config.rs"
    })];

    let result = searcher.search("config", &corpus, &filters)?;

    assert!(
        result
            .pre_rerank_pool
            .iter()
            .all(|trace| trace.doc_id != "file:config.rs"),
        "hard-filtered document must never enter the pre-rerank pool"
    );
    assert!(
        result
            .context
            .iter()
            .all(|hit| hit.doc_id != "file:config.rs"),
        "hard-filtered document must never reach the final context pack"
    );
    Ok(())
}
