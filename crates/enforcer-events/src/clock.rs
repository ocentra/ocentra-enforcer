use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

mod manual;

pub type EventClockSleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type SharedEventClock = Arc<dyn EventClock>;

pub trait EventClock: Send + Sync + 'static {
    fn now(&self) -> EventClockInstant;
    fn sleep<'a>(&'a self, duration: Duration) -> EventClockSleep<'a>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventClockInstant {
    elapsed: Duration,
}

impl EventClockInstant {
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.elapsed.saturating_sub(earlier.elapsed)
    }

    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        Some(Self {
            elapsed: self.elapsed.checked_add(duration)?,
        })
    }
}

impl From<Duration> for EventClockInstant {
    fn from(elapsed: Duration) -> Self {
        Self { elapsed }
    }
}

#[derive(Clone, Debug)]
pub struct SystemEventClock {
    started_at: Instant,
}

impl SystemEventClock {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

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

    fn sleep<'a>(&'a self, duration: Duration) -> EventClockSleep<'a> {
        Box::pin(tokio::time::sleep(duration))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ManualEventClock {
    state: Arc<Mutex<ManualEventClockState>>,
}

#[derive(Default, Debug)]
struct ManualEventClockState {
    now: Duration,
    sleepers: BTreeMap<Duration, Vec<oneshot::Sender<()>>>,
}
