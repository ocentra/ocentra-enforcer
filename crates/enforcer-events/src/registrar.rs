use std::{future::Future, mem};

use crate::{
    DomainEvent, EventBus, EventContext, EventSubscriber, EventingError, SubscriptionHandle,
    SubscriptionReport, UnsubscribeReport,
};

#[derive(Default)]
pub struct EventRegistrar {
    handles: Vec<SubscriptionHandle>,
    disposed: bool,
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
        if self.disposed {
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
        self.disposed = true;
        RegistrarDisposeReport { reports }
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed
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
