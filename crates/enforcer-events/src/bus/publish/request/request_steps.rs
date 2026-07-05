use crate::{EventMetadata, EventingError, RequestEvent, RequestId, RequestOptions, RequestReport};

use super::EventBus;

#[path = "request_steps/helpers.rs"]
mod helpers;

#[path = "request_steps/runner.rs"]
mod runner;

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
    runner::run(bus, event, metadata, options, request_id).await
}
