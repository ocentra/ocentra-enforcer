use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_queue_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::NoSubscriber { event_type } => {
            write!(
                formatter,
                "no subscriber for event type: {}",
                event_type.as_str()
            )
        }
        EventingError::QueueCapacityExceeded {
            event_type,
            capacity,
        } => write!(
            formatter,
            "event queue capacity exceeded for {}: {}",
            event_type.as_str(),
            crate::boundary::event_values::event_count_value(*capacity)
        ),
        EventingError::EventDeadlineExpired { event_type } => {
            write!(
                formatter,
                "event deadline expired for {}",
                event_type.as_str()
            )
        }
        EventingError::DuplicateEventId { event_id } => {
            write!(formatter, "duplicate event id: {}", event_id.as_str())
        }
        EventingError::DuplicateInFlight { idempotency_key } => {
            write!(
                formatter,
                "duplicate in-flight event: {}",
                idempotency_key.as_str()
            )
        }
        EventingError::DuplicateIdempotencyKey { idempotency_key } => {
            write!(
                formatter,
                "duplicate idempotency key: {}",
                idempotency_key.as_str()
            )
        }
        _ => {
            debug_assert!(false, "queue formatter received non-queue error");
            formatter.write_str("eventing queue error")
        }
    }
}
