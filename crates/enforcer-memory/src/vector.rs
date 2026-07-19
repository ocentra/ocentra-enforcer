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
use crate::owned_boundary::RetainedDisplay;
use crate::ranking::ScoredCandidate;
use enforcer_domain::memory_types::{
    EmbeddingCosineSimilarity, EmbeddingVector, SearchGraphFlag, VectorDocumentId, VectorDocuments,
    VectorIndexEntries, VectorIndexEntry, VectorManifestMatches, VectorSearchLimit,
    VectorStaleReason,
};

/// The version-vector manifest one [`VectorIndex`] was built under. Two
/// manifests are compared field-by-field, not by a single hash, so a
/// staleness report can name exactly which field changed.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorManifest {
    pub model: EmbeddingModelInfo,
}

impl VectorManifest {
    pub fn new(model: EmbeddingModelInfo) -> Self {
        Self { model }
    }

    /// Compare this manifest against `candidate`, returning every
    /// mismatched field (D-04: "stale detection on any mismatch" --
    /// report all mismatches, not just the first, so a caller rebuilding
    /// the index knows the full extent of drift).
    pub fn diff(&self, candidate: &EmbeddingModelInfo) -> Vec<VectorStaleReason> {
        let mut reasons = Vec::new();
        let expected = &self.model;
        if expected.embedding_model != candidate.embedding_model {
            reasons.push(VectorStaleReason::EmbeddingModel {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.embedding_model.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.embedding_model.retained_display(),
            });
        }
        if expected.dimension != candidate.dimension {
            reasons.push(VectorStaleReason::Dimension {
                expected: expected.dimension.get(),
                actual: candidate.dimension.get(),
            });
        }
        if expected.dtype != candidate.dtype {
            reasons.push(VectorStaleReason::Dtype {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.dtype.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.dtype.retained_display(),
            });
        }
        if expected.similarity_metric != candidate.similarity_metric {
            reasons.push(VectorStaleReason::SimilarityMetric {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.similarity_metric.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.similarity_metric.retained_display(),
            });
        }
        if expected.normalization != candidate.normalization {
            reasons.push(VectorStaleReason::Normalization {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.normalization.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.normalization.retained_display(),
            });
        }
        if expected.formatter_version != candidate.formatter_version {
            reasons.push(VectorStaleReason::FormatterVersion {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.formatter_version.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.formatter_version.retained_display(),
            });
        }
        if expected.chunker_version != candidate.chunker_version {
            reasons.push(VectorStaleReason::ChunkerVersion {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.chunker_version.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.chunker_version.retained_display(),
            });
        }
        if expected.parser_version != candidate.parser_version {
            reasons.push(VectorStaleReason::ParserVersion {
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed expected metadata.
                expected: expected.parser_version.retained_display(),
                // CLONE-JUSTIFICATION: owned stale reason outlives borrowed candidate metadata.
                actual: candidate.parser_version.retained_display(),
            });
        }
        reasons
    }

    /// `true` if `candidate`'s version vector matches this manifest in
    /// every field.
    pub fn matches(&self, candidate: &EmbeddingModelInfo) -> VectorManifestMatches {
        self.diff(candidate).is_empty().into()
    }
}

/// HNSW-backed dense vector index over a fixed set of `(doc_id,
/// embedding)` pairs, built for one embedder's version vector at a
/// time (recorded in [`VectorIndex::manifest`]).
#[derive(Debug)]
pub struct VectorIndex {
    manifest: VectorManifest,
    entries: VectorIndexEntries,
}

impl VectorIndex {
    /// Build a fresh index over `entries` (`doc_id -> embedding vector`),
    /// tagged with `model` as this index's version-vector manifest.
    /// Rebuilding is always correct and cheap (D-02: "indexes are
    /// disposable") -- there is no incremental-update API in this slice.
    pub fn build(entries: impl Into<VectorIndexEntries>, model: EmbeddingModelInfo) -> Self {
        Self {
            manifest: VectorManifest::new(model),
            entries: entries.into(),
        }
    }

    pub fn manifest(&self) -> &VectorManifest {
        &self.manifest
    }

    pub fn is_empty(&self) -> SearchGraphFlag {
        self.entries.is_empty().into()
    }

    pub fn len(&self) -> VectorSearchLimit {
        self.entries.len().into()
    }

    /// Nearest-neighbor search: the top `limit` documents by cosine
    /// similarity to `query_vector`, scored so "higher is better" (this
    /// crate's shared convention, matching
    /// [`crate::fulltext::FullTextIndex::search`]). The current proof
    /// corpora are small enough that exact search is preferable to a
    /// transitive ANN dependency with unmaintained serialization baggage.
    pub fn search(
        &self,
        query_vector: impl Into<EmbeddingVector>,
        limit: impl Into<VectorSearchLimit>,
    ) -> Vec<ScoredCandidate> {
        let query_vector = query_vector.into();
        let limit = limit.into().get();
        if self.entries.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scored: Vec<ScoredCandidate> = self
            .entries
            .iter()
            .map(|entry| ScoredCandidate {
                // CLONE-JUSTIFICATION: returned search result owns its document id beyond the index borrow.
                doc_id: entry.doc_id.as_str().into(),
                score: cosine_similarity(&query_vector, &entry.vector)
                    .as_f64()
                    .into(),
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .score
                .get()
                .total_cmp(&left.score.get())
                .then_with(|| left.doc_id.cmp(&right.doc_id))
        });
        scored.truncate(limit);
        scored
    }
}

fn cosine_similarity(left: &EmbeddingVector, right: &EmbeddingVector) -> EmbeddingCosineSimilarity {
    let left = left.as_ref();
    let right = right.as_ref();
    if left.len() != right.len() || left.is_empty() {
        return 0.0.into();
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
        return 0.0.into();
    }
    // CAST-JUSTIFICATION: cosine is accumulated in f64 for stable ranking,
    // while the shared similarity brand stores the compact f32 wire value.
    ((dot / (left_norm.sqrt() * right_norm.sqrt())) as f32).into()
}

/// Build the `(doc_id, embedding)` entries an embedder produces for a
/// document set, keeping the doc-id association explicit rather than
/// relying on iteration-order alignment.
pub fn embed_documents(
    embedder: &dyn crate::embed::Embedder,
    documents: impl Into<VectorDocuments>,
) -> Result<VectorIndexEntries> {
    let documents = documents.into();
    let mut seen: HashMap<&str, ()> = HashMap::new();
    let mut entries = VectorIndexEntries::new();
    for document in documents.iter() {
        if seen.insert(document.id.as_str(), ()).is_none() {
            // CLONE-JUSTIFICATION: returned embedded entry owns its id beyond the borrowed documents slice.
            entries.push(VectorIndexEntry {
                doc_id: VectorDocumentId::from(document.id.as_str()),
                vector: embedder.embed(enforcer_domain::memory_types::ParserSourceText::from(
                    document.text.as_str(),
                ))?,
            });
        }
    }
    Ok(entries)
}
