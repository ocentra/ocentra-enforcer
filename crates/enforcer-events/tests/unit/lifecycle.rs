use std::{sync::Arc, sync::Mutex as StdMutex, time::Duration};

use tokio::sync::{Barrier, Mutex};

use super::fixtures::{
    metadata, metadata_with_event_id, subscriber, subscriber_for_event, test_event,
    test_event_for_type, test_event_with_aggregate, TestEvent, TestText, OTHER_EVENT_TYPE,
    OTHER_SUBSCRIBER, OTHER_TARGET, TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use crate::{DispatchMode, EventBus, EventRegistrar, EventingError, RegistrarStatus};
use enforcer_domain::events_types::HandlerOutcome;

#[tokio::test]
async fn ordered_dispatch_serializes_same_aggregate_transitions(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_clone = Arc::clone(&observed);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |context| {
            let observed = Arc::clone(&observed_clone);
            async move {
                observed
                    .lock()
                    .await
                    .push(format!("{}:start", context.payload().label));
                tokio::task::yield_now().await;
                observed
                    .lock()
                    .await
                    .push(format!("{}:end", context.payload().label));
                Ok(())
            }
        },
    )
    .await?;

    let first = bus.publish_with_mode(
        test_event(TestText("first".to_owned()))?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("ordered-same-aggregate-event-1".to_owned()),
        )?,
        DispatchMode::OrderedByAggregateKey,
    );
    let second = bus.publish_with_mode(
        test_event(TestText("second".to_owned()))?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("ordered-same-aggregate-event-2".to_owned()),
        )?,
        DispatchMode::OrderedByAggregateKey,
    );
    let (first_report, second_report) = tokio::join!(first, second);

    assert_eq!(crate::event_count_value(first_report?.handled_count), 1);
    assert_eq!(crate::event_count_value(second_report?.handled_count), 1);
    assert_eq!(
        observed.lock().await.as_slice(),
        &[
            "first:start".to_string(),
            "first:end".to_string(),
            "second:start".to_string(),
            "second:end".to_string()
        ]
    );
    assert_eq!(
        crate::event_count_value(bus.clear_for_test().await.aggregate_gate_count),
        0
    );
    Ok(())
}

#[tokio::test]
async fn ordered_dispatch_allows_different_aggregates_to_run_concurrently(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = Arc::clone(&barrier);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let barrier = Arc::clone(&barrier_clone);
            async move {
                barrier.wait().await;
                Ok(())
            }
        },
    )
    .await?;

    let first = bus.publish_with_mode(
        test_event_with_aggregate(
            TestText("first".to_owned()),
            TestText("aggregate-a".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("ordered-different-aggregate-event-1".to_owned()),
        )?,
        DispatchMode::OrderedByAggregateKey,
    );
    let second = bus.publish_with_mode(
        test_event_with_aggregate(
            TestText("second".to_owned()),
            TestText("aggregate-b".to_owned()),
        )?,
        metadata_with_event_id(
            TestText(TEST_TARGET.to_owned()),
            TestText("ordered-different-aggregate-event-2".to_owned()),
        )?,
        DispatchMode::OrderedByAggregateKey,
    );
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        Box::pin(async { tokio::join!(first, second) }),
    )
    .await?;

    assert_eq!(crate::event_count_value(result.0?.handled_count), 1);
    assert_eq!(crate::event_count_value(result.1?.handled_count), 1);
    assert_eq!(
        crate::event_count_value(bus.clear_for_test().await.aggregate_gate_count),
        0
    );
    Ok(())
}

#[tokio::test]
async fn nested_publish_uses_context_publisher_without_deadlock(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let handled = Arc::new(Mutex::new(Vec::new()));
    let nested_handled = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |context| async move {
            let nested_event = test_event_for_type(
                TestText("nested".to_owned()),
                TestText(OTHER_EVENT_TYPE.to_owned()),
            )
            .map_err(|e| EventingError::InvalidValue {
                field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                    "nested_event",
                ),
                value: enforcer_domain::events_types::EventErrorReason::from_diagnostic(
                    e.to_string(),
                ),
            })?;
            let nested_metadata = metadata_with_event_id(
                TestText(OTHER_TARGET.to_owned()),
                TestText("nested-publish-event-1".to_owned()),
            )
            .map_err(|e| EventingError::InvalidValue {
                field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                    "nested_metadata",
                ),
                value: enforcer_domain::events_types::EventErrorReason::from_diagnostic(
                    e.to_string(),
                ),
            })?;
            context
                .publisher()
                .publish(nested_event, nested_metadata)
                .await?;
            Ok(())
        },
    )
    .await?;
    bus.subscribe::<TestEvent, _, _>(
        subscriber_for_event(
            TestText(OTHER_SUBSCRIBER.to_owned()),
            TestText(OTHER_TARGET.to_owned()),
            TestText(OTHER_EVENT_TYPE.to_owned()),
        )?,
        move |context| {
            let handled = Arc::clone(&nested_handled);
            async move {
                handled.lock().await.push(context.payload().label.clone());
                Ok(())
            }
        },
    )
    .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;

    assert_eq!(crate::event_count_value(report.handled_count), 1);
    assert_eq!(handled.lock().await.as_slice(), &["nested".to_string()]);
    assert_eq!(bus.journal().await.len(), 2);
    Ok(())
}

#[tokio::test]
async fn detached_publish_returns_observable_report(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;

    let report = bus
        .publish_detached(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
            DispatchMode::Sequential,
        )
        .await??;

    assert_eq!(crate::event_count_value(report.subscriber_count), 1);
    assert_eq!(crate::event_count_value(report.handled_count), 1);
    Ok(())
}

#[tokio::test]
async fn sync_subscriber_adapter_uses_typed_dispatch_path(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let handled = Arc::new(StdMutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    let subscription = bus
        .subscribe_sync::<TestEvent, _>(
            subscriber(
                TestText(TEST_SUBSCRIBER.to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            move |context| {
                let Ok(mut guard) = handled_clone.lock() else {
                    return Err(EventingError::EmptyValue {
                        field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                            "sync_handled_lock_poisoned",
                        ),
                    });
                };
                guard.push(context.payload().label.clone());
                Ok(())
            },
        )
        .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;

    assert_eq!(
        subscription.event_type.as_str(),
        super::fixtures::TEST_EVENT_TYPE
    );
    assert_eq!(crate::event_count_value(report.subscriber_count), 1);
    assert_eq!(crate::event_count_value(report.handled_count), 1);
    let Ok(handled_guard) = handled.lock() else {
        return Err("sync handled lock poisoned".into());
    };
    assert_eq!(handled_guard.as_slice(), &[TEST_LABEL.to_string()]);
    Ok(())
}

#[tokio::test]
async fn panicking_handler_isolated_as_dead_letter_report(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async {
            std::panic::resume_unwind(Box::new("eventing test panic"));
        },
    )
    .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(report.handler_reports[0].outcome, HandlerOutcome::Panicked);
    assert_eq!(crate::event_count_value(report.handled_count), 0);
    assert_eq!(crate::event_count_value(report.dead_letter_count), 1);
    assert_eq!(
        dead_letters[0]
            .subscriber_id
            .as_ref()
            .ok_or("handler dead letter has subscriber")?
            .as_str(),
        TEST_SUBSCRIBER
    );
    Ok(())
}

#[tokio::test]
async fn subscription_handle_drop_unsubscribes_handler(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let handled = Arc::new(Mutex::new(0_usize));
    let handled_clone = Arc::clone(&handled);
    let handle = bus
        .subscribe_with_handle::<TestEvent, _, _>(
            subscriber(
                TestText(TEST_SUBSCRIBER.to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            move |_| {
                let handled = Arc::clone(&handled_clone);
                async move {
                    *handled.lock().await += 1;
                    Ok(())
                }
            },
        )
        .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    drop(handle);
    let second_report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;

    assert_eq!(crate::event_count_value(report.handled_count), 1);
    assert_eq!(crate::event_count_value(second_report.subscriber_count), 0);
    assert_eq!(*handled.lock().await, 1);
    Ok(())
}

#[tokio::test]
// CANCELLATION-TEST: registrar_dispose_cancellation_removes_all_owned_subscriptions
async fn registrar_dispose_cancellation_removes_all_owned_subscriptions(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let mut registrar = EventRegistrar::new();
    registrar
        .subscribe::<TestEvent, _, _>(
            &bus,
            subscriber(
                TestText(TEST_SUBSCRIBER.to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            |_| async { Ok(()) },
        )
        .await?;

    let dispose_report = registrar.dispose();
    let publish_report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let subscribe_after_dispose = registrar
        .subscribe::<TestEvent, _, _>(
            &bus,
            subscriber(
                TestText(OTHER_SUBSCRIBER.to_owned()),
                TestText(OTHER_TARGET.to_owned()),
            )?,
            |_| async { Ok(()) },
        )
        .await;

    assert_eq!(dispose_report.reports.len(), 1);
    assert!(matches!(
        dispose_report.reports[0].removal_state,
        enforcer_domain::events_types::SubscriptionRemovalState::Removed
    ));
    assert_eq!(registrar.status(), RegistrarStatus::Disposed);
    assert_eq!(crate::event_count_value(publish_report.subscriber_count), 0);
    assert!(matches!(
        subscribe_after_dispose,
        Err(EventingError::RegistrarDisposed)
    ));
    Ok(())
}
