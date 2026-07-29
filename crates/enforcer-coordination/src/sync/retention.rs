//! Retention and compaction of append-only coordination streams.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::boundary::{archived_stream_dir, streams_dir};
use crate::error::Result;
use crate::sync::stream::list_stream_files;
use enforcer_domain::coordination_types::{
    CompactionKeepCount, CoordinationArchiveStamp, CoordinationEventCount, CoordinationLedgerPath,
    CoordinationLedgerRoot, CoordinationStreamName,
};

/// Per-stream compaction outcome.
#[derive(Debug, Clone)]
pub struct CompactedStream {
    pub stream: CoordinationStreamName,
    pub archived_events: CoordinationEventCount,
    pub retained_events: CoordinationEventCount,
    pub archive_path: CoordinationLedgerPath,
}

/// Result of a compaction pass.
#[derive(Debug, Clone, Default)]
pub struct CompactionResult {
    pub compacted_streams: Vec<CompactedStream>,
}

/// Move older events into an archive segment while retaining the requested
/// positive number of newest events in each live stream.
pub fn compact_ledger(
    root: &CoordinationLedgerRoot,
    keep_latest: CompactionKeepCount,
) -> Result<CompactionResult> {
    let root_path = root.as_path();
    let keep_count = keep_latest.value().get();
    let mut compacted_streams = Vec::new();
    for stream in list_stream_files(root_path)? {
        let stream_path = streams_dir(root_path).join(stream.as_str());
        let raw = fs::read_to_string(&stream_path)?;
        let lines: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
        if lines.len() <= keep_count {
            continue;
        }
        let split_at = lines.len() - keep_count;
        let (archive_lines, retained_lines) = lines.split_at(split_at);
        let archive_dir = archived_stream_dir(root_path, stream.as_str());
        fs::create_dir_all(&archive_dir)?;
        let stamp = archive_stamp()?;
        let archive_path = archive_dir.join(format!("{}.ndjson", stamp.as_str()));
        {
            use std::io::Write;
            let mut handle = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&archive_path)?;
            writeln!(handle, "{}", archive_lines.join("\n"))?;
        }
        let temporary_path = stream_path.with_extension("ndjson.compact.tmp");
        fs::write(&temporary_path, format!("{}\n", retained_lines.join("\n")))?;
        fs::rename(&temporary_path, &stream_path)?;
        compacted_streams.push(CompactedStream {
            stream,
            archived_events: CoordinationEventCount::from_collection(archive_lines),
            retained_events: CoordinationEventCount::from_collection(retained_lines),
            archive_path: CoordinationLedgerPath::from_absolute_path(&archive_path)?,
        });
    }
    Ok(CompactionResult { compacted_streams })
}

fn archive_stamp() -> Result<CoordinationArchiveStamp> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_else(|before_epoch| before_epoch.duration().as_nanos());
    Ok(CoordinationArchiveStamp::try_from(format!("{nanos:020}"))?)
}
