//! Request/response JSON payload conversion boundary.
//!
//! BOUNDARY-INVARIANT: raw JSON response data is encoded and decoded only at
//! this edge; request registry state carries a typed opaque payload handle.
//! BOUNDARY-TEST: malformed response payloads and response validation are
//! covered by request/response contract tests.
//! BOUNDARY-OWNER: enforcer-events.
//! boundaryOwnerNote: enforcer-events owns the request completion wire shape
//! and its immediate conversion into typed request-domain state.
//! ROUNDTRIP-TEST: `tests/unit/envelope.rs` verifies canonical request report
//! serialization; invalid response payloads are rejected by request tests.

use crate::error::EventingError;
use crate::request::{EventResponseContract, RequestCompletionReport};
use enforcer_domain::events_types::{EventErrorReason, RequestCompletionOutcome, RequestId};

/// Opaque request response JSON kept at the response transport boundary.
pub(crate) struct RequestPayload {
    value: serde_json::Value,
}

impl RequestPayload {
    pub(crate) fn from_response<R>(
        request_id: &RequestId,
        response: R,
    ) -> Result<Self, EventingError>
    where
        R: EventResponseContract,
    {
        response.validate()?;
        let value = serde_json::to_value(response).map_err(|error| {
            EventingError::RequestResponseEncode {
                request_id: request_id.clone(),
                reason: EventErrorReason::from_diagnostic(error.to_string()),
            }
        })?;
        Ok(Self { value })
    }

    pub(crate) fn decode<R>(self, request_id: &RequestId) -> Result<R, EventingError>
    where
        R: EventResponseContract,
    {
        let response: R = serde_json::from_value(self.value).map_err(|error| {
            EventingError::RequestResponseDecode {
                request_id: request_id.clone(),
                reason: EventErrorReason::from_diagnostic(error.to_string()),
            }
        })?;
        response.validate()?;
        Ok(response)
    }
}

/// JSON presentation of a request completion report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCompletionReportResponse {
    pub request_id: String,
    pub outcome: String,
}

impl From<&RequestCompletionReport> for RequestCompletionReportResponse {
    fn from(value: &RequestCompletionReport) -> Self {
        Self {
            request_id: value.request_id.as_str().to_owned(),
            outcome: request_completion_outcome_token(value.outcome).to_owned(),
        }
    }
}

impl TryFrom<RequestCompletionReportResponse> for RequestCompletionReport {
    type Error = EventingError;

    fn try_from(value: RequestCompletionReportResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: value.request_id.try_into()?,
            outcome: request_completion_outcome_from_token(value.outcome)?,
        })
    }
}

fn request_completion_outcome_token(outcome: RequestCompletionOutcome) -> &'static str {
    match outcome {
        RequestCompletionOutcome::Completed => "completed",
        RequestCompletionOutcome::Duplicate => "duplicate",
        RequestCompletionOutcome::Late => "late",
    }
}

fn request_completion_outcome_from_token(
    value: String,
) -> Result<RequestCompletionOutcome, EventingError> {
    match value.as_str() {
        "completed" => Ok(RequestCompletionOutcome::Completed),
        "duplicate" => Ok(RequestCompletionOutcome::Duplicate),
        "late" => Ok(RequestCompletionOutcome::Late),
        _ => Err(EventingError::invalid_value(
            enforcer_domain::events_types::EventErrorField::from_diagnostic(
                "request_completion_outcome",
            ),
            EventErrorReason::from_diagnostic(value),
        )),
    }
}
