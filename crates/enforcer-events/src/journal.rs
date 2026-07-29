use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use std::{future::Future, pin::Pin, sync::Arc};

use enforcer_domain::events_types::{JournalDispatchPhase, JournalHash, JournalSequence};

use crate::error::EventingError;

pub(crate) mod hash_chain;
pub mod ndjson;
pub mod policy;

pub type JournalAppendFuture<'a> =
    Pin<Box<dyn Future<Output = Result<JournalAppend, EventingError>> + Send + 'a>>;

/// Contract implemented by event journal.
pub trait EventJournal: Send + Sync {
    fn append<'a>(&'a self, envelope: &'a StoredEventEnvelope) -> JournalAppendFuture<'a>;

    fn append_phase<'a>(
        &'a self,
        envelope: &'a StoredEventEnvelope,
        _phase: JournalDispatchPhase,
    ) -> JournalAppendFuture<'a> {
        self.append(envelope)
    }
}

pub type SharedEventJournal = Arc<dyn EventJournal>;

/// Event-runtime data for journal append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalAppend {
    pub sequence: JournalSequence,
    pub previous_hash: Option<JournalHash>,
    pub current_hash: Option<JournalHash>,
}
