//! X06.4 embedding layer: the `Embedder` trait plus capability-state
//! types adopted from OcentraParent's runtime-status contract shapes
//! (`LoadState`, `ResourceClass`, `DegradedState` -- BORROW_POLICY §2:
//! "contracts only", re-typed here as enforcer-native Rust types, no
//! OcentraParent code exists to copy).
//!
//! # D-03 (DEFAULT): backend-neutral local runtime; deterministic
//! default ships in this slice
//!
//! D-03 makes llama.cpp/GGUF the first-class local runtime shape,
//! keeps ONNX Runtime (`ort`) as an optional backend behind
//! `ort-models`, and requires zero-network default behavior. What
//! ships here unconditionally is [`Embedder`], the [`HashingEmbedder`]
//! deterministic default implementation, and the capability-state types
//! every implementation (real or default) reports through so a caller
//! can always tell whether a result came from a real model or a
//! degraded stand-in.
//!
//! [`HashingEmbedder`] is a deterministic hashing-projection embedder:
//! it tokenizes text with [`crate::fulltext::tokenize`] (so it shares
//! the crate's code-aware camelCase/snake_case splitting) and projects
//! each term into a fixed-width vector via a stable hash, summed and
//! L2-normalized. It is NOT a semantic model -- it reports
//! `LoadState::Degraded(DegradedState::ProviderUnavailable)` on every
//! call, honestly, per OWNER_INTENT's "degraded mode is labeled and NOT
//! accepted for feature parity". It still gives deterministic, testable
//! "semantic-ish" behavior (near-duplicate/shared-vocabulary queries
//! land close in cosine space) good enough to exercise the fusion/
//! rerank/HNSW machinery end-to-end with zero network and zero model
//! weights.

use crate::error::Result;
use crate::fulltext::tokenize;

/// Embedding dimension the default hashing embedder produces. Any real
/// real embedder reports its own model's native dimension via
/// [`EmbeddingModelInfo::dimension`]; callers must never assume a fixed
/// dimension across implementations -- the vector index's manifest (see
/// [`crate::vector`]) records the dimension that was actually used to
/// build it and rejects mismatched queries.
pub const HASHING_EMBEDDER_DIMENSION: usize = 64;

use enforcer_domain::memory_types::{
    ComplexitySourceBytes, DegradedState, EmbeddingChunkerVersion, EmbeddingCosineSimilarity,
    EmbeddingDimension, EmbeddingDtype, EmbeddingFormatterVersion, EmbeddingModelName,
    EmbeddingNormalization, EmbeddingParserVersion, EmbeddingSimilarityMetric,
    EmbeddingTermProjection, EmbeddingVector, LoadState, MemoryFullTextInput, MemoryFullTextToken,
    ParserSourceText, ResourceClass,
};

/// Static description of the embedding model an [`Embedder`]
/// implementation reports -- this is the "version vector" half that
/// lands in the vector index manifest (D-04: manifests carry the full
/// version vector) so stale-detection can fire on ANY of these changing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingModelInfo {
    pub embedding_model: EmbeddingModelName,
    pub dimension: EmbeddingDimension,
    pub dtype: EmbeddingDtype,
    pub similarity_metric: EmbeddingSimilarityMetric,
    pub normalization: EmbeddingNormalization,
    pub formatter_version: EmbeddingFormatterVersion,
    pub chunker_version: EmbeddingChunkerVersion,
    pub parser_version: EmbeddingParserVersion,
}

/// One embedding vector plus the model info it was produced under, so
/// callers never have to separately track "which model made this
/// vector" -- it always travels with the vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    pub vector: EmbeddingVector,
    pub model: EmbeddingModelInfo,
}

/// The embedding capability seam. `enforcer-memory`'s default build
/// ships only [`HashingEmbedder`] (see module docs); real local
/// implementations satisfy this same trait without changing any caller.
pub trait Embedder: Send + Sync {
    /// Embed `text` into this embedder's vector space.
    fn embed(&self, text: ParserSourceText<'_>) -> Result<EmbeddingVector>;

    /// Static model info this embedder reports -- part of the vector
    /// index manifest's version vector (D-04).
    fn model_info(&self) -> EmbeddingModelInfo;

    /// Current capability state. Called once per query by
    /// [`crate::search::HybridSearcher`] so a result can honestly report
    /// whether it ran degraded (OWNER_INTENT: never silently upgraded).
    fn state(&self) -> LoadState;

    /// Hardware class this embedder is running/would run on.
    fn resource_class(&self) -> ResourceClass {
        ResourceClass::Cpu
    }
}

/// The deterministic, zero-network, zero-model-download default
/// embedder (see module docs). Always reports
/// `LoadState::Degraded(DegradedState::ProviderUnavailable)` -- it is a
/// stand-in for a real semantic model, never claimed as one.
#[derive(Debug, Default, Clone, Copy)]
pub struct HashingEmbedder;

impl HashingEmbedder {
    pub fn new() -> Self {
        Self
    }

    /// Stable (non-random, cross-process-reproducible) hash of a term
    /// into `0..HASHING_EMBEDDER_DIMENSION`, plus a `+1.0`/`-1.0` sign
    /// bit derived from a second hash bit -- the classic hashing-trick
    /// feature projection, deterministic across runs and platforms
    /// (FNV-1a, not `std`'s randomized `DefaultHasher`).
    fn project_term(term: &MemoryFullTextToken) -> EmbeddingTermProjection {
        fnv1a(ComplexitySourceBytes::from(term.as_str().as_bytes()))
    }
}

/// Stable FNV-1a projection for one token.
fn fnv1a(bytes: ComplexitySourceBytes<'_>) -> EmbeddingTermProjection {
    let digest = {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0100_0000_01b3;
        let mut hash = OFFSET_BASIS;
        for &byte in bytes.as_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
        hash
    };
    let dimension = u64::try_from(HASHING_EMBEDDER_DIMENSION).unwrap_or(u64::MAX);
    let index = usize::try_from(digest % dimension).map_or(0, |value| value);
    let sign = if digest & 1 == 0 { 1.0 } else { -1.0 };
    EmbeddingTermProjection {
        index: index.into(),
        sign: sign.into(),
    }
}

impl Embedder for HashingEmbedder {
    fn embed(&self, text: ParserSourceText<'_>) -> Result<EmbeddingVector> {
        let mut vector = vec![0.0f32; HASHING_EMBEDDER_DIMENSION];
        for term in tokenize(&MemoryFullTextInput::from(text.as_str())) {
            let projection = Self::project_term(&term);
            if let Some(slot) = vector.get_mut(projection.index.get()) {
                *slot += projection.sign.get();
            }
        }
        let vector = EmbeddingVector::from(vector);
        Ok(l2_normalize(&vector))
    }

    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            embedding_model: "enforcer-hashing-projection-v1".into(),
            dimension: HASHING_EMBEDDER_DIMENSION.into(),
            dtype: "f32".into(),
            similarity_metric: "cosine".into(),
            normalization: "l2".into(),
            formatter_version: "1".into(),
            chunker_version: "1".into(),
            parser_version: "1".into(),
        }
    }

    fn state(&self) -> LoadState {
        LoadState::Degraded(DegradedState::ProviderUnavailable)
    }
}

/// Runtime-selectable local embedder.
///
/// This is the handoff point from "retrieval always uses the degraded
/// hashing stand-in" to "retrieval can be given a real local provider
/// when a validated model spec/cache exists", without changing the
/// [`Embedder`] trait or violating zero-network defaults.
pub enum LocalEmbedder {
    Hashing(HashingEmbedder),
    #[cfg(feature = "ort-models")]
    Ort(Box<crate::ort_runtime::real::OrtEmbedder>),
}

impl std::fmt::Debug for LocalEmbedder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hashing(embedder) => formatter.debug_tuple("Hashing").field(embedder).finish(),
            #[cfg(feature = "ort-models")]
            Self::Ort(_) => formatter.debug_tuple("Ort").field(&"OrtEmbedder").finish(),
        }
    }
}

impl LocalEmbedder {
    pub fn hashing() -> Self {
        Self::Hashing(HashingEmbedder::new())
    }

    #[cfg(feature = "ort-models")]
    pub fn try_ort(
        spec: &crate::model_runtime::ModelSpecDto,
        provider: enforcer_domain::memory_types::ProviderKind,
    ) -> Result<Self> {
        Ok(Self::Ort(Box::new(
            crate::ort_runtime::real::OrtEmbedder::load(spec, provider)?,
        )))
    }

    #[cfg(not(feature = "ort-models"))]
    pub fn try_ort(
        _spec: &crate::model_runtime::ModelSpecDto,
        _provider: enforcer_domain::memory_types::ProviderKind,
    ) -> Result<Self> {
        Err(crate::error::MemoryError::ModelRuntime {
            operation: "load-local-ort-embedder".into(),
            reason: "ort-models feature is not compiled; default retrieval remains degraded/provider-unavailable".into(),
        })
    }
}

impl Default for LocalEmbedder {
    fn default() -> Self {
        Self::hashing()
    }
}

impl Embedder for LocalEmbedder {
    fn embed(&self, text: ParserSourceText<'_>) -> Result<EmbeddingVector> {
        match self {
            Self::Hashing(embedder) => embedder.embed(text),
            #[cfg(feature = "ort-models")]
            Self::Ort(embedder) => embedder.embed(text),
        }
    }

    fn model_info(&self) -> EmbeddingModelInfo {
        match self {
            Self::Hashing(embedder) => embedder.model_info(),
            #[cfg(feature = "ort-models")]
            Self::Ort(embedder) => embedder.model_info(),
        }
    }

    fn state(&self) -> LoadState {
        match self {
            Self::Hashing(embedder) => embedder.state(),
            #[cfg(feature = "ort-models")]
            Self::Ort(embedder) => embedder.state(),
        }
    }

    fn resource_class(&self) -> ResourceClass {
        match self {
            Self::Hashing(embedder) => embedder.resource_class(),
            #[cfg(feature = "ort-models")]
            Self::Ort(embedder) => embedder.resource_class(),
        }
    }
}

/// In-place L2 normalization; a zero vector is left as-is (no NaN from
/// dividing by zero norm).
fn l2_normalize(vector: &EmbeddingVector) -> EmbeddingVector {
    let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    let mut normalized = vector.as_slice().to_vec();
    if norm > 0.0 {
        for value in &mut normalized {
            *value /= norm;
        }
    }
    normalized.into()
}

/// Cosine similarity between two equal-length vectors. Returns `0.0` for
/// mismatched lengths or a zero-norm vector rather than panicking --
/// callers (fusion/HNSW candidate scoring) treat that as "no signal",
/// never a crash.
pub fn cosine_similarity(a: &EmbeddingVector, b: &EmbeddingVector) -> EmbeddingCosineSimilarity {
    let a = a.as_ref();
    let b = b.as_ref();
    if a.len() != b.len() || a.is_empty() {
        return 0.0.into();
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0.into()
    } else {
        (dot / (norm_a * norm_b)).into()
    }
}
