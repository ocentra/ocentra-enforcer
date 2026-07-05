use std::sync::Arc;

use futures::future::join_all;

use crate::{HandlerExecutionPolicy, SharedEventClock, StoredEventEnvelope};

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
        reports.push(
            dispatch_one(
                stored.clone(),
                subscriber,
                publisher.clone(),
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
    join_all(subscribers.into_iter().map(|subscriber| {
        dispatch_one(
            stored.clone(),
            subscriber,
            publisher.clone(),
            policy.clone(),
            Arc::clone(&clock),
        )
    }))
    .await
}
