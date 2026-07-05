use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

use crate::{
    EventingError, JournalHash, NdjsonEventJournal, ReplayCursor, ReplayFilter, ReplayMode,
    ReplayReadReport, ReplayRecord,
};

use super::record;

pub(super) async fn read(
    journal: &NdjsonEventJournal,
    filter: ReplayFilter,
    mode: ReplayMode,
) -> Result<ReplayReadReport, EventingError> {
    let file = File::open(journal.path())
        .await
        .map_err(|error| EventingError::journal_io(journal.path_string(), &error))?;
    let mut lines = BufReader::new(file).lines();
    let mut line_number = 0_usize;
    let mut records = Vec::new();
    let mut skipped_count = 0_usize;
    let mut last_sequence = filter.cursor.next_sequence.saturating_sub(1);
    let mut expected_previous_hash: Option<JournalHash> = None;

    while let Some(line) = record::next_line(&mut lines, journal).await? {
        line_number += 1;
        skipped_count += usize::from(!record::process_record(
            mode,
            &line,
            line_number,
            &mut expected_previous_hash,
            &filter,
            &mut last_sequence,
            &mut records,
        )?);
    }

    Ok(ReplayReadReport {
        mode,
        cursor: ReplayCursor::after(last_sequence),
        records,
        skipped_count,
    })
}
