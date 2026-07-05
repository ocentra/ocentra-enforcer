use std::{num::NonZeroUsize, time::Duration};

use crate::EventingError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventQueuePolicy {
    capacity: Option<NonZeroUsize>,
    no_subscriber: NoSubscriberQueuePolicy,
    overflow: QueueOverflowPolicy,
    ttl: Option<Duration>,
    idempotency_registry: bool,
}

impl EventQueuePolicy {
    pub fn no_subscriber_queue(capacity: usize) -> Result<Self, EventingError> {
        let capacity =
            NonZeroUsize::new(capacity).ok_or_else(|| EventingError::InvalidQueuePolicy {
                reason: String::from("queue capacity must be greater than zero"),
            })?;
        Ok(Self {
            capacity: Some(capacity),
            no_subscriber: NoSubscriberQueuePolicy::Queue,
            overflow: QueueOverflowPolicy::DropOldestAndDeadLetter,
            ttl: None,
            idempotency_registry: false,
        })
    }

    pub fn with_no_subscriber_policy(
        mut self,
        policy: NoSubscriberQueuePolicy,
    ) -> Result<Self, EventingError> {
        if matches!(policy, NoSubscriberQueuePolicy::Queue) && self.capacity.is_none() {
            return Err(EventingError::InvalidQueuePolicy {
                reason: String::from("queued no-subscriber policy requires bounded capacity"),
            });
        }
        self.no_subscriber = policy;
        Ok(self)
    }

    pub fn with_overflow_policy(mut self, policy: QueueOverflowPolicy) -> Self {
        self.overflow = policy;
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Result<Self, EventingError> {
        if ttl.is_zero() {
            return Err(EventingError::InvalidQueuePolicy {
                reason: String::from("queue ttl must be greater than zero"),
            });
        }
        self.ttl = Some(ttl);
        Ok(self)
    }

    pub fn with_idempotency_registry(mut self) -> Self {
        self.idempotency_registry = true;
        self
    }

    pub fn capacity(&self) -> Option<usize> {
        self.capacity.map(NonZeroUsize::get)
    }

    pub fn no_subscriber(&self) -> NoSubscriberQueuePolicy {
        self.no_subscriber
    }

    pub fn overflow(&self) -> QueueOverflowPolicy {
        self.overflow
    }

    pub fn ttl(&self) -> Option<Duration> {
        self.ttl
    }

    pub fn idempotency_registry_enabled(&self) -> bool {
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
            idempotency_registry: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoSubscriberQueuePolicy {
    DispatchWithoutSubscribers,
    Queue,
    DeadLetter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueOverflowPolicy {
    RejectPublish,
    DeadLetterRejected,
    DropOldestAndDeadLetter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueDisposition {
    Dispatched,
    QueuedNoSubscriber,
    DeadLetteredNoSubscriber,
    DeadLetteredQueueOverflow,
    DeadLetteredDeadlineExpired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueReport {
    pub disposition: QueueDisposition,
    pub queued_count: usize,
    pub capacity: Option<usize>,
}
