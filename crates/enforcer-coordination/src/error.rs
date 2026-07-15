//! Coordination-crate error type. Distinct from `enforcer_domain::boundary::decode_error::DecodeError`
//! because coordination errors are operational (claim conflicts, ledger
//! corruption, IO) rather than pure decode-at-boundary failures.

use std::fmt;

#[derive(Debug)]
pub enum CoordinationError {
    /// A value failed a coordination-local identity/format check.
    Invalid { field: &'static str, value: String },
    /// Underlying filesystem/IO failure.
    Io(std::io::Error),
    /// Ledger event (de)serialization failure.
    Serde(serde_json::Error),
    /// A domain-layer `enforcer-domain` decode failure (e.g. bad `HubName`/`LaneId`).
    Decode(enforcer_domain::boundary::decode_error::DecodeError),
    /// The requested claim/release/closeout could not proceed for a stated reason.
    Rejected(String),
    /// An event's stored hash does not match its recomputed wire hash.
    HashMismatch { event_id: String },
    /// The append-only stream lock could not be acquired within the deadline.
    LockTimeout { path: String },
}

impl CoordinationError {
    pub fn invalid(field: &'static str, value: impl Into<String>) -> Self {
        Self::Invalid {
            field,
            value: value.into(),
        }
    }

    pub fn rejected(message: impl Into<String>) -> Self {
        Self::Rejected(message.into())
    }
}

impl fmt::Display for CoordinationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { field, value } => {
                write!(f, "invalid {field}: {value:?}")
            }
            Self::Io(err) => write!(f, "coordination IO error: {err}"),
            Self::Serde(err) => write!(f, "coordination serde error: {err}"),
            Self::Decode(err) => write!(f, "coordination decode error: {err}"),
            Self::Rejected(message) => write!(f, "{message}"),
            Self::HashMismatch { event_id } => {
                write!(f, "event {event_id} hash mismatch")
            }
            Self::LockTimeout { path } => {
                write!(f, "timed out acquiring stream lock {path}")
            }
        }
    }
}

impl std::error::Error for CoordinationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Serde(err) => Some(err),
            Self::Decode(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CoordinationError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for CoordinationError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

impl From<enforcer_domain::boundary::decode_error::DecodeError> for CoordinationError {
    fn from(err: enforcer_domain::boundary::decode_error::DecodeError) -> Self {
        Self::Decode(err)
    }
}

pub type Result<T> = std::result::Result<T, CoordinationError>;
