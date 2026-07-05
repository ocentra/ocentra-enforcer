//! Local-only model cache manifest loading and validation.
//!
//! This is the Enforcer-side cache contract, not a downloader. It can prove
//! whether locally installed llama.cpp/GGUF or ONNX artifacts are present and
//! hash-compatible; network acquisition remains a separate explicit step.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};
use crate::local_runtime::{
    infer_artifact_kind, LocalRuntimeAcceleration, LocalRuntimeArtifact, LocalRuntimeArtifactKind,
    LocalRuntimeBackend, LocalRuntimeCandidate,
};
use crate::model_runtime::{
    CacheCorruptionReasonCode, CacheHealth, CacheState, CacheUnavailableReason, ManifestIntegrity,
    ModelCacheStatus, ModelTask, SourcePolicy,
};

pub const MODEL_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheManifest {
    pub schema_version: u32,
    pub backend: LocalRuntimeBackend,
    pub task: ModelTask,
    pub model_id: String,
    pub revision: String,
    pub acceleration: LocalRuntimeAcceleration,
    pub artifacts: Vec<ModelCacheArtifactEntry>,
}

impl ModelCacheManifest {
    pub fn to_candidate(&self, manifest_dir: &Path) -> LocalRuntimeCandidate {
        LocalRuntimeCandidate {
            backend: self.backend,
            task: self.task,
            model_id: self.model_id.clone(),
            acceleration: self.acceleration,
            source_policy: SourcePolicy::LocalCache,
            artifacts: self
                .artifacts
                .iter()
                .map(|artifact| artifact.to_runtime_artifact(manifest_dir))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheArtifactEntry {
    pub kind: Option<LocalRuntimeArtifactKind>,
    pub path: String,
    pub sha256: String,
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming_manifest_path: Option<String>,
}

impl ModelCacheArtifactEntry {
    fn to_runtime_artifact(&self, manifest_dir: &Path) -> LocalRuntimeArtifact {
        let path = manifest_dir.join(&self.path);
        LocalRuntimeArtifact {
            kind: self.kind.unwrap_or_else(|| {
                infer_artifact_kind(&self.path).unwrap_or(LocalRuntimeArtifactKind::Manifest)
            }),
            path,
            sha256: Some(self.sha256.clone()),
            size_bytes: self.size_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheValidation {
    pub status: ModelCacheStatus,
    pub manifest: Option<ModelCacheManifest>,
    pub candidate: Option<LocalRuntimeCandidate>,
}

pub fn load_model_cache_manifest(path: &Path) -> Result<ModelCacheManifest> {
    let text = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let manifest: ModelCacheManifest = serde_json::from_str(&text)?;
    validate_manifest_shape(&manifest)?;
    Ok(manifest)
}

pub fn validate_model_cache_manifest(path: &Path) -> Result<ModelCacheValidation> {
    let checked_at = "local-cache-validation".to_owned();
    if !path.exists() {
        return Ok(ModelCacheValidation {
            status: ModelCacheStatus::unavailable(
                path.display().to_string(),
                checked_at,
                CacheUnavailableReason::ManifestUnavailable,
            ),
            manifest: None,
            candidate: None,
        });
    }

    let manifest = load_model_cache_manifest(path)?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new(""));
    let candidate = manifest.to_candidate(manifest_dir);

    for artifact in &candidate.artifacts {
        if let Some(expected_sha256) = &artifact.sha256 {
            crate::model_runtime::validate_file_hash(
                &artifact.path,
                expected_sha256,
                "validate-model-cache-artifact",
            )?;
        }
    }

    let total_size = candidate
        .artifacts
        .iter()
        .map(|artifact| {
            artifact
                .size_bytes
                .unwrap_or_else(|| metadata_len(&artifact.path))
        })
        .sum();

    Ok(ModelCacheValidation {
        status: ModelCacheStatus {
            artifact_ref: manifest.model_id.clone(),
            manifest_ref: Some(path.display().to_string()),
            source_policy: SourcePolicy::LocalCache,
            cache_state: CacheState::CacheReady,
            cache_health: CacheHealth::Healthy,
            manifest_integrity: ManifestIntegrity::Verified,
            download_enabled: false,
            download_status: crate::model_runtime::DownloadStatus::DownloadDisabled,
            cache_byte_size: total_size,
            checked_at,
            unavailable_reason: None,
            storage_error: None,
            corruption_reason: None,
        },
        manifest: Some(manifest),
        candidate: Some(candidate),
    })
}

pub fn corrupted_cache_status(
    artifact_ref: impl Into<String>,
    manifest_ref: impl Into<String>,
    checked_at: impl Into<String>,
    reason: CacheCorruptionReasonCode,
) -> ModelCacheStatus {
    ModelCacheStatus {
        artifact_ref: artifact_ref.into(),
        manifest_ref: Some(manifest_ref.into()),
        source_policy: SourcePolicy::LocalCache,
        cache_state: CacheState::CacheCorrupted,
        cache_health: CacheHealth::Corrupted,
        manifest_integrity: ManifestIntegrity::Failed,
        download_enabled: false,
        download_status: crate::model_runtime::DownloadStatus::DownloadDisabled,
        cache_byte_size: 0,
        checked_at: checked_at.into(),
        unavailable_reason: None,
        storage_error: None,
        corruption_reason: Some(reason),
    }
}

fn validate_manifest_shape(manifest: &ModelCacheManifest) -> Result<()> {
    if manifest.schema_version != MODEL_CACHE_SCHEMA_VERSION {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest",
            reason: format!(
                "unsupported model cache schema version {}",
                manifest.schema_version
            ),
        });
    }
    if manifest.model_id.trim().is_empty() {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest",
            reason: "modelId must not be empty".to_owned(),
        });
    }
    if manifest.artifacts.is_empty() {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest",
            reason: "manifest must list at least one artifact".to_owned(),
        });
    }
    for artifact in &manifest.artifacts {
        crate::model_runtime::validate_sha256_hex(&artifact.sha256)?;
        if artifact.path.trim().is_empty() || PathBuf::from(&artifact.path).is_absolute() {
            return Err(MemoryError::ModelRuntime {
                operation: "validate-model-cache-manifest",
                reason: format!("artifact path must be relative: {:?}", artifact.path),
            });
        }
    }
    Ok(())
}

fn metadata_len(path: &Path) -> u64 {
    path.metadata().map(|metadata| metadata.len()).unwrap_or(0)
}
