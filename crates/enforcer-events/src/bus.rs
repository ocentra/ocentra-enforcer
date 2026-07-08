use std::{
    collections::BTreeMap,
    future::{ready, Future},
    sync::{Arc, Mutex, PoisonError},
};

use tokio::sync::{RwLock, Semaphore};

use crate::{
    AggregateKey, DomainEvent, EventQueue, EventType, EventingError, HandlerExecutionPolicy,
    JournalPolicy, RequestRegistry, SharedEventClock, SharedEventJournal, StoredEventEnvelope,
};

mod active_dispatch;
mod aggregate_gate;
mod builders;
mod dispatch;
mod journaling;
mod lifecycle;
mod publish;
pub mod publisher;
mod queue_drain;
pub mod reports;
pub mod subscriber;

use subscriber::{insert_subscriber, record_for, remove_subscriber, SubscriberRecord};

use active_dispatch::ActiveDispatchTracker;

use publisher::{EventContext, EventPublisher};
use reports::dead_letter::DeadLetter;
use reports::handler::{EventMetricsSnapshot, HandlerReport, PublishReport, QueueDrainReport};
use subscriber::{EventSubscriber, SubscriptionHandle, SubscriptionReport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchMode {
    Sequential,
    Concurrent,
    OrderedByAggregateKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBusClearReport {
    pub subscription_count: usize,
    pub stored_journal_count: usize,
    pub dead_letter_count: usize,
    pub aggregate_gate_count: usize,
    pub queued_event_count: usize,
    pub queued_idempotency_key_count: usize,
    pub in_flight_idempotency_key_count: usize,
    pub completed_idempotency_key_count: usize,
    pub pending_request_count: usize,
    pub completed_request_count: usize,
    pub timed_out_request_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownMode {
    Drain,
    DeadLetterQueued,
    DropQueuedForTestOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBusShutdownReport {
    pub mode: ShutdownMode,
    pub already_shutdown: bool,
    pub subscription_count: usize,
    pub aggregate_gate_count: usize,
    pub queued_event_count: usize,
    pub queued_dispatched_count: usize,
    pub queued_expired_count: usize,
    pub queued_dead_lettered_count: usize,
    pub queued_dropped_count: usize,
    pub in_flight_dispatch_count: usize,
    pub pending_request_count: usize,
    pub completed_request_count: usize,
    pub timed_out_request_count: usize,
}

#[derive(Clone)]
pub struct EventBus {
    registry: Arc<Mutex<BTreeMap<EventType, Vec<SubscriberRecord>>>>,
    stored_journal: Arc<RwLock<Vec<StoredEventEnvelope>>>,
    dead_letters: Arc<RwLock<Vec<DeadLetter>>>,
    aggregate_gates: Arc<Mutex<BTreeMap<AggregateKey, Arc<Semaphore>>>>,
    handler_policy: HandlerExecutionPolicy,
    queue: EventQueue,
    requests: RequestRegistry,
    journal_policy: JournalPolicy,
    event_journal: Option<SharedEventJournal>,
    clock: SharedEventClock,
    shutdown: Arc<Mutex<EventBusLifecycleState>>,
    active_dispatches: ActiveDispatchTracker,
}

impl EventBus {
    pub async fn subscribe<E, F, Fut>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionReport, EventingError>
    where
        E: DomainEvent,
        F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
    {
        let report = SubscriptionReport {
            subscriber_id: subscriber.id.clone(),
            event_type: subscriber.event_type.clone(),
            target_handler: subscriber.target_handler.clone(),
            drain_report: empty_queue_drain_report(),
        };
        self.insert_subscriber(record_for(&subscriber, handler)?)?;
        let drain_report = self.drain_after_subscribe(&report).await?;
        let report = SubscriptionReport {
            drain_report,
            ..report
        };
        Ok(report)
    }

    pub async fn subscribe_with_handle<E, F, Fut>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionHandle, EventingError>
    where
        E: DomainEvent,
        F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
    {
        let report = SubscriptionReport {
            subscriber_id: subscriber.id.clone(),
            event_type: subscriber.event_type.clone(),
            target_handler: subscriber.target_handler.clone(),
            drain_report: empty_queue_drain_report(),
        };
        self.insert_subscriber(record_for(&subscriber, handler)?)?;
        let drain_report = self.drain_after_subscribe(&report).await?;
        let report = SubscriptionReport {
            drain_report,
            ..report
        };
        Ok(SubscriptionHandle::new(Arc::clone(&self.registry), report))
    }

    pub async fn subscribe_sync<E, F>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionReport, EventingError>
    where
        E: DomainEvent,
        F: Fn(EventContext<E>) -> Result<(), EventingError> + Send + Sync + 'static,
    {
        self.subscribe::<E, _, _>(subscriber, move |context| ready(handler(context)))
            .await
    }

    fn insert_subscriber(&self, record: SubscriberRecord) -> Result<(), EventingError> {
        self.ensure_active()?;
        insert_subscriber(&self.registry, record)
    }

    pub async fn metrics_snapshot(&self) -> EventMetricsSnapshot {
        let subscription_count = self
            .registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(Vec::len)
            .sum();
        EventMetricsSnapshot {
            subscription_count,
            stored_event_count: self.stored_journal.read().await.len(),
            dead_letter_count: self.dead_letters.read().await.len(),
            queue: self.queue.metrics(),
            requests: self.requests.metrics(),
        }
    }

    async fn drain_after_subscribe(
        &self,
        report: &SubscriptionReport,
    ) -> Result<QueueDrainReport, EventingError> {
        match self
            .drain_queued_for_event_unchecked(DispatchMode::Sequential, &report.event_type)
            .await
        {
            Ok(drain_report) => Ok(drain_report),
            Err(error) => {
                remove_subscriber(&self.registry, &report.event_type, &report.subscriber_id);
                Err(error)
            }
        }
    }

    fn ensure_active(&self) -> Result<(), EventingError> {
        if *self.shutdown.lock().unwrap_or_else(PoisonError::into_inner)
            != EventBusLifecycleState::Active
        {
            return Err(EventingError::BusShutdown);
        }
        Ok(())
    }

    fn begin_shutdown(&self) -> bool {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(PoisonError::into_inner);
        match *shutdown {
            EventBusLifecycleState::Active => {
                *shutdown = EventBusLifecycleState::ShuttingDown;
                false
            }
            EventBusLifecycleState::ShuttingDown | EventBusLifecycleState::Shutdown => true,
        }
    }

    fn mark_shutdown(&self) {
        *self.shutdown.lock().unwrap_or_else(PoisonError::into_inner) =
            EventBusLifecycleState::Shutdown;
    }

    fn rollback_shutdown(&self) {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(PoisonError::into_inner);
        if *shutdown == EventBusLifecycleState::ShuttingDown {
            *shutdown = EventBusLifecycleState::Active;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventBusLifecycleState {
    Active,
    ShuttingDown,
    Shutdown,
}

fn empty_queue_drain_report() -> QueueDrainReport {
    QueueDrainReport {
        queued_before: 0,
        dispatched_count: 0,
        expired_count: 0,
        remaining_count: 0,
        dispatch_reports: Vec::new(),
    }
}
