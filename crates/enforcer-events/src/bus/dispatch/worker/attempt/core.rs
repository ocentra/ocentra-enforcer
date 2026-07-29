use std::panic::AssertUnwindSafe;

use futures::FutureExt;

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{clock::SharedEventClock, execution::HandlerExecutionPolicy};

use super::super::EventPublisher;
use super::super::SubscriberRecord;
use super::outcome::AttemptOutcome;

pub(super) async fn dispatch_attempt(
    stored: StoredEventEnvelope,
    subscriber: &SubscriberRecord,
    publisher: EventPublisher,
    policy: &HandlerExecutionPolicy,
    clock: SharedEventClock,
) -> AttemptOutcome {
    // CANCELLATION-TEST: `tests/unit/handler_policy.rs` drives this handler
    // timeout branch through EventBus and verifies retry/dead-letter outcomes.
    let attempt = AssertUnwindSafe((subscriber.handler)(stored, publisher)).catch_unwind();
    let result = match policy.timeout() {
        Some(timeout) => {
            // CANCEL-SAFE: losing the attempt branch drops the handler future; losing the clock branch unregisters its sleep future.
            tokio::select! {
                result = attempt => Ok(result),
                _ = clock.sleep(timeout) => Err(AttemptOutcome::TimedOut),
            }
        }
        None => Ok(attempt.await),
    };
    match result {
        Ok(Ok(Ok(()))) => AttemptOutcome::Handled,
        Ok(Ok(Err(error))) => AttemptOutcome::Failed(error),
        Ok(Err(_)) => AttemptOutcome::Panicked,
        Err(outcome) => outcome,
    }
}
