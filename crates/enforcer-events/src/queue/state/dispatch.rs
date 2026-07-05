use std::sync::PoisonError;

use crate::bus::reports::dead_letter::DeadLetterReason;
use crate::queue::policy::{
    NoSubscriberQueuePolicy, QueueDisposition, QueueOverflowPolicy, QueueReport,
};
use crate::queue::reservation::DispatchReservation;
use crate::{EventClockInstant, EventType, EventingError, StoredEventEnvelope};

use super::{EventQueue, NoSubscriberQueueDecision};

impl EventQueue {
    pub(crate) fn enqueue_no_subscriber(
        &self,
        stored: StoredEventEnvelope,
        now: EventClockInstant,
    ) -> Result<NoSubscriberQueueDecision, EventingError> {
        let event_type = stored.contract.event_type.clone();
        match self.policy.no_subscriber() {
            NoSubscriberQueuePolicy::DispatchWithoutSubscribers => Ok(
                NoSubscriberQueueDecision::Dispatch(self.report(QueueDisposition::Dispatched)),
            ),
            NoSubscriberQueuePolicy::DeadLetter => Ok(NoSubscriberQueueDecision::DeadLetter(
                self.report(QueueDisposition::DeadLetteredNoSubscriber),
                DeadLetterReason::NoSubscriber,
                EventingError::NoSubscriber { event_type },
            )),
            NoSubscriberQueuePolicy::Queue => self.try_enqueue(stored, now),
        }
    }

    pub(crate) fn reserve_dispatch(
        &self,
        stored: &StoredEventEnvelope,
    ) -> Result<DispatchReservation, EventingError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let event_id = stored.event_id.clone();
        if state.queued_event_ids.contains(&event_id)
            || !state.in_flight_event_ids.insert(event_id.clone())
        {
            return Err(EventingError::DuplicateEventId { event_id });
        }
        if !self.policy.idempotency_registry_enabled() {
            return Ok(DispatchReservation::new(self.clone(), Some(event_id), None));
        }
        let key = stored.idempotency_key.clone();
        if state.completed_keys.contains(&key) || state.queued_keys.contains(&key) {
            state.in_flight_event_ids.remove(&event_id);
            return Err(EventingError::DuplicateIdempotencyKey {
                idempotency_key: key,
            });
        }
        if !state.in_flight_keys.insert(key.clone()) {
            state.in_flight_event_ids.remove(&event_id);
            return Err(EventingError::DuplicateInFlight {
                idempotency_key: key,
            });
        }
        Ok(DispatchReservation::new(
            self.clone(),
            Some(event_id),
            Some(key),
        ))
    }
}
