//! Serde wire DTOs for feedback-decision persistence.
//!
//! Raw strings exist only at this NDJSON/JSON boundary and are converted to
//! `super::FeedbackDecisionRecord` immediately after decoding.
//! NEGATIVE-TEST: this module's tests reject invalid classifications, state
//! mismatches, and unsupported schema versions before domain construction.
//! ROUNDTRIP-TEST: `feedback_decision_dto_round_trip_through_domain_and_json`
//! verifies the bidirectional JSON and domain conversion.

use enforcer_domain::hashes::Sha256;
use enforcer_domain::mechanization_types::{
    ExternalDiagnosticCode, FeedbackDecisionSchemaVersion, FeedbackScaffoldState, FeedbackToolName,
    MechanizationClassification,
};
use std::num::NonZeroU32;

use super::FeedbackDecisionRecord;

const FEEDBACK_DECISION_EVENT_TYPE_WIRE: &str = "harness.feedback.decision";

/// Canonical event identity used by feedback decision records. The raw
/// spelling is private to this persistence boundary and converted before it
/// enters the domain record.
pub(super) fn feedback_decision_event_type() -> Result<
    enforcer_domain::events_types::EventType,
    enforcer_domain::boundary::decode_error::DecodeError,
> {
    FEEDBACK_DECISION_EVENT_TYPE_WIRE.to_owned().try_into()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Persistence DTO for one feedback classification decision.
pub struct FeedbackDecisionDto {
    pub schema_version: u32,
    pub event_type: String,
    pub input_fingerprint: Sha256,
    pub tool: String,
    pub source_rule_id: String,
    pub classification: String,
    pub proposed: bool,
}

impl From<&FeedbackDecisionRecord> for FeedbackDecisionDto {
    fn from(record: &FeedbackDecisionRecord) -> Self {
        Self {
            schema_version: NonZeroU32::MIN.get(),
            event_type: record.event_type().to_string(),
            input_fingerprint: record.input_fingerprint().clone(),
            tool: record.tool().to_string(),
            source_rule_id: record.source_rule_id().to_string(),
            classification: match record.classification() {
                MechanizationClassification::Prevent => "prevent",
                MechanizationClassification::Detect => "detect",
            }
            .to_owned(),
            proposed: match record.scaffold_state() {
                FeedbackScaffoldState::Proposed => true,
                FeedbackScaffoldState::NotProposed => false,
            },
        }
    }
}

impl TryFrom<FeedbackDecisionDto> for FeedbackDecisionRecord {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(wire: FeedbackDecisionDto) -> Result<Self, Self::Error> {
        let schema_version = NonZeroU32::new(wire.schema_version).ok_or_else(|| {
            enforcer_domain::boundary::decode_error::DecodeError::new(
                "schemaVersion",
                "must be greater than zero",
            )
        })?;
        if schema_version != NonZeroU32::MIN {
            return Err(enforcer_domain::boundary::decode_error::DecodeError::new(
                "schemaVersion",
                "unsupported feedback decision schema version",
            ));
        }
        let classification = match wire.classification.as_str() {
            "prevent" => MechanizationClassification::Prevent,
            "detect" => MechanizationClassification::Detect,
            _ => {
                return Err(enforcer_domain::boundary::decode_error::DecodeError::new(
                    "classification",
                    "must be prevent or detect",
                ))
            }
        };
        let proposed = match wire.proposed {
            true => FeedbackScaffoldState::Proposed,
            false => FeedbackScaffoldState::NotProposed,
        };
        let expected_proposed = match classification {
            MechanizationClassification::Prevent => FeedbackScaffoldState::Proposed,
            MechanizationClassification::Detect => FeedbackScaffoldState::NotProposed,
        };
        if proposed != expected_proposed {
            return Err(enforcer_domain::boundary::decode_error::DecodeError::new(
                "proposed",
                "must agree with the feedback classification",
            ));
        }
        Ok(Self {
            schema_version: FeedbackDecisionSchemaVersion::try_new(schema_version),
            event_type: wire.event_type.try_into()?,
            input_fingerprint: wire.input_fingerprint,
            tool: FeedbackToolName::try_from(wire.tool)?,
            source_rule_id: ExternalDiagnosticCode::try_from(wire.source_rule_id)?,
            classification,
            proposed,
        })
    }
}
