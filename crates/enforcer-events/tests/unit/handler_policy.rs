use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use super::fixtures::{
    metadata, subscriber, test_event, TestEvent, TestText, TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use crate::{EventBus, EventRecorder, EventingError, HandlerExecutionPolicy};
use enforcer_domain::events_types::HandlerOutcome;

async fn retry_attempt(attempts: Arc<AtomicUsize>) -> Result<(), EventingError> {
    let previous = attempts.fetch_add(1, Ordering::SeqCst);
    if previous == 0 {
        Err(EventingError::EmptyValue {
            field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                "retryable_handler_failure".to_owned(),
            ),
        })
    } else {
        Ok(())
    }
}

async fn timeout_attempt(attempts: Arc<AtomicUsize>) -> Result<(), EventingError> {
    attempts.fetch_add(1, Ordering::SeqCst);
    std::future::pending::<Result<(), EventingError>>().await
}

#[tokio::test]
async fn retry_policy_retries_failed_attempt_and_reports_trace_fields(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus =
        EventBus::with_handler_policy(HandlerExecutionPolicy::new(None, crate::event_count(2))?);
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = Arc::clone(&attempts);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| retry_attempt(Arc::clone(&attempts_clone)),
    )
    .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let handler_report = &report.handler_reports[0];

    assert_eq!(handler_report.outcome, HandlerOutcome::Handled);
    assert_eq!(crate::event_count_value(handler_report.attempts), 2);
    assert_eq!(handler_report.trace.event_id, report.event_id);
    assert_eq!(handler_report.trace.event_type, report.event_type);
    assert_eq!(
        handler_report.trace.correlation_id,
        metadata(TestText(TEST_TARGET.to_owned()))?.correlation_id
    );
    assert_eq!(handler_report.trace.target_handler.as_str(), TEST_TARGET);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn timeout_policy_retries_then_dead_letters_final_timeout(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::with_handler_policy(HandlerExecutionPolicy::new(
        Some(Duration::from_millis(5).into()),
        crate::event_count(2),
    )?);
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = Arc::clone(&attempts);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| timeout_attempt(Arc::clone(&attempts_clone)),
    )
    .await?;

    let report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(report.handler_reports[0].outcome, HandlerOutcome::TimedOut);
    assert_eq!(
        crate::event_count_value(report.handler_reports[0].attempts),
        2
    );
    assert_eq!(crate::event_count_value(report.dead_letter_count), 1);
    assert_eq!(
        dead_letters[0]
            .subscriber_id
            .as_ref()
            .ok_or("handler dead letter has subscriber")?
            .as_str(),
        TEST_SUBSCRIBER
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn event_recorder_uses_real_subscription_and_can_unsubscribe(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let recorder = EventRecorder::<TestEvent>::attach(
        &bus,
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
    )
    .await?;

    let first_report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;
    let recorded = recorder.recorded().await;
    assert!(matches!(
        recorder.unsubscribe(),
        enforcer_domain::events_types::SubscriptionRemovalState::Removed
    ));
    let second_report = bus
        .publish(
            test_event(TestText(TEST_LABEL.to_owned()))?,
            metadata(TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;

    assert_eq!(crate::event_count_value(first_report.handled_count), 1);
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].payload.label, TEST_LABEL);
    assert_eq!(crate::event_count_value(second_report.subscriber_count), 0);
    Ok(())
}
