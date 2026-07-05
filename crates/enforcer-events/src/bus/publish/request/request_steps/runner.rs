use crate::{EventMetadata, EventingError, RequestEvent, RequestId, RequestOptions, RequestReport};

use super::super::EventBus;
use super::helpers::{
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
    E: RequestEvent,
{
    let mut receiver = bus.requests.register(request_id.clone())?;
    let bus_for_publish = bus.clone();
    let mut publish = tokio::spawn(async move { bus_for_publish.publish(event, metadata).await });
    let mut timeout = bus.clock.sleep(options.timeout());
    let mut publish_report = None;
    let mut response_payload = None;

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
            abort_request_publish(&mut publish, false).await;
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

    let report_request_id = request_id.clone();
    complete_request::<E>(request_id, &mut publish_report, &mut response_payload)?.ok_or(
        EventingError::RequestIncomplete {
            request_id: report_request_id,
        },
    )
}
