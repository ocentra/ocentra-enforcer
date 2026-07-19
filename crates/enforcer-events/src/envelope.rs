use enforcer_domain::events_types::{
    AggregateKey, CausationId, CorrelationId, EventCustody, EventErrorField, EventErrorReason,
    EventId, EventPriority, EventType, IdempotencyKey, RecordedAt, RuntimeInstanceId, RuntimeRole,
    SchemaVersion, SourceComponent, SourceService, TargetHandler,
};

use crate::{clock::EventClockInstant, error::EventingError};

/// Domain contract implemented by every event payload.
pub trait DomainEvent: Clone + Send + Sync + 'static {
    fn contract(&self) -> Result<EventContract, EventingError>;
    fn aggregate_key(&self) -> Result<AggregateKey, EventingError>;
    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError>;
}

/// Typed identity and version of an event contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContract {
    pub event_type: EventType,
    pub schema_version: SchemaVersion,
}
impl EventContract {
    /// Executes the new event-runtime operation.
    pub fn new(event_type: EventType, schema_version: SchemaVersion) -> Self {
        Self {
            event_type,
            schema_version,
        }
    }
}

/// Typed origin metadata for an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSource {
    pub custody: EventCustody,
    pub role: RuntimeRole,
    pub service: SourceService,
    pub component: SourceComponent,
    pub instance_id: RuntimeInstanceId,
}
impl EventSource {
    /// Executes the new event-runtime operation.
    pub fn new(
        custody: EventCustody,
        role: RuntimeRole,
        service: SourceService,
        component: SourceComponent,
        instance_id: RuntimeInstanceId,
    ) -> Self {
        Self {
            custody,
            role,
            service,
            component,
            instance_id,
        }
    }
}

/// Typed metadata used to build an event envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMetadata {
    pub event_id: EventId,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub source: EventSource,
    pub observed_at: RecordedAt,
    pub target_handler: Option<TargetHandler>,
    pub priority: EventPriority,
    pub deadline: Option<EventClockInstant>,
}
impl EventMetadata {
    /// Executes the new event-runtime operation.
    pub fn new(correlation_id: CorrelationId, source: EventSource) -> Result<Self, EventingError> {
        let observed_at = chrono::Utc::now().to_rfc3339();
        // CLONE-JUSTIFICATION: parsing consumes its candidate while the error retains the rejected timestamp.
        let observed_at = RecordedAt::try_new(observed_at.clone()).map_err(|_decode_error| {
            EventingError::invalid_value(
                EventErrorField::from_diagnostic("recorded_at"),
                EventErrorReason::from_diagnostic(observed_at),
            )
        })?;
        Ok(Self {
            event_id: EventId::generated(),
            correlation_id,
            causation_id: None,
            source,
            observed_at,
            target_handler: None,
            priority: EventPriority::Normal,
            deadline: None,
        })
    }
    /// Executes the from parts event-runtime operation.
    pub fn from_parts(
        event_id: EventId,
        correlation_id: CorrelationId,
        source: EventSource,
        observed_at: RecordedAt,
        target_handler: Option<TargetHandler>,
    ) -> Self {
        Self {
            event_id,
            correlation_id,
            causation_id: None,
            source,
            observed_at,
            target_handler,
            priority: EventPriority::Normal,
            deadline: None,
        }
    }
    /// Executes the with causation id event-runtime operation.
    pub fn with_causation_id(mut self, causation_id: CausationId) -> Self {
        self.causation_id = Some(causation_id);
        self
    }
    /// Executes the with priority event-runtime operation.
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }
    /// Executes the with deadline event-runtime operation.
    pub fn with_deadline(mut self, deadline: EventClockInstant) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

/// Live typed event frame; persistence and JSON live at the boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct EventFrame<E> {
    pub contract: EventContract,
    pub event_id: EventId,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub aggregate_key: AggregateKey,
    pub idempotency_key: IdempotencyKey,
    pub source: EventSource,
    pub observed_at: RecordedAt,
    pub target_handler: Option<TargetHandler>,
    pub priority: EventPriority,
    pub deadline: Option<EventClockInstant>,
    pub payload: E,
}

impl<E> EventFrame<E>
where
    E: DomainEvent,
{
    /// Executes the from event event-runtime operation.
    pub fn from_event(payload: E, metadata: EventMetadata) -> Result<Self, EventingError> {
        Ok(Self {
            contract: payload.contract()?,
            event_id: metadata.event_id,
            correlation_id: metadata.correlation_id,
            causation_id: metadata.causation_id,
            aggregate_key: payload.aggregate_key()?,
            idempotency_key: payload.idempotency_key()?,
            source: metadata.source,
            observed_at: metadata.observed_at,
            target_handler: metadata.target_handler,
            priority: metadata.priority,
            deadline: metadata.deadline,
            payload,
        })
    }
}
// INVALID-INPUT-TEST: envelope boundary tests reject malformed metadata, zero
// schema versions, and stored/live contract mismatches.
// ROUNDTRIP-TEST: `tests/unit/envelope.rs` proves live and stored envelope
// metadata survives canonical persistence conversion.
// `EventFrame` is the live typed domain frame, not a transport DTO; its
// explicit `store` boundary conversion is implemented in envelope_persistence.
