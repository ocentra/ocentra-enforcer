use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify};

use super::fixtures::{
    metadata, metadata_with_event_id, subscriber, subscriber_for_event, test_event,
    test_event_for_type, test_event_with_idempotency, TestText, OTHER_EVENT_TYPE, OTHER_TARGET,
    TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use crate::{
    AggregateKey, DomainEvent, EventBus, EventContract, EventQueuePolicy, EventResponseContract,
    EventingError, IdempotencyKey, RequestEvent, RequestId, RequestOptions, SchemaVersion,
    ShutdownMode,
};
use enforcer_events::bus::reports::dead_letter::DeadLetterReason;

const SHUTDOWN_REQUEST_EVENT_TYPE: &str = "eventing.shutdown.request";
const SHUTDOWN_REQUEST_ID: &str = "eventing-shutdown-request";
const SHUTDOWN_REQUEST_AGGREGATE: &str = "eventing-shutdown-aggregate";
const SHUTDOWN_REQUEST_IDEMPOTENCY: &str = "eventing-shutdown-idempotency";

#[tokio::test]
async fn production_shutdown_drain_dispatches_queue_and_dead_letters_remaining(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(4)?);
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("shutdown-drain-dispatch".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("shutdown-drain-event-1".to_owned()),
        )?,
    )
    .await?;
    bus.publish(
        test_event_for_type(
            TestText("unmatched".to_owned()),
            TestText(OTHER_EVENT_TYPE.to_owned()),
        )?,
        metadata_with_event_id(
            TestText(OTHER_TARGET.to_owned()),
            TestText("shutdown-drain-event-2".to_owned()),
        )?,
    )
    .await?;

    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |context| {
            let handled = Arc::clone(&handled_clone);
            async move {
                handled.lock().await.push(context.payload().label.clone());
                Ok(())
            }
        },
    )
    .await?;

    let report = bus.shutdown(ShutdownMode::Drain).await?;
    let dead_letters = bus.dead_letters().await;
    let publish_after_shutdown = bus
        .publish(
            test_event(TestText("after-shutdown".to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await;
    let subscribe_after_shutdown = bus
        .subscribe::<super::fixtures::TestEvent, _, _>(
            subscriber(
                TestText("shutdown-subscriber-after".to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            |_| async { Ok(()) },
        )
        .await;

    assert_eq!(report.mode, ShutdownMode::Drain);
    assert!(!report.already_shutdown);
    assert_eq!(report.subscription_count, 1);
    assert_eq!(report.queued_event_count, 1);
    assert_eq!(report.queued_dispatched_count, 0);
    assert_eq!(report.queued_expired_count, 0);
    assert_eq!(report.queued_dead_lettered_count, 1);
    assert_eq!(report.queued_dropped_count, 0);
    assert_eq!(handled.lock().await.as_slice(), &[TEST_LABEL.to_string()]);
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].reason, DeadLetterReason::Shutdown);
    assert!(matches!(
        publish_after_shutdown,
        Err(EventingError::BusShutdown)
    ));
    assert!(matches!(
        subscribe_after_shutdown,
        Err(EventingError::BusShutdown)
    ));
    Ok(())
}

#[tokio::test]
async fn production_shutdown_dead_letters_queued_without_dispatch(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(2)?);
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("shutdown-dead-letter".to_owned()),
        )?,
        metadata(TestText(TEST_TARGET.to_owned()))?,
    )
    .await?;

    let report = bus.shutdown(ShutdownMode::DeadLetterQueued).await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(report.queued_event_count, 1);
    assert_eq!(report.queued_dispatched_count, 0);
    assert_eq!(report.queued_dead_lettered_count, 1);
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].reason, DeadLetterReason::Shutdown);
    Ok(())
}

#[tokio::test]
async fn production_shutdown_waits_for_active_dispatch_before_clearing_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::new();
    let handler_started = Arc::new(Notify::new());
    let release_handler = Arc::new(Notify::new());
    let handled = Arc::new(Mutex::new(0_usize));
    let handler_started_clone = Arc::clone(&handler_started);
    let release_handler_clone = Arc::clone(&release_handler);
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText("shutdown-active-dispatch-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let handler_started = Arc::clone(&handler_started_clone);
            let release_handler = Arc::clone(&release_handler_clone);
            let handled = Arc::clone(&handled_clone);
            async move {
                handler_started.notify_one();
                release_handler.notified().await;
                *handled.lock().await += 1;
                Ok(())
            }
        },
    )
    .await?;

    let publish_bus = bus.clone();
    let publish_event = test_event(TestText(TEST_LABEL.to_owned()))?;
    let publish_metadata = metadata(TestText(TEST_TARGET.to_owned()))?;
    let publish = tokio::spawn(async move { publish_bus.publish(publish_event, publish_metadata).await });
    handler_started.notified().await;
    let shutdown_bus = bus.clone();
    let shutdown = tokio::spawn(async move { shutdown_bus.shutdown(ShutdownMode::Drain).await });
    tokio::time::sleep(Duration::from_millis(10)).await;

    assert!(!shutdown.is_finished());
    release_handler.notify_waiters();
    let report = shutdown.await??;
    let publish_report = publish.await??;

    assert_eq!(report.in_flight_dispatch_count, 1);
    assert_eq!(report.subscription_count, 1);
    assert_eq!(publish_report.handled_count, 1);
    assert_eq!(*handled.lock().await, 1);
    assert!(matches!(
        bus.publish(
            test_event(TestText("after-active-shutdown".to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?
        )
        .await,
        Err(EventingError::BusShutdown)
    ));
    Ok(())
}

#[tokio::test]
async fn test_only_shutdown_drop_reports_dropped_queued_work(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(2)?);
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("shutdown-drop-test-only".to_owned()),
        )?,
        metadata(TestText(TEST_TARGET.to_owned()))?,
    )
    .await?;

    let report = bus.shutdown(ShutdownMode::DropQueuedForTestOnly).await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(report.queued_event_count, 1);
    assert_eq!(report.queued_dead_lettered_count, 0);
    assert_eq!(report.queued_dropped_count, 1);
    assert!(dead_letters.is_empty());
    Ok(())
}

#[tokio::test]
async fn production_shutdown_cancels_pending_request_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::new();
    let handler_seen = Arc::new(Notify::new());
    let handler_seen_clone = Arc::clone(&handler_seen);
    bus.subscribe::<ShutdownRequestEvent, _, _>(
        subscriber_for_event(
            TestText("shutdown-request-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
            TestText(SHUTDOWN_REQUEST_EVENT_TYPE.to_owned()),
        )?,
        move |_| {
            let handler_seen = Arc::clone(&handler_seen_clone);
            async move {
                handler_seen.notify_one();
                Ok(())
            }
        },
    )
    .await?;

    let request_bus = bus.clone();
    let request_event = ShutdownRequestEvent::new()?;
    let request_metadata = metadata(TestText(TEST_TARGET.to_owned()))?;
    let request_timeout = RequestOptions::with_timeout(Duration::from_secs(60))?;
    let request = tokio::spawn(async move {
        request_bus
            .publish_request(request_event, request_metadata, request_timeout)
            .await
    });

    handler_seen.notified().await;
    let report = bus.shutdown(ShutdownMode::Drain).await?;
    let result = request.await?;
    let second_shutdown = bus.shutdown(ShutdownMode::Drain).await?;

    assert_eq!(report.pending_request_count, 1);
    assert!(matches!(result, Err(EventingError::RequestTimedOut { .. })));
    assert!(second_shutdown.already_shutdown);
    assert_eq!(second_shutdown.queued_event_count, 0);
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ShutdownRequestEvent {
    request_id: RequestId,
}

impl ShutdownRequestEvent {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            request_id: RequestId::parse(SHUTDOWN_REQUEST_ID)?,
        })
    }
}

impl DomainEvent for ShutdownRequestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            crate::EventType::parse(SHUTDOWN_REQUEST_EVENT_TYPE)?,
            SchemaVersion::new(1)?,
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        AggregateKey::parse(SHUTDOWN_REQUEST_AGGREGATE)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        IdempotencyKey::parse(SHUTDOWN_REQUEST_IDEMPOTENCY)
    }
}

impl RequestEvent for ShutdownRequestEvent {
    type Response = ShutdownResponse;

    fn request_id(&self) -> Result<RequestId, EventingError> {
        Ok(self.request_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ShutdownResponse {
    decision: String,
}

impl EventResponseContract for ShutdownResponse {}
