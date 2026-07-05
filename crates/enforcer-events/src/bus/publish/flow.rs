use std::sync::PoisonError;

use crate::{
    DomainEvent, EventEnvelope, EventMetadata, EventingError, JournalDispatchPhase, PublishReport,
    QueueDisposition, StoredEventEnvelope,
};

use super::{DispatchMode, DispatchStoredError, EventBus, SubscriberRecord};
use crate::bus::reports::dead_letters_for;
use crate::bus::reports::handler::{HandlerOutcome, HandlerReport};

mod dispatching;

/// What to dispatch: the stored envelope, the subscribers it fans out to,
/// and the mode to dispatch under -- grouped so `dispatch_stored`/
/// `dispatch_stored_checked` take one cohesive parameter instead of three
/// independent ones that are always supplied together.
pub(in crate::bus) struct DispatchRequest {
    pub(in crate::bus) stored: StoredEventEnvelope,
    pub(in crate::bus) subscribers: Vec<SubscriberRecord>,
    pub(in crate::bus) dispatch_mode: DispatchMode,
}

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
    let queue_report = bus.queue.report(QueueDisposition::Dispatched);
    bus.dispatch_stored(
        DispatchRequest {
            stored,
            subscribers,
            dispatch_mode,
        },
        queue_report,
        true,
    )
    .await
}

impl EventBus {
    pub(in crate::bus) async fn dispatch_stored(
        &self,
        request: DispatchRequest,
        queue_report: crate::QueueReport,
        write_journal: bool,
    ) -> Result<PublishReport, EventingError> {
        self.dispatch_stored_checked(request, queue_report, write_journal)
            .await
            .map_err(DispatchStoredError::into_error)
    }

    pub(in crate::bus) async fn dispatch_stored_checked(
        &self,
        request: DispatchRequest,
        queue_report: crate::QueueReport,
        write_journal: bool,
    ) -> Result<PublishReport, DispatchStoredError> {
        let DispatchRequest {
            stored,
            subscribers,
            dispatch_mode,
        } = request;
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

    pub(in crate::bus) fn subscribers_for(&self, stored: &StoredEventEnvelope) -> Vec<SubscriberRecord> {
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
