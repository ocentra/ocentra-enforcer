use crate::{
    error::EventingError,
    journal::ndjson::NdjsonEventJournal,
    replay::{ReplayFilter, ReplayReadReport},
};
use enforcer_domain::events_types::ReplayMode;

#[path = "read/record.rs"]
mod record;

#[path = "read/runner.rs"]
mod runner;

impl NdjsonEventJournal {
    /// Executes the replay projection event-runtime operation.
    pub async fn replay_projection(
        &self,
        filter: ReplayFilter,
    ) -> Result<ReplayReadReport, EventingError> {
        self.read(filter, ReplayMode::ProjectionOnly).await
    }

    /// Executes the replay action records event-runtime operation.
    pub async fn replay_action_records(
        &self,
        filter: ReplayFilter,
    ) -> Result<ReplayReadReport, EventingError> {
        self.read(filter, ReplayMode::ActionHandlersAllowed).await
    }

    async fn read(
        &self,
        filter: ReplayFilter,
        mode: ReplayMode,
    ) -> Result<ReplayReadReport, EventingError> {
        runner::read(self, filter, mode).await
    }
}
