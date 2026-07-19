//! Canonical values used by the durable memory store.
//!
//! These types deliberately live in the dependency-leaf domain crate: an
//! artifact identity and a project-store identity are passed across memory,
//! install, report, and transport boundaries and must not acquire a
//! crate-local duplicate.

use crate::boundary::decode_error::DecodeError;
use crate::hashes::{Sha256, SHA256_PREFIX};
use crate::paths::{RelPath, RepoRoot};

macro_rules! memory_text_target {
    () => {
        str
    };
}

macro_rules! memory_bytes_target {
    () => {
        [u8]
    };
}

macro_rules! memory_vector_target {
    () => {
        [f32]
    };
}

macro_rules! transparent_memory_wire {
    ($type:ty, $raw:ty) => {
        impl serde::Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serde::Serialize::serialize(&self.0, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = <$raw as serde::Deserialize>::deserialize(deserializer)?;
                Ok(Self::from(raw))
            }
        }
    };
}

/// A content-addressed artifact identity: the SHA-256 of its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for ArtifactId."]
pub struct ArtifactId(Sha256);

impl ArtifactId {
    /// Derive a content-addressed identity from the exact artifact bytes.
    pub fn from_content(content: &[u8]) -> Self {
        Self(crate::boundary::hash::validate(content))
    }

    /// Wrap a validated digest without asserting that the content exists.
    pub fn from_digest(digest: Sha256) -> Self {
        Self(digest)
    }

    /// View the complete prefixed SHA-256 identity.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Access the validated digest backing this artifact identity.
    pub fn digest(&self) -> &Sha256 {
        &self.0
    }
}

impl From<Sha256> for ArtifactId {
    fn from(value: Sha256) -> Self {
        Self::from_digest(value)
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

transparent_memory_wire!(ArtifactId, Sha256);

/// Deterministic filesystem-safe key for one normalized repository store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for ProjectId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct ProjectId(String);

impl ProjectId {
    /// Decode an existing store-directory identity, rejecting invalid length or hex text.
    pub fn try_new(value: String) -> Option<Self> {
        (value.len() == 16
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then_some(Self(value))
    }

    /// Derive the store identity from a repository root.
    pub fn from_repo_root(root: &RepoRoot) -> Self {
        let digest = crate::boundary::hash::validate(root.as_str().as_bytes());
        let hex = digest
            .as_str()
            .strip_prefix(SHA256_PREFIX)
            .unwrap_or(digest.as_str());
        // ALLOC-JUSTIFICATION: ProjectId owns the stable, fixed-width digest prefix.
        Self(hex.get(..16).unwrap_or(hex).to_owned())
    }

    /// View the filesystem-safe project key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validated Hugging Face repository coordinate (`owner/model`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for HfRepositoryId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct HfRepositoryId(String);

impl HfRepositoryId {
    /// Construct a repository coordinate only when it has one safe owner/model separator.
    pub fn try_new(value: String) -> Option<Self> {
        let parts: Vec<_> = value.split('/').collect();
        let valid = parts.len() == 2
            && parts.iter().all(|part| !part.is_empty())
            && !value.contains("..")
            && !value.contains("//")
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
        valid.then_some(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HfRepositoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validated repository-relative Hugging Face artifact path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for HfFilePath."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct HfFilePath(String);

impl HfFilePath {
    /// Construct a safe relative artifact path.
    pub fn try_new(value: String) -> Option<Self> {
        let valid = !value.trim().is_empty()
            && !value.contains("..")
            && !value.contains('\0')
            && !value.starts_with('/')
            && !value.starts_with('\\')
            && !value.contains("//")
            && !value.contains("\\\\");
        valid.then_some(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HfFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validated model revision label used in a Hugging Face download request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for HfRevision."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct HfRevision(String);

impl HfRevision {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Option<Self> {
        (!value.trim().is_empty() && !value.contains("..")).then_some(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HfRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable model selection identity within a Hugging Face runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for HfModelId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct HfModelId(String);

impl HfModelId {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Option<Self> {
        (!value.trim().is_empty()).then_some(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HfModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for RiskLevel."]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for ImpactScope."]
pub enum ImpactScope {
    #[default]
    All,
    SymbolsOnly,
    RoutesOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for DetectChangesScope."]
pub enum DetectChangesScope {
    Symbols,
    Impact,
    FilesOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LessonStatus."]
pub enum LessonStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for StreamingArtifactStatus."]
pub enum StreamingArtifactStatus {
    Ready,
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingArtifactKey."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct StreamingArtifactKey(String);

impl std::fmt::Debug for StreamingArtifactKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StreamingArtifactKey([REDACTED])")
    }
}

impl StreamingArtifactKey {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Option<Self> {
        (!value.trim().is_empty()).then_some(Self(value))
    }
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingRelativePath."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct StreamingRelativePath(String);

impl StreamingRelativePath {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Option<Self> {
        (!value.trim().is_empty()).then_some(Self(value))
    }
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingCacheSchemaVersion."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct StreamingCacheSchemaVersion(u32);

impl StreamingCacheSchemaVersion {
    /// Initial persisted cache schema.
    pub const INITIAL: Self = Self(1);

    /// Brand an already validated positive cache schema version.
    pub const fn try_new(value: std::num::NonZeroU32) -> Self {
        Self(value.get())
    }
}

impl From<StreamingCacheSchemaVersion> for u32 {
    fn from(value: StreamingCacheSchemaVersion) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingChunkCount."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct StreamingChunkCount(u64);

impl StreamingChunkCount {
    /// Empty chunk count.
    pub const ZERO: Self = Self(0);

    /// Brand an already validated positive chunk count.
    pub const fn try_new(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }

    /// Convert a zero-based chunk index into the inclusive chunk count.
    pub const fn from_last_index(index: StreamingChunkIndex) -> Self {
        Self(index.0 + 1)
    }
}

impl From<StreamingChunkCount> for u64 {
    fn from(value: StreamingChunkCount) -> Self {
        value.0
    }
}

/// Zero-based position of one chunk inside a streaming artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingChunkIndex."]
#[doc = "BRAND-INVARIANT: the value is a zero-based position owned by one streaming cache reader or writer."]
pub struct StreamingChunkIndex(u64);

impl StreamingChunkIndex {
    /// First chunk in an artifact.
    pub const ZERO: Self = Self(0);

    /// Advance to the next contiguous chunk.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl From<StreamingChunkIndex> for u64 {
    fn from(value: StreamingChunkIndex) -> Self {
        value.0
    }
}

impl std::fmt::Display for StreamingChunkIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Outcome of advancing a streaming cache reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingChunkAdvance {
    Opened,
    Exhausted,
}

/// Maximum number of enrichment tasks allowed to execute concurrently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for WorkerConcurrency."]
#[doc = "BRAND-INVARIANT: the value is always non-zero; raw storage remains private."]
pub struct WorkerConcurrency(std::num::NonZeroUsize);

impl WorkerConcurrency {
    /// One worker, used by deterministic single-flight execution and tests.
    pub const SINGLE: Self = Self(std::num::NonZeroUsize::MIN);

    /// Brand a validated non-zero concurrency limit.
    pub const fn from_nonzero(value: std::num::NonZeroUsize) -> Self {
        Self(value)
    }
}

impl From<WorkerConcurrency> for usize {
    fn from(value: WorkerConcurrency) -> Self {
        value.0.get()
    }
}

/// Number of attempts already made for one enrichment task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for RetryAttemptCount."]
#[doc = "BRAND-INVARIANT: the value is a monotonic task-local attempt count; raw storage remains private."]
pub struct RetryAttemptCount(u32);

impl RetryAttemptCount {
    /// Fresh task that has not yet been attempted.
    pub const ZERO: Self = Self(0);

    /// Default bounded retry budget.
    pub const DEFAULT_LIMIT: Self = Self(3);

    /// High attempt count used to verify that exponential backoff remains capped.
    pub const BACKOFF_SATURATION_PROBE: Self = Self(30);

    /// Advance after one completed attempt.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// Previous attempt count, if this is not the fresh-task state.
    pub const fn previous(self) -> Option<Self> {
        match self.0.checked_sub(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl From<RetryAttemptCount> for u32 {
    fn from(value: RetryAttemptCount) -> Self {
        value.0
    }
}

/// Borrowed repository-relative path used for parser classification and dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: parser dispatch borrows one repository-relative path without retaining or normalizing its storage."]
pub struct ParserRelativePath<'a>(&'a str);

impl<'a> ParserRelativePath<'a> {
    /// Return the borrowed repository-relative path.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ParserRelativePath<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a String> for ParserRelativePath<'a> {
    fn from(value: &'a String) -> Self {
        Self(value.as_str())
    }
}

/// Borrowed source text consumed by one parser dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: parser dispatch borrows source text for the duration of one parse and never retains it."]
pub struct ParserSourceText<'a>(&'a str);

impl<'a> ParserSourceText<'a> {
    /// Return the borrowed source text.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ParserSourceText<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a String> for ParserSourceText<'a> {
    fn from(value: &'a String) -> Self {
        Self(value.as_str())
    }
}

/// Borrowed filename suffixes used to classify C-family test files.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: parser classification borrows one fixed suffix vocabulary for the duration of a dispatch."]
pub struct ParserTestSuffixes<'a>(&'a [&'a str]);

impl<'a> ParserTestSuffixes<'a> {
    /// View the borrowed test-file suffix vocabulary.
    pub const fn as_slice(self) -> &'a [&'a str] {
        self.0
    }
}

impl<'a> From<&'a [&'a str]> for ParserTestSuffixes<'a> {
    fn from(value: &'a [&'a str]) -> Self {
        Self(value)
    }
}

/// One-based source position carried by code-graph nodes and edges.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphSourceLine."]
#[doc = "BRAND-INVARIANT: the value preserves the parser or projection source position exactly; raw storage remains private."]
pub struct GraphSourceLine(usize);

impl GraphSourceLine {
    /// Unknown or unavailable source position at a projection boundary.
    pub const UNKNOWN: Self = Self(0);

    /// Return the exact stored source position.
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for GraphSourceLine {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GraphSourceLine> for usize {
    fn from(value: GraphSourceLine) -> Self {
        value.0
    }
}

impl PartialEq<usize> for GraphSourceLine {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<usize> for GraphSourceLine {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::fmt::Display for GraphSourceLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Number of commits that changed one indexed file.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphChangeCount."]
#[doc = "BRAND-INVARIANT: the value is the exact non-negative git-history count for one indexed path."]
pub struct GraphChangeCount(usize);

impl GraphChangeCount {
    /// No observed changes.
    pub const ZERO: Self = Self(0);

    /// Return the exact observed change count.
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for GraphChangeCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GraphChangeCount> for usize {
    fn from(value: GraphChangeCount) -> Self {
        value.0
    }
}

/// Number of nodes in one graph snapshot.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphNodeCount."]
#[doc = "BRAND-INVARIANT: the value is the exact number of serialized graph nodes."]
pub struct GraphNodeCount(usize);

impl GraphNodeCount {
    /// Brand an exact graph-node count; every usize value is valid.
    pub const fn try_new(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for GraphNodeCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GraphNodeCount> for usize {
    fn from(value: GraphNodeCount) -> Self {
        value.0
    }
}

/// Number of edges in one graph snapshot.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphEdgeCount."]
#[doc = "BRAND-INVARIANT: the value is the exact number of serialized graph edges."]
pub struct GraphEdgeCount(usize);

impl GraphEdgeCount {
    /// Brand an exact graph-edge count; every usize value is valid.
    pub const fn try_new(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for GraphEdgeCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GraphEdgeCount> for usize {
    fn from(value: GraphEdgeCount) -> Self {
        value.0
    }
}

/// Byte length of an exported graph artifact.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphArtifactByteCount."]
#[doc = "BRAND-INVARIANT: the value is an exact serialized or compressed artifact length."]
pub struct GraphArtifactByteCount(u64);

impl GraphArtifactByteCount {
    /// Brand an exact artifact byte count; every u64 value is valid.
    pub const fn try_new(value: u64) -> Self {
        Self(value)
    }
}

impl From<u64> for GraphArtifactByteCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<GraphArtifactByteCount> for u64 {
    fn from(value: GraphArtifactByteCount) -> Self {
        value.0
    }
}

impl TryFrom<usize> for GraphArtifactByteCount {
    type Error = std::num::TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value.try_into().map(Self)
    }
}

/// Version of the durable graph-artifact metadata schema.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphArtifactSchemaVersion."]
#[doc = "BRAND-INVARIANT: preserves the exact unsigned wire version so a boundary can reject unsupported graph schemas."]
pub struct GraphArtifactSchemaVersion(u32);

impl GraphArtifactSchemaVersion {
    /// Current graph artifact metadata schema.
    pub const CURRENT: Self = Self(2);
}

impl From<u32> for GraphArtifactSchemaVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<GraphArtifactSchemaVersion> for u32 {
    fn from(value: GraphArtifactSchemaVersion) -> Self {
        value.0
    }
}

impl std::fmt::Display for GraphArtifactSchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Compression level recorded for a graph artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphCompressionLevel."]
#[doc = "BRAND-INVARIANT: preserves the exact signed zstd level recorded with persisted graph bytes."]
pub struct GraphCompressionLevel(i32);

impl GraphCompressionLevel {
    /// Fast, moderate compression used by graph exports.
    pub const FAST: Self = Self(3);
}

impl From<i32> for GraphCompressionLevel {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl From<GraphCompressionLevel> for i32 {
    fn from(value: GraphCompressionLevel) -> Self {
        value.0
    }
}

/// Whether a persisted graph file has only text-level indexing.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphTextOnly."]
#[doc = "BRAND-INVARIANT: the boolean wire value has one named graph-file meaning."]
pub struct GraphTextOnly(bool);

impl GraphTextOnly {
    /// Structurally indexed source file.
    pub const STRUCTURED: Self = Self(false);

    /// File retained with text-only indexing.
    pub const TEXT_ONLY: Self = Self(true);

    /// Whether this file is text-only.
    pub const fn is_text_only(self) -> bool {
        self.0
    }
}

/// Shingle width used by one persisted source-body fingerprint.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for GraphShingleSize."]
#[doc = "BRAND-INVARIANT: the value records the exact shingle width selected by fingerprint generation."]
pub struct GraphShingleSize(usize);

impl GraphShingleSize {
    /// Brand an exact shingle width; every usize value is valid.
    pub const fn try_new(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for GraphShingleSize {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GraphShingleSize> for usize {
    fn from(value: GraphShingleSize) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for StreamingByteCount."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct StreamingByteCount(u64);

impl StreamingByteCount {
    /// Empty byte count.
    pub const ZERO: Self = Self(0);

    /// Brand an already validated positive byte count.
    pub const fn try_new(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }
}

impl From<u64> for StreamingByteCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<StreamingByteCount> for u64 {
    fn from(value: StreamingByteCount) -> Self {
        value.0
    }
}

/// Whether an artifact should use the chunked streaming-cache representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingChunkDecision {
    Direct,
    Chunked,
}

impl StreamingChunkDecision {
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Chunked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for FreshnessState."]
pub enum FreshnessState {
    NoIndexBuilt,
    Fresh { built_at: String, watermark: u64 },
    Stale { watermark: u64, log_length: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProjectStatus."]
pub enum ProjectStatus {
    Ready,
    Empty,
}

/// Stable taxonomy for model-runtime evidence recorded by Memory.
///
/// SERIALIZATION-DOC: values use kebab-case on the durable observation and
/// proof wires; variant spellings are part of that persisted contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for ModelRuntimeObservationKind."]
pub enum ModelRuntimeObservationKind {
    ModelLoadFailure,
    ProviderDowngrade,
    ArtifactHashMismatch,
    TokenizerHashMismatch,
    DegradedFallback,
    SuccessfulLocalLoad,
    RetrievalQualityProof,
    RerankerLiftProof,
    TokenReductionProof,
    RouteChoiceImprovement,
    RecurrenceOrNegativeEvidence,
}

impl ModelRuntimeObservationKind {
    /// Return the stable durable-wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelLoadFailure => "model-load-failure",
            Self::ProviderDowngrade => "provider-downgrade",
            Self::ArtifactHashMismatch => "artifact-hash-mismatch",
            Self::TokenizerHashMismatch => "tokenizer-hash-mismatch",
            Self::DegradedFallback => "degraded-fallback",
            Self::SuccessfulLocalLoad => "successful-local-load",
            Self::RetrievalQualityProof => "retrieval-quality-proof",
            Self::RerankerLiftProof => "reranker-lift-proof",
            Self::TokenReductionProof => "token-reduction-proof",
            Self::RouteChoiceImprovement => "route-choice-improvement",
            Self::RecurrenceOrNegativeEvidence => "recurrence-or-negative-evidence",
        }
    }
}

/// Result-shape mode for repository code search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for CodeSearchMode."]
pub enum CodeSearchMode {
    Compact,
    Full,
    Files,
}

/// Retrieval engine selected by graph search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for GraphSearchMode."]
pub enum GraphSearchMode {
    #[default]
    Bm25,
    Regex,
}

/// Monotonic, gap-free position assigned by one append-only memory log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for Seq."]
#[doc = "BRAND-INVARIANT: the append-only log owns assignment; callers only carry the typed position."]
pub struct Seq(u64);

impl Seq {
    /// The first position in an empty append-only log.
    pub const GENESIS: Self = Self(0);

    /// Advance to the next gap-free position.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// Carry a position assigned by the append-only log into domain code.
    pub const fn from_log_position(position: u64) -> Self {
        Self(position)
    }

    /// Restore a non-empty log length read from the filesystem boundary.
    pub const fn from_nonzero(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }
}

impl From<Seq> for u64 {
    fn from(value: Seq) -> Self {
        value.0
    }
}

impl From<u64> for Seq {
    fn from(value: u64) -> Self {
        Self::from_log_position(value)
    }
}

impl std::fmt::Display for Seq {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

transparent_memory_wire!(Seq, u64);

/// Persisted wire-schema version for the append-only Memory logs.
/// SERIALIZATION-DOC: this value is serialized only by the Memory log persistence boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: this is the schema version selected by the Memory persistence boundary."]
pub struct MemoryLogSchemaVersionWire(u32);

/// Domain name for the wire-owned Memory log schema version.
pub type MemoryLogSchemaVersion = MemoryLogSchemaVersionWire;

impl MemoryLogSchemaVersion {
    /// The initial, currently supported Memory log schema.
    pub const INITIAL: Self = Self(1);

    /// Creates a supported Memory log schema version.
    pub fn try_new(value: u32) -> Result<Self, DecodeError> {
        if value == Self::INITIAL.0 {
            Ok(Self(value))
        } else {
            Err(DecodeError::new(
                "memoryLogSchemaVersion",
                "expected the currently supported schema version",
            ))
        }
    }
}

impl From<MemoryLogSchemaVersion> for u32 {
    fn from(value: MemoryLogSchemaVersion) -> Self {
        value.0
    }
}

impl TryFrom<u32> for MemoryLogSchemaVersion {
    type Error = DecodeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<'de> serde::Deserialize<'de> for MemoryLogSchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

macro_rules! validated_memory_text {
    ($(#[$doc:meta])* $name:ident, $field:literal, $valid:expr, $reason:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            #[doc = "The new operation for this canonical domain value."]
            pub fn new(value: String) -> Result<Self, DecodeError> {
                if value.trim().is_empty()
                    || value.chars().any(char::is_control)
                    || !($valid)(&value)
                {
                    return Err(DecodeError::new($field, $reason));
                }
                Ok(Self(value))
            }

            #[doc = "The as_str operation for this canonical domain value."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
                Self::new(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_memory_text!(
    #[doc = "Stable identity of one code-graph symbol node."]
    SymbolNodeId,
    "symbolNodeId",
    |value: &str| value.starts_with("sym:") && value.len() > 4,
    "must be printable text beginning with `sym:`"
);
validated_memory_text!(
    #[doc = "Stable identity retained for one indexed graph chunk."]
    ChunkId,
    "chunkId",
    |value: &str| value.starts_with("sym:") || value.starts_with("chunk:"),
    "must be a printable `sym:` or `chunk:` identity"
);
validated_memory_text!(
    #[doc = "Stable identity of one node in a persisted graph snapshot."]
    GraphSnapshotNodeId,
    "graphSnapshotNodeId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Repository-relative path retained in a persisted graph snapshot."]
    GraphSnapshotRelativePath,
    "graphSnapshotRelativePath",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Stable identity of one chunk retained in a persisted graph snapshot."]
    GraphSnapshotChunkId,
    "graphSnapshotChunkId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Optional source commit reference retained in a persisted graph snapshot."]
    GraphSnapshotCommit,
    "graphSnapshotCommit",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Symbol name retained in a persisted graph snapshot."]
    GraphSnapshotSymbolName,
    "graphSnapshotSymbolName",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Source fingerprint retained in a persisted graph snapshot."]
    GraphSnapshotFingerprint,
    "graphSnapshotFingerprint",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Source-body gram retained in a persisted graph snapshot."]
    GraphSnapshotBodyGram,
    "graphSnapshotBodyGram",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Imported module path retained in a persisted graph snapshot."]
    GraphSnapshotModulePath,
    "graphSnapshotModulePath",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Call target retained in a persisted graph snapshot."]
    GraphSnapshotCallee,
    "graphSnapshotCallee",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "HTTP method retained in a persisted route snapshot."]
    GraphSnapshotRouteMethod,
    "graphSnapshotRouteMethod",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Route path retained in a persisted graph snapshot."]
    GraphSnapshotRoutePath,
    "graphSnapshotRoutePath",
    |_: &str| true,
    "must be non-empty printable text"
);

macro_rules! owned_memory_text {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Default,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[doc = "BRAND-INVARIANT: the producing Memory analysis owns the semantic meaning; raw storage remains private."]
        pub struct $name(String);

        impl $name {
            /// View the analysis-owned text.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Whether the analysis-owned text is empty.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            /// Remove all analysis-owned text while retaining the semantic brand.
            pub fn clear(&mut self) {
                self.0.clear();
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                // ALLOC-JUSTIFICATION: the canonical brand owns text beyond the borrowed boundary input.
                Self(value.to_owned())
            }
        }

        impl From<&String> for $name {
            fn from(value: &String) -> Self {
                Self::from(value.as_str())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::ops::Deref for $name {
            type Target = memory_text_target!();

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                self.0 == *other
            }
        }
    };
}

owned_memory_text!(
    #[doc = "Named architecture section, layer, or hotspot."]
    ArchitectureName
);
owned_memory_text!(
    #[doc = "Repository-relative path in an architecture report."]
    ArchitectureReportPath
);

/// Borrowed repository-relative path used while building an architecture report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "BRAND-INVARIANT: architecture traversal borrows slash-normalized graph paths without retaining them."]
pub struct ArchitecturePath<'a>(&'a str);

impl<'a> ArchitecturePath<'a> {
    /// Return the borrowed repository-relative path.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ArchitecturePath<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a String> for ArchitecturePath<'a> {
    fn from(value: &'a String) -> Self {
        Self(value.as_str())
    }
}

/// Match decision returned by architecture path predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: the architecture traversal owns this path-match decision; raw storage remains private."]
pub struct ArchitecturePathMatch(bool);

impl ArchitecturePathMatch {
    /// Brand the explicit architecture path-match decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for ArchitecturePathMatch {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ArchitecturePathMatch> for bool {
    fn from(value: ArchitecturePathMatch) -> Self {
        value.0
    }
}

/// Presence decision returned by complexity grammar capability checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: the complexity grammar table owns this capability decision; raw storage remains private."]
pub struct ComplexityNodeKindPresence(bool);

impl ComplexityNodeKindPresence {
    /// Brand the explicit complexity-node capability decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for ComplexityNodeKindPresence {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ComplexityNodeKindPresence> for bool {
    fn from(value: ComplexityNodeKindPresence) -> Self {
        value.0
    }
}

/// Maximum hotspot rows retained by an architecture report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact zero-inclusive count of hotspot rows retained; every usize value is meaningful."]
pub struct ArchitectureHotspotLimit(usize);

impl ArchitectureHotspotLimit {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for ArchitectureHotspotLimit {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Maximum clustering iterations allowed while building an architecture report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact zero-inclusive iteration count; zero explicitly disables iterative refinement."]
pub struct ArchitectureMaxIterations(usize);

impl ArchitectureMaxIterations {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for ArchitectureMaxIterations {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Number of transient embedding failures injected before success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative failure count; every usize value is meaningful."]
pub struct EnrichmentFailureCount(usize);

impl EnrichmentFailureCount {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for EnrichmentFailureCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Number of enrichment attempts observed by a deterministic test embedder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative attempt count; every usize value is meaningful."]
pub struct EnrichmentAttemptCount(usize);

impl EnrichmentAttemptCount {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for EnrichmentAttemptCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Vector slot selected by the deterministic hashing projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is a zero-based projection index whose upper bound is owned by the embedding vector boundary."]
pub struct EmbeddingProjectionIndex(usize);

impl EmbeddingProjectionIndex {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for EmbeddingProjectionIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Signed contribution selected by the deterministic hashing projection.
#[derive(Debug, Clone, Copy, PartialEq)]
#[doc = "BRAND-INVARIANT: this preserves the signed projection contribution produced by the embedding boundary."]
pub struct EmbeddingProjectionSign(f32);

impl EmbeddingProjectionSign {
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl From<f32> for EmbeddingProjectionSign {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

/// Named hashing projection result for one normalized term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmbeddingTermProjection {
    pub index: EmbeddingProjectionIndex,
    pub sign: EmbeddingProjectionSign,
}

/// Cosine similarity produced by the embedding boundary.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[doc = "BRAND-INVARIANT: this preserves the signed cosine result produced by the normalized embedding boundary."]
pub struct EmbeddingCosineSimilarity(f32);

impl EmbeddingCosineSimilarity {
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Return the similarity as the widened scalar required by ranking APIs.
    pub fn as_f64(self) -> f64 {
        self.get().into()
    }
}

impl From<f32> for EmbeddingCosineSimilarity {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl PartialEq<f32> for EmbeddingCosineSimilarity {
    fn eq(&self, other: &f32) -> bool {
        self.0 == *other
    }
}

impl std::fmt::Display for EmbeddingCosineSimilarity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
owned_memory_text!(
    #[doc = "HTTP method in an architecture route report."]
    ArchitectureRouteMethod
);
owned_memory_text!(
    #[doc = "Human-readable basis for one architecture classification."]
    ArchitectureReason
);
owned_memory_text!(
    #[doc = "Stable cluster identity in an architecture report."]
    ArchitectureClusterId
);
owned_memory_text!(
    #[doc = "Stable graph-node identity in an architecture report."]
    ArchitectureNodeId
);
owned_memory_text!(
    #[doc = "Stable node identity carried by the append-only Memory graph log."]
    GraphEventNodeId
);
owned_memory_text!(
    #[doc = "Canonical node-kind label carried by the append-only Memory graph log."]
    GraphEventNodeKind
);
owned_memory_text!(
    #[doc = "Canonical edge label carried by the append-only Memory graph log."]
    GraphEventEdgeLabel
);
owned_memory_text!(
    #[doc = "Stable identity of one background Memory enrichment task."]
    MemoryTaskKey
);
owned_memory_text!(
    #[doc = "Human-readable negative evidence attached to a learned Memory lesson."]
    MemoryRecurrenceNegativeReason
);
owned_memory_text!(
    #[doc = "Language label in an architecture report."]
    ArchitectureLanguage
);
owned_memory_text!(
    #[doc = "Stable node identity in complexity propagation."]
    ComplexityNodeId
);

/// Canonical set of tree-sitter node-kind names used by complexity analysis.
#[derive(Debug, Clone, Copy, Default)]
#[doc = "BRAND-INVARIANT: language adapters select immutable grammar node kinds; raw storage remains private."]
pub struct MemoryAstNodeKindSet(&'static [&'static str]);

/// One parser-owned tree-sitter node-kind name inspected by complexity analysis.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: the borrowed text comes directly from the active parser grammar and cannot outlive that input."]
pub struct MemoryAstNodeKind<'a>(&'a str);

impl<'a> From<&'a str> for MemoryAstNodeKind<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl MemoryAstNodeKindSet {
    /// Construct one immutable grammar node-kind set.
    pub const fn from_static(values: &'static [&'static str]) -> Self {
        Self(values)
    }

    /// Whether this set contains a grammar node kind.
    pub fn has_node_kind(self, value: MemoryAstNodeKind<'_>) -> bool {
        self.0.contains(&value.0)
    }

    /// Whether this grammar node-kind set is empty.
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

/// Canonical tree-sitter child-field name used by complexity analysis.
#[derive(Debug, Clone, Copy, Default)]
#[doc = "BRAND-INVARIANT: language adapters select immutable grammar field names; raw storage remains private."]
pub struct MemoryAstFieldName(&'static str);

impl MemoryAstFieldName {
    /// Construct one immutable grammar child-field name.
    pub const fn from_static(value: &'static str) -> Self {
        Self(value)
    }

    /// Return the grammar child-field name.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Borrowed source bytes inspected by one complexity-analysis pass.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: complexity analysis borrows parser source without copying or retaining it."]
pub struct ComplexitySourceBytes<'a>(&'a [u8]);

impl<'a> ComplexitySourceBytes<'a> {
    /// Return the borrowed parser source bytes.
    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

impl<'a> From<&'a [u8]> for ComplexitySourceBytes<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(value)
    }
}

/// Borrowed source text inspected by one complexity-analysis pass.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: complexity analysis borrows parser source without copying or retaining it."]
pub struct ComplexitySourceText<'a>(&'a str);

impl<'a> ComplexitySourceText<'a> {
    /// Return the borrowed parser source text.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ComplexitySourceText<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

/// Stable parser symbol location used as a complexity-metric key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComplexitySymbolLocation {
    pub name: ParsedSymbolName,
    pub line: GraphSourceLine,
}

impl ComplexitySymbolLocation {
    /// Construct a complexity symbol location from parser-owned parts.
    pub fn new(name: impl Into<ParsedSymbolName>, line: impl Into<GraphSourceLine>) -> Self {
        Self {
            name: name.into(),
            line: line.into(),
        }
    }
}

/// One function node supplied to transitive complexity propagation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityCallGraphNode {
    pub id: ComplexityNodeId,
    pub loop_depth: ComplexityMeasure,
    pub self_recursive: ComplexitySignal,
    pub callees: Vec<ComplexityNodeId>,
}

/// Borrowed call-graph input supplied to transitive complexity propagation.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: complexity propagation borrows a complete typed call-graph snapshot without retaining it."]
pub struct ComplexityCallGraph<'a>(&'a [ComplexityCallGraphNode]);

impl<'a> ComplexityCallGraph<'a> {
    /// Borrow one typed call-graph snapshot.
    pub const fn new(nodes: &'a [ComplexityCallGraphNode]) -> Self {
        Self(nodes)
    }

    /// Return the typed call-graph nodes.
    pub const fn nodes(self) -> &'a [ComplexityCallGraphNode] {
        self.0
    }
}

/// Transitive complexity metrics propagated across one call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComplexityTransitiveMetrics {
    pub transitive_loop_depth: ComplexityMeasure,
    pub recursive: ComplexitySignal,
}

/// Canonical node-keyed result of transitive complexity propagation.
#[derive(Debug, Clone, Default)]
pub struct ComplexityPropagation(
    std::collections::HashMap<ComplexityNodeId, ComplexityTransitiveMetrics>,
);

impl ComplexityPropagation {
    /// Look up propagated metrics by stable complexity node id.
    pub fn get(&self, id: &str) -> Option<&ComplexityTransitiveMetrics> {
        self.0.get(id)
    }
}

impl FromIterator<(ComplexityNodeId, ComplexityTransitiveMetrics)> for ComplexityPropagation {
    fn from_iter<T: IntoIterator<Item = (ComplexityNodeId, ComplexityTransitiveMetrics)>>(
        iter: T,
    ) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for ComplexityPropagation {
    type Item = (ComplexityNodeId, ComplexityTransitiveMetrics);
    type IntoIter =
        std::collections::hash_map::IntoIter<ComplexityNodeId, ComplexityTransitiveMetrics>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Index<&str> for ComplexityPropagation {
    type Output = ComplexityTransitiveMetrics;

    fn index(&self, index: &str) -> &Self::Output {
        std::ops::Index::index(&self.0, index)
    }
}
owned_memory_text!(
    #[doc = "Stable identity of one procedural-memory record."]
    ProceduralRecordId
);
owned_memory_text!(
    #[doc = "Lesson identity referenced by a procedural-memory record."]
    ProceduralLessonReference
);
owned_memory_text!(
    #[doc = "Detail recorded for one procedural-memory outcome."]
    ProceduralDetail
);
owned_memory_text!(
    #[doc = "Timestamp retained by a procedural-memory or route-trace record."]
    MemoryObservationTimestamp
);
owned_memory_text!(
    #[doc = "Stable identity assigned to one durable Memory log entry."]
    MemoryLogEntryId
);
owned_memory_text!(
    #[doc = "Runtime source that emitted one model observation."]
    ModelRuntimeObservationSource
);
owned_memory_text!(
    #[doc = "Runtime execution identity attached to one model observation."]
    ModelRuntimeObservationRunId
);
owned_memory_text!(
    #[doc = "Project identity persisted in a Memory store marker."]
    MemoryStoreMarkerProjectId
);
owned_memory_text!(
    #[doc = "Stable identity of one retrieval route trace."]
    RouteTraceId
);
owned_memory_text!(
    #[doc = "Query text retained by one retrieval route trace."]
    RouteTraceQuery
);
owned_memory_text!(
    #[doc = "Retrieval route selected for one query."]
    RetrievalRoute
);
owned_memory_text!(
    #[doc = "Searchable text projected from one procedural-memory or route-trace record."]
    MemoryObservationSearchText
);
owned_memory_text!(
    #[doc = "Stable umbrella identity projected from one Memory graph node."]
    MemoryGraphNodeId
);
owned_memory_text!(
    #[doc = "Searchable text projected from one Memory graph node."]
    MemoryGraphSearchText
);
owned_memory_text!(
    #[doc = "Digest text retained by a typed Memory integrity error."]
    MemoryErrorDigest
);
owned_memory_text!(
    #[doc = "Artifact identity retained by a typed Memory integrity error."]
    MemoryErrorArtifactId
);
owned_memory_text!(
    #[doc = "Operation name retained by a typed Memory runtime or invariant error."]
    MemoryErrorOperation
);
owned_memory_text!(
    #[doc = "Failure reason retained by a typed Memory runtime or invariant error."]
    MemoryErrorReason
);
owned_memory_text!(
    #[doc = "Full-text query supplied to repository graph search."]
    SearchGraphQuery
);
owned_memory_text!(
    #[doc = "Name, qualified-name, or file pattern supplied to repository graph search."]
    SearchGraphPattern
);
owned_memory_text!(
    #[doc = "Relationship kind selected for repository graph traversal."]
    SearchGraphRelationship
);
owned_memory_text!(
    #[doc = "Stable node identity returned by repository graph search."]
    SearchGraphNodeId
);
owned_memory_text!(
    #[doc = "Node name returned by repository graph search."]
    SearchGraphNodeName
);
owned_memory_text!(
    #[doc = "Qualified node name returned by repository graph search."]
    SearchGraphQualifiedName
);
owned_memory_text!(
    #[doc = "Repository-relative file path returned by repository graph search."]
    SearchGraphFilePath
);
owned_memory_text!(
    #[doc = "Project name attached to cross-repository protocol evidence."]
    CrossRepoProjectName
);
owned_memory_text!(
    #[doc = "Source file identity attached to cross-repository protocol evidence."]
    CrossRepoSourceFileId
);
owned_memory_text!(
    #[doc = "HTTP method attached to cross-repository route evidence."]
    CrossRepoMethod
);
owned_memory_text!(
    #[doc = "HTTP route path attached to cross-repository route evidence."]
    CrossRepoPath
);
owned_memory_text!(
    #[doc = "Topic or protocol operation key attached to cross-repository evidence."]
    CrossRepoOperationKey
);
owned_memory_text!(
    #[doc = "Repository-relative path carried by an impact-analysis report."]
    ImpactPath
);
owned_memory_text!(
    #[doc = "Stable graph-node identity carried by an impact-analysis report."]
    ImpactNodeId
);
owned_memory_text!(
    #[doc = "Symbol name carried by the baseline-shaped change-detection view."]
    ImpactSymbolName
);
owned_memory_text!(
    #[doc = "Symbol label carried by the baseline-shaped change-detection view."]
    ImpactSymbolLabel
);
owned_memory_text!(
    #[doc = "Repository-relative path carried by graph-augmented code search."]
    CodeSearchPath
);
owned_memory_text!(
    #[doc = "Matched or contextual source line carried by graph-augmented code search."]
    CodeSearchText
);
owned_memory_text!(
    #[doc = "Containing symbol name carried by graph-augmented code search."]
    CodeSearchSymbolName
);
owned_memory_text!(
    #[doc = "Reason an indexed file could not be read during code search."]
    CodeSearchUnreadableReason
);
owned_memory_text!(
    #[doc = "Model identifier recorded by a local Memory model-cache manifest."]
    ModelCacheModelId
);
owned_memory_text!(
    #[doc = "Model revision recorded by a local Memory model-cache manifest."]
    ModelCacheRevision
);
owned_memory_text!(
    #[doc = "Repository-relative artifact path recorded by a local Memory model-cache manifest."]
    ModelCacheArtifactPath
);
owned_memory_text!(
    #[doc = "Expected SHA-256 digest recorded by a local Memory model-cache manifest."]
    ModelCacheArtifactSha256
);
owned_memory_text!(
    #[doc = "Optional streaming-manifest path recorded by a local Memory model-cache manifest."]
    ModelCacheStreamingManifestPath
);
owned_memory_text!(
    #[doc = "Artifact reference retained by a corrupted Memory cache status."]
    ModelCacheArtifactRef
);
owned_memory_text!(
    #[doc = "Manifest reference retained by a corrupted Memory cache status."]
    ModelCacheManifestRef
);
owned_memory_text!(
    #[doc = "Status-check timestamp retained by a corrupted Memory cache status."]
    ModelCacheCheckedAt
);
owned_memory_text!(
    #[doc = "Stable identifier assigned to a detected Memory graph cluster."]
    MemoryClusterId
);
owned_memory_text!(
    #[doc = "Graph-node identifier retained as a detected cluster member."]
    MemoryClusterNodeId
);
owned_memory_text!(
    #[doc = "File-node identifier retained as a detected cluster member."]
    MemoryClusterFileId
);
owned_memory_text!(
    #[doc = "Symbol-node identifier retained as a detected cluster member."]
    MemoryClusterSymbolId
);
owned_memory_text!(
    #[doc = "Tool name selected by a Memory CLI invocation."]
    MemoryCliToolName
);
owned_memory_text!(
    #[doc = "JSON argument object carried by a Memory CLI invocation."]
    MemoryCliArgsJson
);
owned_memory_text!(
    #[doc = "Serialized MCP-compatible result envelope returned by a Memory CLI invocation."]
    MemoryCliEnvelopeJson
);
owned_memory_text!(
    #[doc = "First textual content item decoded from a Memory CLI result envelope."]
    MemoryCliEnvelopeText
);
owned_memory_text!(
    #[doc = "Canonical JSON object key derived from a Memory CLI flag name."]
    MemoryCliFlagKey
);
owned_memory_text!(
    #[doc = "Raw scalar value supplied to a Memory CLI flag before JSON coercion."]
    MemoryCliFlagValue
);
owned_memory_text!(
    #[doc = "Standard-output text produced by a Memory CLI invocation."]
    MemoryCliStdout
);
owned_memory_text!(
    #[doc = "Standard-error text produced by a Memory CLI invocation."]
    MemoryCliStderr
);
owned_memory_text!(
    #[doc = "One command-line argument token decoded by the Memory CLI boundary."]
    MemoryCliArgument
);

/// Command-line argument tokens supplied after the Memory CLI subcommand.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryCliArguments(Vec<MemoryCliArgument>);

impl MemoryCliArguments {
    pub fn as_slice(&self) -> &[MemoryCliArgument] {
        &self.0
    }
}

impl From<Vec<MemoryCliArgument>> for MemoryCliArguments {
    fn from(value: Vec<MemoryCliArgument>) -> Self {
        Self(value)
    }
}

impl From<&MemoryCliArguments> for MemoryCliArguments {
    fn from(value: &MemoryCliArguments) -> Self {
        Self(value.as_slice().to_vec())
    }
}

impl IntoIterator for MemoryCliArguments {
    type Item = MemoryCliArgument;
    type IntoIter = std::vec::IntoIter<MemoryCliArgument>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a MemoryCliArguments {
    type Item = &'a MemoryCliArgument;
    type IntoIter = std::slice::Iter<'a, MemoryCliArgument>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
owned_memory_text!(
    #[doc = "Node-label name returned by Memory graph-schema introspection."]
    GraphSchemaLabel
);
owned_memory_text!(
    #[doc = "Edge-type name returned by Memory graph-schema introspection."]
    GraphSchemaEdgeType
);
owned_memory_text!(
    #[doc = "Property name returned by Memory graph-schema introspection."]
    GraphSchemaProperty
);
owned_memory_text!(
    #[doc = "Graph-node identifier returned by core Memory graph analysis."]
    MemoryAnalysisNodeId
);
owned_memory_text!(
    #[doc = "Last processing error retained by the Memory ingestion queue."]
    MemoryQueueLastError
);

/// Number of nodes currently stored by one in-process Memory graph.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: the graph owns this exact non-negative node count; zero is a valid empty graph."]
pub struct MemoryGraphNodeCount(usize);

impl MemoryGraphNodeCount {
    /// The count of an empty graph.
    pub const ZERO: Self = Self(0);

    /// Return the exact graph node count at a numeric boundary.
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for MemoryGraphNodeCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<MemoryGraphNodeCount> for usize {
    fn from(value: MemoryGraphNodeCount) -> Self {
        value.0
    }
}

impl PartialEq<usize> for MemoryGraphNodeCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

/// Explicit empty/non-empty state of one in-process Memory graph.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: graph storage computes this state from its owned node collection."]
pub struct MemoryGraphEmpty(bool);

impl MemoryGraphEmpty {
    /// Whether the graph contains no nodes.
    pub const fn is_empty(self) -> bool {
        self.0
    }
}

impl From<bool> for MemoryGraphEmpty {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<MemoryGraphEmpty> for bool {
    fn from(value: MemoryGraphEmpty) -> Self {
        value.0
    }
}

impl PartialEq<bool> for MemoryGraphEmpty {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

/// Whether one Memory retry policy has exhausted its allowed attempts.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: retry policy computes this state from typed attempt and limit values."]
pub struct MemoryQueueExhausted(bool);

impl MemoryQueueExhausted {
    /// Whether no further retry is permitted.
    pub const fn is_exhausted(self) -> bool {
        self.0
    }
}

impl From<bool> for MemoryQueueExhausted {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<MemoryQueueExhausted> for bool {
    fn from(value: MemoryQueueExhausted) -> Self {
        value.0
    }
}

impl PartialEq<bool> for MemoryQueueExhausted {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

/// Number of entries retained by one Memory dead-letter queue.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: the dead-letter queue owns this exact non-negative entry count."]
pub struct MemoryQueueLength(usize);

impl MemoryQueueLength {
    /// The count of an empty dead-letter queue.
    pub const ZERO: Self = Self(0);

    /// Return the exact retained-entry count at a numeric boundary.
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for MemoryQueueLength {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<MemoryQueueLength> for usize {
    fn from(value: MemoryQueueLength) -> Self {
        value.0
    }
}

impl PartialEq<usize> for MemoryQueueLength {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

/// Explicit empty/non-empty state of one Memory dead-letter queue.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: queue storage computes this state from its owned entry collection."]
pub struct MemoryQueueEmpty(bool);

impl MemoryQueueEmpty {
    /// Whether the dead-letter queue contains no entries.
    pub const fn is_empty(self) -> bool {
        self.0
    }
}

impl From<bool> for MemoryQueueEmpty {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<MemoryQueueEmpty> for bool {
    fn from(value: MemoryQueueEmpty) -> Self {
        value.0
    }
}

impl PartialEq<bool> for MemoryQueueEmpty {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

/// Bounded delay applied before retrying one failed Memory enrichment task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: the queue policy constructs non-negative delays and caps every scaled value at its configured maximum."]
pub struct MemoryRetryDelay(std::time::Duration);

/// Zero-inclusive factor applied to a branded retry delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact zero-inclusive retry multiplier; every u32 value has defined saturating behavior."]
pub struct MemoryRetryMultiplier(u32);

impl MemoryRetryMultiplier {
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for MemoryRetryMultiplier {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl MemoryRetryDelay {
    /// Brand a millisecond delay at the domain boundary.
    pub const fn from_millis(milliseconds: u64) -> Self {
        Self(std::time::Duration::from_millis(milliseconds))
    }

    /// Brand a second delay at the domain boundary.
    pub const fn from_secs(seconds: u64) -> Self {
        Self(std::time::Duration::from_secs(seconds))
    }

    /// View the underlying scheduler duration at the async runtime boundary.
    pub const fn get(self) -> std::time::Duration {
        self.0
    }

    /// Scale this delay while preserving `Duration`'s saturation behavior.
    pub fn saturating_mul(self, multiplier: MemoryRetryMultiplier) -> Self {
        Self(self.0.saturating_mul(multiplier.get()))
    }

    /// Select the smaller of two branded retry delays.
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl From<std::time::Duration> for MemoryRetryDelay {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

impl From<MemoryRetryDelay> for std::time::Duration {
    fn from(value: MemoryRetryDelay) -> Self {
        value.0
    }
}

impl PartialEq<std::time::Duration> for MemoryRetryDelay {
    fn eq(&self, other: &std::time::Duration) -> bool {
        self.0 == *other
    }
}
owned_memory_text!(
    #[doc = "Normalized query token explaining one Memory recall match."]
    MemoryRecallMatchedToken
);
owned_memory_text!(
    #[doc = "Local-first query text evaluated by Memory recall."]
    MemoryRecallQuery
);
owned_memory_text!(
    #[doc = "Text entering or leaving one stage of the Memory community-export redaction pipeline."]
    MemoryRedactionText
);
owned_memory_text!(
    #[doc = "Repository root stripped from paths during Memory community-export redaction."]
    MemoryRedactionRepoRoot
);
owned_memory_text!(
    #[doc = "Explicit identity value removed during Memory community-export redaction."]
    MemoryRedactionIdentity
);
owned_memory_text!(
    #[doc = "Human-readable reason a Memory input row was quarantined."]
    MemoryQuarantineReason
);
owned_memory_text!(
    #[doc = "Cached file-summary text produced by Memory enrichment."]
    MemorySummaryText
);
owned_memory_text!(
    #[doc = "Repository-relative path supplied to Weaver index enrichment callbacks."]
    WeaverRelativePath
);
owned_memory_text!(
    #[doc = "Content identity supplied by a Weaver index enrichment callback."]
    WeaverContentHash
);
owned_memory_text!(
    #[doc = "Graph node identity supplied by a Weaver index enrichment callback."]
    WeaverNodeId
);

/// Node identities supplied for one path by a Weaver index enrichment callback.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeaverNodeIds(Vec<WeaverNodeId>);

impl From<Vec<WeaverNodeId>> for WeaverNodeIds {
    fn from(values: Vec<WeaverNodeId>) -> Self {
        Self(values)
    }
}

impl IntoIterator for WeaverNodeIds {
    type Item = WeaverNodeId;
    type IntoIter = std::vec::IntoIter<WeaverNodeId>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
owned_memory_text!(
    #[doc = "Repository-relative source path addressed by the Memory summary cache."]
    MemorySummaryRelativePath
);
owned_memory_text!(
    #[doc = "Graph node identity retained by the Memory summary entity-link table."]
    MemorySummaryNodeId
);
owned_memory_text!(
    #[doc = "Enclosing-symbol identifier carried by a Memory call-resolution result."]
    MemoryResolutionSourceSymbolId
);
owned_memory_text!(
    #[doc = "Candidate target-symbol identifier carried by a Memory call-resolution result."]
    MemoryResolutionCandidateId
);
owned_memory_text!(
    #[doc = "Callee text consumed by the Memory call-resolution ladder."]
    MemoryResolutionCallee
);
owned_memory_text!(
    #[doc = "Method name consumed by the Memory call-resolution ladder."]
    MemoryResolutionMethodName
);
owned_memory_text!(
    #[doc = "Type name consumed by the Memory call-resolution ladder."]
    MemoryResolutionTypeName
);
owned_memory_text!(
    #[doc = "Symbol name consumed by the Memory call-resolution registry."]
    MemoryResolutionSymbolName
);
owned_memory_text!(
    #[doc = "Import module path consumed by the Memory call-resolution registry."]
    MemoryResolutionModulePath
);
owned_memory_text!(
    #[doc = "Transport protocol name carried by a Memory diagnostic request record."]
    MemoryDiagnosticProtocol
);
owned_memory_text!(
    #[doc = "Method name carried by a Memory diagnostic request record."]
    MemoryDiagnosticMethod
);
owned_memory_text!(
    #[doc = "Tool name carried by a Memory diagnostic request record."]
    MemoryDiagnosticTool
);
owned_memory_text!(
    #[doc = "Repository-relative path carried by a Memory file-skip diagnostic."]
    MemoryDiagnosticFilePath
);
owned_memory_text!(
    #[doc = "Human-readable reason carried by a Memory file-skip diagnostic."]
    MemoryDiagnosticSkipReason
);
owned_memory_text!(
    #[doc = "General field value entering Memory diagnostic redaction."]
    MemoryDiagnosticFieldValue
);
owned_memory_text!(
    #[doc = "Free-text value entering the tighter Memory diagnostic redaction policy."]
    MemoryDiagnosticFreeText
);
owned_memory_text!(
    #[doc = "Sanitized field value emitted by Memory diagnostic redaction."]
    MemoryDiagnosticRedactedValue
);

/// Elapsed processing time recorded for one Memory diagnostic request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryDiagnosticRequestDuration(std::time::Duration);

impl MemoryDiagnosticRequestDuration {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for MemoryDiagnosticRequestDuration {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}
owned_memory_text!(
    #[doc = "Evidence source label attached to a trusted Memory record."]
    MemoryRecordEvidenceSource
);
owned_memory_text!(
    #[doc = "Writer identity attached to trusted Memory record provenance."]
    MemoryRecordWriter
);
owned_memory_text!(
    #[doc = "Session identity attached to trusted Memory record provenance."]
    MemoryRecordSessionId
);
owned_memory_text!(
    #[doc = "Model identity attached to trusted Memory record provenance."]
    MemoryRecordModel
);
owned_memory_text!(
    #[doc = "User identity attached to trusted Memory record provenance."]
    MemoryRecordUser
);
owned_memory_text!(
    #[doc = "Active lesson identifier carried by a Memory session-start recall pack."]
    MemorySessionLessonId
);
owned_memory_text!(
    #[doc = "Rendered Memory session-start recall text for hook context injection."]
    MemorySessionRecallText
);
owned_memory_text!(
    #[doc = "Landing reference carried by a Memory evidence-chain artifact."]
    MemoryEvidenceLandedAt
);
owned_memory_text!(
    #[doc = "Proof-journal reference attached to a Memory evidence-chain landing."]
    MemoryEvidenceProofRef
);
owned_memory_text!(
    #[doc = "Last commit identifier attached to Memory path history."]
    MemoryGitLastCommit
);
owned_memory_text!(
    #[doc = "Repository-relative path queried through Memory git history."]
    MemoryGitRelativePath
);
owned_memory_text!(
    #[doc = "Raw path text entering the Memory filesystem decoding boundary."]
    MemoryPathInput
);

/// Working directory retained by the Memory git metadata adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryGitWorkdir(std::path::PathBuf);

impl MemoryGitWorkdir {
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryGitWorkdir {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}
owned_memory_text!(
    #[doc = "Timestamp attached to a rejected Memory federation bundle."]
    MemoryFederationRejectedAt
);
owned_memory_text!(
    #[doc = "Repository context grouped by the Memory analytics read model."]
    MemoryAnalyticsRepoContext
);
owned_memory_text!(
    #[doc = "Stable identity of one orchestration lesson."]
    MemoryLedgerLessonId
);
owned_memory_text!(
    #[doc = "Date attached to one orchestration lesson."]
    MemoryLessonDate
);
owned_memory_text!(
    #[doc = "Observed condition recorded by one orchestration lesson."]
    MemoryLessonObserved
);
owned_memory_text!(
    #[doc = "Instruction recorded by one orchestration lesson."]
    MemoryLessonText
);
owned_memory_text!(
    #[doc = "Landing reference attached to one orchestration lesson."]
    MemoryLessonLandedAt
);
owned_memory_text!(
    #[doc = "Delivery mechanism attached to one orchestration lesson."]
    MemoryLessonShipsVia
);
owned_memory_text!(
    #[doc = "Complete markdown ledger document accepted by orchestration-lesson parsing."]
    MemoryLessonLedgerDocument
);
owned_memory_text!(
    #[doc = "One source line read from an orchestration-lesson ledger."]
    MemoryLessonLedgerLine
);
owned_memory_text!(
    #[doc = "One trimmed cell read from an orchestration-lesson ledger row."]
    MemoryLessonLedgerCell
);
owned_memory_text!(
    #[doc = "Search text produced from one orchestration lesson."]
    MemoryLessonSearchText
);
owned_memory_text!(
    #[doc = "Stable identity of one Memory architecture decision record."]
    MemoryAdrId
);
owned_memory_text!(
    #[doc = "Title of one Memory architecture decision record."]
    MemoryAdrTitle
);
owned_memory_text!(
    #[doc = "Section name within one Memory architecture decision record."]
    MemoryAdrSectionName
);
owned_memory_text!(
    #[doc = "Section body within one Memory architecture decision record."]
    MemoryAdrSectionBody
);
owned_memory_text!(
    #[doc = "Graph node identity linked to one Memory architecture decision record."]
    MemoryAdrLinkedNodeId
);
owned_memory_text!(
    #[doc = "Whole-document content returned by Memory ADR retrieval."]
    MemoryAdrDocumentContent
);
owned_memory_text!(
    #[doc = "Stable identity of one Memory search document."]
    MemorySearchDocumentId
);
owned_memory_text!(
    #[doc = "Text entering the Memory full-text tokenizer."]
    MemoryFullTextInput
);
owned_memory_text!(
    #[doc = "One normalized token emitted by the Memory full-text tokenizer."]
    MemoryFullTextToken
);
owned_memory_text!(
    #[doc = "Query text evaluated by the Memory full-text index."]
    MemoryFullTextQuery
);
owned_memory_text!(
    #[doc = "Indexable text retained by one Memory search document."]
    MemorySearchDocumentText
);
owned_memory_text!(
    #[doc = "Hybrid retrieval query evaluated by the Memory search pipeline."]
    MemorySearchQuery
);
owned_memory_text!(
    #[doc = "Human-readable snippet retained by one Memory search document."]
    MemorySearchDocumentSnippet
);
owned_memory_text!(
    #[doc = "Source path retained by one Memory search document."]
    MemorySearchDocumentSourcePath
);
owned_memory_text!(
    #[doc = "Source symbol identity at the origin of a Memory data-flow edge."]
    MemoryDataFlowSourceSymbolId
);
owned_memory_text!(
    #[doc = "Target symbol identity reached by a Memory data-flow edge."]
    MemoryDataFlowTargetSymbolId
);
owned_memory_text!(
    #[doc = "Argument expression carried by a Memory data-flow edge."]
    MemoryDataFlowArgumentExpression
);
owned_memory_text!(
    #[doc = "Normalized source hash retained by a Memory source-body fingerprint."]
    MemoryFingerprintSourceHash
);
owned_memory_text!(
    #[doc = "MinHash payload retained by a Memory source-body fingerprint."]
    MemoryFingerprintValue
);
owned_memory_text!(
    #[doc = "Normalized source-body gram retained by a Memory source-body fingerprint."]
    MemoryFingerprintBodyGram
);
owned_memory_text!(
    #[doc = "One normalized lexical unit used to build a source-body fingerprint."]
    MemoryFingerprintLexeme
);

/// Ordered normalized lexical tokens used to build a source-body fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryFingerprintLexemes(Vec<MemoryFingerprintLexeme>);

impl From<Vec<MemoryFingerprintLexeme>> for MemoryFingerprintLexemes {
    fn from(value: Vec<MemoryFingerprintLexeme>) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for MemoryFingerprintLexemes {
    type Target = [MemoryFingerprintLexeme];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
owned_memory_text!(
    #[doc = "Symbol name extracted by a language parser."]
    ParsedSymbolName
);
owned_memory_text!(
    #[doc = "HTTP method extracted by a language parser."]
    ParsedRouteMethod
);
owned_memory_text!(
    #[doc = "HTTP route path extracted by a language parser."]
    ParsedRoutePath
);
owned_memory_text!(
    #[doc = "Import module path extracted by a language parser."]
    ParsedModulePath
);
owned_memory_text!(
    #[doc = "Call callee name extracted by a language parser."]
    ParsedCallee
);
owned_memory_text!(
    #[doc = "Source expression text retained by a language parser."]
    ParsedExpressionText
);
owned_memory_text!(
    #[doc = "Type, trait, decorator, container, or member name extracted by a language parser."]
    ParsedReferenceName
);
owned_memory_text!(
    #[doc = "Stable identity assigned to an ingested incident observation."]
    IngestIncidentId
);
owned_memory_text!(
    #[doc = "Lesson identity referenced by an ingested observation."]
    IngestLessonId
);
owned_memory_text!(
    #[doc = "Rule identity attached to an ingested observation."]
    IngestRuleId
);
owned_memory_text!(
    #[doc = "Fault classification attached to an ingested observation."]
    IngestFaultClass
);
owned_memory_text!(
    #[doc = "Repository context attached to an ingested observation."]
    IngestRepoContext
);
owned_memory_text!(
    #[doc = "Enforcement surface that produced an ingested observation."]
    IngestSourceSurface
);
owned_memory_text!(
    #[doc = "Timestamp retained for an ingested observation."]
    IngestTimestamp
);
owned_memory_text!(
    #[doc = "Complete append-only NDJSON document accepted by Memory ingestion."]
    IngestNdjsonDocument
);
owned_memory_text!(
    #[doc = "Payload discriminator attached to one durable Memory observation."]
    IngestObservationPayloadKind
);
owned_memory_text!(
    #[doc = "Search text produced from one ingested Memory incident."]
    IngestIncidentSearchText
);

owned_memory_text!(
    #[doc = "Serialized structured payload attached to one durable Memory observation."]
    IngestObservationPayload
);

/// Structured payload carried by a durable Memory observation log entry.
/// SERIALIZATION-DOC: JSON is retained solely as opaque data at the Memory log boundary.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: this opaque JSON is owned by the Memory observation boundary."]
pub struct MemoryObservationPayloadWire(serde_json::Value);

/// Domain name for the wire-owned opaque observation payload.
pub type MemoryObservationPayload = MemoryObservationPayloadWire;
owned_memory_text!(
    #[doc = "Canonical qualified symbol name returned by source-snippet retrieval."]
    SnippetQualifiedName
);
owned_memory_text!(
    #[doc = "Repository-relative source path returned by source-snippet retrieval."]
    SnippetRelativePath
);
owned_memory_text!(
    #[doc = "SHA-256 identity of byte-exact source returned by snippet retrieval."]
    SnippetSha256
);
owned_memory_text!(
    #[doc = "Caller or callee name returned as snippet-neighbor context."]
    SnippetNeighborName
);
owned_memory_text!(
    #[doc = "Stable graph-node identity carried by a trace response."]
    TraceNodeId
);
owned_memory_text!(
    #[doc = "Captured call argument expression carried by a data-flow trace."]
    TraceArgumentExpression
);
owned_memory_text!(
    #[doc = "Callee parameter name carried by a data-flow trace when known."]
    TraceParameterName
);
owned_memory_text!(
    #[doc = "HTTP method carried by a cross-service trace mediator."]
    TraceRouteMethod
);
owned_memory_text!(
    #[doc = "HTTP path carried by a cross-service trace mediator."]
    TraceRoutePath
);
owned_memory_text!(
    #[doc = "Variable identifier retained by a parsed graph query."]
    GraphQueryVariable
);
owned_memory_text!(
    #[doc = "Node label retained by a parsed graph query."]
    GraphQueryLabel
);
owned_memory_text!(
    #[doc = "Relationship type retained by a parsed graph query."]
    GraphQueryRelationshipType
);
owned_memory_text!(
    #[doc = "Node property retained by a parsed graph query column."]
    GraphQueryProperty
);

/// One graph-query result row mapping query variables to resolved node identities.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphQueryResultRow(
    std::collections::BTreeMap<GraphQueryVariable, MemoryAnalysisNodeId>,
);

impl GraphQueryResultRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        variable: GraphQueryVariable,
        node_id: MemoryAnalysisNodeId,
    ) -> Option<MemoryAnalysisNodeId> {
        self.0.insert(variable, node_id)
    }

    pub fn get(&self, variable: &str) -> Option<&MemoryAnalysisNodeId> {
        self.0.get(variable)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GraphQueryVariable, &MemoryAnalysisNodeId)> {
        self.0.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &MemoryAnalysisNodeId> {
        self.0.values()
    }
}

impl From<std::collections::BTreeMap<GraphQueryVariable, MemoryAnalysisNodeId>>
    for GraphQueryResultRow
{
    fn from(values: std::collections::BTreeMap<GraphQueryVariable, MemoryAnalysisNodeId>) -> Self {
        Self(values)
    }
}

impl std::ops::Index<&str> for GraphQueryResultRow {
    type Output = MemoryAnalysisNodeId;

    fn index(&self, variable: &str) -> &Self::Output {
        std::ops::Index::index(&self.0, variable)
    }
}

owned_memory_text!(
    #[doc = "Stable document identity carried through retrieval ranking."]
    RankingDocumentId
);
owned_memory_text!(
    #[doc = "Source snippet carried through retrieval ranking."]
    RankingSnippet
);
owned_memory_text!(
    #[doc = "Optional repository source path carried through retrieval ranking."]
    RankingSourcePath
);
owned_memory_text!(
    #[doc = "Embedding model identity retained in vector manifests."]
    EmbeddingModelName
);
owned_memory_text!(
    #[doc = "Embedding scalar data type retained in vector manifests."]
    EmbeddingDtype
);
owned_memory_text!(
    #[doc = "Similarity metric retained in vector manifests."]
    EmbeddingSimilarityMetric
);
owned_memory_text!(
    #[doc = "Normalization strategy retained in vector manifests."]
    EmbeddingNormalization
);
owned_memory_text!(
    #[doc = "Formatter version retained in vector manifests."]
    EmbeddingFormatterVersion
);
owned_memory_text!(
    #[doc = "Chunker version retained in vector manifests."]
    EmbeddingChunkerVersion
);
owned_memory_text!(
    #[doc = "Parser version retained in vector manifests."]
    EmbeddingParserVersion
);
owned_memory_text!(
    #[doc = "Stable project identity returned by the Memory project registry."]
    MemoryProjectId
);
owned_memory_text!(
    #[doc = "Operator-facing name of one hard ranking filter."]
    RankingFilterName
);

/// Canonical executable predicate for one hard ranking filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingFilterDecision {
    Reject,
    Allow,
}

impl From<bool> for RankingFilterDecision {
    fn from(value: bool) -> Self {
        if value {
            Self::Allow
        } else {
            Self::Reject
        }
    }
}

impl From<RankingFilterDecision> for bool {
    fn from(value: RankingFilterDecision) -> Self {
        matches!(value, RankingFilterDecision::Allow)
    }
}

impl RankingFilterDecision {
    /// Return the decision as the boolean required by legacy ranking callers.
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allow)
    }
}

pub struct RankingHardFilterPredicate(
    Box<dyn Fn(&RankingDocumentId) -> RankingFilterDecision + Send + Sync + 'static>,
);

impl RankingHardFilterPredicate {
    /// Wrap a filter that can inspect only the branded document identity.
    pub fn from_predicate<R, F>(predicate: F) -> Self
    where
        F: Fn(&RankingDocumentId) -> R + Send + Sync + 'static,
        R: Into<RankingFilterDecision>,
    {
        Self(Box::new(move |document_id| predicate(document_id).into()))
    }

    /// Evaluate the binary inclusion decision for one document.
    pub fn is_allowed(&self, document_id: &RankingDocumentId) -> bool {
        (self.0)(document_id).is_allowed()
    }
}

impl std::fmt::Debug for RankingHardFilterPredicate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RankingHardFilterPredicate(<function>)")
    }
}
owned_memory_text!(
    #[doc = "Repository root returned by the Memory project registry."]
    MemoryProjectRepoRoot
);
owned_memory_text!(
    #[doc = "Project initialization timestamp returned by the Memory project registry."]
    MemoryProjectInitializedAt
);
owned_memory_text!(
    #[doc = "Append-log name returned by project index status."]
    MemoryProjectLogName
);
owned_memory_text!(
    #[doc = "Stable graph-node identity carried by a similarity edge."]
    SimilarityNodeId
);
owned_memory_text!(
    #[doc = "Stable symbol identity used inside the Memory resolution registry."]
    MemoryResolutionSymbolId
);
owned_memory_text!(
    #[doc = "Stable file identity used inside the Memory resolution registry."]
    MemoryResolutionFileId
);
owned_memory_text!(
    #[doc = "Repository-relative file path used for import-aware Memory resolution."]
    MemoryResolutionFilePath
);
owned_memory_text!(
    #[doc = "Identifier supplied to the Memory similarity tokenizer."]
    SimilarityIdentifierName
);
owned_memory_text!(
    #[doc = "Normalized identifier token used by Memory similarity analysis."]
    SimilarityIdentifierAtom
);
owned_memory_text!(
    #[doc = "Repository-relative path used by Memory similarity analysis."]
    SimilarityPath
);

/// Fixed-width MinHash lanes retained by the Memory similarity engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for SimilarityMinHashValues."]
#[doc = "BRAND-INVARIANT: exactly 64 decoded lanes are retained; raw storage remains private."]
pub struct SimilarityMinHashValues([u32; 64]);

impl SimilarityMinHashValues {
    /// Store the validated fixed-width lane array.
    pub const fn from_array(value: [u32; 64]) -> Self {
        Self(value)
    }

    /// Return the decoded lanes for deterministic comparison.
    pub const fn as_array(self) -> [u32; 64] {
        self.0
    }
}

/// Ordered normalized lexemes produced from one similarity identifier.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SimilarityIdentifierLexemes(Vec<SimilarityIdentifierAtom>);

impl SimilarityIdentifierLexemes {
    /// Construct an empty token collection.
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Append one normalized identifier token.
    pub fn push(&mut self, token: SimilarityIdentifierAtom) {
        self.0.push(token);
    }

    /// Whether tokenization produced no evidence.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for SimilarityIdentifierLexemes {
    type Item = SimilarityIdentifierAtom;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Canonically ordered endpoints for one similarity edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimilarityNodePair {
    pub source_id: SimilarityNodeId,
    pub target_id: SimilarityNodeId,
}

impl SimilarityNodePair {
    /// Order two node identities lexically so each undirected pair is emitted once.
    pub fn ordered(left: SimilarityNodeId, right: SimilarityNodeId) -> Self {
        if left <= right {
            Self {
                source_id: left,
                target_id: right,
            }
        } else {
            Self {
                source_id: right,
                target_id: left,
            }
        }
    }
}
owned_memory_text!(
    #[doc = "Optional exporting-repository git head carried by a shared-memory bundle."]
    MemoryBundleGitHead
);
owned_memory_text!(
    #[doc = "Content hash carried by a shared-memory bundle."]
    MemoryBundleContentHash
);
owned_memory_text!(
    #[doc = "Creator identity carried by a non-community shared-memory bundle."]
    MemoryBundleCreator
);
owned_memory_text!(
    #[doc = "Creation timestamp carried by a shared-memory bundle."]
    MemoryBundleCreatedAt
);
owned_memory_text!(
    #[doc = "Detached signature encoded on a shared-memory bundle."]
    MemoryBundleSignatureHex
);
owned_memory_text!(
    #[doc = "Signer public key encoded on a shared-memory bundle."]
    MemoryBundlePublicKeyHex
);

/// Absolute filesystem path read by source-snippet retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for SnippetAbsolutePath."]
#[doc = "BRAND-INVARIANT: snippet retrieval resolves the repository-relative identity to this path."]
pub struct SnippetAbsolutePath(std::path::PathBuf);

impl SnippetAbsolutePath {
    /// View the resolved absolute path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

/// Repository root used for byte-exact snippet retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnippetRepoRoot(std::path::PathBuf);

impl SnippetRepoRoot {
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<&std::path::Path> for SnippetRepoRoot {
    fn from(value: &std::path::Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl From<&std::path::PathBuf> for SnippetRepoRoot {
    fn from(value: &std::path::PathBuf) -> Self {
        Self(value.as_path().to_path_buf())
    }
}

/// Whether snippet retrieval should include caller, callee, and sibling context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly select neighbor inclusion policy."]
pub struct SnippetIncludeNeighbors(bool);

impl SnippetIncludeNeighbors {
    pub const fn is_included(self) -> bool {
        self.0
    }
}

impl From<bool> for SnippetIncludeNeighbors {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

/// Root directory monitored by the Memory filesystem watcher.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryWatchRoot(std::path::PathBuf);

impl MemoryWatchRoot {
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<&std::path::Path> for MemoryWatchRoot {
    fn from(value: &std::path::Path) -> Self {
        Self(value.to_path_buf())
    }
}

/// Debounce duration applied to one native filesystem-event burst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryWatchDebounceWindow(std::time::Duration);

impl MemoryWatchDebounceWindow {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for MemoryWatchDebounceWindow {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

/// Deadlock-guard deadline duration for one watcher receive operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryWatchDeadline(std::time::Duration);

impl MemoryWatchDeadline {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for MemoryWatchDeadline {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

/// Corpus file count used by adaptive watcher polling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative watched-file count; zero represents an empty corpus."]
pub struct MemoryWatchFileCount(usize);

impl MemoryWatchFileCount {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for MemoryWatchFileCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Adaptive polling interval selected for the watcher fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryWatchPollInterval(std::time::Duration);

impl MemoryWatchPollInterval {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for MemoryWatchPollInterval {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

impl PartialEq<std::time::Duration> for MemoryWatchPollInterval {
    fn eq(&self, other: &std::time::Duration) -> bool {
        self.0 == *other
    }
}

/// Whether a native filesystem event can affect indexed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly encode event relevance."]
pub struct MemoryWatchEventRelevant(bool);

impl MemoryWatchEventRelevant {
    /// Brand the explicit filesystem-event relevance decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for MemoryWatchEventRelevant {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<MemoryWatchEventRelevant> for bool {
    fn from(value: MemoryWatchEventRelevant) -> Self {
        value.0
    }
}

/// Whether the current repository HEAD differs from the recorded watcher state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly encode repository-head change state."]
pub struct MemoryWatchGitHeadChanged(bool);

impl MemoryWatchGitHeadChanged {
    /// Brand the explicit repository-head change decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for MemoryWatchGitHeadChanged {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<MemoryWatchGitHeadChanged> for bool {
    fn from(value: MemoryWatchGitHeadChanged) -> Self {
        value.0
    }
}

owned_memory_text!(
    #[doc = "Previously observed repository HEAD used by watcher polling."]
    MemoryWatchGitHead
);

impl From<std::path::PathBuf> for SnippetAbsolutePath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

transparent_memory_wire!(SnippetAbsolutePath, std::path::PathBuf);

impl From<SnippetAbsolutePath> for std::path::PathBuf {
    fn from(value: SnippetAbsolutePath) -> Self {
        value.0
    }
}

impl AsRef<std::path::Path> for SnippetAbsolutePath {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

impl std::ops::Deref for SnippetAbsolutePath {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Filesystem path emitted by the Memory reindex watcher.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: the Memory watcher owns event-path collection and debounce semantics."]
pub struct MemoryWatchPath(std::path::PathBuf);

impl MemoryWatchPath {
    /// View the watcher-owned path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryWatchPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<MemoryWatchPath> for std::path::PathBuf {
    fn from(value: MemoryWatchPath) -> Self {
        value.0
    }
}

impl AsRef<std::path::Path> for MemoryWatchPath {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

impl std::ops::Deref for MemoryWatchPath {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

macro_rules! memory_usize {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Default,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[doc = "BRAND-INVARIANT: the producing Memory analysis assigns the semantic unit; raw storage remains private."]
        pub struct $name(usize);

        impl $name {
            /// Return the underlying snippet quantity.
            pub const fn get(self) -> usize {
                self.0
            }
        }

        impl From<usize> for $name {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<usize> for $name {
            fn eq(&self, other: &usize) -> bool {
                self.0 == *other
            }
        }

        impl PartialOrd<usize> for $name {
            fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl std::ops::AddAssign<usize> for $name {
            fn add_assign(&mut self, rhs: usize) {
                self.0 += rhs;
            }
        }
    };
}

memory_usize!(
    #[doc = "Byte offset into source content returned by snippet retrieval."]
    SnippetByteOffset
);
memory_usize!(
    #[doc = "Inbound or outbound call-degree count returned by snippet retrieval."]
    SnippetCallCount
);
memory_usize!(
    #[doc = "Maximum traversal depth selected for a graph trace."]
    TraceDepth
);
memory_usize!(
    #[doc = "Minimum or maximum traversal depth retained by a parsed graph query."]
    GraphQueryTraversalDepth
);
memory_usize!(
    #[doc = "Maximum result-row count retained by a parsed graph query."]
    GraphQueryLimit
);
memory_usize!(
    #[doc = "One-based position assigned by a retrieval ranker."]
    RankingPosition
);
memory_usize!(
    #[doc = "Number of scalar components in one embedding vector."]
    EmbeddingDimension
);
memory_usize!(
    #[doc = "Lesson, incident, replay, or record count retained by a learning projection."]
    LearningProjectionCount
);
memory_usize!(
    #[doc = "Number of resolved graph edges crossing a pair of detected clusters."]
    MemoryInterClusterEdgeCount
);
memory_usize!(
    #[doc = "Number of graph rows carrying one schema label or edge type."]
    GraphSchemaRowCount
);
memory_usize!(
    #[doc = "Traversal depth returned by core Memory graph analysis."]
    MemoryAnalysisDepth
);
memory_usize!(
    #[doc = "Inbound or outbound degree returned by core Memory graph analysis."]
    MemoryAnalysisDegree
);
memory_usize!(
    #[doc = "Node count returned by core Memory graph analysis."]
    MemoryAnalysisNodeCount
);
memory_usize!(
    #[doc = "Edge count returned by core Memory graph analysis."]
    MemoryAnalysisEdgeCount
);
memory_usize!(
    #[doc = "Maximum number of ranked results retained by core Memory graph analysis."]
    MemoryAnalysisResultLimit
);
memory_usize!(
    #[doc = "Number of graph nodes assigned to one detected Memory cluster."]
    MemoryClusterSize
);
memory_usize!(
    #[doc = "Number of entries currently retained by a content-addressed artifact manifest."]
    ArtifactManifestEntryCount
);
memory_usize!(
    #[doc = "Maximum label-propagation iterations allowed for Memory clustering."]
    MemoryClusterIterationLimit
);
memory_usize!(
    #[doc = "Maximum source-snippet length retained by Memory redaction."]
    MemoryRedactionSnippetLength
);
memory_usize!(
    #[doc = "Zero-based source-row index retained by Memory quarantine reporting."]
    MemoryQuarantineRowIndex
);
memory_usize!(
    #[doc = "Incident count carried by one Memory session-start lesson summary."]
    MemorySessionIncidentCount
);
memory_usize!(
    #[doc = "Total active-lesson count carried by a Memory session-start recall pack."]
    MemorySessionActiveLessonCount
);
memory_usize!(
    #[doc = "Maximum active-lesson summaries emitted by a Memory session-start recall pack."]
    MemorySessionRecallLimit
);
memory_usize!(
    #[doc = "Running post-landing recurrence count carried by Memory evidence."]
    MemoryEvidenceRecurrenceCount
);
memory_usize!(
    #[doc = "Commit count attached to Memory path history."]
    MemoryGitChangeCount
);
memory_usize!(
    #[doc = "Record or lesson count returned by Memory federation import."]
    MemoryFederationImportCount
);
memory_usize!(
    #[doc = "Record and lesson node count carried by a Memory sharing bundle snapshot."]
    MemoryBundleNodeCount
);
memory_usize!(
    #[doc = "Estimated token count used by Memory retrieval measurement."]
    MemorySearchTokenCount
);
memory_usize!(
    #[doc = "Naive whole-document count used as the Memory token-reduction baseline."]
    MemorySearchNaiveFileCount
);
memory_usize!(
    #[doc = "Average document byte length used as the Memory token-reduction baseline."]
    MemorySearchAverageDocumentBytes
);
memory_usize!(
    #[doc = "Maximum result count requested from the Memory full-text index."]
    MemoryFullTextLimit
);
memory_usize!(
    #[doc = "Number of MinHash lanes retained by a Memory source-body fingerprint."]
    MemoryFingerprintHashCount
);
memory_usize!(
    #[doc = "Number of Memory records accepted from one ingestion document or replay."]
    IngestRecordCount
);
memory_usize!(
    #[doc = "One-based line number identifying malformed Memory ingestion input."]
    IngestLineNumber
);
memory_usize!(
    #[doc = "Number of procedural-memory and route-trace records replayed from durable storage."]
    MemoryObservationReplayCount
);
memory_usize!(
    #[doc = "Zero-based source line retained by a typed Memory integrity error."]
    MemoryErrorLineIndex
);
memory_usize!(
    #[doc = "Number of quarantined rows retained by a typed Memory error."]
    MemoryErrorRowCount
);
memory_usize!(
    #[doc = "Number of similarity edges emitted for one Memory symbol."]
    SimilarityEdgeCount
);

/// Filesystem path retained by a typed Memory error.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryErrorPath(std::path::PathBuf);

/// Filesystem path owned by one append-only Memory log.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: append-log construction decodes and owns this filesystem location before durable use."]
pub struct MemoryAppendLogPath(std::path::PathBuf);

impl MemoryAppendLogPath {
    /// Borrow the decoded append-log path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryAppendLogPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<&std::path::Path> for MemoryAppendLogPath {
    fn from(value: &std::path::Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl From<std::path::PathBuf> for MemoryErrorPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<String> for MemoryErrorPath {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&std::path::Path> for MemoryErrorPath {
    fn from(value: &std::path::Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl std::fmt::Display for MemoryErrorPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(formatter)
    }
}

macro_rules! memory_error_u64 {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Return the retained error quantity.
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

memory_error_u64!(
    #[doc = "Manifest watermark retained by a typed stale-index error."]
    MemoryErrorManifestWatermark
);
memory_error_u64!(
    #[doc = "Durable log length retained by a typed stale-index error."]
    MemoryErrorLogLength
);

/// Score assigned by a retrieval or reranking stage.
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for RankingScore."]
#[doc = "BRAND-INVARIANT: a retrieval stage assigns this score; raw storage remains private."]
pub struct RankingScore(f64);

impl RankingScore {
    /// Return the underlying retrieval score.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for RankingScore {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<RankingScore> for f64 {
    fn from(value: RankingScore) -> Self {
        value.0
    }
}

impl PartialEq<f64> for RankingScore {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for RankingScore {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// Similarity score assigned to a materialized graph edge.
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for SimilarityScore."]
#[doc = "BRAND-INVARIANT: preserves the exact score emitted by the similarity engine; raw storage remains private."]
pub struct SimilarityScore(f64);

impl SimilarityScore {
    /// Return the underlying similarity score.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for SimilarityScore {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl PartialEq<f64> for SimilarityScore {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for SimilarityScore {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// Ordering delta measured between pre-rerank and post-rerank results.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[doc = "Canonical domain representation for MemorySearchRerankerLift."]
#[doc = "BRAND-INVARIANT: preserves the signed reranker delta measured by evaluation; raw storage remains private."]
pub struct MemorySearchRerankerLift(f64);

impl MemorySearchRerankerLift {
    /// Return the measured reranker lift.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for MemorySearchRerankerLift {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl PartialEq<f64> for MemorySearchRerankerLift {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

/// Ratio of naive baseline context size to actual context-pack size.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[doc = "BRAND-INVARIANT: zero denotes unavailable reduction evidence; positive values are finite ratios derived from branded token counts."]
pub struct MemoryContextReductionRatio(f64);

impl MemoryContextReductionRatio {
    /// Return the computed token-reduction ratio.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<f64> for MemoryContextReductionRatio {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl PartialEq<f64> for MemoryContextReductionRatio {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for MemoryContextReductionRatio {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// Clean or finding observation count grouped by Memory analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[doc = "Canonical domain representation for MemoryAnalyticsObservationCount."]
#[doc = "BRAND-INVARIANT: a non-negative observation count; zero is a valid cardinality."]
pub struct MemoryAnalyticsObservationCount(u64);

impl MemoryAnalyticsObservationCount {
    /// Return the grouped observation count.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for MemoryAnalyticsObservationCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialEq<u64> for MemoryAnalyticsObservationCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl std::ops::AddAssign<u64> for MemoryAnalyticsObservationCount {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

/// Number of runtime observations recorded for one traced edge.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for TraceObservationCount."]
#[doc = "BRAND-INVARIANT: runtime trace ingestion accumulates this count; raw storage remains private."]
pub struct TraceObservationCount(u64);

impl TraceObservationCount {
    /// Return the underlying observation count.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for TraceObservationCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<TraceObservationCount> for u64 {
    fn from(value: TraceObservationCount) -> Self {
        value.0
    }
}

impl PartialEq<u64> for TraceObservationCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

/// Node, edge, or log-entry count returned by project status.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for MemoryProjectCount."]
#[doc = "BRAND-INVARIANT: project status computes this count; raw storage remains private."]
pub struct MemoryProjectCount(u64);

impl MemoryProjectCount {
    /// Return the underlying project-status count.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for MemoryProjectCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<MemoryProjectCount> for u64 {
    fn from(value: MemoryProjectCount) -> Self {
        value.0
    }
}

impl PartialEq<u64> for MemoryProjectCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u64> for MemoryProjectCount {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// Shared-memory bundle schema version.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for MemoryBundleSchemaVersion."]
#[doc = "BRAND-INVARIANT: preserves the exact unsigned wire version so bundle validation can reject unsupported schemas."]
pub struct MemoryBundleSchemaVersion(u32);

impl MemoryBundleSchemaVersion {
    /// Brand the exact shared-memory bundle schema version.
    pub const fn try_new(value: u32) -> Self {
        Self(value)
    }
}

impl From<u32> for MemoryBundleSchemaVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<MemoryBundleSchemaVersion> for u32 {
    fn from(value: MemoryBundleSchemaVersion) -> Self {
        value.0
    }
}

impl PartialEq<u32> for MemoryBundleSchemaVersion {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

/// Local model-cache manifest schema version.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for ModelCacheSchemaVersion."]
#[doc = "BRAND-INVARIANT: preserves the exact unsigned wire version so cache validation can reject unsupported schemas."]
pub struct ModelCacheSchemaVersion(u32);

impl ModelCacheSchemaVersion {
    /// Brand the exact model-cache schema version.
    pub const fn try_new(value: u32) -> Self {
        Self(value)
    }
}

impl From<u32> for ModelCacheSchemaVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<ModelCacheSchemaVersion> for u32 {
    fn from(value: ModelCacheSchemaVersion) -> Self {
        value.0
    }
}

impl PartialEq<u32> for ModelCacheSchemaVersion {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl std::fmt::Display for ModelCacheSchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Optional expected artifact byte size from a local model-cache manifest.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "Canonical domain representation for ModelCacheArtifactSizeBytes."]
#[doc = "BRAND-INVARIANT: a non-negative exact byte count; zero is valid for an empty artifact."]
pub struct ModelCacheArtifactSizeBytes(u64);

impl ModelCacheArtifactSizeBytes {
    /// Brand an exact model-cache artifact size; every u64 value is valid.
    pub const fn try_new(value: u64) -> Self {
        Self(value)
    }

    /// Return the exact artifact byte count.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Borrowed local filesystem path of one model-cache artifact.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: cache validation borrows an artifact path from one decoded local manifest candidate."]
pub struct ModelCacheArtifactFile<'a>(&'a std::path::Path);

impl<'a> ModelCacheArtifactFile<'a> {
    /// View the borrowed artifact path.
    pub const fn as_path(self) -> &'a std::path::Path {
        self.0
    }
}

impl<'a> From<&'a std::path::Path> for ModelCacheArtifactFile<'a> {
    fn from(value: &'a std::path::Path) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a std::path::PathBuf> for ModelCacheArtifactFile<'a> {
    fn from(value: &'a std::path::PathBuf) -> Self {
        Self(value.as_path())
    }
}

impl From<u64> for ModelCacheArtifactSizeBytes {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ModelCacheArtifactSizeBytes> for u64 {
    fn from(value: ModelCacheArtifactSizeBytes) -> Self {
        value.0
    }
}

/// Process exit status selected by the Memory CLI transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for MemoryCliExitCode."]
#[doc = "BRAND-INVARIANT: preserves the signed process status selected by the CLI boundary."]
pub struct MemoryCliExitCode(i32);

impl MemoryCliExitCode {
    /// Brand the signed process status selected by the CLI boundary.
    pub const fn try_new(value: i32) -> Self {
        Self(value)
    }
}

impl From<i32> for MemoryCliExitCode {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl From<MemoryCliExitCode> for i32 {
    fn from(value: MemoryCliExitCode) -> Self {
        value.0
    }
}

impl PartialEq<i32> for MemoryCliExitCode {
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

/// Elapsed processing time recorded for one Memory CLI request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryCliRequestDuration(std::time::Duration);

impl MemoryCliRequestDuration {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for MemoryCliRequestDuration {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

/// Success/error disposition decoded from a Memory CLI result envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCliResultDisposition {
    Success,
    Error,
}

impl MemoryCliResultDisposition {
    pub const fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }
}

/// Store directory associated with one registered Memory project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for MemoryProjectStoreRoot."]
pub struct MemoryProjectStoreRoot(std::path::PathBuf);

impl MemoryProjectStoreRoot {
    /// View the project store root.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryProjectStoreRoot {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

transparent_memory_wire!(MemoryProjectStoreRoot, std::path::PathBuf);

impl std::ops::Deref for MemoryProjectStoreRoot {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Borrowed parent directory containing registered Memory project stores.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: the stores directory is supplied by the Memory runtime boundary and is never retained."]
pub struct MemoryStoresDirectory<'a>(&'a std::path::Path);

impl<'a> MemoryStoresDirectory<'a> {
    /// View the borrowed stores directory.
    pub const fn as_path(self) -> &'a std::path::Path {
        self.0
    }
}

impl<'a> From<&'a std::path::Path> for MemoryStoresDirectory<'a> {
    fn from(value: &'a std::path::Path) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a std::path::PathBuf> for MemoryStoresDirectory<'a> {
    fn from(value: &'a std::path::PathBuf) -> Self {
        Self(value.as_path())
    }
}

/// Owned parent directory containing registered Memory project stores.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: the path names the configured parent of project stores and is canonicalized before destructive containment checks."]
pub struct MemoryStoresRoot(std::path::PathBuf);

impl MemoryStoresRoot {
    /// View the owned stores root.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryStoresRoot {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

/// Filesystem path to one persistence artifact owned by a Memory project store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: store artifact paths are derived beneath a validated Memory project store root."]
pub struct MemoryStorePath(std::path::PathBuf);

impl MemoryStorePath {
    /// View the store-owned artifact path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for MemoryStorePath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<&std::path::Path> for MemoryStorePath {
    fn from(value: &std::path::Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl From<&std::path::PathBuf> for MemoryStorePath {
    fn from(value: &std::path::PathBuf) -> Self {
        Self(value.as_path().to_path_buf())
    }
}

impl AsRef<std::path::Path> for MemoryStorePath {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

impl std::ops::Deref for MemoryStorePath {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Exact bytes returned by content-addressed artifact retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: the byte sequence is exact content-addressed payload data, including the valid empty payload."]
pub struct MemoryArtifactBytes(Vec<u8>);

impl MemoryArtifactBytes {}

impl From<Vec<u8>> for MemoryArtifactBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<const N: usize> From<&[u8; N]> for MemoryArtifactBytes {
    fn from(value: &[u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl From<&[u8]> for MemoryArtifactBytes {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

owned_memory_text!(
    #[doc = "Content-addressed key stored in an artifact manifest map."]
    ArtifactManifestEntryKey
);
owned_memory_text!(
    #[doc = "Optional repository-relative path attached to an artifact manifest entry."]
    ArtifactManifestRelativePath
);
owned_memory_text!(
    #[doc = "Timestamp attached to an artifact manifest entry."]
    ArtifactManifestTimestamp
);
owned_memory_text!(
    #[doc = "Source append-log name retained by an index manifest."]
    IndexManifestSourceLog
);
owned_memory_text!(
    #[doc = "Build timestamp retained by an index manifest."]
    IndexManifestBuiltAt
);

/// Current append-log length or index source high-watermark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: this is the exact zero-inclusive append-log length; every u64 value is meaningful."]
pub struct IndexManifestWatermark(u64);

impl IndexManifestWatermark {
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for IndexManifestWatermark {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

transparent_memory_wire!(IndexManifestWatermark, u64);

/// Monotonic instant injected into the open-store cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreCacheInstant(pub(crate) std::time::Instant);

impl StoreCacheInstant {
    pub fn saturating_duration_since(self, earlier: Self) -> std::time::Duration {
        self.0.saturating_duration_since(earlier.0)
    }
}

/// Idle duration after which an open store is evicted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreCacheIdleTimeout(std::time::Duration);

impl StoreCacheIdleTimeout {
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for StoreCacheIdleTimeout {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

/// Whether an open-store cache currently contains a requested key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly encode cache membership."]
pub struct StoreCacheContains(bool);

impl StoreCacheContains {
    /// Brand the explicit cache-membership decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for StoreCacheContains {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<StoreCacheContains> for bool {
    fn from(value: StoreCacheContains) -> Self {
        value.0
    }
}

/// Number of runtime trace records ingested or replayed from durable storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative trace-record count; zero represents an empty store."]
pub struct TraceStoreRecordCount(usize);

impl TraceStoreRecordCount {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for TraceStoreRecordCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl PartialEq<usize> for TraceStoreRecordCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

/// Number of nodes materialized in an operational SQLite graph projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative materialized-node count; zero represents an empty graph."]
pub struct OperationalGraphNodeCount(u64);

impl OperationalGraphNodeCount {
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for OperationalGraphNodeCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialEq<u64> for OperationalGraphNodeCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

/// Number of edges materialized in an operational SQLite graph projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: this is the exact non-negative materialized-edge count; zero represents an edge-free graph."]
pub struct OperationalGraphEdgeCount(u64);

impl OperationalGraphEdgeCount {
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for OperationalGraphEdgeCount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialEq<u64> for OperationalGraphEdgeCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

/// Deterministically ordered node rows from an operational graph projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalGraphNodeRow {
    pub node_id: GraphEventNodeId,
    pub node_kind: GraphEventNodeKind,
}

/// Deterministically ordered node rows from an operational graph projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalGraphNodeSnapshot(Vec<OperationalGraphNodeRow>);

impl From<Vec<OperationalGraphNodeRow>> for OperationalGraphNodeSnapshot {
    fn from(value: Vec<OperationalGraphNodeRow>) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for OperationalGraphNodeSnapshot {
    type Target = [OperationalGraphNodeRow];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<Vec<(String, String)>> for OperationalGraphNodeSnapshot {
    fn eq(&self, other: &Vec<(String, String)>) -> bool {
        self.0.len() == other.len()
            && self
                .0
                .iter()
                .zip(other)
                .all(|(row, raw)| row.node_id.as_str() == raw.0 && row.node_kind.as_str() == raw.1)
    }
}

/// Deterministically ordered edge rows from an operational graph projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalGraphEdgeRow {
    pub from_id: GraphEventNodeId,
    pub to_id: GraphEventNodeId,
    pub label: GraphEventEdgeLabel,
}

/// Deterministically ordered edge rows from an operational graph projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalGraphEdgeSnapshot(Vec<OperationalGraphEdgeRow>);

impl From<Vec<OperationalGraphEdgeRow>> for OperationalGraphEdgeSnapshot {
    fn from(value: Vec<OperationalGraphEdgeRow>) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for OperationalGraphEdgeSnapshot {
    type Target = [OperationalGraphEdgeRow];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for MemoryArtifactBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<Vec<u8>> for MemoryArtifactBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0 == *other
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for MemoryArtifactBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.0.as_slice() == other.as_slice()
    }
}

/// Borrowed repository root used by graph-artifact persistence.
#[derive(Debug, Clone, Copy)]
pub struct GraphArtifactRepoRoot<'a>(&'a std::path::Path);

impl<'a> GraphArtifactRepoRoot<'a> {
    /// View the borrowed artifact repository root.
    pub const fn as_path(self) -> &'a std::path::Path {
        self.0
    }
}

impl<'a> From<&'a std::path::Path> for GraphArtifactRepoRoot<'a> {
    fn from(value: &'a std::path::Path) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a std::path::PathBuf> for GraphArtifactRepoRoot<'a> {
    fn from(value: &'a std::path::PathBuf) -> Self {
        Self(value.as_path())
    }
}

/// Directory containing one repository's persisted graph artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphArtifactDirectory(std::path::PathBuf);

impl GraphArtifactDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for GraphArtifactDirectory {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for GraphArtifactDirectory {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

impl AsRef<std::path::Path> for GraphArtifactDirectory {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

/// Filesystem path to one graph-artifact persistence component.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphArtifactPath(std::path::PathBuf);

impl From<std::path::PathBuf> for GraphArtifactPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl AsRef<std::path::Path> for GraphArtifactPath {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl std::fmt::Display for GraphArtifactPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(formatter)
    }
}

impl PartialEq<std::path::PathBuf> for GraphArtifactPath {
    fn eq(&self, other: &std::path::PathBuf) -> bool {
        self.0 == *other
    }
}

/// Whether a complete supported graph artifact is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly encode complete graph-artifact presence."]
pub struct GraphArtifactPresence(bool);

impl GraphArtifactPresence {
    pub const fn is_present(self) -> bool {
        self.0
    }
}

impl From<bool> for GraphArtifactPresence {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<GraphArtifactPresence> for bool {
    fn from(value: GraphArtifactPresence) -> Self {
        value.0
    }
}

owned_memory_text!(
    #[doc = "Project label persisted in graph-artifact metadata."]
    GraphArtifactProjectName
);
owned_memory_text!(
    #[doc = "Optional source commit persisted in graph-artifact metadata."]
    GraphArtifactCommit
);
owned_memory_text!(
    #[doc = "Index timestamp persisted in graph-artifact metadata."]
    GraphArtifactIndexedAt
);

/// Filesystem path of a validated streaming-cache manifest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for StreamingCacheManifestPath."]
pub struct StreamingCacheManifestPath(std::path::PathBuf);

impl StreamingCacheManifestPath {
    /// View the manifest path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for StreamingCacheManifestPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<StreamingCacheManifestPath> for std::path::PathBuf {
    fn from(value: StreamingCacheManifestPath) -> Self {
        value.0
    }
}

impl AsRef<std::path::Path> for StreamingCacheManifestPath {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

impl std::ops::Deref for StreamingCacheManifestPath {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Filesystem directory containing validated streaming-cache chunk files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamingCacheChunksDirectory(std::path::PathBuf);

impl StreamingCacheChunksDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for StreamingCacheChunksDirectory {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for StreamingCacheChunksDirectory {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Filesystem path of one numbered streaming-cache chunk.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: the path is derived beneath a validated streaming-cache chunks directory from a typed chunk index."]
pub struct StreamingCacheChunkPath(std::path::PathBuf);

impl StreamingCacheChunkPath {
    /// View the chunk path.
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<std::path::PathBuf> for StreamingCacheChunkPath {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value)
    }
}

impl From<StreamingCacheChunkPath> for std::path::PathBuf {
    fn from(value: StreamingCacheChunkPath) -> Self {
        value.0
    }
}

impl AsRef<std::path::Path> for StreamingCacheChunkPath {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

/// Borrowed identifier or relative path sanitized into one cache path segment.
#[derive(Debug, Clone, Copy)]
#[doc = "BRAND-INVARIANT: cache layout borrows the source text only while producing a filesystem-safe owned segment."]
pub struct StreamingCacheSegmentInput<'a>(&'a str);

impl<'a> StreamingCacheSegmentInput<'a> {
    /// View the unsanitized segment input.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for StreamingCacheSegmentInput<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

/// Owned filesystem-safe streaming-cache path segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "BRAND-INVARIANT: every character is ASCII alphanumeric or one of dash, underscore, and dot; empty inputs map to artifact."]
pub struct StreamingCachePathSegment(String);

impl StreamingCachePathSegment {
    /// Create a filesystem-safe cache segment after enforcing its invariant.
    pub fn try_new(value: String) -> Result<Self, crate::boundary::decode_error::DecodeError> {
        if !value.is_empty()
            && !matches!(value.as_str(), "." | "..")
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            Ok(Self(value))
        } else {
            Err(crate::boundary::decode_error::DecodeError::new(
                "streamingCachePathSegment",
                "must be non-empty, must not be a path-navigation marker, and may contain only ASCII alphanumeric, dash, underscore, or dot characters",
            ))
        }
    }

    /// View the sanitized path segment.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Compressed shared-memory payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for MemoryBundlePayload."]
#[doc = "BRAND-INVARIANT: exact signed or encrypted bundle bytes; an empty payload remains representable for boundary rejection."]
pub struct MemoryBundlePayload(Vec<u8>);

impl MemoryBundlePayload {
    /// View the compressed payload.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Mutably view the first compressed byte when present.
    pub fn first_mut(&mut self) -> Option<&mut u8> {
        self.0.first_mut()
    }
}

impl From<Vec<u8>> for MemoryBundlePayload {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<MemoryBundlePayload> for Vec<u8> {
    fn from(value: MemoryBundlePayload) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for MemoryBundlePayload {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::Deref for MemoryBundlePayload {
    type Target = memory_bytes_target!();

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// Dense vector returned by an embedding model.
#[derive(Debug, Clone, PartialEq, Default)]
#[doc = "Canonical domain representation for EmbeddingVector."]
#[doc = "BRAND-INVARIANT: an Embedder produces this ordered vector; raw storage remains private."]
pub struct EmbeddingVector(Vec<f32>);

impl EmbeddingVector {
    /// View the embedding components.
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }
}

impl From<Vec<f32>> for EmbeddingVector {
    fn from(value: Vec<f32>) -> Self {
        Self(value)
    }
}

transparent_memory_wire!(EmbeddingVector, Vec<f32>);

impl From<EmbeddingVector> for Vec<f32> {
    fn from(value: EmbeddingVector) -> Self {
        value.0
    }
}

impl AsRef<[f32]> for EmbeddingVector {
    fn as_ref(&self) -> &[f32] {
        self.as_slice()
    }
}

impl std::ops::Deref for EmbeddingVector {
    type Target = memory_vector_target!();

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl From<&Vec<f32>> for EmbeddingVector {
    fn from(value: &Vec<f32>) -> Self {
        Self(value.as_slice().to_vec())
    }
}

impl<const N: usize> From<&[f32; N]> for EmbeddingVector {
    fn from(value: &[f32; N]) -> Self {
        Self(value.to_vec())
    }
}

owned_memory_text!(
    #[doc = "Stable document identity stored in a Memory vector index."]
    VectorDocumentId
);
owned_memory_text!(
    #[doc = "Document text submitted to the Memory embedding boundary."]
    VectorDocumentText
);

/// One source document awaiting embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorDocument {
    pub id: VectorDocumentId,
    pub text: VectorDocumentText,
}

/// Canonical document batch submitted to the embedding boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VectorDocuments(Vec<VectorDocument>);

impl VectorDocuments {
    pub fn iter(&self) -> impl Iterator<Item = &VectorDocument> {
        self.0.iter()
    }
}

impl From<&Vec<(String, String)>> for VectorDocuments {
    fn from(value: &Vec<(String, String)>) -> Self {
        Self(
            value
                .iter()
                .map(|(id, text)| VectorDocument {
                    id: id.as_str().into(),
                    text: text.as_str().into(),
                })
                .collect(),
        )
    }
}

/// One document identity and its embedding vector.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorIndexEntry {
    pub doc_id: VectorDocumentId,
    pub vector: EmbeddingVector,
}

/// Canonical entries owned by one Memory vector index.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VectorIndexEntries(Vec<VectorIndexEntry>);

impl VectorIndexEntries {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, entry: VectorIndexEntry) {
        self.0.push(entry);
    }

    pub fn iter(&self) -> impl Iterator<Item = &VectorIndexEntry> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<&VectorIndexEntries> for VectorIndexEntries {
    fn from(value: &VectorIndexEntries) -> Self {
        Self(value.0.as_slice().to_vec())
    }
}

impl From<&Vec<(String, Vec<f32>)>> for VectorIndexEntries {
    fn from(value: &Vec<(String, Vec<f32>)>) -> Self {
        Self(
            value
                .iter()
                .map(|(doc_id, vector)| VectorIndexEntry {
                    doc_id: doc_id.as_str().into(),
                    vector: vector.as_slice().to_vec().into(),
                })
                .collect(),
        )
    }
}

impl<const N: usize> From<&[(String, Vec<f32>); N]> for VectorIndexEntries {
    fn from(value: &[(String, Vec<f32>); N]) -> Self {
        Self(
            value
                .iter()
                .map(|(doc_id, vector)| VectorIndexEntry {
                    doc_id: doc_id.as_str().into(),
                    vector: vector.as_slice().to_vec().into(),
                })
                .collect(),
        )
    }
}

/// Maximum number of vector-search candidates returned to ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: this is the exact zero-inclusive candidate count; zero explicitly requests no candidates."]
pub struct VectorSearchLimit(usize);

impl VectorSearchLimit {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for VectorSearchLimit {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Whether an embedding model exactly matches a vector manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: both boolean states are valid and explicitly encode complete manifest compatibility."]
pub struct VectorManifestMatches(bool);

impl VectorManifestMatches {
    /// Brand the explicit vector-manifest compatibility decision.
    pub const fn try_new(value: bool) -> Self {
        Self(value)
    }
}

impl From<bool> for VectorManifestMatches {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<VectorManifestMatches> for bool {
    fn from(value: VectorManifestMatches) -> Self {
        value.0
    }
}

macro_rules! memory_bool {
    ($(#[$doc:meta])* $name:ident, $predicate:ident) => {
        $(#[$doc])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Default,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[doc = "BRAND-INVARIANT: the producing Memory request or analysis assigns the semantic flag; raw storage remains private."]
        pub struct $name(bool);

        impl $name {
            #[doc = "Return whether this Memory option is enabled."]
            pub const fn $predicate(self) -> bool {
                self.0
            }
        }

        impl From<bool> for $name {
            fn from(value: bool) -> Self {
                Self(value)
            }
        }

        impl From<$name> for bool {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<bool> for $name {
            fn eq(&self, other: &bool) -> bool {
                self.0 == *other
            }
        }
    };
}

memory_bool!(
    #[doc = "Whether graph traces may traverse nodes classified as tests."]
    TraceIncludeTests,
    includes_tests
);
memory_bool!(
    #[doc = "Whether a Memory CLI invocation emits its raw JSON envelope."]
    MemoryCliJsonOutput,
    is_json_output
);
memory_bool!(
    #[doc = "Whether a Memory CLI invocation requested progress reporting."]
    MemoryCliProgress,
    reports_progress
);
memory_bool!(
    #[doc = "Whether a Memory ADR lookup found no stored document."]
    MemoryAdrNoDocument,
    is_no_document
);
memory_bool!(
    #[doc = "Whether a cached Memory file summary requires recomputation."]
    MemorySummaryStale,
    is_stale
);
memory_bool!(
    #[doc = "Whether a Memory diagnostic request record represents an error."]
    MemoryDiagnosticIsError,
    is_error
);
memory_bool!(
    #[doc = "Whether a Memory evidence recurrence point occurred after landing."]
    MemoryEvidenceSinceLanding,
    is_since_landing
);
memory_bool!(
    #[doc = "Whether a Memory evidence report retains its originating t0 provenance."]
    MemoryEvidenceHasT0Provenance,
    has_t0_provenance
);
memory_bool!(
    #[doc = "Whether a hybrid Memory search used any degraded or failed capability."]
    MemorySearchIsDegraded,
    is_degraded
);
memory_bool!(
    #[doc = "Whether call traces attach baseline-compatible hop risk labels."]
    TraceRiskLabels,
    includes_risk_labels
);
memory_bool!(
    #[doc = "Whether a parsed graph query deduplicates returned rows."]
    GraphQueryDistinct,
    is_distinct
);
memory_bool!(
    #[doc = "Whether a parsed graph query returns a row count."]
    GraphQueryCount,
    is_count
);
memory_bool!(
    #[doc = "Whether parsed graph-query ordering is descending."]
    GraphQueryDescending,
    is_descending
);
memory_bool!(
    #[doc = "Whether a runtime trace record has an unresolved caller."]
    TraceUnresolvedCaller,
    is_unresolved
);
memory_bool!(
    #[doc = "Whether a runtime trace record has an unresolved callee."]
    TraceUnresolvedCallee,
    is_unresolved
);
memory_bool!(
    #[doc = "Whether both endpoints of a similarity edge are declared in the same file."]
    SimilaritySameFile,
    is_same_file
);
memory_bool!(
    #[doc = "Whether core Memory graph analysis contains a requested node."]
    MemoryAnalysisContainsNode,
    contains_node
);
memory_bool!(
    #[doc = "Whether a Git diff touched the requested Memory repository-relative path."]
    MemoryGitPathTouched,
    is_touched
);
memory_bool!(
    #[doc = "Whether a Memory similarity symbol still has edge emission budget."]
    SimilarityEdgeBudgetAvailable,
    has_budget
);
memory_bool!(
    #[doc = "Whether a Memory CLI envelope represents an unknown tool name."]
    MemoryCliUnknownTool,
    is_unknown_tool
);
memory_bool!(
    #[doc = "Whether a trace query found a path within its requested depth."]
    TracePathExists,
    exists
);
memory_bool!(
    #[doc = "Whether a Memory bundle signer key is present in the local trust list."]
    MemoryTrustListMembership,
    is_trusted
);
memory_bool!(
    #[doc = "Whether a content-addressed artifact manifest contains no entries."]
    ArtifactManifestIsEmpty,
    is_empty
);
memory_bool!(
    #[doc = "Whether a rich Memory evidence report is missing required t0 provenance."]
    MemoryEvidenceIncomplete,
    is_incomplete
);
memory_bool!(
    #[doc = "Whether one parsed Memory lesson-ledger row is a Markdown separator."]
    MemoryLessonSeparatorRow,
    is_separator
);
memory_bool!(
    #[doc = "Whether a parser-classified C-family path names a test file."]
    ParserTestPath,
    is_test
);
memory_bool!(
    #[doc = "Whether a Memory summary entity is present in the link table."]
    MemorySummaryEntityLinked,
    is_linked
);

/// Byte-exact source payload returned by snippet retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[doc = "Canonical domain representation for SnippetSourceBytes."]
#[doc = "BRAND-INVARIANT: snippet retrieval owns byte-range selection; raw storage remains private."]
pub struct SnippetSourceBytes(Vec<u8>);

impl SnippetSourceBytes {
    /// View the byte-exact snippet source.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for SnippetSourceBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

transparent_memory_wire!(SnippetSourceBytes, Vec<u8>);

impl From<SnippetSourceBytes> for Vec<u8> {
    fn from(value: SnippetSourceBytes) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for SnippetSourceBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::Deref for SnippetSourceBytes {
    type Target = memory_bytes_target!();

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl PartialEq<[u8]> for SnippetSourceBytes {
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == other
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for SnippetSourceBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.0.as_slice() == *other
    }
}

/// Resolution tier used to retrieve a source snippet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for SnippetMatchMethod."]
pub enum SnippetMatchMethod {
    /// The requested suffix resolved uniquely at a qualified-name boundary.
    Suffix,
}

impl SnippetMatchMethod {
    /// Return the stable wire spelling used by the snippet tool.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Suffix => "suffix",
        }
    }
}

macro_rules! search_graph_integer {
    ($(#[$doc:meta])* $name:ident, $raw:ty) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[doc = "BRAND-INVARIANT: graph-search construction assigns the semantic unit; raw storage remains private."]
        pub struct $name($raw);

        impl $name {
            /// Return the underlying graph-search quantity.
            pub const fn get(self) -> $raw {
                self.0
            }
        }

        impl From<$raw> for $name {
            fn from(value: $raw) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $raw {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<$raw> for $name {
            fn eq(&self, other: &$raw) -> bool {
                self.0 == *other
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

search_graph_integer!(
    #[doc = "Relationship degree threshold or observed relationship degree in graph search."]
    SearchGraphDegree,
    i64
);
search_graph_integer!(
    #[doc = "Observed relationship degree returned by graph search."]
    SearchGraphObservedDegree,
    u32
);
search_graph_integer!(
    #[doc = "Maximum result count requested from graph search."]
    SearchGraphLimit,
    usize
);
search_graph_integer!(
    #[doc = "Result offset requested from graph search."]
    SearchGraphOffset,
    usize
);
search_graph_integer!(
    #[doc = "Total matching row count returned by graph search."]
    SearchGraphTotal,
    usize
);
search_graph_integer!(
    #[doc = "Typed project or protocol-edge count in a cross-repository report."]
    CrossRepoCount,
    usize
);
search_graph_integer!(
    #[doc = "Typed count, depth, or graph degree carried by impact analysis."]
    ImpactQuantity,
    usize
);
search_graph_integer!(
    #[doc = "One-based matched source line carried by code search."]
    CodeSearchLine,
    usize
);
search_graph_integer!(
    #[doc = "Context-line, result-limit, or match-count quantity carried by code search."]
    CodeSearchQuantity,
    usize
);
search_graph_integer!(
    #[doc = "Signed structural ordering rank carried by code search."]
    CodeSearchStructuralRank,
    i64
);

/// Borrowed regular-expression pattern supplied to graph-augmented code search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: the code-search request owns the regex interpretation while borrowing caller text."]
pub struct CodeSearchPattern<'a>(&'a str);

impl<'a> CodeSearchPattern<'a> {
    /// View the borrowed pattern.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for CodeSearchPattern<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

/// Explicit graph-search behavior switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "BRAND-INVARIANT: the flag is attached to one named graph-search behavior rather than passed as an unlabelled primitive."]
pub struct SearchGraphFlag(bool);

impl SearchGraphFlag {
    /// Whether the selected graph-search behavior is enabled.
    pub const fn is_enabled(self) -> bool {
        self.0
    }
}

impl From<bool> for SearchGraphFlag {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<SearchGraphFlag> for bool {
    fn from(value: SearchGraphFlag) -> Self {
        value.0
    }
}

impl PartialEq<bool> for SearchGraphFlag {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

/// Explicit yes/no signal produced by impact analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "BRAND-INVARIANT: impact analysis computes this signal from graph evidence; raw storage remains private."]
pub struct ImpactSignal(bool);

/// Borrowed graph-node identifier inspected by impact analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: impact analysis borrows an existing graph identifier only for one scope decision."]
pub struct ImpactNodeRef<'a>(&'a str);

impl<'a> ImpactNodeRef<'a> {
    /// View the borrowed graph-node identifier.
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ImpactNodeRef<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl ImpactSignal {
    /// Whether the impact signal is present.
    pub const fn is_present(self) -> bool {
        self.0
    }
}

/// Whether an ingested observation records clean negative evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "BRAND-INVARIANT: the ingestion boundary assigns this flag to the named clean-run meaning."]
pub struct IngestClean(bool);

impl IngestClean {
    /// Whether the observation is clean negative evidence.
    pub const fn is_clean(self) -> bool {
        self.0
    }
}

impl From<bool> for IngestClean {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<IngestClean> for bool {
    fn from(value: IngestClean) -> Self {
        value.0
    }
}

transparent_memory_wire!(IngestClean, bool);

impl PartialEq<bool> for IngestClean {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

impl From<bool> for ImpactSignal {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<ImpactSignal> for bool {
    fn from(value: ImpactSignal) -> Self {
        value.0
    }
}

impl PartialEq<bool> for ImpactSignal {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

/// Whether another graph-search result page exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "BRAND-INVARIANT: pagination logic computes this value from typed offset, limit, and total quantities."]
pub struct SearchGraphHasMore(bool);

impl SearchGraphHasMore {
    /// Whether another result page exists.
    pub const fn has_more(self) -> bool {
        self.0
    }
}

impl From<bool> for SearchGraphHasMore {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<SearchGraphHasMore> for bool {
    fn from(value: SearchGraphHasMore) -> Self {
        value.0
    }
}

impl PartialEq<bool> for SearchGraphHasMore {
    fn eq(&self, other: &bool) -> bool {
        self.0 == *other
    }
}

macro_rules! search_graph_measure {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        #[doc = "BRAND-INVARIANT: graph-search scoring owns the meaning and ordering convention; raw storage remains private."]
        pub struct $name(f64);

        impl $name {
            /// Return the graph-search measure.
            pub const fn get(self) -> f64 {
                self.0
            }
        }

        impl From<f64> for $name {
            fn from(value: f64) -> Self {
                Self(value)
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<f64> for $name {
            fn eq(&self, other: &f64) -> bool {
                self.0 == *other
            }
        }
    };
}

search_graph_measure!(
    #[doc = "BM25-derived ordering rank returned by graph search."]
    SearchGraphRank
);
search_graph_measure!(
    #[doc = "Semantic similarity score returned by graph search."]
    SearchGraphScore
);

/// Normalized confidence attached to one retrieval route trace.
#[derive(Debug, Clone, Copy, PartialEq)]
#[doc = "BRAND-INVARIANT: construction clamps finite values to zero through one and maps non-finite values to zero."]
pub struct RouteConfidence(f64);

impl RouteConfidence {
    /// Normalize an externally supplied confidence signal.
    pub fn normalized(value: f64) -> Self {
        if value.is_finite() {
            Self(value.clamp(0.0, 1.0))
        } else {
            Self(0.0)
        }
    }

    /// Return the normalized confidence.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl From<RouteConfidence> for f64 {
    fn from(value: RouteConfidence) -> Self {
        value.0
    }
}

impl From<f64> for RouteConfidence {
    fn from(value: f64) -> Self {
        Self::normalized(value)
    }
}

impl PartialEq<f64> for RouteConfidence {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

/// Success ratio computed from procedural-memory outcomes for one lesson.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[doc = "BRAND-INVARIANT: construction divides successful outcomes by a non-zero total, producing a finite zero-through-one ratio."]
pub struct ProceduralSuccessRate(f64);

impl ProceduralSuccessRate {
    /// Compute a procedural success ratio from successful and total outcomes.
    pub fn from_counts(successes: usize, total: usize) -> Option<Self> {
        (total != 0).then(|| Self(procedural_ratio(successes, total)))
    }

    /// Return the normalized success ratio.
    pub const fn get(self) -> f64 {
        self.0
    }
}

fn procedural_ratio(successes: usize, total: usize) -> f64 {
    // CAST-JUSTIFICATION: counters are projected to f64 only after the caller
    // proves a non-zero denominator; the result is a normalized telemetry ratio.
    successes as f64 / total as f64
}

impl From<ProceduralSuccessRate> for f64 {
    fn from(value: ProceduralSuccessRate) -> Self {
        value.0
    }
}

impl PartialEq<f64> for ProceduralSuccessRate {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl serde::Serialize for RouteConfidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for RouteConfidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <f64 as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::normalized(value))
    }
}

/// Non-negative complexity, nesting, parameter, or loop measure.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: the complexity analysis owns the metric definition; raw storage remains private."]
pub struct ComplexityMeasure(u32);

impl ComplexityMeasure {
    /// Zero observed complexity contribution.
    pub const ZERO: Self = Self(0);

    /// Baseline cyclomatic complexity for one callable.
    pub const BASELINE: Self = Self(1);

    /// Saturating metric composition.
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Return the exact metric value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for ComplexityMeasure {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<ComplexityMeasure> for u32 {
    fn from(value: ComplexityMeasure) -> Self {
        value.0
    }
}

impl From<ComplexityMeasure> for i64 {
    fn from(value: ComplexityMeasure) -> Self {
        value.get().into()
    }
}

impl std::ops::AddAssign<u32> for ComplexityMeasure {
    fn add_assign(&mut self, rhs: u32) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl PartialEq<u32> for ComplexityMeasure {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u32> for ComplexityMeasure {
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// Presence of one recursion-related complexity signal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for ComplexitySignal."]
pub enum ComplexitySignal {
    /// Signal was not observed.
    #[default]
    Absent,
    /// Signal was observed.
    Present,
}

impl ComplexitySignal {
    /// Whether the signal was observed.
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
}

impl From<bool> for ComplexitySignal {
    fn from(value: bool) -> Self {
        if value {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

/// Count of architecture report members, files, symbols, or edges.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: the producing architecture analysis assigns the exact non-negative count."]
pub struct ArchitectureItemCount(usize);

impl ArchitectureItemCount {
    /// Brand an exact architecture item count; every usize value is valid.
    pub const fn try_new(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for ArchitectureItemCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<ArchitectureItemCount> for usize {
    fn from(value: ArchitectureItemCount) -> Self {
        value.0
    }
}

impl std::ops::AddAssign<usize> for ArchitectureItemCount {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl PartialEq<usize> for ArchitectureItemCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<usize> for ArchitectureItemCount {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::iter::Sum<ArchitectureItemCount> for usize {
    fn sum<I: Iterator<Item = ArchitectureItemCount>>(iter: I) -> Self {
        iter.map(usize::from).sum()
    }
}

/// Zero-based layer position in an architecture ordering.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
#[doc = "BRAND-INVARIANT: the value is assigned by the deterministic layering pass."]
pub struct ArchitectureLayerIndex(usize);

impl ArchitectureLayerIndex {
    /// Brand an exact architecture layer index; every usize value is valid.
    pub const fn try_new(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for ArchitectureLayerIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<ArchitectureLayerIndex> for usize {
    fn from(value: ArchitectureLayerIndex) -> Self {
        value.0
    }
}

/// Ratio of internal to total incident edges for one cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
#[doc = "BRAND-INVARIANT: the architecture analysis computes a finite ratio in the inclusive range zero to one."]
pub struct ArchitectureCohesion(f64);

impl From<f64> for ArchitectureCohesion {
    fn from(value: f64) -> Self {
        if value.is_finite() {
            Self(value.clamp(0.0, 1.0))
        } else {
            Self(0.0)
        }
    }
}

transparent_memory_wire!(ArchitectureCohesion, f64);

impl From<ArchitectureCohesion> for f64 {
    fn from(value: ArchitectureCohesion) -> Self {
        value.0
    }
}

impl PartialOrd<f64> for ArchitectureCohesion {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<f64> for ArchitectureCohesion {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<ArchitectureCohesion> for f64 {
    fn partial_cmp(&self, other: &ArchitectureCohesion) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl PartialEq<ArchitectureCohesion> for f64 {
    fn eq(&self, other: &ArchitectureCohesion) -> bool {
        *self == other.0
    }
}

impl std::fmt::Display for ArchitectureCohesion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

validated_memory_text!(
    #[doc = "Stable identity of one durable memory lesson."]
    MemoryLessonId,
    "memoryLessonId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Stable identity of one durable observation."]
    ObservationId,
    "observationId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Stable identity of one retrieval or proof query."]
    QueryId,
    "queryId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Stable identity of one model or memory run."]
    RunId,
    "runId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Provider-independent model identity."]
    ModelId,
    "modelId",
    |_: &str| true,
    "must be non-empty printable text"
);
validated_memory_text!(
    #[doc = "Human-readable explanation for a model selection decision."]
    SelectionReason,
    "selectionReason",
    |_: &str| true,
    "must be non-empty printable text"
);

/// Stable identity of a graph file node (`file:<relative-path>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for FileNodeId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct FileNodeId(String);

impl FileNodeId {
    #[doc = "The from_rel_path operation for this canonical domain value."]
    pub fn from_rel_path(path: &RelPath) -> Self {
        Self(format!("file:{}", path.as_str()))
    }

    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let relative = value
            .strip_prefix("file:")
            .ok_or_else(|| DecodeError::new("fileNodeId", "must start with `file:`"))?;
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        let _validated_relative = RelPath::try_from(relative.to_owned())?;
        Ok(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FileNodeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::str::FromStr for FileNodeId {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        Self::try_new(value.to_owned())
    }
}

impl TryFrom<String> for FileNodeId {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Lowercase unprefixed SHA-256 used by persisted source fingerprints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for SourceHash."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct SourceHash(String);

impl SourceHash {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        valid
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("sourceHash", "must be 64 lowercase hex characters"))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceHash {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::str::FromStr for SourceHash {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: SourceHash owns validated text after parsing a borrowed hash.
        Self::try_new(value.to_owned())
    }
}

impl std::fmt::Display for SourceHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Git object identity, supporting SHA-1 and SHA-256 repositories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for CommitId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct CommitId(String);

impl CommitId {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid_length = matches!(value.len(), 40 | 64);
        let valid = valid_length
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        valid.then_some(Self(value)).ok_or_else(|| {
            DecodeError::new("commitId", "must be a lowercase SHA-1 or SHA-256 object id")
        })
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CommitId {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::str::FromStr for CommitId {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: CommitId owns validated text after parsing a borrowed object id.
        Self::try_new(value.to_owned())
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

macro_rules! validated_memory_wire {
    ($($value:ty),+ $(,)?) => {
        $(
            impl serde::Serialize for $value {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    serializer.serialize_str(self.as_str())
                }
            }

            impl<'de> serde::Deserialize<'de> for $value {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                    Self::try_from(value).map_err(serde::de::Error::custom)
                }
            }
        )+
    };
}

validated_memory_wire!(FileNodeId, SourceHash, CommitId);

/// Supported local-model quantization formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for ModelQuantization."]
pub enum ModelQuantization {
    Q4KM,
}

impl ModelQuantization {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Q4KM => "Q4_K_M",
        }
    }
}

impl std::fmt::Display for ModelQuantization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl serde::Serialize for ModelQuantization {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ModelQuantization {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        match value.as_str() {
            "Q4_K_M" => Ok(Self::Q4KM),
            _ => Err(serde::de::Error::custom("unsupported model quantization")),
        }
    }
}

/// Hardware resource used by a local model capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClass {
    Cpu,
    Gpu,
    Npu,
}

/// Typed reason a local model capability is degraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradedState {
    ProviderUnavailable,
    Overloaded,
    ModelLoadFailed,
    InvalidOutput,
    LowConfidence,
}

/// Capability load state for a local model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Unavailable,
    Loading,
    Loaded,
    Degraded(DegradedState),
    Failed,
}

/// Canonical Memory domain value for ModelTask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTask {
    Embedding,
    Reranking,
    Summarization,
}
/// Canonical Memory domain value for SourcePolicy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePolicy {
    Bundled,
    ParentInstalled,
    LocalCache,
    Unavailable,
}
/// Canonical Memory domain value for CacheState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Unavailable,
    NotCached,
    CacheReady,
    CacheDegraded,
    CacheCorrupted,
    StorageError,
    ArtifactPresent,
    ArtifactMissing,
    HashMismatch,
    TokenizerMismatch,
}
/// Canonical Memory domain value for ManifestIntegrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestIntegrity {
    Unavailable,
    Verified,
    Unchecked,
    ManifestMissing,
    ChecksumMismatch,
    SignatureInvalid,
    Corrupted,
    Failed,
}
/// Canonical Memory domain value for CacheHealth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheHealth {
    Healthy,
    Degraded,
    Unavailable,
    DownloadDisabled,
    Corrupted,
    StorageError,
}
/// Canonical Memory domain value for DownloadStatus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    DownloadDisabled,
    DownloadNotRequested,
    DownloadInProgress,
    DownloadComplete,
    DownloadFailed,
}
/// Canonical Memory domain value for CacheUnavailableReason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheUnavailableReason {
    ModelSourceUnconfigured,
    ArtifactNotInstalled,
    ManifestUnavailable,
    DownloadDisabled,
    CacheStorageUnavailable,
    IntegrityUnverified,
    CorruptionDetected,
}
/// Canonical Memory domain value for CacheStorageErrorCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStorageErrorCode {
    CacheRootUnavailable,
    ManifestReadFailed,
    ArtifactReadFailed,
    MetadataWriteDisabled,
    StoragePermissionDenied,
    QuotaUnavailable,
}
/// Canonical Memory domain value for CacheCorruptionReasonCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCorruptionReasonCode {
    ManifestMissing,
    ChecksumMismatch,
    SignatureInvalid,
    ArtifactMissing,
    ManifestArtifactMismatch,
    UnknownIntegrity,
}

/// Runtime provider selected for local inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Cpu,
    DirectMl,
    OpenVino,
    Vulkan,
    Cuda,
    CoreMl,
    Npu,
}

impl ProviderKind {
    pub const fn resource_class(self) -> ResourceClass {
        match self {
            Self::Cpu => ResourceClass::Cpu,
            Self::DirectMl | Self::OpenVino | Self::Vulkan | Self::Cuda | Self::CoreMl => {
                ResourceClass::Gpu
            }
            Self::Npu => ResourceClass::Npu,
        }
    }
}

/// Canonical Memory domain value for ModelCacheRootMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCacheRootMode {
    DevRepoLocal,
    AppData,
}
/// Canonical Memory domain value for ModelRuntimeServiceRoute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRuntimeServiceRoute {
    Health,
    Models,
    LoadModel,
    UnloadModel,
    Chat,
    Embeddings,
    Rerank,
}
/// Canonical Memory domain value for ChatModelArchitecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatModelArchitecture {
    Dense,
    Moe,
}

/// Canonical Memory domain value for LocalRuntimeKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRuntimeKind {
    LlamaCpp,
    OnnxOrt,
    DeterministicFallback,
}

impl LocalRuntimeKind {
    pub const fn is_real_backend(self) -> bool {
        !matches!(self, Self::DeterministicFallback)
    }
}

/// Canonical Memory domain value for LocalRuntimeArtifactKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRuntimeArtifactKind {
    Manifest,
    Model,
    Tokenizer,
    Config,
    Adapter,
    ExternalData,
    Unknown,
}
/// Canonical Memory domain value for LocalRuntimeAcceleration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRuntimeAcceleration {
    Auto,
    Cpu,
    Gpu,
    Npu,
}
/// Canonical Memory domain value for RuntimeWorkload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeWorkload {
    Chat,
    Embedding,
    Reranking,
}
/// Canonical Memory domain value for RuntimeActivityState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeActivityState {
    Idle,
    Loading,
    ChatActive,
    EmbeddingActive,
    RerankingActive,
    Paused,
}
/// Canonical Memory domain value for RuntimeAdmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdmission {
    Admit,
    Queue,
    PauseBackgroundThenAdmit,
}
/// Canonical Memory domain value for RuntimeOwnershipMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeOwnershipMode {
    EnforcerSubprocess,
    EnforcerIsolatedWorker,
    EnforcerInProcess,
    ExternalServer,
    Unmanaged,
}

impl RuntimeOwnershipMode {
    pub const fn is_enforcer_owned(self) -> bool {
        matches!(
            self,
            Self::EnforcerSubprocess | Self::EnforcerIsolatedWorker | Self::EnforcerInProcess
        )
    }
}

/// Canonical Memory domain value for RuntimeExecutionIsolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeExecutionIsolation {
    EnforcerManagedChildProcess,
    EnforcerIsolatedWorkerProcess,
    EnforcerInProcessLibrary,
    ExternalServerProcess,
    UnmanagedProcess,
}
/// Canonical Memory domain value for RuntimeRequestProtocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRequestProtocol {
    EnforcerWorkerEnv,
    EnforcerStdio,
    ExternalHttp,
    None,
}

/// Product responsibilities owned by the local runtime manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeManagedCapability {
    LoadUnload,
    PauseResumeCancel,
    TimeoutKill,
    ProviderSelection,
    CachePolicy,
    ChatHistoryPolicy,
    WorkloadAdmission,
}

/// Workload executed by the isolated ORT worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrtWorkerTask {
    Embedding,
    Reranker,
}

impl OrtWorkerTask {
    pub const fn env_value(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Reranker => "reranker",
        }
    }
}

/// Canonical Memory domain value for OrtWorkerLifecycleState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrtWorkerLifecycleState {
    Idle,
    Loading,
    Ready,
    EmbeddingActive,
    RerankingActive,
    PausedEmbedding,
    PausedReranking,
    Cancelled,
    TimedOut,
    Unloaded,
}
/// Canonical Memory domain value for OrtWorkerLifecycleAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrtWorkerLifecycleAction {
    Load,
    MarkReady,
    StartEmbedding,
    StartReranker,
    Pause,
    Resume,
    Cancel,
    TimeoutKill,
    Unload,
}

impl OrtWorkerLifecycleState {
    pub const fn activity(self) -> RuntimeActivityState {
        match self {
            Self::Idle | Self::Ready | Self::Cancelled | Self::TimedOut | Self::Unloaded => {
                RuntimeActivityState::Idle
            }
            Self::Loading => RuntimeActivityState::Loading,
            Self::EmbeddingActive => RuntimeActivityState::EmbeddingActive,
            Self::RerankingActive => RuntimeActivityState::RerankingActive,
            Self::PausedEmbedding | Self::PausedReranking => RuntimeActivityState::Paused,
        }
    }
}

/// Kind of record accepted into the local memory graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    UserPref,
    Lesson,
    Decision,
    Observation,
    Incident,
}

/// Domain partition assigned to a memory record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordDomain {
    Harness,
    Code,
    User,
}

/// Priority tier used by background Memory work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryPriority {
    Cold,
    Warm,
    Hot,
}

/// Signal that produced a similarity edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityMode {
    MinHashFingerprint,
    BodyShingle,
    IdentifierToken,
}

/// Confidence assigned to a resolved code reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionConfidence {
    Resolved,
    Probable,
    Ambiguous,
    Unresolved,
}

/// Intended sharing audience for an exported Memory bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryShareScope {
    Personal,
    Team,
    Community,
}

/// Explicit per-operation permission to export Memory data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportConsent {
    #[default]
    NotGranted,
    Granted,
}

/// Structural source kind represented by a retrieval document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentKind {
    Function,
    Route,
    Type,
    Test,
    File,
    Lesson,
    Artifact,
    Summary,
    Other,
}

/// Typed multiplier or additive weight applied during memory retrieval ranking.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[doc = "Canonical domain representation for SearchBoost."]
#[doc = "BRAND-INVARIANT: the value is selected by the typed retrieval taxonomy; raw storage remains private."]
pub struct SearchBoost(f64);

impl From<SearchBoost> for f64 {
    fn from(value: SearchBoost) -> Self {
        value.0
    }
}

impl DocumentKind {
    pub const fn label_boost(self) -> SearchBoost {
        SearchBoost(match self {
            Self::Function => 10.0,
            Self::Route => 8.0,
            Self::Type => 5.0,
            Self::Test => 4.0,
            Self::Lesson => 6.0,
            Self::Summary => 3.0,
            Self::Artifact => 2.0,
            Self::File | Self::Other => 1.0,
        })
    }
}

/// Relationship traversed by Memory's code adjacency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryEdgeKind {
    Imports,
    Calls,
    Route,
    Contains,
    Inherits,
    Implements,
    Decorates,
    TypeRef,
    Defines,
    DataFlows,
}

/// Direction used for graph traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    Out,
    In,
    Both,
}

/// Risk label assigned to a graph hop by trace distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLabel {
    Critical,
    High,
    Medium,
    Low,
}

impl RiskLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

/// Fidelity of a data-flow trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approximation {
    CallGraphOnly,
}

/// Evidence used to match an HTTP relationship across repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossHttpMatchKind {
    RouteDeclaration,
    HttpClient,
    LiteralUrl,
}

/// Cross-repository communication protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossRepoProtocol {
    Async,
    Grpc,
    Graphql,
    Trpc,
}

/// Provenance of a traced graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeProvenance {
    Parsed,
    Inferred,
    Runtime,
}

/// Active and next embedding index generations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for EmbeddingGenerationId."]
pub enum EmbeddingGenerationId {
    /// Sentinel used only while recovering from a poisoned in-memory lock.
    Recovery,
    /// Valid immutable embedding-index generation.
    Generation(std::num::NonZeroU32),
}

impl EmbeddingGenerationId {
    /// Poison-recovery generation used when the in-memory state lock cannot be read.
    pub const RECOVERY: Self = Self::Recovery;

    /// Initial generation used by a newly constructed weaver.
    pub const INITIAL: Self = Self::Generation(std::num::NonZeroU32::MIN);

    /// Brand a validated non-zero generation read at a configuration boundary.
    pub const fn from_nonzero(value: std::num::NonZeroU32) -> Self {
        Self::Generation(value)
    }
}

impl From<EmbeddingGenerationId> for u32 {
    fn from(value: EmbeddingGenerationId) -> Self {
        match value {
            EmbeddingGenerationId::Recovery => 0,
            EmbeddingGenerationId::Generation(value) => value.get(),
        }
    }
}

impl std::fmt::Display for EmbeddingGenerationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recovery => 0,
            Self::Generation(value) => value.get(),
        }
        .fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingGeneration {
    Stable {
        active: EmbeddingGenerationId,
    },
    Migrating {
        active: EmbeddingGenerationId,
        next: EmbeddingGenerationId,
    },
}

impl EmbeddingGeneration {
    pub const fn active(self) -> EmbeddingGenerationId {
        match self {
            Self::Stable { active } | Self::Migrating { active, .. } => active,
        }
    }

    pub const fn next(self) -> Option<EmbeddingGenerationId> {
        match self {
            Self::Stable { .. } => None,
            Self::Migrating { next, .. } => Some(next),
        }
    }

    pub const fn is_migrating(self) -> bool {
        matches!(self, Self::Migrating { .. })
    }
}

/// Symbol kind persisted in a graph artifact snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphSymbolKindSnapshot {
    Function,
    Type,
    Test,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    TypeAlias,
    Module,
    Lambda,
    Variable,
    Constant,
}

/// Kind of operation executed by a llama.cpp probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaCppProbeKind {
    Generate,
    Embedding,
}

/// Backend preference passed to llama.cpp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaCppBackendHint {
    Auto,
    Native,
    Vulkan,
    OpenVino,
}

/// Lifecycle state of an Enforcer-owned llama.cpp runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaCppLifecycleState {
    Idle,
    ToolchainReady,
    ModelLoading,
    Ready,
    ChatActive,
    EmbeddingActive,
    PausedChat,
    PausedEmbedding,
    Cancelled,
    TimedOut,
    Unloaded,
}

/// Lifecycle action applied to an Enforcer-owned llama.cpp runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaCppLifecycleAction {
    ResolveToolchain,
    LoadModel,
    MarkReady,
    StartChat,
    StartEmbedding,
    Pause,
    Resume,
    Cancel,
    TimeoutKill,
    Unload,
}

/// Outcome of applying procedural Memory guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralOutcome {
    RetrievalSuccess,
    RetrievalFailure,
    FixSuccess,
    FixFailure,
}

impl ProceduralOutcome {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::RetrievalSuccess | Self::FixSuccess)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RetrievalSuccess => "retrieval-success",
            Self::RetrievalFailure => "retrieval-failure",
            Self::FixSuccess => "fix-success",
            Self::FixFailure => "fix-failure",
        }
    }
}

/// Architecture report section requested by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aspect {
    All,
    Overview,
    Structure,
    Dependencies,
    Routes,
    Languages,
    Packages,
    EntryPoints,
    Hotspots,
    Boundaries,
    Layers,
    FileTree,
    Clusters,
}

/// Kind of executable or callable architecture entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPointKind {
    BinaryMain,
    LibraryRoot,
    RouteHandler,
}

/// Dependency-layer classification assigned to an architecture section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerCategory {
    Entry,
    Api,
    Core,
    Leaf,
    Internal,
}

/// Severity threshold for Memory diagnostic records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
    None,
}

impl Level {
    pub fn from_env_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "0" | "debug" => Some(Self::Debug),
            "1" | "info" => Some(Self::Info),
            "2" | "warn" | "warning" => Some(Self::Warn),
            "3" | "error" => Some(Self::Error),
            "4" | "none" => Some(Self::None),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::None => "none",
        }
    }

    pub const fn should_emit(self, configured_min: Self) -> bool {
        match configured_min {
            Self::Debug => true,
            Self::Info => !matches!(self, Self::Debug),
            Self::Warn => matches!(self, Self::Warn | Self::Error | Self::None),
            Self::Error => matches!(self, Self::Error | Self::None),
            Self::None => false,
        }
    }
}

/// Encoding used for emitted Memory diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

/// Indexing phase that skipped a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipPhase {
    Walk,
    Parse,
    Extract,
}

impl std::fmt::Display for SkipPhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Walk => "walk",
            Self::Parse => "parse",
            Self::Extract => "extract",
        })
    }
}

/// Flattened load state used in runtime reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStateReport {
    Unavailable,
    Loading,
    Loaded,
    DegradedProviderUnavailable,
    DegradedOverloaded,
    DegradedModelLoadFailed,
    DegradedInvalidOutput,
    DegradedLowConfidence,
    Failed,
}

impl From<LoadState> for LoadStateReport {
    fn from(value: LoadState) -> Self {
        match value {
            LoadState::Unavailable => Self::Unavailable,
            LoadState::Loading => Self::Loading,
            LoadState::Loaded => Self::Loaded,
            LoadState::Degraded(DegradedState::ProviderUnavailable) => {
                Self::DegradedProviderUnavailable
            }
            LoadState::Degraded(DegradedState::Overloaded) => Self::DegradedOverloaded,
            LoadState::Degraded(DegradedState::ModelLoadFailed) => Self::DegradedModelLoadFailed,
            LoadState::Degraded(DegradedState::InvalidOutput) => Self::DegradedInvalidOutput,
            LoadState::Degraded(DegradedState::LowConfidence) => Self::DegradedLowConfidence,
            LoadState::Failed => Self::Failed,
        }
    }
}

/// Flattened resource class used in runtime reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClassReport {
    Cpu,
    Gpu,
    Npu,
}

impl From<ResourceClass> for ResourceClassReport {
    fn from(value: ResourceClass) -> Self {
        match value {
            ResourceClass::Cpu => Self::Cpu,
            ResourceClass::Gpu => Self::Gpu,
            ResourceClass::Npu => Self::Npu,
        }
    }
}

/// Depth/cost mode for repository indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndexMode {
    #[default]
    Full,
    Moderate,
    Fast,
}

impl IndexMode {
    pub const fn computes_git_history(self) -> bool {
        !matches!(self, Self::Fast)
    }
}

/// Structural label assigned to a code-graph search node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    Function,
    Type,
    Test,
    File,
    TextOnly,
    Tombstone,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    TypeAlias,
    Module,
    Lambda,
    Variable,
    Constant,
}

impl NodeLabel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "Function",
            Self::Type => "Type",
            Self::Test => "Test",
            Self::File => "File",
            Self::TextOnly => "TextOnly",
            Self::Tombstone => "Tombstone",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Struct => "Struct",
            Self::Interface => "Interface",
            Self::Enum => "Enum",
            Self::TypeAlias => "TypeAlias",
            Self::Module => "Module",
            Self::Lambda => "Lambda",
            Self::Variable => "Variable",
            Self::Constant => "Constant",
        }
    }

    pub const fn bm25_boost(&self) -> SearchBoost {
        SearchBoost(match self {
            Self::Function | Self::Test | Self::Method | Self::Lambda => 10.0,
            Self::Type | Self::Class | Self::Interface | Self::Enum => 5.0,
            Self::File
            | Self::TextOnly
            | Self::Tombstone
            | Self::Struct
            | Self::TypeAlias
            | Self::Module
            | Self::Variable
            | Self::Constant => 0.0,
        })
    }

    pub const fn is_bm25_noise(&self) -> bool {
        matches!(
            self,
            Self::File | Self::TextOnly | Self::Module | Self::Variable
        )
    }
}

/// Result of one background memory-enrichment attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    Succeeded {
        task_key: MemoryTaskKey,
    },
    RetryScheduled {
        task_key: MemoryTaskKey,
        attempt: RetryAttemptCount,
    },
    DeadLettered {
        task_key: MemoryTaskKey,
        attempts: RetryAttemptCount,
    },
}

/// Field-level reason that a persisted vector index cannot be reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorStaleReason {
    EmbeddingModel { expected: String, actual: String },
    Dimension { expected: usize, actual: usize },
    Dtype { expected: String, actual: String },
    SimilarityMetric { expected: String, actual: String },
    Normalization { expected: String, actual: String },
    FormatterVersion { expected: String, actual: String },
    ChunkerVersion { expected: String, actual: String },
    ParserVersion { expected: String, actual: String },
}

/// Language family supported by memory's structural complexity analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLanguage {
    Rust,
    TypeScriptOrJavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Php,
}

/// Syntax-only classification of a method-call receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverHint {
    SelfOrThis,
    NewExpression,
    Identifier,
    Literal,
    Other,
}

/// Canonical language classification persisted on memory graph nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageTag {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Php,
    Kotlin,
    Swift,
    Tsx,
    Solidity,
    Gdscript,
    Dart,
    Scala,
    Groovy,
    Ruby,
    Zig,
    ObjectiveC,
    Bash,
    Lua,
    Elixir,
    Haskell,
    OCaml,
    Erlang,
    Cuda,
    D,
    PowerShell,
    Fsharp,
    Gleam,
    Glsl,
    Ada,
    Apex,
    Crystal,
    R,
    Perl,
    Clojure,
    Julia,
    Odin,
    Pascal,
    Qml,
    Rescript,
    Squirrel,
    Sway,
    Starlark,
    Templ,
    Typst,
    Wgsl,
    Wolfram,
    Slang,
    Scss,
    Cmake,
    Makefile,
    Fortran,
    Vimscript,
    Puppet,
    Elm,
    Bicep,
    Bitbake,
    Cairo,
    Cfscript,
    Func,
    Move,
    Nickel,
    Jsonnet,
    Just,
    Hlsl,
    Ispc,
    Purescript,
    Magma,
    Hare,
    Pony,
    Nasm,
    Cobol,
    Commonlisp,
    Lean,
    Tlaplus,
    Verilog,
    Vhdl,
    Systemverilog,
    Capnp,
    EmacsLisp,
    Agda,
    Form,
    Awk,
    Fish,
    Zsh,
    Tcl,
    Scheme,
    Racket,
    Smithy,
    Pine,
    Matlab,
    Luau,
    Teal,
    Fennel,
    Meson,
    Kconfig,
    Hcl,
    Nix,
    Sql,
    Protobuf,
    Prisma,
    Pkl,
    Thrift,
    Wit,
    LlvmIr,
    TableGen,
    Cfml,
    Gotemplate,
    Devicetree,
    Smali,
    Json5,
    Kdl,
    LinkerScript,
    Liquid,
    Markdown,
    Mermaid,
    Po,
    Properties,
    Regex,
    Assembly,
    Astro,
    Beancount,
    Bibtex,
    Blade,
    Css,
    Csv,
    Diff,
    Dockerfile,
    Dotenv,
    Gitattributes,
    Gitignore,
    Gn,
    GoMod,
    Graphql,
    Html,
    Hyprlang,
    Ini,
    Janet,
    Jinja2,
    Jsdoc,
    Json,
    Requirements,
    Ron,
    Rst,
    Soql,
    Sosl,
    Sshconfig,
    Svelte,
    Toml,
    Vue,
    Xml,
    Yaml,
    ConfigToml,
    ConfigJson,
    ConfigYaml,
    TextOnly,
}

/// Rollup status for one required Memory proof-matrix prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProofPrefixStatus {
    Green,
    Red,
    Pending,
}

/// Structural graph mutation persisted in the append-only Memory log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEventKind {
    NodeAdded {
        node_id: GraphEventNodeId,
        node_kind: GraphEventNodeKind,
    },
    EdgeAdded {
        from: GraphEventNodeId,
        to: GraphEventNodeId,
        label: GraphEventEdgeLabel,
    },
}

/// Recurrence or clean-run evidence attached to a learned Memory lesson.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrenceNegativeKind {
    RecurrenceCount {
        recurrence_count: MemoryEvidenceRecurrenceCount,
        previous_count: Option<MemoryEvidenceRecurrenceCount>,
    },
    NegativeEvidence {
        reason: MemoryRecurrenceNegativeReason,
    },
}

impl MemoryProofPrefixStatus {
    pub const fn is_green(self) -> bool {
        matches!(self, Self::Green)
    }
}

macro_rules! memory_enum_wire {
    ($type:ty { $($variant:path => $wire:literal),+ $(,)? }) => {
        impl serde::Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(match *self {
                    $($variant => $wire),+
                })
            }
        }

        impl<'de> serde::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                match value.as_str() {
                    $($wire => Ok($variant)),+,
                    _ => Err(serde::de::Error::custom(concat!(
                        "unsupported ",
                        stringify!($type),
                        " value"
                    ))),
                }
            }
        }
    };
}

memory_enum_wire!(SnippetMatchMethod {
    SnippetMatchMethod::Suffix => "suffix"
});

memory_enum_wire!(ComplexitySignal {
    ComplexitySignal::Absent => "Absent",
    ComplexitySignal::Present => "Present"
});

memory_enum_wire!(ModelRuntimeObservationKind {
    ModelRuntimeObservationKind::ModelLoadFailure => "model-load-failure",
    ModelRuntimeObservationKind::ProviderDowngrade => "provider-downgrade",
    ModelRuntimeObservationKind::ArtifactHashMismatch => "artifact-hash-mismatch",
    ModelRuntimeObservationKind::TokenizerHashMismatch => "tokenizer-hash-mismatch",
    ModelRuntimeObservationKind::DegradedFallback => "degraded-fallback",
    ModelRuntimeObservationKind::SuccessfulLocalLoad => "successful-local-load",
    ModelRuntimeObservationKind::RetrievalQualityProof => "retrieval-quality-proof",
    ModelRuntimeObservationKind::RerankerLiftProof => "reranker-lift-proof",
    ModelRuntimeObservationKind::TokenReductionProof => "token-reduction-proof",
    ModelRuntimeObservationKind::RouteChoiceImprovement => "route-choice-improvement",
    ModelRuntimeObservationKind::RecurrenceOrNegativeEvidence => "recurrence-or-negative-evidence",
});
memory_enum_wire!(RecordKind {
    RecordKind::UserPref => "userPref",
    RecordKind::Lesson => "lesson",
    RecordKind::Decision => "decision",
    RecordKind::Observation => "observation",
    RecordKind::Incident => "incident",
});
memory_enum_wire!(RecordDomain {
    RecordDomain::Harness => "harness",
    RecordDomain::Code => "code",
    RecordDomain::User => "user",
});
memory_enum_wire!(MemoryShareScope {
    MemoryShareScope::Personal => "personal",
    MemoryShareScope::Team => "team",
    MemoryShareScope::Community => "community",
});
memory_enum_wire!(DocumentKind {
    DocumentKind::Function => "function",
    DocumentKind::Route => "route",
    DocumentKind::Type => "type",
    DocumentKind::Test => "test",
    DocumentKind::File => "file",
    DocumentKind::Lesson => "lesson",
    DocumentKind::Artifact => "artifact",
    DocumentKind::Summary => "summary",
    DocumentKind::Other => "other",
});
memory_enum_wire!(MemoryProofPrefixStatus {
    MemoryProofPrefixStatus::Green => "green",
    MemoryProofPrefixStatus::Red => "red",
    MemoryProofPrefixStatus::Pending => "pending",
});

memory_enum_wire!(EdgeProvenance {
    EdgeProvenance::Parsed => "parsed",
    EdgeProvenance::Inferred => "inferred",
    EdgeProvenance::Runtime => "runtime",
});

memory_enum_wire!(GraphSymbolKindSnapshot {
    GraphSymbolKindSnapshot::Function => "Function",
    GraphSymbolKindSnapshot::Type => "Type",
    GraphSymbolKindSnapshot::Test => "Test",
    GraphSymbolKindSnapshot::Method => "Method",
    GraphSymbolKindSnapshot::Class => "Class",
    GraphSymbolKindSnapshot::Struct => "Struct",
    GraphSymbolKindSnapshot::Interface => "Interface",
    GraphSymbolKindSnapshot::Enum => "Enum",
    GraphSymbolKindSnapshot::TypeAlias => "TypeAlias",
    GraphSymbolKindSnapshot::Module => "Module",
    GraphSymbolKindSnapshot::Lambda => "Lambda",
    GraphSymbolKindSnapshot::Variable => "Variable",
    GraphSymbolKindSnapshot::Constant => "Constant",
});

memory_enum_wire!(LlamaCppProbeKind {
    LlamaCppProbeKind::Generate => "generate",
    LlamaCppProbeKind::Embedding => "embedding",
});

memory_enum_wire!(LlamaCppBackendHint {
    LlamaCppBackendHint::Auto => "auto",
    LlamaCppBackendHint::Native => "native",
    LlamaCppBackendHint::Vulkan => "vulkan",
    LlamaCppBackendHint::OpenVino => "open-vino",
});

memory_enum_wire!(LlamaCppLifecycleState {
    LlamaCppLifecycleState::Idle => "idle",
    LlamaCppLifecycleState::ToolchainReady => "toolchain-ready",
    LlamaCppLifecycleState::ModelLoading => "model-loading",
    LlamaCppLifecycleState::Ready => "ready",
    LlamaCppLifecycleState::ChatActive => "chat-active",
    LlamaCppLifecycleState::EmbeddingActive => "embedding-active",
    LlamaCppLifecycleState::PausedChat => "paused-chat",
    LlamaCppLifecycleState::PausedEmbedding => "paused-embedding",
    LlamaCppLifecycleState::Cancelled => "cancelled",
    LlamaCppLifecycleState::TimedOut => "timed-out",
    LlamaCppLifecycleState::Unloaded => "unloaded",
});

memory_enum_wire!(LlamaCppLifecycleAction {
    LlamaCppLifecycleAction::ResolveToolchain => "resolve-toolchain",
    LlamaCppLifecycleAction::LoadModel => "load-model",
    LlamaCppLifecycleAction::MarkReady => "mark-ready",
    LlamaCppLifecycleAction::StartChat => "start-chat",
    LlamaCppLifecycleAction::StartEmbedding => "start-embedding",
    LlamaCppLifecycleAction::Pause => "pause",
    LlamaCppLifecycleAction::Resume => "resume",
    LlamaCppLifecycleAction::Cancel => "cancel",
    LlamaCppLifecycleAction::TimeoutKill => "timeout-kill",
    LlamaCppLifecycleAction::Unload => "unload",
});

memory_enum_wire!(ProceduralOutcome {
    ProceduralOutcome::RetrievalSuccess => "retrieval-success",
    ProceduralOutcome::RetrievalFailure => "retrieval-failure",
    ProceduralOutcome::FixSuccess => "fix-success",
    ProceduralOutcome::FixFailure => "fix-failure",
});

memory_enum_wire!(LoadStateReport {
    LoadStateReport::Unavailable => "unavailable",
    LoadStateReport::Loading => "loading",
    LoadStateReport::Loaded => "loaded",
    LoadStateReport::DegradedProviderUnavailable => "degraded-provider-unavailable",
    LoadStateReport::DegradedOverloaded => "degraded-overloaded",
    LoadStateReport::DegradedModelLoadFailed => "degraded-model-load-failed",
    LoadStateReport::DegradedInvalidOutput => "degraded-invalid-output",
    LoadStateReport::DegradedLowConfidence => "degraded-low-confidence",
    LoadStateReport::Failed => "failed",
});

memory_enum_wire!(ResourceClassReport {
    ResourceClassReport::Cpu => "cpu",
    ResourceClassReport::Gpu => "gpu",
    ResourceClassReport::Npu => "npu",
});
memory_enum_wire!(ModelTask {
    ModelTask::Embedding => "embedding",
    ModelTask::Reranking => "reranking",
    ModelTask::Summarization => "summarization",
});
memory_enum_wire!(SourcePolicy {
    SourcePolicy::Bundled => "bundled",
    SourcePolicy::ParentInstalled => "parent-installed",
    SourcePolicy::LocalCache => "local-cache",
    SourcePolicy::Unavailable => "unavailable",
});
memory_enum_wire!(CacheState {
    CacheState::Unavailable => "unavailable",
    CacheState::NotCached => "not-cached",
    CacheState::CacheReady => "cache-ready",
    CacheState::CacheDegraded => "cache-degraded",
    CacheState::CacheCorrupted => "cache-corrupted",
    CacheState::StorageError => "storage-error",
    CacheState::ArtifactPresent => "artifact-present",
    CacheState::ArtifactMissing => "artifact-missing",
    CacheState::HashMismatch => "hash-mismatch",
    CacheState::TokenizerMismatch => "tokenizer-mismatch",
});
memory_enum_wire!(ManifestIntegrity {
    ManifestIntegrity::Unavailable => "unavailable",
    ManifestIntegrity::Verified => "verified",
    ManifestIntegrity::Unchecked => "unchecked",
    ManifestIntegrity::ManifestMissing => "manifest-missing",
    ManifestIntegrity::ChecksumMismatch => "checksum-mismatch",
    ManifestIntegrity::SignatureInvalid => "signature-invalid",
    ManifestIntegrity::Corrupted => "corrupted",
    ManifestIntegrity::Failed => "failed",
});
memory_enum_wire!(CacheHealth {
    CacheHealth::Healthy => "healthy",
    CacheHealth::Degraded => "degraded",
    CacheHealth::Unavailable => "unavailable",
    CacheHealth::DownloadDisabled => "download-disabled",
    CacheHealth::Corrupted => "corrupted",
    CacheHealth::StorageError => "storage-error",
});
memory_enum_wire!(DownloadStatus {
    DownloadStatus::DownloadDisabled => "download-disabled",
    DownloadStatus::DownloadNotRequested => "download-not-requested",
    DownloadStatus::DownloadInProgress => "download-in-progress",
    DownloadStatus::DownloadComplete => "download-complete",
    DownloadStatus::DownloadFailed => "download-failed",
});
memory_enum_wire!(CacheUnavailableReason {
    CacheUnavailableReason::ModelSourceUnconfigured => "model-source-unconfigured",
    CacheUnavailableReason::ArtifactNotInstalled => "artifact-not-installed",
    CacheUnavailableReason::ManifestUnavailable => "manifest-unavailable",
    CacheUnavailableReason::DownloadDisabled => "download-disabled",
    CacheUnavailableReason::CacheStorageUnavailable => "cache-storage-unavailable",
    CacheUnavailableReason::IntegrityUnverified => "integrity-unverified",
    CacheUnavailableReason::CorruptionDetected => "corruption-detected",
});
memory_enum_wire!(CacheStorageErrorCode {
    CacheStorageErrorCode::CacheRootUnavailable => "cache-root-unavailable",
    CacheStorageErrorCode::ManifestReadFailed => "manifest-read-failed",
    CacheStorageErrorCode::ArtifactReadFailed => "artifact-read-failed",
    CacheStorageErrorCode::MetadataWriteDisabled => "metadata-write-disabled",
    CacheStorageErrorCode::StoragePermissionDenied => "storage-permission-denied",
    CacheStorageErrorCode::QuotaUnavailable => "quota-unavailable",
});
memory_enum_wire!(CacheCorruptionReasonCode {
    CacheCorruptionReasonCode::ManifestMissing => "manifest-missing",
    CacheCorruptionReasonCode::ChecksumMismatch => "checksum-mismatch",
    CacheCorruptionReasonCode::SignatureInvalid => "signature-invalid",
    CacheCorruptionReasonCode::ArtifactMissing => "artifact-missing",
    CacheCorruptionReasonCode::ManifestArtifactMismatch => "manifest-artifact-mismatch",
    CacheCorruptionReasonCode::UnknownIntegrity => "unknown-integrity",
});
memory_enum_wire!(ProviderKind {
    ProviderKind::Cpu => "cpu",
    ProviderKind::DirectMl => "direct-ml",
    ProviderKind::OpenVino => "open-vino",
    ProviderKind::Vulkan => "vulkan",
    ProviderKind::Cuda => "cuda",
    ProviderKind::CoreMl => "core-ml",
    ProviderKind::Npu => "npu",
});
memory_enum_wire!(ModelCacheRootMode {
    ModelCacheRootMode::DevRepoLocal => "dev-repo-local",
    ModelCacheRootMode::AppData => "app-data",
});
memory_enum_wire!(ModelRuntimeServiceRoute {
    ModelRuntimeServiceRoute::Health => "health",
    ModelRuntimeServiceRoute::Models => "models",
    ModelRuntimeServiceRoute::LoadModel => "load-model",
    ModelRuntimeServiceRoute::UnloadModel => "unload-model",
    ModelRuntimeServiceRoute::Chat => "chat",
    ModelRuntimeServiceRoute::Embeddings => "embeddings",
    ModelRuntimeServiceRoute::Rerank => "rerank",
});
memory_enum_wire!(LocalRuntimeKind {
    LocalRuntimeKind::LlamaCpp => "llama-cpp",
    LocalRuntimeKind::OnnxOrt => "onnx-ort",
    LocalRuntimeKind::DeterministicFallback => "deterministic-fallback",
});
memory_enum_wire!(LocalRuntimeArtifactKind {
    LocalRuntimeArtifactKind::Manifest => "manifest",
    LocalRuntimeArtifactKind::Model => "model",
    LocalRuntimeArtifactKind::Tokenizer => "tokenizer",
    LocalRuntimeArtifactKind::Config => "config",
    LocalRuntimeArtifactKind::Adapter => "adapter",
    LocalRuntimeArtifactKind::ExternalData => "external-data",
    LocalRuntimeArtifactKind::Unknown => "unknown",
});
memory_enum_wire!(LocalRuntimeAcceleration {
    LocalRuntimeAcceleration::Auto => "auto",
    LocalRuntimeAcceleration::Cpu => "cpu",
    LocalRuntimeAcceleration::Gpu => "gpu",
    LocalRuntimeAcceleration::Npu => "npu",
});
memory_enum_wire!(RuntimeWorkload {
    RuntimeWorkload::Chat => "chat",
    RuntimeWorkload::Embedding => "embedding",
    RuntimeWorkload::Reranking => "reranking",
});
memory_enum_wire!(RuntimeActivityState {
    RuntimeActivityState::Idle => "idle",
    RuntimeActivityState::Loading => "loading",
    RuntimeActivityState::ChatActive => "chat-active",
    RuntimeActivityState::EmbeddingActive => "embedding-active",
    RuntimeActivityState::RerankingActive => "reranking-active",
    RuntimeActivityState::Paused => "paused",
});
memory_enum_wire!(RuntimeAdmission {
    RuntimeAdmission::Admit => "admit",
    RuntimeAdmission::Queue => "queue",
    RuntimeAdmission::PauseBackgroundThenAdmit => "pause-background-then-admit",
});
memory_enum_wire!(RuntimeOwnershipMode {
    RuntimeOwnershipMode::EnforcerSubprocess => "enforcer-subprocess",
    RuntimeOwnershipMode::EnforcerIsolatedWorker => "enforcer-isolated-worker",
    RuntimeOwnershipMode::EnforcerInProcess => "enforcer-in-process",
    RuntimeOwnershipMode::ExternalServer => "external-server",
    RuntimeOwnershipMode::Unmanaged => "unmanaged",
});
memory_enum_wire!(RuntimeExecutionIsolation {
    RuntimeExecutionIsolation::EnforcerManagedChildProcess => "enforcer-managed-child-process",
    RuntimeExecutionIsolation::EnforcerIsolatedWorkerProcess => "enforcer-isolated-worker-process",
    RuntimeExecutionIsolation::EnforcerInProcessLibrary => "enforcer-in-process-library",
    RuntimeExecutionIsolation::ExternalServerProcess => "external-server-process",
    RuntimeExecutionIsolation::UnmanagedProcess => "unmanaged-process",
});
memory_enum_wire!(RuntimeRequestProtocol {
    RuntimeRequestProtocol::EnforcerWorkerEnv => "enforcer-worker-env",
    RuntimeRequestProtocol::EnforcerStdio => "enforcer-stdio",
    RuntimeRequestProtocol::ExternalHttp => "external-http",
    RuntimeRequestProtocol::None => "none",
});
memory_enum_wire!(RuntimeManagedCapability {
    RuntimeManagedCapability::LoadUnload => "load-unload",
    RuntimeManagedCapability::PauseResumeCancel => "pause-resume-cancel",
    RuntimeManagedCapability::TimeoutKill => "timeout-kill",
    RuntimeManagedCapability::ProviderSelection => "provider-selection",
    RuntimeManagedCapability::CachePolicy => "cache-policy",
    RuntimeManagedCapability::ChatHistoryPolicy => "chat-history-policy",
    RuntimeManagedCapability::WorkloadAdmission => "workload-admission",
});
memory_enum_wire!(OrtWorkerTask {
    OrtWorkerTask::Embedding => "embedding",
    OrtWorkerTask::Reranker => "reranker",
});
memory_enum_wire!(OrtWorkerLifecycleState {
    OrtWorkerLifecycleState::Idle => "idle",
    OrtWorkerLifecycleState::Loading => "loading",
    OrtWorkerLifecycleState::Ready => "ready",
    OrtWorkerLifecycleState::EmbeddingActive => "embedding-active",
    OrtWorkerLifecycleState::RerankingActive => "reranking-active",
    OrtWorkerLifecycleState::PausedEmbedding => "paused-embedding",
    OrtWorkerLifecycleState::PausedReranking => "paused-reranking",
    OrtWorkerLifecycleState::Cancelled => "cancelled",
    OrtWorkerLifecycleState::TimedOut => "timed-out",
    OrtWorkerLifecycleState::Unloaded => "unloaded",
});
memory_enum_wire!(OrtWorkerLifecycleAction {
    OrtWorkerLifecycleAction::Load => "load",
    OrtWorkerLifecycleAction::MarkReady => "mark-ready",
    OrtWorkerLifecycleAction::StartEmbedding => "start-embedding",
    OrtWorkerLifecycleAction::StartReranker => "start-reranker",
    OrtWorkerLifecycleAction::Pause => "pause",
    OrtWorkerLifecycleAction::Resume => "resume",
    OrtWorkerLifecycleAction::Cancel => "cancel",
    OrtWorkerLifecycleAction::TimeoutKill => "timeout-kill",
    OrtWorkerLifecycleAction::Unload => "unload",
});

#[cfg(test)]
mod route_confidence_property_tests {
    use super::RouteConfidence;
    use proptest::{prelude::any, proptest};

    proptest! {
        #[test]
        fn normalized_confidence_is_bounded(value in any::<f64>()) {
            let normalized = RouteConfidence::normalized(value).get();
            assert!(normalized.is_finite());
            assert!((0.0..=1.0).contains(&normalized));
        }
    }
}

#[cfg(test)]
mod retry_delay_tests {
    use super::{
        MemoryGraphEmpty, MemoryGraphNodeCount, MemoryQueueEmpty, MemoryQueueExhausted,
        MemoryQueueLength, MemoryRetryDelay,
    };

    #[test]
    fn constructors_preserve_duration_units() {
        assert_eq!(MemoryRetryDelay::from_millis(50).get().as_millis(), 50);
        assert_eq!(MemoryRetryDelay::from_secs(2).get().as_secs(), 2);
    }

    #[test]
    fn graph_and_queue_measurements_preserve_zero_and_boolean_states() {
        assert_eq!(MemoryGraphNodeCount::ZERO.get(), 0);
        assert!(MemoryGraphEmpty::from(true).is_empty());
        assert!(MemoryQueueExhausted::from(true).is_exhausted());
        assert_eq!(MemoryQueueLength::ZERO.get(), 0);
        assert!(MemoryQueueEmpty::from(true).is_empty());
    }
}
