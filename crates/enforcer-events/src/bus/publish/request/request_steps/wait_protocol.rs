use enforcer_domain::events_types::{
    EventErrorField, EventErrorReason, RequestId, RequestPublishState,
};
use tokio::task::JoinHandle;

use crate::{
    bus::reports::handler::PublishReport,
    clock::EventClockSleep,
    error::EventingError,
    request::{RequestEvent, RequestReport},
};

use super::super::EventBus;

/// The bus and request identity shared, unchanged, across every step of one
/// request/response wait -- grouped so the `await_*_after_*` helpers below
/// take one cohesive parameter instead of two independent ones.
pub(super) struct RequestWait<'a> {
    pub(super) bus: &'a EventBus,
    pub(super) request_id: &'a RequestId,
}

pub(super) fn request_publish_result(
    result: Result<Result<PublishReport, EventingError>, tokio::task::JoinError>,
) -> Result<PublishReport, EventingError> {
    result.map_err(|error| {
        EventingError::invalid_value(
            EventErrorField::from_diagnostic("request_publish_task"),
            // ALLOC-JUSTIFICATION: the join failure must become owned diagnostic state after the task error is dropped.
            EventErrorReason::from_diagnostic(error.to_string()),
        )
    })?
}

pub(super) async fn abort_request_publish(
    publish: &mut JoinHandle<Result<PublishReport, EventingError>>,
    publish_state: RequestPublishState,
) {
    if publish_state == RequestPublishState::Complete {
        return;
    }
    publish.abort();
    let _ = publish.await;
}

pub(super) async fn handle_publish_result(
    bus: &EventBus,
    request_id: &RequestId,
    result: Result<Result<PublishReport, EventingError>, tokio::task::JoinError>,
) -> Result<PublishReport, EventingError> {
    request_publish_result(result).inspect_err(|_| {
        bus.requests.cancel(request_id);
    })
}

pub(super) async fn handle_receiver_result(
    bus: &EventBus,
    request_id: &RequestId,
    payload: Result<
        crate::boundary::request_persistence::RequestPayload,
        tokio::sync::oneshot::error::RecvError,
    >,
) -> Result<crate::boundary::request_persistence::RequestPayload, EventingError> {
    match payload {
        Ok(payload) => Ok(payload),
        Err(_) => {
            bus.requests.timeout(request_id);
            // CLONE-JUSTIFICATION: the timeout error owns the request identity after registry cleanup releases its borrow.
            Err(EventingError::RequestTimedOut {
                request_id: request_id.clone(),
            })
        }
    }
}

pub(super) async fn await_response_after_publish(
    wait: RequestWait<'_>,
    receiver: &mut tokio::sync::oneshot::Receiver<
        crate::boundary::request_persistence::RequestPayload,
    >,
    timeout: &mut EventClockSleep<'_>,
    publish: &mut JoinHandle<Result<PublishReport, EventingError>>,
    response_payload: &mut Option<crate::boundary::request_persistence::RequestPayload>,
) -> Result<(), EventingError> {
    let RequestWait { bus, request_id } = wait;
    // CANCEL-SAFE: response completion consumes the receiver; timeout marks the request terminal and joins the aborted publish task.
    // CANCELLATION-TEST: `tests/unit/request_response.rs::request_timeout_covers_slow_handler_dispatch`
    // drives this select's timeout arm and verifies the handler remains incomplete and in-flight state is released.
    tokio::select! {
        payload = receiver => {
            *response_payload = Some(handle_receiver_result(bus, request_id, payload).await?);
        }
        _ = timeout.as_mut() => {
            bus.requests.timeout(request_id);
            abort_request_publish(publish, RequestPublishState::Complete).await;
            // CLONE-JUSTIFICATION: the timeout error retains the request identity after the wait context is released.
            return Err(EventingError::RequestTimedOut {
                request_id: request_id.clone(),
            });
        }
    }
    Ok(())
}

pub(super) async fn await_publish_after_response(
    wait: RequestWait<'_>,
    publish: &mut JoinHandle<Result<PublishReport, EventingError>>,
    timeout: &mut EventClockSleep<'_>,
    publish_report: &mut Option<PublishReport>,
) -> Result<(), EventingError> {
    let RequestWait { bus, request_id } = wait;
    // CANCEL-SAFE: publish completion is retained; timeout marks the request terminal and aborts then joins the publish task.
    // CANCELLATION-TEST: `tests/unit/request_response.rs::request_timeout_aborts_never_completing_publish_and_releases_in_flight`
    // drives this select's timeout arm and verifies the retained publish task releases in-flight state.
    tokio::select! {
        result = &mut *publish => {
            *publish_report = Some(handle_publish_result(bus, request_id, result).await?);
        }
        _ = timeout.as_mut() => {
            bus.requests.timeout(request_id);
            abort_request_publish(publish, RequestPublishState::Pending).await;
            // CLONE-JUSTIFICATION: the timeout error retains the request identity after the wait context is released.
            return Err(EventingError::RequestTimedOut {
                request_id: request_id.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn complete_request<E>(
    request_id: RequestId,
    publish_report: &mut Option<PublishReport>,
    response_payload: &mut Option<crate::boundary::request_persistence::RequestPayload>,
) -> Result<Option<RequestReport<E::Response>>, EventingError>
where
    E: RequestEvent,
{
    let (Some(publish_report), Some(response_payload)) =
        (publish_report.take(), response_payload.take())
    else {
        return Ok(None);
    };
    let response = response_payload.decode::<E::Response>(&request_id)?;
    Ok(Some(RequestReport {
        request_id,
        response,
        publish_report,
    }))
}
