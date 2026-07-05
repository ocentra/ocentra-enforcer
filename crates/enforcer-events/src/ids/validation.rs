use crate::EventingError;

use super::{
    EventNamespace, CAUSATION_ID_LABEL, CORRELATION_ID_LABEL, EVENT_ID_LABEL,
    EVENT_NAMESPACE_LABEL, EVENT_TYPE_LABEL, JOURNAL_HASH_LABEL, REQUEST_ID_LABEL,
    RUNTIME_INSTANCE_ID_LABEL, SOURCE_COMPONENT_LABEL, SOURCE_SERVICE_LABEL, SUBSCRIBER_ID_LABEL,
    TARGET_HANDLER_LABEL,
};

pub(super) fn event_namespace_from_event_type(
    event_type: &super::EventType,
) -> Result<EventNamespace, EventingError> {
    let namespace = event_type
        .as_str()
        .split(['.', '/'])
        .next()
        .ok_or_else(|| EventingError::empty_value(EVENT_NAMESPACE_LABEL))?;
    EventNamespace::parse(namespace)
}

pub(super) fn event_namespace_matches_event_type(
    namespace: &EventNamespace,
    event_type: &super::EventType,
) -> bool {
    event_type.as_str() == namespace.as_str()
        || event_type
            .as_str()
            .strip_prefix(namespace.as_str())
            .is_some_and(|suffix| suffix.starts_with(['.', '/']))
}

pub(super) fn validate_text(field: &'static str, value: String) -> Result<String, EventingError> {
    if value.trim().is_empty() {
        return Err(EventingError::empty_value(field));
    }
    match field {
        EVENT_TYPE_LABEL | EVENT_NAMESPACE_LABEL => validate_event_taxonomy(field, &value)?,
        EVENT_ID_LABEL
        | CORRELATION_ID_LABEL
        | CAUSATION_ID_LABEL
        | REQUEST_ID_LABEL
        | JOURNAL_HASH_LABEL
        | SUBSCRIBER_ID_LABEL
        | SOURCE_SERVICE_LABEL
        | SOURCE_COMPONENT_LABEL
        | RUNTIME_INSTANCE_ID_LABEL
        | TARGET_HANDLER_LABEL => validate_identifier_without_whitespace(field, &value)?,
        _ => {}
    }
    Ok(value)
}

fn validate_event_taxonomy(field: &'static str, value: &str) -> Result<(), EventingError> {
    let mut previous_was_separator = false;
    for (index, character) in value.chars().enumerate() {
        let is_separator = matches!(character, '.' | '/');
        let is_valid =
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-') || is_separator;
        if !is_valid || (is_separator && (index == 0 || previous_was_separator)) {
            return Err(EventingError::invalid_value(field, value));
        }
        previous_was_separator = is_separator;
    }
    if previous_was_separator {
        return Err(EventingError::invalid_value(field, value));
    }
    Ok(())
}

fn validate_identifier_without_whitespace(
    field: &'static str,
    value: &str,
) -> Result<(), EventingError> {
    if value.chars().any(char::is_whitespace) {
        return Err(EventingError::invalid_value(field, value));
    }
    Ok(())
}
