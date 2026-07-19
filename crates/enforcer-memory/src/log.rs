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

use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{MemoryError, QuarantinedRow, Result};
use crate::owned_boundary::RetainedDisplay;
use enforcer_domain::core_types::ChainBreak;
use enforcer_domain::memory_types::{
    MemoryAppendLogPath, MemoryErrorLineIndex, MemoryErrorLogLength, Seq,
};

/// An append-only hash-chained log of `T` entries at `path`, with its
/// digest chain independently persisted to `path` + `.chain`.
/// Intentional Debug implementation: `impl std::fmt::Debug for AppendLog<T>`
/// below keeps the sink internals private while exposing path/high-watermark.
pub struct AppendLog<T> {
    path: MemoryAppendLogPath,
    sink: enforcer_core::telemetry::RunTelemetrySink<T>,
    next_seq: Seq,
}

impl<T> std::fmt::Debug for AppendLog<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppendLog")
            .field("path", &self.path)
            .field("next_seq", &self.next_seq)
            .finish_non_exhaustive()
    }
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
                path: path.to_path_buf().into(),
                source: std::io::Error::other(source.retained_display()),
            }
        })?;
        let next_seq = match current_log_len(path) {
            Ok(length) => std::num::NonZeroU64::new(length.get())
                .map(Seq::from_nonzero)
                .unwrap_or(Seq::GENESIS),
            Err(_) => Seq::GENESIS,
        };
        Ok(Self {
            path: path.into(),
            sink,
            next_seq,
        })
    }

    /// Append `build_entry(seq)` under the next gap-free sequence number,
    /// returning the assigned [`Seq`]. `build_entry` receives the
    /// assigned seq so the entry's own `seq` field (part of its wire
    /// shape) always matches the log's bookkeeping -- there is exactly
    /// one place a sequence number is minted.
    pub fn append_with_seq(&mut self, build_entry: impl FnOnce(Seq) -> T) -> Result<Seq> {
        let seq = self.next_seq;
        let entry = build_entry(seq);
        self.sink.append(&entry).map_err(|source| MemoryError::Io {
            path: self.path.as_path().into(),
            source: std::io::Error::other(source.retained_display()),
        })?;
        self.next_seq = seq.next();
        Ok(seq)
    }

    /// The next sequence number that will be assigned (== current log
    /// length / high-watermark).
    pub fn high_watermark(&self) -> Seq {
        self.next_seq
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

/// Verify the chain (against the independently-persisted `.chain`
/// sidecar) and parse every entry, quarantining rows that fail to
/// deserialize or break `seq` monotonicity rather than aborting the
/// whole read. A chain-level tamper (digest mismatch, reordering,
/// truncation) is NOT quarantinable -- it means the log's integrity
/// itself is compromised, so it is a hard error.
pub fn read_verified<T>(path: &Path, extract_seq: impl Fn(&T) -> Seq) -> Result<ReadOutcome<T>>
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
            path: path.to_path_buf().into(),
            source: std::io::Error::other(source.retained_display()),
        })?;
    if let Err(break_) = chain_outcome {
        // BRAND-INVARIANT: chain indices/counts are validated by the core
        // verifier before this durable error projection.
        let (line_index, recorded, expected) = match break_ {
            ChainBreak::DigestMismatch {
                index,
                recorded,
                expected,
            } => (
                {
                    let line_index: MemoryErrorLineIndex = index.into();
                    line_index
                },
                recorded.retained_display(),
                expected.retained_display(),
            ),
            ChainBreak::LengthMismatch {
                index,
                recorded_digests,
                data_lines,
            } => (
                {
                    let line_index: MemoryErrorLineIndex = index.into();
                    line_index
                },
                format!("{recorded_digests} recorded digest(s)"),
                format!("{data_lines} data line(s)"),
            ),
        };
        return Err(MemoryError::ChainTamper {
            path: path.to_path_buf().into(),
            line_index,
            recorded: recorded.into(),
            expected: expected.into(),
        });
    }

    let raw = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;

    let mut entries = Vec::new();
    let mut quarantined = Vec::new();
    let mut expected_seq = Seq::GENESIS;

    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match crate::boundary::json::decode::<T>(line) {
            Ok(entry) => {
                let seq = extract_seq(&entry);
                if seq != expected_seq {
                    quarantined.push(QuarantinedRow {
                        index: index.into(),
                        reason: format!("sequence gap: expected seq {expected_seq}, found {seq}")
                            .into(),
                    });
                    // Resynchronize so a single gap does not cascade into
                    // quarantining every subsequent well-formed row.
                    expected_seq = seq.next();
                    continue;
                }
                expected_seq = expected_seq.next();
                entries.push(entry);
            }
            Err(e) => {
                quarantined.push(QuarantinedRow {
                    index: index.into(),
                    reason: format!("failed to deserialize: {e}").into(),
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
fn current_log_len(path: &Path) -> Result<MemoryErrorLogLength> {
    if !path.exists() {
        return Ok(0.into());
    }
    let raw = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;
    let line_count = raw.lines().filter(|line| !line.trim().is_empty()).count();
    // BRAND-INVARIANT: line_count is the validated number of durable data rows.
    Ok(MemoryErrorLogLength::from(
        u64::try_from(line_count).unwrap_or(u64::MAX),
    ))
}
