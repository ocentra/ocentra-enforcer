use crate::boundary::journal_persistence::decode_journal_entry;
use crate::journal::hash_chain::verify_hash_chain_entry;
use crate::{
    error::EventingError,
    journal::ndjson::{NdjsonEventJournal, NdjsonJournalEntry},
    replay::{ReplayFilter, ReplayRecord},
};
use enforcer_domain::events_types::{
    EventCount, EventMatchState, JournalDispatchPhase, JournalHash, JournalLine, JournalSequence,
    ReplayMode,
};

use tokio::{
    fs::File,
    io::{BufReader, Lines},
};

pub(super) enum JournalReadLine {
    Empty,
    Content(JournalLine),
}

pub(super) async fn next_line(
    lines: &mut Lines<BufReader<File>>,
    journal: &NdjsonEventJournal,
) -> Result<Option<JournalReadLine>, EventingError> {
    let line = lines
        .next_line()
        .await
        .map_err(|error| EventingError::journal_io(journal.journal_path(), &error))?;
    Ok(line.map(|line| {
        if line.trim().is_empty() {
            JournalReadLine::Empty
        } else {
            JournalReadLine::Content(JournalLine::from_diagnostic(line))
        }
    }))
}

pub(super) fn read_record(
    mode: ReplayMode,
    line: &JournalLine,
    line_number: EventCount,
    expected_previous_hash: &Option<JournalHash>,
    filter: &ReplayFilter,
) -> Result<Option<NdjsonJournalEntry>, EventingError> {
    let entry = decode_journal_entry(line, line_number)?;
    verify_hash_chain_entry(&entry, expected_previous_hash).map_err(|reason| {
        EventingError::JournalCorruptLine {
            line: line_number,
            reason,
        }
    })?;
    if should_skip_entry(mode, &entry, filter) == EventMatchState::Matches {
        return Ok(None);
    }
    Ok(Some(entry))
}

/// The running state accumulated while scanning a journal for replay
/// records: the rolling hash-chain expectation, the highest sequence seen
/// so far, and the accepted records themselves -- grouped so
/// `process_record` takes one cohesive, mutably-borrowed parameter instead
/// of three independent ones updated together on every line.
pub(super) struct ReplayAccumulator<'a> {
    pub(super) expected_previous_hash: &'a mut Option<JournalHash>,
    pub(super) last_sequence: &'a mut JournalSequence,
    pub(super) records: &'a mut Vec<ReplayRecord>,
}

pub(super) fn process_record(
    mode: ReplayMode,
    line: &JournalReadLine,
    line_number: EventCount,
    filter: &ReplayFilter,
    accumulator: &mut ReplayAccumulator<'_>,
) -> Result<EventMatchState, EventingError> {
    let JournalReadLine::Content(line) = line else {
        return Ok(EventMatchState::DoesNotMatch);
    };
    let Some(record) = read_record(
        mode,
        line,
        line_number,
        accumulator.expected_previous_hash,
        filter,
    )?
    else {
        return Ok(EventMatchState::DoesNotMatch);
    };
    // CLONE-JUSTIFICATION: the replay cursor retains the rolling hash while the record is consumed into output.
    *accumulator.expected_previous_hash = record.append.current_hash.clone();
    *accumulator.last_sequence = (*accumulator.last_sequence).max(record.append.sequence);
    accumulator.records.push(ReplayRecord {
        sequence: record.append.sequence,
        envelope: record.envelope,
    });
    Ok(EventMatchState::Matches)
}

fn should_skip_entry(
    mode: ReplayMode,
    entry: &NdjsonJournalEntry,
    filter: &ReplayFilter,
) -> EventMatchState {
    if mode == ReplayMode::ActionHandlersAllowed
        && entry.phase != JournalDispatchPhase::AfterDispatch
        || filter.matches(entry) == EventMatchState::DoesNotMatch
    {
        EventMatchState::Matches
    } else {
        EventMatchState::DoesNotMatch
    }
}
