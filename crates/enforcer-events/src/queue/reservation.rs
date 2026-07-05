use crate::{EventId, IdempotencyKey};

use super::state::EventQueue;

pub(crate) struct DispatchReservation {
    queue: EventQueue,
    event_id: Option<EventId>,
    key: Option<IdempotencyKey>,
}

impl DispatchReservation {
    pub(super) fn new(
        queue: EventQueue,
        event_id: Option<EventId>,
        key: Option<IdempotencyKey>,
    ) -> Self {
        Self {
            queue,
            event_id,
            key,
        }
    }

    pub(crate) fn complete(mut self) {
        if let Some(event_id) = self.event_id.take() {
            if let Some(key) = self.key.take() {
                self.queue.mark_completed(&event_id, key);
            } else {
                self.queue.release_in_flight(&event_id, None);
            }
        }
    }
}

impl Drop for DispatchReservation {
    fn drop(&mut self) {
        if let Some(event_id) = self.event_id.take() {
            self.queue.release_in_flight(&event_id, self.key.as_ref());
            self.key.take();
        }
    }
}
