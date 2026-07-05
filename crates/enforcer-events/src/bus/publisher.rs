use crate::{
    DomainEvent, EventEnvelope, EventMetadata, EventingError, RequestCompletionReport,
    RequestEvent, RequestId,
};

use super::{DispatchMode, EventBus, PublishReport};

#[derive(Clone)]
pub struct EventPublisher {
    bus: EventBus,
}

impl EventPublisher {
    pub(super) fn new(bus: EventBus) -> Self {
        Self { bus }
    }

    pub async fn publish<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent,
    {
        self.bus.publish(event, metadata).await
    }

    pub async fn publish_with_mode<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        dispatch_mode: DispatchMode,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent,
    {
        self.bus
            .publish_with_mode(event, metadata, dispatch_mode)
            .await
    }

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

#[derive(Clone, Debug)]
pub struct EventContext<E>
where
    E: DomainEvent,
{
    envelope: EventEnvelope<E>,
    publisher: EventPublisher,
}

impl<E> EventContext<E>
where
    E: DomainEvent,
{
    pub(super) fn new(envelope: EventEnvelope<E>, publisher: EventPublisher) -> Self {
        Self {
            envelope,
            publisher,
        }
    }

    pub fn envelope(&self) -> &EventEnvelope<E> {
        &self.envelope
    }

    pub fn payload(&self) -> &E {
        &self.envelope.payload
    }

    pub fn publisher(&self) -> &EventPublisher {
        &self.publisher
    }
}

impl<E> EventContext<E>
where
    E: RequestEvent,
{
    pub async fn complete_request(
        &self,
        response: E::Response,
    ) -> Result<RequestCompletionReport, EventingError> {
        self.publisher
            .complete_request::<E>(self.payload().request_id()?, response)
            .await
    }
}
