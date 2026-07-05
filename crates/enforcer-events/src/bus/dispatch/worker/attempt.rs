use crate::{
    HandlerExecutionPolicy, HandlerReport, SharedEventClock, StoredEventEnvelope, SubscriberId,
    TargetHandler,
};

use super::{EventPublisher, SubscriberRecord};

mod core;
mod outcome;
mod reports;

pub(super) async fn dispatch_attempt_report(
    stored: StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    publisher: EventPublisher,
    policy: &HandlerExecutionPolicy,
    clock: SharedEventClock,
    attempt: usize,
) -> Option<HandlerReport> {
    match core::dispatch_attempt(stored.clone(), subscriber, publisher, policy, clock).await {
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
    attempts: usize,
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
