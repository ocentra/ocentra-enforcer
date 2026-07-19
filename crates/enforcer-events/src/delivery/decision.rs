use enforcer_domain::events_types::{
    EventDeliveryDecisionState, EventDeliveryRequirementsState, EventDeliveryRouteKind,
};

pub(super) fn decision_state(
    route_kind: EventDeliveryRouteKind,
    requirements_state: EventDeliveryRequirementsState,
) -> EventDeliveryDecisionState {
    match (route_kind, requirements_state) {
        (EventDeliveryRouteKind::LocalInProcess | EventDeliveryRouteKind::LocalService, _) => {
            EventDeliveryDecisionState::LocalRouteReady
        }
        (EventDeliveryRouteKind::ExternalTransport, EventDeliveryRequirementsState::Satisfied) => {
            EventDeliveryDecisionState::ExternalTransportRouteRequirementsSatisfied
        }
        (EventDeliveryRouteKind::ExternalTransport, EventDeliveryRequirementsState::Missing) => {
            EventDeliveryDecisionState::ExternalTransportRouteManualRequired
        }
        (EventDeliveryRouteKind::ExternalRelay, EventDeliveryRequirementsState::Satisfied) => {
            EventDeliveryDecisionState::ExternalRelayRouteRequirementsSatisfied
        }
        (EventDeliveryRouteKind::ExternalRelay, EventDeliveryRequirementsState::Missing) => {
            EventDeliveryDecisionState::ExternalRelayRouteManualRequired
        }
    }
}
