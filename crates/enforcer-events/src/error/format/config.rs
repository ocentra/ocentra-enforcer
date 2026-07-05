use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_config_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::EmptyValue { field } => write!(formatter, "empty eventing value: {field}"),
        EventingError::InvalidValue { field, value } => {
            write!(formatter, "invalid eventing value for {field}: {value}")
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
        EventingError::InvalidQueuePolicy { reason } => {
            write!(formatter, "invalid event queue policy: {reason}")
        }
        _ => {
            debug_assert!(false, "core config formatter received non-config error");
            formatter.write_str("eventing config error")
        }
    }
}
