use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, PoisonError},
};

use crate::{
    DomainEvent, EventBus, EventEnvelope, EventSubscriber, EventingError, SubscriptionHandle,
};

pub struct EventRecorder<E>
where
    E: DomainEvent + Clone + Send + Sync + 'static,
{
    events: Arc<Mutex<Vec<EventEnvelope<E>>>>,
    handle: SubscriptionHandle,
    _event: PhantomData<E>,
}

impl<E> EventRecorder<E>
where
    E: DomainEvent + Clone + Send + Sync + 'static,
{
    pub async fn attach(
        bus: &EventBus,
        subscriber: EventSubscriber,
    ) -> Result<Self, EventingError> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded_events = Arc::clone(&events);
        let handle = bus
            .subscribe_with_handle::<E, _, _>(subscriber, move |context| {
                let recorded_events = Arc::clone(&recorded_events);
                async move {
                    recorded_events
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(context.envelope().clone());
                    Ok(())
                }
            })
            .await?;
        Ok(Self {
            events,
            handle,
            _event: PhantomData,
        })
    }

    pub async fn recorded(&self) -> Vec<EventEnvelope<E>> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn unsubscribe(&self) -> bool {
        self.handle.unsubscribe().removed
    }
}
