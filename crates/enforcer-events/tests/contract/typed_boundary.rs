use super::support::{
    metadata, subscriber, test_event, TestEvent, TestText, OTHER_EVENT_TYPE, OTHER_SUBSCRIBER,
    OTHER_TARGET, TEST_LABEL, TEST_SUBSCRIBER, TEST_TARGET,
};
use enforcer_events::bus::{EventBus, ShutdownMode};
use enforcer_events::envelope::{EventEnvelope, EventPriority};
use enforcer_events::error::EventingError;
use enforcer_events::ids::{CausationId, EventNamespace, EventType, RecordedAt, SchemaVersion};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn event_bus_dispatches_typed_envelope_and_stores_serialized_boundary(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let handled = Arc::new(Mutex::new(Vec::new()));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            &TestText(TEST_SUBSCRIBER.to_owned()),
            &TestText(TEST_TARGET.to_owned()),
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

    let metadata = metadata(&TestText(TEST_TARGET.to_owned()))?
        .with_causation_id(CausationId::parse("causation-test-1")?)
        .with_priority(EventPriority::High);
    let report = bus
        .publish(test_event(&TestText(TEST_LABEL.to_owned()))?, metadata)
        .await?;
    let journal = bus.journal().await;
    let decoded: EventEnvelope<TestEvent> = journal[0].decode()?;

    assert_eq!(report.subscriber_count, 1);
    assert_eq!(report.handled_count, 1);
    assert_eq!(handled.lock().await.as_slice(), &[TEST_LABEL.to_string()]);
    assert_eq!(decoded.payload.label, TEST_LABEL);
    assert_eq!(
        decoded
            .causation_id
            .as_ref()
            .ok_or("causation id is stored")?
            .as_str(),
        "causation-test-1"
    );
    assert_eq!(decoded.priority, EventPriority::High);
    assert_eq!(journal.len(), 1);
    Ok(())
}

#[tokio::test]
async fn target_handler_filter_prevents_wrong_handler_delivery(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let handled = Arc::new(Mutex::new(0_usize));
    let handled_clone = Arc::clone(&handled);
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            &TestText(TEST_SUBSCRIBER.to_owned()),
            &TestText(TEST_TARGET.to_owned()),
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
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            &TestText(OTHER_SUBSCRIBER.to_owned()),
            &TestText(OTHER_TARGET.to_owned()),
        )?,
        |_| async { Ok(()) },
    )
    .await?;

    let report = bus
        .publish(
            test_event(&TestText(TEST_LABEL.to_owned()))?,
            metadata(&TestText(TEST_TARGET.to_owned()))?,
        )
        .await?;

    assert_eq!(report.subscriber_count, 1);
    assert_eq!(*handled.lock().await, 1);
    Ok(())
}

#[tokio::test]
async fn shutdown_cancels_future_event_delivery(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();

    let report = bus.shutdown(ShutdownMode::Drain).await?;
    let publish_after_shutdown = bus
        .publish(
            test_event(&TestText(TEST_LABEL.to_owned()))?,
            metadata(&TestText(TEST_TARGET.to_owned()))?,
        )
        .await;

    assert_eq!(report.mode, ShutdownMode::Drain);
    assert!(!report.already_shutdown);
    assert_eq!(report.queued_event_count, 0);
    assert_eq!(publish_after_shutdown, Err(EventingError::BusShutdown));
    Ok(())
}

#[tokio::test]
async fn concurrent_dispatch_records_handler_dead_letter_without_losing_journal(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            &TestText(TEST_SUBSCRIBER.to_owned()),
            &TestText(TEST_TARGET.to_owned()),
        )?,
        |_| async {
            Err(EventingError::InvalidValue {
                field: "handler_failure",
                value: "handler_failure".to_string(),
            })
        },
    )
    .await?;

    let report = bus
        .publish_with_mode(
            test_event(&TestText(TEST_LABEL.to_owned()))?,
            metadata(&TestText(TEST_TARGET.to_owned()))?,
            enforcer_events::bus::DispatchMode::Concurrent,
        )
        .await?;
    let dead_letters = bus.dead_letters().await;

    assert_eq!(report.dead_letter_count, 1);
    assert_eq!(report.handled_count, 0);
    assert_eq!(bus.journal().await.len(), 1);
    assert_eq!(
        dead_letters[0]
            .target_handler
            .as_ref()
            .ok_or("handler dead letter has target")?
            .as_str(),
        TEST_TARGET
    );
    Ok(())
}

#[tokio::test]
async fn duplicate_subscriber_ids_are_rejected(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let duplicate = subscriber(
        &TestText(TEST_SUBSCRIBER.to_owned()),
        &TestText(TEST_TARGET.to_owned()),
    )?;
    bus.subscribe::<TestEvent, _, _>(duplicate.clone(), |_| async { Ok(()) })
        .await?;

    let result = bus
        .subscribe::<TestEvent, _, _>(duplicate, |_| async { Ok(()) })
        .await;

    assert!(matches!(
        result,
        Err(EventingError::DuplicateSubscriber { .. })
    ));
    Ok(())
}

#[test]
fn eventing_newtypes_reject_empty_values_and_zero_versions(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    assert_eq!(
        EventType::parse(""),
        Err(EventingError::EmptyValue {
            field: "event_type"
        })
    );
    for invalid_taxonomy in [".leading", "trailing.", "empty..segment"] {
        assert_eq!(
            EventType::parse(invalid_taxonomy),
            Err(EventingError::InvalidValue {
                field: "event_type",
                value: invalid_taxonomy.to_string(),
            })
        );
    }
    assert_eq!(
        EventType::parse("eventing/slash-taxonomy/observed")?.as_str(),
        "eventing/slash-taxonomy/observed"
    );
    assert_eq!(
        RecordedAt::parse(" "),
        Err(EventingError::EmptyValue {
            field: "recorded_at"
        })
    );
    assert_eq!(SchemaVersion::new(0), Err(EventingError::InvalidVersion));
    Ok(())
}

#[test]
fn event_namespaces_match_dot_and_slash_event_taxonomy(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let slash_event = EventType::parse("network/transport/observed")?;
    let dot_event = EventType::parse("network.transport.observed")?;
    let network_namespace = EventNamespace::parse("network")?;

    assert_eq!(
        EventNamespace::from_event_type(&slash_event)?.as_str(),
        "network"
    );
    assert!(network_namespace.matches_event_type(&slash_event));
    assert!(network_namespace.matches_event_type(&dot_event));
    Ok(())
}

#[test]
fn stored_decode_rejects_contract_mismatch() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    let envelope = EventEnvelope::from_event(
        test_event(&TestText(TEST_LABEL.to_owned()))?,
        metadata(&TestText(TEST_TARGET.to_owned()))?,
    )?;
    let mut stored = envelope.store()?;
    stored.contract.event_type = EventType::parse(OTHER_EVENT_TYPE)?;

    let decoded = stored.decode::<super::support::TestEvent>();

    assert!(matches!(
        decoded,
        Err(EventingError::ContractMismatch { .. })
    ));
    Ok(())
}
