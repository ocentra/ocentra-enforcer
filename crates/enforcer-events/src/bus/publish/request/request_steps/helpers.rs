use tokio::task::JoinHandle;

use crate::{
    EventClockSleep, EventingError, PublishReport, RequestEvent, RequestId, RequestReport,
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
    result
        .map_err(|error| EventingError::invalid_value("request_publish_task", error.to_string()))?
}

pub(super) async fn abort_request_publish(
    publish: &mut JoinHandle<Result<PublishReport, EventingError>>,
    publish_done: bool,
) {
    if publish_done {
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
    payload: Result<crate::request::RequestPayload, tokio::sync::oneshot::error::RecvError>,
) -> Result<crate::request::RequestPayload, EventingError> {
    match payload {
        Ok(payload) => Ok(payload),
        Err(_) => {
            bus.requests.timeout(request_id);
            Err(EventingError::RequestTimedOut {
                request_id: request_id.clone(),
            })
        }
    }
}

pub(super) async fn await_response_after_publish(
    wait: RequestWait<'_>,
    receiver: &mut tokio::sync::oneshot::Receiver<crate::request::RequestPayload>,
    timeout: &mut EventClockSleep<'_>,
    publish: &mut JoinHandle<Result<PublishReport, EventingError>>,
    response_payload: &mut Option<crate::request::RequestPayload>,
) -> Result<(), EventingError> {
    let RequestWait { bus, request_id } = wait;
    tokio::select! {
        payload = receiver => {
            *response_payload = Some(handle_receiver_result(bus, request_id, payload).await?);
        }
        _ = timeout.as_mut() => {
            bus.requests.timeout(request_id);
            abort_request_publish(publish, true).await;
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
    tokio::select! {
        result = &mut *publish => {
            *publish_report = Some(handle_publish_result(bus, request_id, result).await?);
        }
        _ = timeout.as_mut() => {
            bus.requests.timeout(request_id);
            abort_request_publish(publish, false).await;
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
    response_payload: &mut Option<crate::request::RequestPayload>,
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
