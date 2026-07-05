use enforcer_events::envelope::{
    DomainEvent, EventContract, EventEnvelope, EventMetadata, EventSource, StoredEventEnvelope,
};
use enforcer_events::error::EventingError;
use enforcer_events::ids::{
    AggregateKey, CorrelationId, EventCustody, EventId, EventType, IdempotencyKey, RecordedAt,
    RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent, SourceService, TargetHandler,
};
use serde::{Deserialize, Serialize};

const TEST_EVENT_TYPE: &str = "eventing.version-skew.roundtrip";
const TEST_EVENT_ID: &str = "eventing-version-skew-event-id";
const TEST_CORRELATION_ID: &str = "eventing-version-skew-correlation-id";
const TEST_AGGREGATE_KEY: &str = "eventing-version-skew-aggregate";
const TEST_IDEMPOTENCY_KEY: &str = "eventing-version-skew-idempotency";
const TEST_CUSTODY: &str = "local-only";
const TEST_RUNTIME_ROLE: &str = "parent";
const TEST_SOURCE_SERVICE: &str = "eventing-version-skew-service";
const TEST_SOURCE_COMPONENT: &str = "eventing-version-skew-component";
const TEST_RUNTIME_INSTANCE: &str = "eventing-version-skew-runtime";
const TEST_TARGET: &str = "eventing-version-skew-target";
const TEST_OBSERVED_AT: &str = "2026-06-13T20:20:00Z";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct VersionedRoundtripEvent {
    label: String,
}

impl DomainEvent for VersionedRoundtripEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            EventType::parse(TEST_EVENT_TYPE)?,
            SchemaVersion::new(1)?,
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        AggregateKey::parse(TEST_AGGREGATE_KEY)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        IdempotencyKey::parse(TEST_IDEMPOTENCY_KEY)
    }
}

#[test]
fn stored_envelope_rejects_newer_schema_version_without_silent_decode(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventEnvelope::from_event(
        VersionedRoundtripEvent {
            label: String::from("current-contract"),
        },
        metadata()?,
    )?;
    let mut stored = live.store()?;
    stored.contract.schema_version = SchemaVersion::new(2)?;

    let error = match stored.decode::<VersionedRoundtripEvent>() {
        Err(e) => e,
        Ok(_) => {
            return Err(
                "newer stored schema version must fail closed: expected Err but got Ok".into(),
            )
        }
    };

    assert_eq!(
        error,
        EventingError::ContractMismatch {
            expected: EventType::parse(TEST_EVENT_TYPE)?,
            received: EventType::parse(TEST_EVENT_TYPE)?,
            expected_schema_version: SchemaVersion::new(1)?,
            received_schema_version: SchemaVersion::new(2)?,
        }
    );
    assert_eq!(
        error.to_string(),
        "event contract mismatch: expected eventing.version-skew.roundtrip@1, received eventing.version-skew.roundtrip@2"
    );

    Ok(())
}

#[test]
fn stored_envelope_rejects_older_schema_version_without_silent_decode(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventEnvelope::from_event(
        VersionedRoundtripEvent {
            label: String::from("current-contract"),
        },
        metadata()?,
    )?;
    let mut stored = live.store()?;
    stored.contract.schema_version = SchemaVersion::new(1)?;

    let stored_json = serde_json::to_value(&stored)?;
    let mut skewed_json = stored_json;
    skewed_json["contract"]["schemaVersion"] = serde_json::Value::from(0);

    let error = match serde_json::from_value::<StoredEventEnvelope>(skewed_json) {
        Err(e) => e,
        Ok(_) => {
            return Err(
                "zero stored schema version must fail during deserialize: expected Err but got Ok"
                    .into(),
            )
        }
    };

    assert!(error
        .to_string()
        .contains("event schema version must be nonzero"));

    Ok(())
}

fn metadata() -> Result<EventMetadata, EventingError> {
    Ok(EventMetadata::from_parts(
        EventId::parse(TEST_EVENT_ID)?,
        CorrelationId::parse(TEST_CORRELATION_ID)?,
        EventSource::new(
            EventCustody::parse(TEST_CUSTODY)?,
            RuntimeRole::parse(TEST_RUNTIME_ROLE)?,
            SourceService::parse(TEST_SOURCE_SERVICE)?,
            SourceComponent::parse(TEST_SOURCE_COMPONENT)?,
            RuntimeInstanceId::parse(TEST_RUNTIME_INSTANCE)?,
        ),
        RecordedAt::parse(TEST_OBSERVED_AT)?,
        Some(TargetHandler::parse(TEST_TARGET)?),
    ))
}
