use crate::{EventType, EventingError};

use super::{DispatchMode, EventBus, QueueDrainReport};

mod runner;

impl EventBus {
    pub async fn drain_queued(
        &self,
        dispatch_mode: DispatchMode,
    ) -> Result<QueueDrainReport, EventingError> {
        self.ensure_active()?;
        runner::drain_queued_matching_unchecked(self, dispatch_mode, None).await
    }

    pub(super) async fn drain_queued_unchecked(
        &self,
        dispatch_mode: DispatchMode,
    ) -> Result<QueueDrainReport, EventingError> {
        runner::drain_queued_matching_unchecked(self, dispatch_mode, None).await
    }

    pub(super) async fn drain_queued_for_event_unchecked(
        &self,
        dispatch_mode: DispatchMode,
        event_type: &EventType,
    ) -> Result<QueueDrainReport, EventingError> {
        runner::drain_queued_matching_unchecked(self, dispatch_mode, Some(event_type)).await
    }
}
