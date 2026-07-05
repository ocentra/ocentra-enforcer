use enforcer_events::bus::subscriber::EventSubscriber;
use enforcer_events::envelope::{DomainEvent, EventContract, EventMetadata, EventSource};
use enforcer_events::error::EventingError;
use enforcer_events::ids::{
    AggregateKey, CorrelationId, EventCustody, EventId, EventType, IdempotencyKey, RecordedAt,
    RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent, SourceService, SubscriberId,
    TargetHandler,
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(super) struct TestText(pub(super) String);

pub(super) const TEST_EVENT_TYPE: &str = "eventing.test.observed";
pub(super) const OTHER_EVENT_TYPE: &str = "eventing.test.other";
const TEST_EVENT_ID: &str = "event-test-1";
const TEST_CORRELATION_ID: &str = "correlation-test-1";
const TEST_AGGREGATE: &str = "aggregate-test-1";
const TEST_IDEMPOTENCY: &str = "idempotency-test-1";
const TEST_SOURCE_SERVICE: &str = "eventing-test-service";
const TEST_SOURCE_COMPONENT: &str = "eventing-test-component";
const TEST_INSTANCE: &str = "eventing-test-instance";
const TEST_CUSTODY: &str = "local-only";
const TEST_RUNTIME_ROLE: &str = "agent";
pub(super) const TEST_TARGET: &str = "eventing-test-handler";
pub(super) const TEST_SUBSCRIBER: &str = "eventing-test-subscriber";
const TEST_OBSERVED_AT: &str = "2026-06-03T22:30:00Z";
pub(super) const TEST_LABEL: &str = "typed envelope proof";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TestEvent {
    pub(super) label: String,
    aggregate_key: AggregateKey,
    idempotency_key: IdempotencyKey,
    event_type: EventType,
}

impl DomainEvent for TestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            self.event_type.clone(),
            SchemaVersion::new(1)?,
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(self.aggregate_key.clone())
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(self.idempotency_key.clone())
    }
}

pub(super) fn test_event(label: TestText) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_idempotency(
        label,
        TestText(TEST_EVENT_TYPE.to_owned()),
        TestText(TEST_IDEMPOTENCY.to_owned()),
    )
}

pub(super) fn test_event_for_type(
    label: TestText,
    event_type: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_idempotency(label, event_type, TestText(TEST_IDEMPOTENCY.to_owned()))
}

pub(super) fn test_event_with_idempotency(
    label: TestText,
    idempotency_key: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_idempotency(
        label,
        TestText(TEST_EVENT_TYPE.to_owned()),
        idempotency_key,
    )
}

fn test_event_for_type_with_idempotency(
    label: TestText,
    event_type: TestText,
    idempotency_key: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(TestEvent {
        label: label.0,
        aggregate_key: AggregateKey::parse(TEST_AGGREGATE)?,
        idempotency_key: IdempotencyKey::parse(idempotency_key.0)?,
        event_type: EventType::parse(event_type.0)?,
    })
}

pub(super) fn metadata(target: TestText) -> Result<EventMetadata, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventMetadata::from_parts(
        EventId::parse(TEST_EVENT_ID)?,
        CorrelationId::parse(TEST_CORRELATION_ID)?,
        source()?,
        RecordedAt::parse(TEST_OBSERVED_AT)?,
        Some(TargetHandler::parse(target.0)?),
    ))
}

fn source() -> Result<EventSource, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventSource::new(
        EventCustody::parse(TEST_CUSTODY)?,
        RuntimeRole::parse(TEST_RUNTIME_ROLE)?,
        SourceService::parse(TEST_SOURCE_SERVICE)?,
        SourceComponent::parse(TEST_SOURCE_COMPONENT)?,
        RuntimeInstanceId::parse(TEST_INSTANCE)?,
    ))
}

pub(super) fn subscriber(
    id: TestText,
    target: TestText,
) -> Result<EventSubscriber, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventSubscriber::new(
        SubscriberId::parse(id.0)?,
        EventType::parse(TEST_EVENT_TYPE)?,
        TargetHandler::parse(target.0)?,
    ))
}
