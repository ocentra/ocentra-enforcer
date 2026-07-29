use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_config_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::EmptyValue { field } => {
            write!(formatter, "empty eventing value: {}", field.as_str())
        }
        EventingError::InvalidValue { field, value } => {
            write!(
                formatter,
                "invalid eventing value for {}: {value}",
                field.as_str()
            )
        }
        EventingError::InvalidVersion => {
            formatter.write_str("event schema version must be nonzero")
        }
        EventingError::PayloadEncode { reason } => {
            write!(formatter, "payload encode failed: {reason}")
        }
        EventingError::InvalidHandlerPolicy { reason } => {
            write!(formatter, "invalid event handler policy: {reason}")
        }
        EventingError::HandlerPolicyTimeoutMustBePositive => {
            formatter.write_str("invalid event handler policy: timeout must be greater than zero")
        }
        EventingError::HandlerPolicyMaxAttemptsMustBePositive => {
            formatter.write_str("invalid event handler policy: max_attempts must be greater than zero")
        }
        EventingError::HandlerPolicyProducedNoAttempt => {
            formatter.write_str("invalid event handler policy: handler execution policy produced no attempt")
        }
        EventingError::InvalidQueuePolicy { reason } => {
            write!(formatter, "invalid event queue policy: {reason}")
        }
        EventingError::QueuePolicyCapacityMustBePositive => formatter.write_str("invalid event queue policy: queue capacity must be greater than zero"),
        EventingError::QueuePolicyQueuedRequiresCapacity => formatter.write_str("invalid event queue policy: queued no-subscriber policy requires bounded capacity"),
        EventingError::QueuePolicyTtlMustBePositive => formatter.write_str("invalid event queue policy: queue ttl must be greater than zero"),
        EventingError::QueuePolicyCapacityNotConfigured => formatter.write_str("invalid event queue policy: queue capacity is not configured"),
        EventingError::QueuePolicyDropOldestRequiresQueuedEvent => formatter.write_str("invalid event queue policy: drop-oldest overflow requires a queued event"),
        _ => {
            debug_assert!(false, "core config formatter received non-config error");
            formatter.write_str("eventing config error")
        }
    }
}
