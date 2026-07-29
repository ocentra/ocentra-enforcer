use crate::{
    envelope::EventMetadata,
    error::EventingError,
    request::{RequestEvent, RequestOptions, RequestReport},
};
use enforcer_domain::events_types::RequestId;

use super::EventBus;

#[path = "request_steps/wait_protocol.rs"]
mod request_wait_protocol;

#[path = "request_steps/runner.rs"]
mod request_execution;

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
    request_execution::run(bus, event, metadata, options, request_id).await
}
