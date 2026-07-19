use enforcer_domain::events_types::{
    CausationId, CorrelationId, EventId, EventNamespace, EventType, JournalHash, RequestId,
    RuntimeInstanceId, SchemaVersion, SourceComponent, SourceService, SubscriberId, TargetHandler,
};

#[test]
fn event_type_and_namespace_reject_empty_or_malformed_taxonomy() {
    assert_decode_path(EventType::parse(" "), "event_type");
    assert_decode_path(EventType::parse(".tracking.location"), "event_type");
    assert_decode_path(
        EventNamespace::parse("tracking..location"),
        "event_namespace",
    );
}

#[test]
fn event_namespace_matches_exact_and_child_event_types_only(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let namespace = EventNamespace::parse("tracking")?;
    let exact = EventType::parse("tracking")?;
    let child = EventType::parse(&format!("{}.{}", namespace.as_str(), "location.observed"))?;
    let sibling = EventType::parse("tracking-location.observed")?;

    assert_eq!(namespace.as_str(), "tracking");
    assert_eq!(EventNamespace::from_event_type(&exact)?, namespace);
    assert_eq!(EventNamespace::from_event_type(&child)?, namespace);
    assert_ne!(EventNamespace::from_event_type(&sibling)?, namespace);
    Ok(())
}

#[test]
fn schema_version_rejects_zero_and_preserves_nonzero_value(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    assert!(std::num::NonZeroU16::new(0).is_none());
    assert_eq!(
        SchemaVersion::try_new(
            std::num::NonZeroU16::new(3).ok_or("schema version must be positive")?,
        )
        .as_nonzero()
        .get(),
        3
    );
    Ok(())
}

#[test]
fn strong_identifier_wrappers_accept_existing_lineage_and_hash_values(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    assert_eq!(EventId::parse("event-parity-1")?.as_str(), "event-parity-1");
    assert_eq!(
        CorrelationId::parse("correlation-parity-1")?.as_str(),
        "correlation-parity-1"
    );
    assert_eq!(
        CausationId::parse("causation-test-1")?.as_str(),
        "causation-test-1"
    );
    assert_eq!(
        RequestId::parse("request-a-0000")?.as_str(),
        "request-a-0000"
    );
    assert_eq!(
        JournalHash::parse("journal-hash-parity-1")?.as_str(),
        "journal-hash-parity-1"
    );
    assert_eq!(
        SubscriberId::parse("subscriber-parity-1")?.as_str(),
        "subscriber-parity-1"
    );
    Ok(())
}

#[test]
fn strong_identifier_wrappers_reject_whitespace_values() {
    assert_invalid(&EventId::parse(" event-parity-1"));
    assert_invalid(&CorrelationId::parse("correlation parity 1"));
    assert_invalid(&CausationId::parse("causation-test-1\t"));
    assert_invalid(&RequestId::parse("request-\nparity-1"));
    assert_invalid(&JournalHash::parse("journal hash parity 1"));
}

fn existing_repo_routing_values() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let values = vec![
        SubscriberId::parse("subscriber.app.observer")?
            .as_str()
            .to_owned(),
        SubscriberId::parse("subscriber.child-policy.evaluator")?
            .as_str()
            .to_owned(),
        TargetHandler::parse("target.child-domain.observer")?
            .as_str()
            .to_owned(),
        TargetHandler::parse("child-runtime.tracking")?
            .as_str()
            .to_owned(),
        SourceService::parse("eventing-integration-service")?
            .as_str()
            .to_owned(),
        SourceComponent::parse("eventing-integration-component")?
            .as_str()
            .to_owned(),
        RuntimeInstanceId::parse("eventing-integration-runtime")?
            .as_str()
            .to_owned(),
    ];
    Ok(values)
}

#[test]
fn routing_and_source_wrappers_accept_existing_repo_values() {
    assert_eq!(
        existing_repo_routing_values().map_err(|error| error.to_string()),
        Ok(vec![
            String::from("subscriber.app.observer"),
            String::from("subscriber.child-policy.evaluator"),
            String::from("target.child-domain.observer"),
            String::from("child-runtime.tracking"),
            String::from("eventing-integration-service"),
            String::from("eventing-integration-component"),
            String::from("eventing-integration-runtime"),
        ])
    );
}

#[test]
fn routing_and_source_wrappers_reject_whitespace_values() {
    assert!(matches!(
        SubscriberId::parse("subscriber app observer"),
        Err(error) if !error.path.is_empty() && !error.reason.is_empty()
    ));
    assert!(matches!(
        TargetHandler::parse("target.child-domain observer"),
        Err(error) if !error.path.is_empty() && !error.reason.is_empty()
    ));
    assert!(matches!(
        SourceService::parse("eventing integration service"),
        Err(error) if !error.path.is_empty() && !error.reason.is_empty()
    ));
    assert!(matches!(
        SourceComponent::parse("eventing-integration-component "),
        Err(error) if !error.path.is_empty() && !error.reason.is_empty()
    ));
    assert!(matches!(
        RuntimeInstanceId::parse("\teventing-integration-runtime"),
        Err(error) if !error.path.is_empty() && !error.reason.is_empty()
    ));
}

fn assert_invalid<T>(result: &Result<T, enforcer_domain::boundary::decode_error::DecodeError>) {
    assert!(matches!(result, Err(error) if !error.path.is_empty() && !error.reason.is_empty()));
}

fn assert_decode_path<T>(
    result: Result<T, enforcer_domain::boundary::decode_error::DecodeError>,
    path: &str,
) {
    assert!(matches!(result, Err(error) if error.path == path && !error.reason.is_empty()));
}
