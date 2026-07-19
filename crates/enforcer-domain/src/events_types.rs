//! Canonical event-transport value types.
//!
//! These brands are transport values shared by event producers, queues,
//! journals, and consumers.  The event runtime owns dispatch behavior; this
//! dependency-leaf module owns the validated values that cross those edges.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::boundary::decode_error::DecodeError;

static EVENT_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static REQUEST_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

macro_rules! event_text_identifier {
    ($name:ident, $label:literal, taxonomy) => {
        event_text_identifier!($name, $label, event_taxonomy);
    };
    ($name:ident, $label:literal, identifier) => {
        event_text_identifier!($name, $label, identifier_without_whitespace);
    };
    ($name:ident, $label:literal, unrestricted) => {
        event_text_identifier!($name, $label, non_blank);
    };
    ($name:ident, $label:literal, $validate:ident) => {
        #[doc = concat!("Validated event-domain text for `", $label, "`.")]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = "Construct the value, rejecting invalid event text."]
            pub fn try_new(value: String) -> Result<Self, DecodeError> {
                $validate($label, &value)?;
                Ok(Self(value))
            }

            #[doc = "Parse validated event text from a borrowed source."]
            pub fn parse(value: &str) -> Result<Self, DecodeError> {
                $validate($label, value)?;
                // ALLOC-JUSTIFICATION: canonical event text outlives the borrowed boundary input.
                Ok(Self(value.to_owned()))
            }

            #[doc = "View the validated event text."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                $validate($label, &value)?;
                Ok(Self(value))
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

fn non_blank(label: &str, value: &str) -> Result<(), DecodeError> {
    if value.trim().is_empty() {
        return Err(DecodeError::new(label, "expected a non-blank value"));
    }
    Ok(())
}

fn identifier_without_whitespace(label: &str, value: &str) -> Result<(), DecodeError> {
    non_blank(label, value)?;
    if value.chars().any(char::is_whitespace) {
        return Err(DecodeError::new(
            label,
            "identifier must not contain whitespace",
        ));
    }
    Ok(())
}

fn event_taxonomy(label: &str, value: &str) -> Result<(), DecodeError> {
    non_blank(label, value)?;
    let mut previous_separator = false;
    for (index, character) in value.chars().enumerate() {
        let separator = matches!(character, '.' | '/');
        let valid =
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-') || separator;
        if !valid || (separator && (index == 0 || previous_separator)) {
            return Err(DecodeError::new(
                label,
                "expected event taxonomy segments separated by `.` or `/`",
            )
            .with_input_hint(value));
        }
        previous_separator = separator;
    }
    if previous_separator {
        return Err(
            DecodeError::new(label, "event taxonomy must not end with a separator")
                .with_input_hint(value),
        );
    }
    Ok(())
}

event_text_identifier!(EventType, "event_type", taxonomy);

impl EventType {
    /// Canonical event type emitted for one coordination fix-loop decision.
    pub fn coordination_fix_loop_decision() -> Self {
        Self("coordination.fix_loop.decision".to_owned())
    }
}
event_text_identifier!(EventNamespace, "event_namespace", taxonomy);
event_text_identifier!(EventId, "event_id", identifier);
event_text_identifier!(CorrelationId, "correlation_id", identifier);
event_text_identifier!(CausationId, "causation_id", identifier);
event_text_identifier!(RequestId, "request_id", identifier);
event_text_identifier!(JournalHash, "journal_hash", identifier);
event_text_identifier!(AggregateKey, "aggregate_key", unrestricted);
event_text_identifier!(IdempotencyKey, "idempotency_key", unrestricted);
event_text_identifier!(SubscriberId, "subscriber_id", identifier);
event_text_identifier!(TargetHandler, "target_handler", identifier);
event_text_identifier!(EventCustody, "event_custody", unrestricted);
event_text_identifier!(RuntimeRole, "runtime_role", unrestricted);
event_text_identifier!(SourceService, "source_service", identifier);
event_text_identifier!(SourceComponent, "source_component", identifier);
event_text_identifier!(RuntimeInstanceId, "runtime_instance_id", identifier);
event_text_identifier!(RecordedAt, "recorded_at", unrestricted);
event_text_identifier!(EventErrorReason, "event_error_reason", unrestricted);
event_text_identifier!(EventErrorPath, "event_error_path", unrestricted);
event_text_identifier!(EventSemanticId, "event_semantic_id", unrestricted);
event_text_identifier!(EventSourceSemantic, "event_source_semantic", unrestricted);
event_text_identifier!(RustTypeName, "rust_type_name", unrestricted);
event_text_identifier!(EventProofArtifact, "event_proof_artifact", unrestricted);
event_text_identifier!(
    EventCompatibilityNote,
    "event_compatibility_note",
    unrestricted
);
event_text_identifier!(RenderedMarkdown, "rendered_markdown", unrestricted);
event_text_identifier!(JournalLine, "journal_line", unrestricted);
event_text_identifier!(JournalPath, "journal_path", unrestricted);
event_text_identifier!(EventErrorField, "event_error_field", identifier);

/// A duration carried by event runtime decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "BRAND-INVARIANT: event timing is represented explicitly; zero is a valid duration."]
pub struct EventDuration(std::time::Duration);

impl EventDuration {
    /// Zero-length event duration.
    pub const ZERO: Self = Self(std::time::Duration::ZERO);

    /// Construct a positive event duration from boundary milliseconds.
    pub const fn try_new_millis(value: std::num::NonZeroU64) -> Self {
        Self(std::time::Duration::from_millis(value.get()))
    }

    /// Construct a positive event duration from boundary nanoseconds.
    pub const fn try_new_nanos(value: std::num::NonZeroU64) -> Self {
        Self(std::time::Duration::from_nanos(value.get()))
    }

    /// Return positive nanoseconds for boundary encoding, preserving zero as absence.
    pub fn as_nonzero_nanos(self) -> Option<std::num::NonZeroU64> {
        let nanos = u64::try_from(self.0.as_nanos()).unwrap_or(u64::MAX);
        std::num::NonZeroU64::new(nanos)
    }

    /// Access the represented runtime duration.
    pub const fn value(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for EventDuration {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

/// Whether an event-backed activity is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for EventActivityState."]
pub enum EventActivityState {
    Active,
    Inactive,
}

/// Whether journal processing should append or skip an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for JournalAppendDecision."]
pub enum JournalAppendDecision {
    Append,
    Skip,
}

/// Whether queue idempotency enforcement is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for QueueIdempotencyState."]
pub enum QueueIdempotencyState {
    Enabled,
    Disabled,
}

/// Whether a queued event has passed its expiration boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for QueueExpirationState."]
pub enum QueueExpirationState {
    Expired,
    Current,
}

/// Whether an event satisfies a requested match predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for EventMatchState."]
pub enum EventMatchState {
    Matches,
    DoesNotMatch,
}

/// Whether publication for a request has completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for RequestPublishState."]
pub enum RequestPublishState {
    Complete,
    Pending,
}

/// Whether journal recovery produced a recovered event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for JournalRecoveryState."]
pub enum JournalRecoveryState {
    Recovered,
    Unrecovered,
}

impl EventErrorReason {
    /// Build an infallible diagnostic reason while preserving the non-blank invariant.
    #[must_use]
    #[doc = "Build a diagnostic event-error reason with a stable fallback."]
    pub fn from_diagnostic(value: impl Into<String>) -> Self {
        let value = value.into();
        if value.trim().is_empty() {
            Self(String::from("unspecified event error"))
        } else {
            Self(value)
        }
    }
}

impl EventErrorPath {
    /// Build an infallible diagnostic path while preserving the non-blank invariant.
    #[must_use]
    #[doc = "Build a diagnostic event-error path with a stable fallback."]
    pub fn from_diagnostic(value: impl Into<String>) -> Self {
        let value = value.into();
        if value.trim().is_empty() {
            Self(String::from("unknown event error path"))
        } else {
            Self(value)
        }
    }
}

impl EventErrorField {
    /// Build an infallible diagnostic field with a stable valid fallback.
    #[must_use]
    #[doc = "Build a diagnostic event-error field with a stable fallback."]
    pub fn from_diagnostic(value: impl Into<String>) -> Self {
        let value = value.into();
        match Self::try_new(value) {
            Ok(field) => field,
            Err(_) => Self(String::from("decoded_value")),
        }
    }
}

impl JournalPath {
    /// Build an infallible diagnostic journal path while preserving the non-blank invariant.
    #[must_use]
    #[doc = "Build a diagnostic journal path with a stable fallback."]
    pub fn from_diagnostic(value: impl Into<String>) -> Self {
        let value = value.into();
        if value.trim().is_empty() {
            Self(String::from("unknown journal path"))
        } else {
            Self(value)
        }
    }
}

impl JournalLine {
    /// Build an infallible diagnostic journal line while preserving the non-blank invariant.
    #[must_use]
    #[doc = "Build a diagnostic journal line with a stable fallback."]
    pub fn from_diagnostic(value: impl Into<String>) -> Self {
        let value = value.into();
        if value.trim().is_empty() {
            Self(String::from("unavailable journal line"))
        } else {
            Self(value)
        }
    }
}

/// A validated, non-zero position in an event journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "BRAND-INVARIANT: journal positions are always non-zero."]
pub struct JournalSequence(std::num::NonZeroU64);

impl JournalSequence {
    pub const fn first() -> Self {
        Self(std::num::NonZeroU64::MIN)
    }

    pub const fn try_new(value: std::num::NonZeroU64) -> Self {
        Self(value)
    }

    /// Construct a journal position from its persistence representation.
    pub fn new(value: u64) -> Result<Self, DecodeError> {
        std::num::NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| DecodeError::new("journal_sequence", "must be greater than zero"))
    }

    /// Return the primitive journal position for persistence encoding.
    pub const fn as_u64(self) -> u64 {
        self.0.get()
    }

    /// Return the non-zero journal position for persistence encoding.
    pub const fn as_nonzero(self) -> std::num::NonZeroU64 {
        self.0
    }

    /// Advance to the next journal position without overflowing.
    pub fn saturating_next(self) -> Self {
        Self(
            std::num::NonZeroU64::new(self.0.get().saturating_add(1))
                .unwrap_or(std::num::NonZeroU64::MAX),
        )
    }
}

impl std::fmt::Display for JournalSequence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0.get())
    }
}

impl EventId {
    /// Generate a process-local event identity.
    pub fn generated() -> Self {
        let micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_micros())
            .unwrap_or(0);
        Self(format!(
            "event-{micros}-{}",
            EVENT_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

impl RequestId {
    /// Generate a process-local request identity.
    pub fn generated() -> Self {
        let micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_micros())
            .unwrap_or(0);
        Self(format!(
            "request-{micros}-{}",
            REQUEST_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

impl EventNamespace {
    /// Derive the leading namespace segment from an event type.
    pub fn from_event_type(event_type: &EventType) -> Result<Self, DecodeError> {
        let namespace = event_type.as_str().split(['.', '/']).next();
        match namespace {
            // ALLOC-JUSTIFICATION: the derived namespace is retained independently of the event type.
            Some(value) => Self::parse(value),
            None => Err(DecodeError::new(
                "event_namespace",
                "event type has no namespace",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "BRAND-INVARIANT: event schema versions are always non-zero."]
pub struct SchemaVersion(std::num::NonZeroU16);

impl SchemaVersion {
    pub const fn try_new(value: std::num::NonZeroU16) -> Self {
        Self(value)
    }

    /// Return the non-zero schema version for persistence encoding.
    pub const fn as_nonzero(self) -> std::num::NonZeroU16 {
        self.0
    }
}

/// A non-negative cardinality emitted by event runtime reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "BRAND-INVARIANT: event counts are non-negative and saturate at numeric bounds."]
pub struct EventCount(usize);

impl EventCount {
    pub const ZERO: Self = Self(0);

    pub const fn try_new(value: std::num::NonZeroUsize) -> Self {
        Self(value.get())
    }

    /// Return a positive count for boundary encoding, preserving zero as absence.
    pub const fn as_nonzero(self) -> Option<std::num::NonZeroUsize> {
        std::num::NonZeroUsize::new(self.0)
    }

    pub fn from_collection<T>(values: &[T]) -> Self {
        Self(values.len())
    }

    /// Return the saturating successor count.
    pub fn incremented(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Return the saturating predecessor count.
    pub fn decremented(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

/// Explicit lifecycle state emitted by event runtime reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventShutdownState."]
pub enum EventShutdownState {
    Active,
    AlreadyShutdown,
}

/// The outcome of removing a subscriber from an event bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for SubscriptionRemovalState."]
pub enum SubscriptionRemovalState {
    Removed,
    AlreadyAbsent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventPriority."]
pub enum EventPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for DispatchMode."]
pub enum DispatchMode {
    Sequential,
    Concurrent,
    OrderedByAggregateKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for ShutdownMode."]
pub enum ShutdownMode {
    Drain,
    DeadLetterQueued,
    DropQueuedForTestOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for NoSubscriberQueuePolicy."]
pub enum NoSubscriberQueuePolicy {
    DispatchWithoutSubscribers,
    Queue,
    DeadLetter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for QueueOverflowPolicy."]
pub enum QueueOverflowPolicy {
    RejectPublish,
    DeadLetterRejected,
    DropOldestAndDeadLetter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for QueueDisposition."]
pub enum QueueDisposition {
    Dispatched,
    QueuedNoSubscriber,
    DeadLetteredNoSubscriber,
    DeadLetteredQueueOverflow,
    DeadLetteredDeadlineExpired,
}

/// Outcome of completing an event request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for RequestCompletionOutcome."]
pub enum RequestCompletionOutcome {
    Completed,
    Duplicate,
    Late,
}

/// Outcome reported by an event handler invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for HandlerOutcome."]
pub enum HandlerOutcome {
    Handled,
    Failed,
    TimedOut,
    DeadlineExpired,
    Panicked,
}

/// Stable reason for routing an event to the dead-letter journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for DeadLetterReason."]
pub enum DeadLetterReason {
    HandlerFailed,
    HandlerTimedOut,
    HandlerDeadlineExpired,
    HandlerPanicked,
    NoSubscriber,
    QueueOverflow,
    QueueExpired,
    DeadlineExpired,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventCompatibilityStatus."]
pub enum EventCompatibilityStatus {
    Compatible,
    IntentionalDeviation,
    ManualRequired,
}

impl EventCompatibilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::IntentionalDeviation => "intentional-deviation",
            Self::ManualRequired => "manual-required",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventTopologyStatus."]
pub enum EventTopologyStatus {
    Covered,
    NoPublisher,
    NoSubscriber,
    AcceptedOneSided,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryRouteKind."]
pub enum EventDeliveryRouteKind {
    LocalInProcess,
    LocalService,
    ExternalTransport,
    ExternalRelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryDecisionState."]
pub enum EventDeliveryDecisionState {
    LocalRouteReady,
    ExternalTransportRouteManualRequired,
    ExternalRelayRouteManualRequired,
    ExternalTransportRouteRequirementsSatisfied,
    ExternalRelayRouteRequirementsSatisfied,
}

/// Whether a delivery route has all of its required proof artifacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryRequirementsState."]
pub enum EventDeliveryRequirementsState {
    Satisfied,
    Missing,
}

/// Whether a delivery configuration explicitly requires idempotency tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryIdempotencyRequirement."]
pub enum EventDeliveryIdempotencyRequirement {
    Required,
    NotRequired,
}

/// How a bounded delivery queue handles overflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryOverflowPolicy."]
pub enum EventDeliveryOverflowPolicy {
    DeadLetter,
    Reject,
}

/// Whether an input attempts to claim a delivery capability that is not provided here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryClaimState."]
pub enum EventDeliveryClaimState {
    NotClaimed,
    Claimed,
}

/// Whether the event runtime currently provides a specific delivery capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryCapabilityState."]
pub enum EventDeliveryCapabilityState {
    Available,
    Unavailable,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for EventDeliveryRequiredArtifact."]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for RegistrarStatus."]
pub enum RegistrarStatus {
    #[default]
    Active,
    Disposed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for ReplayMode."]
pub enum ReplayMode {
    ProjectionOnly,
    ActionHandlersAllowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalHashChain."]
pub enum JournalHashChain {
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalFlushPolicy."]
pub enum JournalFlushPolicy {
    Always,
    Buffered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalMode."]
pub enum JournalMode {
    Disabled,
    BeforeDispatch,
    AfterDispatch,
    BeforeAndAfterDispatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalDispatchPhase."]
pub enum JournalDispatchPhase {
    BeforeDispatch,
    AfterDispatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc = "Canonical domain representation for JournalSelector."]
pub enum JournalSelector {
    All,
    EventTypes(Vec<EventType>),
    Namespaces(Vec<EventNamespace>),
    ContractAllowlist(Vec<EventType>),
}

macro_rules! serde_kebab_case_unit_enum {
    ($name:ty, {$($variant:path => $wire:literal),+ $(,)?}) => {
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(match self { $($variant => $wire),+ })
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let wire = <String as serde::Deserialize>::deserialize(deserializer)?;
                match wire.as_str() {
                    $($wire => Ok($variant)),+,
                    _ => Err(serde::de::Error::unknown_variant(&wire, &[$($wire),+])),
                }
            }
        }
    };
}

serde_kebab_case_unit_enum!(EventCompatibilityStatus, {
    EventCompatibilityStatus::Compatible => "compatible",
    EventCompatibilityStatus::IntentionalDeviation => "intentional-deviation",
    EventCompatibilityStatus::ManualRequired => "manual-required",
});
serde_kebab_case_unit_enum!(EventTopologyStatus, {
    EventTopologyStatus::Covered => "covered",
    EventTopologyStatus::NoPublisher => "no-publisher",
    EventTopologyStatus::NoSubscriber => "no-subscriber",
    EventTopologyStatus::AcceptedOneSided => "accepted-one-sided",
});
serde_kebab_case_unit_enum!(EventDeliveryRouteKind, {
    EventDeliveryRouteKind::LocalInProcess => "local-in-process",
    EventDeliveryRouteKind::LocalService => "local-service",
    EventDeliveryRouteKind::ExternalTransport => "external-transport",
    EventDeliveryRouteKind::ExternalRelay => "external-relay",
});
serde_kebab_case_unit_enum!(DeadLetterReason, {
    DeadLetterReason::HandlerFailed => "handler-failed",
    DeadLetterReason::HandlerTimedOut => "handler-timed-out",
    DeadLetterReason::HandlerDeadlineExpired => "handler-deadline-expired",
    DeadLetterReason::HandlerPanicked => "handler-panicked",
    DeadLetterReason::NoSubscriber => "no-subscriber",
    DeadLetterReason::QueueOverflow => "queue-overflow",
    DeadLetterReason::QueueExpired => "queue-expired",
    DeadLetterReason::DeadlineExpired => "deadline-expired",
    DeadLetterReason::Shutdown => "shutdown",
});
serde_kebab_case_unit_enum!(EventDeliveryDecisionState, {
    EventDeliveryDecisionState::LocalRouteReady => "local-route-ready",
    EventDeliveryDecisionState::ExternalTransportRouteManualRequired => "external-transport-route-manual-required",
    EventDeliveryDecisionState::ExternalRelayRouteManualRequired => "external-relay-route-manual-required",
    EventDeliveryDecisionState::ExternalTransportRouteRequirementsSatisfied => "external-transport-route-requirements-satisfied",
    EventDeliveryDecisionState::ExternalRelayRouteRequirementsSatisfied => "external-relay-route-requirements-satisfied",
});
serde_kebab_case_unit_enum!(EventDeliveryRequiredArtifact, {
    EventDeliveryRequiredArtifact::CustodyProof => "custody-proof",
    EventDeliveryRequiredArtifact::PublisherAuthProof => "publisher-auth-proof",
    EventDeliveryRequiredArtifact::SubscriberAuthProof => "subscriber-auth-proof",
    EventDeliveryRequiredArtifact::EncryptionProof => "encryption-proof",
    EventDeliveryRequiredArtifact::RetentionPolicy => "retention-policy",
    EventDeliveryRequiredArtifact::ReplayPlan => "replay-plan",
    EventDeliveryRequiredArtifact::DeletionPlan => "deletion-plan",
    EventDeliveryRequiredArtifact::BackpressurePolicy => "backpressure-policy",
    EventDeliveryRequiredArtifact::OffsetPolicy => "offset-policy",
    EventDeliveryRequiredArtifact::DedupePolicy => "dedupe-policy",
    EventDeliveryRequiredArtifact::TransportConfig => "transport-config",
    EventDeliveryRequiredArtifact::ExternalRelayIdentity => "external-relay-identity",
    EventDeliveryRequiredArtifact::ExternalRelayPolicy => "external-relay-policy",
});
serde_kebab_case_unit_enum!(ReplayMode, {
    ReplayMode::ProjectionOnly => "projection-only",
    ReplayMode::ActionHandlersAllowed => "action-handlers-allowed",
});
serde_kebab_case_unit_enum!(JournalDispatchPhase, {
    JournalDispatchPhase::BeforeDispatch => "before-dispatch",
    JournalDispatchPhase::AfterDispatch => "after-dispatch",
});

#[cfg(test)]
mod property_tests {
    use super::EventType;
    use proptest::{prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn event_type_parser_accepts_generated_taxonomy(raw in "[a-z][a-z0-9_]{0,23}(\\.[a-z][a-z0-9_]{0,23}){1,3}") {
            prop_assert_eq!(
                EventType::parse(&raw).map(|value| value.as_str().to_owned()),
                Ok(raw)
            );
        }
    }
}
