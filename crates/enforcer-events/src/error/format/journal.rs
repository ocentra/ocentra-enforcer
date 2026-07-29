use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_journal_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::JournalIo { path, reason } => {
            write!(formatter, "event journal io failed for {path}: {reason}")
        }
        EventingError::JournalEncode { reason } => {
            write!(formatter, "event journal encode failed: {reason}")
        }
        EventingError::JournalDecode { reason } => {
            write!(formatter, "event journal decode failed: {reason}")
        }
        EventingError::JournalCorruptLine { line, reason } => {
            write!(
                formatter,
                "event journal corrupt line {}: {reason}",
                crate::boundary::event_values::event_count_value(*line)
            )
        }
        EventingError::JournalAppendGateClosed => {
            formatter.write_str("event journal append gate is closed")
        }
        EventingError::ReplayActionNotAllowed { event_type } => {
            write!(
                formatter,
                "event replay action handlers are not allowed for {}",
                event_type.as_str()
            )
        }
        _ => {
            debug_assert!(false, "journal error formatter received non-journal error");
            formatter.write_str("eventing journal error")
        }
    }
}
