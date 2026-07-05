use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::bus::reports::dead_letter::DeadLetterReason;
use crate::queue::policy::{EventQueuePolicy, QueueReport};
use crate::{
    EventClockInstant, EventId, EventType, EventingError, IdempotencyKey, StoredEventEnvelope,
};

#[path = "state/dispatch.rs"]
mod dispatch;
#[path = "state/enqueue.rs"]
mod enqueue;
#[path = "state/read.rs"]
mod read;
#[path = "state/report.rs"]
mod report;
#[path = "state/write.rs"]
mod write;

const COMPLETED_IDEMPOTENCY_RETENTION_LIMIT: usize = 4096;

#[derive(Clone)]
pub(crate) struct EventQueue {
    policy: EventQueuePolicy,
    state: Arc<Mutex<EventQueueState>>,
}

impl EventQueue {
    pub(crate) fn new(policy: EventQueuePolicy) -> Self {
        Self {
            policy,
            state: Arc::new(Mutex::new(EventQueueState::default())),
        }
    }

    pub(crate) fn policy(&self) -> &EventQueuePolicy {
        &self.policy
    }
}

#[derive(Default)]
struct EventQueueState {
    queued: VecDeque<QueuedEnvelope>,
    queued_event_ids: BTreeSet<EventId>,
    in_flight_event_ids: BTreeSet<EventId>,
    queued_keys: BTreeSet<IdempotencyKey>,
    in_flight_keys: BTreeSet<IdempotencyKey>,
    completed_keys: BTreeSet<IdempotencyKey>,
    completed_key_order: VecDeque<IdempotencyKey>,
}

#[derive(Clone)]
pub(crate) struct QueuedEnvelope {
    pub(crate) stored: StoredEventEnvelope,
    enqueued_at: EventClockInstant,
}

impl QueuedEnvelope {
    pub(crate) fn is_expired(&self, now: EventClockInstant, ttl: Option<Duration>) -> bool {
        ttl.is_some_and(|ttl| now.duration_since(self.enqueued_at) >= ttl)
    }

    fn matches_event_type(&self, event_type: &EventType) -> bool {
        &self.stored.contract.event_type == event_type
    }
}

pub(crate) enum NoSubscriberQueueDecision {
    Dispatch(QueueReport),
    Queued(QueueReport),
    QueuedWithDeadLetter(
        QueueReport,
        Box<StoredEventEnvelope>,
        DeadLetterReason,
        EventingError,
    ),
    DeadLetter(QueueReport, DeadLetterReason, EventingError),
}

pub(crate) struct EventQueueClearReport {
    pub(crate) queued_event_count: usize,
    pub(crate) queued_idempotency_key_count: usize,
    pub(crate) in_flight_idempotency_key_count: usize,
    pub(crate) completed_idempotency_key_count: usize,
}

fn trim_completed_keys(state: &mut EventQueueState) {
    while state.completed_key_order.len() > COMPLETED_IDEMPOTENCY_RETENTION_LIMIT {
        if let Some(expired) = state.completed_key_order.pop_front() {
            state.completed_keys.remove(&expired);
        }
    }
}
