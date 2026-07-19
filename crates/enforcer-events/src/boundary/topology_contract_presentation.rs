//! Contract and subscriber presentation values for topology reports.
//!
//! BOUNDARY-INVARIANT: these outbound-only values are created from validated
//! topology-domain records and never flow into routing decisions.
//! BOUNDARY-TEST: topology manifest contract tests verify canonical output.
//! ROUNDTRIP-TEST: tests/contract/topology_manifest.rs serializes these
//! presentation contracts and verifies their canonical JSON keys.
//! NEGATIVE-TEST: `tests/contract/topology_manifest.rs` rejects invalid topology input.

use std::num::NonZeroU16;

use enforcer_domain::events_types::SchemaVersion;
use serde::Serialize;

use crate::{
    envelope::EventContract, error::EventingError, topology::EventTopologySubscriberTarget,
};

/// Explicit JSON presentation of a topology contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologyContractResponse {
    pub event_type: String,
    pub schema_version: u16,
}

/// Explicit JSON presentation of a topology subscriber target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologySubscriberTargetResponse {
    pub subscriber_id: String,
    pub target_handler: String,
}

impl TryFrom<EventTopologyContractResponse> for EventContract {
    type Error = EventingError;

    fn try_from(value: EventTopologyContractResponse) -> Result<Self, Self::Error> {
        Ok(Self::new(
            value.event_type.try_into()?,
            SchemaVersion::try_new(
                NonZeroU16::new(value.schema_version).ok_or(EventingError::InvalidVersion)?,
            ),
        ))
    }
}

impl From<&EventTopologySubscriberTarget> for EventTopologySubscriberTargetResponse {
    fn from(value: &EventTopologySubscriberTarget) -> Self {
        Self {
            subscriber_id: value.subscriber_id.as_str().to_owned(),
            target_handler: value.target_handler.as_str().to_owned(),
        }
    }
}

impl TryFrom<EventTopologySubscriberTargetResponse> for EventTopologySubscriberTarget {
    type Error = EventingError;

    fn try_from(value: EventTopologySubscriberTargetResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            subscriber_id: value.subscriber_id.try_into()?,
            target_handler: value.target_handler.try_into()?,
        })
    }
}
