mod artifacts;
mod decision;
pub mod validation;

pub fn decide_event_delivery_route(
    input: validation::EventDeliveryDecisionInput,
) -> Result<validation::EventDeliveryDecisionProof, validation::EventDeliveryDecisionError> {
    validation::reject_claims(&input)?;
    validation::validate_subscriber_filter(&input)?;
    validation::validate_backpressure(&input.backpressure_policy)?;

    let required_artifacts = artifacts::required_artifacts(input.route_kind);
    let missing_artifacts = artifacts::missing_artifacts(&input, &required_artifacts);
    let decision_state = decision::decision_state(input.route_kind, missing_artifacts.is_empty());
    let local_delivery_ready = matches!(
        decision_state,
        validation::EventDeliveryDecisionState::LocalRouteReady
            | validation::EventDeliveryDecisionState::ExternalTransportRouteRequirementsSatisfied
            | validation::EventDeliveryDecisionState::ExternalRelayRouteRequirementsSatisfied
    );

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
        local_delivery_ready,
        external_transport_delivery_implemented: false,
        external_relay_delivery_implemented: false,
        subscriber_filtering_enabled: true,
        decision_authority: false,
        side_effect_authority: false,
    })
}
