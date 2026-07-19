use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{
    bus::reports::handler::HandlerReport, clock::SharedEventClock,
    execution::HandlerExecutionPolicy,
};

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
    // CLONE-JUSTIFICATION: fallback reports retain handler identity after the subscriber record is consumed by dispatch.
    let subscriber_id = subscriber.id.clone();
    let target_handler = subscriber.target_handler.clone();
    let context = DispatchContext {
        publisher,
        policy,
        clock,
    };
    for attempt in
        1..=crate::boundary::event_values::event_count_value(context.policy.max_attempts())
    {
        let attempt_count = crate::boundary::event_values::event_count(attempt);
        if stored.is_deadline_expired(context.clock.now()) {
            return attempt::deadline_expired_report(
                &stored,
                subscriber_id,
                target_handler,
                crate::boundary::event_values::event_count(attempt - 1),
            );
        }
        if let Some(report) =
            // CLONE-JUSTIFICATION: each retry consumes an owned envelope while the original remains for subsequent attempts and final reporting.
            attempt::dispatch_attempt_report(
                stored.clone(),
                &subscriber,
                &context,
                attempt_count,
            )
            .await
        {
            return report;
        }
    }
    attempt::dispatch_exhausted_report(&stored, subscriber_id, target_handler)
}
