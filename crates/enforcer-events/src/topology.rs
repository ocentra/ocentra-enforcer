use std::collections::BTreeSet;

use serde::Serialize;

use crate::topology_presentation_boundary::{EventTopologyMarkdown, EventTopologyRustType};

use crate::{
    EventContract, EventContractRegistry, EventNamespace, EventType, SourceComponent, SubscriberId,
    TargetHandler,
};


/// SERIALIZATION-DOC: outbound topology input is emitted in the canonical eventing manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologyPublisher {
    pub event_type: EventType,
    pub source_component: SourceComponent,
}

/// SERIALIZATION-DOC: outbound subscriber wiring is emitted in the canonical eventing manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologySubscriber {
    pub event_type: EventType,
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

/// SERIALIZATION-DOC: outbound event-family variants are emitted in the canonical eventing manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologyFamilyVariant {
    pub family: EventNamespace,
    pub event_type: EventType,
}

/// SERIALIZATION-DOC: outbound subscriber targets use stable camelCase field names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologySubscriberTarget {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

/// SERIALIZATION-DOC: outbound topology status is a stable kebab-case wire value.
/// SERDE-TAG-JUSTIFICATION: this fieldless enum is represented as its explicit status string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum EventTopologyStatus {
    Covered,
    NoPublisher,
    NoSubscriber,
    AcceptedOneSided,
}

/// SERIALIZATION-DOC: generated manifest entries are the published topology report contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTopologyEntry {
    pub contract: EventContract,
    pub(crate) rust_type: EventTopologyRustType,
    pub publishers: Vec<SourceComponent>,
    pub subscribers: Vec<EventTopologySubscriberTarget>,
    pub families: Vec<EventNamespace>,
    pub status: EventTopologyStatus,
}

/// SERIALIZATION-DOC: generated manifest wraps the ordered published topology entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventTopologyManifest {
    pub(crate) entries: Vec<EventTopologyEntry>,
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
                        // CLONE-JUSTIFICATION: the manifest owns a stable snapshot independent of caller slices.
                        .map(|publisher| publisher.source_component.clone()),
                );
                let subscribers = collect_sorted_unique(
                    subscribers
                        .iter()
                        .filter(|subscriber| &subscriber.event_type == event_type)
                        // CLONE-JUSTIFICATION: the manifest owns a stable subscriber-target snapshot.
                        .map(|subscriber| EventTopologySubscriberTarget {
                            subscriber_id: subscriber.subscriber_id.clone(),
                            target_handler: subscriber.target_handler.clone(),
                        }),
                );
                let families = collect_sorted_unique(
                    family_variants
                        .iter()
                        .filter(|variant| &variant.event_type == event_type)
                        // CLONE-JUSTIFICATION: the manifest owns family values after the input slice is released.
                        .map(|variant| variant.family.clone()),
                );
                EventTopologyEntry {
                    // CLONE-JUSTIFICATION: descriptors remain owned by the registry while the manifest owns its contract.
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
            .filter(|entry| matches!(entry.status, EventTopologyStatus::NoPublisher | EventTopologyStatus::NoSubscriber))
            .collect()
    }

    pub fn render_markdown(&self) -> EventTopologyMarkdown {
        EventTopologyMarkdown::render(&self.entries)
    }
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
