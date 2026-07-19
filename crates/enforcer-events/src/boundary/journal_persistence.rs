//! NDJSON wire representation boundary for durable event journals.
//!
//! BOUNDARY-INVARIANT: NDJSON values are converted to typed journal records
//! here before journal/replay code can consume them.
//! BOUNDARY-TEST: malformed and hash-chain journal records are covered by
//! journal replay contract tests.
//! ROUNDTRIP-TEST: `tests/journal_replay/file.rs` serializes journal entries,
//! reopens them through `NdjsonJournalEntryDto`, and verifies typed hash/phase/envelope state.
//! BOUNDARY-OWNER: enforcer-events.
//! boundaryOwnerNote: enforcer-events owns the expanded journal wire surface
//! and uses a decode conversion for every raw record before it reaches
//! validated journal-domain state.
//! NEGATIVE-TEST: `tests/journal_replay/file.rs` rejects malformed records and
//! tampered hash-chain entries before replay.

use enforcer_domain::events_types::{EventCount, EventErrorField, EventErrorReason, JournalLine};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

use crate::{
    boundary::{
        journal_phase_persistence::JournalDispatchPhaseDto,
        stored_event_persistence::StoredEventEnvelopeDto,
    },
    error::EventingError,
    journal::ndjson::NdjsonJournalEntry,
    journal::JournalAppend,
};

/// JSON DTO for journal append metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalAppendDto {
    pub sequence: u64,
    // DEFAULT-JUSTIFICATION: journals created before hash chaining have no previous hash.
    #[serde(default)]
    pub previous_hash: Option<String>,
    // DEFAULT-JUSTIFICATION: journals created before hash chaining have no current hash.
    #[serde(default)]
    pub current_hash: Option<String>,
}

impl From<&JournalAppend> for JournalAppendDto {
    fn from(value: &JournalAppend) -> Self {
        Self {
            sequence: crate::boundary::event_values::journal_sequence_value(value.sequence),
            previous_hash: value
                .previous_hash
                .as_ref()
                .map(|hash| hash.as_str().to_owned()),
            current_hash: value
                .current_hash
                .as_ref()
                .map(|hash| hash.as_str().to_owned()),
        }
    }
}
impl TryFrom<JournalAppendDto> for JournalAppend {
    type Error = EventingError;
    fn try_from(value: JournalAppendDto) -> Result<Self, Self::Error> {
        Ok(Self {
            sequence: enforcer_domain::events_types::JournalSequence::try_new(
                NonZeroU64::new(value.sequence).ok_or_else(|| {
                    EventingError::invalid_value(
                        EventErrorField::from_diagnostic("journal_sequence"),
                        EventErrorReason::from_diagnostic(String::from(
                            "journal sequence must be positive",
                        )),
                    )
                })?,
            ),
            previous_hash: value
                .previous_hash
                .map(|hash| {
                    let raw = hash.clone();
                    raw.try_into().map_err(|_decode_error| {
                        EventingError::invalid_value(
                            EventErrorField::from_diagnostic("journal_hash"),
                            EventErrorReason::from_diagnostic(hash),
                        )
                    })
                })
                .transpose()?,
            current_hash: value
                .current_hash
                .map(|hash| {
                    let raw = hash.clone();
                    raw.try_into().map_err(|_decode_error| {
                        EventingError::invalid_value(
                            EventErrorField::from_diagnostic("journal_hash"),
                            EventErrorReason::from_diagnostic(hash),
                        )
                    })
                })
                .transpose()?,
        })
    }
}

/// JSON DTO for a durable journal entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NdjsonJournalEntryDto {
    pub append: JournalAppendDto,
    // DEFAULT-JUSTIFICATION: legacy journal entries predate phase tagging and represent completed dispatches.
    #[serde(default = "default_journal_phase")]
    pub phase: JournalDispatchPhaseDto,
    pub envelope: StoredEventEnvelopeDto,
}
impl From<&NdjsonJournalEntry> for NdjsonJournalEntryDto {
    fn from(value: &NdjsonJournalEntry) -> Self {
        Self {
            append: JournalAppendDto::from(&value.append),
            phase: value.phase.into(),
            envelope: StoredEventEnvelopeDto::from(&value.envelope),
        }
    }
}
impl TryFrom<NdjsonJournalEntryDto> for NdjsonJournalEntry {
    type Error = EventingError;
    fn try_from(value: NdjsonJournalEntryDto) -> Result<Self, Self::Error> {
        Ok(Self {
            append: value.append.try_into()?,
            phase: value.phase.into(),
            envelope: value.envelope.try_into()?,
        })
    }
}
fn default_journal_phase() -> JournalDispatchPhaseDto {
    JournalDispatchPhaseDto::AfterDispatch
}

pub(crate) fn decode_journal_entry(
    line: &JournalLine,
    line_number: EventCount,
) -> Result<NdjsonJournalEntry, EventingError> {
    let wire: NdjsonJournalEntryDto = serde_json::from_str(line.as_str()).map_err(|error| {
        EventingError::JournalCorruptLine {
            line: line_number,
            // ALLOC-JUSTIFICATION: the boundary error owns serde context after parsing returns.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        }
    })?;
    wire.try_into()
        .map_err(|error: EventingError| EventingError::JournalCorruptLine {
            line: line_number,
            // ALLOC-JUSTIFICATION: the boundary error owns conversion context after the source error is consumed.
            reason: EventErrorReason::from_diagnostic(error.to_string()),
        })
}
