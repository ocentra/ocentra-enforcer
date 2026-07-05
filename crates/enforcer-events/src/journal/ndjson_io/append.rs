use std::sync::{Arc, PoisonError};

use crate::journal::{hash_chain::hash_entry, EventJournal, JournalAppendFuture};
use crate::{EventingError, JournalDispatchPhase, JournalHash, StoredEventEnvelope};

use super::{JournalAppend, JournalHashChain, NdjsonEventJournal, NdjsonJournalOptions};

impl NdjsonEventJournal {
    async fn append_entry(
        &self,
        envelope: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> Result<JournalAppend, EventingError> {
        // The append gate semaphore is never closed anywhere in this crate,
        // so this only fails if that invariant is ever violated by a future
        // change.
        let _append_permit = Arc::clone(&self.append_gate)
            .acquire_owned()
            .await
            .map_err(|_closed| EventingError::JournalAppendGateClosed)?;
        self.recover_state().await?;
        let append = {
            let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let next_sequence = state.next_sequence.saturating_add(1);
            let previous_hash = previous_hash(&self.options, &state);
            let current_hash = current_hash(
                &self.options,
                next_sequence,
                &previous_hash,
                envelope,
                phase,
            )?;
            JournalAppend {
                sequence: next_sequence,
                previous_hash,
                current_hash,
            }
        };
        self.write_entry(&append, envelope, phase).await?;
        {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.next_sequence = append.sequence;
            state.previous_hash = append.current_hash.clone();
            state.recovered = true;
        }
        Ok(append)
    }
}

impl EventJournal for NdjsonEventJournal {
    fn append<'a>(&'a self, envelope: &'a StoredEventEnvelope) -> JournalAppendFuture<'a> {
        Box::pin(async move {
            self.append_entry(envelope, JournalDispatchPhase::AfterDispatch)
                .await
        })
    }

    fn append_phase<'a>(
        &'a self,
        envelope: &'a StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> JournalAppendFuture<'a> {
        Box::pin(async move { self.append_entry(envelope, phase).await })
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
