use serde::{Deserialize, Serialize};

use crate::{EventNamespace, SourceComponent, SubscriberId, TargetHandler};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventDeliveryRouteKind {
    LocalInProcess,
    LocalService,
    ExternalTransport,
    ExternalRelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventDeliveryDecisionState {
    LocalRouteReady,
    ExternalTransportRouteManualRequired,
    ExternalRelayRouteManualRequired,
    ExternalTransportRouteRequirementsSatisfied,
    ExternalRelayRouteRequirementsSatisfied,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventDeliveryRequiredArtifact {
    CustodyProof = 0,
    PublisherAuthProof = 1,
    SubscriberAuthProof = 2,
    EncryptionProof = 3,
    RetentionPolicy = 4,
    ReplayPlan = 5,
    DeletionPlan = 6,
    BackpressurePolicy = 7,
    OffsetPolicy = 8,
    DedupePolicy = 9,
    TransportConfig = 10,
    ExternalRelayIdentity = 11,
    ExternalRelayPolicy = 12,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDeliveryBackpressurePolicy {
    pub bounded_queue_capacity: usize,
    pub ttl_millis: u64,
    pub overflow_dead_letters: bool,
    pub idempotency_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDeliverySubscriberFilter {
    pub subscriber_id: SubscriberId,
    pub target_handler: TargetHandler,
    pub event_namespace: EventNamespace,
    pub accepted_event_types: Vec<crate::EventType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub external_transport_delivery_claimed: bool,
    pub external_relay_delivery_claimed: bool,
    pub decision_authority_claimed: bool,
    pub side_effect_authority_claimed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub local_delivery_ready: bool,
    pub external_transport_delivery_implemented: bool,
    pub external_relay_delivery_implemented: bool,
    pub subscriber_filtering_enabled: bool,
    pub decision_authority: bool,
    pub side_effect_authority: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventDeliveryDecisionError {
    EmptySubscriberAcceptedEvents,
    SubscriberFilterOutsideNamespace,
    InvalidBackpressureCapacity,
    InvalidBackpressureTtl,
    LiveExternalTransportDeliveryClaimRejected,
    LiveExternalRelayDeliveryClaimRejected,
    DecisionAuthorityClaimRejected,
    SideEffectAuthorityClaimRejected,
}

pub(super) fn reject_claims(
    input: &EventDeliveryDecisionInput,
) -> Result<(), EventDeliveryDecisionError> {
    if input.external_transport_delivery_claimed {
        return Err(EventDeliveryDecisionError::LiveExternalTransportDeliveryClaimRejected);
    }
    if input.external_relay_delivery_claimed {
        return Err(EventDeliveryDecisionError::LiveExternalRelayDeliveryClaimRejected);
    }
    if input.decision_authority_claimed {
        return Err(EventDeliveryDecisionError::DecisionAuthorityClaimRejected);
    }
    if input.side_effect_authority_claimed {
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
        .any(|event_type| !input.event_namespace.matches_event_type(event_type))
    {
        return Err(EventDeliveryDecisionError::SubscriberFilterOutsideNamespace);
    }
    Ok(())
}

pub(super) fn validate_backpressure(
    policy: &EventDeliveryBackpressurePolicy,
) -> Result<(), EventDeliveryDecisionError> {
    if policy.bounded_queue_capacity == 0 {
        return Err(EventDeliveryDecisionError::InvalidBackpressureCapacity);
    }
    if policy.ttl_millis == 0 {
        return Err(EventDeliveryDecisionError::InvalidBackpressureTtl);
    }
    Ok(())
}

pub fn artifact_ref(
    input: &EventDeliveryDecisionInput,
    artifact: EventDeliveryRequiredArtifact,
) -> Option<&SourceComponent> {
    ARTIFACT_ACCESSORS
        .get(artifact as usize)
        .and_then(|accessor| accessor(input))
}

type ArtifactAccessor = fn(&EventDeliveryDecisionInput) -> Option<&SourceComponent>;

const ARTIFACT_ACCESSORS: [ArtifactAccessor; 13] = [
    custody_proof_ref,
    publisher_auth_ref,
    subscriber_auth_ref,
    encryption_ref,
    retention_policy_ref,
    replay_plan_ref,
    deletion_plan_ref,
    backpressure_policy_ref,
    offset_policy_ref,
    dedupe_policy_ref,
    transport_config_ref,
    relay_identity_ref,
    relay_policy_ref,
];

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
