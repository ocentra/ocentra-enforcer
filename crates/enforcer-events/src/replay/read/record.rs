use crate::journal::hash_chain::verify_hash_chain_entry;
use crate::{
    EventingError, JournalDispatchPhase, JournalHash, NdjsonEventJournal, NdjsonJournalEntry,
    ReplayFilter, ReplayMode, ReplayRecord,
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader, Lines},
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

pub(super) fn process_record(
    mode: ReplayMode,
    line: &str,
    line_number: usize,
    expected_previous_hash: &mut Option<JournalHash>,
    filter: &ReplayFilter,
    last_sequence: &mut u64,
    records: &mut Vec<ReplayRecord>,
) -> Result<bool, EventingError> {
    let Some(record) = read_record(mode, line, line_number, expected_previous_hash, filter)? else {
        return Ok(false);
    };
    *expected_previous_hash = record.append.current_hash.clone();
    *last_sequence = (*last_sequence).max(record.append.sequence);
    records.push(ReplayRecord {
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
