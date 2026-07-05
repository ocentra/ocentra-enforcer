use std::sync::Arc;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::journal::{
    hash_chain::{hash_entry, verify_hash_chain_entry},
    EventJournal, JournalAppendFuture,
};
use crate::{EventingError, JournalDispatchPhase, JournalHash, StoredEventEnvelope};

use super::{
    ndjson_state::NdjsonJournalState, JournalAppend, JournalFlushPolicy, JournalHashChain,
    NdjsonEventJournal, NdjsonJournalEntry, NdjsonJournalOptions,
};

#[path = "ndjson_io/append.rs"]
mod append;
#[path = "ndjson_io/recover.rs"]
mod recover;
#[path = "ndjson_io/write.rs"]
mod write;
