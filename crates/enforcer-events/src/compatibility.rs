use enforcer_domain::events_types::{
    EventCompatibilityNote, EventCompatibilityStatus, EventProofArtifact, EventSemanticId,
    EventSourceSemantic, RenderedMarkdown, RustTypeName,
};

use crate::error::EventingError;

const MATRIX_TITLE: &str = "# Eventing Compatibility Matrix";
const MATRIX_HEADER: &str =
    "| Semantic Id | Source Semantic | Rust Surface | Status | Proof | Note |";
const MATRIX_SEPARATOR: &str = "| --- | --- | --- | --- | --- | --- |";

/// Event-runtime data for event compatibility entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCompatibilityEntry {
    semantic_id: EventSemanticId,
    source_semantic: EventSourceSemantic,
    rust_surface: RustTypeName,
    status: EventCompatibilityStatus,
    proof_artifact: EventProofArtifact,
    compatibility_note: EventCompatibilityNote,
}

/// Event-runtime data for event compatibility matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCompatibilityMatrix {
    entries: Vec<EventCompatibilityEntry>,
}

impl EventCompatibilityMatrix {
    /// Executes the ocentra games lineage event-runtime operation.
    pub fn ocentra_games_lineage() -> Result<Self, EventingError> {
        let entries = LINEAGE_ROWS
            .iter()
            .map(|row| {
                // ALLOC-JUSTIFICATION: compatibility rows retain owned validated values after the static row adapter returns.
                Ok(EventCompatibilityEntry {
                    semantic_id: EventSemanticId::try_new(row.0.to_owned())?,
                    // ALLOC-JUSTIFICATION: compatibility entries independently retain their source semantics.
                    source_semantic: EventSourceSemantic::try_new(row.1.to_owned())?,
                    rust_surface: RustTypeName::try_new(row.2.to_owned())?,
                    status: row.3,
                    // ALLOC-JUSTIFICATION: proof and note values are retained in the compatibility report.
                    proof_artifact: EventProofArtifact::try_new(row.4.to_owned())?,
                    compatibility_note: EventCompatibilityNote::try_new(row.5.to_owned())?,
                })
            })
            .collect::<Result<Vec<_>, EventingError>>()?;
        Ok(Self { entries })
    }

    /// Executes the entries event-runtime operation.
    pub fn entries(&self) -> &[EventCompatibilityEntry] {
        &self.entries
    }

    /// Executes the entry event-runtime operation.
    pub fn entry(&self, semantic_id: &EventSemanticId) -> Option<&EventCompatibilityEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.semantic_id == semantic_id)
    }

    /// Executes the compatible entries event-runtime operation.
    pub fn compatible_entries(&self) -> Vec<&EventCompatibilityEntry> {
        self.entries_by_status(EventCompatibilityStatus::Compatible)
    }

    /// Executes the intentional deviations event-runtime operation.
    pub fn intentional_deviations(&self) -> Vec<&EventCompatibilityEntry> {
        self.entries_by_status(EventCompatibilityStatus::IntentionalDeviation)
    }

    /// Executes the manual required entries event-runtime operation.
    pub fn manual_required_entries(&self) -> Vec<&EventCompatibilityEntry> {
        self.entries_by_status(EventCompatibilityStatus::ManualRequired)
    }

    /// Executes the render markdown event-runtime operation.
    pub fn render_markdown(&self) -> Result<RenderedMarkdown, EventingError> {
        let mut markdown = String::from(MATRIX_TITLE);
        markdown.push_str("\n\n");
        markdown.push_str(MATRIX_HEADER);
        markdown.push('\n');
        markdown.push_str(MATRIX_SEPARATOR);
        markdown.push('\n');
        for entry in &self.entries {
            markdown.push_str("| ");
            markdown.push_str(&entry.semantic_id.as_str().replace('|', "\\|"));
            markdown.push_str(" | ");
            markdown.push_str(&entry.source_semantic.as_str().replace('|', "\\|"));
            markdown.push_str(" | ");
            markdown.push_str(&entry.rust_surface.as_str().replace('|', "\\|"));
            markdown.push_str(" | ");
            markdown.push_str(entry.status.as_str());
            markdown.push_str(" | ");
            markdown.push_str(&entry.proof_artifact.as_str().replace('|', "\\|"));
            markdown.push_str(" | ");
            markdown.push_str(&entry.compatibility_note.as_str().replace('|', "\\|"));
            markdown.push_str(" |\n");
        }
        RenderedMarkdown::try_new(markdown).map_err(EventingError::from)
    }

    fn entries_by_status(&self, status: EventCompatibilityStatus) -> Vec<&EventCompatibilityEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status == status)
            .collect()
    }
}

impl EventCompatibilityEntry {
    /// Executes the semantic id event-runtime operation.
    pub fn semantic_id(&self) -> &EventSemanticId {
        &self.semantic_id
    }

    /// Executes the source semantic event-runtime operation.
    pub fn source_semantic(&self) -> &EventSourceSemantic {
        &self.source_semantic
    }

    /// Executes the rust surface event-runtime operation.
    pub fn rust_surface(&self) -> &RustTypeName {
        &self.rust_surface
    }

    /// Executes the status event-runtime operation.
    pub fn status(&self) -> EventCompatibilityStatus {
        self.status
    }

    /// Executes the proof artifact event-runtime operation.
    pub fn proof_artifact(&self) -> &EventProofArtifact {
        &self.proof_artifact
    }

    /// Executes the compatibility note event-runtime operation.
    pub fn compatibility_note(&self) -> &EventCompatibilityNote {
        &self.compatibility_note
    }
}

type CompatibilityRow = (
    &'static str,
    &'static str,
    &'static str,
    EventCompatibilityStatus,
    &'static str,
    &'static str,
);

const LINEAGE_ROWS: &[CompatibilityRow] = &[
    (
        "class-backed-contracts",
        "Class-backed contracts expose canonical static event types",
        "Payload-derived DomainEvent::contract plus EventContractRegistry descriptors",
        EventCompatibilityStatus::IntentionalDeviation,
        "output/eventing-plan-proof/72-contract-registry/proof-summary.json",
        "Payload-derived contracts support Rust enum variants; stored decode rejects mismatches",
    ),
    (
        "event-args-metadata",
        "EventArgsBase carries unique id, timestamp, and target handler",
        "EventMetadata and EventFrame carry event id, observed_at, and target",
        EventCompatibilityStatus::Compatible,
        "test-results/eventing-network-runtime-proof/proof.json",
        "Metadata is part of the typed envelope and stored boundary",
    ),
    (
        "target-handler-routing",
        "Target handler can constrain delivery to one subscriber",
        "EventMetadata.target_handler filters EventSubscriber target handlers",
        EventCompatibilityStatus::Compatible,
        "test-results/eventing-network-runtime-proof/proof.json",
        "Wrong-target subscribers are reported as not invoked",
    ),
    (
        "subscribe-unsubscribe",
        "EventBus subscribe, subscribeAsync, and unsubscribe",
        "EventBus::subscribe, subscribe_with_handle, and SubscriptionHandle",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/14-24-runtime-lifecycle/proof-summary.json",
        "Async handlers are first-class and unsubscribe is idempotent",
    ),
    (
        "sync-and-async-publish",
        "EventBus publish and publishAsync",
        "publish, publish_and_wait, and publish_detached",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/14-24-runtime-lifecycle/proof-summary.json",
        "Detached publish returns an observable join report",
    ),
    (
        "registrar-dispose",
        "EventRegistrar owns scoped subscriptions and dispose",
        "EventRegistrar subscribe, dispose, Drop cleanup, and disposed guard",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/14-24-runtime-lifecycle/proof-summary.json",
        "Dispose removes owned subscriptions and rejects new ones",
    ),
    (
        "operation-result-deferred",
        "OperationResult and OperationDeferred request/response flow",
        "RequestEvent::Response, EventResponseContract, and RequestRegistry",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/31-35-request-response/proof-summary.json",
        "Local completion is validated and separated from durable results",
    ),
    (
        "queue-retry-timeout",
        "Queueing, retry, TTL, max queue, and timeout semantics",
        "EventQueuePolicy, HandlerExecutionPolicy, and ManualEventClock",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/71-manual-clock/proof-summary.json",
        "Deterministic manual clock proof avoids long wall-clock sleeps",
    ),
    (
        "in-flight-duplicate-guard",
        "In-flight duplicate guard prevents repeated work",
        "IdempotencyKey queue and in-flight duplicate registry",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/25-30-queue-policy/proof-summary.json",
        "Concurrent duplicate publish rejects while the first is active",
    ),
    (
        "isolated-test-bus",
        "Isolated test bus and clear lifecycle",
        "EventBus::new, ManualEventClock, EventRecorder, and clear_for_test",
        EventCompatibilityStatus::Compatible,
        "output/eventing-plan-proof/74-lifecycle-clear/proof-summary.json",
        "Test clear is explicit and does not create a production singleton",
    ),
    (
        "payload-republish-override",
        "Payload-carried republish or force override",
        "Explicit idempotency rejection; constrained override remains unclaimed",
        EventCompatibilityStatus::IntentionalDeviation,
        "output/eventing-plan-proof/73-duplicate-subscriber/proof-summary.json",
        "Future override must be a typed policy with reason and report",
    ),
    (
        "payload-disposal-callbacks",
        "Event payload disposal callbacks or resource handles",
        "Immutable payload facts; local handles stay in registries",
        EventCompatibilityStatus::IntentionalDeviation,
        "output/eventing-plan-proof/66-76-source-safety/proof-summary.json",
        "Payloads cannot carry deferred, cancellation, or cleanup handles",
    ),
    (
        "broker-backed-delivery",
        "Cross-process or broker-backed event delivery",
        "Stored envelope transport boundary is not yet broker-backed",
        EventCompatibilityStatus::ManualRequired,
        "docs/plans/network-plan/workpacks/README.md#workpack-45",
        "Broker delivery is P6 and cannot redefine local dispatch semantics",
    ),
];
// INVALID-INPUT-TEST: contract compatibility tests reject malformed event type
// values before compatibility rows are constructed.
