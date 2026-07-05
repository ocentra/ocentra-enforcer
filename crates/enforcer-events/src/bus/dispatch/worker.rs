use crate::{HandlerExecutionPolicy, HandlerReport, SharedEventClock, StoredEventEnvelope};

use super::{EventPublisher, SubscriberRecord};

mod attempt;
use attempt::DispatchContext;

pub(super) async fn dispatch_one(
    stored: StoredEventEnvelope,
    subscriber: SubscriberRecord,
    publisher: EventPublisher,
    policy: HandlerExecutionPolicy,
    clock: SharedEventClock,
) -> HandlerReport {
    let subscriber_id = subscriber.id.clone();
    let target_handler = subscriber.target_handler.clone();
    let context = DispatchContext {
        publisher,
        policy,
        clock,
    };
    for attempt in 1..=context.policy.max_attempts() {
        if stored.is_deadline_expired(context.clock.now()) {
            return attempt::deadline_expired_report(
                &stored,
                subscriber_id,
                target_handler,
                attempt - 1,
            );
        }
        if let Some(report) =
            attempt::dispatch_attempt_report(stored.clone(), &subscriber, &context, attempt).await
        {
            return report;
        }
    }
    attempt::dispatch_exhausted_report(&stored, subscriber_id, target_handler)
}
