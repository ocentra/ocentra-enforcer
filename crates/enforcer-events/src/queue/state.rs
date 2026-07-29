use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use crate::queue::policy::{EventQueuePolicy, QueueReport};
use crate::{clock::EventClockInstant, error::EventingError};
use enforcer_domain::events_types::{
    DeadLetterReason, EventCount, EventDuration, EventId, EventMatchState, EventType,
    IdempotencyKey, QueueExpirationState,
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
    pub(crate) enqueued_at: EventClockInstant,
}

impl QueuedEnvelope {
    pub(crate) fn expiration(
        &self,
        now: EventClockInstant,
        ttl: Option<EventDuration>,
    ) -> QueueExpirationState {
        if ttl.is_some_and(|ttl| now.duration_since(self.enqueued_at) >= ttl) {
            QueueExpirationState::Expired
        } else {
            QueueExpirationState::Current
        }
    }

    fn event_type_match(&self, event_type: &EventType) -> EventMatchState {
        if &self.stored.contract.event_type == event_type {
            EventMatchState::Matches
        } else {
            EventMatchState::DoesNotMatch
        }
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
    pub(crate) queued_event_count: EventCount,
    pub(crate) queued_idempotency_key_count: EventCount,
    pub(crate) in_flight_idempotency_key_count: EventCount,
    pub(crate) completed_idempotency_key_count: EventCount,
}

fn trim_completed_keys(state: &mut EventQueueState) {
    while state.completed_key_order.len() > COMPLETED_IDEMPOTENCY_RETENTION_LIMIT {
        if let Some(expired) = state.completed_key_order.pop_front() {
            state.completed_keys.remove(&expired);
        }
    }
}
