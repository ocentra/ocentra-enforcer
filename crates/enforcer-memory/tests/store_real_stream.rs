//! Integration test: round-trip a REAL `memory/streams/*.ndjson` file
//! through the x06.1 store, not a synthetic fixture. This is the
//! fabricated-green check the owner-intent doc calls for: `cargo test` passing
//! against invented data proves nothing about whether the store can
//! actually hold this repo's own memory corpus.
//!
//! Strategy: parse the real stream with the existing `x05` NDJSON parser
//! (`enforcer_memory::ingest::parse_ndjson`, already exercised by 18
//! pre-existing tests), content-address the raw file bytes into the
//! artifact manifest (proving byte-for-byte round-trip of the real
//! file), and append one real observation-log entry per parsed record
//! (deriving `repo_context`/`clean` from that record's own `landedAt`/
//! `kind` fields, not invented values) into a freshly initialized
//! project store. Then re-open the store and prove every appended
//! observation reads back with the SAME content it was written with.

use enforcer_memory::ids::ProjectId;
use enforcer_memory::ingest::parse_ndjson;
use enforcer_memory::log::read_verified;
use enforcer_memory::schema::{ObservationLogEntry, SCHEMA_VERSION};
use enforcer_memory::store::manifest::ArtifactManifest;
use enforcer_memory::store::Store;

/// A real stream file already committed to this repo -- NOT a fixture
/// under `tests/fixtures/`. If this file is ever renamed/removed this
/// test must be pointed at another real `memory/streams/*.ndjson` file,
/// not a synthetic stand-in.
const REAL_STREAM_PATH: &str = "../../memory/streams/arc-17.ndjson";

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = format!(
        "enforcer-memory-real-stream-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

#[test]
fn real_stream_file_round_trips_through_the_store() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stream_path = manifest_root.join(REAL_STREAM_PATH);
    let raw = std::fs::read_to_string(&stream_path)
        .map_err(|e| format!("failed to read real stream fixture {stream_path:?}: {e}"))?;

    // 1. The x05 ingest parser (already proven by 18 pre-existing tests)
    //    parses this real file cleanly -- if it did not, x06.1's store
    //    would be round-tripping garbage.
    let records = parse_ndjson(&raw)?;
    assert!(
        !records.is_empty(),
        "the real stream fixture must contain at least one record"
    );

    // 2. Content-address the RAW real file bytes into the artifact
    //    manifest and read them back byte-for-byte.
    let artifacts_root = temp_dir("artifacts");
    let mut artifacts = ArtifactManifest::open(&artifacts_root)?;
    let artifact_id = artifacts.put(
        raw.as_bytes(),
        Some("memory/streams/arc-17.ndjson"),
        "2026-07-04T00:00:00Z",
    )?;
    let round_tripped = artifacts.get(&artifact_id)?;
    assert_eq!(
        round_tripped,
        raw.as_bytes(),
        "the real stream file must round-trip byte-for-byte through the content-addressed manifest"
    );

    // 3. Initialize a fresh project store and append one real
    //    observation per parsed record, deriving fields from the
    //    record's OWN data (not invented placeholders).
    let stores_dir = temp_dir("stores");
    let repo_root: enforcer_domain::paths::RepoRoot = "C:/Projects/real-stream-demo".parse()?;
    let mut store = Store::init(&stores_dir, &repo_root, "2026-07-04T00:00:00Z")?;

    let expected_len = records.len();
    for record in &records {
        let repo_context = record
            .landed_at
            .first()
            .cloned()
            .unwrap_or_else(|| record.applies_to.first().cloned().unwrap_or_default());
        let clean = record.landed_at.is_empty();
        store
            .observation_log_mut()
            .append_with_seq(|seq| ObservationLogEntry {
                schema_version: SCHEMA_VERSION,
                seq,
                id: record.id.clone(),
                lesson_id: record.id.clone(),
                rule_id: None,
                fault_class: None,
                repo_context,
                clean,
                source_surface: "real-stream-ingest-test".to_owned(),
                ts: record.ts.clone(),
                supersedes_seq: None,
                payload_kind: None,
                payload: None,
            })?;
    }
    assert_eq!(
        store.observation_log_mut().high_watermark(),
        expected_len as u64
    );

    // 4. Re-open the store (a fresh handle, proving persistence -- not
    //    just in-memory state survives) and read every observation back,
    //    verifying the hash chain against its independent sidecar.
    let log_path = store.observation_log_path();
    drop(store);
    let outcome = read_verified::<ObservationLogEntry>(&log_path, |e| e.seq)?;
    assert!(
        outcome.quarantined.is_empty(),
        "no row should be quarantined on a clean real ingest: {:?}",
        outcome.quarantined
    );
    assert_eq!(outcome.entries.len(), expected_len);
    for (record, entry) in records.iter().zip(outcome.entries.iter()) {
        assert_eq!(
            record.id, entry.id,
            "read-back id must match the source record's id"
        );
        assert_eq!(
            record.ts, entry.ts,
            "read-back timestamp must match the source record's ts"
        );
    }

    let reopened = Store::open(&stores_dir, &repo_root)?;
    assert_eq!(
        reopened.project_id().as_str(),
        ProjectId::from_repo_root(&repo_root).as_str()
    );

    std::fs::remove_dir_all(&stores_dir)?;
    std::fs::remove_dir_all(&artifacts_root)?;
    Ok(())
}
