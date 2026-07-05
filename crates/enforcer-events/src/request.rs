use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::bus::reports::EventRequestMetrics;
use crate::{DomainEvent, EventingError, PublishReport, RequestId};

mod request_helpers;

const TERMINAL_REQUEST_RETENTION_LIMIT: usize = 4096;

pub trait EventResponseContract:
    Clone + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn validate(&self) -> Result<(), EventingError> {
        Ok(())
    }
}

pub trait RequestEvent: DomainEvent {
    type Response: EventResponseContract;

    fn request_id(&self) -> Result<RequestId, EventingError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestOptions {
    timeout: Duration,
}

impl RequestOptions {
    pub fn with_timeout(timeout: Duration) -> Result<Self, EventingError> {
        if timeout.is_zero() {
            return Err(EventingError::InvalidRequestOptions {
                reason: String::from("request timeout must be greater than zero"),
            });
        }
        Ok(Self { timeout })
    }

    pub fn timeout(self) -> Duration {
        self.timeout
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestCompletionOutcome {
    Completed,
    Duplicate,
    Late,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCompletionReport {
    pub request_id: RequestId,
    pub outcome: RequestCompletionOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestReport<R>
where
    R: EventResponseContract,
{
    pub request_id: RequestId,
    pub response: R,
    pub publish_report: PublishReport,
}

#[derive(Clone, Default)]
pub(crate) struct RequestRegistry {
    state: Arc<Mutex<RequestRegistryState>>,
}

impl RequestRegistry {
    pub(crate) fn register(
        &self,
        request_id: RequestId,
    ) -> Result<oneshot::Receiver<RequestPayload>, EventingError> {
        let (sender, receiver) = oneshot::channel();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.entries.contains_key(&request_id) {
            return Err(EventingError::DuplicateRequest { request_id });
        }
        state
            .entries
            .insert(request_id, RequestEntry::pending(sender));
        Ok(receiver)
    }

    pub(crate) fn complete<R>(
        &self,
        request_id: RequestId,
        response: R,
    ) -> Result<RequestCompletionReport, EventingError>
    where
        R: EventResponseContract,
    {
        let payload = RequestPayload::from_response(&request_id, response)?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(entry) = state.entries.get_mut(&request_id) else {
            return Ok(completion_report(
                request_id,
                RequestCompletionOutcome::Late,
            ));
        };
        match entry.state {
            RequestState::Pending => {
                request_helpers::complete_pending_request(&mut state, request_id, payload)
            }
            RequestState::Completed => Ok(completion_report(
                request_id,
                RequestCompletionOutcome::Duplicate,
            )),
            RequestState::TimedOut => Ok(completion_report(
                request_id,
                RequestCompletionOutcome::Late,
            )),
        }
    }

    pub(crate) fn timeout(&self, request_id: &RequestId) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = state.entries.get_mut(request_id) {
            if entry.state == RequestState::Pending {
                entry.state = RequestState::TimedOut;
                entry.sender.take();
                request_helpers::mark_terminal(&mut state, request_id);
            }
        }
        request_helpers::trim_terminal_requests(&mut state);
    }

    pub(crate) fn cancel(&self, request_id: &RequestId) -> bool {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let removed = state.entries.remove(request_id).is_some();
        if removed {
            state
                .terminal_order
                .retain(|terminal_id| terminal_id != request_id);
        }
        removed
    }

    pub(crate) fn metrics(&self) -> EventRequestMetrics {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        EventRequestMetrics {
            pending_request_count: state
                .entries
                .values()
                .filter(|entry| entry.state == RequestState::Pending)
                .count(),
            completed_request_count: state
                .entries
                .values()
                .filter(|entry| entry.state == RequestState::Completed)
                .count(),
            timed_out_request_count: state
                .entries
                .values()
                .filter(|entry| entry.state == RequestState::TimedOut)
                .count(),
        }
    }

    pub(crate) fn clear_for_test(&self) -> RequestRegistryClearReport {
        self.clear_entries()
    }

    pub(crate) fn cancel_for_shutdown(&self) -> RequestRegistryClearReport {
        self.clear_entries()
    }

    fn clear_entries(&self) -> RequestRegistryClearReport {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let report = request_helpers::request_registry_report(&state);
        state.entries.clear();
        state.terminal_order.clear();
        report
    }
}

#[derive(Default)]
struct RequestRegistryState {
    entries: BTreeMap<RequestId, RequestEntry>,
    terminal_order: VecDeque<RequestId>,
}

pub(crate) struct RequestRegistryClearReport {
    pub(crate) pending_request_count: usize,
    pub(crate) completed_request_count: usize,
    pub(crate) timed_out_request_count: usize,
}

struct RequestEntry {
    state: RequestState,
    sender: Option<oneshot::Sender<RequestPayload>>,
}

impl RequestEntry {
    fn pending(sender: oneshot::Sender<RequestPayload>) -> Self {
        Self {
            state: RequestState::Pending,
            sender: Some(sender),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestState {
    Pending,
    Completed,
    TimedOut,
}

pub(crate) struct RequestPayload {
    value: serde_json::Value,
}

impl RequestPayload {
    fn from_response<R>(request_id: &RequestId, response: R) -> Result<Self, EventingError>
    where
        R: EventResponseContract,
    {
        response.validate()?;
        let value = serde_json::to_value(response).map_err(|error| {
            EventingError::RequestResponseEncode {
                request_id: request_id.clone(),
                reason: error.to_string(),
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
                reason: error.to_string(),
            }
        })?;
        response.validate()?;
        Ok(response)
    }
}

pub(super) fn completion_report(
    request_id: RequestId,
    outcome: RequestCompletionOutcome,
) -> RequestCompletionReport {
    RequestCompletionReport {
        request_id,
        outcome,
    }
}
