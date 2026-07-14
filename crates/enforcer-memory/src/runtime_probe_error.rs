//! Typed failure boundary for the X06 runtime probe entry point.

/// Captures every fallible source used while emitting a runtime probe proof.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeProbeError {
    /// Filesystem or standard-output operation failed.
    #[error("runtime probe I/O failed: {0}")]
    Io(#[from] std::io::Error),

    /// Runtime proof JSON could not be encoded or decoded.
    #[error("runtime probe JSON failed: {0}")]
    Json(#[from] serde_json::Error),

    /// A memory runtime operation returned its typed crate error.
    #[error(transparent)]
    Memory(#[from] crate::error::MemoryError),
}
