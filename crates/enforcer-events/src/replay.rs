use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use enforcer_domain::events_types::{
    CorrelationId, EventMatchState, EventType, JournalSequence, ReplayMode,
};

use crate::journal::ndjson::{NdjsonEventJournal, NdjsonJournalEntry};

mod read;

/// Event-runtime data for replay cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayCursor {
    pub next_sequence: JournalSequence,
}

impl ReplayCursor {
    /// Executes the start event-runtime operation.
    pub fn start() -> Self {
        Self {
            next_sequence: JournalSequence::first(),
        }
    }

    /// Executes the after event-runtime operation.
    pub fn after(sequence: JournalSequence) -> Self {
        Self {
            next_sequence: sequence.saturating_next(),
        }
    }
}

impl Default for ReplayCursor {
    fn default() -> Self {
        Self::start()
    }
}

/// Event-runtime data for replay filter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayFilter {
    pub cursor: ReplayCursor,
    pub event_types: Vec<EventType>,
    pub correlation_id: Option<CorrelationId>,
}

impl ReplayFilter {
    /// Executes the all event-runtime operation.
    pub fn all() -> Self {
        Self::default()
    }

    /// Executes the for event type event-runtime operation.
    pub fn for_event_type(event_type: EventType) -> Self {
        Self {
            event_types: vec![event_type],
            ..Self::default()
        }
    }

    /// Executes the with correlation id event-runtime operation.
    pub fn with_correlation_id(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Executes the with cursor event-runtime operation.
    pub fn with_cursor(mut self, cursor: ReplayCursor) -> Self {
        self.cursor = cursor;
        self
    }

    pub(crate) fn matches(&self, entry: &NdjsonJournalEntry) -> EventMatchState {
        if entry.append.sequence >= self.cursor.next_sequence
            && (self.event_types.is_empty()
                || self
                    .event_types
                    .iter()
                    .any(|event_type| event_type == &entry.envelope.contract.event_type))
            && self
                .correlation_id
                .as_ref()
                .is_none_or(|correlation_id| correlation_id == &entry.envelope.correlation_id)
        {
            EventMatchState::Matches
        } else {
            EventMatchState::DoesNotMatch
        }
    }
}

/// Event-runtime data for replay record.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayRecord {
    pub sequence: JournalSequence,
    pub envelope: StoredEventEnvelope,
}

/// Event-runtime data for replay read report.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReadReport {
    pub mode: ReplayMode,
    pub cursor: ReplayCursor,
    pub records: Vec<ReplayRecord>,
    pub skipped_count: enforcer_domain::events_types::EventCount,
}

impl NdjsonEventJournal {}
