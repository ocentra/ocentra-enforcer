use std::sync::Arc;

use futures::future::join_all;

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{clock::SharedEventClock, execution::HandlerExecutionPolicy};

use super::{EventPublisher, HandlerReport, SubscriberRecord};

mod worker;
use worker::dispatch_one;

pub(super) async fn dispatch_sequential(
    stored: StoredEventEnvelope,
    subscribers: Vec<SubscriberRecord>,
    publisher: EventPublisher,
    policy: HandlerExecutionPolicy,
    clock: SharedEventClock,
) -> Vec<HandlerReport> {
    let mut reports = Vec::new();
    for subscriber in subscribers {
        // CLONE-JUSTIFICATION: each sequential subscriber receives an independently owned envelope, publisher, and retry policy.
        reports.push(
            dispatch_one(
                stored.clone(),
                subscriber,
                // CLONE-JUSTIFICATION: each concurrent dispatch task owns a publisher handle.
                publisher.clone(),
                // CLONE-JUSTIFICATION: each concurrent dispatch task owns its immutable execution policy.
                policy.clone(),
                Arc::clone(&clock),
            )
            .await,
        );
    }
    reports
}

pub(super) async fn dispatch_concurrent(
    stored: StoredEventEnvelope,
    subscribers: Vec<SubscriberRecord>,
    publisher: EventPublisher,
    policy: HandlerExecutionPolicy,
    clock: SharedEventClock,
) -> Vec<HandlerReport> {
    // CLONE-JUSTIFICATION: concurrent handler futures must each own their envelope, publisher, policy, and shared clock handle.
    join_all(subscribers.into_iter().map(|subscriber| {
        dispatch_one(
            stored.clone(),
            subscriber,
            // CLONE-JUSTIFICATION: targeted dispatch transfers an owned publisher handle to the worker.
            publisher.clone(),
            // CLONE-JUSTIFICATION: targeted dispatch transfers an owned policy snapshot to the worker.
            policy.clone(),
            Arc::clone(&clock),
        )
    }))
    .await
}
