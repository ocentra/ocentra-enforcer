use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use enforcer_domain::events_types::EventDuration;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::oneshot;

mod manual;

pub type EventClockSleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type SharedEventClock = Arc<dyn EventClock>;

/// Contract implemented by event clock.
pub trait EventClock: Send + Sync + 'static {
    fn now(&self) -> EventClockInstant;
    fn sleep<'a>(&'a self, duration: EventDuration) -> EventClockSleep<'a>;
}

/// Event-runtime data for event clock instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventClockInstant {
    elapsed: Duration,
}

// SERIALIZATION-DOC: clock instants persist as their process-relative elapsed Duration representation.
impl Serialize for EventClockInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.elapsed.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EventClockInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            elapsed: Duration::deserialize(deserializer)?,
        })
    }
}

impl EventClockInstant {
    /// Executes the duration since event-runtime operation.
    pub fn duration_since(self, earlier: Self) -> EventDuration {
        crate::boundary::event_values::event_duration(self.elapsed.saturating_sub(earlier.elapsed))
    }

    /// Executes the checked add event-runtime operation.
    pub fn checked_add(self, duration: EventDuration) -> Option<Self> {
        Some(Self {
            elapsed: self.elapsed.checked_add(
                crate::boundary::event_values::event_duration_value(duration),
            )?,
        })
    }
}

impl From<Duration> for EventClockInstant {
    fn from(elapsed: Duration) -> Self {
        Self { elapsed }
    }
}

/// Event-runtime data for system event clock.
#[derive(Clone, Debug)]
pub struct SystemEventClock {
    started_at: SystemClockEpoch,
}

/// Monotonic process-local epoch used only by the system clock adapter.
#[derive(Clone, Copy, Debug)]
#[doc = "BRAND-INVARIANT: the raw Instant never crosses the clock adapter boundary."]
struct SystemClockEpoch(Instant);

impl SystemClockEpoch {
    fn now() -> Self {
        Self(Instant::now())
    }

    fn elapsed(self) -> Duration {
        self.0.elapsed()
    }
}

impl SystemEventClock {
    /// Executes the new event-runtime operation.
    pub fn new() -> Self {
        Self {
            started_at: SystemClockEpoch::now(),
        }
    }

    /// Executes the shared event-runtime operation.
    pub fn shared() -> SharedEventClock {
        Arc::new(Self::new())
    }
}

impl Default for SystemEventClock {
    fn default() -> Self {
        Self::new()
    }
}

impl EventClock for SystemEventClock {
    fn now(&self) -> EventClockInstant {
        EventClockInstant::from(self.started_at.elapsed())
    }

    fn sleep<'a>(&'a self, duration: EventDuration) -> EventClockSleep<'a> {
        Box::pin(tokio::time::sleep(
            crate::boundary::event_values::event_duration_value(duration),
        ))
    }
}

/// Event-runtime data for manual event clock.
#[derive(Clone, Debug, Default)]
pub struct ManualEventClock {
    state: Arc<Mutex<ManualEventClockState>>,
}

#[derive(Default, Debug)]
struct ManualEventClockState {
    now: Duration,
    sleepers: BTreeMap<Duration, Vec<oneshot::Sender<()>>>,
}
