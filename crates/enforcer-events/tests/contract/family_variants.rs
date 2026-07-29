use super::support::TestText;
use enforcer_domain::events_types::{
    AggregateKey, CorrelationId, EventCustody, EventErrorReason, EventType, IdempotencyKey,
    RecordedAt, RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent, SourceService,
    SubscriberId, TargetHandler,
};
use enforcer_events::bus::subscriber::EventSubscriber;
use enforcer_events::bus::EventBus;
use enforcer_events::contract_registry::EventContractRegistry;
use enforcer_events::envelope::{
    DomainEvent, EventContract, EventFrame, EventMetadata, EventSource,
};
use enforcer_events::error::EventingError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const APPROVED_EVENT_TYPE: &str = "eventing.family.decision.approved";
const REJECTED_EVENT_TYPE: &str = "eventing.family.decision.rejected";
const APPROVED_LABEL: &str = "approved";
const REJECTED_LABEL: &str = "rejected";
const FAMILY_AGGREGATE: &str = "family-decision-aggregate";
const APPROVED_IDEMPOTENCY: &str = "family-approved-idempotency";
const REJECTED_IDEMPOTENCY: &str = "family-rejected-idempotency";
const FAMILY_CORRELATION: &str = "family-correlation";
const FAMILY_EVENT_ID: &str = "family-event-1";
const FAMILY_OBSERVED_AT: &str = "2026-06-04T02:45:00Z";
const FAMILY_SOURCE_SERVICE: &str = "family-service";
const FAMILY_SOURCE_COMPONENT: &str = "family-component";
const FAMILY_INSTANCE: &str = "family-instance";
const FAMILY_CUSTODY: &str = "local-only";
const FAMILY_RUNTIME_ROLE: &str = "agent";
const FAMILY_TARGET: &str = "family-target";
const APPROVED_SUBSCRIBER: &str = "family-approved-subscriber";
const REJECTED_SUBSCRIBER: &str = "family-rejected-subscriber";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct DecisionPayload {
    label: String,
    aggregate_key: String,
    idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family_variant", content = "payload", rename_all = "kebab-case")]
enum DecisionFamilyEvent {
    Approved(DecisionPayload),
    Rejected(DecisionPayload),
}

impl DomainEvent for DecisionFamilyEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        let event_type = match self {
            Self::Approved(_) => APPROVED_EVENT_TYPE,
            Self::Rejected(_) => REJECTED_EVENT_TYPE,
        };
        Ok(EventContract::new(
            EventType::parse(event_type)?,
            SchemaVersion::try_new(std::num::NonZeroU16::MIN),
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(AggregateKey::parse(&decision_payload(self).aggregate_key)?)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(IdempotencyKey::parse(
            &decision_payload(self).idempotency_key,
        )?)
    }
}

#[tokio::test]
async fn family_subscriber_receives_typed_enum_variants_without_downcast(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let received = Arc::new(Mutex::new(Vec::<TestText>::new()));

    let approved_seen = Arc::clone(&received);
    bus.subscribe::<DecisionFamilyEvent, _, _>(
        family_subscriber(
            TestText(APPROVED_SUBSCRIBER.to_owned()),
            TestText(APPROVED_EVENT_TYPE.to_owned()),
        )?,
        move |context| {
            let approved_seen = Arc::clone(&approved_seen);
            async move {
                let DecisionFamilyEvent::Approved(payload) = context.payload() else {
                    return Err(EventingError::InvalidValue {
                        field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                            "family_variant".to_owned(),
                        ),
                        value: EventErrorReason::parse(
                            "approved subscriber received a non-approved variant",
                        )?,
                    });
                };
                record_payload(&approved_seen, payload)
            }
        },
    )
    .await?;

    let rejected_seen = Arc::clone(&received);
    bus.subscribe::<DecisionFamilyEvent, _, _>(
        family_subscriber(
            TestText(REJECTED_SUBSCRIBER.to_owned()),
            TestText(REJECTED_EVENT_TYPE.to_owned()),
        )?,
        move |context| {
            let rejected_seen = Arc::clone(&rejected_seen);
            async move {
                let DecisionFamilyEvent::Rejected(payload) = context.payload() else {
                    return Err(EventingError::InvalidValue {
                        field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                            "family_variant".to_owned(),
                        ),
                        value: EventErrorReason::parse(
                            "rejected subscriber received a non-rejected variant",
                        )?,
                    });
                };
                record_payload(&rejected_seen, payload)
            }
        },
    )
    .await?;

    bus.publish(approved_event()?, family_metadata()?).await?;
    bus.publish(rejected_event()?, family_metadata()?).await?;

    let Ok(received_guard) = received.lock() else {
        return Err("received lock: mutex poisoned".into());
    };
    assert_eq!(
        received_guard.as_slice(),
        [
            TestText(APPROVED_LABEL.to_string()),
            TestText(REJECTED_LABEL.to_string())
        ]
    );
    Ok(())
}

#[test]
fn family_variant_stored_decode_rejects_contract_variant_mismatch(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let envelope = EventFrame::from_event(approved_event()?, family_metadata()?)?;
    let mut stored = envelope.store()?;
    stored.contract = EventContract::new(
        EventType::parse(REJECTED_EVENT_TYPE)?,
        SchemaVersion::try_new(std::num::NonZeroU16::MIN),
    );

    assert!(matches!(
        stored.decode::<DecisionFamilyEvent>(),
        Err(EventingError::ContractMismatch { .. })
    ));
    Ok(())
}

#[test]
fn family_variants_register_as_distinct_contract_descriptors(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut registry = EventContractRegistry::new();
    registry.register_event(&approved_event()?)?;
    registry.register_event(&rejected_event()?)?;

    let event_types = registry
        .descriptors()
        .map(|descriptor| descriptor.event_type().as_str().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        vec![
            APPROVED_EVENT_TYPE.to_string(),
            REJECTED_EVENT_TYPE.to_string()
        ]
    );
    Ok(())
}

fn approved_event() -> Result<DecisionFamilyEvent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(DecisionFamilyEvent::Approved(DecisionPayload {
        label: APPROVED_LABEL.to_string(),
        aggregate_key: FAMILY_AGGREGATE.to_owned(),
        idempotency_key: APPROVED_IDEMPOTENCY.to_owned(),
    }))
}

fn rejected_event() -> Result<DecisionFamilyEvent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(DecisionFamilyEvent::Rejected(DecisionPayload {
        label: REJECTED_LABEL.to_string(),
        aggregate_key: FAMILY_AGGREGATE.to_owned(),
        idempotency_key: REJECTED_IDEMPOTENCY.to_owned(),
    }))
}

fn decision_payload(event: &DecisionFamilyEvent) -> &DecisionPayload {
    match event {
        DecisionFamilyEvent::Approved(payload) | DecisionFamilyEvent::Rejected(payload) => payload,
    }
}

fn record_payload(
    received: &Arc<Mutex<Vec<TestText>>>,
    payload: &DecisionPayload,
) -> Result<(), EventingError> {
    let Ok(mut guard) = received.lock() else {
        return Err(EventingError::InvalidValue {
            field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                "received".to_owned(),
            ),
            value: EventErrorReason::parse("mutex poisoned")?,
        });
    };
    guard.push(TestText(payload.label.clone()));
    Ok(())
}

fn family_metadata() -> Result<EventMetadata, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventMetadata::from_parts(
        enforcer_domain::events_types::EventId::parse(FAMILY_EVENT_ID)?,
        CorrelationId::parse(FAMILY_CORRELATION)?,
        EventSource::new(
            EventCustody::parse(FAMILY_CUSTODY)?,
            RuntimeRole::parse(FAMILY_RUNTIME_ROLE)?,
            SourceService::parse(FAMILY_SOURCE_SERVICE)?,
            SourceComponent::parse(FAMILY_SOURCE_COMPONENT)?,
            RuntimeInstanceId::parse(FAMILY_INSTANCE)?,
        ),
        RecordedAt::parse(FAMILY_OBSERVED_AT)?,
        Some(TargetHandler::parse(FAMILY_TARGET)?),
    ))
}

fn family_subscriber(
    id: TestText,
    event_type: TestText,
) -> Result<EventSubscriber, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventSubscriber::new(
        SubscriberId::parse(&{ id.0 })?,
        EventType::parse(&{ event_type.0 })?,
        TargetHandler::parse(FAMILY_TARGET)?,
    ))
}
