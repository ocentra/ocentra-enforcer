//! Outbound rendering boundary for topology reports.

use serde::Serialize;

use crate::topology::{EventTopologyEntry, EventTopologyStatus, EventTopologySubscriberTarget};
use crate::{EventNamespace, SourceComponent};

const EMPTY_CELL: &str = "none";

/// Typed outbound artifact for the generated topology markdown report.
/// BRAND-INVARIANT: the contained text is produced by the canonical topology renderer or is a non-empty supplied report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyMarkdown(String);

impl EventTopologyMarkdown {
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
            markdown.push_str(&entry.contract.schema_version.value().to_string());
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_components(&entry.publishers)));
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_subscribers(&entry.subscribers)));
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(&join_families(&entry.families)));
            markdown.push_str(" | ");
            markdown.push_str(entry.status.as_str());
            markdown.push_str(" | ");
            markdown.push_str(&escape_cell(entry.rust_type.as_str()));
            markdown.push_str(" |\n");
        }
        Self(markdown)
    }

    pub fn lines(&self) -> core::str::Lines<'_> { self.0.lines() }
}

/// SERIALIZATION-DOC: rust type text is emitted as the manifest's `rustType` field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct EventTopologyRustType(String);

impl EventTopologyRustType {
    pub(crate) fn from_static(value: &'static str) -> Self { Self(String::from(value)) }
    pub(crate) fn as_str(&self) -> &str { &self.0 }
}

impl EventTopologyStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self { Self::Covered => "covered", Self::NoPublisher => "no-publisher", Self::NoSubscriber => "no-subscriber", Self::AcceptedOneSided => "accepted-one-sided" }
    }
}

fn join_components(values: &[SourceComponent]) -> String { join(values.iter().map(SourceComponent::as_str)) }
fn join_families(values: &[EventNamespace]) -> String { join(values.iter().map(EventNamespace::as_str)) }
fn join_subscribers(values: &[EventTopologySubscriberTarget]) -> String {
    if values.is_empty() { return String::from(EMPTY_CELL); }
    values.iter().map(|value| format!("{} -> {}", value.subscriber_id.as_str(), value.target_handler.as_str())).collect::<Vec<_>>().join(", ")
}
fn join<'a>(values: impl Iterator<Item = &'a str>) -> String { let values=values.collect::<Vec<_>>(); if values.is_empty() { String::from(EMPTY_CELL) } else { values.join(", ") } }
fn escape_cell(value: &str) -> String { value.replace('|', "\\|") }
