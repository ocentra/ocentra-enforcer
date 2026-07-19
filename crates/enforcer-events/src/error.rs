use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::events_types::{
    EventCount, EventErrorField, EventErrorPath, EventErrorReason, EventId, EventType,
    IdempotencyKey, JournalPath, RequestId, SchemaVersion, SubscriberId,
};

/// Event-runtime variants for eventing error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EventingError {
    #[error("empty eventing value: {field}", field = field.as_str())]
    EmptyValue { field: EventErrorField },
    #[error("invalid eventing value for {field}: {value}", field = field.as_str())]
    InvalidValue {
        field: EventErrorField,
        value: EventErrorReason,
    },
    #[error("event schema version must be nonzero")]
    InvalidVersion,
    #[error("payload encode failed: {reason}")]
    PayloadEncode { reason: EventErrorReason },
    #[error("payload decode failed for {event_type}: {reason}", event_type = event_type.as_str())]
    PayloadDecode {
        event_type: EventType,
        reason: EventErrorReason,
    },
    #[error(
        "event contract mismatch: expected {expected}@{expected_schema_version}, received {received}@{received_schema_version}",
        expected = expected.as_str(),
        expected_schema_version = expected_schema_version.as_nonzero().get(),
        received = received.as_str(),
        received_schema_version = received_schema_version.as_nonzero().get(),
    )]
    ContractMismatch {
        expected: EventType,
        received: EventType,
        expected_schema_version: SchemaVersion,
        received_schema_version: SchemaVersion,
    },
    #[error("duplicate event contract: {event_type}", event_type = event_type.as_str())]
    DuplicateEventContract { event_type: EventType },
    #[error("duplicate subscriber: {subscriber_id}", subscriber_id = subscriber_id.as_str())]
    DuplicateSubscriber { subscriber_id: SubscriberId },
    #[error("event handler panicked: {subscriber_id}", subscriber_id = subscriber_id.as_str())]
    HandlerPanicked { subscriber_id: SubscriberId },
    #[error("event handler timed out: {subscriber_id}", subscriber_id = subscriber_id.as_str())]
    HandlerTimedOut { subscriber_id: SubscriberId },
    #[error("invalid event handler policy: {reason}")]
    InvalidHandlerPolicy { reason: EventErrorReason },
    #[error("invalid event queue policy: {reason}")]
    InvalidQueuePolicy { reason: EventErrorReason },
    #[error("no subscriber for event type: {event_type}", event_type = event_type.as_str())]
    NoSubscriber { event_type: EventType },
    #[error(
        "event queue capacity exceeded for {event_type}: {capacity}",
        event_type = event_type.as_str(),
        capacity = crate::boundary::event_values::event_count_value(*capacity),
    )]
    QueueCapacityExceeded {
        event_type: EventType,
        capacity: EventCount,
    },
    #[error("event deadline expired for {event_type}", event_type = event_type.as_str())]
    EventDeadlineExpired { event_type: EventType },
    #[error("duplicate event id: {event_id}", event_id = event_id.as_str())]
    DuplicateEventId { event_id: EventId },
    #[error("duplicate in-flight event: {idempotency_key}", idempotency_key = idempotency_key.as_str())]
    DuplicateInFlight { idempotency_key: IdempotencyKey },
    #[error("duplicate idempotency key: {idempotency_key}", idempotency_key = idempotency_key.as_str())]
    DuplicateIdempotencyKey { idempotency_key: IdempotencyKey },
    #[error("invalid event request options: {reason}")]
    InvalidRequestOptions { reason: EventErrorReason },
    #[error("duplicate request id: {request_id}", request_id = request_id.as_str())]
    DuplicateRequest { request_id: RequestId },
    #[error("event request timed out: {request_id}", request_id = request_id.as_str())]
    RequestTimedOut { request_id: RequestId },
    #[error(
        "event request response encode failed for {request_id}: {reason}",
        request_id = request_id.as_str(),
    )]
    RequestResponseEncode {
        request_id: RequestId,
        reason: EventErrorReason,
    },
    #[error(
        "event request response decode failed for {request_id}: {reason}",
        request_id = request_id.as_str(),
    )]
    RequestResponseDecode {
        request_id: RequestId,
        reason: EventErrorReason,
    },
    /// Both the publish outcome and the response payload were expected to
    /// be populated by the time the request/response flow completes (every
    /// code path either fills both before returning `Ok` or returns early
    /// with an `Err`); this is only reachable if that internal invariant is
    /// ever violated by a future change.
    #[error(
        "event request completed without a response for {request_id}",
        request_id = request_id.as_str(),
    )]
    RequestIncomplete { request_id: RequestId },
    #[error("event bus is shut down")]
    BusShutdown,
    #[error("event journal io failed for {path}: {reason}")]
    JournalIo {
        path: EventErrorPath,
        reason: EventErrorReason,
    },
    #[error("event journal encode failed: {reason}")]
    JournalEncode { reason: EventErrorReason },
    #[error("event journal decode failed: {reason}")]
    JournalDecode { reason: EventErrorReason },
    #[error(
        "event journal corrupt line {line}: {reason}",
        line = crate::boundary::event_values::event_count_value(*line),
    )]
    JournalCorruptLine {
        line: EventCount,
        reason: EventErrorReason,
    },
    /// The per-journal append gate (a semaphore bounding concurrent
    /// appends to one) was closed; this crate never closes it, so this is
    /// only reachable if that invariant is ever violated by a future
    /// change.
    #[error("event journal append gate is closed")]
    JournalAppendGateClosed,
    #[error(
        "event replay action handlers are not allowed for {event_type}",
        event_type = event_type.as_str(),
    )]
    ReplayActionNotAllowed { event_type: EventType },
    #[error("event registrar is disposed")]
    RegistrarDisposed,
}

impl EventingError {
    pub(crate) fn invalid_value(field: EventErrorField, value: EventErrorReason) -> Self {
        Self::InvalidValue { field, value }
    }

    pub(crate) fn payload_encode(error: &serde_json::Error) -> Self {
        Self::PayloadEncode {
            // ALLOC-JUSTIFICATION: EventingError owns the serializer diagnostic after the borrowed serde error expires.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        }
    }

    pub(crate) fn payload_decode(event_type: EventType, error: &serde_json::Error) -> Self {
        Self::PayloadDecode {
            event_type,
            // ALLOC-JUSTIFICATION: EventingError owns the deserializer diagnostic after the borrowed serde error expires.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        }
    }

    pub(crate) fn journal_io(path: &JournalPath, error: &std::io::Error) -> Self {
        Self::JournalIo {
            path: EventErrorPath::from_diagnostic(path.as_str()),
            // ALLOC-JUSTIFICATION: EventingError owns the I/O diagnostic after the borrowed std error expires.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        }
    }

    pub(crate) fn journal_encode(error: &serde_json::Error) -> Self {
        Self::JournalEncode {
            // ALLOC-JUSTIFICATION: EventingError owns the journal serializer diagnostic after the borrowed serde error expires.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        }
    }
}

impl From<DecodeError> for EventingError {
    fn from(error: DecodeError) -> Self {
        Self::InvalidValue {
            field: EventErrorField::from_diagnostic("decoded_value"),
            // ALLOC-JUSTIFICATION: EventingError owns the decode diagnostic independently of the consumed DecodeError.
            value: EventErrorReason::from_diagnostic(error.to_string()),
        }
    }
}
