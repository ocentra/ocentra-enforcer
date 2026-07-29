use enforcer_domain::events_types::{JournalHash, JournalRecoveryState, JournalSequence};

#[derive(Debug)]
pub(super) struct NdjsonJournalState {
    pub(super) last_sequence: Option<JournalSequence>,
    pub(super) previous_hash: Option<JournalHash>,
    pub(super) recovery: JournalRecoveryState,
}

impl Default for NdjsonJournalState {
    fn default() -> Self {
        Self {
            last_sequence: None,
            previous_hash: None,
            recovery: JournalRecoveryState::Unrecovered,
        }
    }
}

impl NdjsonJournalState {
    pub(super) fn recovered_empty() -> Self {
        Self {
            last_sequence: None,
            previous_hash: None,
            recovery: JournalRecoveryState::Recovered,
        }
    }
}
