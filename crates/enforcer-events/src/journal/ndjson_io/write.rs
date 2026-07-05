use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::journal::hash_chain::hash_entry;
use crate::{EventingError, JournalDispatchPhase, JournalHash, StoredEventEnvelope};

use super::{
    JournalAppend, JournalFlushPolicy, JournalHashChain, NdjsonEventJournal, NdjsonJournalOptions,
};

impl NdjsonEventJournal {
    pub(crate) async fn write_entry(
        &self,
        append: &JournalAppend,
        envelope: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<(), EventingError> {
        let entry = super::NdjsonJournalEntry {
            append: append.clone(),
            phase,
            envelope: envelope.clone(),
        };
        let mut line =
            serde_json::to_vec(&entry).map_err(|error| EventingError::journal_encode(&error))?;
        line.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(|error| EventingError::journal_io(self.path_string(), &error))?;
        file.write_all(&line)
            .await
            .map_err(|error| EventingError::journal_io(self.path_string(), &error))?;
        if self.options.flush == JournalFlushPolicy::Always {
            file.flush()
                .await
                .map_err(|error| EventingError::journal_io(self.path_string(), &error))?;
        }
        Ok(())
    }
}

fn previous_hash(
    options: &NdjsonJournalOptions,
    state: &super::super::ndjson_state::NdjsonJournalState,
) -> Option<JournalHash> {
    match options.hash_chain {
        JournalHashChain::Disabled => None,
        JournalHashChain::Enabled => state.previous_hash.clone(),
    }
}

fn current_hash(
    options: &NdjsonJournalOptions,
    sequence: u64,
    previous_hash: &Option<JournalHash>,
    envelope: &StoredEventEnvelope,
    phase: JournalDispatchPhase,
) -> Result<Option<JournalHash>, EventingError> {
    match options.hash_chain {
        JournalHashChain::Disabled => Ok(None),
        JournalHashChain::Enabled => {
            hash_entry(sequence, previous_hash.as_ref(), envelope, phase).map(Some)
        }
    }
}
