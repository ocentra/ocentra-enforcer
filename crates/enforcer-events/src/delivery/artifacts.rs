use enforcer_domain::events_types::{EventDeliveryRequiredArtifact, EventDeliveryRouteKind};

use super::validation::EventDeliveryDecisionInput;

pub(super) fn required_artifacts(
    route_kind: EventDeliveryRouteKind,
) -> Vec<EventDeliveryRequiredArtifact> {
    match route_kind {
        EventDeliveryRouteKind::LocalInProcess | EventDeliveryRouteKind::LocalService => Vec::new(),
        EventDeliveryRouteKind::ExternalTransport => external_transport_required_artifacts(),
        EventDeliveryRouteKind::ExternalRelay => {
            let mut requirements = external_transport_required_artifacts();
            requirements.push(EventDeliveryRequiredArtifact::ExternalRelayIdentity);
            requirements.push(EventDeliveryRequiredArtifact::ExternalRelayPolicy);
            requirements
        }
    }
}

fn external_transport_required_artifacts() -> Vec<EventDeliveryRequiredArtifact> {
    vec![
        EventDeliveryRequiredArtifact::CustodyProof,
        EventDeliveryRequiredArtifact::PublisherAuthProof,
        EventDeliveryRequiredArtifact::SubscriberAuthProof,
        EventDeliveryRequiredArtifact::EncryptionProof,
        EventDeliveryRequiredArtifact::RetentionPolicy,
        EventDeliveryRequiredArtifact::ReplayPlan,
        EventDeliveryRequiredArtifact::DeletionPlan,
        EventDeliveryRequiredArtifact::BackpressurePolicy,
        EventDeliveryRequiredArtifact::OffsetPolicy,
        EventDeliveryRequiredArtifact::DedupePolicy,
        EventDeliveryRequiredArtifact::TransportConfig,
    ]
}

pub(super) fn missing_artifacts(
    input: &EventDeliveryDecisionInput,
    required_artifacts: &[EventDeliveryRequiredArtifact],
) -> Vec<EventDeliveryRequiredArtifact> {
    required_artifacts
        .iter()
        .copied()
        .filter(|artifact| super::validation::artifact_ref(input, *artifact).is_none())
        .collect()
}
