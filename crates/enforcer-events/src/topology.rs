use std::collections::BTreeSet;

use enforcer_domain::events_types::{
    EventNamespace, EventTopologyStatus, EventType, RustTypeName, SourceComponent, SubscriberId,
    TargetHandler,
};

use crate::boundary::topology_presentation::{
    EventTopologyManifestResponse, EventTopologyMarkdown,
};

use crate::{contract_registry::EventContractRegistry, envelope::EventContract};

/// Typed publisher input used to build a canonical topology manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyPublisher {
    pub event_type: EventType,
    pub source_component: SourceComponent,
}

/// Typed subscriber input used to build a canonical topology manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologySubscriber {
    pub event_type: EventType,
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

/// Typed event-family input used to build a canonical topology manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyFamilyVariant {
    pub family: EventNamespace,
    pub event_type: EventType,
}

/// Canonical subscriber target retained by a topology manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventTopologySubscriberTarget {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
}

/// Canonical generated topology entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyEntry {
    pub contract: EventContract,
    pub(crate) rust_type: RustTypeName,
    pub publishers: Vec<SourceComponent>,
    pub subscribers: Vec<EventTopologySubscriberTarget>,
    pub families: Vec<EventNamespace>,
    pub status: EventTopologyStatus,
}

/// Canonical ordered topology manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTopologyManifest {
    pub(crate) entries: Vec<EventTopologyEntry>,
}

impl EventTopologyManifest {
    /// Executes the from registry event-runtime operation.
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
                    rust_type: descriptor.rust_type().clone(),
                    status: status_for(event_type, &publishers, &subscribers, &accepted),
                    publishers,
                    subscribers,
                    families,
                }
            })
            .collect::<Vec<_>>();
        Self { entries }
    }

    /// Executes the entries event-runtime operation.
    pub fn entries(&self) -> &[EventTopologyEntry] {
        &self.entries
    }

    /// Executes the unready entries event-runtime operation.
    pub fn unready_entries(&self) -> Vec<&EventTopologyEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.status,
                    EventTopologyStatus::NoPublisher | EventTopologyStatus::NoSubscriber
                )
            })
            .collect()
    }

    /// Executes the render markdown event-runtime operation.
    pub fn render_markdown(&self) -> EventTopologyMarkdown {
        EventTopologyMarkdown::render(&self.entries)
    }

    /// Convert canonical topology values to the explicit presentation contract
    /// used by JSON/reporting callers.
    pub fn presentation(&self) -> EventTopologyManifestResponse {
        EventTopologyManifestResponse::from(self)
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
