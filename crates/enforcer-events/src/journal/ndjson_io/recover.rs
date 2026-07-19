use std::sync::PoisonError;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::boundary::journal_persistence::decode_journal_entry;
use crate::journal::hash_chain::verify_hash_chain_entry;
use crate::journal::ndjson::NdjsonEventJournal;
use crate::{error::EventingError, journal::ndjson::NdjsonJournalEntry};
use enforcer_domain::events_types::{EventCount, JournalHash, JournalLine, JournalRecoveryState};

impl NdjsonEventJournal {
    pub(crate) async fn recover_state(&self) -> Result<(), EventingError> {
        if self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recovery
            == JournalRecoveryState::Recovered
        {
            return Ok(());
        }
        let recovered = self.read_recovered_state().await?;
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.recovery == JournalRecoveryState::Unrecovered {
            *state = recovered;
        }
        Ok(())
    }

    async fn read_recovered_state(
        &self,
    ) -> Result<super::super::ndjson_state::NdjsonJournalState, EventingError> {
        let file = match File::open(self.path.as_path()).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(super::super::ndjson_state::NdjsonJournalState::recovered_empty());
            }
            Err(error) => return Err(EventingError::journal_io(self.journal_path(), &error)),
        };
        let mut lines = BufReader::new(file).lines();
        let mut line_number = EventCount::default();
        let mut state = super::super::ndjson_state::NdjsonJournalState::recovered_empty();
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| EventingError::journal_io(self.journal_path(), &error))?
        {
            line_number = line_number.incremented();
            if line.trim().is_empty() {
                continue;
            }
            let line = JournalLine::from_diagnostic(line);
            if let Some(entry) = read_recovered_entry(&line, line_number, &state.previous_hash)? {
                state.last_sequence = Some(entry.append.sequence);
                state.previous_hash = entry.append.current_hash;
            }
        }
        Ok(state)
    }
}

fn read_recovered_entry(
    line: &JournalLine,
    line_number: EventCount,
    expected_previous_hash: &Option<JournalHash>,
) -> Result<Option<NdjsonJournalEntry>, EventingError> {
    let entry = decode_journal_entry(line, line_number)?;
    verify_hash_chain_entry(&entry, expected_previous_hash).map_err(|reason| {
        EventingError::JournalCorruptLine {
            line: line_number,
            reason,
        }
    })?;
    Ok(Some(entry))
}
