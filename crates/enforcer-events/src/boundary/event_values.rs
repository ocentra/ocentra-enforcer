//! Primitive conversion at the event-runtime boundary.

// BOUNDARY-INVARIANT: raw collection lengths and clock durations are branded
// before entering event-domain state and decoded only for runtime adapters.
// boundaryOwnerNote: enforcer-events owns runtime primitive conversion for its
// canonical event-domain values; every raw input passes through a decode
// conversion into a branded domain value before returning to runtime code.
// Negative and zero inputs are covered by the event runtime policy tests.

use enforcer_domain::events_types::{EventCount, EventDuration, JournalSequence, SchemaVersion};

/// Brand a collection length as an event count.
pub(crate) fn event_count(value: usize) -> EventCount {
    std::num::NonZeroUsize::new(value)
        .map(EventCount::try_new)
        .unwrap_or(EventCount::ZERO)
}

/// Decode an event count for collection and loop adapters.
pub(crate) fn event_count_value(value: EventCount) -> usize {
    value.as_nonzero().map_or(0, std::num::NonZeroUsize::get)
}

/// Decode an event duration for clock and timeout adapters.
pub(crate) fn event_duration_value(value: EventDuration) -> std::time::Duration {
    value
        .as_nonzero_nanos()
        .map_or(std::time::Duration::ZERO, |nanos| {
            std::time::Duration::from_nanos(nanos.get())
        })
}

/// Brand an adapter duration for event-domain use.
pub(crate) fn event_duration(value: std::time::Duration) -> EventDuration {
    let nanos = u64::try_from(value.as_nanos()).unwrap_or(u64::MAX);
    std::num::NonZeroU64::new(nanos)
        .map(EventDuration::try_new_nanos)
        .unwrap_or(EventDuration::ZERO)
}

/// Decode a schema version for persistence and presentation adapters.
pub(crate) fn schema_version_value(value: SchemaVersion) -> u16 {
    value.as_nonzero().get()
}

/// Decode a journal sequence for persistence and presentation adapters.
pub(crate) fn journal_sequence_value(value: JournalSequence) -> u64 {
    value.as_nonzero().get()
}
