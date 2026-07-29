use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_request_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::InvalidRequestOptions { reason } => {
            write!(formatter, "invalid event request options: {reason}")
        }
        EventingError::RequestOptionsTimeoutMustBePositive => {
            formatter.write_str("invalid event request options: request timeout must be greater than zero")
        }
        EventingError::DuplicateRequest { request_id } => {
            write!(formatter, "duplicate request id: {}", request_id.as_str())
        }
        EventingError::RequestTimedOut { request_id } => {
            write!(
                formatter,
                "event request timed out: {}",
                request_id.as_str()
            )
        }
        EventingError::RequestResponseEncode { request_id, reason } => {
            write!(
                formatter,
                "event request response encode failed for {}: {reason}",
                request_id.as_str()
            )
        }
        EventingError::RequestResponseDecode { request_id, reason } => {
            write!(
                formatter,
                "event request response decode failed for {}: {reason}",
                request_id.as_str()
            )
        }
        EventingError::RequestIncomplete { request_id } => {
            write!(
                formatter,
                "event request completed without a response for {}",
                request_id.as_str()
            )
        }
        EventingError::BusShutdown => formatter.write_str("event bus is shut down"),
        _ => {
            debug_assert!(false, "request error formatter received non-request error");
            formatter.write_str("eventing request error")
        }
    }
}
