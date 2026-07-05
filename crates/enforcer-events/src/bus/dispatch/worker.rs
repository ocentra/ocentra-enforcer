use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use futures::FutureExt;

use crate::{
    EventingError, HandlerExecutionPolicy, HandlerOutcome, HandlerReport, SharedEventClock,
    StoredEventEnvelope, SubscriberId, TargetHandler,
};

use super::{EventPublisher, SubscriberRecord};

mod attempt;

pub(super) async fn dispatch_one(
    stored: StoredEventEnvelope,
    subscriber: SubscriberRecord,
    publisher: EventPublisher,
    policy: HandlerExecutionPolicy,
    clock: SharedEventClock,
) -> HandlerReport {
    let subscriber_id = subscriber.id.clone();
    let target_handler = subscriber.target_handler.clone();
    for attempt in 1..=policy.max_attempts() {
        if stored.is_deadline_expired(clock.now()) {
            return attempt::deadline_expired_report(
                &stored,
                subscriber_id,
                target_handler,
                attempt - 1,
            );
        }
        if let Some(report) = attempt::dispatch_attempt_report(
            stored.clone(),
            &subscriber,
            publisher.clone(),
            &policy,
            Arc::clone(&clock),
            attempt,
        )
        .await
        {
            return report;
        }
    }
    attempt::dispatch_exhausted_report(&stored, subscriber_id, target_handler)
}
