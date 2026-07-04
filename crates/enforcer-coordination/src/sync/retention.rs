//! Retention / compact -> archived-segment model.
//!
//! Ported from `src/coordination/vendor/retention.js`. `compact_ledger` moves
//! older events into `archive/streams/<stream>/<stamp>.ndjson`, keeping only
//! the newest `keep_latest` lines in the live file. Combined with
//! `sync::stream::stream_segments` (archived-first ordering), the append-only
//! read-everything invariant is preserved across compaction (arc-16 workpack
//! row "Retention / compact -> archived-segment model").

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{archived_stream_dir, streams_dir};
use crate::error::{CoordinationError, Result};
use crate::sync::stream::list_stream_files;

/// Per-stream compaction outcome.
#[derive(Debug, Clone)]
pub struct CompactedStream {
    pub stream: String,
    pub archived_events: usize,
    pub retained_events: usize,
    pub archive_path: PathBuf,
}

/// Result of a compaction pass.
#[derive(Debug, Clone, Default)]
pub struct CompactionResult {
    pub compacted_streams: Vec<CompactedStream>,
}

/// Compact every live stream, keeping only the newest `keep_latest` lines
/// live and moving the rest into a timestamped archive segment. Ported from
/// `retention.js#compactLedger`.
pub fn compact_ledger(root: &std::path::Path, keep_latest: usize) -> Result<CompactionResult> {
    if keep_latest == 0 {
        return Err(CoordinationError::rejected("keepLatest must be a positive integer"));
    }
    let mut compacted_streams = Vec::new();
    for stream in list_stream_files(root)? {
        let stream_path = streams_dir(root).join(&stream);
        let raw = fs::read_to_string(&stream_path)?;
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() <= keep_latest {
            continue;
        }
        let split_at = lines.len() - keep_latest;
        let archive_lines = &lines[..split_at];
        let retained_lines = &lines[split_at..];
        let archive_dir = archived_stream_dir(root, &stream);
        fs::create_dir_all(&archive_dir)?;
        let archive_path = archive_dir.join(format!("{}.ndjson", archive_stamp()));
        // `wx`-equivalent: fail if the stamped archive file already exists,
        // matching the JS source's exclusive-create semantics.
        {
            use std::io::Write;
            let mut handle = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&archive_path)?;
            writeln!(handle, "{}", archive_lines.join("\n"))?;
        }
        let tmp_path = stream_path.with_extension("ndjson.compact.tmp");
        fs::write(&tmp_path, format!("{}\n", retained_lines.join("\n")))?;
        fs::rename(&tmp_path, &stream_path)?;
        compacted_streams.push(CompactedStream {
            stream,
            archived_events: archive_lines.len(),
            retained_events: retained_lines.len(),
            archive_path,
        });
    }
    Ok(CompactionResult { compacted_streams })
}

fn archive_stamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{now:020}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::NodeId;
    use crate::events::hash_for_event_value;
    use crate::events::HubEvent;
    use crate::sync::stream::{append_completed_event, read_all_streams};
    use enforcer_domain::ids::LaneId;
    use tempfile::tempdir;

    fn sample_event(seq: u64) -> HubEvent {
        let mut event = HubEvent {
            id: format!("evt_{seq:08}"),
            schema: 1,
            hub: "test-hub".into(),
            node_id: "node_test".into(),
            node_name: "TestNode".into(),
            lane: "arc-16".into(),
            writer: "node_test.arc-16".into(),
            kind: "claim".into(),
            ts: format!("2026-07-04T00:{:02}:00.000Z", seq % 60),
            seq,
            prev_event_id: None,
            prev_hash: None,
            hash: String::new(),
            to: None,
            body: None,
            message_id: None,
            paths: Some(vec!["src/lib.rs".into()]),
            reason: None,
            owner: None,
            owners: None,
            state: None,
            worker_state: None,
            task_id: None,
            task_state: None,
            title: None,
            pr_url: None,
            summary: None,
            ttl_seconds: None,
            session_id: None,
            context: None,
        };
        let value = serde_json::to_value(&event).expect("serializable");
        event.hash = hash_for_event_value(&value);
        event
    }

    #[test]
    fn compaction_round_trips_every_event_and_shrinks_live_stream() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let node = NodeId::parse("node_test").expect("valid node id");
        let lane: LaneId = "arc-16".parse().expect("valid lane id");

        for seq in 1..=5u64 {
            append_completed_event(root, &node, &lane, &sample_event(seq)).expect("append");
        }

        let before = read_all_streams(root).expect("read before");
        assert_eq!(before.events.len(), 5);

        let result = compact_ledger(root, 2).expect("compact");
        assert_eq!(result.compacted_streams.len(), 1);
        let compacted = &result.compacted_streams[0];
        assert_eq!(compacted.archived_events, 3);
        assert_eq!(compacted.retained_events, 2);
        assert!(compacted.archive_path.exists());

        let after = read_all_streams(root).expect("read after (archived + live)");
        assert_eq!(after.events.len(), 5, "compaction must not lose events");

        let live_path = streams_dir(root).join("node_test.arc-16.ndjson");
        let live_lines: Vec<String> = fs::read_to_string(&live_path)
            .expect("read live")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_owned)
            .collect();
        assert_eq!(live_lines.len(), 2, "live stream must shrink after compaction");
    }
}
