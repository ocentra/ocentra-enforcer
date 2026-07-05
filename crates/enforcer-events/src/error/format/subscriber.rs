use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_subscriber_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::DuplicateSubscriber { subscriber_id } => {
            write!(
                formatter,
                "duplicate subscriber: {}",
                subscriber_id.as_str()
            )
        }
        EventingError::HandlerPanicked { subscriber_id } => {
            write!(
                formatter,
                "event handler panicked: {}",
                subscriber_id.as_str()
            )
        }
        EventingError::HandlerTimedOut { subscriber_id } => {
            write!(
                formatter,
                "event handler timed out: {}",
                subscriber_id.as_str()
            )
        }
        _ => {
            debug_assert!(false, "subscriber formatter received non-subscriber error");
            formatter.write_str("eventing subscriber error")
        }
    }
}
