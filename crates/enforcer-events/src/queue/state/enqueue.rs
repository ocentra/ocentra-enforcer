use std::sync::PoisonError;

use crate::bus::reports::dead_letter::DeadLetterReason;
use crate::queue::policy::{QueueDisposition, QueueOverflowPolicy, QueueReport};
use crate::{EventClockInstant, EventingError, StoredEventEnvelope};

use super::{EventQueue, EventQueueState, NoSubscriberQueueDecision, QueuedEnvelope};

impl EventQueue {
    pub(crate) fn try_enqueue(
        &self,
        stored: StoredEventEnvelope,
        now: EventClockInstant,
    ) -> Result<NoSubscriberQueueDecision, EventingError> {
        let Some(capacity) = self.policy.capacity() else {
            return Err(EventingError::InvalidQueuePolicy {
                reason: String::from("queue capacity is not configured"),
            });
        };
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        // CLONE-JUSTIFICATION: The queued envelope retains its event id while the queue index owns a separate lookup key.
        let event_id = stored.event_id.clone();
        ensure_event_id_available(&state, &event_id)?;
        // CLONE-JUSTIFICATION: The queued envelope retains its idempotency key while duplicate detection borrows a stable key.
        let key = stored.idempotency_key.clone();
        ensure_idempotency_available(self.policy.idempotency_registry_enabled(), &state, &key)?;
        if state.queued.len() >= capacity {
            return self.overflow_decision(stored, &mut state, capacity, now);
        }
        enqueue_queued_event(
            self.policy.idempotency_registry_enabled(),
            &mut state,
            stored,
            now,
            key,
        );
        Ok(NoSubscriberQueueDecision::Queued(QueueReport {
            disposition: QueueDisposition::QueuedNoSubscriber,
            queued_count: state.queued.len(),
            capacity: self.policy.capacity(),
        }))
    }

    fn overflow_decision(
        &self,
        stored: StoredEventEnvelope,
        state: &mut EventQueueState,
        capacity: usize,
        now: EventClockInstant,
    ) -> Result<NoSubscriberQueueDecision, EventingError> {
        // CLONE-JUSTIFICATION: `stored` is moved into the selected queue outcome, so an owned event type must survive for reject/dead-letter errors.
        let event_type = stored.contract.event_type.clone();
        match self.policy.overflow() {
            QueueOverflowPolicy::RejectPublish => reject_overflow(event_type, capacity),
            QueueOverflowPolicy::DeadLetterRejected => Ok(dead_letter_overflow(
                state,
                event_type,
                capacity,
                self.policy.capacity(),
            )),
            QueueOverflowPolicy::DropOldestAndDeadLetter => {
                drop_oldest_and_dead_letter(self, stored, state, capacity, now)
            }
        }
    }
}

fn reject_overflow(
    event_type: crate::EventType,
    capacity: usize,
) -> Result<NoSubscriberQueueDecision, EventingError> {
    Err(EventingError::QueueCapacityExceeded {
        event_type,
        capacity,
    })
}

fn dead_letter_overflow(
    state: &EventQueueState,
    event_type: crate::EventType,
    capacity: usize,
    policy_capacity: Option<usize>,
) -> NoSubscriberQueueDecision {
    NoSubscriberQueueDecision::DeadLetter(
        QueueReport {
            disposition: QueueDisposition::DeadLetteredQueueOverflow,
            queued_count: state.queued.len(),
            capacity: policy_capacity,
        },
        DeadLetterReason::QueueOverflow,
        EventingError::QueueCapacityExceeded {
            event_type,
            capacity,
        },
    )
}

fn drop_oldest_and_dead_letter(
    queue: &EventQueue,
    stored: StoredEventEnvelope,
    state: &mut EventQueueState,
    capacity: usize,
    now: EventClockInstant,
) -> Result<NoSubscriberQueueDecision, EventingError> {
    let Some(dropped) = state.queued.pop_front() else {
        return Err(EventingError::InvalidQueuePolicy {
            reason: String::from("drop-oldest overflow requires a queued event"),
        });
    };
    state.queued_event_ids.remove(&dropped.stored.event_id);
    state.queued_keys.remove(&dropped.stored.idempotency_key);
    // CLONE-JUSTIFICATION: The replacement envelope remains owned by the queue after its event-id lookup key is inserted.
    state.queued_event_ids.insert(stored.event_id.clone());
    if queue.policy.idempotency_registry_enabled() {
        // CLONE-JUSTIFICATION: The replacement envelope remains owned by the queue after its idempotency lookup key is inserted.
        state.queued_keys.insert(stored.idempotency_key.clone());
    }
    // CLONE-JUSTIFICATION: The dropped envelope is returned to the caller, while the capacity error requires an owned event type.
    let dropped_event_type = dropped.stored.contract.event_type.clone();
    state.queued.push_back(QueuedEnvelope {
        stored,
        enqueued_at: now,
    });
    Ok(NoSubscriberQueueDecision::QueuedWithDeadLetter(
        QueueReport {
            disposition: QueueDisposition::DeadLetteredQueueOverflow,
            queued_count: state.queued.len(),
            capacity: queue.policy.capacity(),
        },
        Box::new(dropped.stored),
        DeadLetterReason::QueueOverflow,
        EventingError::QueueCapacityExceeded {
            event_type: dropped_event_type,
            capacity,
        },
    ))
}

fn ensure_event_id_available(
    state: &super::EventQueueState,
    event_id: &crate::EventId,
) -> Result<(), EventingError> {
    if state.queued_event_ids.contains(event_id) || state.in_flight_event_ids.contains(event_id) {
        // CLONE-JUSTIFICATION: The duplicate-id error owns the id, while the caller retains the borrowed id for its queue operation.
        return Err(EventingError::DuplicateEventId {
            event_id: event_id.clone(),
        });
    }
    Ok(())
}

fn ensure_idempotency_available(
    enabled: bool,
    state: &super::EventQueueState,
    key: &crate::IdempotencyKey,
) -> Result<(), EventingError> {
    if enabled
        && (state.completed_keys.contains(key)
            || state.queued_keys.contains(key)
            || state.in_flight_keys.contains(key))
    {
        // CLONE-JUSTIFICATION: The duplicate-key error owns the key, while the caller retains the borrowed key for its queue operation.
        return Err(EventingError::DuplicateIdempotencyKey {
            idempotency_key: key.clone(),
        });
    }
    Ok(())
}

fn enqueue_queued_event(
    enabled: bool,
    state: &mut super::EventQueueState,
    stored: StoredEventEnvelope,
    now: EventClockInstant,
    key: crate::IdempotencyKey,
) {
    // CLONE-JUSTIFICATION: The queued envelope retains its event id while the queue index owns a separate lookup key.
    state.queued_event_ids.insert(stored.event_id.clone());
    if enabled {
        state.queued_keys.insert(key);
    }
    state.queued.push_back(QueuedEnvelope {
        stored,
        enqueued_at: now,
    });
}
