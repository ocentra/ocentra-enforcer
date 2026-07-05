//! Append-only, hash-chained NDJSON logs for the x06.1 store.
//!
//! Built directly on `enforcer_core::telemetry` (the d04 primitive
//! already used by run telemetry): a `.chain` sidecar records one digest
//! per data line, INDEPENDENTLY of the data file itself. Verification
//! reads both files and compares -- it never recomputes an "expected"
//! digest from the same file being checked (that is the vacuous
//! self-referential check L37 specifically flags as false assurance).
//!
//! On top of the core primitive this module adds what x06.1 needs that
//! plain run-telemetry does not:
//! - typed generic log over any `Serialize + DeserializeOwned` entry,
//!   with the entry's own `seq` field checked for gap-free monotonicity
//!   on read (a hole in the sequence is corruption even if every
//!   individual line's chain digest verifies);
//! - corruption detection + quarantine: a data line that fails to
//!   deserialize, or whose `seq` breaks monotonicity, is quarantined
//!   (recorded, not silently dropped, not allowed to abort the whole
//!   read) and reported back to the caller;
//! - `append_with_seq`, which assigns the next gap-free `Seq` so callers
//!   never mint their own.

use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{MemoryError, QuarantinedRow, Result};
use crate::ids::Seq;

/// An append-only hash-chained log of `T` entries at `path`, with its
/// digest chain independently persisted to `path` + `.chain`.
pub struct AppendLog<T> {
    path: PathBuf,
    sink: enforcer_core::telemetry::RunTelemetrySink<T>,
    next_seq: Seq,
}

/// The outcome of reading a log end-to-end: the entries that parsed and
/// verified cleanly, plus any rows that had to be quarantined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOutcome<T> {
    pub entries: Vec<T>,
    pub quarantined: Vec<QuarantinedRow>,
}

impl<T> AppendLog<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// Open (or create) the log at `path`. Does NOT itself verify the
    /// existing chain -- callers that need to trust an existing log must
    /// call [`AppendLog::read_verified`] first (matching this crate's
    /// "corruption is reported, not silently accepted" contract: opening
    /// for append should not require reading, but reading for trust
    /// always verifies).
    pub fn open(path: &Path) -> Result<Self> {
        let sink = enforcer_core::telemetry::RunTelemetrySink::open(path).map_err(|source| {
            MemoryError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::other(source.to_string()),
            }
        })?;
        let next_seq = current_log_len(path).map(Seq).unwrap_or(Seq::GENESIS);
        Ok(Self {
            path: path.to_path_buf(),
            sink,
            next_seq,
        })
    }

    /// Append `build_entry(seq)` under the next gap-free sequence number,
    /// returning the assigned [`Seq`]. `build_entry` receives the
    /// assigned seq so the entry's own `seq` field (part of its wire
    /// shape) always matches the log's bookkeeping -- there is exactly
    /// one place a sequence number is minted.
    pub fn append_with_seq(&mut self, build_entry: impl FnOnce(u64) -> T) -> Result<Seq> {
        let seq = self.next_seq;
        let entry = build_entry(seq.0);
        self.sink.append(&entry).map_err(|source| MemoryError::Io {
            path: self.path.clone(),
            source: std::io::Error::other(source.to_string()),
        })?;
        self.next_seq = seq.next();
        Ok(seq)
    }

    /// The next sequence number that will be assigned (== current log
    /// length / high-watermark).
    pub fn high_watermark(&self) -> u64 {
        self.next_seq.0
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Verify the chain (against the independently-persisted `.chain`
/// sidecar) and parse every entry, quarantining rows that fail to
/// deserialize or break `seq` monotonicity rather than aborting the
/// whole read. A chain-level tamper (digest mismatch, reordering,
/// truncation) is NOT quarantinable -- it means the log's integrity
/// itself is compromised, so it is a hard error.
pub fn read_verified<T>(path: &Path, extract_seq: impl Fn(&T) -> u64) -> Result<ReadOutcome<T>>
where
    T: DeserializeOwned + Clone,
{
    if !path.exists() {
        return Ok(ReadOutcome {
            entries: Vec::new(),
            quarantined: Vec::new(),
        });
    }

    let chain_outcome =
        enforcer_core::telemetry::verify_file_chain(path).map_err(|source| MemoryError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(source.to_string()),
        })?;
    if let Err(break_) = chain_outcome {
        return Err(MemoryError::ChainTamper {
            path: path.to_path_buf(),
            line_index: break_.index,
            recorded: break_.recorded,
            expected: break_.expected,
        });
    }

    let raw = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut entries = Vec::new();
    let mut quarantined = Vec::new();
    let mut expected_seq: u64 = 0;

    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(entry) => {
                let seq = extract_seq(&entry);
                if seq != expected_seq {
                    quarantined.push(QuarantinedRow {
                        index,
                        reason: format!("sequence gap: expected seq {expected_seq}, found {seq}"),
                    });
                    // Resynchronize so a single gap does not cascade into
                    // quarantining every subsequent well-formed row.
                    expected_seq = seq + 1;
                    continue;
                }
                expected_seq += 1;
                entries.push(entry);
            }
            Err(e) => {
                quarantined.push(QuarantinedRow {
                    index,
                    reason: format!("failed to deserialize: {e}"),
                });
            }
        }
    }

    Ok(ReadOutcome {
        entries,
        quarantined,
    })
}

/// Current number of data lines already in the log at `path` (0 if the
/// file does not yet exist). Used to resume `Seq` assignment on reopen.
fn current_log_len(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let raw = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ObservationLogEntry, SCHEMA_VERSION};

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "enforcer-memory-log-{}-{}-{name}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(enforcer_core::telemetry::chain_sidecar_path(path));
    }

    fn read_file(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    fn write_file(path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content).map_err(|source| MemoryError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    fn sample(seq: u64) -> ObservationLogEntry {
        ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: format!("obs-{seq:04}"),
            lesson_id: "L1".to_owned(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-memory".to_owned(),
            clean: true,
            source_surface: "scan".to_owned(),
            ts: "2026-07-04T00:00:00Z".to_owned(),
            supersedes_seq: None,
        }
    }

    #[test]
    fn append_assigns_gap_free_seq_and_reads_back() -> Result<()> {
        let path = temp_path("append");
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            let s0 = log.append_with_seq(sample)?;
            let s1 = log.append_with_seq(sample)?;
            assert_eq!(s0.0, 0);
            assert_eq!(s1.0, 1);
            assert_eq!(log.high_watermark(), 2);
        }
        let outcome = read_verified::<ObservationLogEntry>(&path, |e| e.seq)?;
        assert_eq!(outcome.entries.len(), 2);
        assert!(outcome.quarantined.is_empty());
        cleanup(&path);
        Ok(())
    }

    #[test]
    fn supersede_records_the_relation_without_deleting_the_earlier_row() -> Result<()> {
        let path = temp_path("supersede");
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            log.append_with_seq(sample)?;
            log.append_with_seq(|seq| {
                let mut e = sample(seq);
                e.supersedes_seq = Some(0);
                e.clean = false;
                e
            })?;
        }
        let outcome = read_verified::<ObservationLogEntry>(&path, |e| e.seq)?;
        assert_eq!(outcome.entries.len(), 2, "supersede appends, never deletes");
        assert_eq!(outcome.entries[1].supersedes_seq, Some(0));
        assert!(outcome.entries[0].clean);
        assert!(!outcome.entries[1].clean);
        cleanup(&path);
        Ok(())
    }

    #[test]
    fn corrupt_row_is_quarantined_not_dropped_silently() -> Result<()> {
        let path = temp_path("corrupt-row");
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            log.append_with_seq(sample)?;
            log.append_with_seq(sample)?;
        }
        // Corrupt line 1 (0-indexed) so it no longer deserializes as
        // ObservationLogEntry, but leave its recorded chain digest intact
        // -- this must be caught by seq/shape validation, and since the
        // corruption trips the chain (payload changed under a fixed
        // digest), verification fails closed as a tamper, which is
        // stricter than quarantine and still correct behavior. To
        // exercise pure row-level corruption (not a chain break) we
        // instead corrupt via a controlled append that produces a
        // structurally-odd-but-still-chain-valid row: a duplicate seq.
        cleanup(&path);
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            log.append_with_seq(|_seq| sample(0))?;
            // Force a duplicate/gapped seq by writing seq=0 again instead
            // of the log-assigned value.
            log.append_with_seq(|_seq| sample(0))?;
        }
        let outcome = read_verified::<ObservationLogEntry>(&path, |e| e.seq)?;
        assert_eq!(
            outcome.entries.len(),
            1,
            "only the first seq=0 row is accepted"
        );
        assert_eq!(
            outcome.quarantined.len(),
            1,
            "the duplicate seq row is quarantined"
        );
        assert!(outcome.quarantined[0].reason.contains("sequence gap"));
        cleanup(&path);
        Ok(())
    }

    #[test]
    fn tampered_chain_is_rejected_against_independent_sidecar() -> Result<()> {
        // L37: the tampering test that proves verify REJECTS a modified
        // row by comparing against the independently-persisted .chain
        // sidecar, never by recomputing "expected" from the same
        // (tampered) data file.
        let path = temp_path("tamper");
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            log.append_with_seq(sample)?;
            log.append_with_seq(sample)?;
        }
        let sidecar_before = read_file(&enforcer_core::telemetry::chain_sidecar_path(&path))?;

        let raw = read_file(&path)?;
        let tampered = raw.replacen("\"clean\":true", "\"clean\":false", 1);
        assert_ne!(tampered, raw, "tamper must actually change bytes");
        write_file(&path, &tampered)?;

        // Sidecar is untouched -- exactly the "independently persisted"
        // property under test.
        let sidecar_after = read_file(&enforcer_core::telemetry::chain_sidecar_path(&path))?;
        assert_eq!(sidecar_before, sidecar_after);

        let outcome = read_verified::<ObservationLogEntry>(&path, |e| e.seq);
        assert!(
            matches!(outcome, Err(MemoryError::ChainTamper { line_index: 0, .. })),
            "expected ChainTamper at line 0, got {outcome:?}"
        );
        cleanup(&path);
        Ok(())
    }

    #[test]
    fn malformed_json_row_is_quarantined() -> Result<()> {
        // A row that fails to deserialize at all (not just a seq gap)
        // must also be quarantined, not silently skipped and not a hard
        // abort of the whole read -- exercised independently of the
        // chain-tamper path above (this corrupts the file directly
        // without going through the sink, so the chain sidecar naturally
        // will not match either; we assert on the deserialize-failure
        // reason text when the row parses as *invalid JSON structure*
        // matched against T, via a schema-shape break that still yields
        // valid JSON but wrong types).
        let path = temp_path("malformed-json");
        {
            let mut log: AppendLog<ObservationLogEntry> = AppendLog::open(&path)?;
            log.append_with_seq(sample)?;
        }
        // Simulate a corrupt data file whose sidecar was rebuilt to
        // match (models e.g. an offline repair tool that resyncs the
        // sidecar but leaves row-shape corruption for the row-level
        // quarantine path to catch).
        write_file(&path, "{\"not\":\"a valid ObservationLogEntry\"}\n")?;
        let digest = enforcer_core::hash_chain::link_digest(
            None,
            "{\"not\":\"a valid ObservationLogEntry\"}".as_bytes(),
        );
        write_file(
            &enforcer_core::telemetry::chain_sidecar_path(&path),
            &format!("{digest}\n"),
        )?;
        let outcome = read_verified::<ObservationLogEntry>(&path, |e| e.seq)?;
        assert!(outcome.entries.is_empty());
        assert_eq!(outcome.quarantined.len(), 1);
        assert!(outcome.quarantined[0]
            .reason
            .contains("failed to deserialize"));
        cleanup(&path);
        Ok(())
    }
}
