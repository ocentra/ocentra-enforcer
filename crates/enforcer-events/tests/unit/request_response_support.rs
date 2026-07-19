use serde::{Deserialize, Serialize};

use crate::{
    AggregateKey, DomainEvent, EventContract, EventResponseContract, EventingError, IdempotencyKey,
    RequestEvent, RequestId, SchemaVersion,
};

#[derive(Clone)]
pub(super) struct TestText(pub(super) String);

pub(super) const REQUEST_EVENT_TYPE: &str = "eventing.test.requested";
pub(super) const RESULT_EVENT_TYPE: &str = "eventing.test.completed";
const REQUEST_AGGREGATE: &str = "request-aggregate";
pub(super) const REQUEST_ID: &str = "request-response-id";
const REQUEST_IDEMPOTENCY: &str = "request-idempotency";
const RESULT_IDEMPOTENCY: &str = "request-result-idempotency";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TestRequestEvent {
    label: String,
    request_id: String,
}

impl DomainEvent for TestRequestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            crate::EventType::parse(REQUEST_EVENT_TYPE)?,
            SchemaVersion::try_new(std::num::NonZeroU16::MIN),
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(AggregateKey::parse(REQUEST_AGGREGATE)?)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(IdempotencyKey::parse(REQUEST_IDEMPOTENCY)?)
    }
}

impl RequestEvent for TestRequestEvent {
    type Response = TestResponse;

    fn request_id(&self) -> Result<RequestId, EventingError> {
        Ok(RequestId::parse(&self.request_id)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct InvalidContractRequestEvent {
    request_id: String,
}

impl InvalidContractRequestEvent {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            request_id: REQUEST_ID.to_owned(),
        })
    }
}

impl DomainEvent for InvalidContractRequestEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Err(EventingError::InvalidVersion)
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(AggregateKey::parse(REQUEST_AGGREGATE)?)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(IdempotencyKey::parse(REQUEST_IDEMPOTENCY)?)
    }
}

impl RequestEvent for InvalidContractRequestEvent {
    type Response = TestResponse;

    fn request_id(&self) -> Result<RequestId, EventingError> {
        Ok(RequestId::parse(&self.request_id)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TestResponse {
    pub(super) decision: String,
}

impl TestResponse {
    pub(super) fn approved() -> Self {
        Self {
            decision: String::from("approved"),
        }
    }

    pub(super) fn invalid() -> Self {
        Self {
            decision: String::from(" "),
        }
    }
}

impl EventResponseContract for TestResponse {
    fn validate(&self) -> Result<(), EventingError> {
        if self.decision.trim().is_empty() {
            return Err(EventingError::EmptyValue {
                field: enforcer_domain::events_types::EventErrorField::from_diagnostic(
                    "test_response_decision",
                ),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TestResultEvent {
    label: String,
}

impl DomainEvent for TestResultEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            crate::EventType::parse(RESULT_EVENT_TYPE)?,
            SchemaVersion::try_new(std::num::NonZeroU16::MIN),
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(AggregateKey::parse(REQUEST_AGGREGATE)?)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(IdempotencyKey::parse(RESULT_IDEMPOTENCY)?)
    }
}

pub(super) fn test_request(
    label: TestText,
) -> Result<TestRequestEvent, Box<dyn std::error::Error + Send + Sync>> {
    test_request_with_id(label, TestText(REQUEST_ID.to_owned()))
}

pub(super) fn test_request_with_id(
    label: TestText,
    request_id: TestText,
) -> Result<TestRequestEvent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(TestRequestEvent {
        label: label.0,
        request_id: request_id.0,
    })
}

pub(super) fn test_result_event() -> TestResultEvent {
    TestResultEvent {
        label: String::from("durable-result"),
    }
}
