use enforcer_memory::error::{MemoryError, Result};
use enforcer_memory::log::{read_verified, AppendLog};
use enforcer_memory::schema::{ObservationLogEntry, SCHEMA_VERSION};
use std::path::{Path, PathBuf};

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
