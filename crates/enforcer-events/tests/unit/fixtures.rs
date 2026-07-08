use serde::{Deserialize, Serialize};

use crate::{
    AggregateKey, CorrelationId, DomainEvent, EventContract, EventCustody, EventMetadata,
    EventSource, EventSubscriber, EventType, EventingError, IdempotencyKey, RecordedAt,
    RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent, SourceService, SubscriberId,
    TargetHandler,
};

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
pub(super) const OTHER_TARGET: &str = "eventing-other-handler";
pub(super) const TEST_SUBSCRIBER: &str = "eventing-test-subscriber";
pub(super) const OTHER_SUBSCRIBER: &str = "eventing-other-subscriber";
const TEST_OBSERVED_AT: &str = "2026-06-03T22:30:00Z";
pub(super) const TEST_LABEL: &str = "typed envelope proof";

#[derive(Clone)]
pub(super) struct TestText(pub(super) String);

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

pub(super) fn test_event(
    label: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_with_aggregate(label, TestText(TEST_AGGREGATE.to_owned()))
}

pub(super) fn test_event_with_aggregate(
    label: TestText,
    aggregate_key: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_aggregate(label, aggregate_key, TestText(TEST_EVENT_TYPE.to_owned()))
}

pub(super) fn test_event_for_type(
    label: TestText,
    event_type: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_aggregate(label, TestText(TEST_AGGREGATE.to_owned()), event_type)
}

pub(super) fn test_event_with_idempotency(
    label: TestText,
    idempotency_key: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_aggregate_and_idempotency(
        label,
        TestText(TEST_AGGREGATE.to_owned()),
        TestText(TEST_EVENT_TYPE.to_owned()),
        idempotency_key,
    )
}

fn test_event_for_type_with_aggregate(
    label: TestText,
    aggregate_key: TestText,
    event_type: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_event_for_type_with_aggregate_and_idempotency(
        label,
        aggregate_key,
        event_type,
        TestText(TEST_IDEMPOTENCY.to_owned()),
    )
}

pub(super) fn test_event_for_type_with_aggregate_and_idempotency(
    label: TestText,
    aggregate_key: TestText,
    event_type: TestText,
    idempotency_key: TestText,
) -> Result<TestEvent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(TestEvent {
        label: label.0,
        aggregate_key: AggregateKey::parse(aggregate_key.0)?,
        idempotency_key: IdempotencyKey::parse(idempotency_key.0)?,
        event_type: EventType::parse(event_type.0)?,
    })
}

pub(super) fn metadata(
    target: TestText,
) -> Result<EventMetadata, Box<dyn std::error::Error + Send + Sync>> {
    metadata_with_event_id(target, TestText(TEST_EVENT_ID.to_owned()))
}

pub(super) fn metadata_with_event_id(
    target: TestText,
    event_id: TestText,
) -> Result<EventMetadata, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventMetadata::from_parts(
        crate::EventId::parse(event_id.0)?,
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
    subscriber_for_event(id, target, TestText(TEST_EVENT_TYPE.to_owned()))
}

pub(super) fn subscriber_for_event(
    id: TestText,
    target: TestText,
    event_type: TestText,
) -> Result<EventSubscriber, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventSubscriber::new(
        SubscriberId::parse(id.0)?,
        EventType::parse(event_type.0)?,
        TargetHandler::parse(target.0)?,
    ))
}
