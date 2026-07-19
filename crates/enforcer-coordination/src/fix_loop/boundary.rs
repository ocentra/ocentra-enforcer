//! Serialized boundary for fix-loop decision events.
//!
//! BOUNDARY-INVARIANT: primitive JSON fields exist only in this response;
//! fix-loop behavior consumes the validated domain event.
//! BOUNDARY-TEST: the response conversion round-trip and negative invalid
//! conversion are covered below.
//! BOUNDARY-OWNER: enforcer-coordination.
//! boundaryOwnerNote: enforcer-coordination owns this telemetry conversion boundary.

use enforcer_domain::coordination_types::{
    FindingCount, FixAcceptance, FixGeneratorName, FixIteration, IterationReason,
};
use serde::{Deserialize, Serialize};

use super::FixLoopDecisionEvent;

// SERIALIZATION-DOC: camelCase fields preserve the fix-loop telemetry contract.
/// Serialized response for one fix-loop decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixLoopDecisionEventResponse {
    pub generator_name: String,
    pub iteration: u32,
    pub findings_before: usize,
    pub findings_after: usize,
    pub accepted: bool,
    pub reason: String,
}

impl TryFrom<FixLoopDecisionEventResponse> for FixLoopDecisionEvent {
    type Error = enforcer_domain::boundary::decode_error::DecodeError;

    fn try_from(response: FixLoopDecisionEventResponse) -> Result<Self, Self::Error> {
        let iteration = std::num::NonZeroU32::new(response.iteration).ok_or_else(|| {
            enforcer_domain::boundary::decode_error::DecodeError::new(
                "fixIteration",
                "expected a positive iteration",
            )
        })?;
        Ok(Self {
            generator_name: FixGeneratorName::try_from(response.generator_name)?,
            iteration: FixIteration::new(iteration),
            findings_before: finding_count_from_usize(response.findings_before),
            findings_after: finding_count_from_usize(response.findings_after),
            accepted: if response.accepted {
                FixAcceptance::Accepted
            } else {
                FixAcceptance::Reverted
            },
            reason: IterationReason::parse(&response.reason)?,
        })
    }
}

impl From<&FixLoopDecisionEvent> for FixLoopDecisionEventResponse {
    fn from(event: &FixLoopDecisionEvent) -> Self {
        Self {
            // ALLOC-JUSTIFICATION: the serialized response owns its wire field independently of the domain event.
            generator_name: event.generator_name.as_str().to_owned(),
            iteration: event.iteration.value().get(),
            findings_before: finding_count_value(event.findings_before),
            findings_after: finding_count_value(event.findings_after),
            accepted: matches!(event.accepted, FixAcceptance::Accepted),
            reason: event.reason.as_str().to_owned(),
        }
    }
}

fn finding_count_from_usize(value: usize) -> FindingCount {
    let markers = vec![(); value];
    FindingCount::from_collection(&markers)
}

fn finding_count_value(value: FindingCount) -> usize {
    value.to_string().parse::<usize>().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{FixLoopDecisionEvent, FixLoopDecisionEventResponse};
    use enforcer_domain::coordination_types::{
        FixAcceptance, FixGeneratorName, FixIteration, IterationReason,
    };

    #[test]
    fn response_round_trip_preserves_the_domain_event() -> Result<(), Box<dyn std::error::Error>> {
        let event = FixLoopDecisionEvent {
            generator_name: FixGeneratorName::parse("marker-remover")?,
            iteration: FixIteration::new(std::num::NonZeroU32::MIN),
            findings_before: super::finding_count_from_usize(2),
            findings_after: super::finding_count_from_usize(1),
            accepted: FixAcceptance::Accepted,
            reason: IterationReason::Improved,
        };
        let response = FixLoopDecisionEventResponse::from(&event);
        assert_eq!(FixLoopDecisionEvent::try_from(response)?, event);
        Ok(())
    }

    #[test]
    fn negative_conversion_rejects_invalid_zero_iteration() {
        let response = FixLoopDecisionEventResponse {
            generator_name: "marker-remover".to_owned(),
            iteration: 0,
            findings_before: 2,
            findings_after: 1,
            accepted: true,
            reason: "improved".to_owned(),
        };
        assert_eq!(
            FixLoopDecisionEvent::try_from(response),
            Err(enforcer_domain::boundary::decode_error::DecodeError::new(
                "fixIteration",
                "expected a positive iteration",
            ))
        );
    }
}
