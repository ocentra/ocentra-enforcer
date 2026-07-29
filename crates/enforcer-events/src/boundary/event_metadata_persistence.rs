//! Metadata and live-envelope wire values for event persistence.
//!
//! BOUNDARY-INVARIANT: all identifier strings are validated during conversion
//! into live event values.
//! BOUNDARY-TEST: envelope tests reject malformed metadata and contract skew.
//! ROUNDTRIP-TEST: `tests/unit/envelope.rs` round-trips metadata and live envelopes.
//! NEGATIVE-TEST: `tests/unit/envelope.rs` rejects malformed identifiers and contract skew.

use enforcer_domain::events_types::EventPriority;
use serde::{Deserialize, Serialize};

use crate::{
    boundary::{
        envelope_persistence::parse_event_value,
        event_contract_persistence::{EventContractDto, EventPriorityDto, EventSourceDto},
    },
    clock::EventClockInstant,
    envelope::{DomainEvent, EventContract, EventFrame, EventMetadata},
    error::EventingError,
};

/// JSON DTO for event metadata at the persistence edge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMetadataDto {
    pub event_id: String,
    pub correlation_id: String,
    // DEFAULT-JUSTIFICATION: older stored envelopes omitted optional causation metadata.
    #[serde(default)]
    pub causation_id: Option<String>,
    pub source: EventSourceDto,
    pub observed_at: String,
    pub target_handler: Option<String>,
    // DEFAULT-JUSTIFICATION: priority was added after the original envelope schema; normal preserves legacy behavior.
    #[serde(default)]
    pub priority: EventPriorityDto,
    // DEFAULT-JUSTIFICATION: legacy envelopes without a deadline remain unbounded.
    #[serde(default)]
    pub deadline: Option<EventClockInstant>,
}

impl From<&EventMetadata> for EventMetadataDto {
    fn from(value: &EventMetadata) -> Self {
        Self {
            event_id: value.event_id.as_str().to_owned(),
            correlation_id: value.correlation_id.as_str().to_owned(),
            causation_id: value.causation_id.as_ref().map(|id| id.as_str().to_owned()),
            source: EventSourceDto::from(&value.source),
            observed_at: value.observed_at.as_str().to_owned(),
            target_handler: value
                .target_handler
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            priority: value.priority.into(),
            deadline: value.deadline,
        }
    }
}

impl TryFrom<EventMetadataDto> for EventMetadata {
    type Error = EventingError;

    fn try_from(value: EventMetadataDto) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: parse_event_value(value.event_id, "event_id")?,
            correlation_id: parse_event_value(value.correlation_id, "correlation_id")?,
            causation_id: value
                .causation_id
                .map(|id| parse_event_value(id, "causation_id"))
                .transpose()?,
            source: value.source.try_into()?,
            observed_at: parse_event_value(value.observed_at, "recorded_at")?,
            target_handler: value
                .target_handler
                .map(|value| parse_event_value(value, "target_handler"))
                .transpose()?,
            priority: value.priority.into(),
            deadline: value.deadline,
        })
    }
}

/// JSON DTO for a live typed event envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelopeDto<P> {
    pub contract: EventContractDto,
    pub event_id: String,
    pub correlation_id: String,
    // DEFAULT-JUSTIFICATION: older wire envelopes omitted optional causation metadata.
    #[serde(default)]
    pub causation_id: Option<String>,
    pub aggregate_key: String,
    pub idempotency_key: String,
    pub source: EventSourceDto,
    pub observed_at: String,
    pub target_handler: Option<String>,
    // DEFAULT-JUSTIFICATION: priority was added after the original wire schema; normal preserves legacy behavior.
    #[serde(default)]
    pub priority: EventPriorityDto,
    // DEFAULT-JUSTIFICATION: legacy wire envelopes without a deadline remain unbounded.
    #[serde(default)]
    pub deadline: Option<EventClockInstant>,
    pub payload: P,
}

impl<E: Clone> From<&EventFrame<E>> for EventEnvelopeDto<E> {
    fn from(value: &EventFrame<E>) -> Self {
        Self {
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
            payload: value.payload.clone(),
        }
    }
}

impl<E: DomainEvent> TryFrom<EventEnvelopeDto<E>> for EventFrame<E> {
    type Error = EventingError;

    fn try_from(value: EventEnvelopeDto<E>) -> Result<Self, Self::Error> {
        let contract: EventContract = value.contract.try_into()?;
        let payload = value.payload;
        let expected = payload.contract()?;
        if expected != contract {
            return Err(EventingError::ContractMismatch {
                expected: expected.event_type,
                received: contract.event_type,
                expected_schema_version: expected.schema_version,
                received_schema_version: contract.schema_version,
            });
        }
        Ok(Self {
            contract,
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
            priority: EventPriority::from(value.priority),
            deadline: value.deadline,
            payload,
        })
    }
}
