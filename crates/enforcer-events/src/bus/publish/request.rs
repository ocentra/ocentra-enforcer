use tokio::task::JoinHandle;

use crate::{
    EventMetadata, EventingError, PublishReport, RequestEvent, RequestOptions, RequestReport,
};

use super::EventBus;

mod request_steps;

pub(super) async fn publish_request<E>(
    bus: &EventBus,
    event: E,
    metadata: EventMetadata,
    options: RequestOptions,
) -> Result<RequestReport<E::Response>, EventingError>
where
    E: RequestEvent,
{
    bus.ensure_active()?;
    let request_id = event.request_id()?;
    request_steps::run(bus, event, metadata, options, request_id).await
}
