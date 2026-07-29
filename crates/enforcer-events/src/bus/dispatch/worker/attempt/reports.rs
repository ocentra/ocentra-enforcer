use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::error::EventingError;
use enforcer_domain::events_types::{EventCount, HandlerOutcome, SubscriberId, TargetHandler};

use super::super::super::super::{
    reports::handler::{HandlerIdentity, HandlerReport},
    SubscriberRecord,
};

pub(super) fn deadline_expired_report(
    stored: &StoredEventEnvelope,
    subscriber_id: SubscriberId,
    target_handler: TargetHandler,
    attempts: EventCount,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            subscriber_id,
            target_handler,
        },
        HandlerOutcome::DeadlineExpired,
        Some(EventingError::EventDeadlineExpired {
            // CLONE-JUSTIFICATION: the report-owned error outlives this borrowed stored envelope.
            event_type: stored.contract.event_type.clone(),
        }),
        attempts,
    )
}

pub(super) fn dispatch_exhausted_report(
    stored: &StoredEventEnvelope,
    subscriber_id: SubscriberId,
    target_handler: TargetHandler,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            subscriber_id,
            target_handler,
        },
        HandlerOutcome::Failed,
        Some(EventingError::HandlerPolicyProducedNoAttempt),
        EventCount::ZERO,
    )
}

pub(super) fn handled_report(
    stored: &StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    attempts: EventCount,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            // CLONE-JUSTIFICATION: HandlerReport owns the subscriber identity after this borrowed record is released.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: HandlerReport owns the target handler after this borrowed record is released.
            target_handler: subscriber.target_handler.clone(),
        },
        HandlerOutcome::Handled,
        None,
        attempts,
    )
}

pub(super) fn failed_report(
    stored: &StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    attempts: EventCount,
    error: EventingError,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            // CLONE-JUSTIFICATION: HandlerReport owns the subscriber identity after this borrowed record is released.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: HandlerReport owns the target handler after this borrowed record is released.
            target_handler: subscriber.target_handler.clone(),
        },
        HandlerOutcome::Failed,
        Some(error),
        attempts,
    )
}

pub(super) fn timed_out_report(
    stored: &StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    attempts: EventCount,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            // CLONE-JUSTIFICATION: HandlerReport owns the subscriber identity after this borrowed record is released.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: HandlerReport owns the target handler after this borrowed record is released.
            target_handler: subscriber.target_handler.clone(),
        },
        HandlerOutcome::TimedOut,
        Some(EventingError::HandlerTimedOut {
            // CLONE-JUSTIFICATION: the report-owned timeout error retains the subscriber identity after the borrowed record is released.
            subscriber_id: subscriber.id.clone(),
        }),
        attempts,
    )
}

pub(super) fn panicked_report(
    stored: &StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    attempts: EventCount,
) -> HandlerReport {
    HandlerReport::new(
        stored,
        HandlerIdentity {
            // CLONE-JUSTIFICATION: HandlerReport owns the subscriber identity after this borrowed record is released.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: HandlerReport owns the target handler after this borrowed record is released.
            target_handler: subscriber.target_handler.clone(),
        },
        HandlerOutcome::Panicked,
        Some(EventingError::HandlerPanicked {
            // CLONE-JUSTIFICATION: the report-owned panic error retains the subscriber identity after the borrowed record is released.
            subscriber_id: subscriber.id.clone(),
        }),
        attempts,
    )
}
