use crate::journal::hash_chain::verify_hash_chain_entry;
use crate::{
    EventingError, JournalDispatchPhase, JournalHash, NdjsonEventJournal, NdjsonJournalEntry,
    ReplayFilter, ReplayMode, ReplayRecord,
};

use tokio::{
    fs::File,
    io::{BufReader, Lines},
};

pub(super) async fn next_line(
    lines: &mut Lines<BufReader<File>>,
    journal: &NdjsonEventJournal,
) -> Result<Option<String>, EventingError> {
    lines
        .next_line()
        .await
        .map_err(|error| EventingError::journal_io(journal.path_string(), &error))
}

pub(super) fn read_record(
    mode: ReplayMode,
    line: &str,
    line_number: usize,
    expected_previous_hash: &Option<JournalHash>,
    filter: &ReplayFilter,
) -> Result<Option<NdjsonJournalEntry>, EventingError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    let entry = parse_entry(line, line_number)?;
    verify_hash_chain_entry(&entry, expected_previous_hash).map_err(|reason| {
        EventingError::JournalCorruptLine {
            line: line_number,
            reason,
        }
    })?;
    if should_skip_entry(mode, &entry, filter) {
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
    pub(super) last_sequence: &'a mut u64,
    pub(super) records: &'a mut Vec<ReplayRecord>,
}

pub(super) fn process_record(
    mode: ReplayMode,
    line: &str,
    line_number: usize,
    filter: &ReplayFilter,
    accumulator: &mut ReplayAccumulator<'_>,
) -> Result<bool, EventingError> {
    let Some(record) = read_record(
        mode,
        line,
        line_number,
        accumulator.expected_previous_hash,
        filter,
    )?
    else {
        return Ok(false);
    };
    *accumulator.expected_previous_hash = record.append.current_hash.clone();
    *accumulator.last_sequence = (*accumulator.last_sequence).max(record.append.sequence);
    accumulator.records.push(ReplayRecord {
        sequence: record.append.sequence,
        envelope: record.envelope,
    });
    Ok(true)
}

fn parse_entry(line: &str, line_number: usize) -> Result<NdjsonJournalEntry, EventingError> {
    serde_json::from_str(line).map_err(|error| EventingError::JournalCorruptLine {
        line: line_number,
        reason: error.to_string(),
    })
}

fn should_skip_entry(mode: ReplayMode, entry: &NdjsonJournalEntry, filter: &ReplayFilter) -> bool {
    mode == ReplayMode::ActionHandlersAllowed && entry.phase != JournalDispatchPhase::AfterDispatch
        || !filter.matches(entry)
}
