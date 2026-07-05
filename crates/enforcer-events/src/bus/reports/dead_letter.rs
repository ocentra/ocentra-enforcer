use serde::{Deserialize, Serialize};

use crate::{
    AggregateKey, CorrelationId, DomainEvent, EventContract, EventId, EventType, EventingError,
    IdempotencyKey, SchemaVersion, StoredEventEnvelope, SubscriberId, TargetHandler,
};

const DEAD_LETTER_RECORDED_EVENT_TYPE: &str = "eventing.dead_letter.recorded";
const DEAD_LETTER_RECORDED_SCHEMA_VERSION: u16 = 1;
const DEAD_LETTER_IDEMPOTENCY_PREFIX: &str = "dead-letter";
const DEAD_LETTER_IDEMPOTENCY_SEPARATOR: &str = "-";

pub fn dead_letter_recorded_event_type() -> Result<EventType, EventingError> {
    EventType::parse(DEAD_LETTER_RECORDED_EVENT_TYPE)
}

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
        report.error.clone().map(|error| Self {
            envelope: stored.clone(),
            subscriber_id: Some(report.subscriber_id.clone()),
            target_handler: Some(report.target_handler.clone()),
            reason: report.outcome.dead_letter_reason(),
            error,
        })
    }

    pub(crate) fn for_queue(
        stored: &StoredEventEnvelope,
        reason: DeadLetterReason,
        error: EventingError,
    ) -> Self {
        Self {
            envelope: stored.clone(),
            subscriber_id: None,
            target_handler: None,
            reason,
            error,
        }
    }

    pub fn as_event(&self) -> DeadLetterEvent {
        DeadLetterEvent {
            original_event_id: self.envelope.event_id.clone(),
            original_event_type: self.envelope.contract.event_type.clone(),
            original_correlation_id: self.envelope.correlation_id.clone(),
            reason: self.reason,
            subscriber_id: self.subscriber_id.clone(),
            target_handler: self.target_handler.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeadLetterReason {
    HandlerFailed,
    HandlerTimedOut,
    HandlerDeadlineExpired,
    HandlerPanicked,
    NoSubscriber,
    QueueOverflow,
    QueueExpired,
    DeadlineExpired,
    Shutdown,
}

impl DeadLetterReason {
    pub(crate) fn idempotency_label(self) -> &'static str {
        match self {
            Self::HandlerFailed => "handler-failed",
            Self::HandlerTimedOut => "handler-timed-out",
            Self::HandlerDeadlineExpired => "handler-deadline-expired",
            Self::HandlerPanicked => "handler-panicked",
            Self::NoSubscriber => "no-subscriber",
            Self::QueueOverflow => "queue-overflow",
            Self::QueueExpired => "queue-expired",
            Self::DeadlineExpired => "deadline-expired",
            Self::Shutdown => "shutdown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
            SchemaVersion::new(DEAD_LETTER_RECORDED_SCHEMA_VERSION)?,
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        AggregateKey::parse(self.original_event_id.as_str())
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        let mut value = String::from(DEAD_LETTER_IDEMPOTENCY_PREFIX);
        value.push_str(DEAD_LETTER_IDEMPOTENCY_SEPARATOR);
        value.push_str(self.original_event_id.as_str());
        value.push_str(DEAD_LETTER_IDEMPOTENCY_SEPARATOR);
        value.push_str(self.reason.idempotency_label());
        IdempotencyKey::parse(value)
    }
}
