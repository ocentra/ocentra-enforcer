use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use std::{
    collections::BTreeMap,
    fmt,
    future::{ready, Future},
    sync::{Arc, Mutex, PoisonError},
};

use enforcer_domain::events_types::{
    AggregateKey, DispatchMode, EventCount, EventShutdownState, ShutdownMode,
};
use tokio::sync::{RwLock, Semaphore};

use crate::{
    clock::SharedEventClock,
    envelope::DomainEvent,
    error::EventingError,
    execution::HandlerExecutionPolicy,
    journal::{policy::JournalPolicy, SharedEventJournal},
    queue::state::EventQueue,
    request::RequestRegistry,
};
use serde::de::DeserializeOwned;

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

use subscriber::{
    insert_subscriber, record_for, remove_subscriber, SubscriberRecord, SubscriberRegistry,
};

use active_dispatch::ActiveDispatchTracker;

use publisher::{EventContext, EventPublisher};
use reports::dead_letter::DeadLetter;
use reports::handler::{EventMetricsSnapshot, HandlerReport, PublishReport, QueueDrainReport};
use subscriber::{EventSubscriber, SubscriptionHandle, SubscriptionReport};

/// Event-runtime data for event bus clear report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBusClearReport {
    pub subscription_count: EventCount,
    pub stored_journal_count: EventCount,
    pub dead_letter_count: EventCount,
    pub aggregate_gate_count: EventCount,
    pub queued_event_count: EventCount,
    pub queued_idempotency_key_count: EventCount,
    pub in_flight_idempotency_key_count: EventCount,
    pub completed_idempotency_key_count: EventCount,
    pub pending_request_count: EventCount,
    pub completed_request_count: EventCount,
    pub timed_out_request_count: EventCount,
}

/// Event-runtime data for event bus shutdown report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBusShutdownReport {
    pub mode: ShutdownMode,
    pub shutdown_state: EventShutdownState,
    pub subscription_count: EventCount,
    pub aggregate_gate_count: EventCount,
    pub queued_event_count: EventCount,
    pub queued_dispatched_count: EventCount,
    pub queued_expired_count: EventCount,
    pub queued_dead_lettered_count: EventCount,
    pub queued_dropped_count: EventCount,
    pub in_flight_dispatch_count: EventCount,
    pub pending_request_count: EventCount,
    pub completed_request_count: EventCount,
    pub timed_out_request_count: EventCount,
}

/// Event-runtime data for event bus.
#[derive(Clone)]
pub struct EventBus {
    registry: SubscriberRegistry,
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

impl fmt::Debug for EventBus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EventBus")
            .field("handler_policy", &self.handler_policy)
            .field("journal_policy", &self.journal_policy)
            .field("has_event_journal", &self.event_journal.is_some())
            .finish_non_exhaustive()
    }
}

impl EventBus {
    /// Executes the subscribe event-runtime operation.
    pub async fn subscribe<E, F, Fut>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionReport, EventingError>
    where
        E: DomainEvent + DeserializeOwned,
        F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
    {
        // CLONE-JUSTIFICATION: the subscription report owns stable identifiers while the subscriber is retained independently by the registry.
        let report = SubscriptionReport {
            // CLONE-JUSTIFICATION: the registry retains subscriber identity while the returned report owns a snapshot.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: the report remains valid independently of registry lifetime.
            event_type: subscriber.event_type.clone(),
            // CLONE-JUSTIFICATION: handler routing identity is owned by both registry and report.
            target_handler: subscriber.target_handler.clone(),
            drain_report: empty_queue_drain_report(),
        };
        self.insert_subscriber(record_for(&subscriber, handler)?)?;
        let drain_report = self.drain_after_subscribe(&report).await?;
        // CLONE-JUSTIFICATION: the returned handle report owns identifiers while the subscriber record is inserted separately.
        let report = SubscriptionReport {
            drain_report,
            ..report
        };
        Ok(report)
    }

    /// Executes the subscribe with handle event-runtime operation.
    pub async fn subscribe_with_handle<E, F, Fut>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionHandle, EventingError>
    where
        E: DomainEvent + DeserializeOwned,
        F: Fn(EventContext<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventingError>> + Send + 'static,
    {
        let report = SubscriptionReport {
            // CLONE-JUSTIFICATION: the registry retains subscriber identity while the returned handle owns its report.
            subscriber_id: subscriber.id.clone(),
            // CLONE-JUSTIFICATION: the report and registry independently own the event routing key.
            event_type: subscriber.event_type.clone(),
            // CLONE-JUSTIFICATION: the report and registry independently own the handler target.
            target_handler: subscriber.target_handler.clone(),
            drain_report: empty_queue_drain_report(),
        };
        self.insert_subscriber(record_for(&subscriber, handler)?)?;
        let drain_report = self.drain_after_subscribe(&report).await?;
        let report = SubscriptionReport {
            drain_report,
            ..report
        };
        // CLONE-JUSTIFICATION: the handle shares the named registry so unsubscribe remains valid after this borrow ends.
        Ok(SubscriptionHandle::new(self.registry.clone(), report))
    }

    /// Executes the subscribe sync event-runtime operation.
    pub async fn subscribe_sync<E, F>(
        &self,
        subscriber: EventSubscriber,
        handler: F,
    ) -> Result<SubscriptionReport, EventingError>
    where
        E: DomainEvent + DeserializeOwned,
        F: Fn(EventContext<E>) -> Result<(), EventingError> + Send + Sync + 'static,
    {
        self.subscribe::<E, _, _>(subscriber, move |context| ready(handler(context)))
            .await
    }

    fn insert_subscriber(&self, record: SubscriberRecord) -> Result<(), EventingError> {
        self.ensure_active()?;
        insert_subscriber(&self.registry, record)
    }

    /// Executes the metrics snapshot event-runtime operation.
    pub async fn metrics_snapshot(&self) -> EventMetricsSnapshot {
        let subscription_count = crate::boundary::event_values::event_count(
            self.registry.lock().values().map(Vec::len).sum::<usize>(),
        );
        EventMetricsSnapshot {
            subscription_count,
            stored_event_count: crate::boundary::event_values::event_count(
                self.stored_journal.read().await.len(),
            ),
            dead_letter_count: crate::boundary::event_values::event_count(
                self.dead_letters.read().await.len(),
            ),
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

    fn begin_shutdown(&self) -> EventShutdownState {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(PoisonError::into_inner);
        match *shutdown {
            EventBusLifecycleState::Active => {
                *shutdown = EventBusLifecycleState::ShuttingDown;
                EventShutdownState::Active
            }
            EventBusLifecycleState::ShuttingDown | EventBusLifecycleState::Shutdown => {
                EventShutdownState::AlreadyShutdown
            }
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
        queued_before: EventCount::ZERO,
        dispatched_count: EventCount::ZERO,
        expired_count: EventCount::ZERO,
        remaining_count: EventCount::ZERO,
        dispatch_reports: Vec::new(),
    }
}
