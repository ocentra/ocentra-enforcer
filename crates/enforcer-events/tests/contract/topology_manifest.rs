use super::support::{test_event_for_type, TestText, OTHER_EVENT_TYPE, TEST_EVENT_TYPE};
use enforcer_domain::events_types::{
    EventNamespace, EventTopologyStatus, EventType, SourceComponent, SubscriberId, TargetHandler,
};
use enforcer_events::boundary::topology_presentation::EventTopologyManifestResponse;
use enforcer_events::boundary::topology_contract_presentation::{
    EventTopologyContractResponse, EventTopologySubscriberTargetResponse,
};
use enforcer_events::boundary::request_persistence::RequestCompletionReportResponse;
use enforcer_events::contract_registry::EventContractRegistry;
use enforcer_events::envelope::EventContract;
use enforcer_events::request::RequestCompletionReport;
use enforcer_events::topology::{
    EventTopologyEntry, EventTopologyFamilyVariant, EventTopologyManifest, EventTopologyPublisher,
    EventTopologySubscriber, EventTopologySubscriberTarget,
};
use serde_json::Value;

const NO_SUBSCRIBER_EVENT_TYPE: &str = "eventing.topology.no_subscriber";
const ACCEPTED_NO_PUBLISHER_EVENT_TYPE: &str = "eventing.topology.accepted_no_publisher";
const COVERED_PUBLISHER: &str = "covered-publisher";
const ORPHAN_PUBLISHER: &str = "orphan-publisher";
const COVERED_SUBSCRIBER: &str = "covered-subscriber";
const ACCEPTED_SUBSCRIBER: &str = "accepted-subscriber";
const TOPOLOGY_TARGET: &str = "topology-target";
const FAMILY_ID: &str = "eventing.topology.family";

#[test]
fn topology_manifest_classifies_covered_orphan_and_accepted_states(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = topology_registry()?;
    let accepted_event = EventType::parse(ACCEPTED_NO_PUBLISHER_EVENT_TYPE)?;

    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[
            publisher(
                TestText(TEST_EVENT_TYPE.to_owned()),
                TestText(COVERED_PUBLISHER.to_owned()),
            )?,
            publisher(
                TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()),
                TestText(ORPHAN_PUBLISHER.to_owned()),
            )?,
        ],
        &[
            subscriber(
                TestText(TEST_EVENT_TYPE.to_owned()),
                TestText(COVERED_SUBSCRIBER.to_owned()),
            )?,
            subscriber(
                TestText(ACCEPTED_NO_PUBLISHER_EVENT_TYPE.to_owned()),
                TestText(ACCEPTED_SUBSCRIBER.to_owned()),
            )?,
        ],
        &[
            family_variant(TestText(TEST_EVENT_TYPE.to_owned()))?,
            family_variant(TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()))?,
        ],
        &[accepted_event],
    );

    assert_eq!(
        entry(&manifest, TestText(TEST_EVENT_TYPE.to_owned()))?.status,
        EventTopologyStatus::Covered
    );
    assert_eq!(
        entry(&manifest, TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()))?.status,
        EventTopologyStatus::NoSubscriber
    );
    assert_eq!(
        entry(&manifest, TestText(OTHER_EVENT_TYPE.to_owned()))?.status,
        EventTopologyStatus::NoPublisher
    );
    assert_eq!(
        entry(
            &manifest,
            TestText(ACCEPTED_NO_PUBLISHER_EVENT_TYPE.to_owned())
        )?
        .status,
        EventTopologyStatus::AcceptedOneSided
    );
    assert_eq!(
        manifest
            .unready_entries()
            .iter()
            .map(|entry| entry.contract.event_type.as_str())
            .collect::<Vec<_>>(),
        vec![OTHER_EVENT_TYPE, NO_SUBSCRIBER_EVENT_TYPE]
    );
    Ok(())
}

#[test]
fn topology_manifest_records_family_variants_and_sorted_descriptors(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = topology_registry()?;
    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[publisher(
            TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()),
            TestText(ORPHAN_PUBLISHER.to_owned()),
        )?],
        &[],
        &[
            family_variant(TestText(TEST_EVENT_TYPE.to_owned()))?,
            family_variant(TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()))?,
        ],
        &[],
    );

    let event_types = manifest
        .entries()
        .iter()
        .map(|entry| entry.contract.event_type.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        vec![
            TEST_EVENT_TYPE,
            OTHER_EVENT_TYPE,
            ACCEPTED_NO_PUBLISHER_EVENT_TYPE,
            NO_SUBSCRIBER_EVENT_TYPE
        ]
    );
    assert_eq!(
        entry(&manifest, TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()))?.families[0].as_str(),
        FAMILY_ID
    );
    Ok(())
}

#[test]
fn topology_manifest_renders_deterministic_markdown(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = topology_registry()?;
    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[publisher(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_PUBLISHER.to_owned()),
        )?],
        &[subscriber(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_SUBSCRIBER.to_owned()),
        )?],
        &[family_variant(TestText(TEST_EVENT_TYPE.to_owned()))?],
        &[],
    );

    let markdown = manifest.render_markdown();
    let lines = markdown.lines().collect::<Vec<_>>();
    assert_eq!(lines.first().copied(), Some("# Event Topology Manifest"));
    assert!(lines.iter().any(|line| {
        *line == "| Event Type | Schema Version | Publishers | Subscribers | Families | Status | Rust Type |"
    }));
    assert!(lines.iter().any(|line| {
        *line
            == "| eventing.test.observed | 1 | covered-publisher | covered-subscriber -> topology-target | eventing.topology.family | covered | contract::support::TestEvent |"
    }));
    assert!(lines.iter().any(|line| {
        *line
            == "| eventing.test.other | 1 | none | none | none | no-publisher | contract::support::TestEvent |"
    }));
    Ok(())
}

#[test]
fn topology_manifest_serializes_canonical_eventing_entry_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = topology_registry()?;
    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[publisher(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_PUBLISHER.to_owned()),
        )?],
        &[subscriber(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_SUBSCRIBER.to_owned()),
        )?],
        &[family_variant(TestText(TEST_EVENT_TYPE.to_owned()))?],
        &[],
    );

    let manifest_json = serde_json::to_value(manifest.presentation())?;
    let entry = manifest_entry(&manifest_json, TestText(TEST_EVENT_TYPE.to_owned()))?
        .as_object()
        .ok_or("manifest entry")?;
    let subscriber_target = entry["subscribers"][0]
        .as_object()
        .ok_or("subscriber target object")?;

    assert_eq!(entry["contract"]["eventType"], Value::from(TEST_EVENT_TYPE));
    assert_eq!(entry["contract"]["schemaVersion"], Value::from(1));
    assert_eq!(
        entry["rustType"],
        Value::from("contract::support::TestEvent")
    );
    assert!(entry.get("rust_type").is_none());
    assert_eq!(
        subscriber_target.get("subscriberId"),
        Some(&Value::from(COVERED_SUBSCRIBER))
    );
    assert_eq!(
        subscriber_target.get("targetHandler"),
        Some(&Value::from(TOPOLOGY_TARGET))
    );

    let response = manifest.presentation();
    let recovered = EventTopologyManifest::try_from(response.clone())?;
    assert_eq!(recovered.presentation(), response);
    Ok(())
}

#[test]
fn topology_presentation_rejects_invalid_contract_and_subscriber_values(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = topology_registry()?;
    let manifest = EventTopologyManifest::from_registry(
        &registry,
        &[publisher(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_PUBLISHER.to_owned()),
        )?],
        &[subscriber(
            TestText(TEST_EVENT_TYPE.to_owned()),
            TestText(COVERED_SUBSCRIBER.to_owned()),
        )?],
        &[],
        &[],
    );
    let invalid_contract = EventContract::try_from(EventTopologyContractResponse {
        event_type: String::new(),
        schema_version: 1,
    });
    assert!(matches!(
        invalid_contract,
        Err(enforcer_events::error::EventingError::InvalidValue { field, .. })
            if field.as_str() == "decoded_value"
    ));

    let mut invalid: EventTopologyManifestResponse = manifest.presentation();
    invalid.entries[0].subscribers[0].target_handler = "invalid handler".to_owned();
    let invalid_subscriber = EventTopologySubscriberTarget::try_from(
        EventTopologySubscriberTargetResponse {
            subscriber_id: COVERED_SUBSCRIBER.to_owned(),
            target_handler: "invalid handler".to_owned(),
        },
    );
    assert!(matches!(
        invalid_subscriber,
        Err(enforcer_events::error::EventingError::InvalidValue { field, .. })
            if field.as_str() == "decoded_value"
    ));

    let invalid_entry: enforcer_events::boundary::topology_presentation::EventTopologyEntryResponse = invalid.entries.remove(0);
    let invalid_entry = EventTopologyEntry::try_from(invalid_entry);
    assert!(matches!(
        invalid_entry,
        Err(enforcer_events::error::EventingError::InvalidValue { field, .. })
            if field.as_str() == "decoded_value"
    ));

    let mut invalid_manifest: EventTopologyManifestResponse = manifest.presentation();
    invalid_manifest.entries[0].contract.event_type.clear();
    let invalid_manifest = EventTopologyManifest::try_from(invalid_manifest);
    assert!(matches!(
        invalid_manifest,
        Err(enforcer_events::error::EventingError::InvalidValue { field, .. })
            if field.as_str() == "decoded_value"
    ));

    assert_eq!(
        RequestCompletionReport::try_from(RequestCompletionReportResponse {
            request_id: "request-1".to_owned(),
            outcome: "invalid".to_owned(),
        }),
        Err(enforcer_events::error::EventingError::InvalidValue {
            field: enforcer_domain::events_types::EventErrorField::parse(
                "request_completion_outcome",
            )?,
            value: enforcer_domain::events_types::EventErrorReason::parse("invalid")?,
        })
    );
    Ok(())
}

fn topology_registry() -> Result<EventContractRegistry, Box<dyn std::error::Error + Send + Sync>> {
    let mut registry = EventContractRegistry::new();
    registry.register_event(&test_event_for_type(
        &TestText("covered".to_owned()),
        &TestText(TEST_EVENT_TYPE.to_owned()),
    )?)?;
    registry.register_event(&test_event_for_type(
        &TestText("other".to_owned()),
        &TestText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;
    registry.register_event(&test_event_for_type(
        &TestText("accepted".to_owned()),
        &TestText(ACCEPTED_NO_PUBLISHER_EVENT_TYPE.to_owned()),
    )?)?;
    registry.register_event(&test_event_for_type(
        &TestText("orphan".to_owned()),
        &TestText(NO_SUBSCRIBER_EVENT_TYPE.to_owned()),
    )?)?;
    Ok(registry)
}

fn publisher(
    event_type: TestText,
    component: TestText,
) -> Result<EventTopologyPublisher, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventTopologyPublisher {
        event_type: EventType::parse(&{ event_type.0 })?,
        source_component: SourceComponent::parse(&{ component.0 })?,
    })
}

fn subscriber(
    event_type: TestText,
    subscriber_id: TestText,
) -> Result<EventTopologySubscriber, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventTopologySubscriber {
        event_type: EventType::parse(&{ event_type.0 })?,
        subscriber_id: SubscriberId::parse(&{ subscriber_id.0 })?,
        target_handler: TargetHandler::parse(TOPOLOGY_TARGET)?,
    })
}

fn family_variant(
    event_type: TestText,
) -> Result<EventTopologyFamilyVariant, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventTopologyFamilyVariant {
        family: EventNamespace::parse(FAMILY_ID)?,
        event_type: EventType::parse(&{ event_type.0 })?,
    })
}

fn entry(
    manifest: &EventTopologyManifest,
    event_type: TestText,
) -> Result<&EventTopologyEntry, Box<dyn std::error::Error + Send + Sync>> {
    let event_type = event_type.0;
    manifest
        .entries()
        .iter()
        .find(|entry| entry.contract.event_type.as_str() == event_type)
        .ok_or_else(|| "topology entry exists".into())
}

fn manifest_entry(
    manifest_json: &Value,
    event_type: TestText,
) -> Result<&Value, Box<dyn std::error::Error + Send + Sync>> {
    let event_type = event_type.0;
    manifest_json["entries"]
        .as_array()
        .ok_or("entries array")?
        .iter()
        .find(|entry| entry["contract"]["eventType"] == event_type)
        .ok_or_else(|| "manifest entry exists".into())
}
