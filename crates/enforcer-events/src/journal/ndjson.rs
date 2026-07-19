use std::sync::{Arc, Mutex};

use enforcer_domain::events_types::{
    JournalDispatchPhase, JournalFlushPolicy, JournalHashChain, JournalPath,
};
use tokio::sync::Semaphore;

use super::{JournalAppend, SharedEventJournal};
use crate::boundary::{
    journal_file_path::JournalFilePath, stored_event_persistence::StoredEventEnvelope,
};

#[path = "ndjson_io.rs"]
mod ndjson_io;
#[path = "ndjson_state.rs"]
mod ndjson_state;
use self::ndjson_state::NdjsonJournalState;

/// Event-runtime data for ndjson journal options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdjsonJournalOptions {
    pub hash_chain: JournalHashChain,
    pub flush: JournalFlushPolicy,
}
impl NdjsonJournalOptions {
    /// Executes the hash chain event-runtime operation.
    pub fn hash_chain() -> Self {
        Self {
            hash_chain: JournalHashChain::Enabled,
            flush: JournalFlushPolicy::Always,
        }
    }
}
impl Default for NdjsonJournalOptions {
    fn default() -> Self {
        Self {
            hash_chain: JournalHashChain::Disabled,
            flush: JournalFlushPolicy::Always,
        }
    }
}

/// Event-runtime data for ndjson event journal.
#[derive(Clone, Debug)]
pub struct NdjsonEventJournal {
    path: JournalFilePath,
    options: NdjsonJournalOptions,
    state: Arc<Mutex<NdjsonJournalState>>,
    append_gate: Arc<Semaphore>,
}
impl NdjsonEventJournal {
    /// Executes the new event-runtime operation.
    pub fn new(path: JournalPath) -> Self {
        Self::with_options(path, NdjsonJournalOptions::default())
    }
    /// Executes the with options event-runtime operation.
    pub fn with_options(path: JournalPath, options: NdjsonJournalOptions) -> Self {
        Self {
            path: JournalFilePath::new(path),
            options,
            state: Arc::new(Mutex::new(NdjsonJournalState::default())),
            append_gate: Arc::new(Semaphore::new(1)),
        }
    }
    /// Executes the shared event-runtime operation.
    pub fn shared(self) -> SharedEventJournal {
        Arc::new(self)
    }
    pub(crate) fn journal_path(&self) -> &JournalPath {
        self.path.domain()
    }
    pub(crate) fn file_path(&self) -> &JournalFilePath {
        &self.path
    }
}

/// Event-runtime data for ndjson journal entry.
#[derive(Clone, Debug, PartialEq)]
pub struct NdjsonJournalEntry {
    pub append: JournalAppend,
    pub phase: JournalDispatchPhase,
    pub envelope: StoredEventEnvelope,
}
