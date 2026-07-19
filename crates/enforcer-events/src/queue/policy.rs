use enforcer_domain::events_types::EventErrorReason;
use enforcer_domain::events_types::{
    EventCount, EventDuration, NoSubscriberQueuePolicy, QueueDisposition, QueueIdempotencyState,
    QueueOverflowPolicy,
};
use std::num::NonZeroUsize;

use crate::error::EventingError;

/// Event-runtime data for event queue policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventQueuePolicy {
    capacity: Option<NonZeroUsize>,
    no_subscriber: NoSubscriberQueuePolicy,
    overflow: QueueOverflowPolicy,
    ttl: Option<EventDuration>,
    idempotency_registry: QueueIdempotencyState,
}

impl EventQueuePolicy {
    /// Executes the no subscriber queue event-runtime operation.
    pub fn no_subscriber_queue(capacity: EventCount) -> Result<Self, EventingError> {
        let capacity = capacity
            .as_nonzero()
            .ok_or_else(|| EventingError::InvalidQueuePolicy {
                reason: EventErrorReason::from_diagnostic(
                    "queue capacity must be greater than zero",
                ),
            })?;
        Ok(Self {
            capacity: Some(capacity),
            no_subscriber: NoSubscriberQueuePolicy::Queue,
            overflow: QueueOverflowPolicy::DropOldestAndDeadLetter,
            ttl: None,
            idempotency_registry: QueueIdempotencyState::Disabled,
        })
    }

    /// Executes the with no subscriber policy event-runtime operation.
    pub fn with_no_subscriber_policy(
        mut self,
        policy: NoSubscriberQueuePolicy,
    ) -> Result<Self, EventingError> {
        if matches!(policy, NoSubscriberQueuePolicy::Queue) && self.capacity.is_none() {
            return Err(EventingError::InvalidQueuePolicy {
                reason: EventErrorReason::from_diagnostic(
                    "queued no-subscriber policy requires bounded capacity",
                ),
            });
        }
        self.no_subscriber = policy;
        Ok(self)
    }

    /// Executes the with overflow policy event-runtime operation.
    pub fn with_overflow_policy(mut self, policy: QueueOverflowPolicy) -> Self {
        self.overflow = policy;
        self
    }

    /// Executes the with ttl event-runtime operation.
    pub fn with_ttl(mut self, ttl: EventDuration) -> Result<Self, EventingError> {
        if ttl.value().is_zero() {
            return Err(EventingError::InvalidQueuePolicy {
                reason: EventErrorReason::from_diagnostic("queue ttl must be greater than zero"),
            });
        }
        self.ttl = Some(ttl);
        Ok(self)
    }

    /// Executes the with idempotency registry event-runtime operation.
    pub fn with_idempotency_registry(mut self) -> Self {
        self.idempotency_registry = QueueIdempotencyState::Enabled;
        self
    }

    /// Executes the capacity event-runtime operation.
    pub fn capacity(&self) -> Option<EventCount> {
        self.capacity
            .map(|capacity| crate::boundary::event_values::event_count(capacity.get()))
    }

    /// Executes the no subscriber event-runtime operation.
    pub fn no_subscriber(&self) -> NoSubscriberQueuePolicy {
        self.no_subscriber
    }

    /// Executes the overflow event-runtime operation.
    pub fn overflow(&self) -> QueueOverflowPolicy {
        self.overflow
    }

    /// Executes the ttl event-runtime operation.
    pub fn ttl(&self) -> Option<EventDuration> {
        self.ttl
    }

    /// Executes the idempotency registry event-runtime operation.
    pub fn idempotency_registry(&self) -> QueueIdempotencyState {
        self.idempotency_registry
    }
}

impl Default for EventQueuePolicy {
    fn default() -> Self {
        Self {
            capacity: None,
            no_subscriber: NoSubscriberQueuePolicy::DispatchWithoutSubscribers,
            overflow: QueueOverflowPolicy::RejectPublish,
            ttl: None,
            idempotency_registry: QueueIdempotencyState::Disabled,
        }
    }
}

/// Event-runtime data for queue report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueReport {
    pub disposition: QueueDisposition,
    pub queued_count: EventCount,
    pub capacity: Option<EventCount>,
}
