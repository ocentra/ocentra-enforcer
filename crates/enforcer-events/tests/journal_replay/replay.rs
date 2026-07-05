use enforcer_events::bus::{DispatchMode, EventBus};
use enforcer_events::error::EventingError;
use enforcer_events::ids::CorrelationId;
use enforcer_events::journal::ndjson::{NdjsonEventJournal, NdjsonJournalOptions};
use enforcer_events::journal::policy::{JournalPolicy, JournalSelector};
use enforcer_events::journal::EventJournal;
use enforcer_events::queue::policy::EventQueuePolicy;
use enforcer_events::replay::{ReplayCursor, ReplayFilter, ReplayMode};
use std::sync::Arc;

use super::fixtures::{
    metadata, subscriber, test_event, test_event_for_type, test_event_with_idempotency, TestEvent,
    TestText, OTHER_EVENT_TYPE, TEST_EVENT_TYPE, TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use super::support::{
    cleanup, event_type, journal_path, stored_event, tamper_first_journal_payload_label,
    TestText as SupportText,
};

#[tokio::test]
async fn replay_cursor_and_filters_read_ordered_projection_records(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = journal_path(SupportText("replay-filters".to_owned()));
    let journal = NdjsonEventJournal::new(&path);
    let first = stored_event(test_event(TestText(TEST_LABEL.to_owned()))?)?;
    let second = stored_event(test_event_for_type(
        TestText("other".to_owned()),
        TestText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;
    let mut third = stored_event(test_event(TestText("third".to_owned()))?)?;
    third.correlation_id = CorrelationId::parse("correlation-replay-3")?;

    journal.append(&first).await?;
    journal.append(&second).await?;
    journal.append(&third).await?;

    let report = journal
        .replay_projection(
            ReplayFilter::for_event_type(event_type(SupportText(TEST_EVENT_TYPE.to_owned()))?)
                .with_correlation_id(third.correlation_id.clone())
                .with_cursor(ReplayCursor::after(1)),
        )
        .await?;

    assert_eq!(report.mode, ReplayMode::ProjectionOnly);
    assert_eq!(report.records.len(), 1);
    assert_eq!(report.records[0].sequence, 3);
    assert_eq!(report.records[0].envelope.event_id, third.event_id);
    assert_eq!(report.cursor.next_sequence, 4);
    assert_eq!(report.skipped_count, 2);
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn replay_corrupt_line_is_reported_explicitly() -> Result<(), Box<dyn std::error::Error>> {
    let path = journal_path(SupportText("corrupt-line".to_owned()));
    tokio::fs::write(&path, "not-json\n").await?;
    let journal = NdjsonEventJournal::new(&path);

    let result = journal.replay_projection(ReplayFilter::all()).await;

    assert!(matches!(
        result,
        Err(EventingError::JournalCorruptLine { line: 1, .. })
    ));
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn replay_rejects_tampered_hash_chain_payload() -> Result<(), Box<dyn std::error::Error>> {
    let path = journal_path(SupportText("replay-tampered-hash-chain".to_owned()));
    let journal = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    journal
        .append(&stored_event(test_event(TestText(TEST_LABEL.to_owned()))?)?)
        .await?;
    journal
        .append(&stored_event(test_event_for_type(
            TestText("second event".to_owned()),
            TestText(OTHER_EVENT_TYPE.to_owned()),
        )?)?)
        .await?;
    tamper_first_journal_payload_label(path.clone(), SupportText("tampered event".to_owned()))
        .await?;

    let result = journal.replay_projection(ReplayFilter::all()).await;

    let Err(EventingError::JournalCorruptLine { line: 1, reason }) = result else {
        return Err("expected a corrupt-line error at line 1 for the tampered payload".into());
    };
    assert_eq!(
        reason.split(": expected ").next(),
        Some("journal hash-chain current hash mismatch at sequence 1")
    );
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn action_replay_dispatches_queued_drain_event_once() -> Result<(), Box<dyn std::error::Error>>
{
    let path = journal_path(SupportText("queued-drain-action-replay".to_owned()));
    let journal = NdjsonEventJournal::new(&path);
    let bus = EventBus::with_journal_and_queue_policy(
        JournalPolicy::before_and_after_dispatch(JournalSelector::All),
        journal.clone().shared(),
        EventQueuePolicy::no_subscriber_queue(2)?,
    );
    bus.publish(
        test_event_with_idempotency(
            TestText(TEST_LABEL.to_owned()),
            TestText("queued-drain-replay-key".to_owned()),
        )?,
        metadata(TestText(TEST_TARGET.to_owned()))?,
    )
    .await?;
    let handled = Arc::new(tokio::sync::Mutex::new(0_usize));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
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
    assert_eq!(*handled.lock().await, 1);

    let action = journal.replay_action_records(ReplayFilter::all()).await?;
    let projection = journal.replay_projection(ReplayFilter::all()).await?;
    let replay_bus = EventBus::new();
    let replay_handled = Arc::new(tokio::sync::Mutex::new(0_usize));
    let replay_handled_clone = Arc::clone(&replay_handled);
    replay_bus
        .subscribe::<TestEvent, _, _>(
            subscriber(
                TestText("replay-subscriber".to_owned()),
                TestText(TEST_TARGET.to_owned()),
            )?,
            move |_| {
                let handled = Arc::clone(&replay_handled_clone);
                async move {
                    *handled.lock().await += 1;
                    Ok(())
                }
            },
        )
        .await?;
    let reports = replay_bus
        .replay_to_handlers(action.records, action.mode, DispatchMode::Sequential)
        .await?;

    assert_eq!(projection.records.len(), 2);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].handled_count, 1);
    assert_eq!(*replay_handled.lock().await, 1);
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn projection_replay_cannot_run_handlers_without_action_mode(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = journal_path(SupportText("projection-gate".to_owned()));
    let journal = NdjsonEventJournal::new(&path);
    journal
        .append(&stored_event(test_event(TestText(TEST_LABEL.to_owned()))?)?)
        .await?;
    let projection = journal.replay_projection(ReplayFilter::all()).await?;
    let bus = EventBus::new();

    let blocked = bus
        .replay_to_handlers(
            projection.records.clone(),
            projection.mode,
            DispatchMode::Sequential,
        )
        .await;
    assert!(matches!(
        blocked,
        Err(EventingError::ReplayActionNotAllowed { .. })
    ));

    let handled = Arc::new(tokio::sync::Mutex::new(0_usize));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
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
    let action = journal.replay_action_records(ReplayFilter::all()).await?;
    let reports = bus
        .replay_to_handlers(action.records, action.mode, DispatchMode::Sequential)
        .await?;

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].handled_count, 1);
    assert_eq!(*handled.lock().await, 1);
    cleanup(path).await;
    Ok(())
}
