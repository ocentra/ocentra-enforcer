use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::error::Result;
use enforcer_memory::vector::StaleReason;
use enforcer_memory::vector::{embed_documents, VectorIndex, VectorManifest};

fn model_info() -> enforcer_memory::embed::EmbeddingModelInfo {
    HashingEmbedder::new().model_info()
}

#[test]
fn exact_vector_query_returns_the_matching_document_first() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let entries = vec![
        ("a".to_owned(), embedder.embed("parse config file")?),
        ("b".to_owned(), embedder.embed("write log entry")?),
    ];
    let index = VectorIndex::build(&entries, model_info());
    let query_vec = embedder.embed("parse config file")?;
    let hits = index.search(&query_vec, 2);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].doc_id, "a");
    Ok(())
}

#[test]
fn empty_index_returns_no_hits() {
    let index = VectorIndex::build(&[], model_info());
    assert!(index.is_empty());
    let hits = index.search(&[0.0, 1.0], 5);
    assert!(hits.is_empty());
}

#[test]
fn manifest_matches_identical_model_info() {
    let manifest = VectorManifest::new(model_info());
    assert!(manifest.matches(&model_info()));
}

#[test]
fn manifest_detects_dimension_mismatch() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.dimension += 1;
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, StaleReason::Dimension { .. })));
    assert!(!manifest.matches(&other));
}

#[test]
fn manifest_detects_embedding_model_name_mismatch() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.embedding_model = "some-other-model".to_owned();
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, StaleReason::EmbeddingModel { .. })));
}

#[test]
fn manifest_reports_every_mismatched_field_not_just_the_first() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.dimension += 1;
    other.dtype = "f16".to_owned();
    let diff = manifest.diff(&other);
    assert!(diff.len() >= 2);
}

#[test]
fn embed_documents_dedups_repeated_doc_ids() -> Result<()> {
    let embedder = HashingEmbedder::new();
    let docs = vec![
        ("a".to_owned(), "first".to_owned()),
        ("a".to_owned(), "second".to_owned()),
    ];
    let entries = embed_documents(&embedder, &docs)?;
    assert_eq!(entries.len(), 1);
    Ok(())
}
