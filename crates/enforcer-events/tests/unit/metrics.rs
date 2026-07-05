use std::time::Duration;

use super::fixtures::{
    metadata, metadata_with_event_id, subscriber, test_event_with_idempotency, TestEvent, TestText,
    TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use crate::bus::reports::dead_letter::DeadLetterReason;
use crate::bus::EventBus;
use crate::error::EventingError;
use crate::queue::policy::EventQueuePolicy;
use crate::request::EventResponseContract;
use crate::request::RequestOptions;

const IN_MEMORY_RETENTION_PROBE_COUNT: usize = 4097;
const EXPECTED_IN_MEMORY_RETENTION_LIMIT: usize = 4096;

#[tokio::test]
async fn metrics_snapshot_reports_queue_dead_letter_journal_and_request_counts(
) -> Result<(), Box<dyn std::error::Error>> {
    let policy = EventQueuePolicy::no_subscriber_queue(1)?.with_idempotency_registry();
    let bus = EventBus::with_queue_policy(policy);
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("metrics-queued-idempotency".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("metrics-event-1".to_owned()),
        )?,
    )
    .await?;
    bus.publish(
        test_event_with_idempotency(
            TestText("overflow".to_owned()),
            TestText("metrics-overflow-idempotency".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("metrics-event-2".to_owned()),
        )?,
    )
    .await?;

    let queued = bus.metrics_snapshot().await;
    assert_eq!(queued.subscription_count, 0);
    assert_eq!(queued.stored_event_count, 2);
    assert_eq!(queued.dead_letter_count, 1);
    assert_eq!(queued.queue.queued_event_count, 1);
    assert_eq!(queued.queue.queued_event_id_count, 1);
    assert_eq!(queued.queue.queued_idempotency_key_count, 1);
    assert_eq!(queued.queue.completed_idempotency_key_count, 1);
    assert_eq!(queued.queue.capacity, Some(1));

    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;
    let drained = bus.metrics_snapshot().await;
    assert_eq!(drained.subscription_count, 1);
    assert_eq!(drained.queue.queued_event_count, 0);
    assert_eq!(drained.queue.completed_idempotency_key_count, 2);

    let timeout = bus
        .publish_request(
            SlowMetricsRequest::new()?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
            RequestOptions::with_timeout(Duration::from_millis(1))?,
        )
        .await;
    assert!(matches!(
        timeout,
        Err(EventingError::RequestTimedOut { .. })
    ));
    let timed_out = bus.metrics_snapshot().await;
    assert_eq!(timed_out.requests.pending_request_count, 0);
    assert_eq!(timed_out.requests.timed_out_request_count, 1);
    assert_eq!(timed_out.dead_letter_count, 1);
    assert_eq!(
        bus.dead_letters().await[0].reason,
        DeadLetterReason::QueueOverflow
    );
    Ok(())
}

#[tokio::test]
async fn metrics_snapshot_reports_bounded_in_memory_event_retention(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::new();
    for index in 0..IN_MEMORY_RETENTION_PROBE_COUNT {
        bus.publish(
            test_event_with_idempotency(
                TestText(TEST_LABEL.to_owned()),
                TestText(format!("retention-key-{index}").to_owned()),
            )?,
            metadata_with_event_id(
                TestText(TEST_TARGET.to_owned()),
                TestText(format!("retention-event-{index}").to_owned()),
            )?,
        )
        .await?;
    }

    let metrics = bus.metrics_snapshot().await;
    let journal = bus.journal().await;

    assert_eq!(
        metrics.stored_event_count,
        EXPECTED_IN_MEMORY_RETENTION_LIMIT
    );
    assert_eq!(journal.len(), EXPECTED_IN_MEMORY_RETENTION_LIMIT);
    assert_eq!(
        journal
            .first()
            .ok_or("retained journal has first entry")?
            .event_id
            .as_str(),
        "retention-event-1"
    );
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SlowMetricsRequest {
    request_id: crate::RequestId,
}

impl SlowMetricsRequest {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            request_id: crate::RequestId::parse("metrics-request-1")?,
        })
    }
}

impl crate::DomainEvent for SlowMetricsRequest {
    fn contract(&self) -> Result<crate::EventContract, EventingError> {
        Ok(crate::EventContract::new(
            crate::EventType::parse("eventing.metrics.request")?,
            crate::SchemaVersion::new(1)?,
        ))
    }

    fn aggregate_key(&self) -> Result<crate::AggregateKey, EventingError> {
        crate::AggregateKey::parse("metrics-request-aggregate")
    }

    fn idempotency_key(&self) -> Result<crate::IdempotencyKey, EventingError> {
        crate::IdempotencyKey::parse("metrics-request-idempotency")
    }
}

impl crate::RequestEvent for SlowMetricsRequest {
    type Response = MetricsResponse;

    fn request_id(&self) -> Result<crate::RequestId, EventingError> {
        Ok(self.request_id.clone())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct MetricsResponse {
    decision: String,
}

impl EventResponseContract for MetricsResponse {}
