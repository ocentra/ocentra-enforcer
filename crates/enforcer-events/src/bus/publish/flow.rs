use serde::Serialize;

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{
    bus::reports::handler::PublishReport,
    envelope::{DomainEvent, EventFrame, EventMetadata},
    error::EventingError,
    queue::policy::QueueReport,
};
use enforcer_domain::events_types::{
    HandlerOutcome, JournalAppendDecision, JournalDispatchPhase, QueueDisposition,
};

use super::{DispatchMode, DispatchStoredError, EventBus};
use crate::bus::reports::dead_letters_for;
use crate::bus::reports::handler::HandlerReport;
use crate::bus::SubscriberRecord;

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
    E: DomainEvent + Serialize,
{
    bus.ensure_active()?;
    let stored = EventFrame::from_event(event, metadata)?.store()?;
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
        JournalAppendDecision::Append,
    )
    .await
}

impl EventBus {
    pub(in crate::bus) async fn dispatch_stored(
        &self,
        request: DispatchRequest,
        queue_report: QueueReport,
        journal_decision: JournalAppendDecision,
    ) -> Result<PublishReport, EventingError> {
        self.dispatch_stored_checked(request, queue_report, journal_decision)
            .await
            .map_err(DispatchStoredError::into_error)
    }

    pub(in crate::bus) async fn dispatch_stored_checked(
        &self,
        request: DispatchRequest,
        queue_report: QueueReport,
        journal_decision: JournalAppendDecision,
    ) -> Result<PublishReport, DispatchStoredError> {
        let DispatchRequest {
            stored,
            subscribers,
            dispatch_mode,
        } = request;
        let reservation = self.queue.reserve_dispatch(&stored)?;
        let _active_dispatch = self.active_dispatches.enter();
        if journal_decision == JournalAppendDecision::Append {
            self.record_stored_snapshot(&stored).await;
        }
        self.append_journal_phase(&stored, JournalDispatchPhase::BeforeDispatch)
            .await
            .map_err(DispatchStoredError::BeforeDispatch)?;
        let handler_reports = self
            // CLONE-JUSTIFICATION: dispatch owns the envelope/subscriber batch while publication retains them for reports and dead letters.
            .dispatch(stored.clone(), subscribers.clone(), dispatch_mode)
            .await
            .map_err(DispatchStoredError::BeforeDispatch)?;
        reservation.complete();
        let dead_letters = dead_letters_for(&stored, &handler_reports);
        if !dead_letters.is_empty() {
            // CLONE-JUSTIFICATION: journal recording owns a batch while the publish report returns the same dead letters.
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
            subscriber_count: crate::boundary::event_values::event_count(subscribers.len()),
            handled_count: crate::boundary::event_values::event_count(
                handler_reports
                    .iter()
                    .filter(|report| report.outcome == HandlerOutcome::Handled)
                    .count(),
            ),
            dead_letter_count: crate::boundary::event_values::event_count(dead_letters.len()),
            handler_reports,
        })
    }

    pub(in crate::bus) fn subscribers_for(
        &self,
        stored: &StoredEventEnvelope,
    ) -> Vec<SubscriberRecord> {
        let registry = self.registry.lock();
        let subscribers = match registry.get(&stored.contract.event_type) {
            // CLONE-JUSTIFICATION: dispatch owns a subscriber snapshot after the registry lock is released.
            Some(subscribers) => subscribers.clone(),
            None => Vec::new(),
        };
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
    ) -> Result<Vec<HandlerReport>, EventingError> {
        dispatching::dispatch(self, stored, subscribers, dispatch_mode).await
    }
}
