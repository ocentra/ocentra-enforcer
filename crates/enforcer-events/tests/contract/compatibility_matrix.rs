use enforcer_domain::events_types::{EventCompatibilityStatus, EventSemanticId};
use enforcer_events::compatibility::EventCompatibilityMatrix;

const CLASS_CONTRACTS: &str = "class-backed-contracts";
const EVENT_METADATA: &str = "event-args-metadata";
const TARGET_ROUTING: &str = "target-handler-routing";
const SUBSCRIBE_UNSUBSCRIBE: &str = "subscribe-unsubscribe";
const SYNC_ASYNC_PUBLISH: &str = "sync-and-async-publish";
const REGISTRAR_DISPOSE: &str = "registrar-dispose";
const OPERATION_DEFERRED: &str = "operation-result-deferred";
const QUEUE_RETRY_TIMEOUT: &str = "queue-retry-timeout";
const DUPLICATE_GUARD: &str = "in-flight-duplicate-guard";
const ISOLATED_TEST_BUS: &str = "isolated-test-bus";
const REPUBLISH_OVERRIDE: &str = "payload-republish-override";
const DISPOSAL_CALLBACKS: &str = "payload-disposal-callbacks";
const BROKER_DELIVERY: &str = "broker-backed-delivery";

#[test]
fn compatibility_matrix_covers_games_lineage_semantics(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage()?;
    let semantic_ids = matrix
        .entries()
        .iter()
        .map(|entry| entry.semantic_id().as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        semantic_ids,
        vec![
            CLASS_CONTRACTS,
            EVENT_METADATA,
            TARGET_ROUTING,
            SUBSCRIBE_UNSUBSCRIBE,
            SYNC_ASYNC_PUBLISH,
            REGISTRAR_DISPOSE,
            OPERATION_DEFERRED,
            QUEUE_RETRY_TIMEOUT,
            DUPLICATE_GUARD,
            ISOLATED_TEST_BUS,
            REPUBLISH_OVERRIDE,
            DISPOSAL_CALLBACKS,
            BROKER_DELIVERY,
        ]
    );
    assert_eq!(matrix.compatible_entries().len(), 9);
    assert_eq!(matrix.intentional_deviations().len(), 3);
    assert_eq!(matrix.manual_required_entries().len(), 1);
    Ok(())
}

#[test]
fn compatibility_matrix_marks_deviations_and_manual_required_scope(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage()?;

    assert_eq!(
        matrix
            .entry(&EventSemanticId::parse(REPUBLISH_OVERRIDE)?)
            .ok_or("republish entry")?
            .status(),
        EventCompatibilityStatus::IntentionalDeviation
    );
    assert_eq!(
        matrix
            .entry(&EventSemanticId::parse(DISPOSAL_CALLBACKS)?)
            .ok_or("disposal entry")?
            .status(),
        EventCompatibilityStatus::IntentionalDeviation
    );
    assert_eq!(
        matrix
            .entry(&EventSemanticId::parse(BROKER_DELIVERY)?)
            .ok_or("broker entry")?
            .status(),
        EventCompatibilityStatus::ManualRequired
    );
    for entry in matrix.entries() {
        assert!(!entry.source_semantic().as_str().is_empty());
        assert!(!entry.rust_surface().as_str().is_empty());
        assert!(!entry.proof_artifact().as_str().is_empty());
        assert!(!entry.compatibility_note().as_str().is_empty());
    }
    Ok(())
}

#[test]
fn compatibility_matrix_renders_deterministic_markdown(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matrix = EventCompatibilityMatrix::ocentra_games_lineage()?;
    let markdown = matrix.render_markdown()?;

    let lines = markdown.as_str().lines().collect::<Vec<_>>();
    assert_eq!(
        lines.first().copied(),
        Some("# Eventing Compatibility Matrix")
    );
    assert!(
        lines.contains(&"| Semantic Id | Source Semantic | Rust Surface | Status | Proof | Note |")
    );
    assert!(lines.contains(
        &"| class-backed-contracts | Class-backed contracts expose canonical static event types | Payload-derived DomainEvent::contract plus EventContractRegistry descriptors | intentional-deviation | output/eventing-plan-proof/72-contract-registry/proof-summary.json | Payload-derived contracts support Rust enum variants; stored decode rejects mismatches |"
    ));
    assert!(lines.contains(
        &"| payload-republish-override | Payload-carried republish or force override | Explicit idempotency rejection; constrained override remains unclaimed | intentional-deviation | output/eventing-plan-proof/73-duplicate-subscriber/proof-summary.json | Future override must be a typed policy with reason and report |"
    ));
    assert!(lines.contains(
        &"| broker-backed-delivery | Cross-process or broker-backed event delivery | Stored envelope transport boundary is not yet broker-backed | manual-required | docs/plans/network-plan/workpacks/README.md#workpack-45 | Broker delivery is P6 and cannot redefine local dispatch semantics |"
    ));
    Ok(())
}
