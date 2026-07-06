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
