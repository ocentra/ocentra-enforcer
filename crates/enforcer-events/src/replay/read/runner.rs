use enforcer_domain::events_types::{EventCount, EventMatchState, JournalHash, ReplayMode};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

use crate::{
    error::EventingError,
    journal::ndjson::NdjsonEventJournal,
    replay::{ReplayCursor, ReplayFilter, ReplayReadReport},
};

use super::record;
use record::ReplayAccumulator;

pub(super) async fn read(
    journal: &NdjsonEventJournal,
    filter: ReplayFilter,
    mode: ReplayMode,
) -> Result<ReplayReadReport, EventingError> {
    let file = File::open(journal.file_path().as_path())
        .await
        .map_err(|error| EventingError::journal_io(journal.journal_path(), &error))?;
    let mut lines = BufReader::new(file).lines();
    let mut line_number = EventCount::default();
    let mut records = Vec::new();
    let mut skipped_count = enforcer_domain::events_types::EventCount::default();
    let mut last_sequence = filter.cursor.next_sequence;
    let mut expected_previous_hash: Option<JournalHash> = None;

    while let Some(line) = record::next_line(&mut lines, journal).await? {
        line_number = line_number.incremented();
        let mut accumulator = ReplayAccumulator {
            expected_previous_hash: &mut expected_previous_hash,
            last_sequence: &mut last_sequence,
            records: &mut records,
        };
        if record::process_record(mode, &line, line_number, &filter, &mut accumulator)?
            == EventMatchState::DoesNotMatch
        {
            skipped_count = skipped_count.incremented();
        }
    }

    Ok(ReplayReadReport {
        mode,
        cursor: ReplayCursor::after(last_sequence),
        records,
        skipped_count,
    })
}
