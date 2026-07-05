use crate::JournalHash;

#[derive(Default, Debug)]
pub(super) struct NdjsonJournalState {
    pub(super) next_sequence: u64,
    pub(super) previous_hash: Option<JournalHash>,
    pub(super) recovered: bool,
}

impl NdjsonJournalState {
    pub(super) fn recovered_empty() -> Self {
        Self {
            next_sequence: 0,
            previous_hash: None,
            recovered: true,
        }
    }
}
