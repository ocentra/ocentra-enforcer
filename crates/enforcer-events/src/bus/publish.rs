use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use enforcer_domain::events_types::RequestId;
use tokio::task::JoinHandle;

use crate::{
    envelope::{DomainEvent, EventMetadata},
    error::EventingError,
    request::{RequestCompletionReport, RequestEvent, RequestOptions, RequestReport},
};
use serde::Serialize;

use super::{reports::dead_letter::DeadLetter, DispatchMode, EventBus, PublishReport};

pub(super) mod flow;
mod request;

pub(crate) enum DispatchStoredError {
    BeforeDispatch(EventingError),
    AfterDispatch(EventingError),
}

impl DispatchStoredError {
    fn into_error(self) -> EventingError {
        match self {
            Self::BeforeDispatch(error) | Self::AfterDispatch(error) => error,
        }
    }
}

impl From<EventingError> for DispatchStoredError {
    fn from(error: EventingError) -> Self {
        Self::BeforeDispatch(error)
    }
}

impl EventBus {
    /// Executes the publish event-runtime operation.
    pub async fn publish<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent + Serialize,
    {
        flow::publish_with_mode(self, event, metadata, DispatchMode::Sequential).await
    }

    /// Executes the publish and wait event-runtime operation.
    pub async fn publish_and_wait<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent + Serialize,
    {
        self.publish(event, metadata).await
    }

    /// Executes the publish detached event-runtime operation.
    pub fn publish_detached<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        dispatch_mode: DispatchMode,
    ) -> JoinHandle<Result<PublishReport, EventingError>>
    where
        E: DomainEvent + Serialize,
    {
        // CLONE-JUSTIFICATION: the detached task must own a bus handle beyond the caller's borrow.
        let bus = self.clone();
        // SHUTDOWN-TEST: production_shutdown verifies returned publish tasks settle before drain shutdown completes.
        let task = tokio::spawn(async move {
            flow::publish_with_mode(&bus, event, metadata, dispatch_mode).await
        });
        task
    }

    /// Executes the publish request event-runtime operation.
    pub async fn publish_request<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        options: RequestOptions,
    ) -> Result<RequestReport<E::Response>, EventingError>
    where
        E: RequestEvent + Serialize,
    {
        request::publish_request(self, event, metadata, options).await
    }

    /// Executes the publish with mode event-runtime operation.
    pub async fn publish_with_mode<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        dispatch_mode: DispatchMode,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent + Serialize,
    {
        flow::publish_with_mode(self, event, metadata, dispatch_mode).await
    }

    /// Executes the journal event-runtime operation.
    pub async fn journal(&self) -> Vec<StoredEventEnvelope> {
        // CLONE-JUSTIFICATION: callers receive an owned point-in-time journal snapshot independent of the bus lock.
        self.stored_journal.read().await.clone()
    }

    /// Executes the dead letters event-runtime operation.
    pub async fn dead_letters(&self) -> Vec<DeadLetter> {
        // CLONE-JUSTIFICATION: callers receive an owned point-in-time dead-letter snapshot independent of the bus lock.
        self.dead_letters.read().await.clone()
    }

    pub(super) async fn complete_request<E>(
        &self,
        request_id: RequestId,
        response: E::Response,
    ) -> Result<RequestCompletionReport, EventingError>
    where
        E: RequestEvent,
    {
        self.requests.complete(request_id, response)
    }
}
