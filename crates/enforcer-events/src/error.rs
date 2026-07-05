use std::{error::Error, fmt};

use crate::{EventId, EventType, IdempotencyKey, RequestId, SchemaVersion, SubscriberId};

mod format;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventingError {
    EmptyValue {
        field: &'static str,
    },
    InvalidValue {
        field: &'static str,
        value: String,
    },
    InvalidVersion,
    PayloadEncode {
        reason: String,
    },
    PayloadDecode {
        event_type: EventType,
        reason: String,
    },
    ContractMismatch {
        expected: EventType,
        received: EventType,
        expected_schema_version: SchemaVersion,
        received_schema_version: SchemaVersion,
    },
    DuplicateEventContract {
        event_type: EventType,
    },
    DuplicateSubscriber {
        subscriber_id: SubscriberId,
    },
    HandlerPanicked {
        subscriber_id: SubscriberId,
    },
    HandlerTimedOut {
        subscriber_id: SubscriberId,
    },
    InvalidHandlerPolicy {
        reason: String,
    },
    InvalidQueuePolicy {
        reason: String,
    },
    NoSubscriber {
        event_type: EventType,
    },
    QueueCapacityExceeded {
        event_type: EventType,
        capacity: usize,
    },
    EventDeadlineExpired {
        event_type: EventType,
    },
    DuplicateEventId {
        event_id: EventId,
    },
    DuplicateInFlight {
        idempotency_key: IdempotencyKey,
    },
    DuplicateIdempotencyKey {
        idempotency_key: IdempotencyKey,
    },
    InvalidRequestOptions {
        reason: String,
    },
    DuplicateRequest {
        request_id: RequestId,
    },
    RequestTimedOut {
        request_id: RequestId,
    },
    RequestResponseEncode {
        request_id: RequestId,
        reason: String,
    },
    RequestResponseDecode {
        request_id: RequestId,
        reason: String,
    },
    /// Both the publish outcome and the response payload were expected to
    /// be populated by the time the request/response flow completes (every
    /// code path either fills both before returning `Ok` or returns early
    /// with an `Err`); this is only reachable if that internal invariant is
    /// ever violated by a future change.
    RequestIncomplete {
        request_id: RequestId,
    },
    BusShutdown,
    JournalIo {
        path: String,
        reason: String,
    },
    JournalEncode {
        reason: String,
    },
    JournalDecode {
        reason: String,
    },
    JournalCorruptLine {
        line: usize,
        reason: String,
    },
    /// The per-journal append gate (a semaphore bounding concurrent
    /// appends to one) was closed; this crate never closes it, so this is
    /// only reachable if that invariant is ever violated by a future
    /// change.
    JournalAppendGateClosed,
    ReplayActionNotAllowed {
        event_type: EventType,
    },
    RegistrarDisposed,
}

impl EventingError {
    pub(crate) fn empty_value(field: &'static str) -> Self {
        Self::EmptyValue { field }
    }

    pub(crate) fn invalid_value(field: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidValue {
            field,
            value: value.into(),
        }
    }

    pub(crate) fn payload_encode(error: &serde_json::Error) -> Self {
        Self::PayloadEncode {
            reason: error.to_string(),
        }
    }

    pub(crate) fn payload_decode(event_type: EventType, error: &serde_json::Error) -> Self {
        Self::PayloadDecode {
            event_type,
            reason: error.to_string(),
        }
    }

    pub(crate) fn journal_io(path: String, error: &std::io::Error) -> Self {
        Self::JournalIo {
            path,
            reason: error.to_string(),
        }
    }

    pub(crate) fn journal_encode(error: &serde_json::Error) -> Self {
        Self::JournalEncode {
            reason: error.to_string(),
        }
    }
}

impl fmt::Display for EventingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        format::fmt_eventing_error(self, formatter)
    }
}

impl Error for EventingError {}
