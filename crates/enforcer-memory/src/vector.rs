//! X06.4 vector index (D-04 DEFAULT): HNSW via `hnsw_rs` (pure Rust, no
//! C++ toolchain, no external vector service -- harvested dependency
//! choice per BORROW_POLICY §2/TabAgentServer `Rust/indexing`).
//!
//! One [`VectorIndex`] instance serves either the code-chunk corpus or
//! the lessons/artifacts/summaries corpus (D-04: "one index for code
//! chunks, one for lessons/artifacts/summaries") -- this module is
//! corpus-agnostic; callers construct two separate instances.
//!
//! # Manifests and stale detection
//!
//! Every index carries the full version vector Rag-Guide doctrine
//! requires ([`crate::embed::EmbeddingModelInfo`]): embedding_model,
//! dimension, dtype, similarity_metric, normalization, and the
//! formatter/chunker/parser versions. [`VectorManifest::matches`]
//! compares a manifest against the embedder that is about to be used
//! for a query; ANY field mismatch is staleness (D-04/Rag-Guide:
//! "changing ANY invalidates" -- never partial-trust a mismatched
//! index).

use std::collections::HashMap;

use hnsw_rs::anndists::dist::DistCosine;
use hnsw_rs::hnsw::{Hnsw, Neighbour};

use crate::embed::EmbeddingModelInfo;
use crate::error::Result;
use crate::ranking::ScoredCandidate;

/// The version-vector manifest one [`VectorIndex`] was built under. Two
/// manifests are compared field-by-field, not by a single hash, so a
/// staleness report can name exactly which field changed.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorManifest {
    pub model: EmbeddingModelInfo,
}

/// Why a vector index is considered stale relative to a candidate
/// embedder/model-info -- named per mismatched field so callers/
/// diagnostics can report precisely, never just "stale".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    EmbeddingModel { expected: String, actual: String },
    Dimension { expected: usize, actual: usize },
    Dtype { expected: String, actual: String },
    SimilarityMetric { expected: String, actual: String },
    Normalization { expected: String, actual: String },
    FormatterVersion { expected: String, actual: String },
    ChunkerVersion { expected: String, actual: String },
    ParserVersion { expected: String, actual: String },
}

impl VectorManifest {
    pub fn new(model: EmbeddingModelInfo) -> Self {
        Self { model }
    }

    /// Compare this manifest against `candidate`, returning every
    /// mismatched field (D-04: "stale detection on any mismatch" --
    /// report all mismatches, not just the first, so a caller rebuilding
    /// the index knows the full extent of drift).
    pub fn diff(&self, candidate: &EmbeddingModelInfo) -> Vec<StaleReason> {
        let mut reasons = Vec::new();
        let expected = &self.model;
        if expected.embedding_model != candidate.embedding_model {
            reasons.push(StaleReason::EmbeddingModel {
                expected: expected.embedding_model.clone(),
                actual: candidate.embedding_model.clone(),
            });
        }
        if expected.dimension != candidate.dimension {
            reasons.push(StaleReason::Dimension {
                expected: expected.dimension,
                actual: candidate.dimension,
            });
        }
        if expected.dtype != candidate.dtype {
            reasons.push(StaleReason::Dtype {
                expected: expected.dtype.clone(),
                actual: candidate.dtype.clone(),
            });
        }
        if expected.similarity_metric != candidate.similarity_metric {
            reasons.push(StaleReason::SimilarityMetric {
                expected: expected.similarity_metric.clone(),
                actual: candidate.similarity_metric.clone(),
            });
        }
        if expected.normalization != candidate.normalization {
            reasons.push(StaleReason::Normalization {
                expected: expected.normalization.clone(),
                actual: candidate.normalization.clone(),
            });
        }
        if expected.formatter_version != candidate.formatter_version {
            reasons.push(StaleReason::FormatterVersion {
                expected: expected.formatter_version.clone(),
                actual: candidate.formatter_version.clone(),
            });
        }
        if expected.chunker_version != candidate.chunker_version {
            reasons.push(StaleReason::ChunkerVersion {
                expected: expected.chunker_version.clone(),
                actual: candidate.chunker_version.clone(),
            });
        }
        if expected.parser_version != candidate.parser_version {
            reasons.push(StaleReason::ParserVersion {
                expected: expected.parser_version.clone(),
                actual: candidate.parser_version.clone(),
            });
        }
        reasons
    }

    /// `true` if `candidate`'s version vector matches this manifest in
    /// every field.
    pub fn matches(&self, candidate: &EmbeddingModelInfo) -> bool {
        self.diff(candidate).is_empty()
    }
}

/// HNSW-backed dense vector index over a fixed set of `(doc_id,
/// embedding)` pairs, built for one embedder's version vector at a
/// time (recorded in [`VectorIndex::manifest`]).
pub struct VectorIndex {
    manifest: VectorManifest,
    // `hnsw_rs` indexes by a caller-assigned `usize` id; `ids` maps that
    // back to the caller's stable string doc id.
    ids: Vec<String>,
    hnsw: Hnsw<'static, f32, DistCosine>,
}

impl VectorIndex {
    /// Build a fresh index over `entries` (`doc_id -> embedding vector`),
    /// tagged with `model` as this index's version-vector manifest.
    /// Rebuilding is always correct and cheap (D-02: "indexes are
    /// disposable") -- there is no incremental-update API in this slice.
    pub fn build(entries: &[(String, Vec<f32>)], model: EmbeddingModelInfo) -> Self {
        let max_elements = entries.len().max(1);
        // hnsw_rs constructor: (max_nb_connection, max_elements, max_layer, ef_construction, dist).
        let hnsw = Hnsw::<f32, DistCosine>::new(16, max_elements, 16, 200, DistCosine {});
        let mut ids = Vec::with_capacity(entries.len());
        for (index, (doc_id, vector)) in entries.iter().enumerate() {
            hnsw.insert((vector.as_slice(), index));
            ids.push(doc_id.clone());
        }
        Self {
            manifest: VectorManifest::new(model),
            ids,
            hnsw,
        }
    }

    pub fn manifest(&self) -> &VectorManifest {
        &self.manifest
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Approximate nearest-neighbor search: the top `limit` documents by
    /// cosine similarity to `query_vector`, scored so "higher is better"
    /// (this crate's shared convention, matching
    /// [`crate::fulltext::FullTextIndex::search`]).
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<ScoredCandidate> {
        if self.ids.is_empty() || limit == 0 {
            return Vec::new();
        }
        let ef_search = (limit * 4).max(32);
        let neighbours: Vec<Neighbour> = self.hnsw.search(query_vector, limit, ef_search);
        neighbours
            .into_iter()
            .filter_map(|neighbour| {
                let doc_id = self.ids.get(neighbour.d_id)?.clone();
                // hnsw_rs reports distance (lower = closer); `DistCosine`
                // is `1 - cosine_similarity`, so invert back to a
                // "higher is better" similarity score.
                let score = 1.0 - f64::from(neighbour.distance);
                Some(ScoredCandidate { doc_id, score })
            })
            .collect()
    }
}

/// Build the `(doc_id, embedding)` entries an embedder produces for a
/// document set, keeping the doc-id association explicit rather than
/// relying on iteration-order alignment.
pub fn embed_documents(
    embedder: &dyn crate::embed::Embedder,
    documents: &[(String, String)],
) -> Result<Vec<(String, Vec<f32>)>> {
    let mut seen: HashMap<&str, ()> = HashMap::new();
    let mut entries = Vec::new();
    for (doc_id, text) in documents {
        if seen.insert(doc_id.as_str(), ()).is_none() {
            entries.push((doc_id.clone(), embedder.embed(text)?));
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{Embedder, HashingEmbedder};

    fn model_info() -> EmbeddingModelInfo {
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
}
