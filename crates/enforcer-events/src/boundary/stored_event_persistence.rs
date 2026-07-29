//! Durable payload and stored-envelope persistence values.
//!
//! BOUNDARY-INVARIANT: stored JSON is decoded and contract-checked before it
//! becomes a live event envelope.
//! BOUNDARY-TEST: stored envelope round-trip and malformed payload tests cover
//! this conversion boundary.
//! ROUNDTRIP-TEST: `tests/unit/envelope.rs` persists and restores stored envelopes.
//! NEGATIVE-TEST: `tests/unit/envelope.rs` rejects malformed DTO values and contract mismatch.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use enforcer_domain::events_types::{
    AggregateKey, CausationId, CorrelationId, EventId, EventPriority, IdempotencyKey, RecordedAt,
    TargetHandler,
};

use crate::{
    boundary::{
        envelope_persistence::parse_event_value,
        event_contract_persistence::{EventContractDto, EventSourceDto},
        event_metadata_persistence::EventEnvelopeDto,
    },
    clock::EventClockInstant,
    envelope::{DomainEvent, EventContract, EventFrame, EventSource},
    error::EventingError,
};

/// Opaque durable JSON representation of an event payload.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredEventPayload {
    value: serde_json::Value,
}

impl StoredEventPayload {
    fn from_event<E: Serialize>(payload: &E) -> Result<Self, EventingError> {
        Ok(Self {
            value: serde_json::to_value(payload)
                .map_err(|error| EventingError::payload_encode(&error))?,
        })
    }

    fn decode<E: DeserializeOwned>(&self) -> Result<E, serde_json::Error> {
        serde_json::from_value(self.value.clone())
    }
}

/// Durable typed event record; JSON conversion is kept in this boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredEventEnvelope {
    pub contract: EventContract,
    pub event_id: EventId,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub aggregate_key: AggregateKey,
    pub idempotency_key: IdempotencyKey,
    pub source: EventSource,
    pub observed_at: RecordedAt,
    pub target_handler: Option<TargetHandler>,
    pub priority: EventPriority,
    pub deadline: Option<EventClockInstant>,
    pub payload: StoredEventPayload,
}

impl<E: DomainEvent> EventFrame<E> {
    /// Converts this live envelope into its durable representation.
    pub fn store(&self) -> Result<StoredEventEnvelope, EventingError>
    where
        E: Serialize,
    {
        Ok(StoredEventEnvelope {
            contract: self.contract.clone(),
            event_id: self.event_id.clone(),
            correlation_id: self.correlation_id.clone(),
            causation_id: self.causation_id.clone(),
            aggregate_key: self.aggregate_key.clone(),
            idempotency_key: self.idempotency_key.clone(),
            source: self.source.clone(),
            observed_at: self.observed_at.clone(),
            target_handler: self.target_handler.clone(),
            priority: self.priority,
            deadline: self.deadline,
            payload: StoredEventPayload::from_event(&self.payload)?,
        })
    }
}

impl StoredEventEnvelope {
    /// Decodes and contract-checks the durable payload.
    pub fn decode<E: DomainEvent + DeserializeOwned>(
        &self,
    ) -> Result<EventFrame<E>, EventingError> {
        let payload: E = self.payload.decode().map_err(|error| {
            EventingError::payload_decode(self.contract.event_type.clone(), &error)
        })?;
        let expected = payload.contract()?;
        if expected != self.contract {
            return Err(EventingError::ContractMismatch {
                expected: expected.event_type,
                received: self.contract.event_type.clone(),
                expected_schema_version: expected.schema_version,
                received_schema_version: self.contract.schema_version,
            });
        }
        Ok(EventFrame {
            contract: self.contract.clone(),
            event_id: self.event_id.clone(),
            correlation_id: self.correlation_id.clone(),
            causation_id: self.causation_id.clone(),
            aggregate_key: self.aggregate_key.clone(),
            idempotency_key: self.idempotency_key.clone(),
            source: self.source.clone(),
            observed_at: self.observed_at.clone(),
            target_handler: self.target_handler.clone(),
            priority: self.priority,
            deadline: self.deadline,
            payload,
        })
    }

    /// Reports whether the stored deadline has elapsed.
    pub fn is_deadline_expired(&self, now: EventClockInstant) -> bool {
        self.deadline.is_some_and(|deadline| now >= deadline)
    }
}

/// JSON payload DTO for a durable event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StoredEventPayloadDto(pub serde_json::Value);

impl From<&StoredEventPayload> for StoredEventPayloadDto {
    fn from(value: &StoredEventPayload) -> Self {
        Self(value.value.clone())
    }
}

impl From<StoredEventPayloadDto> for StoredEventPayload {
    fn from(value: StoredEventPayloadDto) -> Self {
        Self { value: value.0 }
    }
}

/// JSON DTO for a durable event envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StoredEventEnvelopeDto(pub EventEnvelopeDto<StoredEventPayloadDto>);

impl From<&StoredEventEnvelope> for StoredEventEnvelopeDto {
    fn from(value: &StoredEventEnvelope) -> Self {
        Self(EventEnvelopeDto {
            contract: EventContractDto::from(&value.contract),
            event_id: value.event_id.as_str().to_owned(),
            correlation_id: value.correlation_id.as_str().to_owned(),
            causation_id: value.causation_id.as_ref().map(|id| id.as_str().to_owned()),
            aggregate_key: value.aggregate_key.as_str().to_owned(),
            idempotency_key: value.idempotency_key.as_str().to_owned(),
            source: EventSourceDto::from(&value.source),
            observed_at: value.observed_at.as_str().to_owned(),
            target_handler: value
                .target_handler
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            priority: value.priority.into(),
            deadline: value.deadline,
            payload: StoredEventPayloadDto::from(&value.payload),
        })
    }
}

impl TryFrom<StoredEventEnvelopeDto> for StoredEventEnvelope {
    type Error = EventingError;

    fn try_from(value: StoredEventEnvelopeDto) -> Result<Self, Self::Error> {
        let value = value.0;
        Ok(Self {
            contract: value.contract.try_into()?,
            event_id: parse_event_value(value.event_id, "event_id")?,
            correlation_id: parse_event_value(value.correlation_id, "correlation_id")?,
            causation_id: value
                .causation_id
                .map(|id| parse_event_value(id, "causation_id"))
                .transpose()?,
            aggregate_key: parse_event_value(value.aggregate_key, "aggregate_key")?,
            idempotency_key: parse_event_value(value.idempotency_key, "idempotency_key")?,
            source: value.source.try_into()?,
            observed_at: parse_event_value(value.observed_at, "recorded_at")?,
            target_handler: value
                .target_handler
                .map(|value| parse_event_value(value, "target_handler"))
                .transpose()?,
            priority: value.priority.into(),
            deadline: value.deadline,
            payload: value.payload.into(),
        })
    }
}
