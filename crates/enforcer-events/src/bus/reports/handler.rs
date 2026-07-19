use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{error::EventingError, queue::policy::QueueReport};
use enforcer_domain::events_types::{
    CorrelationId, DeadLetterReason, EventCount, EventId, EventType, HandlerOutcome, SubscriberId,
    TargetHandler,
};

use super::DispatchMode;

pub(crate) fn dead_letter_reason(outcome: HandlerOutcome) -> DeadLetterReason {
    match outcome {
        HandlerOutcome::Handled | HandlerOutcome::Failed => DeadLetterReason::HandlerFailed,
        HandlerOutcome::TimedOut => DeadLetterReason::HandlerTimedOut,
        HandlerOutcome::DeadlineExpired => DeadLetterReason::HandlerDeadlineExpired,
        HandlerOutcome::Panicked => DeadLetterReason::HandlerPanicked,
    }
}

/// Event-runtime data for handler report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerReport {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
    pub outcome: HandlerOutcome,
    pub error: Option<EventingError>,
    pub attempts: EventCount,
    pub trace: EventTraceFields,
}

/// Which subscriber handled (or was asked to handle) an event -- grouped so
/// `HandlerReport::new` takes one cohesive parameter instead of two
/// independent ones that are always supplied together.
pub(crate) struct HandlerIdentity {
    pub(crate) subscriber_id: SubscriberId,
    pub(crate) target_handler: TargetHandler,
}

impl HandlerReport {
    pub(crate) fn new(
        stored: &StoredEventEnvelope,
        identity: HandlerIdentity,
        outcome: HandlerOutcome,
        error: Option<EventingError>,
        attempts: EventCount,
    ) -> Self {
        let HandlerIdentity {
            subscriber_id,
            target_handler,
        } = identity;
        // CLONE-JUSTIFICATION: the report trace owns immutable identity values after the stored envelope and handler identity are released.
        let trace = EventTraceFields {
            event_id: stored.event_id.clone(),
            event_type: stored.contract.event_type.clone(),
            correlation_id: stored.correlation_id.clone(),
            // CLONE-JUSTIFICATION: the report owns subscriber identity while the dispatch context retains it.
            subscriber_id: subscriber_id.clone(),
            // CLONE-JUSTIFICATION: the report owns handler identity while the dispatch context retains it.
            target_handler: target_handler.clone(),
            outcome,
            attempts,
        };
        Self {
            subscriber_id,
            target_handler,
            outcome,
            error,
            attempts,
            trace,
        }
    }
}

/// Event-runtime data for event trace fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTraceFields {
    pub event_id: EventId,
    pub event_type: EventType,
    pub correlation_id: CorrelationId,
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
    pub outcome: HandlerOutcome,
    pub attempts: EventCount,
}

/// Event-runtime data for publish report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishReport {
    pub event_id: EventId,
    pub event_type: EventType,
    pub dispatch_mode: DispatchMode,
    pub queue_report: QueueReport,
    pub subscriber_count: EventCount,
    pub handled_count: EventCount,
    pub dead_letter_count: EventCount,
    pub handler_reports: Vec<HandlerReport>,
}

/// Event-runtime data for queue drain report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueDrainReport {
    pub queued_before: EventCount,
    pub dispatched_count: EventCount,
    pub expired_count: EventCount,
    pub remaining_count: EventCount,
    pub dispatch_reports: Vec<PublishReport>,
}

/// Event-runtime data for event metrics snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMetricsSnapshot {
    pub subscription_count: EventCount,
    pub stored_event_count: EventCount,
    pub dead_letter_count: EventCount,
    pub queue: super::EventQueueMetrics,
    pub requests: super::EventRequestMetrics,
}
