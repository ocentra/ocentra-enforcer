use crate::bus::reports::handler::HandlerIdentity;
use crate::{
    EventingError, HandlerOutcome, HandlerReport, StoredEventEnvelope, SubscriberId, TargetHandler,
};

use super::SubscriberRecord;

pub(super) fn deadline_expired_report(
    stored: &StoredEventEnvelope,
    subscriber_id: SubscriberId,
    target_handler: TargetHandler,
    attempts: usize,
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
        Some(EventingError::InvalidHandlerPolicy {
            reason: String::from("handler execution policy produced no attempt"),
        }),
        0,
    )
}

pub(super) fn handled_report(
    stored: &StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    attempts: usize,
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
    attempts: usize,
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
    attempts: usize,
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
    attempts: usize,
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
