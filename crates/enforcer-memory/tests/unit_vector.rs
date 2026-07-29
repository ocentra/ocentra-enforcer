use enforcer_domain::memory_types::{ParserSourceText, VectorStaleReason};
use enforcer_domain::memory_types::{VectorIndexEntries, VectorIndexEntry};
use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::error::Result;
use enforcer_memory::vector::{embed_documents, VectorIndex, VectorManifest};

fn model_info() -> enforcer_memory::embed::EmbeddingModelInfo {
    HashingEmbedder::new().model_info()
}

#[test]
fn exact_vector_query_returns_the_matching_document_first() -> Result<()> {
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
    let index = VectorIndex::build(entries, model_info());
    let query_vec = embedder.embed(ParserSourceText::from("parse config file"))?;
    let hits = index.search(query_vec, 2);
    assert_eq!(hits[0].doc_id, "a");
    Ok(())
}

#[test]
fn empty_index_returns_no_hits() {
    let index = VectorIndex::build(&[], model_info());
    assert!(index.is_empty().is_enabled());
    let hits = index.search(&[0.0, 1.0], 5);
    assert!(hits.is_empty());
}

#[test]
fn manifest_matches_identical_model_info() {
    let manifest = VectorManifest::new(model_info());
    assert!(bool::from(manifest.matches(&model_info())));
}

#[test]
fn manifest_detects_dimension_mismatch() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.dimension += 1;
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, VectorStaleReason::Dimension { .. })));
    assert!(!bool::from(manifest.matches(&other)));
}

#[test]
fn manifest_detects_embedding_model_name_mismatch() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.embedding_model = "some-other-model".into();
    let diff = manifest.diff(&other);
    assert!(diff
        .iter()
        .any(|reason| matches!(reason, VectorStaleReason::EmbeddingModel { .. })));
}

#[test]
fn manifest_reports_every_mismatched_field_not_just_the_first() {
    let manifest = VectorManifest::new(model_info());
    let mut other = model_info();
    other.dimension += 1;
    other.dtype = "f16".into();
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
