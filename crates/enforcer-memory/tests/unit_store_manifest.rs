use enforcer_memory::error::{MemoryError, Result};
use enforcer_memory::store::manifest::{
    check_index_freshness, write_index_manifest, ArtifactManifest,
};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let unique = format!(
        "enforcer-memory-manifest-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

#[test]
fn put_get_round_trips_and_dedups_identical_content() -> Result<()> {
    let root = temp_dir("artifact-roundtrip");
    let mut manifest = ArtifactManifest::open(&root)?;
    let id1 = manifest.put(b"hello world", Some("a.txt".into()), "2026-07-04T00:00:00Z")?;
    let id2 = manifest.put(b"hello world", Some("b.txt".into()), "2026-07-04T00:00:01Z")?;
    assert_eq!(id1, id2, "identical content dedups to the same id");
    assert_eq!(manifest.len(), 1);
    let content = manifest.get(&id1)?;
    assert_eq!(content, b"hello world");
    std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io {
        path: root.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn get_detects_a_corrupted_blob() -> Result<()> {
    let root = temp_dir("artifact-corrupt");
    let mut manifest = ArtifactManifest::open(&root)?;
    let id = manifest.put(b"original content", None, "2026-07-04T00:00:00Z")?;
    let blob_path = root.join(id.digest().hex());
    std::fs::write(&blob_path, b"corrupted!!").map_err(|source| MemoryError::Io {
        path: blob_path.into(),
        source,
    })?;
    let outcome = manifest.get(&id);
    assert!(matches!(
        outcome,
        Err(MemoryError::ArtifactDigestMismatch { .. })
    ));
    std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io {
        path: root.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn stale_index_is_rejected_and_fresh_index_is_accepted() -> Result<()> {
    let root = temp_dir("index-freshness");
    std::fs::create_dir_all(&root).map_err(|source| MemoryError::Io {
        path: root.clone().into(),
        source,
    })?;
    let manifest_path = root.join("index.json");

    // No manifest yet: nothing to be stale relative to.
    assert!(check_index_freshness(&manifest_path, 5)?.is_none());

    write_index_manifest(&manifest_path, "observation", 5, "2026-07-04T00:00:00Z")?;
    // Fresh: manifest watermark == current log length.
    let manifest = check_index_freshness(&manifest_path, 5)?.ok_or_else(|| {
        MemoryError::InternalInvariant {
            operation: "read fresh index manifest".to_string().into(),
            reason: "fresh manifest unexpectedly absent".to_string().into(),
        }
    })?;
    assert_eq!(manifest.source_high_watermark, 5);

    // Log grew past the manifest's recorded watermark: stale.
    let outcome = check_index_freshness(&manifest_path, 8);
    assert!(matches!(outcome, Err(MemoryError::StaleIndex { .. })));

    std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io {
        path: root.into(),
        source,
    })?;
    Ok(())
}
