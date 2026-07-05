use std::sync::{Arc, PoisonError};

use crate::{
    DomainEvent, EventEnvelope, EventMetadata, EventingError, JournalDispatchPhase, PublishReport,
    QueueDisposition, StoredEventEnvelope,
};

use super::{DispatchMode, DispatchStoredError, EventBus, SubscriberRecord};
use crate::bus::reports::handler::{HandlerOutcome, HandlerReport};
use crate::bus::reports::{dead_letters_for, empty_publish_report};

mod dispatching;

pub(super) async fn publish_with_mode<E>(
    bus: &EventBus,
    event: E,
    metadata: EventMetadata,
    dispatch_mode: DispatchMode,
) -> Result<PublishReport, EventingError>
where
    E: DomainEvent,
{
    bus.ensure_active()?;
    let stored = EventEnvelope::from_event(event, metadata)?.store()?;
    if stored.is_deadline_expired(bus.clock.now()) {
        return dispatching::dead_letter_expired_deadline(bus, stored, dispatch_mode).await;
    }
    let subscribers = bus.subscribers_for(&stored);
    if subscribers.is_empty() {
        return dispatching::publish_without_subscribers(bus, stored, dispatch_mode).await;
    }
    bus.dispatch_stored(
        stored,
        subscribers,
        dispatch_mode,
        bus.queue.report(QueueDisposition::Dispatched),
        true,
    )
    .await
}

impl EventBus {
    pub(crate) async fn dispatch_stored(
        &self,
        stored: StoredEventEnvelope,
        subscribers: Vec<SubscriberRecord>,
        dispatch_mode: DispatchMode,
        queue_report: crate::QueueReport,
        write_journal: bool,
    ) -> Result<PublishReport, EventingError> {
        self.dispatch_stored_checked(
            stored,
            subscribers,
            dispatch_mode,
            queue_report,
            write_journal,
        )
        .await
        .map_err(DispatchStoredError::into_error)
    }

    pub(crate) async fn dispatch_stored_checked(
        &self,
        stored: StoredEventEnvelope,
        subscribers: Vec<SubscriberRecord>,
        dispatch_mode: DispatchMode,
        queue_report: crate::QueueReport,
        write_journal: bool,
    ) -> Result<PublishReport, DispatchStoredError> {
        let reservation = self.queue.reserve_dispatch(&stored)?;
        let _active_dispatch = self.active_dispatches.enter();
        if write_journal {
            self.record_stored_snapshot(&stored).await;
        }
        self.append_journal_phase(&stored, JournalDispatchPhase::BeforeDispatch)
            .await
            .map_err(DispatchStoredError::BeforeDispatch)?;
        let handler_reports = self
            .dispatch(stored.clone(), subscribers.clone(), dispatch_mode)
            .await;
        reservation.complete();
        let dead_letters = dead_letters_for(&stored, &handler_reports);
        if !dead_letters.is_empty() {
            self.record_dead_letters(dead_letters.clone()).await;
        }
        self.append_journal_phase(&stored, JournalDispatchPhase::AfterDispatch)
            .await
            .map_err(DispatchStoredError::AfterDispatch)?;
        Ok(PublishReport {
            event_id: stored.event_id,
            event_type: stored.contract.event_type,
            dispatch_mode,
            queue_report,
            subscriber_count: subscribers.len(),
            handled_count: handler_reports
                .iter()
                .filter(|report| report.outcome == HandlerOutcome::Handled)
                .count(),
            dead_letter_count: dead_letters.len(),
            handler_reports,
        })
    }

    pub(crate) fn subscribers_for(&self, stored: &StoredEventEnvelope) -> Vec<SubscriberRecord> {
        let registry = self.registry.lock().unwrap_or_else(PoisonError::into_inner);
        let subscribers = registry
            .get(&stored.contract.event_type)
            .cloned()
            .unwrap_or_default();
        match &stored.target_handler {
            Some(target) => subscribers
                .into_iter()
                .filter(|subscriber| &subscriber.target_handler == target)
                .collect(),
            None => subscribers,
        }
    }

    async fn dispatch(
        &self,
        stored: StoredEventEnvelope,
        subscribers: Vec<SubscriberRecord>,
        dispatch_mode: DispatchMode,
    ) -> Vec<HandlerReport> {
        dispatching::dispatch(self, stored, subscribers, dispatch_mode).await
    }
}
