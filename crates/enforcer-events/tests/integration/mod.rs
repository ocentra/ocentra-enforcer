#[path = "event_flow_contract.rs"]
mod event_flow_contract;

fn event_count_value(value: enforcer_domain::events_types::EventCount) -> usize {
    value.as_nonzero().map_or(0, std::num::NonZeroUsize::get)
}
