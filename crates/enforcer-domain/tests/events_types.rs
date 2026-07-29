use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::events_types::EventType;
use enforcer_domain::events_types::{
    DeadLetterReason, EventActivityState, EventCompatibilityNote, EventDuration, EventErrorField,
    EventErrorPath, EventErrorReason, EventMatchState, EventProofArtifact, EventSemanticId,
    EventSourceSemantic, HandlerOutcome, JournalAppendDecision, JournalLine, JournalPath,
    JournalRecoveryState, QueueExpirationState, QueueIdempotencyState, RenderedMarkdown,
    RequestCompletionOutcome, RequestPublishState, RustTypeName,
};
use proptest::proptest;

fn assert_rejected(result: Result<(), DecodeError>) -> Result<(), DecodeError> {
    match result {
        Err(error) => {
            assert_ne!(error.reason, "");
            Ok(())
        }
        Ok(()) => Err(DecodeError::new(
            "eventCompatibilityText",
            "expected blank text to be rejected",
        )),
    }
}

proptest! {
    #[test]
    fn event_type_parse_accepts_generated_taxonomy(raw in "[a-z][a-z0-9_]{0,23}(\\.[a-z][a-z0-9_]{0,23}){1,3}") {
        let parsed = EventType::parse(&raw);
        assert_eq!(parsed.map(|value| value.as_str().to_owned()), Ok(raw));
    }
}

#[test]
fn diagnostic_event_error_brands_preserve_nonblank_values() {
    assert_eq!(
        EventErrorReason::from_diagnostic(String::from("dispatch failed")).as_str(),
        "dispatch failed"
    );
    assert_eq!(
        EventErrorPath::from_diagnostic(String::from("journal/append")).as_str(),
        "journal/append"
    );
}

#[test]
fn diagnostic_event_error_brands_normalize_blank_values() {
    let reason = EventErrorReason::from_diagnostic(String::from("  "));
    let path = EventErrorPath::from_diagnostic(String::new());

    assert!(!reason.as_str().trim().is_empty());
    assert!(!path.as_str().trim().is_empty());
    assert_eq!(reason.as_str(), "unspecified event error");
    assert_eq!(path.as_str(), "unknown event error path");
}

#[test]
fn event_compatibility_text_brands_preserve_nonblank_values() -> Result<(), DecodeError> {
    assert_eq!(
        EventSemanticId::try_new(String::from("match.started"))?.as_str(),
        "match.started"
    );
    assert_eq!(
        EventSourceSemantic::try_new(String::from("runtime event"))?.as_str(),
        "runtime event"
    );
    assert_eq!(
        RustTypeName::try_new(String::from("MatchStarted"))?.as_str(),
        "MatchStarted"
    );
    assert_eq!(
        EventProofArtifact::try_new(String::from("tests/fixtures/events.json"))?.as_str(),
        "tests/fixtures/events.json"
    );
    assert_eq!(
        EventCompatibilityNote::try_new(String::from("wire-compatible"))?.as_str(),
        "wire-compatible"
    );
    assert_eq!(
        RenderedMarkdown::try_new(String::from("| event | status |"))?.as_str(),
        "| event | status |"
    );
    Ok(())
}

#[test]
fn event_compatibility_text_brands_reject_blank_values() -> Result<(), DecodeError> {
    for rejection in [
        EventSemanticId::try_new(String::from(" ")).map(|_| ()),
        EventSourceSemantic::try_new(String::from(" ")).map(|_| ()),
        RustTypeName::try_new(String::from(" ")).map(|_| ()),
        EventProofArtifact::try_new(String::from(" ")).map(|_| ()),
        EventCompatibilityNote::try_new(String::from(" ")).map(|_| ()),
        RenderedMarkdown::try_new(String::from(" ")).map(|_| ()),
    ] {
        assert_rejected(rejection)?;
    }
    Ok(())
}

#[test]
fn event_runtime_state_types_keep_distinct_decisions() {
    let branded = EventDuration::try_new_millis(std::num::NonZeroU64::MIN);
    assert_ne!(EventDuration::ZERO, branded);
    assert_ne!(EventActivityState::Active, EventActivityState::Inactive);
    assert_ne!(JournalAppendDecision::Append, JournalAppendDecision::Skip);
    assert_ne!(
        QueueIdempotencyState::Enabled,
        QueueIdempotencyState::Disabled
    );
    assert_ne!(QueueExpirationState::Expired, QueueExpirationState::Current);
    assert_ne!(EventMatchState::Matches, EventMatchState::DoesNotMatch);
    assert_ne!(RequestPublishState::Complete, RequestPublishState::Pending);
    assert_ne!(
        JournalRecoveryState::Recovered,
        JournalRecoveryState::Unrecovered
    );
    assert_ne!(
        RequestCompletionOutcome::Completed,
        RequestCompletionOutcome::Late
    );
    assert_ne!(HandlerOutcome::Handled, HandlerOutcome::Failed);
    assert_ne!(
        DeadLetterReason::HandlerFailed,
        DeadLetterReason::HandlerTimedOut
    );
}

#[test]
fn dead_letter_reason_keeps_stable_wire_tokens() -> Result<(), serde_json::Error> {
    let encoded = serde_json::to_string(&DeadLetterReason::HandlerDeadlineExpired)?;
    assert_eq!(encoded, "\"handler-deadline-expired\"");
    let decoded: DeadLetterReason = serde_json::from_str("\"queue-overflow\"")?;
    assert_eq!(decoded, DeadLetterReason::QueueOverflow);
    let error = match serde_json::from_str::<DeadLetterReason>("\"unsupported\"") {
        Err(error) => error,
        Ok(_) => {
            return Err(serde_json::Error::io(std::io::Error::other(
                "unsupported dead-letter reason was accepted",
            )))
        }
    };
    assert_eq!(error.classify(), serde_json::error::Category::Data);
    assert_eq!(error.line(), 0);
    assert_eq!(error.column(), 0);
    assert_eq!(
        error.to_string(),
        "unknown variant `unsupported`, expected one of `handler-failed`, `handler-timed-out`, `handler-deadline-expired`, `handler-panicked`, `no-subscriber`, `queue-overflow`, `queue-expired`, `deadline-expired`, `shutdown`"
    );
    Ok(())
}

#[test]
fn journal_and_error_field_brands_validate_boundary_text() -> Result<(), DecodeError> {
    assert_eq!(
        JournalLine::try_new(String::from("{\"seq\":1}"))?.as_str(),
        "{\"seq\":1}"
    );
    assert_eq!(
        JournalPath::try_new(String::from("events/journal.ndjson"))?.as_str(),
        "events/journal.ndjson"
    );
    assert_eq!(
        EventErrorField::try_new(String::from("event_type"))?.as_str(),
        "event_type"
    );
    assert_rejected(JournalLine::try_new(String::from(" ")).map(|_| ()))?;
    assert_rejected(JournalPath::try_new(String::from(" ")).map(|_| ()))?;
    assert_rejected(EventErrorField::try_new(String::from("has space")).map(|_| ()))?;
    assert_eq!(
        EventErrorField::from_diagnostic(String::from("valid_field")).as_str(),
        "valid_field"
    );
    assert_eq!(
        EventErrorField::from_diagnostic(String::from("has space")).as_str(),
        "decoded_value"
    );
    assert_eq!(
        EventErrorField::from_diagnostic(String::new()).as_str(),
        "decoded_value"
    );
    Ok(())
}

#[test]
fn diagnostic_journal_brands_preserve_nonblank_values() {
    assert_eq!(
        JournalPath::from_diagnostic(String::from("events/journal.ndjson")).as_str(),
        "events/journal.ndjson"
    );
    assert_eq!(
        JournalLine::from_diagnostic(String::from("{\"seq\":1}")).as_str(),
        "{\"seq\":1}"
    );
}

#[test]
fn diagnostic_journal_brands_normalize_blank_values() {
    assert_eq!(
        JournalPath::from_diagnostic(String::from("  ")).as_str(),
        "unknown journal path"
    );
    assert_eq!(
        JournalLine::from_diagnostic(String::new()).as_str(),
        "unavailable journal line"
    );
}
