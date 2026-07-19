use std::sync::PoisonError;

use enforcer_domain::events_types::{EventId, IdempotencyKey, QueueIdempotencyState};

use super::{EventQueue, EventQueueClearReport};

impl EventQueue {
    pub(crate) fn mark_completed(&self, event_id: &EventId, key: IdempotencyKey) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.in_flight_event_ids.remove(event_id);
        state.in_flight_keys.remove(&key);
        // CLONE-JUSTIFICATION: the completed-key registry and its FIFO eviction
        // order each retain an owned copy of the same idempotency key.
        if self.policy.idempotency_registry() == QueueIdempotencyState::Enabled
            && state.completed_keys.insert(key.clone())
        {
            state.completed_key_order.push_back(key);
            super::trim_completed_keys(&mut state);
        }
    }

    pub(crate) fn release_in_flight(&self, event_id: &EventId, key: Option<&IdempotencyKey>) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.in_flight_event_ids.remove(event_id);
        if let Some(key) = key {
            state.in_flight_keys.remove(key);
        }
    }

    pub(crate) fn clear_for_test(&self) -> EventQueueClearReport {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let report = EventQueueClearReport {
            queued_event_count: crate::boundary::event_values::event_count(state.queued.len()),
            queued_idempotency_key_count: crate::boundary::event_values::event_count(
                state.queued_keys.len(),
            ),
            in_flight_idempotency_key_count: crate::boundary::event_values::event_count(
                state.in_flight_keys.len(),
            ),
            completed_idempotency_key_count: crate::boundary::event_values::event_count(
                state.completed_keys.len(),
            ),
        };
        state.queued.clear();
        state.queued_event_ids.clear();
        state.queued_keys.clear();
        state.in_flight_event_ids.clear();
        state.in_flight_keys.clear();
        state.completed_keys.clear();
        state.completed_key_order.clear();
        report
    }
}
