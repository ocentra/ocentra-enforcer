use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, PoisonError,
    },
};

use crate::{
    DomainEvent, EventType, EventingError, StoredEventEnvelope, SubscriberId, TargetHandler,
};

use super::{EventContext, EventPublisher, QueueDrainReport};

type HandlerFuture = Pin<Box<dyn Future<Output = Result<(), EventingError>> + Send>>;
type StoredHandler = dyn Fn(StoredEventEnvelope, EventPublisher) -> HandlerFuture + Send + Sync;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSubscriber {
    pub id: SubscriberId,
    pub event_type: EventType,
    pub target_handler: TargetHandler,
}

impl EventSubscriber {
    pub fn new(id: SubscriberId, event_type: EventType, target_handler: TargetHandler) -> Self {
        Self {
            id,
            event_type,
            target_handler,
        }
    }
}

#[derive(Clone)]
pub(super) struct SubscriberRecord {
    pub(super) id: SubscriberId,
    pub(super) event_type: EventType,
    pub(super) target_handler: TargetHandler,
    pub(super) handler: Arc<StoredHandler>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionReport {
    pub subscriber_id: SubscriberId,
    pub event_type: EventType,
    pub target_handler: TargetHandler,
    pub drain_report: QueueDrainReport,
}

pub struct SubscriptionHandle {
    registry: Arc<Mutex<BTreeMap<EventType, Vec<SubscriberRecord>>>>,
    report: SubscriptionReport,
    active: Arc<AtomicBool>,
}

impl SubscriptionHandle {
    pub(super) fn new(
        registry: Arc<Mutex<BTreeMap<EventType, Vec<SubscriberRecord>>>>,
        report: SubscriptionReport,
    ) -> Self {
        Self {
            registry,
            report,
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn report(&self) -> SubscriptionReport {
        // CLONE-JUSTIFICATION: the handle retains its report for idempotent unsubscribe,
        // while callers receive an independently owned snapshot.
        self.report.clone()
    }

    pub fn unsubscribe(&self) -> UnsubscribeReport {
        let removed = if self.active.swap(false, Ordering::AcqRel) {
            remove_subscriber(
                &self.registry,
                &self.report.event_type,
                &self.report.subscriber_id,
            )
        } else {
            false
        };
        UnsubscribeReport {
            // CLONE-JUSTIFICATION: the handle retains its report so repeated unsubscribe
            // calls can return the same subscription identity without changing state.
            subscriber_id: self.report.subscriber_id.clone(),
            // CLONE-JUSTIFICATION: the handle retains its report so repeated unsubscribe
            // calls can return the same subscription identity without changing state.
            event_type: self.report.event_type.clone(),
            removed,
        }
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        let _ = self.unsubscribe();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsubscribeReport {
    pub subscriber_id: SubscriberId,
    pub event_type: EventType,
    pub removed: bool,
}

pub(super) fn record_for<E, F, Fut>(
    subscriber: &EventSubscriber,
    handler: F,
) -> Result<SubscriberRecord, EventingError>
where
    E: DomainEvent,
    F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
{
    let callback = Arc::new(handler);
    Ok(SubscriberRecord {
        // CLONE-JUSTIFICATION: the subscriber is borrowed by the registration API while
        // the registry record must retain an owned identifier after the call returns.
        id: subscriber.id.clone(),
        // CLONE-JUSTIFICATION: the subscriber is borrowed by the registration API while
        // the registry record must retain an owned event type after the call returns.
        event_type: subscriber.event_type.clone(),
        // CLONE-JUSTIFICATION: the subscriber is borrowed by the registration API while
        // the registry record must retain an owned routing target after the call returns.
        target_handler: subscriber.target_handler.clone(),
        handler: Arc::new(move |stored, publisher| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                let envelope = stored.decode::<E>()?;
                callback(EventContext::new(envelope, publisher)).await
            })
        }),
    })
}

pub(super) fn insert_subscriber(
    registry: &Arc<Mutex<BTreeMap<EventType, Vec<SubscriberRecord>>>>,
    record: SubscriberRecord,
) -> Result<(), EventingError> {
    let mut registry = registry.lock().unwrap_or_else(PoisonError::into_inner);
    // CLONE-JUSTIFICATION: the map key and the queued record each own the event type;
    // the record remains intact for insertion after selecting its bucket.
    let subscribers = registry.entry(record.event_type.clone()).or_default();
    if subscribers
        .iter()
        .any(|subscriber| subscriber.id == record.id)
    {
        return Err(EventingError::DuplicateSubscriber {
            subscriber_id: record.id,
        });
    }
    subscribers.push(record);
    Ok(())
}

pub(super) fn remove_subscriber(
    registry: &Arc<Mutex<BTreeMap<EventType, Vec<SubscriberRecord>>>>,
    event_type: &EventType,
    subscriber_id: &SubscriberId,
) -> bool {
    let mut registry = registry.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(subscribers) = registry.get_mut(event_type) else {
        return false;
    };
    let original_len = subscribers.len();
    subscribers.retain(|subscriber| &subscriber.id != subscriber_id);
    let removed = subscribers.len() != original_len;
    if subscribers.is_empty() {
        registry.remove(event_type);
    }
    removed
}
