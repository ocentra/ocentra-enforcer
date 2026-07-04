//! Run-telemetry sink (d04): wires the generic [`crate::ndjson_writer`]
//! append-only writer, the two-layer [`crate::redaction`], and the
//! [`crate::hash_chain`] primitive together into ONE append operation for
//! any run record type. `enforcer-core` stays domain-agnostic — this module
//! is generic over `T`; the `enforcer-domain::run_record::RunRecord` shape
//! rides it (see `crates/enforcer-core/tests/telemetry.rs` for the
//! domain-level integration proof, which takes `enforcer-domain` as a
//! dev-dependency only — no cycle in the normal build graph).
//!
//! Contract:
//! - Both redaction layers ALWAYS run over the record's JSON form before it
//!   is written anywhere (never bypassable).
//! - The record is serde-validated at the decode boundary: `append` fails
//!   closed if the record cannot round-trip through `serde_json::Value` and
//!   back through `T`'s own `Deserialize` impl (guards against a `Serialize`
//!   impl that produces a shape its own `Deserialize` cannot parse back).
//! - Append is atomic and newline-terminated (delegated to
//!   [`crate::ndjson_writer::NdjsonWriter`], which opens append-only and
//!   flushes every line — a crash mid-write leaves at most one truncated
//!   trailing line, never a torn record injected earlier in the file).
//! - Each appended line is also fed through the hash-chain so replay can
//!   verify the whole run-telemetry stream has not been tampered with.
//! - Telemetry emission is an OBSERVER: this sink never inspects or
//!   influences a process exit code; callers write telemetry independently
//!   of whatever exit path they take.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::hash_chain::link_digest;
use crate::redaction::Redactor;

/// Stable default path (relative to the repo/working root) that run
/// telemetry is appended to.
pub const DEFAULT_RUN_TELEMETRY_PATH: &str = "proof/telemetry/runs.ndjson";

/// Default path as a [`PathBuf`], for callers that want an owned value.
pub fn default_run_telemetry_path() -> PathBuf {
    PathBuf::from(DEFAULT_RUN_TELEMETRY_PATH)
}

/// Sidecar-file suffix holding one recorded hash-chain digest per data
/// line, in order. Kept as a separate append-only file (rather than a field
/// embedded in `T`) so the digest manifest is independent of, and does not
/// leak into, the record schema `T` owns.
pub const CHAIN_SIDECAR_SUFFIX: &str = ".chain";

/// The sidecar path that records the persisted hash-chain digests for a
/// given NDJSON data file.
pub fn chain_sidecar_path(data_path: &Path) -> PathBuf {
    let mut os_string = data_path.as_os_str().to_owned();
    os_string.push(CHAIN_SIDECAR_SUFFIX);
    PathBuf::from(os_string)
}

/// A run-telemetry sink: one physical NDJSON file, redaction applied to
/// every record, with a running hash-chain digest over appended lines
/// persisted to a `.chain` sidecar so cold verification can detect
/// tampering with the data file (recomputing digests from the data file
/// alone cannot detect tampering — the recorded digest must be compared
/// against an independently-persisted value).
pub struct RunTelemetrySink<T> {
    data_file: std::fs::File,
    chain_sidecar: std::fs::File,
    redactor: Redactor,
    prev_digest: Option<String>,
    _record: std::marker::PhantomData<T>,
}

impl<T> RunTelemetrySink<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    /// Open (or create) the sink at `path`, starting a fresh hash-chain
    /// (genesis link — `prev_digest` starts at `None`). Re-opening an
    /// existing file continues appending to it but does NOT replay prior
    /// digests into memory; the in-memory chain only covers records
    /// appended through THIS sink instance — cold verification of a whole
    /// file's chain (across process restarts) goes through
    /// [`verify_file_chain`] instead, which reads the sidecar directly.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let data_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        let chain_sidecar = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(chain_sidecar_path(path))?;
        Ok(Self {
            data_file,
            chain_sidecar,
            redactor: Redactor::with_defaults()?,
            prev_digest: None,
            _record: std::marker::PhantomData,
        })
    }

    /// Redact, validate, and append one record as a single NDJSON line, and
    /// append its hash-chain digest (computed over those EXACT written
    /// bytes) to the `.chain` sidecar. Fails closed: if the record does not
    /// round-trip cleanly through `serde_json::Value` and back through
    /// `T::deserialize`, nothing is written to either file.
    pub fn append(&mut self, record: &T) -> Result<String> {
        let mut value = serde_json::to_value(record)?;
        self.redactor.redact(&mut value);

        // Fail-closed re-verification: the redacted shape must still decode
        // as `T`. The decoded copy is discarded — it only proves the
        // redacted `value` is a valid `T` before that exact JSON value is
        // serialized to the line we write and hash.
        let _: T = serde_json::from_value(value.clone()).map_err(|e| {
            Error::Decode(crate::error::DecodeError::new(
                "runTelemetry.record",
                format!("record failed fail-closed decode round-trip after redaction: {e}"),
            ))
        })?;
        let line = serde_json::to_string(&value)?;

        // Compute the digest over exactly the bytes that will be written,
        // so the sidecar and the data file can never silently diverge.
        let digest = link_digest(self.prev_digest.as_deref(), line.as_bytes());

        self.data_file.write_all(line.as_bytes())?;
        self.data_file.write_all(b"\n")?;
        self.data_file.flush()?;

        self.chain_sidecar.write_all(digest.as_bytes())?;
        self.chain_sidecar.write_all(b"\n")?;
        self.chain_sidecar.flush()?;

        self.prev_digest = Some(digest.clone());
        Ok(digest)
    }

    /// The most recently computed hash-chain digest, if any record has been
    /// appended through this sink instance yet.
    pub fn last_digest(&self) -> Option<&str> {
        self.prev_digest.as_deref()
    }
}

/// Verify the hash-chain over every line in an NDJSON file against the
/// digests recorded in its `.chain` sidecar (see [`chain_sidecar_path`]).
/// Comparing against the independently-persisted sidecar — rather than
/// recomputing digests from the data file alone — is what makes this able
/// to detect tampering with the data file: recomputation-only verification
/// against no external record would trivially "verify" any content,
/// tampered or not. Returns the number of verified lines or the first
/// broken link (index, recorded-vs-expected digest).
pub fn verify_file_chain(
    path: &Path,
) -> Result<std::result::Result<usize, crate::hash_chain::ChainBreak>> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&[u8]> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::as_bytes)
        .collect();

    let sidecar_content = std::fs::read_to_string(chain_sidecar_path(path))?;
    let recorded_digests: Vec<&str> = sidecar_content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if recorded_digests.len() != lines.len() {
        return Ok(Err(crate::hash_chain::ChainBreak {
            index: recorded_digests.len().min(lines.len()),
            recorded: format!("{} recorded digest(s)", recorded_digests.len()),
            expected: format!("{} data line(s)", lines.len()),
        }));
    }

    let links = lines.iter().copied().zip(recorded_digests.iter().copied());
    Ok(crate::hash_chain::verify_chain(links))
}

#[cfg(test)]
mod tests {
    use super::{RunTelemetrySink, DEFAULT_RUN_TELEMETRY_PATH};
    use crate::error::Result;

    #[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
    struct SampleRecord {
        seq: u32,
        secret: String,
        note: String,
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "enforcer-core-telemetry-{}-{}-{name}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    /// Remove both the data file and its `.chain` sidecar.
    fn cleanup(path: &std::path::Path) -> Result<()> {
        std::fs::remove_file(path)?;
        std::fs::remove_file(super::chain_sidecar_path(path))?;
        Ok(())
    }

    #[test]
    fn default_path_matches_the_documented_proof_location() {
        assert_eq!(DEFAULT_RUN_TELEMETRY_PATH, "proof/telemetry/runs.ndjson");
    }

    #[test]
    fn append_redacts_before_writing() -> Result<()> {
        let path = temp_path("redact");
        {
            let mut sink: RunTelemetrySink<SampleRecord> = RunTelemetrySink::open(&path)?;
            sink.append(&SampleRecord {
                seq: 1,
                secret: "hunter2".to_owned(),
                note: "clean".to_owned(),
            })?;
        }
        let raw = std::fs::read_to_string(&path)?;
        assert!(!raw.contains("hunter2"));
        assert!(raw.contains("[REDACTED]"));
        cleanup(&path)?;
        Ok(())
    }

    #[test]
    fn two_appends_produce_two_independently_parseable_lines_and_chain_verifies() -> Result<()> {
        let path = temp_path("chain");
        {
            let mut sink: RunTelemetrySink<SampleRecord> = RunTelemetrySink::open(&path)?;
            let d1 = sink.append(&SampleRecord {
                seq: 1,
                secret: "s1".to_owned(),
                note: "first".to_owned(),
            })?;
            let d2 = sink.append(&SampleRecord {
                seq: 2,
                secret: "s2".to_owned(),
                note: "second".to_owned(),
            })?;
            assert_ne!(d1, d2);
        }
        let raw = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let _: SampleRecord = serde_json::from_str(line)?;
        }
        let outcome = super::verify_file_chain(&path)?;
        assert_eq!(outcome, Ok(2));
        cleanup(&path)?;
        Ok(())
    }

    #[test]
    fn tampering_with_a_line_breaks_chain_verification() -> Result<()> {
        let path = temp_path("tamper");
        {
            let mut sink: RunTelemetrySink<SampleRecord> = RunTelemetrySink::open(&path)?;
            sink.append(&SampleRecord {
                seq: 1,
                secret: "s1".to_owned(),
                note: "first".to_owned(),
            })?;
            sink.append(&SampleRecord {
                seq: 2,
                secret: "s2".to_owned(),
                note: "second".to_owned(),
            })?;
        }
        let raw = std::fs::read_to_string(&path)?;
        assert!(
            raw.contains("\"seq\":1"),
            "fixture assumption failed, raw was: {raw}"
        );
        let tampered = raw.replacen("\"seq\":1", "\"seq\":999", 1);
        assert_ne!(
            tampered, raw,
            "tamper replacement must actually change content"
        );
        std::fs::write(&path, tampered)?;
        let outcome = super::verify_file_chain(&path)?;
        assert!(outcome.is_err(), "chain must break, got: {outcome:?}");
        cleanup(&path)?;
        Ok(())
    }
}
