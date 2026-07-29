//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Serialized model-cache manifest DTOs.
//!
//! BOUNDARY-INVARIANT: these DTOs model the persisted cache wire contract;
//! validation converts their fields to canonical runtime decisions before use.

use serde::{Deserialize, Serialize};

use crate::local_runtime::LocalRuntimeCandidateDto;
use crate::model_runtime::ModelCacheStatusDto;
use enforcer_domain::memory_types::{
    LocalRuntimeAcceleration, LocalRuntimeArtifactKind, LocalRuntimeKind, ModelCacheArtifactPath,
    ModelCacheArtifactSha256, ModelCacheArtifactSizeBytes, ModelCacheModelId, ModelCacheRevision,
    ModelCacheSchemaVersion, ModelCacheStreamingManifestPath, ModelTask,
};

// ROUNDTRIP-TEST: model_cache::manifest_roundtrip_preserves_cache_contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheManifestDto {
    pub schema_version: ModelCacheSchemaVersion,
    pub backend: LocalRuntimeKind,
    pub task: ModelTask,
    pub model_id: ModelCacheModelId,
    pub revision: ModelCacheRevision,
    pub acceleration: LocalRuntimeAcceleration,
    pub artifacts: Vec<ModelCacheArtifactEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheArtifactEntryDto {
    pub kind: Option<LocalRuntimeArtifactKind>,
    pub path: ModelCacheArtifactPath,
    pub sha256: ModelCacheArtifactSha256,
    pub size_bytes: Option<ModelCacheArtifactSizeBytes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_manifest_path: Option<ModelCacheStreamingManifestPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheValidationDto {
    pub status: ModelCacheStatusDto,
    pub manifest: Option<ModelCacheManifestDto>,
    pub candidate: Option<LocalRuntimeCandidateDto>,
}
