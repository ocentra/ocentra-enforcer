//! Local-only model cache manifest loading and validation.
//!
//! This is the Enforcer-side cache contract, not a downloader. It can prove
//! whether locally installed llama.cpp/GGUF or ONNX artifacts are present and
//! hash-compatible; network acquisition remains a separate explicit step.

use std::path::{Path, PathBuf};

use crate::boundary::model_cache::{
    ModelCacheArtifactEntryDto, ModelCacheManifestDto, ModelCacheValidationDto,
};
use crate::error::{MemoryError, Result};
use crate::local_runtime::{
    infer_artifact_kind, LocalRuntimeArtifactDto, LocalRuntimeCandidateDto,
};
use crate::model_runtime::ModelCacheStatusDto;
use crate::owned_boundary::{Retained, RetainedDisplay};
use enforcer_domain::memory_types::{
    CacheCorruptionReasonCode, CacheHealth, CacheState, CacheUnavailableReason,
    LocalRuntimeArtifactKind, ManifestIntegrity, ModelCacheArtifactFile, ModelCacheArtifactRef,
    ModelCacheArtifactSizeBytes, ModelCacheCheckedAt, ModelCacheManifestRef, SourcePolicy,
};

pub const MODEL_CACHE_SCHEMA_VERSION: u32 = 1;

impl ModelCacheManifestDto {
    pub fn to_candidate(&self, manifest_dir: &Path) -> LocalRuntimeCandidateDto {
        LocalRuntimeCandidateDto {
            backend: self.backend,
            task: self.task,
            model_id: self.model_id.retained_display(),
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

impl ModelCacheArtifactEntryDto {
    fn to_runtime_artifact(&self, manifest_dir: &Path) -> LocalRuntimeArtifactDto {
        let path = manifest_dir.join(self.path.as_str());
        LocalRuntimeArtifactDto {
            kind: self.kind.unwrap_or_else(|| {
                infer_artifact_kind(&self.path).unwrap_or(LocalRuntimeArtifactKind::Manifest)
            }),
            path,
            sha256: Some(self.sha256.retained_display()),
            size_bytes: self.size_bytes.map(Into::into),
        }
    }
}

pub fn load_model_cache_manifest(path: &Path) -> Result<ModelCacheManifestDto> {
    let text = std::fs::read_to_string(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf().into(),
        source,
    })?;
    let manifest: ModelCacheManifestDto = crate::boundary::json::decode(&text)?;
    validate_manifest_shape(&manifest)?;
    Ok(manifest)
}

pub fn validate_model_cache_manifest(path: &Path) -> Result<ModelCacheValidationDto> {
    let checked_at = "local-cache-validation".retained();
    if !path.exists() {
        return Ok(ModelCacheValidationDto {
            status: ModelCacheStatusDto::unavailable(
                path.display().retained_display(),
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
                .map_or_else(
                    || metadata_len((&artifact.path).into()),
                    ModelCacheArtifactSizeBytes::from,
                )
                .get()
        })
        .sum();

    Ok(ModelCacheValidationDto {
        status: ModelCacheStatusDto {
            artifact_ref: manifest.model_id.retained_display(),
            manifest_ref: Some(path.display().retained_display()),
            source_policy: SourcePolicy::LocalCache,
            cache_state: CacheState::CacheReady,
            cache_health: CacheHealth::Healthy,
            manifest_integrity: ManifestIntegrity::Verified,
            download_enabled: false,
            download_status: enforcer_domain::memory_types::DownloadStatus::DownloadDisabled,
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
    artifact_ref: &ModelCacheArtifactRef,
    manifest_ref: &ModelCacheManifestRef,
    checked_at: &ModelCacheCheckedAt,
    reason: CacheCorruptionReasonCode,
) -> ModelCacheStatusDto {
    ModelCacheStatusDto {
        // ALLOC-JUSTIFICATION: the public wire status owns strings after typed cache inputs leave scope.
        artifact_ref: artifact_ref.as_str().retained(),
        // ALLOC-JUSTIFICATION: the public wire status owns strings after typed cache inputs leave scope.
        manifest_ref: Some(manifest_ref.as_str().retained()),
        source_policy: SourcePolicy::LocalCache,
        cache_state: CacheState::CacheCorrupted,
        cache_health: CacheHealth::Corrupted,
        manifest_integrity: ManifestIntegrity::Failed,
        download_enabled: false,
        download_status: enforcer_domain::memory_types::DownloadStatus::DownloadDisabled,
        cache_byte_size: 0,
        // ALLOC-JUSTIFICATION: the public wire status owns strings after typed cache inputs leave scope.
        checked_at: checked_at.as_str().retained(),
        unavailable_reason: None,
        storage_error: None,
        corruption_reason: Some(reason),
    }
}

fn validate_manifest_shape(manifest: &ModelCacheManifestDto) -> Result<()> {
    if manifest.schema_version != MODEL_CACHE_SCHEMA_VERSION {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest".into(),
            reason: format!(
                "unsupported model cache schema version {}",
                manifest.schema_version
            )
            .into(),
        });
    }
    if manifest.model_id.trim().is_empty() {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest".into(),
            reason: "modelId must not be empty".retained().into(),
        });
    }
    if manifest.artifacts.is_empty() {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-model-cache-manifest".into(),
            reason: "manifest must list at least one artifact".retained().into(),
        });
    }
    for artifact in &manifest.artifacts {
        crate::model_runtime::validate_sha256_hex(&artifact.sha256)?;
        if artifact.path.trim().is_empty() || PathBuf::from(artifact.path.as_str()).is_absolute() {
            return Err(MemoryError::ModelRuntime {
                operation: "validate-model-cache-manifest".into(),
                reason: format!("artifact path must be relative: {:?}", artifact.path).into(),
            });
        }
    }
    Ok(())
}

fn metadata_len(path: ModelCacheArtifactFile<'_>) -> ModelCacheArtifactSizeBytes {
    path.as_path()
        .metadata()
        .map_or_else(|_| 0.into(), |metadata| metadata.len().into())
}
