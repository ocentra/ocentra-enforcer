use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use tokio::{sync::Notify, task::yield_now};

use crate::{
    AggregateKey, DispatchMode, DomainEvent, EventBus, EventClock, EventContract, EventQueuePolicy,
    EventResponseContract, EventType, EventingError, HandlerExecutionPolicy, IdempotencyKey,
    ManualEventClock, RequestCompletionOutcome, RequestEvent, RequestId, RequestOptions,
    SchemaVersion,
};
use enforcer_events::bus::reports::dead_letter::DeadLetterReason;
use enforcer_events::bus::reports::handler::HandlerOutcome;

use super::fixtures::{
    metadata, subscriber, subscriber_for_event, test_event, TestText, TEST_LABEL, TEST_TARGET,
};

const CLOCK_REQUEST_EVENT_TYPE: &str = "eventing.clock.request";
const CLOCK_REQUEST_ID: &str = "eventing-clock-request";
const CLOCK_REQUEST_AGGREGATE: &str = "eventing-clock-aggregate";
const CLOCK_REQUEST_IDEMPOTENCY: &str = "eventing-clock-idempotency";

#[tokio::test]
async fn manual_clock_advances_registered_sleepers_without_wall_clock_sleep(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let sleeper_clock = clock.clone();
    let completed = Arc::new(AtomicUsize::new(0));
    let completed_clone = Arc::clone(&completed);
    let sleeper = tokio::spawn(async move {
        sleeper_clock.sleep(Duration::from_millis(10)).await;
        completed_clone.fetch_add(1, Ordering::SeqCst);
    });

    yield_until(|| clock.pending_sleep_count() == 1).await?;
    clock.advance(Duration::from_millis(9));
    yield_now().await;
    assert_eq!(completed.load(Ordering::SeqCst), 0);

    clock.advance(Duration::from_millis(1));
    sleeper.await?;
    assert_eq!(completed.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn manual_clock_expires_queued_ttl_without_wall_clock_sleep(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let policy = EventQueuePolicy::no_subscriber_queue(2)?.with_ttl(Duration::from_millis(10))?;
    let bus = EventBus::with_queue_policy_and_clock(policy, clock.shared());

    bus.publish(
        test_event(TestText(TEST_LABEL.to_owned()))?,
        metadata(TestText(TEST_TARGET.to_owned()))?,
    )
    .await?;
    clock.advance(Duration::from_millis(11));
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText("manual-clock-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;
    let drain = bus.drain_queued(DispatchMode::Sequential).await?;

    assert_eq!(drain.expired_count, 0);
    assert_eq!(drain.dispatched_count, 0);
    assert_eq!(bus.dead_letters().await.len(), 1);
    Ok(())
}

#[tokio::test]
async fn manual_clock_dead_letters_past_deadline_without_dispatch(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let bus = EventBus::with_clock(clock.shared());
    let attempts = Arc::new(AtomicUsize::new(0));
    let handler_attempts = Arc::clone(&attempts);
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText("manual-deadline-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let handler_attempts = Arc::clone(&handler_attempts);
            async move {
                handler_attempts.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        },
    )
    .await?;
    let deadline = clock
        .now()
        .checked_add(Duration::from_millis(5))
        .ok_or("deadline fits manual clock")?;
    clock.advance(Duration::from_millis(6));

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?.with_deadline(deadline),
        )
        .await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert_eq!(report.subscriber_count, 0);
    assert_eq!(report.dead_letter_count, 1);
    assert_eq!(dead_letters[0].reason, DeadLetterReason::DeadlineExpired);
    Ok(())
}

#[tokio::test]
async fn manual_clock_drives_handler_timeout_retries_without_wall_clock_sleep(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let bus = EventBus::with_handler_policy_and_clock(
        HandlerExecutionPolicy::new(Some(Duration::from_millis(5)), 2)?,
        clock.shared(),
    );
    let attempts = Arc::new(AtomicUsize::new(0));
    let handler_clock = clock.clone();
    let handler_attempts = Arc::clone(&attempts);
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText("manual-timeout-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let handler_clock = handler_clock.clone();
            let handler_attempts = Arc::clone(&handler_attempts);
            async move {
                handler_attempts.fetch_add(1, Ordering::SeqCst);
                handler_clock.sleep(Duration::from_millis(50)).await;
                Ok(())
            }
        },
    )
    .await?;
    let publish_bus = bus.clone();
    let publish_event = test_event(TestText(TEST_LABEL.to_owned()))?;
    let publish_metadata = metadata(TestText(TEST_TARGET.to_owned()))?;
    let publish =
        tokio::spawn(async move { publish_bus.publish(publish_event, publish_metadata).await });

    yield_until(|| attempts.load(Ordering::SeqCst) == 1 && clock.pending_sleep_count() >= 2)
        .await?;
    clock.advance(Duration::from_millis(5));
    yield_until(|| attempts.load(Ordering::SeqCst) == 2 && clock.pending_sleep_count() >= 3)
        .await?;
    clock.advance(Duration::from_millis(5));

    let report = publish.await??;
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(report.handler_reports[0].attempts, 2);
    assert_eq!(report.dead_letter_count, 1);
    Ok(())
}

#[tokio::test]
async fn manual_clock_stops_retry_when_deadline_expires_between_attempts(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let bus = EventBus::with_handler_policy_and_clock(
        HandlerExecutionPolicy::new(None, 3)?,
        clock.shared(),
    );
    let attempts = Arc::new(AtomicUsize::new(0));
    let handler_clock = clock.clone();
    let handler_attempts = Arc::clone(&attempts);
    bus.subscribe::<super::fixtures::TestEvent, _, _>(
        subscriber(
            TestText("manual-retry-deadline-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let handler_clock = handler_clock.clone();
            let handler_attempts = Arc::clone(&handler_attempts);
            async move {
                handler_attempts.fetch_add(1, Ordering::SeqCst);
                handler_clock.sleep(Duration::from_millis(1)).await;
                Err(EventingError::EmptyValue {
                    field: "manual_clock_deadline_retry",
                })
            }
        },
    )
    .await?;
    let deadline = clock
        .now()
        .checked_add(Duration::from_millis(1))
        .ok_or("deadline fits manual clock")?;
    let publish_bus = bus.clone();
    let publish_event = test_event(TestText(TEST_LABEL.to_owned()))?;
    let publish_metadata = metadata(TestText(TEST_TARGET.to_owned()))?.with_deadline(deadline);
    let publish =
        tokio::spawn(async move { publish_bus.publish(publish_event, publish_metadata).await });

    yield_until(|| clock.pending_sleep_count() >= 1).await?;
    clock.advance(Duration::from_millis(1));

    let report = publish.await??;
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        report.handler_reports[0].outcome,
        HandlerOutcome::DeadlineExpired
    );
    assert_eq!(report.handler_reports[0].attempts, 1);
    assert_eq!(report.dead_letter_count, 1);
    Ok(())
}

#[tokio::test]
async fn manual_clock_drives_request_timeout_and_late_completion_without_wall_clock_sleep(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let clock = ManualEventClock::new();
    let bus = EventBus::with_clock(clock.shared());
    let outcomes = Arc::new(Mutex::new(Vec::new()));
    let late_sleep_registered = Arc::new(Notify::new());
    let handler_clock = clock.clone();
    let handler_outcomes = Arc::clone(&outcomes);
    let handler_late_sleep_registered = Arc::clone(&late_sleep_registered);
    bus.subscribe::<ClockRequestEvent, _, _>(
        subscriber_for_event(
            TestText("manual-request-subscriber".to_owned()),
            TestText(TEST_TARGET.to_owned()),
            TestText(CLOCK_REQUEST_EVENT_TYPE.to_owned()),
        )?,
        move |context| {
            let handler_clock = handler_clock.clone();
            let handler_outcomes = Arc::clone(&handler_outcomes);
            let handler_late_sleep_registered = Arc::clone(&handler_late_sleep_registered);
            async move {
                tokio::spawn(async move {
                    let sleep = handler_clock.sleep(Duration::from_millis(20));
                    handler_late_sleep_registered.notify_one();
                    sleep.await;
                    let completion = context.complete_request(ClockResponse::approved()).await?;
                    let Ok(mut guard) = handler_outcomes.lock() else {
                        return Err(EventingError::EmptyValue {
                            field: "outcomes_lock_poisoned",
                        });
                    };
                    guard.push(completion.outcome);
                    Ok::<(), EventingError>(())
                });
                Ok(())
            }
        },
    )
    .await?;
    let request_bus = bus.clone();
    let request_event = ClockRequestEvent::new()?;
    let request_metadata = metadata(TestText(TEST_TARGET.to_owned()))?;
    let request_timeout = RequestOptions::with_timeout(Duration::from_millis(5))?;
    let request = tokio::spawn(async move {
        request_bus
            .publish_request(request_event, request_metadata, request_timeout)
            .await
    });

    late_sleep_registered.notified().await;
    yield_until(|| clock.pending_sleep_count() >= 2).await?;
    clock.advance(Duration::from_millis(5));
    let result = request.await?;
    assert!(matches!(result, Err(EventingError::RequestTimedOut { .. })));

    clock.advance(Duration::from_millis(20));
    yield_until(|| matches!(outcomes.lock(), Ok(guard) if !guard.is_empty())).await?;
    let Ok(outcomes_guard) = outcomes.lock() else {
        return Err("outcomes lock poisoned".into());
    };
    assert_eq!(outcomes_guard.as_slice(), &[RequestCompletionOutcome::Late]);
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ClockRequestEvent {
    request_id: RequestId,
}

impl ClockRequestEvent {
    fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            request_id: RequestId::parse(CLOCK_REQUEST_ID)?,
        })
    }
}

impl DomainEvent for ClockRequestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            EventType::parse(CLOCK_REQUEST_EVENT_TYPE)?,
            SchemaVersion::new(1)?,
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        AggregateKey::parse(CLOCK_REQUEST_AGGREGATE)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        IdempotencyKey::parse(CLOCK_REQUEST_IDEMPOTENCY)
    }
}

impl RequestEvent for ClockRequestEvent {
    type Response = ClockResponse;

    fn request_id(&self) -> Result<RequestId, EventingError> {
        Ok(self.request_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ClockResponse {
    decision: String,
}

impl ClockResponse {
    fn approved() -> Self {
        Self {
            decision: "approved".to_string(),
        }
    }
}

impl EventResponseContract for ClockResponse {}

async fn yield_until(condition: impl Fn() -> bool) -> Result<(), String> {
    for _ in 0..50 {
        if condition() {
            return Ok(());
        }
        yield_now().await;
    }
    Err(String::from(
        "condition did not become true within the polling budget",
    ))
}
