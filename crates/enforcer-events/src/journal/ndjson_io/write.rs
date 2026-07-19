use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::boundary::journal_persistence::NdjsonJournalEntryDto;
use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::journal::ndjson::{JournalFlushPolicy, NdjsonEventJournal, NdjsonJournalEntry};
use crate::{error::EventingError, journal::JournalAppend};
use enforcer_domain::events_types::JournalDispatchPhase;

impl NdjsonEventJournal {
    pub(crate) async fn write_entry(
        &self,
        append: &JournalAppend,
        envelope: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<(), EventingError> {
        let entry = NdjsonJournalEntry {
            // CLONE-JUSTIFICATION: the persistence DTO owns append metadata borrowed from the caller.
            append: append.clone(),
            phase,
            // CLONE-JUSTIFICATION: the persistence entry owns an envelope snapshot across asynchronous file I/O.
            envelope: envelope.clone(),
        };
        let wire_entry = NdjsonJournalEntryDto::from(&entry);
        let mut line = serde_json::to_vec(&wire_entry)
            .map_err(|error| EventingError::journal_encode(&error))?;
        line.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path.as_path())
            .await
            .map_err(|error| EventingError::journal_io(self.journal_path(), &error))?;
        file.write_all(&line)
            .await
            .map_err(|error| EventingError::journal_io(self.journal_path(), &error))?;
        if self.options.flush == JournalFlushPolicy::Always {
            file.flush()
                .await
                .map_err(|error| EventingError::journal_io(self.journal_path(), &error))?;
        }
        Ok(())
    }
}
