use std::sync::{Arc, Mutex, PoisonError};

use enforcer_domain::events_types::{EventActivityState, EventCount};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(super) struct ActiveDispatchTracker {
    state: Arc<Mutex<EventCount>>,
    idle: Arc<Notify>,
}

impl ActiveDispatchTracker {
    pub(super) fn enter(&self) -> ActiveDispatchGuard {
        let mut active_count = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        *active_count = active_count.incremented();
        // CLONE-JUSTIFICATION: the guard owns a tracker handle so Drop can decrement activity after the caller releases its borrow.
        ActiveDispatchGuard {
            tracker: self.clone(),
            activity: EventActivityState::Active,
        }
    }

    pub(super) fn active_count(&self) -> EventCount {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) async fn wait_for_idle(&self) {
        // CANCELLATION: dropping this wait future cancels its notifier registration;
        // otherwise the loop exits as soon as the tracked dispatch count reaches zero.
        loop {
            let notified = self.idle.notified();
            if self.active_count() == EventCount::ZERO {
                return;
            }
            notified.await;
        }
    }
}

pub(super) struct ActiveDispatchGuard {
    tracker: ActiveDispatchTracker,
    activity: EventActivityState,
}

impl Drop for ActiveDispatchGuard {
    fn drop(&mut self) {
        if self.activity == EventActivityState::Inactive {
            return;
        }
        let mut active_count = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *active_count = active_count.decremented();
        if *active_count == EventCount::ZERO {
            self.tracker.idle.notify_waiters();
        }
    }
}
