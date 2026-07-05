//! Content-addressed artifact manifest and index manifests (with source
//! high-watermark staleness detection).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{MemoryError, Result};
use crate::ids::ArtifactId;
use crate::schema::{ArtifactManifestEntry, IndexManifest, SCHEMA_VERSION};

/// A content-addressed artifact store: `put` writes content keyed by its
/// own SHA-256 digest (so identical content is stored once, and the id
/// is never caller-assigned), `get` re-verifies the digest on every read
/// so a corrupted blob is detected rather than silently returned.
pub struct ArtifactManifest {
    root: PathBuf,
    entries: BTreeMap<String, ArtifactManifestEntry>,
    manifest_path: PathBuf,
}

impl ArtifactManifest {
    /// Open (or create) the artifact manifest rooted at `root`. Loads
    /// any existing `manifest.json` index of entries.
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).map_err(|source| MemoryError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let manifest_path = root.join("manifest.json");
        let entries = if manifest_path.exists() {
            let raw =
                std::fs::read_to_string(&manifest_path).map_err(|source| MemoryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?;
            serde_json::from_str(&raw)?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            root: root.to_path_buf(),
            entries,
            manifest_path,
        })
    }

    /// Store `content`, keyed by its own content-addressed id. If an
    /// artifact with this exact content already exists, this is a no-op
    /// dedup (same id, no rewrite). Returns the artifact id.
    pub fn put(&mut self, content: &[u8], rel_path: Option<&str>, ts: &str) -> Result<ArtifactId> {
        let id = ArtifactId::from_content(content);
        let blob_path = self.blob_path(&id);
        if !blob_path.exists() {
            std::fs::write(&blob_path, content).map_err(|source| MemoryError::Io {
                path: blob_path.clone(),
                source,
            })?;
        }
        self.entries.insert(
            id.as_str().to_owned(),
            ArtifactManifestEntry {
                schema_version: SCHEMA_VERSION,
                id: id.as_str().to_owned(),
                rel_path: rel_path.map(str::to_owned),
                byte_len: content.len() as u64,
                ts: ts.to_owned(),
            },
        );
        self.persist()?;
        Ok(id)
    }

    /// Read the content stored under `id`, re-verifying its digest
    /// against the id itself — a corrupted blob (bytes on disk no longer
    /// hash to `id`) is a hard error, never a silent wrong-content
    /// return.
    pub fn get(&self, id: &ArtifactId) -> Result<Vec<u8>> {
        let blob_path = self.blob_path(id);
        let content = std::fs::read(&blob_path).map_err(|source| MemoryError::Io {
            path: blob_path,
            source,
        })?;
        let actual = ArtifactId::from_content(&content);
        if actual.as_str() != id.as_str() {
            return Err(MemoryError::ArtifactDigestMismatch {
                id: id.as_str().to_owned(),
                expected: id.as_str().to_owned(),
                actual: actual.as_str().to_owned(),
            });
        }
        Ok(content)
    }

    pub fn entry(&self, id: &ArtifactId) -> Option<&ArtifactManifestEntry> {
        self.entries.get(id.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn blob_path(&self, id: &ArtifactId) -> PathBuf {
        self.root.join(id.digest().hex())
    }

    fn persist(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.manifest_path, json).map_err(|source| MemoryError::Io {
            path: self.manifest_path.clone(),
            source,
        })
    }
}

/// Load the index manifest at `path`, if any, and compare its recorded
/// `source_high_watermark` against `current_log_length`. Returns
/// [`MemoryError::StaleIndex`] if the manifest is behind the log (the
/// index must be rebuilt before being trusted for reads). Returns
/// `Ok(None)` if no manifest exists yet (nothing to be stale relative
/// to).
pub fn check_index_freshness(
    path: &Path,
    current_log_length: u64,
) -> Result<Option<IndexManifest>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let manifest: IndexManifest = serde_json::from_str(&raw)?;
    if manifest.source_high_watermark < current_log_length {
        return Err(MemoryError::StaleIndex {
            path: path.to_path_buf(),
            manifest_watermark: manifest.source_high_watermark,
            log_length: current_log_length,
        });
    }
    Ok(Some(manifest))
}

/// Write a fresh index manifest recording `source_log`'s current length
/// as the high-watermark this index was built against.
pub fn write_index_manifest(
    path: &Path,
    source_log: &str,
    source_high_watermark: u64,
    built_at: &str,
) -> Result<()> {
    let manifest = IndexManifest {
        schema_version: SCHEMA_VERSION,
        source_log: source_log.to_owned(),
        source_high_watermark,
        built_at: built_at.to_owned(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(path, json).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let id1 = manifest.put(b"hello world", Some("a.txt"), "2026-07-04T00:00:00Z")?;
        let id2 = manifest.put(b"hello world", Some("b.txt"), "2026-07-04T00:00:01Z")?;
        assert_eq!(id1, id2, "identical content dedups to the same id");
        assert_eq!(manifest.len(), 1);
        let content = manifest.get(&id1)?;
        assert_eq!(content, b"hello world");
        std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io { path: root, source })?;
        Ok(())
    }

    #[test]
    fn get_detects_a_corrupted_blob() -> Result<()> {
        let root = temp_dir("artifact-corrupt");
        let mut manifest = ArtifactManifest::open(&root)?;
        let id = manifest.put(b"original content", None, "2026-07-04T00:00:00Z")?;
        let blob_path = root.join(id.digest().hex());
        std::fs::write(&blob_path, b"corrupted!!").map_err(|source| MemoryError::Io {
            path: blob_path,
            source,
        })?;
        let outcome = manifest.get(&id);
        assert!(matches!(
            outcome,
            Err(MemoryError::ArtifactDigestMismatch { .. })
        ));
        std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io { path: root, source })?;
        Ok(())
    }

    #[test]
    fn stale_index_is_rejected_and_fresh_index_is_accepted() -> Result<()> {
        let root = temp_dir("index-freshness");
        std::fs::create_dir_all(&root).map_err(|source| MemoryError::Io {
            path: root.clone(),
            source,
        })?;
        let manifest_path = root.join("index.json");

        // No manifest yet: nothing to be stale relative to.
        assert!(check_index_freshness(&manifest_path, 5)?.is_none());

        write_index_manifest(&manifest_path, "observation", 5, "2026-07-04T00:00:00Z")?;
        // Fresh: manifest watermark == current log length.
        assert!(check_index_freshness(&manifest_path, 5)?.is_some());

        // Log grew past the manifest's recorded watermark: stale.
        let outcome = check_index_freshness(&manifest_path, 8);
        assert!(matches!(outcome, Err(MemoryError::StaleIndex { .. })));

        std::fs::remove_dir_all(&root).map_err(|source| MemoryError::Io { path: root, source })?;
        Ok(())
    }
}
