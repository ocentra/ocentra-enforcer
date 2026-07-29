use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{error::EventingError, replay::ReplayRecord};
use enforcer_domain::events_types::{
    DispatchMode, EventCount, EventErrorField, EventErrorReason, EventType, JournalAppendDecision,
    JournalDispatchPhase, QueueDisposition, ReplayMode,
};

use super::{
    publish::flow::DispatchRequest,
    reports::{dead_letter::DeadLetter, empty_publish_report, handler::PublishReport},
    EventBus,
};

const PROJECTION_ONLY_REPLAY_EVENT_TYPE: &str = "projection-only-replay";
const IN_MEMORY_STORED_EVENT_LIMIT: usize = 4096;
const IN_MEMORY_DEAD_LETTER_LIMIT: usize = 4096;

impl EventBus {
    pub(super) async fn record_stored_snapshot(&self, stored: &StoredEventEnvelope) {
        let mut stored_journal = self.stored_journal.write().await;
        // CLONE-JUSTIFICATION: the in-memory journal owns a durable snapshot while publication continues using the envelope.
        stored_journal.push(stored.clone());
        trim_retained(
            &mut stored_journal,
            crate::boundary::event_values::event_count(IN_MEMORY_STORED_EVENT_LIMIT),
        );
    }

    pub(super) async fn record_dead_letter(&self, dead_letter: DeadLetter) {
        let mut dead_letters = self.dead_letters.write().await;
        dead_letters.push(dead_letter);
        trim_retained(
            &mut dead_letters,
            crate::boundary::event_values::event_count(IN_MEMORY_DEAD_LETTER_LIMIT),
        );
    }

    pub(super) async fn record_dead_letters(&self, new_dead_letters: Vec<DeadLetter>) {
        let mut dead_letters = self.dead_letters.write().await;
        dead_letters.extend(new_dead_letters);
        trim_retained(
            &mut dead_letters,
            crate::boundary::event_values::event_count(IN_MEMORY_DEAD_LETTER_LIMIT),
        );
    }

    pub(super) async fn append_journal_phase(
        &self,
        stored: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<(), EventingError> {
        if self.journal_policy.should_append(stored, phase) == JournalAppendDecision::Skip {
            return Ok(());
        }
        if let Some(journal) = &self.event_journal {
            journal.append_phase(stored, phase).await?;
        }
        Ok(())
    }

    /// Executes the replay to handlers event-runtime operation.
    pub async fn replay_to_handlers(
        &self,
        records: Vec<ReplayRecord>,
        mode: ReplayMode,
        dispatch_mode: DispatchMode,
    ) -> Result<Vec<PublishReport>, EventingError> {
        if mode != ReplayMode::ActionHandlersAllowed {
            let event_type = if let Some(record) = records.first() {
                // CLONE-JUSTIFICATION: replay validation owns the type after the selected journal record is released.
                record.envelope.contract.event_type.clone()
            } else {
                // BRAND-INVARIANT: the fallback event type and its diagnostic
                // fields are fixed validated taxonomy values.
                // ALLOC-JUSTIFICATION: the validated fallback event type is retained in the replay report.
                EventType::try_new(PROJECTION_ONLY_REPLAY_EVENT_TYPE.to_owned()).map_err(
                    |_decode_error| {
                        EventingError::invalid_value(
                            EventErrorField::from_diagnostic(String::from("event_type")),
                            // BRAND-INVARIANT: this reason is a validated diagnostic value,
                            // not an unvalidated domain field.
                            EventErrorReason::from_diagnostic(String::from(
                                PROJECTION_ONLY_REPLAY_EVENT_TYPE,
                            )),
                        )
                    },
                )?
            };
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
                    EventCount::ZERO,
                ));
                continue;
            }
            reports.push(
                self.dispatch_stored(
                    DispatchRequest {
                        stored: record.envelope,
                        subscribers,
                        dispatch_mode,
                    },
                    self.queue.report(QueueDisposition::Dispatched),
                    JournalAppendDecision::Skip,
                )
                .await?,
            );
        }
        Ok(reports)
    }
}

fn trim_retained<T>(values: &mut Vec<T>, limit: EventCount) {
    let limit = crate::boundary::event_values::event_count_value(limit);
    if values.len() > limit {
        values.drain(0..(values.len() - limit));
    }
}
// INVALID-INPUT-TEST: journal replay tests reject projection-only handler
// execution and malformed replay records.
