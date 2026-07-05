use std::{
    any::type_name,
    collections::{btree_map::Entry, BTreeMap},
};

use crate::{DomainEvent, EventContract, EventType, EventingError, SchemaVersion};

const DOC_TITLE: &str = "# Event Contract Registry";
const DOC_EMPTY: &str = "_No event contracts registered._";
const DOC_HEADER: &str = "| Event Type | Schema Version | Rust Type |";
const DOC_SEPARATOR: &str = "| --- | --- | --- |";
const CELL_ESCAPE_TARGET: &str = "|";
const CELL_ESCAPE_REPLACEMENT: &str = "\\|";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContractDescriptor {
    contract: EventContract,
    rust_type: &'static str,
}

impl EventContractDescriptor {
    pub fn from_event<E>(event: &E) -> Result<Self, EventingError>
    where
        E: DomainEvent,
    {
        Ok(Self {
            contract: event.contract()?,
            rust_type: type_name::<E>(),
        })
    }

    pub fn event_type(&self) -> &EventType {
        &self.contract.event_type
    }

    pub fn schema_version(&self) -> SchemaVersion {
        self.contract.schema_version
    }

    pub fn rust_type(&self) -> &'static str {
        self.rust_type
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventContractRegistry {
    descriptors: BTreeMap<EventType, EventContractDescriptor>,
}

impl EventContractRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_event<E>(
        &mut self,
        event: &E,
    ) -> Result<&EventContractDescriptor, EventingError>
    where
        E: DomainEvent,
    {
        self.register(EventContractDescriptor::from_event(event)?)
    }

    pub fn register(
        &mut self,
        descriptor: EventContractDescriptor,
    ) -> Result<&EventContractDescriptor, EventingError> {
        let event_type = descriptor.event_type().clone();
        match self.descriptors.entry(event_type.clone()) {
            Entry::Occupied(_) => Err(EventingError::DuplicateEventContract { event_type }),
            Entry::Vacant(vacant) => Ok(&*vacant.insert(descriptor)),
        }
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &EventContractDescriptor> {
        self.descriptors.values()
    }

    pub fn render_markdown(&self) -> EventContractRegistryDocumentation {
        let mut markdown = String::from(DOC_TITLE);
        markdown.push_str("\n\n");
        if self.descriptors.is_empty() {
            markdown.push_str(DOC_EMPTY);
            markdown.push('\n');
            return EventContractRegistryDocumentation { markdown };
        }
        markdown.push_str(DOC_HEADER);
        markdown.push('\n');
        markdown.push_str(DOC_SEPARATOR);
        markdown.push('\n');
        for descriptor in self.descriptors() {
            markdown.push_str("| ");
            markdown.push_str(&escape_markdown_cell(descriptor.event_type().as_str()));
            markdown.push_str(" | ");
            markdown.push_str(&descriptor.schema_version().value().to_string());
            markdown.push_str(" | ");
            markdown.push_str(&escape_markdown_cell(descriptor.rust_type()));
            markdown.push_str(" |\n");
        }
        EventContractRegistryDocumentation { markdown }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContractRegistryDocumentation {
    markdown: String,
}

impl EventContractRegistryDocumentation {
    pub fn as_str(&self) -> &str {
        &self.markdown
    }

    pub fn into_string(self) -> String {
        self.markdown
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace(CELL_ESCAPE_TARGET, CELL_ESCAPE_REPLACEMENT)
}
