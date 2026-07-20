//! Content-addressed artifact manifest and index manifests (with source
//! high-watermark staleness detection).

use std::collections::BTreeMap;

use crate::boundary::log_schema::{ArtifactManifestEntryDto, IndexManifestDto, SCHEMA_VERSION};
use crate::error::{MemoryError, Result};
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    ArtifactId, ArtifactManifestEntryCount, ArtifactManifestEntryKey, ArtifactManifestIsEmpty,
    ArtifactManifestRelativePath, ArtifactManifestTimestamp, GraphArtifactByteCount,
    IndexManifestBuiltAt, IndexManifestSourceLog, IndexManifestWatermark, MemoryArtifactBytes,
    MemoryStorePath,
};

/// A content-addressed artifact store: `put` writes content keyed by its
/// own SHA-256 digest (so identical content is stored once, and the id
/// is never caller-assigned), `get` re-verifies the digest on every read
/// so a corrupted blob is detected rather than silently returned.
#[derive(Debug)]
pub struct ArtifactManifest {
    root: MemoryStorePath,
    entries: BTreeMap<ArtifactManifestEntryKey, ArtifactManifestEntryDto>,
    manifest_path: MemoryStorePath,
}

impl ArtifactManifest {
    /// Open (or create) the artifact manifest rooted at `root`. Loads
    /// any existing `manifest.json` index of entries.
    pub fn open(root: impl Into<MemoryStorePath>) -> Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|source| MemoryError::Io {
            path: root.to_path_buf().into(),
            source,
        })?;
        let manifest_path = root.join("manifest.json");
        let entries = if manifest_path.exists() {
            let raw =
                std::fs::read_to_string(&manifest_path).map_err(|source| MemoryError::Io {
                    path: manifest_path.retained().into(),
                    source,
                })?;
            crate::boundary::json::decode(&raw)?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            root,
            entries,
            manifest_path: manifest_path.into(),
        })
    }

    /// Store `content`, keyed by its own content-addressed id. If an
    /// artifact with this exact content already exists, this is a no-op
    /// dedup (same id, no rewrite). Returns the artifact id.
    pub fn put(
        &mut self,
        content: impl Into<MemoryArtifactBytes>,
        rel_path: Option<ArtifactManifestRelativePath>,
        ts: impl Into<ArtifactManifestTimestamp>,
    ) -> Result<ArtifactId> {
        let content = content.into();
        let ts = ts.into();
        let id = ArtifactId::from_content(content.as_ref());
        let blob_path = self.blob_path(&id);
        if !blob_path.exists() {
            std::fs::write(&blob_path, content.as_ref()).map_err(|source| MemoryError::Io {
                path: blob_path.as_ref().into(),
                source,
            })?;
        }
        self.entries.insert(
            id.as_str().into(),
            ArtifactManifestEntryDto {
                schema_version: SCHEMA_VERSION,
                id: enforcer_domain::memory_types::ArtifactId::from_content(content.as_ref()),
                rel_path,
                byte_len: match GraphArtifactByteCount::try_from(content.as_ref().len()) {
                    Ok(byte_count) => byte_count,
                    Err(_) => {
                        return Err(MemoryError::ModelRuntime {
                            operation: "write-artifact".into(),
                            reason: "artifact byte count exceeds the supported range".into(),
                        })
                    }
                },
                ts,
            },
        );
        self.persist()?;
        Ok(id)
    }

    /// Read the content stored under `id`, re-verifying its digest
    /// against the id itself — a corrupted blob (bytes on disk no longer
    /// hash to `id`) is a hard error, never a silent wrong-content
    /// return.
    pub fn get(&self, id: &ArtifactId) -> Result<MemoryArtifactBytes> {
        let blob_path = self.blob_path(id);
        let content = std::fs::read(&blob_path).map_err(|source| MemoryError::Io {
            path: blob_path.as_ref().into(),
            source,
        })?;
        let actual = ArtifactId::from_content(&content);
        if actual.as_str() != id.as_str() {
            return Err(MemoryError::ArtifactDigestMismatch {
                id: id.as_str().retained().into(),
                expected: id.as_str().retained().into(),
                actual: actual.as_str().retained().into(),
            });
        }
        Ok(content.into())
    }

    pub fn entry(&self, id: &ArtifactId) -> Option<&ArtifactManifestEntryDto> {
        self.entries
            .iter()
            .find_map(|(key, entry)| (key.as_str() == id.as_str()).then_some(entry))
    }

    pub fn len(&self) -> ArtifactManifestEntryCount {
        self.entries.len().into()
    }

    pub fn is_empty(&self) -> ArtifactManifestIsEmpty {
        self.entries.is_empty().into()
    }

    fn blob_path(&self, id: &ArtifactId) -> MemoryStorePath {
        self.root.join(id.digest().hex()).into()
    }

    fn persist(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.manifest_path, json).map_err(|source| MemoryError::Io {
            path: self.manifest_path.as_ref().to_path_buf().into(),
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
    path: impl Into<MemoryStorePath>,
    current_log_length: impl Into<IndexManifestWatermark>,
) -> Result<Option<IndexManifestDto>> {
    let path = path.into();
    let current_log_length = current_log_length.into();
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;
    let manifest: IndexManifestDto = crate::boundary::json::decode(&raw)?;
    if manifest.source_high_watermark < current_log_length {
        return Err(MemoryError::StaleIndex {
            path: path.to_path_buf().into(),
            manifest_watermark: manifest.source_high_watermark.get().into(),
            log_length: current_log_length.get().into(),
        });
    }
    Ok(Some(manifest))
}

/// Write a fresh index manifest recording `source_log`'s current length
/// as the high-watermark this index was built against.
pub fn write_index_manifest(
    path: impl Into<MemoryStorePath>,
    source_log: impl Into<IndexManifestSourceLog>,
    source_high_watermark: impl Into<IndexManifestWatermark>,
    built_at: impl Into<IndexManifestBuiltAt>,
) -> Result<()> {
    let path = path.into();
    let source_log = source_log.into();
    let source_high_watermark = source_high_watermark.into();
    let built_at = built_at.into();
    let manifest = IndexManifestDto {
        schema_version: SCHEMA_VERSION,
        source_log,
        source_high_watermark,
        built_at,
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&path, json).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })
}
