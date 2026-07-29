#[path = "compatibility_matrix.rs"]
mod compatibility_matrix;
#[path = "contract_registry.rs"]
mod contract_registry;
#[path = "delivery.rs"]
mod delivery;
#[path = "event_marker.rs"]
mod event_marker;
#[path = "family_variants.rs"]
mod family_variants;
#[path = "fixture_parity.rs"]
mod fixture_parity;
#[path = "support.rs"]
mod support;
#[path = "topology_manifest.rs"]
mod topology_manifest;
#[path = "typed_boundary.rs"]
mod typed_boundary;

fn event_count(value: usize) -> enforcer_domain::events_types::EventCount {
    std::num::NonZeroUsize::new(value)
        .map(enforcer_domain::events_types::EventCount::try_new)
        .unwrap_or(enforcer_domain::events_types::EventCount::ZERO)
}

fn event_count_value(value: enforcer_domain::events_types::EventCount) -> usize {
    value.as_nonzero().map_or(0, std::num::NonZeroUsize::get)
}
