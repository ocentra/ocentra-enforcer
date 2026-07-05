use std::{num::NonZeroUsize, time::Duration};

use crate::EventingError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerExecutionPolicy {
    timeout: Option<Duration>,
    max_attempts: NonZeroUsize,
}

impl HandlerExecutionPolicy {
    pub fn new(timeout: Option<Duration>, max_attempts: usize) -> Result<Self, EventingError> {
        if matches!(timeout, Some(duration) if duration.is_zero()) {
            return Err(EventingError::InvalidHandlerPolicy {
                reason: "timeout must be greater than zero".to_string(),
            });
        }
        let max_attempts =
            NonZeroUsize::new(max_attempts).ok_or_else(|| EventingError::InvalidHandlerPolicy {
                reason: "max_attempts must be greater than zero".to_string(),
            })?;
        Ok(Self {
            timeout,
            max_attempts,
        })
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn max_attempts(&self) -> usize {
        self.max_attempts.get()
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
