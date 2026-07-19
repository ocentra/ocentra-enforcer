use enforcer_domain::events_types::{
    AggregateKey, CorrelationId, DeadLetterReason, EventErrorField, EventErrorReason, EventId,
    EventType, IdempotencyKey, SchemaVersion, SubscriberId, TargetHandler,
};
use std::num::NonZeroU16;

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{
    envelope::{DomainEvent, EventContract},
    error::EventingError,
};

const DEAD_LETTER_RECORDED_EVENT_TYPE: &str = "eventing.dead_letter.recorded";
const DEAD_LETTER_RECORDED_SCHEMA_VERSION: u16 = 1;
const DEAD_LETTER_IDEMPOTENCY_PREFIX: &str = "dead-letter";
const DEAD_LETTER_IDEMPOTENCY_SEPARATOR: &str = "-";

/// Executes the dead letter recorded event type event-runtime operation.
pub fn dead_letter_recorded_event_type() -> Result<EventType, EventingError> {
    // ALLOC-JUSTIFICATION: the validated event type is retained by the dead-letter contract.
    EventType::try_new(DEAD_LETTER_RECORDED_EVENT_TYPE.to_owned()).map_err(|_decode_error| {
        EventingError::invalid_value(
            EventErrorField::from_diagnostic("event_type".to_owned()),
            EventErrorReason::from_diagnostic(DEAD_LETTER_RECORDED_EVENT_TYPE.to_owned()),
        )
    })
}

/// Event-runtime data for dead letter.
#[derive(Clone, Debug, PartialEq)]
pub struct DeadLetter {
    pub envelope: StoredEventEnvelope,
    pub subscriber_id: Option<SubscriberId>,
    pub target_handler: Option<TargetHandler>,
    pub reason: DeadLetterReason,
    pub error: EventingError,
}

impl DeadLetter {
    pub(super) fn for_handler(
        stored: &StoredEventEnvelope,
        report: &super::handler::HandlerReport,
    ) -> Option<Self> {
        // CLONE-JUSTIFICATION: the dead-letter report owns its error after the borrowed handler report is released.
        report.error.clone().map(|error| Self {
            // CLONE-JUSTIFICATION: the journaled dead letter owns its envelope after the borrowed storage record is released.
            envelope: stored.clone(),
            // CLONE-JUSTIFICATION: the dead-letter journal retains the subscriber identity independently of the handler report.
            subscriber_id: Some(report.subscriber_id.clone()),
            // CLONE-JUSTIFICATION: the dead-letter journal retains the target handler independently of the handler report.
            target_handler: Some(report.target_handler.clone()),
            reason: super::handler::dead_letter_reason(report.outcome),
            error,
        })
    }

    pub(crate) fn for_queue(
        stored: &StoredEventEnvelope,
        reason: DeadLetterReason,
        error: EventingError,
    ) -> Self {
        Self {
            // CLONE-JUSTIFICATION: the queued dead letter owns its envelope after the borrowed storage record is released.
            envelope: stored.clone(),
            subscriber_id: None,
            target_handler: None,
            reason,
            error,
        }
    }

    /// Executes the as event event-runtime operation.
    pub fn as_event(&self) -> DeadLetterEvent {
        DeadLetterEvent {
            // CLONE-JUSTIFICATION: the published domain event must own the original event id beyond this borrowed journal entry.
            original_event_id: self.envelope.event_id.clone(),
            // CLONE-JUSTIFICATION: the published domain event must own the original event type beyond this borrowed journal entry.
            original_event_type: self.envelope.contract.event_type.clone(),
            // CLONE-JUSTIFICATION: the published domain event must own the original correlation id beyond this borrowed journal entry.
            original_correlation_id: self.envelope.correlation_id.clone(),
            reason: self.reason,
            // CLONE-JUSTIFICATION: the published domain event must own the optional subscriber identity beyond this borrowed journal entry.
            subscriber_id: self.subscriber_id.clone(),
            // CLONE-JUSTIFICATION: the published domain event must own the optional target handler beyond this borrowed journal entry.
            target_handler: self.target_handler.clone(),
        }
    }
}

/// Event-runtime data for dead letter event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeadLetterEvent {
    pub original_event_id: EventId,
    pub original_event_type: EventType,
    pub original_correlation_id: CorrelationId,
    pub reason: DeadLetterReason,
    pub subscriber_id: Option<SubscriberId>,
    pub target_handler: Option<TargetHandler>,
}

impl DomainEvent for DeadLetterEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            dead_letter_recorded_event_type()?,
            SchemaVersion::try_new(
                NonZeroU16::new(DEAD_LETTER_RECORDED_SCHEMA_VERSION)
                    .ok_or(EventingError::InvalidVersion)?,
            ),
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        // ALLOC-JUSTIFICATION: the aggregate key is retained independently of the source event id.
        AggregateKey::try_new(self.original_event_id.as_str().to_owned()).map_err(|_decode_error| {
            EventingError::invalid_value(
                EventErrorField::from_diagnostic("aggregate_key".to_owned()),
                EventErrorReason::from_diagnostic(self.original_event_id.as_str().to_owned()),
            )
        })
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        let mut value = String::from(DEAD_LETTER_IDEMPOTENCY_PREFIX);
        value.push_str(DEAD_LETTER_IDEMPOTENCY_SEPARATOR);
        value.push_str(self.original_event_id.as_str());
        value.push_str(DEAD_LETTER_IDEMPOTENCY_SEPARATOR);
        value.push_str(match self.reason {
            DeadLetterReason::HandlerFailed => "handler-failed",
            DeadLetterReason::HandlerTimedOut => "handler-timed-out",
            DeadLetterReason::HandlerDeadlineExpired => "handler-deadline-expired",
            DeadLetterReason::HandlerPanicked => "handler-panicked",
            DeadLetterReason::NoSubscriber => "no-subscriber",
            DeadLetterReason::QueueOverflow => "queue-overflow",
            DeadLetterReason::QueueExpired => "queue-expired",
            DeadLetterReason::DeadlineExpired => "deadline-expired",
            DeadLetterReason::Shutdown => "shutdown",
        });
        // CLONE-JUSTIFICATION: parsing consumes its candidate while failure reporting preserves the exact rejected key.
        // CLONE-JUSTIFICATION: validation consumes the candidate while diagnostics retain it on failure.
        IdempotencyKey::try_new(value.clone()).map_err(|_decode_error| {
            EventingError::invalid_value(
                // ALLOC-JUSTIFICATION: the typed error owns the canonical diagnostic field after conversion fails.
                EventErrorField::from_diagnostic("idempotency_key".to_owned()),
                EventErrorReason::from_diagnostic(value),
            )
        })
    }
}
// INVALID-INPUT-TEST: dead-letter contract tests reject malformed event and
// idempotency identities before publication.
