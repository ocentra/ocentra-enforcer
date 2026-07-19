use enforcer_domain::events_types::{
    EventCount, EventDeliveryCapabilityState, EventDeliveryClaimState, EventDeliveryDecisionState,
    EventDeliveryIdempotencyRequirement, EventDeliveryOverflowPolicy,
    EventDeliveryRequiredArtifact, EventDeliveryRouteKind, EventDuration, EventNamespace,
    EventType, SourceComponent, SubscriberId, TargetHandler,
};

/// Event-runtime data for event delivery backpressure policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDeliveryBackpressurePolicy {
    pub bounded_queue_capacity: EventCount,
    pub ttl: EventDuration,
    pub overflow_policy: EventDeliveryOverflowPolicy,
    pub idempotency_requirement: EventDeliveryIdempotencyRequirement,
}

/// Event-runtime data for event delivery subscriber filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDeliverySubscriberFilter {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
    pub event_namespace: EventNamespace,
    pub accepted_event_types: Vec<EventType>,
}

/// Event-runtime data for event delivery decision input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDeliveryDecisionInput {
    pub route_kind: EventDeliveryRouteKind,
    pub event_namespace: EventNamespace,
    pub publisher_component: SourceComponent,
    pub subscriber_filter: EventDeliverySubscriberFilter,
    pub backpressure_policy: EventDeliveryBackpressurePolicy,
    pub custody_proof_ref: Option<SourceComponent>,
    pub publisher_auth_ref: Option<SourceComponent>,
    pub subscriber_auth_ref: Option<SourceComponent>,
    pub encryption_ref: Option<SourceComponent>,
    pub retention_policy_ref: Option<SourceComponent>,
    pub replay_plan_ref: Option<SourceComponent>,
    pub deletion_plan_ref: Option<SourceComponent>,
    pub offset_policy_ref: Option<SourceComponent>,
    pub dedupe_policy_ref: Option<SourceComponent>,
    pub transport_config_ref: Option<SourceComponent>,
    pub relay_identity_ref: Option<SourceComponent>,
    pub relay_policy_ref: Option<SourceComponent>,
    pub external_transport_delivery_claim: EventDeliveryClaimState,
    pub external_relay_delivery_claim: EventDeliveryClaimState,
    pub decision_authority_claim: EventDeliveryClaimState,
    pub side_effect_authority_claim: EventDeliveryClaimState,
}

/// Event-runtime data for event delivery decision proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDeliveryDecisionProof {
    pub route_kind: EventDeliveryRouteKind,
    pub decision_state: EventDeliveryDecisionState,
    pub event_namespace: EventNamespace,
    pub publisher_component: SourceComponent,
    pub subscriber_filter: EventDeliverySubscriberFilter,
    pub required_artifacts: Vec<EventDeliveryRequiredArtifact>,
    pub missing_artifacts: Vec<EventDeliveryRequiredArtifact>,
    pub backpressure_policy: EventDeliveryBackpressurePolicy,
    pub retention_policy_ref: Option<SourceComponent>,
    pub local_delivery_capability: EventDeliveryCapabilityState,
    pub external_transport_delivery_capability: EventDeliveryCapabilityState,
    pub external_relay_delivery_capability: EventDeliveryCapabilityState,
    pub subscriber_filtering_capability: EventDeliveryCapabilityState,
    pub decision_authority_capability: EventDeliveryCapabilityState,
    pub side_effect_authority_capability: EventDeliveryCapabilityState,
}

/// Event-runtime variants for event delivery decision error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EventDeliveryDecisionError {
    #[error("event delivery decision rejected: empty subscriber accepted events")]
    EmptySubscriberAcceptedEvents,
    #[error("event delivery decision rejected: subscriber filter outside namespace")]
    SubscriberFilterOutsideNamespace,
    #[error("event delivery decision rejected: invalid backpressure capacity")]
    InvalidBackpressureCapacity,
    #[error("event delivery decision rejected: invalid backpressure ttl")]
    InvalidBackpressureTtl,
    #[error("event delivery decision rejected: live external transport delivery claim rejected")]
    LiveExternalTransportDeliveryClaimRejected,
    #[error("event delivery decision rejected: live external relay delivery claim rejected")]
    LiveExternalRelayDeliveryClaimRejected,
    #[error("event delivery decision rejected: decision authority claim rejected")]
    DecisionAuthorityClaimRejected,
    #[error("event delivery decision rejected: side-effect authority claim rejected")]
    SideEffectAuthorityClaimRejected,
}

pub(super) fn reject_claims(
    input: &EventDeliveryDecisionInput,
) -> Result<(), EventDeliveryDecisionError> {
    if input.external_transport_delivery_claim == EventDeliveryClaimState::Claimed {
        return Err(EventDeliveryDecisionError::LiveExternalTransportDeliveryClaimRejected);
    }
    if input.external_relay_delivery_claim == EventDeliveryClaimState::Claimed {
        return Err(EventDeliveryDecisionError::LiveExternalRelayDeliveryClaimRejected);
    }
    if input.decision_authority_claim == EventDeliveryClaimState::Claimed {
        return Err(EventDeliveryDecisionError::DecisionAuthorityClaimRejected);
    }
    if input.side_effect_authority_claim == EventDeliveryClaimState::Claimed {
        return Err(EventDeliveryDecisionError::SideEffectAuthorityClaimRejected);
    }
    Ok(())
}

pub(super) fn validate_subscriber_filter(
    input: &EventDeliveryDecisionInput,
) -> Result<(), EventDeliveryDecisionError> {
    if input.subscriber_filter.accepted_event_types.is_empty() {
        return Err(EventDeliveryDecisionError::EmptySubscriberAcceptedEvents);
    }
    if input.subscriber_filter.event_namespace != input.event_namespace {
        return Err(EventDeliveryDecisionError::SubscriberFilterOutsideNamespace);
    }
    if input
        .subscriber_filter
        .accepted_event_types
        .iter()
        .any(|event_type| {
            enforcer_domain::events_types::EventNamespace::from_event_type(event_type)
                .map_or(true, |namespace| namespace != input.event_namespace)
        })
    {
        return Err(EventDeliveryDecisionError::SubscriberFilterOutsideNamespace);
    }
    Ok(())
}

pub(super) fn validate_backpressure(
    policy: &EventDeliveryBackpressurePolicy,
) -> Result<(), EventDeliveryDecisionError> {
    if policy.bounded_queue_capacity == EventCount::ZERO {
        return Err(EventDeliveryDecisionError::InvalidBackpressureCapacity);
    }
    if policy.ttl.value().is_zero() {
        return Err(EventDeliveryDecisionError::InvalidBackpressureTtl);
    }
    Ok(())
}

/// Executes the artifact ref event-runtime operation.
pub fn artifact_ref(
    input: &EventDeliveryDecisionInput,
    artifact: EventDeliveryRequiredArtifact,
) -> Option<&SourceComponent> {
    match artifact {
        EventDeliveryRequiredArtifact::CustodyProof => custody_proof_ref(input),
        EventDeliveryRequiredArtifact::PublisherAuthProof => publisher_auth_ref(input),
        EventDeliveryRequiredArtifact::SubscriberAuthProof => subscriber_auth_ref(input),
        EventDeliveryRequiredArtifact::EncryptionProof => encryption_ref(input),
        EventDeliveryRequiredArtifact::RetentionPolicy => retention_policy_ref(input),
        EventDeliveryRequiredArtifact::ReplayPlan => replay_plan_ref(input),
        EventDeliveryRequiredArtifact::DeletionPlan => deletion_plan_ref(input),
        EventDeliveryRequiredArtifact::BackpressurePolicy => backpressure_policy_ref(input),
        EventDeliveryRequiredArtifact::OffsetPolicy => offset_policy_ref(input),
        EventDeliveryRequiredArtifact::DedupePolicy => dedupe_policy_ref(input),
        EventDeliveryRequiredArtifact::TransportConfig => transport_config_ref(input),
        EventDeliveryRequiredArtifact::ExternalRelayIdentity => relay_identity_ref(input),
        EventDeliveryRequiredArtifact::ExternalRelayPolicy => relay_policy_ref(input),
    }
}

fn custody_proof_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.custody_proof_ref.as_ref()
}

fn publisher_auth_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.publisher_auth_ref.as_ref()
}

fn subscriber_auth_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.subscriber_auth_ref.as_ref()
}

fn encryption_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.encryption_ref.as_ref()
}

fn retention_policy_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.retention_policy_ref.as_ref()
}

fn replay_plan_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.replay_plan_ref.as_ref()
}

fn deletion_plan_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.deletion_plan_ref.as_ref()
}

fn backpressure_policy_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    Some(&input.publisher_component)
}

fn offset_policy_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.offset_policy_ref.as_ref()
}

fn dedupe_policy_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.dedupe_policy_ref.as_ref()
}

fn transport_config_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.transport_config_ref.as_ref()
}

fn relay_identity_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.relay_identity_ref.as_ref()
}

fn relay_policy_ref(input: &EventDeliveryDecisionInput) -> Option<&SourceComponent> {
    input.relay_policy_ref.as_ref()
}
