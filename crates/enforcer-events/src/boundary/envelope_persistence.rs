//! Durable JSON persistence and wire conversion boundary for event envelopes.
//!
//! BOUNDARY-INVARIANT: each untrusted transport field passes through a decode
//! conversion into a canonical event-domain value; persistence never decides
//! event routing or delivery policy.
//! BOUNDARY-TEST: envelope round-trip and malformed wire payload tests cover
//! this conversion boundary.
//! BOUNDARY-OWNER: enforcer-events.
//! boundaryOwnerNote: enforcer-events owns the durable envelope wire surface
//! and its immediate conversion into canonical event-domain values.
//! NEGATIVE-TEST: `tests/unit/envelope.rs` rejects invalid zero schema versions,
//! malformed identifiers, and contract-mismatched stored envelopes.

pub(crate) fn parse_event_value<T>(
    value: String,
    field: &'static str,
) -> Result<T, crate::error::EventingError>
where
    T: TryFrom<String>,
{
    T::try_from(value.clone()).map_err(|_decode_error| {
        crate::error::EventingError::invalid_value(
            enforcer_domain::events_types::EventErrorField::from_diagnostic(field),
            enforcer_domain::events_types::EventErrorReason::from_diagnostic(value),
        )
    })
}
