use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(super) struct ActiveDispatchTracker {
    state: Arc<Mutex<usize>>,
    idle: Arc<Notify>,
}

impl ActiveDispatchTracker {
    pub(super) fn enter(&self) -> ActiveDispatchGuard {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        ActiveDispatchGuard {
            tracker: self.clone(),
            active: true,
        }
    }

    pub(super) fn active_count(&self) -> usize {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) async fn wait_for_idle(&self) {
        loop {
            let notified = self.idle.notified();
            if self.active_count() == 0 {
                return;
            }
            notified.await;
        }
    }
}

pub(super) struct ActiveDispatchGuard {
    tracker: ActiveDispatchTracker,
    active: bool,
}

impl Drop for ActiveDispatchGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let mut active_count = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *active_count = active_count.saturating_sub(1);
        if *active_count == 0 {
            self.tracker.idle.notify_waiters();
        }
    }
}
