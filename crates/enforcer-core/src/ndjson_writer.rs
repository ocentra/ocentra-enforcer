//! Generic append-only NDJSON sink (OcentraParent `logging-core` borrow —
//! see the vendoring attribution note in `lib.rs`).
//!
//! One serialized record per line, appended to the target file. The writer
//! NEVER rewrites or truncates existing content: the file is opened with
//! `append(true).create(true)` and only ever grows. d04 run-telemetry
//! records and any pack emitting structured records ride this sink; the
//! proof journal (arc-17) layers the hash-chain envelope on top.

use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;

use crate::error::Result;

/// Append-only newline-delimited-JSON writer, generic over any serde `T`.
#[derive(Debug)]
pub struct NdjsonWriter<T> {
    file: std::fs::File,
    _record: PhantomData<T>,
}

impl<T: serde::Serialize> NdjsonWriter<T> {
    /// Open (or create) the sink file in append-only mode.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        Ok(Self {
            file,
            _record: PhantomData,
        })
    }

    /// Serialize one record and append it as a single line.
    pub fn append(&mut self, record: &T) -> Result<()> {
        let line = serde_json::to_string(record)?;
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}

/// Read every record back from an NDJSON file (test/verification helper —
/// production consumers stream instead of loading whole files).
pub fn read_all<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let content = std::fs::read_to_string(path)?;
    let mut records = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        records.push(serde_json::from_str(line)?);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::{read_all, NdjsonWriter};
    use crate::error::Result;

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Record {
        seq: u32,
        event: String,
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "enforcer-core-ndjson-{}-{}-{name}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn append_round_trips_records() -> Result<()> {
        let path = temp_path("round-trip");
        {
            let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
            writer.append(&Record {
                seq: 1,
                event: "start".to_owned(),
            })?;
            writer.append(&Record {
                seq: 2,
                event: "finish".to_owned(),
            })?;
        }
        let records: Vec<Record> = read_all(&path)?;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[1].event, "finish");
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn reopen_appends_instead_of_truncating() -> Result<()> {
        let path = temp_path("append-only");
        {
            let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
            writer.append(&Record {
                seq: 1,
                event: "first-open".to_owned(),
            })?;
        }
        {
            let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
            writer.append(&Record {
                seq: 2,
                event: "second-open".to_owned(),
            })?;
        }
        let records: Vec<Record> = read_all(&path)?;
        assert_eq!(records.len(), 2, "reopening must never truncate");
        assert_eq!(records[0].event, "first-open");
        assert_eq!(records[1].event, "second-open");
        std::fs::remove_file(&path)?;
        Ok(())
    }
}
