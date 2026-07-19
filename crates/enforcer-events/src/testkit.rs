use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, PoisonError},
};

use crate::{
    bus::{
        subscriber::{EventSubscriber, SubscriptionHandle},
        EventBus,
    },
    envelope::{DomainEvent, EventFrame},
    error::EventingError,
};

/// Event-runtime data for event recorder.
#[derive(Debug)]
pub struct EventRecorder<E>
where
    E: DomainEvent + Clone + std::fmt::Debug + Send + Sync + serde::de::DeserializeOwned + 'static,
{
    events: Arc<Mutex<Vec<EventFrame<E>>>>,
    handle: SubscriptionHandle,
    _event: PhantomData<E>,
}

impl<E> EventRecorder<E>
where
    E: DomainEvent + Clone + std::fmt::Debug + Send + Sync + serde::de::DeserializeOwned + 'static,
{
    /// Executes the attach event-runtime operation.
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
                        // CLONE-JUSTIFICATION: the recorder owns a snapshot while the handler context remains borrowed.
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

    /// Executes the recorded event-runtime operation.
    pub async fn recorded(&self) -> Vec<EventFrame<E>> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            // CLONE-JUSTIFICATION: callers receive an owned snapshot while the recorder retains its history.
            .clone()
    }

    /// Executes the unsubscribe event-runtime operation.
    pub fn unsubscribe(&self) -> enforcer_domain::events_types::SubscriptionRemovalState {
        self.handle.unsubscribe().removal_state
    }
}
