use std::{
    sync::{Arc, PoisonError},
    time::Duration,
};

use tokio::sync::oneshot;

use super::{EventClock, EventClockInstant, EventClockSleep, ManualEventClock, SharedEventClock};

impl ManualEventClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared(&self) -> SharedEventClock {
        Arc::new(self.clone())
    }

    pub fn advance(&self, duration: Duration) {
        let ready_sleepers = self.ready_sleepers(duration);
        for sleeper in ready_sleepers {
            let _ = sleeper.send(());
        }
    }

    pub fn pending_sleep_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .sleepers
            .values()
            .map(Vec::len)
            .sum()
    }
}

impl EventClock for ManualEventClock {
    fn now(&self) -> EventClockInstant {
        EventClockInstant::from(
            self.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .now,
        )
    }

    fn sleep<'a>(&'a self, duration: Duration) -> EventClockSleep<'a> {
        let Some(receiver) = self.register_sleep(duration) else {
            return Box::pin(async {});
        };
        Box::pin(async move {
            let _ = receiver.await;
        })
    }
}

impl ManualEventClock {
    fn ready_sleepers(&self, duration: Duration) -> Vec<oneshot::Sender<()>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        // Saturate rather than panic on overflow: a manual test clock that
        // hits `Instant`'s upper bound should stay pinned there, not abort.
        state.now = state.now.saturating_add(duration);
        let ready_targets = state
            .sleepers
            .keys()
            .copied()
            .take_while(|target| *target <= state.now)
            .collect::<Vec<_>>();
        let mut ready_sleepers = Vec::new();
        for target in ready_targets {
            if let Some(mut sleepers) = state.sleepers.remove(&target) {
                ready_sleepers.append(&mut sleepers);
            }
        }
        ready_sleepers
    }

    fn register_sleep(&self, duration: Duration) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let target = state.now.checked_add(duration)?;
        if target <= state.now {
            return None;
        }
        let (sender, receiver) = oneshot::channel();
        state.sleepers.entry(target).or_default().push(sender);
        Some(receiver)
    }
}
