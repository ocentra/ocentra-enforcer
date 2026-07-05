use tokio::task::JoinHandle;

use crate::{
    DomainEvent, EventMetadata, EventingError, RequestCompletionReport, RequestEvent, RequestId,
    RequestOptions, RequestReport, StoredEventEnvelope,
};

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
    pub async fn publish<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent,
    {
        flow::publish_with_mode(self, event, metadata, DispatchMode::Sequential).await
    }

    pub async fn publish_and_wait<E>(
        &self,
        event: E,
        metadata: EventMetadata,
    ) -> Result<PublishReport, EventingError>
    where
        E: DomainEvent,
    {
        self.publish(event, metadata).await
    }

    pub fn publish_detached<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        dispatch_mode: DispatchMode,
    ) -> JoinHandle<Result<PublishReport, EventingError>>
    where
        E: DomainEvent,
    {
        let bus = self.clone();
        tokio::spawn(
            async move { flow::publish_with_mode(&bus, event, metadata, dispatch_mode).await },
        )
    }

    pub async fn publish_request<E>(
        &self,
        event: E,
        metadata: EventMetadata,
        options: RequestOptions,
    ) -> Result<RequestReport<E::Response>, EventingError>
    where
        E: RequestEvent,
    {
        request::publish_request(self, event, metadata, options).await
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
        flow::publish_with_mode(self, event, metadata, dispatch_mode).await
    }

    pub async fn journal(&self) -> Vec<StoredEventEnvelope> {
        self.stored_journal.read().await.clone()
    }

    pub async fn dead_letters(&self) -> Vec<DeadLetter> {
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
