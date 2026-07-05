use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use enforcer_events::bus::{subscriber::EventSubscriber, EventBus};
use enforcer_events::envelope::{DomainEvent, EventContract, EventMetadata, EventSource};
use enforcer_events::error::EventingError;
use enforcer_events::ids::{
    AggregateKey, CorrelationId, EventCustody, EventId, EventType, IdempotencyKey, RecordedAt,
    RequestId, RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent, SourceService,
    SubscriberId, TargetHandler,
};
use enforcer_events::request::{EventResponseContract, RequestEvent, RequestOptions};
use serde::{Deserialize, Serialize};

const FIRE_AND_FORGET_EVENT_TYPE: &str = "eventing.integration.fire-and-forget";
const REQUEST_EVENT_TYPE: &str = "eventing.integration.requested";
const SCHEMA_VERSION: u16 = 1;
const AGGREGATE_KEY: &str = "eventing-integration-aggregate";
const FIRE_AND_FORGET_IDEMPOTENCY: &str = "eventing-integration-fire-idempotency";
const REQUEST_IDEMPOTENCY: &str = "eventing-integration-request-idempotency";
const FIRE_AND_FORGET_PAYLOAD_REF: &str = "eventing-integration-fire-payload";
const REQUEST_PAYLOAD_REF: &str = "eventing-integration-request-payload";
const RESPONSE_PAYLOAD_REF: &str = "eventing-integration-response-payload";
const REQUEST_ID: &str = "eventing-integration-request-id";
const EVENT_ID: &str = "eventing-integration-event-id";
const CORRELATION_ID: &str = "eventing-integration-correlation-id";
const OBSERVED_AT: &str = "2026-06-12T12:00:00Z";
const EVENT_CUSTODY: &str = "local-only";
const RUNTIME_ROLE: &str = "child";
const SOURCE_SERVICE: &str = "eventing-integration-service";
const SOURCE_COMPONENT: &str = "eventing-integration-component";
const RUNTIME_INSTANCE_ID: &str = "eventing-integration-runtime";
const FIRE_SUBSCRIBER_ID: &str = "eventing-integration-fire-subscriber";
const REQUEST_SUBSCRIBER_ID: &str = "eventing-integration-request-subscriber";
const TARGET_HANDLER: &str = "eventing-integration-handler";
const RESPONSE_TIMEOUT_MILLIS: u64 = 50;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct IntegrationPayloadRef(String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FireAndForgetEvent {
    payload_ref: IntegrationPayloadRef,
}

impl DomainEvent for FireAndForgetEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        event_contract(TestText(FIRE_AND_FORGET_EVENT_TYPE.to_owned()))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        aggregate_key()
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        IdempotencyKey::parse(FIRE_AND_FORGET_IDEMPOTENCY)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AwaitableRequestEvent {
    request_id: RequestId,
    payload_ref: IntegrationPayloadRef,
}

impl DomainEvent for AwaitableRequestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        event_contract(TestText(REQUEST_EVENT_TYPE.to_owned()))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        aggregate_key()
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        IdempotencyKey::parse(REQUEST_IDEMPOTENCY)
    }
}

impl RequestEvent for AwaitableRequestEvent {
    type Response = AwaitableResponseEvent;

    fn request_id(&self) -> Result<RequestId, EventingError> {
        Ok(self.request_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AwaitableResponseEvent {
    payload_ref: IntegrationPayloadRef,
    accepted: bool,
}

impl EventResponseContract for AwaitableResponseEvent {}

#[tokio::test]
async fn publish_and_wait_dispatches_typed_fire_and_forget_event(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();
    let observed_payload = Arc::new(Mutex::new(None));
    let captured_payload = Arc::clone(&observed_payload);

    bus.subscribe::<FireAndForgetEvent, _, _>(fire_subscriber()?, move |context| {
        let captured_payload = Arc::clone(&captured_payload);
        async move {
            if let Ok(mut guard) = captured_payload.lock() {
                guard.replace(context.payload().payload_ref.clone());
            }
            Ok(())
        }
    })
    .await?;

    let report = bus
        .publish_and_wait(fire_and_forget_event(), metadata()?)
        .await?;

    assert_eq!(report.handled_count, 1);
    let observed = match observed_payload.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err("observed payload mutex was poisoned".into()),
    };
    assert_eq!(
        observed,
        Some(IntegrationPayloadRef(
            FIRE_AND_FORGET_PAYLOAD_REF.to_owned()
        ))
    );

    Ok(())
}

#[tokio::test]
async fn publish_request_waits_for_typed_subscriber_response(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = EventBus::new();

    bus.subscribe::<AwaitableRequestEvent, _, _>(request_subscriber()?, |context| async move {
        context.complete_request(awaitable_response_event()?).await?;
        Ok(())
    })
    .await?;

    let report = bus
        .publish_request(
            awaitable_request_event()?,
            metadata()?,
            RequestOptions::with_timeout(Duration::from_millis(RESPONSE_TIMEOUT_MILLIS))?,
        )
        .await?;

    assert_eq!(report.request_id, RequestId::parse(REQUEST_ID)?);
    assert_eq!(report.response, awaitable_response_event()?);
    assert_eq!(report.publish_report.handled_count, 1);

    Ok(())
}

fn fire_and_forget_event() -> FireAndForgetEvent {
    FireAndForgetEvent {
        payload_ref: IntegrationPayloadRef(FIRE_AND_FORGET_PAYLOAD_REF.to_owned()),
    }
}

fn awaitable_request_event() -> Result<AwaitableRequestEvent, EventingError> {
    Ok(AwaitableRequestEvent {
        request_id: RequestId::parse(REQUEST_ID)?,
        payload_ref: IntegrationPayloadRef(REQUEST_PAYLOAD_REF.to_owned()),
    })
}

fn awaitable_response_event() -> Result<AwaitableResponseEvent, EventingError> {
    Ok(AwaitableResponseEvent {
        payload_ref: IntegrationPayloadRef(RESPONSE_PAYLOAD_REF.to_owned()),
        accepted: true,
    })
}

fn fire_subscriber() -> Result<EventSubscriber, EventingError> {
    subscriber(
        TestText(FIRE_SUBSCRIBER_ID.to_owned()),
        TestText(FIRE_AND_FORGET_EVENT_TYPE.to_owned()),
    )
}

fn request_subscriber() -> Result<EventSubscriber, EventingError> {
    subscriber(
        TestText(REQUEST_SUBSCRIBER_ID.to_owned()),
        TestText(REQUEST_EVENT_TYPE.to_owned()),
    )
}

#[derive(Clone)]
pub(super) struct TestText(pub(super) String);

fn subscriber(id: TestText, event_type: TestText) -> Result<EventSubscriber, EventingError> {
    Ok(EventSubscriber::new(
        SubscriberId::parse(id.0)?,
        EventType::parse(event_type.0)?,
        TargetHandler::parse(TARGET_HANDLER)?,
    ))
}

fn metadata() -> Result<EventMetadata, EventingError> {
    Ok(EventMetadata::from_parts(
        EventId::parse(EVENT_ID)?,
        CorrelationId::parse(CORRELATION_ID)?,
        EventSource::new(
            EventCustody::parse(EVENT_CUSTODY)?,
            RuntimeRole::parse(RUNTIME_ROLE)?,
            SourceService::parse(SOURCE_SERVICE)?,
            SourceComponent::parse(SOURCE_COMPONENT)?,
            RuntimeInstanceId::parse(RUNTIME_INSTANCE_ID)?,
        ),
        RecordedAt::parse(OBSERVED_AT)?,
        Some(TargetHandler::parse(TARGET_HANDLER)?),
    ))
}

fn aggregate_key() -> Result<AggregateKey, EventingError> {
    AggregateKey::parse(AGGREGATE_KEY)
}

fn event_contract(event_type: TestText) -> Result<EventContract, EventingError> {
    Ok(EventContract::new(
        EventType::parse(event_type.0)?,
        SchemaVersion::new(SCHEMA_VERSION)?,
    ))
}
