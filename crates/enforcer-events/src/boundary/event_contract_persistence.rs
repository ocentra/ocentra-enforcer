//! Contract, source, and priority wire values for event envelopes.
//!
//! BOUNDARY-INVARIANT: contract metadata passes through a decode conversion
//! into validated domain values before it reaches event dispatch logic.
//! BOUNDARY-TEST: envelope contract tests exercise valid and invalid values.
//! ROUNDTRIP-TEST: `tests/unit/envelope.rs` round-trips contract and source DTOs.
//! NEGATIVE-TEST: `tests/unit/envelope.rs` rejects zero versions and malformed identifiers.

use std::num::NonZeroU16;

use enforcer_domain::events_types::{EventPriority, SchemaVersion};
use serde::{Deserialize, Serialize};

use crate::{
    boundary::envelope_persistence::parse_event_value,
    envelope::{EventContract, EventSource},
    error::EventingError,
};

/// JSON DTO for a typed event contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventContractDto {
    pub event_type: String,
    pub schema_version: u16,
}

impl From<&EventContract> for EventContractDto {
    fn from(value: &EventContract) -> Self {
        Self {
            event_type: value.event_type.as_str().to_owned(),
            schema_version: crate::boundary::event_values::schema_version_value(
                value.schema_version,
            ),
        }
    }
}

impl TryFrom<EventContractDto> for EventContract {
    type Error = EventingError;

    fn try_from(value: EventContractDto) -> Result<Self, Self::Error> {
        Ok(Self::new(
            parse_event_value(value.event_type, "event_type")?,
            SchemaVersion::try_new(
                NonZeroU16::new(value.schema_version).ok_or(EventingError::InvalidVersion)?,
            ),
        ))
    }
}

/// JSON DTO for typed event source metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSourceDto {
    pub custody: String,
    pub role: String,
    pub service: String,
    pub component: String,
    pub instance_id: String,
}

impl From<&EventSource> for EventSourceDto {
    fn from(value: &EventSource) -> Self {
        Self {
            custody: value.custody.as_str().to_owned(),
            role: value.role.as_str().to_owned(),
            service: value.service.as_str().to_owned(),
            component: value.component.as_str().to_owned(),
            instance_id: value.instance_id.as_str().to_owned(),
        }
    }
}

impl TryFrom<EventSourceDto> for EventSource {
    type Error = EventingError;

    fn try_from(value: EventSourceDto) -> Result<Self, Self::Error> {
        Ok(Self::new(
            parse_event_value(value.custody, "event_custody")?,
            parse_event_value(value.role, "runtime_role")?,
            parse_event_value(value.service, "source_service")?,
            parse_event_value(value.component, "source_component")?,
            parse_event_value(value.instance_id, "runtime_instance_id")?,
        ))
    }
}

/// JSON priority token at the persistence edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[doc = "SERDE-TAG-JUSTIFICATION: scalar JSON string token at the envelope boundary."]
pub enum EventPriorityDto {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl From<EventPriority> for EventPriorityDto {
    fn from(value: EventPriority) -> Self {
        match value {
            EventPriority::Low => Self::Low,
            EventPriority::Normal => Self::Normal,
            EventPriority::High => Self::High,
            EventPriority::Critical => Self::Critical,
        }
    }
}

impl From<EventPriorityDto> for EventPriority {
    fn from(value: EventPriorityDto) -> Self {
        match value {
            EventPriorityDto::Low => Self::Low,
            EventPriorityDto::Normal => Self::Normal,
            EventPriorityDto::High => Self::High,
            EventPriorityDto::Critical => Self::Critical,
        }
    }
}
