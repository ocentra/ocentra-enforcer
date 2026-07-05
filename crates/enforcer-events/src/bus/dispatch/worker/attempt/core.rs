use std::panic::AssertUnwindSafe;

use futures::FutureExt;

use crate::{EventingError, HandlerExecutionPolicy, SharedEventClock, StoredEventEnvelope};

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
    let attempt = AssertUnwindSafe((subscriber.handler)(stored, publisher)).catch_unwind();
    let result = match policy.timeout() {
        Some(timeout) => {
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
