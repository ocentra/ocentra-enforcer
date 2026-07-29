use std::sync::PoisonError;

use enforcer_domain::events_types::{
    DeadLetterReason, NoSubscriberQueuePolicy, QueueDisposition, QueueIdempotencyState,
};

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::queue::reservation::DispatchReservation;
use crate::{clock::EventClockInstant, error::EventingError};

use super::{EventQueue, NoSubscriberQueueDecision};

impl EventQueue {
    pub(crate) fn enqueue_no_subscriber(
        &self,
        stored: StoredEventEnvelope,
        now: EventClockInstant,
    ) -> Result<NoSubscriberQueueDecision, EventingError> {
        // CLONE-JUSTIFICATION: the no-subscriber error/report retains the event type after the stored envelope is moved into a queue decision.
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
        // CLONE-JUSTIFICATION: reservation indexes own identity keys while the borrowed stored envelope remains available to dispatch.
        let event_id = stored.event_id.clone();
        if state.queued_event_ids.contains(&event_id)
            || !state.in_flight_event_ids.insert(event_id.clone())
        {
            return Err(EventingError::DuplicateEventId { event_id });
        }
        if self.policy.idempotency_registry() == QueueIdempotencyState::Disabled {
            // CLONE-JUSTIFICATION: the reservation owns a queue handle so Drop can release the in-flight identity.
            return Ok(DispatchReservation::new(self.clone(), Some(event_id), None));
        }
        // CLONE-JUSTIFICATION: the in-flight registry and returned reservation independently own the idempotency key.
        let key = stored.idempotency_key.clone();
        if state.completed_keys.contains(&key) || state.queued_keys.contains(&key) {
            state.in_flight_event_ids.remove(&event_id);
            return Err(EventingError::DuplicateIdempotencyKey {
                idempotency_key: key,
            });
        }
        // CLONE-JUSTIFICATION: in-flight tracking owns the key while the reservation returns it for eventual release.
        if !state.in_flight_keys.insert(key.clone()) {
            state.in_flight_event_ids.remove(&event_id);
            return Err(EventingError::DuplicateInFlight {
                idempotency_key: key,
            });
        }
        // CLONE-JUSTIFICATION: the reservation owns a queue handle so completion or Drop can update registry state.
        Ok(DispatchReservation::new(
            self.clone(),
            Some(event_id),
            Some(key),
        ))
    }
}
