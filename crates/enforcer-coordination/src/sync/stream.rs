//! Append-only event stream: read/write/archive.
//!
//! Ported from `src/coordination/vendor/stream.js`. Preserves the append-only
//! invariant (never rewrite live lines) and the archived-first segment
//! ordering used by retention/compaction (arc-16 workpack row "Retention /
//! compact -> archived-segment model").

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::domain::{lock_path, stream_path, streams_dir, NodeId};
use crate::error::{CoordinationError, Result};
use crate::events::{assert_event_hash, HubEvent};
use enforcer_domain::ids::LaneId;

/// One physical file backing part of a logical stream: either the live
/// `streams/<writer>.ndjson` file or an archived segment under
/// `archive/streams/<writer>.ndjson/<stamp>.ndjson`.
#[derive(Debug, Clone)]
pub struct StreamSegment {
    pub stream_name: String,
    pub path: PathBuf,
    pub archived: bool,
}

/// List the live (non-conflict) `.ndjson` stream file names under
/// `streams/`. Ported from `stream.js#listStreamFiles`.
pub fn list_stream_files(root: &Path) -> Result<Vec<String>> {
    let dir = streams_dir(root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".ndjson") && !name.contains(".conflict."))
        .collect();
    names.sort();
    Ok(names)
}

/// Archived-then-live segments for one logical stream name. Ported from
/// `stream.js#streamSegments`.
pub fn stream_segments(root: &Path, stream_name: &str) -> Result<Vec<StreamSegment>> {
    let mut segments = Vec::new();
    let archive_dir = crate::domain::archived_stream_dir(root, stream_name);
    if let Ok(entries) = fs::read_dir(&archive_dir) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".ndjson"))
            .collect();
        names.sort();
        for name in names {
            segments.push(StreamSegment {
                stream_name: stream_name.to_owned(),
                path: archive_dir.join(name),
                archived: true,
            });
        }
    }
    segments.push(StreamSegment {
        stream_name: stream_name.to_owned(),
        path: streams_dir(root).join(stream_name),
        archived: false,
    });
    Ok(segments)
}

enum ParsedLine {
    Event(HubEvent),
    Warning { line: usize, warning: String },
}

fn read_stream_lenient(path: &Path) -> Result<Vec<ParsedLine>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut parsed = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<HubEvent>(trimmed) {
            Ok(event) => parsed.push(ParsedLine::Event(event)),
            Err(err) => {
                let is_final_line = index == lines.len() - 1
                    || (index == lines.len() - 2 && lines.last() == Some(&""));
                parsed.push(ParsedLine::Warning {
                    line: index + 1,
                    warning: if is_final_line {
                        "ignored malformed final line".to_owned()
                    } else {
                        format!("malformed line: {err}")
                    },
                });
            }
        }
    }
    Ok(parsed)
}

/// Read + verify every event in one physical file. Ported from
/// `stream.js#readStream` (verifies each event's hash as it goes).
pub fn read_stream(path: &Path) -> Result<Vec<HubEvent>> {
    let parsed = read_stream_lenient(path)?;
    let mut events = Vec::with_capacity(parsed.len());
    for item in parsed {
        match item {
            ParsedLine::Event(event) => {
                assert_event_hash(&event)?;
                events.push(event);
            }
            ParsedLine::Warning { line, warning } => {
                return Err(CoordinationError::rejected(format!(
                    "{}:{line}: {warning}",
                    path.display()
                )));
            }
        }
    }
    Ok(events)
}

fn read_canonical_stream(root: &Path, stream_name: &str) -> Result<Vec<HubEvent>> {
    let mut events = Vec::new();
    for segment in stream_segments(root, stream_name)? {
        events.extend(read_stream(&segment.path)?);
    }
    Ok(events)
}

/// Result of reading every stream across the ledger. Ported from
/// `stream.js#readAllStreams`.
pub struct AllStreams {
    pub events: Vec<HubEvent>,
    pub duplicate_count: usize,
    pub warnings: Vec<String>,
}

/// Read every stream in the ledger, deduplicated by event id and sorted by
/// `(ts, id)`. Malformed lines are collected as warnings rather than failing
/// the whole read (lenient mode — matches `readAllStreams`, unlike the
/// strict per-file `read_stream`).
pub fn read_all_streams(root: &Path) -> Result<AllStreams> {
    let mut seen = std::collections::HashSet::new();
    let mut events = Vec::new();
    let mut warnings = Vec::new();
    let mut duplicate_count = 0usize;
    for file_name in list_stream_files(root)? {
        for segment in stream_segments(root, &file_name)? {
            for parsed in read_stream_lenient(&segment.path)? {
                match parsed {
                    ParsedLine::Warning { line, warning } => {
                        warnings.push(format!("{}:{line}: {warning}", segment.path.display()));
                    }
                    ParsedLine::Event(event) => {
                        if seen.contains(&event.id) {
                            duplicate_count += 1;
                            continue;
                        }
                        seen.insert(event.id.clone());
                        events.push(event);
                    }
                }
            }
        }
    }
    events.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.id.cmp(&b.id)));
    Ok(AllStreams {
        events,
        duplicate_count,
        warnings,
    })
}

/// Acquire the per-writer stream lock (an `O_EXCL`-created `.lock` file),
/// run the closure, then always release it. Ported from
/// `stream.js#withStreamLock`. Deadline matches the JS source (5s, 25ms
/// backoff).
fn with_stream_lock<T>(
    root: &Path,
    node_id: &NodeId,
    lane: &LaneId,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let path = lock_path(root, node_id, lane);
    fs::create_dir_all(streams_dir(root))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut handle) => {
                let _ = write!(handle, "{}", std::process::id());
                break;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                if Instant::now() >= deadline {
                    return Err(CoordinationError::LockTimeout {
                        path: path.display().to_string(),
                    });
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(err.into()),
        }
    }
    let result = run();
    let _ = fs::remove_file(&path);
    result
}

/// Append a fully-built (hash-completed) event to `<writer>.ndjson`, holding
/// the writer's stream lock for the duration. Callers are expected to have
/// already computed `seq`/`prevEventId`/`prevHash`/`hash` (see
/// `crate::api::append_event` for the higher-level helper that does this).
pub fn append_completed_event(
    root: &Path,
    node_id: &NodeId,
    lane: &LaneId,
    event: &HubEvent,
) -> Result<()> {
    fs::create_dir_all(streams_dir(root))?;
    with_stream_lock(root, node_id, lane, || {
        let path = stream_path(root, node_id, lane);
        let mut handle = OpenOptions::new().append(true).create(true).open(&path)?;
        let line = serde_json::to_string(event)?;
        writeln!(handle, "{line}")?;
        handle.sync_all()?;
        Ok(())
    })
}

/// Read the current tip (last event) of one writer's live+archived stream,
/// used to compute the next `seq`/`prevEventId`/`prevHash` when appending.
pub fn stream_tip(root: &Path, node_id: &NodeId, lane: &LaneId) -> Result<Option<HubEvent>> {
    let stream_name = format!("{node_id}.{}.ndjson", lane.as_str());
    let events = read_canonical_stream(root, &stream_name)?;
    Ok(events.into_iter().last())
}

/// Raw line reader used by fixtures/tests to assert append-only behavior
/// (never rewriting earlier lines).
pub fn read_lines(path: &Path) -> Result<Vec<String>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }
    Ok(lines)
}

/// Parse a raw ndjson line into a generic JSON `Value` (used by retention to
/// move lines without needing full schema validation of archived content).
pub fn parse_line_value(line: &str) -> Result<Value> {
    Ok(serde_json::from_str(line)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::hash_for_event_value;
    use tempfile::tempdir;

    fn sample_event(seq: u64, prev: Option<(&str, &str)>) -> HubEvent {
        let mut event = HubEvent {
            id: format!("evt_{seq:08}"),
            schema: 1,
            hub: "test-hub".into(),
            node_id: "node_test".into(),
            node_name: "TestNode".into(),
            lane: "arc-16".into(),
            writer: "node_test.arc-16".into(),
            kind: "claim".into(),
            ts: format!("2026-07-04T00:00:{seq:02}.000Z"),
            seq,
            prev_event_id: prev.map(|(id, _)| id.to_owned()),
            prev_hash: prev.map(|(_, hash)| hash.to_owned()),
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
    fn append_then_read_round_trips_and_never_rewrites_earlier_lines() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let node = NodeId::parse("node_test").expect("valid node id");
        let lane: LaneId = "arc-16".parse().expect("valid lane id");

        let first = sample_event(1, None);
        append_completed_event(root, &node, &lane, &first).expect("append 1");
        let lines_after_first = read_lines(&stream_path(root, &node, &lane)).expect("read lines");
        assert_eq!(lines_after_first.len(), 1);

        let second = sample_event(2, Some((&first.id, &first.hash)));
        append_completed_event(root, &node, &lane, &second).expect("append 2");
        let lines_after_second = read_lines(&stream_path(root, &node, &lane)).expect("read lines");
        assert_eq!(lines_after_second.len(), 2);
        assert_eq!(lines_after_second[0], lines_after_first[0], "line 1 must never be rewritten");

        let all = read_all_streams(root).expect("read all streams");
        assert_eq!(all.events.len(), 2);
        assert_eq!(all.duplicate_count, 0);
        assert!(all.warnings.is_empty());
    }

    #[test]
    fn malformed_final_line_is_a_warning_not_a_hard_failure_in_read_all() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(streams_dir(root)).expect("mkdir");
        let path = streams_dir(root).join("node_test.arc-16.ndjson");
        fs::write(&path, "{not valid json\n").expect("write");
        let all = read_all_streams(root).expect("lenient read");
        assert_eq!(all.events.len(), 0);
        assert_eq!(all.warnings.len(), 1);
    }
}
