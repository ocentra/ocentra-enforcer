use enforcer_domain::events_types::{
    AggregateKey, CorrelationId, EventCustody, EventId, EventType, IdempotencyKey, RecordedAt,
    RequestCompletionOutcome, RequestId, RuntimeInstanceId, RuntimeRole, SchemaVersion,
    SourceComponent, SourceService, TargetHandler,
};
use enforcer_events::boundary::event_contract_persistence::{EventContractDto, EventSourceDto};
use enforcer_events::boundary::event_metadata_persistence::{EventEnvelopeDto, EventMetadataDto};
use enforcer_events::boundary::stored_event_persistence::{
    StoredEventEnvelope, StoredEventEnvelopeDto, StoredEventPayloadDto,
};
use enforcer_events::envelope::{
    DomainEvent, EventContract, EventFrame, EventMetadata, EventSource,
};
use enforcer_events::error::EventingError;
use enforcer_events::request::RequestCompletionReport;
use serde::{de::Error as _, Deserialize, Serialize};
use serde_json::json;

const TEST_EVENT_TYPE: &str = "eventing.unit.contract-boundary";
const TEST_EVENT_ID: &str = "eventing-unit-envelope-id";
const TEST_CORRELATION_ID: &str = "eventing-unit-envelope-correlation";
const TEST_AGGREGATE_KEY: &str = "eventing-unit-envelope-aggregate";
const TEST_IDEMPOTENCY_KEY: &str = "eventing-unit-envelope-idempotency";
const TEST_CUSTODY: &str = "local-only";
const TEST_RUNTIME_ROLE: &str = "parent";
const TEST_SOURCE_SERVICE: &str = "eventing-unit-envelope-service";
const TEST_SOURCE_COMPONENT: &str = "eventing-unit-envelope-component";
const TEST_RUNTIME_INSTANCE: &str = "eventing-unit-envelope-runtime";
const TEST_TARGET: &str = "eventing-unit-envelope-target";
const TEST_OBSERVED_AT: &str = "2026-06-13T20:15:00Z";
const OTHER_EVENT_TYPE: &str = "eventing.unit.contract-boundary.other";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct EnvelopeBoundaryEvent {
    label: String,
}

impl DomainEvent for EnvelopeBoundaryEvent {
    fn contract(&self) -> Result<EventContract, EventingError> {
        Ok(EventContract::new(
            EventType::parse(TEST_EVENT_TYPE)?,
            SchemaVersion::try_new(std::num::NonZeroU16::MIN),
        ))
    }

    fn aggregate_key(&self) -> Result<AggregateKey, EventingError> {
        Ok(AggregateKey::parse(TEST_AGGREGATE_KEY)?)
    }

    fn idempotency_key(&self) -> Result<IdempotencyKey, EventingError> {
        Ok(IdempotencyKey::parse(TEST_IDEMPOTENCY_KEY)?)
    }
}

fn assert_json_round_trip<T>(original: &T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let wire = serde_json::to_string(&original)?;
    let decoded: T = serde_json::from_str(&wire)?;
    assert_eq!(&decoded, original);
    Ok(())
}

#[test]
fn event_contract_serde_rejects_zero_schema_version(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let result = serde_json::from_value::<
        enforcer_events::boundary::event_contract_persistence::EventContractDto,
    >(json!({
        "eventType": TEST_EVENT_TYPE,
        "schemaVersion": 0
    }))
    .and_then(|wire| EventContract::try_from(wire).map_err(serde_json::Error::custom));

    let Err(error) = result else {
        return Err("expected zero schema version to be rejected on decode".into());
    };
    assert!(error
        .to_string()
        .contains("event schema version must be nonzero"));
    Ok(())
}

#[test]
fn persistence_dto_round_trips_preserve_typed_event_boundary_values(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventFrame::from_event(
        EnvelopeBoundaryEvent {
            label: String::from("typed-boundary"),
        },
        metadata()?,
    )?;
    let stored = live.store()?;

    assert_json_round_trip::<EventContractDto>(&EventContractDto::from(&live.contract))?;
    assert_json_round_trip::<EventSourceDto>(&EventSourceDto::from(&live.source))?;
    assert_json_round_trip::<EventMetadataDto>(&EventMetadataDto::from(&metadata()?))?;
    let original_envelope: EventEnvelopeDto<EnvelopeBoundaryEvent> = EventEnvelopeDto::from(&live);
    let envelope_wire = serde_json::to_string(&original_envelope)?;
    let decoded_envelope: EventEnvelopeDto<EnvelopeBoundaryEvent> =
        serde_json::from_str(&envelope_wire)?;
    assert_eq!(decoded_envelope, original_envelope);
    assert_json_round_trip::<StoredEventPayloadDto>(
        &StoredEventEnvelopeDto::from(&stored).0.payload,
    )?;
    assert_json_round_trip::<StoredEventEnvelopeDto>(&StoredEventEnvelopeDto::from(&stored))?;
    Ok(())
}

#[test]
fn persistence_dto_conversions_reject_invalid_domain_values(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    assert!(EventContract::try_from(EventContractDto {
        event_type: TEST_EVENT_TYPE.to_owned(),
        schema_version: 0,
    })
    .is_err());

    assert!(EventSource::try_from(EventSourceDto {
        custody: String::new(),
        role: TEST_RUNTIME_ROLE.to_owned(),
        service: TEST_SOURCE_SERVICE.to_owned(),
        component: TEST_SOURCE_COMPONENT.to_owned(),
        instance_id: TEST_RUNTIME_INSTANCE.to_owned(),
    })
    .is_err());

    assert!(EventMetadata::try_from(EventMetadataDto {
        event_id: String::new(),
        correlation_id: TEST_CORRELATION_ID.to_owned(),
        causation_id: None,
        source: EventSourceDto::from(&metadata()?.source),
        observed_at: TEST_OBSERVED_AT.to_owned(),
        target_handler: Some(TEST_TARGET.to_owned()),
        priority: Default::default(),
        deadline: None,
    })
    .is_err());

    let live = EventFrame::from_event(
        EnvelopeBoundaryEvent {
            label: String::from("typed-boundary"),
        },
        metadata()?,
    )?;
    let mut invalid_stored = StoredEventEnvelopeDto::from(&live.store()?);
    invalid_stored.0.event_id.clear();
    assert!(matches!(
        StoredEventEnvelope::try_from(invalid_stored),
        Err(EventingError::InvalidValue { field, value })
            if field.as_str() == "event_id" && value.as_str() == "unspecified event error"
    ));
    Ok(())
}

#[test]
fn stored_envelope_serde_uses_canonical_eventing_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventFrame::from_event(
        EnvelopeBoundaryEvent {
            label: String::from("typed-boundary"),
        },
        metadata()?,
    )?;
    let stored = live.store()?;
    let stored_dto = StoredEventEnvelopeDto::from(&stored);
    let wire = serde_json::to_string(&stored_dto)?;
    let round_trip_stored: StoredEventEnvelopeDto = serde_json::from_str(&wire)?;
    let round_trip_envelope: &EventEnvelopeDto<StoredEventPayloadDto> = &round_trip_stored.0;
    let round_trip_contract: &EventContractDto = &round_trip_envelope.contract;
    let round_trip_source: &EventSourceDto = &round_trip_envelope.source;
    assert_eq!(round_trip_contract.event_type.as_str(), TEST_EVENT_TYPE);
    assert_eq!(
        round_trip_source.instance_id.as_str(),
        TEST_RUNTIME_INSTANCE
    );

    let metadata_dto = EventMetadataDto::from(&metadata()?);
    let metadata_wire = serde_json::to_string(&metadata_dto)?;
    let round_trip_metadata: EventMetadataDto = serde_json::from_str(&metadata_wire)?;
    assert_eq!(round_trip_metadata.event_id.as_str(), TEST_EVENT_ID);

    let stored_json = serde_json::to_value(&round_trip_stored)?;

    assert_eq!(stored_json["contract"]["eventType"], json!(TEST_EVENT_TYPE));
    assert_eq!(stored_json["contract"]["schemaVersion"], json!(1));
    assert_eq!(
        stored_json["source"]["instanceId"],
        json!(TEST_RUNTIME_INSTANCE)
    );
    assert_eq!(stored_json["eventId"], json!(TEST_EVENT_ID));
    assert_eq!(stored_json["correlationId"], json!(TEST_CORRELATION_ID));
    assert_eq!(stored_json["aggregateKey"], json!(TEST_AGGREGATE_KEY));
    assert_eq!(stored_json["idempotencyKey"], json!(TEST_IDEMPOTENCY_KEY));
    assert_eq!(stored_json["observedAt"], json!(TEST_OBSERVED_AT));
    assert_eq!(stored_json["targetHandler"], json!(TEST_TARGET));
    assert!(stored_json.get("event_id").is_none());
    assert!(stored_json["contract"].get("event_type").is_none());
    assert!(stored_json["source"].get("instance_id").is_none());
    Ok(())
}

#[test]
fn request_completion_report_serde_uses_canonical_eventing_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let report = RequestCompletionReport {
        request_id: RequestId::parse("request-completion-1")?,
        outcome: RequestCompletionOutcome::Late,
    };
    let report_json = serde_json::to_value(
        enforcer_events::boundary::request_persistence::RequestCompletionReportResponse::from(
            &report,
        ),
    )?;

    assert_eq!(report_json["requestId"], json!("request-completion-1"));
    assert_eq!(report_json["outcome"], json!("late"));
    assert!(report_json.get("request_id").is_none());
    Ok(())
}

#[test]
fn request_completion_report_response_reconstructs_validated_domain_report(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let report = RequestCompletionReport {
        request_id: RequestId::parse("request-completion-1")?,
        outcome: RequestCompletionOutcome::Late,
    };
    let response =
        enforcer_events::boundary::request_persistence::RequestCompletionReportResponse::from(
            &report,
        );

    assert_eq!(RequestCompletionReport::try_from(response)?, report);
    Ok(())
}

#[test]
fn request_completion_report_response_rejects_unknown_outcome(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let result = RequestCompletionReport::try_from(
        enforcer_events::boundary::request_persistence::RequestCompletionReportResponse {
            request_id: String::from("request-completion-1"),
            outcome: String::from("unexpected"),
        },
    );

    let Err(EventingError::InvalidValue { field, value }) = result else {
        return Err("expected unknown completion outcome to be rejected".into());
    };
    assert_eq!(field.as_str(), "request_completion_outcome");
    assert!(value.as_str().contains("unexpected"));
    Ok(())
}

#[test]
fn live_and_stored_envelopes_preserve_contract_and_metadata(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventFrame::from_event(
        EnvelopeBoundaryEvent {
            label: String::from("typed-boundary"),
        },
        metadata()?,
    )?;
    let stored = live.store()?;
    let decoded: EventFrame<EnvelopeBoundaryEvent> = stored.decode()?;

    assert_eq!(stored.contract.event_type.as_str(), TEST_EVENT_TYPE);
    assert_eq!(stored.contract.schema_version.as_nonzero().get(), 1);
    assert_eq!(stored.event_id.as_str(), TEST_EVENT_ID);
    assert_eq!(stored.correlation_id.as_str(), TEST_CORRELATION_ID);
    assert_eq!(
        stored
            .target_handler
            .as_ref()
            .ok_or("target handler stored")?
            .as_str(),
        TEST_TARGET
    );
    assert_eq!(decoded.payload.label, "typed-boundary");
    assert_eq!(decoded.contract.schema_version.as_nonzero().get(), 1);
    Ok(())
}

#[test]
fn stored_decode_contract_mismatch_reports_event_type_and_schema_version_context(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let live = EventFrame::from_event(
        EnvelopeBoundaryEvent {
            label: String::from("typed-boundary"),
        },
        metadata()?,
    )?;
    let mut stored = live.store()?;
    stored.contract.event_type = EventType::parse(OTHER_EVENT_TYPE)?;
    stored.contract.schema_version =
        SchemaVersion::try_new(std::num::NonZeroU16::new(2).ok_or(EventingError::InvalidVersion)?);

    let error = match stored.decode::<EnvelopeBoundaryEvent>() {
        Err(e) => e,
        Ok(_) => return Err("contract mismatch must fail closed: expected Err but got Ok".into()),
    };

    assert_eq!(
        error,
        EventingError::ContractMismatch {
            expected: EventType::parse(TEST_EVENT_TYPE)?,
            received: EventType::parse(OTHER_EVENT_TYPE)?,
            expected_schema_version: SchemaVersion::try_new(std::num::NonZeroU16::MIN),
            received_schema_version: SchemaVersion::try_new(
                std::num::NonZeroU16::new(2).ok_or(EventingError::InvalidVersion)?,
            ),
        }
    );
    assert_eq!(
        error.to_string(),
        "event contract mismatch: expected eventing.unit.contract-boundary@1, received eventing.unit.contract-boundary.other@2"
    );
    Ok(())
}

fn metadata() -> Result<EventMetadata, Box<dyn std::error::Error + Send + Sync>> {
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
