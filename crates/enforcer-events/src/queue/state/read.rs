use std::sync::PoisonError;

use enforcer_domain::events_types::{
    EventCount, EventMatchState, EventType, QueueIdempotencyState,
};

use super::{EventQueue, QueuedEnvelope};

impl EventQueue {
    pub(crate) fn queued_count(&self, event_type: Option<&EventType>) -> EventCount {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        crate::boundary::event_values::event_count(
            state
                .queued
                .iter()
                .filter(|queued| {
                    event_type.is_none_or(|event_type| {
                        queued.event_type_match(event_type) == EventMatchState::Matches
                    })
                })
                .count(),
        )
    }

    pub(crate) fn take_next_queued(
        &self,
        event_type: Option<&EventType>,
    ) -> Option<QueuedEnvelope> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let position = state.queued.iter().position(|queued| {
            event_type.is_none_or(|event_type| {
                queued.event_type_match(event_type) == EventMatchState::Matches
            })
        })?;
        // `position` was just found via `.position()` on this same
        // `VecDeque` under this same lock, so `remove` cannot miss.
        let queued = state.queued.remove(position)?;
        state.queued_event_ids.remove(&queued.stored.event_id);
        if self.policy.idempotency_registry() == QueueIdempotencyState::Enabled {
            state.queued_keys.remove(&queued.stored.idempotency_key);
        }
        Some(queued)
    }

    pub(crate) fn take_all_queued(&self) -> Vec<QueuedEnvelope> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let queued = state.queued.drain(..).collect();
        state.queued_event_ids.clear();
        state.queued_keys.clear();
        queued
    }

    pub(crate) fn requeue(&self, queued: QueuedEnvelope) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .queued_event_ids
                // CLONE-JUSTIFICATION: the report set owns identity while the queued envelope remains intact.
                .insert(queued.stored.event_id.clone());
        if self.policy.idempotency_registry() == QueueIdempotencyState::Enabled {
            state
                .queued_keys
                // CLONE-JUSTIFICATION: the report set owns idempotency identity while the queued envelope remains intact.
                .insert(queued.stored.idempotency_key.clone());
        }
        state.queued.push_back(queued);
    }
}
