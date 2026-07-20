use enforcer_domain::events_types::{EventCount, EventDuration};
use std::num::NonZeroUsize;

use crate::error::EventingError;

/// Event-runtime data for handler execution policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerExecutionPolicy {
    timeout: Option<EventDuration>,
    max_attempts: NonZeroUsize,
}

impl HandlerExecutionPolicy {
    /// Executes the new event-runtime operation.
    pub fn new(
        timeout: Option<EventDuration>,
        max_attempts: EventCount,
    ) -> Result<Self, EventingError> {
        if matches!(timeout, Some(duration) if duration.value().is_zero()) {
            return Err(EventingError::HandlerPolicyTimeoutMustBePositive);
        }
        let max_attempts = max_attempts
            .as_nonzero()
            .ok_or(EventingError::HandlerPolicyMaxAttemptsMustBePositive)?;
        Ok(Self {
            timeout,
            max_attempts,
        })
    }

    /// Executes the timeout event-runtime operation.
    pub fn timeout(&self) -> Option<EventDuration> {
        self.timeout
    }

    /// Executes the max attempts event-runtime operation.
    pub fn max_attempts(&self) -> EventCount {
        crate::boundary::event_values::event_count(self.max_attempts.get())
    }
}

impl Default for HandlerExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout: None,
            max_attempts: NonZeroUsize::MIN,
        }
    }
}
