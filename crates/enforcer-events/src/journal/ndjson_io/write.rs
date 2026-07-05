use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::journal::ndjson::{JournalFlushPolicy, NdjsonEventJournal, NdjsonJournalEntry};
use crate::{EventingError, JournalAppend, JournalDispatchPhase, StoredEventEnvelope};

impl NdjsonEventJournal {
    pub(crate) async fn write_entry(
        &self,
        append: &JournalAppend,
        envelope: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<(), EventingError> {
        let entry = NdjsonJournalEntry {
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
