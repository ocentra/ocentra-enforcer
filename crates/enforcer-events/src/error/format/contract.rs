use std::fmt;

use super::super::EventingError;

pub(super) fn fmt_contract_error(
    error: &EventingError,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match error {
        EventingError::PayloadDecode { event_type, reason } => {
            write!(
                formatter,
                "payload decode failed for {}: {reason}",
                event_type.as_str()
            )
        }
        EventingError::ContractMismatch {
            expected,
            received,
            expected_schema_version,
            received_schema_version,
        } => write!(
            formatter,
            "event contract mismatch: expected {}@{}, received {}@{}",
            expected.as_str(),
            expected_schema_version.value(),
            received.as_str(),
            received_schema_version.value()
        ),
        EventingError::DuplicateEventContract { event_type } => {
            write!(
                formatter,
                "duplicate event contract: {}",
                event_type.as_str()
            )
        }
        _ => {
            debug_assert!(false, "contract formatter received non-contract error");
            formatter.write_str("eventing contract error")
        }
    }
}
