//! `enforcer-memory` error type: the single fail-closed surface every
//! store/log/schema operation in this crate returns through. Every
//! variant names the failure mode explicitly (no bare `anyhow::Error`)
//! so callers can match on the reason a store or log operation failed
//! rather than parsing a message string.

use enforcer_domain::memory_types::{
    MemoryErrorArtifactId, MemoryErrorDigest, MemoryErrorLineIndex, MemoryErrorLogLength,
    MemoryErrorManifestWatermark, MemoryErrorOperation, MemoryErrorPath, MemoryErrorReason,
    MemoryErrorRowCount, MemoryQuarantineReason, MemoryQuarantineRowIndex,
};

/// A single quarantined row: what went wrong and where it now lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantinedRow {
    /// Zero-based line/row index in the original source.
    pub index: MemoryQuarantineRowIndex,
    /// Human-readable reason the row was quarantined.
    pub reason: MemoryQuarantineReason,
}

/// All `enforcer-memory` store/log/schema errors.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// Opening a store for a project directory that has never been
    /// initialized (no existing store root) — the store MUST NOT create
    /// a fresh "ghost" database for a project it does not already know
    /// about. Callers that intend to create a new project store must go
    /// through an explicit `Store::init` call.
    #[error("no memory store exists for project at {root} -- refusing to create a ghost database; call Store::init explicitly to create one")]
    UnknownProject { root: MemoryErrorPath },

    /// A path failed `enforcer_domain::paths` normalization/validation.
    #[error("invalid path {path:?}: {source}")]
    InvalidPath {
        path: MemoryErrorPath,
        #[source]
        source: enforcer_domain::boundary::decode_error::DecodeError,
    },

    /// The append-only log's hash chain failed verification against its
    /// independently-persisted `.chain` sidecar (L37: never re-derive the
    /// expected digest from the same file being checked).
    #[error("log tamper detected in {path:?} at line {line_index}: recorded digest {recorded} does not match expected {expected}")]
    ChainTamper {
        path: MemoryErrorPath,
        line_index: MemoryErrorLineIndex,
        recorded: MemoryErrorDigest,
        expected: MemoryErrorDigest,
    },

    /// A row could not be parsed/validated and was quarantined rather
    /// than silently dropped or allowed to poison the rest of the log.
    #[error("{count} row(s) quarantined from {path:?}: {}", format_rows(rows))]
    Quarantined {
        path: MemoryErrorPath,
        count: MemoryErrorRowCount,
        rows: Vec<QuarantinedRow>,
    },

    /// An index manifest's recorded high-watermark is behind the log it
    /// claims to index -- the index is stale and must be rebuilt before
    /// being trusted for reads.
    #[error("index manifest {path:?} is stale: high-watermark {manifest_watermark} < log length {log_length}")]
    StaleIndex {
        path: MemoryErrorPath,
        manifest_watermark: MemoryErrorManifestWatermark,
        log_length: MemoryErrorLogLength,
    },

    /// The underlying SQLite operational store returned an error.
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON (de)serialization failure at a store/log boundary.
    #[error("json codec failed: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O failure.
    #[error("io error at {path:?}: {source}")]
    Io {
        path: MemoryErrorPath,
        #[source]
        source: std::io::Error,
    },

    /// A content-addressed artifact's recorded digest did not match the
    /// digest recomputed from its bytes at read time.
    #[error("artifact {id} content digest mismatch: manifest says {expected}, content hashes to {actual}")]
    ArtifactDigestMismatch {
        id: MemoryErrorArtifactId,
        expected: MemoryErrorDigest,
        actual: MemoryErrorDigest,
    },

    /// Local model runtime/cache validation failed. This is deliberately
    /// a typed crate error, not a logged warning, because x06 must learn
    /// from model failures and must never silently claim a loaded model.
    #[error("model runtime {operation} failed: {reason}")]
    ModelRuntime {
        operation: MemoryErrorOperation,
        reason: MemoryErrorReason,
    },

    /// An internal append/replay invariant failed. This is returned as a
    /// typed error instead of panicking so proof runs can record the
    /// failure and keep the harness process alive.
    #[error("internal invariant failed in {operation}: {reason}")]
    InternalInvariant {
        operation: MemoryErrorOperation,
        reason: MemoryErrorReason,
    },
}

fn format_rows(rows: &[QuarantinedRow]) -> MemoryErrorReason {
    rows.iter()
        .map(|r| format!("line {}: {}", r.index, r.reason.as_str()))
        .collect::<Vec<_>>()
        .join("; ")
        .into()
}

/// This crate's `Result` alias.
pub type Result<T> = std::result::Result<T, MemoryError>;
