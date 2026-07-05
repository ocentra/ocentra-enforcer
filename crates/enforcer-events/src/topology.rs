use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    EventContract, EventContractRegistry, EventNamespace, EventType, SourceComponent, SubscriberId,
    TargetHandler,
};

const MARKDOWN_TITLE: &str = "# Event Topology Manifest";
const MARKDOWN_HEADER: &str =
    "| Event Type | Schema Version | Publishers | Subscribers | Families | Status | Rust Type |";
const MARKDOWN_SEPARATOR: &str = "| --- | --- | --- | --- | --- | --- | --- |";
const EMPTY_CELL: &str = "none";
const LIST_SEPARATOR: &str = ", ";
const SUBSCRIBER_TARGET_SEPARATOR: &str = " -> ";
const CELL_ESCAPE_TARGET: &str = "|";
const CELL_ESCAPE_REPLACEMENT: &str = "\\|";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTopologyPublisher {
    pub event_type: EventType,
    pub source_component: SourceComponent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTopologySubscriber {
    pub event_type: EventType,
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTopologyFamilyVariant {
    pub family: EventNamespace,
    pub event_type: EventType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologySubscriberTarget {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventTopologyStatus {
    Covered,
    NoPublisher,
    NoSubscriber,
    AcceptedOneSided,
}

impl EventTopologyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::NoPublisher => "no-publisher",
            Self::NoSubscriber => "no-subscriber",
            Self::AcceptedOneSided => "accepted-one-sided",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologyEntry {
    pub contract: EventContract,
    rust_type: EventTopologyRustType,
    pub publishers: Vec<SourceComponent>,
    pub subscribers: Vec<EventTopologySubscriberTarget>,
    pub families: Vec<EventNamespace>,
    pub status: EventTopologyStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologyManifest {
    entries: Vec<EventTopologyEntry>,
}

impl EventTopologyManifest {
    pub fn from_registry(
        registry: &EventContractRegistry,
        publishers: &[EventTopologyPublisher],
        subscribers: &[EventTopologySubscriber],
        family_variants: &[EventTopologyFamilyVariant],
        accepted_one_sided: &[EventType],
    ) -> Self {
        let accepted = accepted_one_sided.iter().cloned().collect::<BTreeSet<_>>();
        let entries = registry
            .descriptors()
            .map(|descriptor| {
                let event_type = descriptor.event_type();
                let publishers = collect_sorted_unique(
                    publishers
                        .iter()
                        .filter(|publisher| &publisher.event_type == event_type)
                        .map(|publisher| publisher.source_component.clone()),
                );
                let subscribers = collect_sorted_unique(
                    subscribers
                        .iter()
                        .filter(|subscriber| &subscriber.event_type == event_type)
                        .map(|subscriber| EventTopologySubscriberTarget {
                            subscriber_id: subscriber.subscriber_id.clone(),
                            target_handler: subscriber.target_handler.clone(),
                        }),
                );
                let families = collect_sorted_unique(
                    family_variants
                        .iter()
                        .filter(|variant| &variant.event_type == event_type)
                        .map(|variant| variant.family.clone()),
                );
                EventTopologyEntry {
                    contract: EventContract::new(event_type.clone(), descriptor.schema_version()),
                    rust_type: EventTopologyRustType::from_static(descriptor.rust_type()),
                    status: status_for(event_type, &publishers, &subscribers, &accepted),
                    publishers,
                    subscribers,
                    families,
                }
            })
            .collect::<Vec<_>>();
        Self { entries }
    }

    pub fn entries(&self) -> &[EventTopologyEntry] {
        &self.entries
    }

    pub fn unready_entries(&self) -> Vec<&EventTopologyEntry> {
        self.entries
            .iter()
            .filter(|entry| is_unready_status(entry.status))
            .collect()
    }

    pub fn render_markdown(&self) -> String {
        let mut markdown = String::from(MARKDOWN_TITLE);
        markdown.push_str("\n\n");
        markdown.push_str(MARKDOWN_HEADER);
        markdown.push('\n');
        markdown.push_str(MARKDOWN_SEPARATOR);
        markdown.push('\n');
        for entry in &self.entries {
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
        markdown
    }
}

impl EventTopologyEntry {
    pub fn rust_type(&self) -> &str {
        self.rust_type.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct EventTopologyRustType(String);

impl EventTopologyRustType {
    fn from_static(value: &'static str) -> Self {
        Self(String::from(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_unready_status(status: EventTopologyStatus) -> bool {
    matches!(
        status,
        EventTopologyStatus::NoPublisher | EventTopologyStatus::NoSubscriber
    )
}

fn collect_sorted_unique<T>(values: impl Iterator<Item = T>) -> Vec<T>
where
    T: Ord,
{
    let mut values = values.collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn status_for(
    event_type: &EventType,
    publishers: &[SourceComponent],
    subscribers: &[EventTopologySubscriberTarget],
    accepted: &BTreeSet<EventType>,
) -> EventTopologyStatus {
    if !publishers.is_empty() && !subscribers.is_empty() {
        return EventTopologyStatus::Covered;
    }
    if accepted.contains(event_type) {
        return EventTopologyStatus::AcceptedOneSided;
    }
    if publishers.is_empty() {
        return EventTopologyStatus::NoPublisher;
    }
    EventTopologyStatus::NoSubscriber
}

fn join_components(values: &[SourceComponent]) -> String {
    join_string_values(values.iter().map(SourceComponent::as_str))
}

fn join_subscribers(values: &[EventTopologySubscriberTarget]) -> String {
    if values.is_empty() {
        return String::from(EMPTY_CELL);
    }
    values
        .iter()
        .map(|subscriber| {
            let mut value = String::from(subscriber.subscriber_id.as_str());
            value.push_str(SUBSCRIBER_TARGET_SEPARATOR);
            value.push_str(subscriber.target_handler.as_str());
            value
        })
        .collect::<Vec<_>>()
        .join(LIST_SEPARATOR)
}

fn join_families(values: &[EventNamespace]) -> String {
    join_string_values(values.iter().map(EventNamespace::as_str))
}

fn join_string_values<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return String::from(EMPTY_CELL);
    }
    values.join(LIST_SEPARATOR)
}

fn escape_cell(value: &str) -> String {
    value.replace(CELL_ESCAPE_TARGET, CELL_ESCAPE_REPLACEMENT)
}
