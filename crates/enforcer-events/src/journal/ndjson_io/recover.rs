use std::sync::PoisonError;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::journal::hash_chain::verify_hash_chain_entry;
use crate::{EventingError, JournalDispatchPhase, JournalHash, StoredEventEnvelope};

use super::{
    JournalAppend, JournalFlushPolicy, JournalHashChain, NdjsonEventJournal, NdjsonJournalEntry,
    NdjsonJournalOptions,
};

impl NdjsonEventJournal {
    pub(crate) async fn recover_state(&self) -> Result<(), EventingError> {
        if self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recovered
        {
            return Ok(());
        }
        let recovered = self.read_recovered_state().await?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if !state.recovered {
            *state = recovered;
        }
        Ok(())
    }

    pub(crate) async fn read_recovered_state(
        &self,
    ) -> Result<super::super::ndjson_state::NdjsonJournalState, EventingError> {
        let file = match File::open(&self.path).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(super::super::ndjson_state::NdjsonJournalState::recovered_empty());
            }
            Err(error) => return Err(EventingError::journal_io(self.path_string(), &error)),
        };
        let mut lines = BufReader::new(file).lines();
        let mut line_number = 0_usize;
        let mut state = super::super::ndjson_state::NdjsonJournalState::recovered_empty();
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| EventingError::journal_io(self.path_string(), &error))?
        {
            line_number += 1;
            if let Some(entry) = read_recovered_entry(&line, line_number, &state.previous_hash)? {
                state.next_sequence = entry.append.sequence;
                state.previous_hash = entry.append.current_hash;
            }
        }
        Ok(state)
    }
}

fn read_recovered_entry(
    line: &str,
    line_number: usize,
    expected_previous_hash: &Option<crate::JournalHash>,
) -> Result<Option<NdjsonJournalEntry>, EventingError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    let entry: NdjsonJournalEntry =
        serde_json::from_str(line).map_err(|error| EventingError::JournalCorruptLine {
            line: line_number,
            reason: error.to_string(),
        })?;
    verify_hash_chain_entry(&entry, expected_previous_hash).map_err(|reason| {
        EventingError::JournalCorruptLine {
            line: line_number,
            reason,
        }
    })?;
    Ok(Some(entry))
}
