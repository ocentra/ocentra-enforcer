use super::validation::{EventDeliveryDecisionState, EventDeliveryRouteKind};

pub(super) fn decision_state(
    route_kind: EventDeliveryRouteKind,
    requirements_satisfied: bool,
) -> EventDeliveryDecisionState {
    match (route_kind, requirements_satisfied) {
        (EventDeliveryRouteKind::LocalInProcess | EventDeliveryRouteKind::LocalService, _) => {
            EventDeliveryDecisionState::LocalRouteReady
        }
        (EventDeliveryRouteKind::ExternalTransport, true) => {
            EventDeliveryDecisionState::ExternalTransportRouteRequirementsSatisfied
        }
        (EventDeliveryRouteKind::ExternalTransport, false) => {
            EventDeliveryDecisionState::ExternalTransportRouteManualRequired
        }
        (EventDeliveryRouteKind::ExternalRelay, true) => {
            EventDeliveryDecisionState::ExternalRelayRouteRequirementsSatisfied
        }
        (EventDeliveryRouteKind::ExternalRelay, false) => {
            EventDeliveryDecisionState::ExternalRelayRouteManualRequired
        }
    }
}
