use std::io::{self, Write};

use enforcer_events::{
    bus::reports::dead_letter::{DeadLetterEvent, DeadLetterReason},
    contract_registry::EventContractRegistry,
    ids::{
        CorrelationId, EventId, EventNamespace, EventType, SourceComponent, SubscriberId,
        TargetHandler,
    },
    topology::{
        EventTopologyFamilyVariant, EventTopologyManifest, EventTopologyPublisher,
        EventTopologySubscriber,
    },
};

const EXAMPLE_ORIGINAL_EVENT_ID: &str = "eventing-topology-original-1";
const EXAMPLE_ORIGINAL_EVENT_TYPE: &str = "eventing.topology.original";
const EXAMPLE_CORRELATION_ID: &str = "eventing-topology-correlation-1";
const EXAMPLE_PUBLISHER: &str = "eventing-topology-example-publisher";
const EXAMPLE_SUBSCRIBER: &str = "eventing-topology-example-subscriber";
const EXAMPLE_TARGET: &str = "eventing-topology-example-target";
const EXAMPLE_FAMILY: &str = "eventing.topology.example-family";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = DeadLetterEvent {
        original_event_id: EventId::parse(EXAMPLE_ORIGINAL_EVENT_ID)?,
        original_event_type: EventType::parse(EXAMPLE_ORIGINAL_EVENT_TYPE)?,
        original_correlation_id: CorrelationId::parse(EXAMPLE_CORRELATION_ID)?,
        reason: DeadLetterReason::NoSubscriber,
        subscriber_id: None,
        target_handler: None,
    };

    let mut registry = EventContractRegistry::new();
    let event_type = registry.register_event(&event)?.event_type().clone();
    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[EventTopologyPublisher {
            event_type: event_type.clone(),
            source_component: SourceComponent::parse(EXAMPLE_PUBLISHER)?,
        }],
        &[EventTopologySubscriber {
            event_type: event_type.clone(),
            subscriber_id: SubscriberId::parse(EXAMPLE_SUBSCRIBER)?,
            target_handler: TargetHandler::parse(EXAMPLE_TARGET)?,
        }],
        &[EventTopologyFamilyVariant {
            family: EventNamespace::parse(EXAMPLE_FAMILY)?,
            event_type,
        }],
        &[],
    );
    io::stdout()
        .lock()
        .write_all(manifest.render_markdown().as_bytes())?;
    Ok(())
}
