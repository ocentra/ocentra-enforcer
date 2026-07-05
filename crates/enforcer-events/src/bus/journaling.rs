use crate::{
    DispatchMode, EventingError, JournalDispatchPhase, QueueDisposition, ReplayMode, ReplayRecord,
    StoredEventEnvelope,
};

use super::{
    reports::{dead_letter::DeadLetter, empty_publish_report, handler::PublishReport},
    EventBus,
};

const PROJECTION_ONLY_REPLAY_EVENT_TYPE: &str = "projection-only-replay";
const IN_MEMORY_STORED_EVENT_LIMIT: usize = 4096;
const IN_MEMORY_DEAD_LETTER_LIMIT: usize = 4096;

impl EventBus {
    pub(super) async fn record_stored_snapshot(&self, stored: &StoredEventEnvelope) {
        let mut stored_journal = self.stored_journal.write().await;
        stored_journal.push(stored.clone());
        trim_retained(&mut stored_journal, IN_MEMORY_STORED_EVENT_LIMIT);
    }

    pub(super) async fn record_dead_letter(&self, dead_letter: DeadLetter) {
        let mut dead_letters = self.dead_letters.write().await;
        dead_letters.push(dead_letter);
        trim_retained(&mut dead_letters, IN_MEMORY_DEAD_LETTER_LIMIT);
    }

    pub(super) async fn record_dead_letters(&self, new_dead_letters: Vec<DeadLetter>) {
        let mut dead_letters = self.dead_letters.write().await;
        dead_letters.extend(new_dead_letters);
        trim_retained(&mut dead_letters, IN_MEMORY_DEAD_LETTER_LIMIT);
    }

    pub(super) async fn append_journal_phase(
        &self,
        stored: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<(), EventingError> {
        if !self.journal_policy.should_append(stored, phase) {
            return Ok(());
        }
        if let Some(journal) = &self.event_journal {
            journal.append_phase(stored, phase).await?;
        }
        Ok(())
    }

    pub async fn replay_to_handlers(
        &self,
        records: Vec<ReplayRecord>,
        mode: ReplayMode,
        dispatch_mode: DispatchMode,
    ) -> Result<Vec<PublishReport>, EventingError> {
        if mode != ReplayMode::ActionHandlersAllowed {
            let event_type = records
                .first()
                .map(|record| record.envelope.contract.event_type.clone())
                .unwrap_or(crate::EventType::parse(PROJECTION_ONLY_REPLAY_EVENT_TYPE)?);
            return Err(EventingError::ReplayActionNotAllowed { event_type });
        }

        let mut reports = Vec::new();
        for record in records {
            let subscribers = self.subscribers_for(&record.envelope);
            if subscribers.is_empty() {
                reports.push(empty_publish_report(
                    &record.envelope,
                    dispatch_mode,
                    self.queue.report(QueueDisposition::Dispatched),
                    0,
                ));
                continue;
            }
            reports.push(
                self.dispatch_stored(
                    record.envelope,
                    subscribers,
                    dispatch_mode,
                    self.queue.report(QueueDisposition::Dispatched),
                    false,
                )
                .await?,
            );
        }
        Ok(reports)
    }
}

fn trim_retained<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.drain(0..(values.len() - limit));
    }
}
