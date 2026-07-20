use std::{sync::Arc, time::Duration};

// CANCELLATION-TEST: concurrent publish tasks are retained, deterministically released, and joined.

use std::{future::Future, pin::Pin, sync::Mutex as StdMutex};
use tokio::sync::{Mutex, Notify};

use super::fixtures::{
    metadata, metadata_with_event_id, subscriber, subscriber_for_event, test_event,
    test_event_for_type_with_aggregate_and_idempotency, test_event_with_idempotency, TestEvent,
    TestText, OTHER_EVENT_TYPE, OTHER_SUBSCRIBER, OTHER_TARGET, TEST_LABEL, TEST_SUBSCRIBER,
    TEST_TARGET,
};
use crate::{
    DispatchMode, DomainEvent, EventBus, EventJournal, EventQueuePolicy, EventingError,
    JournalAppend, JournalPolicy, JournalSelector, ManualEventClock, QueueDisposition,
};
use enforcer_domain::events_types::{DeadLetterReason, EventErrorPath, EventErrorReason};
use enforcer_events::boundary::stored_event_persistence::StoredEventEnvelope;

fn failing_journal_result(
    call: usize,
    fail_once_on: usize,
) -> Result<JournalAppend, EventingError> {
    if call == fail_once_on {
        return Err(EventingError::JournalIo {
            path: EventErrorPath::from_diagnostic("failing-journal".to_owned()),
            reason: EventErrorReason::from_diagnostic(
                "intentional one-shot append failure".to_owned(),
            ),
        });
    }

    Ok(JournalAppend {
        sequence: enforcer_domain::events_types::JournalSequence::try_new(
            std::num::NonZeroU64::new(call as u64).ok_or_else(|| {
                EventingError::InvalidHandlerPolicy {
                    reason: EventErrorReason::from_diagnostic(String::from(
                        "journal sequence must be positive",
                    )),
                }
            })?,
        ),
        previous_hash: None,
        current_hash: Some(crate::JournalHash::parse(&format!("journal-hash-{call}"))?),
    })
}

#[tokio::test]
async fn no_subscriber_queue_drains_after_subscriber_registers(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(
        crate::event_count(2),
    )?);
    let queued_report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    let subscription = bus
        .subscribe::<TestEvent, _, _>(
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
    let empty_drain = bus.drain_queued(DispatchMode::Sequential).await?;

    assert_eq!(
        queued_report.queue_report.disposition,
        QueueDisposition::QueuedNoSubscriber
    );
    assert_eq!(
        crate::event_count_value(queued_report.queue_report.queued_count),
        1
    );
    assert_eq!(crate::event_count_value(queued_report.subscriber_count), 0);
    assert_eq!(bus.journal().await.len(), 1);
    assert_eq!(
        subscription.event_type.as_str(),
        super::fixtures::TEST_EVENT_TYPE
    );
    assert_eq!(crate::event_count_value(empty_drain.queued_before), 0);
    assert_eq!(handled.lock().await.as_slice(), &[TEST_LABEL.to_string()]);
    Ok(())
}

#[tokio::test]
async fn subscriber_auto_drain_only_drains_matching_event_type(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(
        crate::event_count(4),
    )?);
    bus.publish(
        test_event_with_idempotency(
            TestText("primary queued".to_owned()),
            TestText("queue-scope-primary-key".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("queue-scope-primary-event".to_owned()),
        )?,
    )
    .await?;
    bus.publish(
        test_event_for_type_with_aggregate_and_idempotency(
            TestText("other queued".to_owned()),
            TestText("queue-scope-other-aggregate".to_owned()),
            TestText(OTHER_EVENT_TYPE.to_owned()),
            TestText("queue-scope-other-key".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(OTHER_TARGET.to_owned()),
            TestText("queue-scope-other-event".to_owned()),
        )?,
    )
    .await?;

    let handled_other = Arc::new(Mutex::new(Vec::new()));
    let handled_other_clone = Arc::clone(&handled_other);
    let other_subscription = bus
        .subscribe::<TestEvent, _, _>(
            subscriber_for_event(
                TestText(OTHER_SUBSCRIBER.to_owned()),
                TestText(OTHER_TARGET.to_owned()),
                TestText(OTHER_EVENT_TYPE.to_owned()),
            )?,
            move |context| {
                let handled = Arc::clone(&handled_other_clone);
                async move {
                    handled.lock().await.push(context.payload().label.clone());
                    Ok(())
                }
            },
        )
        .await?;
    let metrics_after_other = bus.metrics_snapshot().await;

    assert_eq!(
        crate::event_count_value(other_subscription.drain_report.queued_before),
        1
    );
    assert_eq!(
        crate::event_count_value(other_subscription.drain_report.dispatched_count),
        1
    );
    assert_eq!(
        crate::event_count_value(other_subscription.drain_report.remaining_count),
        0
    );
    assert_eq!(
        handled_other.lock().await.as_slice(),
        &["other queued".to_string()]
    );
    assert_eq!(
        crate::event_count_value(metrics_after_other.queue.queued_event_count),
        1
    );

    let handled_primary = Arc::new(Mutex::new(Vec::new()));
    let handled_primary_clone = Arc::clone(&handled_primary);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |context| {
            let handled = Arc::clone(&handled_primary_clone);
            async move {
                handled.lock().await.push(context.payload().label.clone());
                Ok(())
            }
        },
    )
    .await?;

    assert_eq!(
        handled_primary.lock().await.as_slice(),
        &["primary queued".to_string()]
    );
    assert_eq!(
        crate::event_count_value(bus.metrics_snapshot().await.queue.queued_event_count),
        0
    );
    Ok(())
}

#[tokio::test]
async fn bounded_queue_overflow_dead_letters_oldest_event_and_keeps_newest(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::with_queue_policy(EventQueuePolicy::no_subscriber_queue(
        crate::event_count(1),
    )?);
    bus.publish(
        test_event_with_idempotency(
            TestText("first".to_owned()),
            TestText("queue-overflow-first".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("queue-overflow-event-1".to_owned()),
        )?,
    )
    .await?;
    let report = bus
        .publish(
            test_event_with_idempotency(
                TestText("second".to_owned()),
                TestText("queue-overflow-second".to_owned()),
            )?,
            metadata_with_event_id(
                TestText(TEST_TARGET.to_owned()),
                TestText("queue-overflow-event-2".to_owned()),
            )?,
        )
        .await?;
    let dead_letters = bus.dead_letters().await;
    let dead_letter_event = dead_letters[0].as_event();
    let expected_dead_letter_type = crate::dead_letter_recorded_event_type()?;

    assert_eq!(
        report.queue_report.disposition,
        QueueDisposition::DeadLetteredQueueOverflow
    );
    assert_eq!(crate::event_count_value(report.dead_letter_count), 1);
    assert_eq!(
        crate::event_count_value(report.queue_report.queued_count),
        1
    );
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].reason, DeadLetterReason::QueueOverflow);
    assert_eq!(
        dead_letters[0].envelope.event_id.as_str(),
        "queue-overflow-event-1"
    );
    assert!(dead_letters[0].subscriber_id.is_none());
    assert!(dead_letters[0].target_handler.is_none());
    assert_eq!(dead_letter_event.reason, DeadLetterReason::QueueOverflow);
    assert_eq!(
        dead_letter_event.contract()?.event_type,
        expected_dead_letter_type
    );

    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
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
    assert_eq!(handled.lock().await.as_slice(), &["second".to_string()]);
    Ok(())
}

#[tokio::test]
async fn queued_event_expires_before_dispatch_when_ttl_elapsed(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let policy = EventQueuePolicy::no_subscriber_queue(crate::event_count(2))?
        .with_ttl(Duration::from_millis(5).into())?;
    let clock = ManualEventClock::new();
    let bus = EventBus::with_queue_policy_and_clock(policy, clock.shared());
    bus.publish(
        test_event(TestText(TEST_LABEL.to_owned()))?,
        metadata(TestText(TEST_TARGET.to_owned()))?,
    )
    .await?;
    clock.advance(Duration::from_millis(20).into());
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;
    let drain = bus.drain_queued(DispatchMode::Sequential).await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(crate::event_count_value(drain.queued_before), 0);
    assert_eq!(crate::event_count_value(drain.dispatched_count), 0);
    assert_eq!(crate::event_count_value(drain.remaining_count), 0);
    assert_eq!(dead_letters[0].reason, DeadLetterReason::QueueExpired);
    Ok(())
}

#[tokio::test]
async fn idempotency_registry_rejects_queued_and_completed_duplicates(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let policy =
        EventQueuePolicy::no_subscriber_queue(crate::event_count(2))?.with_idempotency_registry();
    let bus = EventBus::with_queue_policy(policy);
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("idempotency-queue-key".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("idempotency-queued-event-1".to_owned()),
        )?,
    )
    .await?;

    let queued_duplicate = bus
        .publish(
            test_event_with_idempotency(
                TestText("duplicate".to_owned()),
                TestText("idempotency-queue-key".to_owned()),
            )?,
            metadata_with_event_id(
                TestText(TEST_TARGET.to_owned()),
                TestText("idempotency-queued-event-2".to_owned()),
            )?,
        )
        .await;
    assert!(matches!(
        queued_duplicate,
        Err(EventingError::DuplicateIdempotencyKey { .. })
    ));

    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;

    let completed_duplicate = bus
        .publish(
            test_event_with_idempotency(
                TestText("completed".to_owned()),
                TestText("idempotency-queue-key".to_owned()),
            )?,
            metadata_with_event_id(
                TestText(TEST_TARGET.to_owned()),
                TestText("idempotency-completed-event".to_owned()),
            )?,
        )
        .await;
    assert!(matches!(
        completed_duplicate,
        Err(EventingError::DuplicateIdempotencyKey { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn in_flight_duplicate_guard_rejects_concurrent_event_id(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let started = Arc::new(Notify::new());
    let started_clone = Arc::clone(&started);
    let release = Arc::new(Notify::new());
    let release_clone = Arc::clone(&release);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let started = Arc::clone(&started_clone);
            let release = Arc::clone(&release_clone);
            async move {
                started.notify_waiters();
                release.notified().await;
                Ok(())
            }
        },
    )
    .await?;

    let first_bus = bus.clone();
    let first_event = test_event_with_idempotency(
        TestText(TEST_LABEL.to_owned()),
        TestText("in-flight-idempotency-key-1".to_owned()),
    )?;
    let first_metadata = metadata(TestText(TEST_TARGET.to_owned()))?;
    let first = tokio::spawn(async move { first_bus.publish(first_event, first_metadata).await });
    started.notified().await;
    let duplicate = bus
        .publish(
            test_event_with_idempotency(
                TestText("duplicate".to_owned()),
                TestText("in-flight-idempotency-key-2".to_owned()),
            )?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await;
    release.notify_waiters();
    let first_report = first.await??;

    assert!(matches!(
        duplicate,
        Err(EventingError::DuplicateEventId { .. })
    ));
    assert_eq!(crate::event_count_value(first_report.handled_count), 1);
    Ok(())
}

#[tokio::test]
async fn failed_subscribe_drain_preserves_queued_event_for_retry(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let policy = EventQueuePolicy::no_subscriber_queue(crate::event_count(2))?;
    let journal = Arc::new(FailingJournal::fail_once_on(1));
    let bus = EventBus::with_journal_and_queue_policy(
        JournalPolicy::before_and_after_dispatch(JournalSelector::All),
        journal,
        policy,
    );
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("drain-preserve-idempotency".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("drain-preserve-event-1".to_owned()),
        )?,
    )
    .await?;

    let failed_subscribe = bus
        .subscribe::<TestEvent, _, _>(
            subscriber(
                TestText(TEST_SUBSCRIBER.to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            |_| async { Ok(()) },
        )
        .await;
    assert!(matches!(
        failed_subscribe,
        Err(EventingError::JournalIo { .. })
    ));

    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
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

    assert_eq!(handled.lock().await.as_slice(), &[TEST_LABEL.to_string()]);
    Ok(())
}

#[tokio::test]
async fn after_dispatch_journal_failure_does_not_replay_handler_work(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let policy = EventQueuePolicy::no_subscriber_queue(crate::event_count(2))?;
    let journal = Arc::new(FailingJournal::fail_once_on(2));
    let bus = EventBus::with_journal_and_queue_policy(
        JournalPolicy::before_and_after_dispatch(JournalSelector::All),
        journal,
        policy,
    );
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("drain-after-dispatch-key".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("drain-after-dispatch-event".to_owned()),
        )?,
    )
    .await?;

    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    let failed_subscribe = bus
        .subscribe::<TestEvent, _, _>(
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
        .await;
    assert!(matches!(
        failed_subscribe,
        Err(EventingError::JournalIo { .. })
    ));

    let retry_handled = Arc::new(Mutex::new(Vec::new()));
    let retry_handled_clone = Arc::clone(&retry_handled);
    let retry_subscription = bus
        .subscribe::<TestEvent, _, _>(
            subscriber(
                TestText(TEST_SUBSCRIBER.to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            move |context| {
                let handled = Arc::clone(&retry_handled_clone);
                async move {
                    handled.lock().await.push(context.payload().label.clone());
                    Ok(())
                }
            },
        )
        .await?;

    assert_eq!(handled.lock().await.as_slice(), &[TEST_LABEL.to_string()]);
    assert!(retry_handled.lock().await.is_empty());
    assert_eq!(
        crate::event_count_value(retry_subscription.drain_report.queued_before),
        0
    );
    assert_eq!(
        crate::event_count_value(retry_subscription.drain_report.dispatched_count),
        0
    );
    Ok(())
}

struct FailingJournal {
    calls: StdMutex<usize>,
    fail_once_on: usize,
}

impl FailingJournal {
    fn fail_once_on(call: usize) -> Self {
        Self {
            calls: StdMutex::new(0),
            fail_once_on: call,
        }
    }
}

impl EventJournal for FailingJournal {
    fn append<'a>(
        &'a self,
        _envelope: &'a StoredEventEnvelope,
    ) -> Pin<Box<dyn Future<Output = Result<JournalAppend, EventingError>> + Send + 'a>> {
        Box::pin(async move {
            let call = {
                let mut calls = match self.calls.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        return Err(EventingError::JournalIo {
                            path: EventErrorPath::from_diagnostic("failing-journal".to_owned()),
                            reason: EventErrorReason::from_diagnostic(
                                "failing journal lock poisoned".to_owned(),
                            ),
                        })
                    }
                };
                *calls += 1;
                *calls
            };
            failing_journal_result(call, self.fail_once_on)
        })
    }
}
