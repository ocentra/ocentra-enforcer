use crate::{
    envelope::{DomainEvent, EventFrame, EventMetadata},
    error::EventingError,
    request::{RequestCompletionReport, RequestEvent},
};
use enforcer_domain::events_types::RequestId;

use super::{DispatchMode, EventBus, PublishReport};

/// Event-runtime data for event publisher.
#[derive(Clone)]
pub struct EventPublisher {
    bus: EventBus,
}

impl EventPublisher {
    pub(super) fn new(bus: EventBus) -> Self {
        Self { bus }
    }

    /// Executes the publish event-runtime operation.
    pub async fn publish<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent + serde::Serialize,
    {
        self.bus.publish(event, metadata).await
    }

    /// Executes the publish with mode event-runtime operation.
    pub async fn publish_with_mode<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        dispatch_mode: DispatchMode,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent + serde::Serialize,
    {
        self.bus
            .publish_with_mode(event, metadata, dispatch_mode)
            .await
    }

    /// Executes the complete request event-runtime operation.
    pub async fn complete_request<E>(
        &self,
        request_id: RequestId,
        response: E::Response,
    ) -> Result<RequestCompletionReport, EventingError>
    where
        E: RequestEvent,
    {
        self.bus.complete_request::<E>(request_id, response).await
    }
}

impl std::fmt::Debug for EventPublisher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("EventPublisher")
    }
}

/// Event-runtime data for event context.
#[derive(Clone, Debug)]
pub struct EventContext<E>
where
    E: DomainEvent,
{
    envelope: EventFrame<E>,
    publisher: EventPublisher,
}

impl<E> EventContext<E>
where
    E: DomainEvent,
{
    pub(super) fn new(envelope: EventFrame<E>, publisher: EventPublisher) -> Self {
        Self {
            envelope,
            publisher,
        }
    }

    /// Executes the envelope event-runtime operation.
    pub fn envelope(&self) -> &EventFrame<E> {
        &self.envelope
    }

    /// Executes the payload event-runtime operation.
    pub fn payload(&self) -> &E {
        &self.envelope.payload
    }

    /// Executes the publisher event-runtime operation.
    pub fn publisher(&self) -> &EventPublisher {
        &self.publisher
    }
}

impl<E> EventContext<E>
where
    E: RequestEvent,
{
    /// Executes the complete request event-runtime operation.
    pub async fn complete_request(
        &self,
        response: E::Response,
    ) -> Result<RequestCompletionReport, EventingError> {
        self.publisher
            .complete_request::<E>(self.payload().request_id()?, response)
            .await
    }
}
