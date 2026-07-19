use super::support::{
    test_event, test_event_for_type, TestText as SupportText, OTHER_EVENT_TYPE, TEST_EVENT_TYPE,
};
use enforcer_domain::events_types::EventType;
use enforcer_events::contract_registry::EventContractRegistry;
use enforcer_events::error::EventingError;

#[test]
fn contract_registry_generates_markdown_in_event_type_order(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut registry = EventContractRegistry::new();
    registry.register_event(&test_event_for_type(
        &SupportText("second".to_owned()),
        &SupportText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;
    registry.register_event(&test_event(&SupportText("first".to_owned()))?)?;

    let descriptors = registry
        .descriptors()
        .map(|descriptor| descriptor.event_type().as_str().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        descriptors,
        vec![TEST_EVENT_TYPE.to_string(), OTHER_EVENT_TYPE.to_string()]
    );

    let markdown = registry.render_markdown()?.into_markdown();
    let lines = markdown.as_str().lines().collect::<Vec<_>>();
    assert_eq!(lines.first().copied(), Some("# Event Contract Registry"));
    assert_eq!(
        lines.get(2).copied(),
        Some("| Event Type | Schema Version | Rust Type |")
    );
    assert!(lines
        .iter()
        .any(|line| line.starts_with("| eventing.test.observed | 1 |")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("| eventing.test.other | 1 |")));

    let observed_index = markdown
        .as_str()
        .find(TEST_EVENT_TYPE)
        .ok_or("observed event appears in markdown")?;
    let other_index = markdown
        .as_str()
        .find(OTHER_EVENT_TYPE)
        .ok_or("other event appears in markdown")?;
    assert!(observed_index < other_index);
    Ok(())
}

#[test]
fn contract_registry_rejects_duplicate_event_type(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut registry = EventContractRegistry::new();
    registry.register_event(&test_event(&SupportText("first".to_owned()))?)?;

    let duplicate =
        match registry.register_event(&test_event(&SupportText("duplicate".to_owned()))?) {
            Ok(_) => return Err("duplicate event type registration unexpectedly succeeded".into()),
            Err(error) => error,
        };
    assert_eq!(
        duplicate,
        EventingError::DuplicateEventContract {
            event_type: EventType::parse(TEST_EVENT_TYPE)?
        }
    );
    Ok(())
}

#[test]
fn empty_contract_registry_docs_are_explicit(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let markdown = EventContractRegistry::new().render_markdown()?;

    assert_eq!(
        markdown.markdown().as_str(),
        "# Event Contract Registry\n\n_No event contracts registered._\n"
    );
    Ok(())
}
