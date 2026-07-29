use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use enforcer_domain::events_types::{EventCount, SubscriberId, TargetHandler};
use std::sync::Arc;

use crate::{
    bus::reports::handler::HandlerReport, clock::SharedEventClock,
    execution::HandlerExecutionPolicy,
};

use super::{EventPublisher, SubscriberRecord};

mod core;
mod outcome;
mod reports;

/// The dispatch resources threaded unchanged through every retry attempt for
/// one subscriber: the publisher handlers use to react, the timeout/retry
/// policy, and the clock the timeout races against. Grouped so the
/// per-attempt functions in this module take one cohesive parameter instead
/// of three independent ones.
pub(super) struct DispatchContext {
    pub(super) publisher: EventPublisher,
    pub(super) policy: HandlerExecutionPolicy,
    pub(super) clock: SharedEventClock,
}

pub(super) async fn dispatch_attempt_report(
    stored: StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    context: &DispatchContext,
    attempt: EventCount,
) -> Option<HandlerReport> {
    let DispatchContext {
        publisher,
        policy,
        clock,
    } = context;
    match core::dispatch_attempt(
        // CLONE-JUSTIFICATION: the retry attempt owns an envelope while the worker retains it for later attempts/reporting.
        stored.clone(),
        subscriber,
        // CLONE-JUSTIFICATION: handler execution owns a publisher handle across its asynchronous lifetime.
        publisher.clone(),
        policy,
        Arc::clone(clock),
    )
    .await
    {
        outcome::AttemptOutcome::Handled => {
            Some(reports::handled_report(&stored, subscriber, attempt))
        }
        outcome::AttemptOutcome::Failed(error) if attempt == policy.max_attempts() => {
            Some(reports::failed_report(&stored, subscriber, attempt, error))
        }
        outcome::AttemptOutcome::TimedOut if attempt == policy.max_attempts() => {
            Some(reports::timed_out_report(&stored, subscriber, attempt))
        }
        outcome::AttemptOutcome::Panicked => {
            Some(reports::panicked_report(&stored, subscriber, attempt))
        }
        outcome::AttemptOutcome::Failed(_) | outcome::AttemptOutcome::TimedOut => None,
    }
}

pub(super) fn deadline_expired_report(
    stored: &StoredEventEnvelope,
    subscriber_id: SubscriberId,
    target_handler: TargetHandler,
    attempts: EventCount,
) -> HandlerReport {
    reports::deadline_expired_report(stored, subscriber_id, target_handler, attempts)
}

pub(super) fn dispatch_exhausted_report(
    stored: &StoredEventEnvelope,
    subscriber_id: SubscriberId,
    target_handler: TargetHandler,
) -> HandlerReport {
    reports::dispatch_exhausted_report(stored, subscriber_id, target_handler)
}
