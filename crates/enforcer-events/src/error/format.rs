use std::fmt;

use super::EventingError;

mod config;
mod contract_formatter;
mod journal;
mod queue;
mod request;
mod subscriber;

pub(super) fn fmt_eventing_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::EmptyValue { .. }
        | EventingError::InvalidValue { .. }
        | EventingError::InvalidVersion
        | EventingError::PayloadEncode { .. }
        | EventingError::InvalidHandlerPolicy { .. }
        | EventingError::HandlerPolicyTimeoutMustBePositive
        | EventingError::HandlerPolicyMaxAttemptsMustBePositive
        | EventingError::HandlerPolicyProducedNoAttempt
        | EventingError::InvalidQueuePolicy { .. }
        | EventingError::QueuePolicyCapacityMustBePositive
        | EventingError::QueuePolicyQueuedRequiresCapacity
        | EventingError::QueuePolicyTtlMustBePositive
        | EventingError::QueuePolicyCapacityNotConfigured
        | EventingError::QueuePolicyDropOldestRequiresQueuedEvent => config::fmt_config_error(error, formatter),
        EventingError::PayloadDecode { .. }
        | EventingError::ContractMismatch { .. }
        | EventingError::DuplicateEventContract { .. } => {
            contract_formatter::fmt_contract_error(error, formatter)
        }
        EventingError::DuplicateSubscriber { .. }
        | EventingError::HandlerPanicked { .. }
        | EventingError::HandlerTimedOut { .. } => {
            subscriber::fmt_subscriber_error(error, formatter)
        }
        EventingError::NoSubscriber { .. }
        | EventingError::QueueCapacityExceeded { .. }
        | EventingError::EventDeadlineExpired { .. }
        | EventingError::DuplicateEventId { .. }
        | EventingError::DuplicateInFlight { .. }
        | EventingError::DuplicateIdempotencyKey { .. } => queue::fmt_queue_error(error, formatter),
        EventingError::InvalidRequestOptions { .. }
        | EventingError::RequestOptionsTimeoutMustBePositive
        | EventingError::DuplicateRequest { .. }
        | EventingError::RequestTimedOut { .. }
        | EventingError::RequestResponseEncode { .. }
        | EventingError::RequestResponseDecode { .. }
        | EventingError::RequestIncomplete { .. }
        | EventingError::BusShutdown => request::fmt_request_error(error, formatter),
        EventingError::JournalIo { .. }
        | EventingError::JournalEncode { .. }
        | EventingError::JournalDecode { .. }
        | EventingError::JournalCorruptLine { .. }
        | EventingError::JournalAppendGateClosed
        | EventingError::ReplayActionNotAllowed { .. } => {
            journal::fmt_journal_error(error, formatter)
        }
        EventingError::RegistrarDisposed => formatter.write_str("event registrar is disposed"),
    }
}
