use crate::{
    envelope::EventMetadata,
    error::EventingError,
    request::{RequestEvent, RequestOptions, RequestReport},
};
use enforcer_domain::events_types::{RequestId, RequestPublishState};

use super::super::EventBus;
use super::request_wait_protocol::{
    abort_request_publish, await_publish_after_response, await_response_after_publish,
    complete_request, handle_publish_result, handle_receiver_result, RequestWait,
};

pub(super) async fn run<E>(
    bus: &EventBus,
    event: E,
    metadata: EventMetadata,
    options: RequestOptions,
    request_id: RequestId,
) -> Result<RequestReport<E::Response>, EventingError>
where
    E: RequestEvent + serde::Serialize,
{
    // CLONE-JUSTIFICATION: the registry, spawned publish task, and completion report each require independently owned runtime handles or request identity.
    let mut receiver = bus.requests.register(request_id.clone())?;
    let bus_for_publish = bus.clone();
    let mut publish = tokio::spawn(async move { bus_for_publish.publish(event, metadata).await });
    let mut timeout = bus.clock.sleep(options.timeout());
    let mut publish_report = None;
    let mut response_payload = None;

    // CANCEL-SAFE: the winning branch is persisted, and the remaining publish/response future is explicitly awaited or aborted below.
    // CANCELLATION-TEST: request_response covers timeout cancellation and in-flight identity release.
    let publish_done = tokio::select! {
        result = &mut publish => {
            publish_report = Some(handle_publish_result(bus, &request_id, result).await?);
            true
        }
        payload = &mut receiver => {
            response_payload = Some(handle_receiver_result(bus, &request_id, payload).await?);
            false
        }
        _ = timeout.as_mut() => {
            bus.requests.timeout(&request_id);
            abort_request_publish(&mut publish, RequestPublishState::Pending).await;
            return Err(EventingError::RequestTimedOut { request_id });
        }
    };

    if publish_done {
        await_response_after_publish(
            RequestWait {
                bus,
                request_id: &request_id,
            },
            &mut receiver,
            &mut timeout,
            &mut publish,
            &mut response_payload,
        )
        .await?;
    } else {
        await_publish_after_response(
            RequestWait {
                bus,
                request_id: &request_id,
            },
            &mut publish,
            &mut timeout,
            &mut publish_report,
        )
        .await?;
    }

    // CLONE-JUSTIFICATION: the final report retains the request id after completion consumes the working identity.
    let report_request_id = request_id.clone();
    complete_request::<E>(request_id, &mut publish_report, &mut response_payload)?.ok_or(
        EventingError::RequestIncomplete {
            request_id: report_request_id,
        },
    )
}
