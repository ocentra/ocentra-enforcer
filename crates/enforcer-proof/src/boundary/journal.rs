//! Append-only SHA-256 hash-chained NDJSON persistence boundary.
//!
//! CONSUMES `enforcer-core`'s generic append-only
//! [`enforcer_core::ndjson_writer::NdjsonWriter`] sink and the pure
//! [`enforcer_core::hash_chain`] primitive — no crypto or file-append logic
//! is reimplemented here. This module only adds the proof-specific
//! envelope: a versioned [`JournalRecordEnvelope`] wire shape (`schema_version` +
//! `event_type`) whose on-disk line ALSO carries the hash-chain digest, and
//! **verify-on-open + verify-on-replay** so any break in the chain —
//! tampered payload, reordered line, truncated/missing link — fails closed
//! instead of being silently accepted.
//! Malformed or tampered chains are rejected by the negative open and replay
//! tests in this module.

use std::path::{Path, PathBuf};

use enforcer_core::error::{Error, Result};
use enforcer_core::hash_chain::{link_digest, verify_chain};
use enforcer_core::ndjson_writer::NdjsonWriter;
use enforcer_core::redaction::Redactor;
use enforcer_domain::core_types::ChainBreak;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};

// ROUNDTRIP-TEST: journal append/open/replay tests serialize and deserialize every record.

/// Current journal record schema version. Bumped only on a wire-incompatible
/// shape change.
pub const JOURNAL_SCHEMA_VERSION: u32 = 1;

/// One journal event kind. Kept as a plain string on the wire
/// (`eventType`) so new proof lifecycle events can be added without a
/// journal schema bump, matching the legacy `events.ndjson` `type` field's
/// role while riding the versioned-record contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecordEnvelope {
    /// Journal record schema version.
    pub schema_version: u32,
    /// Event kind, e.g. `proof-started`, `proof-finished`,
    /// `legacy-artifacts-imported`.
    pub event_type: JournalEventType,
    /// Proof run this event belongs to.
    pub run_id: ProofRunId,
    /// Proof id this event belongs to.
    pub proof_id: ProofId,
    /// ISO-8601 timestamp of the event.
    pub timestamp: String,
    /// Free-form structured payload (already redacted before append).
    pub payload: serde_json::Value,
}

/// The on-disk line shape: the record plus the hash-chain digest that folds
/// in the previous line's digest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalLine {
    record: JournalRecordEnvelope,
    /// `sha256:<64 lowercase hex>` digest covering this line's canonical
    /// payload bytes AND the previous line's digest (or nothing, for the
    /// genesis line).
    digest: Sha256,
}

/// A chain break detected while verifying a journal file, translated to a
/// journal-relative description (which line, what was recorded vs.
/// expected).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "proof journal tamper detected at line {line_index} (recorded {recorded}, expected {expected})"
)]
pub struct JournalTamper {
    /// Zero-based line index of the first broken link.
    pub line_index: usize,
    /// The digest recorded on the broken line.
    pub recorded: String,
    /// The digest recomputed from payload + previous digest.
    pub expected: String,
}

impl From<ChainBreak> for JournalTamper {
    fn from(value: ChainBreak) -> Self {
        match value {
            ChainBreak::DigestMismatch {
                index,
                recorded,
                expected,
            } => Self {
                line_index: usize::from(index),
                recorded: recorded.to_string(),
                expected: expected.to_string(),
            },
            ChainBreak::LengthMismatch {
                index,
                recorded_digests,
                data_lines,
            } => Self {
                line_index: usize::from(index),
                recorded: format!("{} recorded digest(s)", usize::from(recorded_digests)),
                expected: format!("{} data line(s)", usize::from(data_lines)),
            },
        }
    }
}

/// An append-only, hash-chained NDJSON journal at `path`.
///
/// [`ProofJournal::open`] performs **verify-on-open**: an existing journal
/// is fully re-verified before any further record is appended, so a
/// tampered file is rejected before it can be extended.
pub struct ProofJournal {
    path: PathBuf,
    last_digest: Option<Sha256>,
}

impl ProofJournal {
    /// Open (or create) the journal at `path`, verifying the existing chain
    /// (if any) before returning. Fails closed on any break.
    pub fn open(path: &Path) -> Result<Self> {
        let last_digest = if path.exists() {
            let lines = read_lines(path)?;
            verify_lines(&lines).map_err(|e| journal_tamper_to_error(&e))?;
            lines.last().map(|line| line.digest.clone())
        } else {
            None
        };
        Ok(Self {
            path: path.to_path_buf(),
            last_digest,
        })
    }

    /// Append one record, redacting it through `redactor` first (two-layer
    /// redaction: key-name then value-pattern, both always run), folding
    /// the new line's digest over the previous line's digest.
    pub fn append(&mut self, redactor: &Redactor, record: JournalRecordEnvelope) -> Result<()> {
        let mut payload = record.payload.clone();
        redactor.redact(&mut payload);
        let redacted_record = JournalRecordEnvelope { payload, ..record };
        let canonical = serde_json::to_vec(&redacted_record)?;
        let digest = link_digest(self.last_digest.as_ref(), &canonical);
        let line = JournalLine {
            record: redacted_record,
            digest: digest.clone(),
        };
        let mut writer: NdjsonWriter<JournalLine> = NdjsonWriter::open(&self.path)?;
        writer.append(&line)?;
        self.last_digest = Some(digest);
        Ok(())
    }

    /// Re-read the journal from disk and **verify-on-replay**: recompute the
    /// whole chain from the genesis line, failing closed on any break. This
    /// is deliberately independent of [`ProofJournal::open`]'s in-memory
    /// state so a caller can re-validate a journal another process/run may
    /// have appended to since it was opened.
    pub fn verify_on_replay(&self) -> Result<usize> {
        let lines = read_lines(&self.path)?;
        verify_lines(&lines).map_err(|e| journal_tamper_to_error(&e))
    }

    /// The records currently on disk, in append order (test/inspection
    /// helper — production replay uses [`ProofJournal::verify_on_replay`]
    /// first).
    pub fn records(&self) -> Result<Vec<JournalRecordEnvelope>> {
        Ok(read_lines(&self.path)?
            .into_iter()
            .map(|line| line.record)
            .collect())
    }
}

fn read_lines(path: &Path) -> Result<Vec<JournalLine>> {
    enforcer_core::ndjson_writer::read_all(path)
}

fn verify_lines(lines: &[JournalLine]) -> std::result::Result<usize, JournalTamper> {
    let canonical: Vec<Vec<u8>> = lines
        .iter()
        .map(|line| serde_json::to_vec(&line.record).unwrap_or_default())
        .collect();
    let links = canonical
        .iter()
        .map(Vec::as_slice)
        .zip(lines.iter().map(|line| &line.digest));
    verify_chain(links).map_err(JournalTamper::from)
}

fn journal_tamper_to_error(tamper: &JournalTamper) -> Error {
    Error::InvalidConfig(tamper.to_string())
}

#[cfg(test)]
mod tests {
    use super::{JournalRecordEnvelope, ProofJournal, JOURNAL_SCHEMA_VERSION};
    use enforcer_core::error::Result;
    use enforcer_core::redaction::Redactor;
    use enforcer_domain::proof_types::{JournalEventType, ProofId, ProofRunId};
    use std::io::Write as _;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "enforcer-proof-journal-{}-{}-{name}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    fn sample_record(event_type: &str, secret: &str) -> Result<JournalRecordEnvelope> {
        Ok(JournalRecordEnvelope {
            schema_version: JOURNAL_SCHEMA_VERSION,
            event_type: JournalEventType::try_from(event_type.to_owned())?,
            run_id: ProofRunId::try_from("run-001".to_owned())?,
            proof_id: ProofId::try_from("PROOF-DEMO".to_owned())?,
            timestamp: "2026-07-04T00:00:00Z".to_owned(),
            payload: serde_json::json!({ "token": secret, "note": "clean" }),
        })
    }

    #[test]
    fn intact_chain_verifies_on_open_and_on_replay() -> Result<()> {
        let path = temp_path("intact");
        let redactor = Redactor::with_defaults()?;
        {
            let mut journal = ProofJournal::open(&path)?;
            journal.append(&redactor, sample_record("proof-started", "s3cr3t-token")?)?;
            journal.append(&redactor, sample_record("proof-finished", "s3cr3t-token")?)?;
        }
        // Re-open re-verifies the whole chain (verify-on-open).
        let journal = ProofJournal::open(&path)?;
        // Independently re-verify by replay.
        assert_eq!(journal.verify_on_replay()?, 2);
        let records = journal.records()?;
        assert_eq!(records.len(), 2);
        // Redaction ran: the secret-bearing key is redacted.
        assert_eq!(records[0].payload["token"], "[REDACTED]");
        assert_eq!(records[0].payload["note"], "clean");
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn tampered_record_fails_closed_on_open() -> Result<()> {
        let path = temp_path("tampered-open");
        let redactor = Redactor::with_defaults()?;
        {
            let mut journal = ProofJournal::open(&path)?;
            journal.append(&redactor, sample_record("proof-started", "x")?)?;
            journal.append(&redactor, sample_record("proof-finished", "x")?)?;
        }
        tamper_second_line(&path)?;
        let outcome = ProofJournal::open(&path);
        assert!(matches!(
            outcome,
            Err(enforcer_core::error::Error::InvalidConfig(ref detail))
                if detail.starts_with("proof journal tamper detected")
        ));
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn tampered_record_fails_closed_on_replay() -> Result<()> {
        let path = temp_path("tampered-replay");
        let redactor = Redactor::with_defaults()?;
        let journal = {
            let mut journal = ProofJournal::open(&path)?;
            journal.append(&redactor, sample_record("proof-started", "x")?)?;
            journal.append(&redactor, sample_record("proof-finished", "x")?)?;
            journal
        };
        tamper_second_line(&path)?;
        let outcome = journal.verify_on_replay();
        assert!(matches!(
            outcome,
            Err(enforcer_core::error::Error::InvalidConfig(ref detail))
                if detail.starts_with("proof journal tamper detected")
        ));
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn reordered_lines_fail_closed() -> Result<()> {
        let path = temp_path("reordered");
        let redactor = Redactor::with_defaults()?;
        {
            let mut journal = ProofJournal::open(&path)?;
            journal.append(&redactor, sample_record("proof-started", "x")?)?;
            journal.append(&redactor, sample_record("proof-finished", "x")?)?;
        }
        let content = std::fs::read_to_string(&path)?;
        let mut lines: Vec<&str> = content.lines().collect();
        lines.swap(0, 1);
        let swapped = lines.join("\n") + "\n";
        std::fs::write(&path, swapped)?;
        assert!(
            ProofJournal::open(&path).is_err(),
            "reordered lines must fail closed"
        );
        std::fs::remove_file(&path)?;
        Ok(())
    }

    /// Corrupt the payload of the second line while leaving its recorded
    /// digest untouched, simulating an at-rest tamper.
    fn tamper_second_line(path: &std::path::Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
        let mut value: serde_json::Value = serde_json::from_str(&lines[1])?;
        value["record"]["eventType"] = serde_json::json!("tampered");
        lines[1] = value.to_string();
        let mut file = std::fs::File::create(path)?;
        file.write_all(lines.join("\n").as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }
}
