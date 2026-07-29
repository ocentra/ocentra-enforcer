mod artifacts;
mod decision;
pub mod validation;

use enforcer_domain::events_types::{EventDeliveryCapabilityState, EventDeliveryDecisionState};

/// Executes the decide event delivery route event-runtime operation.
pub fn decide_event_delivery_route(
    input: validation::EventDeliveryDecisionInput,
) -> Result<validation::EventDeliveryDecisionProof, validation::EventDeliveryDecisionError> {
    validation::reject_claims(&input)?;
    validation::validate_subscriber_filter(&input)?;
    validation::validate_backpressure(&input.backpressure_policy)?;

    let required_artifacts = artifacts::required_artifacts(input.route_kind);
    let missing_artifacts = artifacts::missing_artifacts(&input, &required_artifacts);
    let requirements_state = if missing_artifacts.is_empty() {
        enforcer_domain::events_types::EventDeliveryRequirementsState::Satisfied
    } else {
        enforcer_domain::events_types::EventDeliveryRequirementsState::Missing
    };
    let decision_state = decision::decision_state(input.route_kind, requirements_state);
    let local_delivery_capability = if matches!(
        decision_state,
        EventDeliveryDecisionState::LocalRouteReady
            | EventDeliveryDecisionState::ExternalTransportRouteRequirementsSatisfied
            | EventDeliveryDecisionState::ExternalRelayRouteRequirementsSatisfied
    ) {
        EventDeliveryCapabilityState::Available
    } else {
        EventDeliveryCapabilityState::Unavailable
    };

    Ok(validation::EventDeliveryDecisionProof {
        route_kind: input.route_kind,
        decision_state,
        event_namespace: input.event_namespace,
        publisher_component: input.publisher_component,
        subscriber_filter: input.subscriber_filter,
        required_artifacts,
        missing_artifacts,
        backpressure_policy: input.backpressure_policy,
        retention_policy_ref: input.retention_policy_ref,
        local_delivery_capability,
        external_transport_delivery_capability: EventDeliveryCapabilityState::Unavailable,
        external_relay_delivery_capability: EventDeliveryCapabilityState::Unavailable,
        subscriber_filtering_capability: EventDeliveryCapabilityState::Available,
        decision_authority_capability: EventDeliveryCapabilityState::Unavailable,
        side_effect_authority_capability: EventDeliveryCapabilityState::Unavailable,
    })
}
