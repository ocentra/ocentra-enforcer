//! Journal phase and deterministic hash-input persistence values.
//!
//! BOUNDARY-INVARIANT: phase tokens and hash inputs are formed from validated
//! journal-domain values.
//! BOUNDARY-TEST: journal replay tests cover phase defaults and hash chaining.
//! NEGATIVE-TEST: `tests/journal_replay/file.rs` rejects tampered hash-chain entries.

use enforcer_domain::events_types::{JournalDispatchPhase, JournalSequence};
use serde::{Deserialize, Serialize};

use crate::boundary::stored_event_persistence::StoredEventEnvelopeDto;

/// JSON input shape used to compute the deterministic journal hash chain.
#[derive(Serialize)]
pub(crate) struct JournalHashInputDto {
    pub(crate) sequence: u64,
    pub(crate) previous_hash: Option<String>,
    pub(crate) phase: JournalDispatchPhaseDto,
    pub(crate) envelope: StoredEventEnvelopeDto,
}

impl JournalHashInputDto {
    pub(crate) fn new(
        sequence: JournalSequence,
        previous_hash: Option<String>,
        phase: JournalDispatchPhaseDto,
        envelope: StoredEventEnvelopeDto,
    ) -> Self {
        Self {
            sequence: crate::boundary::event_values::journal_sequence_value(sequence),
            previous_hash,
            phase,
            envelope,
        }
    }
}

/// JSON token for journal dispatch phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[doc = "SERDE-TAG-JUSTIFICATION: scalar JSON token at the journal persistence boundary."]
pub enum JournalDispatchPhaseDto {
    BeforeDispatch,
    AfterDispatch,
}

impl From<JournalDispatchPhase> for JournalDispatchPhaseDto {
    fn from(value: JournalDispatchPhase) -> Self {
        match value {
            JournalDispatchPhase::BeforeDispatch => Self::BeforeDispatch,
            JournalDispatchPhase::AfterDispatch => Self::AfterDispatch,
        }
    }
}

impl From<JournalDispatchPhaseDto> for JournalDispatchPhase {
    fn from(value: JournalDispatchPhaseDto) -> Self {
        match value {
            JournalDispatchPhaseDto::BeforeDispatch => Self::BeforeDispatch,
            JournalDispatchPhaseDto::AfterDispatch => Self::AfterDispatch,
        }
    }
}
