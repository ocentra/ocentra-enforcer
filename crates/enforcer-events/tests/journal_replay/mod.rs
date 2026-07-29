#[path = "bus_policy.rs"]
mod bus_policy;
#[path = "file.rs"]
mod file;
#[path = "fixtures.rs"]
mod fixtures;
#[path = "replay.rs"]
mod replay;
#[path = "support.rs"]
mod support;

fn event_count(value: usize) -> enforcer_domain::events_types::EventCount {
    std::num::NonZeroUsize::new(value)
        .map(enforcer_domain::events_types::EventCount::try_new)
        .unwrap_or(enforcer_domain::events_types::EventCount::ZERO)
}

fn event_count_value(value: enforcer_domain::events_types::EventCount) -> usize {
    value.as_nonzero().map_or(0, std::num::NonZeroUsize::get)
}
