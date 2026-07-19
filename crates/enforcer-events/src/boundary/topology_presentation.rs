//! Outbound rendering and JSON presentation boundary for topology reports.
//!
//! BOUNDARY-INVARIANT: this module is the sole conversion point from typed
//! topology domain values to markdown and JSON response data. These responses
//! never flow back into topology decisions.
//! BOUNDARY-TEST: invalid presentation input is rejected by the topology
//! presentation contract tests.
//! BOUNDARY-OWNER: enforcer-events.
//! boundaryOwnerNote: enforcer-events owns the expanded topology presentation
//! surface and maps it directly from validated topology-domain values.
//! ROUNDTRIP-TEST: `tests/contract/topology_manifest.rs` verifies canonical
//! manifest serialization and rejects invalid topology input.

use enforcer_domain::events_types::{
    EventErrorField, EventErrorReason, EventNamespace, EventTopologyStatus, SourceComponent,
};
use serde::Serialize;

use crate::topology::{EventTopologyEntry, EventTopologyManifest, EventTopologySubscriberTarget};

use super::topology_contract_presentation::{
    EventTopologyContractResponse, EventTopologySubscriberTargetResponse,
};

const EMPTY_CELL: &str = "none";

/// Typed outbound artifact for the generated topology markdown report.
/// BRAND-INVARIANT: the contained text is produced by the canonical topology renderer or is a non-empty supplied report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyMarkdown(String);

impl EventTopologyMarkdown {
    /// Executes the try new event-runtime operation.
    pub fn try_new(value: String) -> Result<Self, String> {
        if value.trim().is_empty() {
            return Err(String::from("topology markdown must not be empty"));
        }
        Ok(Self(value))
    }

    pub(crate) fn render(entries: &[EventTopologyEntry]) -> Self {
        let mut markdown = String::from("# Event Topology Manifest\n\n| Event Type | Schema Version | Publishers | Subscribers | Families | Status | Rust Type |\n| --- | --- | --- | --- | --- | --- | --- |\n");
        for entry in entries {
            markdown.push_str("| ");
            markdown.push_str(&escape_cell(entry.contract.event_type.as_str()));
            markdown.push_str(" | ");
            markdown.push_str(&entry.contract.schema_version.as_nonzero().get().to_string());
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_components(&entry.publishers)));
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_subscribers(&entry.subscribers)));
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_families(&entry.families)));
            markdown.push_str(" | ");
            markdown.push_str(topology_status_text(entry.status));
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(entry.rust_type.as_str()));
            markdown.push_str(" |\n");
        }
        Self(markdown)
    }

    /// Executes the lines event-runtime operation.
    pub fn lines(&self) -> core::str::Lines<'_> {
        self.0.lines()
    }

    /// Executes the as str event-runtime operation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explicit JSON presentation of one topology entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologyEntryResponse {
    pub contract: EventTopologyContractResponse,
    pub rust_type: String,
    pub publishers: Vec<String>,
    pub subscribers: Vec<EventTopologySubscriberTargetResponse>,
    pub families: Vec<String>,
    pub status: String,
}

/// Explicit JSON presentation of the complete topology manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologyManifestResponse {
    pub entries: Vec<EventTopologyEntryResponse>,
}

impl From<&EventTopologyEntry> for EventTopologyEntryResponse {
    fn from(value: &EventTopologyEntry) -> Self {
        Self {
            contract: EventTopologyContractResponse {
                event_type: value.contract.event_type.as_str().to_owned(),
                schema_version: value.contract.schema_version.as_nonzero().get(),
            },
            rust_type: value.rust_type.as_str().to_owned(),
            publishers: value
                .publishers
                .iter()
                .map(|publisher| publisher.as_str().to_owned())
                .collect(),
            subscribers: value.subscribers.iter().map(Into::into).collect(),
            families: value
                .families
                .iter()
                .map(|family| family.as_str().to_owned())
                .collect(),
            status: topology_status_text(value.status).to_owned(),
        }
    }
}

impl TryFrom<EventTopologyEntryResponse> for EventTopologyEntry {
    type Error = crate::error::EventingError;

    fn try_from(value: EventTopologyEntryResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            contract: value.contract.try_into()?,
            rust_type: value.rust_type.try_into()?,
            publishers: value
                .publishers
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<SourceComponent>, _>>()?,
            subscribers: value
                .subscribers
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<EventTopologySubscriberTarget>, _>>()?,
            families: value
                .families
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<EventNamespace>, _>>()?,
            status: topology_status_from_token(value.status)?,
        })
    }
}

impl From<&EventTopologyManifest> for EventTopologyManifestResponse {
    fn from(value: &EventTopologyManifest) -> Self {
        Self {
            entries: value.entries().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<EventTopologyManifestResponse> for EventTopologyManifest {
    type Error = crate::error::EventingError;

    fn try_from(value: EventTopologyManifestResponse) -> Result<Self, Self::Error> {
        value
            .entries
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<EventTopologyEntry>, _>>()
            .map(|entries| EventTopologyManifest { entries })
    }
}

fn topology_status_text(value: EventTopologyStatus) -> &'static str {
    match value {
        EventTopologyStatus::Covered => "covered",
        EventTopologyStatus::NoPublisher => "no-publisher",
        EventTopologyStatus::NoSubscriber => "no-subscriber",
        EventTopologyStatus::AcceptedOneSided => "accepted-one-sided",
    }
}

fn topology_status_from_token(
    value: String,
) -> Result<EventTopologyStatus, crate::error::EventingError> {
    match value.as_str() {
        "covered" => Ok(EventTopologyStatus::Covered),
        "no-publisher" => Ok(EventTopologyStatus::NoPublisher),
        "no-subscriber" => Ok(EventTopologyStatus::NoSubscriber),
        "accepted-one-sided" => Ok(EventTopologyStatus::AcceptedOneSided),
        _ => Err(crate::error::EventingError::invalid_value(
            EventErrorField::from_diagnostic("event_topology_status"),
            EventErrorReason::from_diagnostic(value),
        )),
    }
}

fn join_components(values: &[SourceComponent]) -> String {
    join(values.iter().map(SourceComponent::as_str))
}
fn join_families(values: &[EventNamespace]) -> String {
    join(values.iter().map(EventNamespace::as_str))
}
fn join_subscribers(values: &[EventTopologySubscriberTarget]) -> String {
    if values.is_empty() {
        return String::from(EMPTY_CELL);
    }
    values
        .iter()
        .map(|value| {
            format!(
                "{} -> {}",
                value.subscriber_id.as_str(),
                value.target_handler.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
fn join<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        String::from(EMPTY_CELL)
    } else {
        values.join(", ")
    }
}
fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|")
}
