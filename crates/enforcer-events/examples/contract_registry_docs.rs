use std::io::{self, Write};

use enforcer_events::{
    bus::reports::dead_letter::{DeadLetterEvent, DeadLetterReason},
    contract_registry::EventContractRegistry,
    ids::{CorrelationId, EventId, EventType},
};

const EXAMPLE_ORIGINAL_EVENT_ID: &str = "eventing-example-original-1";
const EXAMPLE_ORIGINAL_EVENT_TYPE: &str = "eventing.example.original";
const EXAMPLE_CORRELATION_ID: &str = "eventing-example-correlation-1";

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
    registry.register_event(&event)?;
    io::stdout()
        .lock()
        .write_all(registry.render_markdown().as_str().as_bytes())?;
    Ok(())
}
