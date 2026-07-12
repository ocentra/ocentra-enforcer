//! BOUNDARY parser for recorded observability reports (OpenTelemetry
//! span/event captures over money paths and security events).
//!
//! BOUNDARY-INVARIANT: [`parse_recorded`] accepts raw recorded JSON and
//! either returns a fully branded
//! [`crate::security_pipeline::observability::ObservabilityOutcome`] or
//! rejects the text as malformed/dishonest with a typed decode failure.
//! Event labels are rejected when empty; correlation ids are validated
//! into the `enforcer_domain` `CorrelationId` brand here, never carried
//! as raw text.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! the shared honesty check this parser applies.

use enforcer_core::error::{DecodeError, Result};
use enforcer_domain::ids::CorrelationId;

use crate::security_pipeline::observability::{
    EventKind, MoneyPathClass, ObservabilityEvent, ObservabilityOutcome, SamplingDisposition,
    SecurityLogPresence,
};
use crate::security_pipeline::seam::EventLabel;

/// Raw wire shape of one recorded observability report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObservabilityRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
    // DEFAULT-JUSTIFICATION: a skipped/errored report legitimately omits
    // the events array; an absent array means "no events were captured".
    #[serde(default)]
    events: Vec<EventRecord>,
}

/// Raw wire shape of one captured event inside a report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventRecord {
    event_id: String,
    money_critical: bool,
    has_security_log: bool,
    correlation_id: Option<String>,
    is_security_event: bool,
    sampled_dropped: bool,
}

/// Parse one recorded observability report into a branded
/// [`ObservabilityOutcome`], rejecting malformed JSON, dishonest shapes,
/// empty event labels, and invalid correlation ids.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant.
pub fn parse_recorded(raw: &str) -> Result<ObservabilityOutcome> {
    let record: ObservabilityRecord = serde_json::from_str(raw).map_err(|source| {
        DecodeError::new("securityPipeline.observability", format!("{source}"))
    })?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(ObservabilityOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(ObservabilityOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(ObservabilityOutcome::Ran {
            ran: record.ran,
            events: record
                .events
                .into_iter()
                .map(event_from_record)
                .collect::<Result<Vec<ObservabilityEvent>>>()?,
        }),
        other => Err(DecodeError::new(
            "securityPipeline.observability.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}

/// Validate one raw captured event into its branded form.
fn event_from_record(record: EventRecord) -> Result<ObservabilityEvent> {
    if record.event_id.trim().is_empty() {
        return Err(DecodeError::new(
            "securityPipeline.observability.eventId",
            "event label must not be empty — every captured event must be nameable",
        )
        .into());
    }
    let correlation = match record.correlation_id {
        None => None,
        Some(raw_id) => Some(raw_id.parse::<CorrelationId>()?),
    };
    Ok(ObservabilityEvent {
        label: EventLabel(record.event_id),
        money_class: if record.money_critical {
            MoneyPathClass::MoneyCritical
        } else {
            MoneyPathClass::Ordinary
        },
        security_log: if record.has_security_log {
            SecurityLogPresence::Emitted
        } else {
            SecurityLogPresence::Missing
        },
        correlation,
        kind: if record.is_security_event {
            EventKind::SecurityEvent
        } else {
            EventKind::MoneyPathSpan
        },
        sampling: if record.sampled_dropped {
            SamplingDisposition::DroppedBySampling
        } else {
            SamplingDisposition::Recorded
        },
    })
}
