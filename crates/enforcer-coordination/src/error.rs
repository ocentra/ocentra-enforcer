//! Coordination-crate error type. Distinct from `enforcer_domain::boundary::decode_error::DecodeError`
//! because coordination errors are operational (claim conflicts, ledger
//! corruption, IO) rather than pure decode-at-boundary failures.

/// Operational failures produced while coordinating claims and ledger state.
#[derive(Debug, thiserror::Error)]
pub enum CoordinationError {
    /// Underlying filesystem/IO failure.
    #[error("coordination IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Ledger event (de)serialization failure.
    #[error("coordination serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Invalid caller-supplied glob pattern.
    #[error("coordination glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
    /// Filesystem error encountered while expanding a valid glob.
    #[error("coordination glob expansion error: {0}")]
    Glob(#[from] glob::GlobError),
    /// A domain-layer `enforcer-domain` decode failure (e.g. bad `HubName`/`LaneId`).
    #[error("coordination decode error: {0}")]
    Decode(#[from] enforcer_domain::boundary::decode_error::DecodeError),
    /// The requested claim/release/closeout could not proceed for a stated reason.
    #[error("{0}")]
    Rejected(enforcer_domain::coordination_types::CoordinationRejection),
    /// An event's stored hash does not match its recomputed wire hash.
    #[error("event {event_id} hash mismatch")]
    HashMismatch {
        event_id: enforcer_domain::coordination_types::ClaimEventId,
    },
    /// The append-only stream lock could not be acquired within the deadline.
    #[error("timed out acquiring stream lock {path}")]
    LockTimeout {
        path: enforcer_domain::coordination_types::CoordinationLedgerPath,
    },
}

impl CoordinationError {
    /// Construct an operational rejection from a validated explanation.
    pub fn rejected(message: enforcer_domain::coordination_types::CoordinationRejection) -> Self {
        Self::Rejected(message)
    }
}

/// Coordination operation result using the crate's canonical error type.
pub type Result<T> = std::result::Result<T, CoordinationError>;
