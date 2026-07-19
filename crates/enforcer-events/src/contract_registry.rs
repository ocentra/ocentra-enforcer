use std::{
    any::type_name,
    collections::{btree_map::Entry, BTreeMap},
};

use crate::{
    envelope::{DomainEvent, EventContract},
    error::EventingError,
};
use enforcer_domain::events_types::{EventType, RenderedMarkdown, RustTypeName, SchemaVersion};

const DOC_TITLE: &str = "# Event Contract Registry";
const DOC_EMPTY: &str = "_No event contracts registered._";
const DOC_HEADER: &str = "| Event Type | Schema Version | Rust Type |";
const DOC_SEPARATOR: &str = "| --- | --- | --- |";

/// Event-runtime data for event contract descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContractDescriptor {
    contract: EventContract,
    rust_type: RustTypeName,
}

impl EventContractDescriptor {
    /// Executes the from event event-runtime operation.
    pub fn from_event<E>(event: &E) -> Result<Self, EventingError>
    where
        E: DomainEvent,
    {
        // ALLOC-JUSTIFICATION: the descriptor retains the concrete Rust type name after registration returns.
        Ok(Self {
            contract: event.contract()?,
            rust_type: RustTypeName::try_new(type_name::<E>().to_owned())?,
        })
    }

    /// Executes the event type event-runtime operation.
    pub fn event_type(&self) -> &EventType {
        &self.contract.event_type
    }

    /// Executes the schema version event-runtime operation.
    pub fn schema_version(&self) -> SchemaVersion {
        self.contract.schema_version
    }

    /// Executes the rust type event-runtime operation.
    pub fn rust_type(&self) -> &RustTypeName {
        &self.rust_type
    }
}

/// Event-runtime data for event contract registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventContractRegistry {
    descriptors: BTreeMap<EventType, EventContractDescriptor>,
}

impl EventContractRegistry {
    /// Executes the new event-runtime operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes the register event event-runtime operation.
    pub fn register_event<E>(
        &mut self,
        event: &E,
    ) -> Result<&EventContractDescriptor, EventingError>
    where
        E: DomainEvent,
    {
        self.register(EventContractDescriptor::from_event(event)?)
    }

    /// Executes the register event-runtime operation.
    pub fn register(
        &mut self,
        descriptor: EventContractDescriptor,
    ) -> Result<&EventContractDescriptor, EventingError> {
        // CLONE-JUSTIFICATION: the map owns its key independently of the descriptor value.
        let event_type = descriptor.event_type().clone();
        match self.descriptors.entry(event_type) {
            Entry::Occupied(occupied) => Err(EventingError::DuplicateEventContract {
                // CLONE-JUSTIFICATION: the error owns the rejected key while the registry retains the existing entry.
                event_type: occupied.key().clone(),
            }),
            Entry::Vacant(vacant) => Ok(&*vacant.insert(descriptor)),
        }
    }

    /// Executes the descriptors event-runtime operation.
    pub fn descriptors(&self) -> impl Iterator<Item = &EventContractDescriptor> {
        self.descriptors.values()
    }

    /// Executes the render markdown event-runtime operation.
    pub fn render_markdown(&self) -> Result<EventContractRegistryDocumentation, EventingError> {
        let mut markdown = String::from(DOC_TITLE);
        markdown.push_str("\n\n");
        if self.descriptors.is_empty() {
            markdown.push_str(DOC_EMPTY);
            markdown.push('\n');
            return Ok(EventContractRegistryDocumentation {
                markdown: RenderedMarkdown::try_new(markdown)?,
            });
        }
        markdown.push_str(DOC_HEADER);
        markdown.push('\n');
        markdown.push_str(DOC_SEPARATOR);
        markdown.push('\n');
        for descriptor in self.descriptors() {
            markdown.push_str("| ");
            markdown.push_str(&descriptor.event_type().as_str().replace('|', "\\|"));
            markdown.push_str(" | ");
            // ALLOC-JUSTIFICATION: rendered documentation owns the decimal schema version in its output buffer.
            markdown.push_str(&descriptor.schema_version().as_nonzero().get().to_string());
            markdown.push_str(" | ");
            markdown.push_str(&descriptor.rust_type().as_str().replace('|', "\\|"));
            markdown.push_str(" |\n");
        }
        Ok(EventContractRegistryDocumentation {
            markdown: RenderedMarkdown::try_new(markdown)?,
        })
    }
}

/// Event-runtime data for event contract registry documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContractRegistryDocumentation {
    markdown: RenderedMarkdown,
}

impl EventContractRegistryDocumentation {
    /// Executes the markdown event-runtime operation.
    pub fn markdown(&self) -> &RenderedMarkdown {
        &self.markdown
    }

    /// Executes the into markdown event-runtime operation.
    pub fn into_markdown(self) -> RenderedMarkdown {
        self.markdown
    }
}
// INVALID-INPUT-TEST: contract registry tests reject malformed event types and
// duplicate registrations before documentation is rendered.
