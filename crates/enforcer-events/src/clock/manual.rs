use std::sync::{Arc, PoisonError};

use enforcer_domain::events_types::{EventCount, EventDuration};
use tokio::sync::oneshot;

use super::{EventClock, EventClockInstant, EventClockSleep, ManualEventClock, SharedEventClock};

impl ManualEventClock {
    /// Executes the new event-runtime operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes the shared event-runtime operation.
    pub fn shared(&self) -> SharedEventClock {
        // CLONE-JUSTIFICATION: shared test clocks intentionally point at the same synchronized manual state.
        Arc::new(self.clone())
    }

    /// Executes the advance event-runtime operation.
    pub fn advance(&self, duration: EventDuration) {
        let ready_sleepers = self.ready_sleepers(duration);
        for sleeper in ready_sleepers {
            if sleeper.send(()).is_err() {
                continue;
            }
        }
    }

    /// Executes the pending sleep count event-runtime operation.
    pub fn pending_sleep_count(&self) -> EventCount {
        let pending_sleep_count = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .sleepers
            .values()
            .map(Vec::len)
            .sum::<usize>();
        crate::boundary::event_values::event_count(pending_sleep_count)
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

    fn sleep<'a>(&'a self, duration: EventDuration) -> EventClockSleep<'a> {
        let Some(receiver) = self.register_sleep(duration) else {
            return Box::pin(async {});
        };
        Box::pin(async move {
            let _ = receiver.await;
        })
    }
}

impl ManualEventClock {
    fn ready_sleepers(&self, duration: EventDuration) -> Vec<oneshot::Sender<()>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        // Saturate rather than panic on overflow: a manual test clock that
        // hits `Instant`'s upper bound should stay pinned there, not abort.
        state.now = state
            .now
            .saturating_add(crate::boundary::event_values::event_duration_value(
                duration,
            ));
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

    fn register_sleep(
        &self,
        duration: EventDuration,
    ) -> Option<tokio::sync::oneshot::Receiver<()>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let target = state
            .now
            .checked_add(crate::boundary::event_values::event_duration_value(
                duration,
            ))?;
        if target <= state.now {
            return None;
        }
        let (sender, receiver) = oneshot::channel();
        state.sleepers.entry(target).or_default().push(sender);
        Some(receiver)
    }
}
