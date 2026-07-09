//! X06.4 vector index (D-04 DEFAULT): owned in-process cosine index
//! (pure Rust, no C++ toolchain, no external vector service).
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
    entries: Vec<(String, Vec<f32>)>,
}

impl VectorIndex {
    /// Build a fresh index over `entries` (`doc_id -> embedding vector`),
    /// tagged with `model` as this index's version-vector manifest.
    /// Rebuilding is always correct and cheap (D-02: "indexes are
    /// disposable") -- there is no incremental-update API in this slice.
    pub fn build(entries: &[(String, Vec<f32>)], model: EmbeddingModelInfo) -> Self {
        Self {
            manifest: VectorManifest::new(model),
            entries: entries.to_vec(),
        }
    }

    pub fn manifest(&self) -> &VectorManifest {
        &self.manifest
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Nearest-neighbor search: the top `limit` documents by cosine
    /// similarity to `query_vector`, scored so "higher is better" (this
    /// crate's shared convention, matching
    /// [`crate::fulltext::FullTextIndex::search`]). The current proof
    /// corpora are small enough that exact search is preferable to a
    /// transitive ANN dependency with unmaintained serialization baggage.
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<ScoredCandidate> {
        if self.entries.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scored: Vec<ScoredCandidate> = self
            .entries
            .iter()
            .map(|(doc_id, vector)| ScoredCandidate {
                doc_id: doc_id.clone(),
                score: cosine_similarity(query_vector, vector),
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.doc_id.cmp(&right.doc_id))
        });
        scored.truncate(limit);
        scored
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left, right) in left.iter().zip(right) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
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
