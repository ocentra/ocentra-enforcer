use std::{future::Future, mem};

use crate::{
    DomainEvent, EventBus, EventContext, EventSubscriber, EventingError, SubscriptionHandle,
    SubscriptionReport, UnsubscribeReport,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RegistrarStatus {
    #[default]
    Active,
    Disposed,
}

#[derive(Default)]
pub struct EventRegistrar {
    handles: Vec<SubscriptionHandle>,
    status: RegistrarStatus,
}

impl EventRegistrar {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn subscribe<E, F, Fut>(
        &mut self,
        bus: &EventBus,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionReport, EventingError>
    where
        E: DomainEvent,
        F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
    {
        // CANCELLATION-TEST: registrar_dispose_cancellation_removes_all_owned_subscriptions
        if self.status == RegistrarStatus::Disposed {
            return Err(EventingError::RegistrarDisposed);
        }
        let handle = bus
            .subscribe_with_handle::<E, _, _>(subscriber, handler)
            .await?;
        let report = handle.report();
        self.handles.push(handle);
        Ok(report)
    }

    pub fn dispose(&mut self) -> RegistrarDisposeReport {
        let handles = mem::take(&mut self.handles);
        let reports = handles
            .into_iter()
            .map(|handle| handle.unsubscribe())
            .collect::<Vec<_>>();
        self.status = RegistrarStatus::Disposed;
        RegistrarDisposeReport { reports }
    }

    pub fn status(&self) -> RegistrarStatus {
        self.status
    }
}

impl std::fmt::Debug for EventRegistrar {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventRegistrar")
            .field("status", &self.status)
            .finish()
    }
}

impl Drop for EventRegistrar {
    fn drop(&mut self) {
        let _ = self.dispose();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrarDisposeReport {
    pub reports: Vec<UnsubscribeReport>,
}
