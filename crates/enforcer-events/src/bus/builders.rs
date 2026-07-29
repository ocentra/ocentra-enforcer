use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use tokio::sync::RwLock;

use crate::{
    clock::{SharedEventClock, SystemEventClock},
    execution::HandlerExecutionPolicy,
    journal::{policy::JournalPolicy, SharedEventJournal},
    queue::{policy::EventQueuePolicy, state::EventQueue},
    request::RequestRegistry,
};

use super::{active_dispatch::ActiveDispatchTracker, EventBus, EventBusLifecycleState};

impl EventBus {
    /// Executes the new event-runtime operation.
    pub fn new() -> Self {
        Self {
            registry: super::subscriber::SubscriberRegistry::default(),
            stored_journal: Arc::new(RwLock::new(Vec::new())),
            dead_letters: Arc::new(RwLock::new(Vec::new())),
            aggregate_gates: Arc::new(Mutex::new(BTreeMap::new())),
            handler_policy: HandlerExecutionPolicy::default(),
            queue: EventQueue::new(EventQueuePolicy::default()),
            requests: RequestRegistry::default(),
            journal_policy: JournalPolicy::default(),
            event_journal: None,
            clock: SystemEventClock::shared(),
            shutdown: Arc::new(Mutex::new(EventBusLifecycleState::Active)),
            active_dispatches: ActiveDispatchTracker::default(),
        }
    }

    /// Executes the with clock event-runtime operation.
    pub fn with_clock(clock: SharedEventClock) -> Self {
        Self {
            clock,
            ..Self::new()
        }
    }

    /// Executes the with handler policy event-runtime operation.
    pub fn with_handler_policy(policy: HandlerExecutionPolicy) -> Self {
        Self {
            handler_policy: policy,
            ..Self::new()
        }
    }

    /// Executes the with handler policy and clock event-runtime operation.
    pub fn with_handler_policy_and_clock(
        policy: HandlerExecutionPolicy,
        clock: SharedEventClock,
    ) -> Self {
        Self {
            handler_policy: policy,
            clock,
            ..Self::new()
        }
    }

    /// Executes the with queue policy event-runtime operation.
    pub fn with_queue_policy(policy: EventQueuePolicy) -> Self {
        Self {
            queue: EventQueue::new(policy),
            ..Self::new()
        }
    }

    /// Executes the with queue policy and clock event-runtime operation.
    pub fn with_queue_policy_and_clock(policy: EventQueuePolicy, clock: SharedEventClock) -> Self {
        Self {
            queue: EventQueue::new(policy),
            clock,
            ..Self::new()
        }
    }

    /// Executes the with policies event-runtime operation.
    pub fn with_policies(
        handler_policy: HandlerExecutionPolicy,
        queue_policy: EventQueuePolicy,
    ) -> Self {
        Self {
            handler_policy,
            queue: EventQueue::new(queue_policy),
            ..Self::new()
        }
    }

    /// Executes the with policies and clock event-runtime operation.
    pub fn with_policies_and_clock(
        handler_policy: HandlerExecutionPolicy,
        queue_policy: EventQueuePolicy,
        clock: SharedEventClock,
    ) -> Self {
        Self {
            handler_policy,
            queue: EventQueue::new(queue_policy),
            clock,
            ..Self::new()
        }
    }

    /// Executes the with journal event-runtime operation.
    pub fn with_journal(policy: JournalPolicy, journal: SharedEventJournal) -> Self {
        Self {
            journal_policy: policy,
            event_journal: Some(journal),
            ..Self::new()
        }
    }

    /// Executes the with journal and queue policy event-runtime operation.
    pub fn with_journal_and_queue_policy(
        journal_policy: JournalPolicy,
        journal: SharedEventJournal,
        queue_policy: EventQueuePolicy,
    ) -> Self {
        Self {
            journal_policy,
            event_journal: Some(journal),
            queue: EventQueue::new(queue_policy),
            ..Self::new()
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
