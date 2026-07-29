use std::fs;

use enforcer_domain::memory_types::{
    CacheHealth, CacheState, ManifestIntegrity, ModelTask, SourcePolicy,
};
use enforcer_domain::memory_types::{
    LocalRuntimeAcceleration, LocalRuntimeArtifactKind, LocalRuntimeKind,
};
use enforcer_memory::boundary::model_cache::{
    ModelCacheArtifactEntryDto, ModelCacheManifestDto, ModelCacheValidationDto,
};
use enforcer_memory::error::MemoryError;
use enforcer_memory::model_cache::{
    load_model_cache_manifest, validate_model_cache_manifest, MODEL_CACHE_SCHEMA_VERSION,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn local_manifest_loads_and_validates_gguf_cache() -> TestResult {
    let temp = TempDir::new()?;
    let model_path = temp.path().join("qwen.gguf");
    fs::write(&model_path, b"gguf-bytes")?;
    let model_hash = enforcer_memory::model_runtime::sha256_file(&model_path)?;
    let manifest_path = temp.path().join("manifest.json");
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: LocalRuntimeKind::LlamaCpp,
        task: ModelTask::Embedding,
        model_id: "qwen3-embedding-gguf".to_string().into(),
        revision: "local".to_string().into(),
        acceleration: LocalRuntimeAcceleration::Cpu,
        artifacts: vec![ModelCacheArtifactEntryDto {
            kind: Some(LocalRuntimeArtifactKind::Model),
            path: "qwen.gguf".to_string().into(),
            sha256: model_hash.into(),
            size_bytes: None,
            streaming_manifest_path: None,
        }],
    };
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let loaded = load_model_cache_manifest(&manifest_path)?;
    let validation = validate_model_cache_manifest(&manifest_path)?;

    assert_eq!(loaded.backend, LocalRuntimeKind::LlamaCpp);
    assert_eq!(validation.status.source_policy, SourcePolicy::LocalCache);
    assert_eq!(validation.status.cache_state, CacheState::CacheReady);
    assert_eq!(validation.status.cache_health, CacheHealth::Healthy);
    assert_eq!(
        validation.status.manifest_integrity,
        ManifestIntegrity::Verified
    );
    let candidate = validation
        .candidate
        .as_ref()
        .ok_or("validated cache must expose its runtime candidate")?;
    assert_eq!(candidate.backend, LocalRuntimeKind::LlamaCpp);
    Ok(())
}

#[test]
fn model_cache_dtos_round_trip_after_validation() -> TestResult {
    let temp = TempDir::new()?;
    let model_path = temp.path().join("qwen.gguf");
    fs::write(&model_path, b"gguf-bytes")?;
    let artifact = ModelCacheArtifactEntryDto {
        kind: Some(LocalRuntimeArtifactKind::Model),
        path: "qwen.gguf".to_string().into(),
        sha256: enforcer_memory::model_runtime::sha256_file(&model_path)?.into(),
        size_bytes: None,
        streaming_manifest_path: None,
    };
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: LocalRuntimeKind::LlamaCpp,
        task: ModelTask::Embedding,
        model_id: "qwen3-embedding-gguf".to_string().into(),
        revision: "local".to_string().into(),
        acceleration: LocalRuntimeAcceleration::Cpu,
        artifacts: vec![artifact.clone()],
    };
    let manifest_path = temp.path().join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let validation: ModelCacheValidationDto = validate_model_cache_manifest(&manifest_path)?;

    let artifact_back: ModelCacheArtifactEntryDto =
        serde_json::from_slice(&serde_json::to_vec(&artifact)?)?;
    let manifest_back: ModelCacheManifestDto =
        serde_json::from_slice(&serde_json::to_vec(&manifest)?)?;
    let validation_back: ModelCacheValidationDto =
        serde_json::from_slice(&serde_json::to_vec(&validation)?)?;
    assert_eq!(artifact_back, artifact);
    assert_eq!(manifest_back, manifest);
    assert_eq!(validation_back, validation);
    Ok(())
}

#[test]
fn missing_manifest_returns_unavailable_status_not_success() -> TestResult {
    let temp = TempDir::new()?;
    let missing = temp.path().join("missing-manifest.json");

    let validation = validate_model_cache_manifest(&missing)?;

    assert!(validation.manifest.is_none());
    assert_eq!(validation.status.source_policy, SourcePolicy::Unavailable);
    assert_eq!(validation.status.cache_state, CacheState::Unavailable);
    assert_eq!(validation.status.cache_health, CacheHealth::Unavailable);
    Ok(())
}

#[test]
fn hash_mismatch_is_typed_runtime_error() -> TestResult {
    let temp = TempDir::new()?;
    let model_path = temp.path().join("qwen.gguf");
    fs::write(&model_path, b"actual")?;
    let manifest_path = temp.path().join("manifest.json");
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: LocalRuntimeKind::LlamaCpp,
        task: ModelTask::Embedding,
        model_id: "qwen3-embedding-gguf".to_string().into(),
        revision: "local".to_string().into(),
        acceleration: LocalRuntimeAcceleration::Cpu,
        artifacts: vec![ModelCacheArtifactEntryDto {
            kind: Some(LocalRuntimeArtifactKind::Model),
            path: "qwen.gguf".to_string().into(),
            sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string()
                .into(),
            size_bytes: None,
            streaming_manifest_path: None,
        }],
    };
    fs::write(&manifest_path, serde_json::to_string(&manifest)?)?;

    let result = validate_model_cache_manifest(&manifest_path);

    assert!(matches!(result, Err(MemoryError::ModelRuntime { .. })));
    Ok(())
}

#[test]
fn absolute_artifact_paths_are_rejected() -> TestResult {
    let temp = TempDir::new()?;
    let manifest_path = temp.path().join("manifest.json");
    let manifest = ModelCacheManifestDto {
        schema_version: MODEL_CACHE_SCHEMA_VERSION.into(),
        backend: LocalRuntimeKind::LlamaCpp,
        task: ModelTask::Embedding,
        model_id: "qwen3-embedding-gguf".to_string().into(),
        revision: "local".to_string().into(),
        acceleration: LocalRuntimeAcceleration::Cpu,
        artifacts: vec![ModelCacheArtifactEntryDto {
            kind: Some(LocalRuntimeArtifactKind::Model),
            path: r"C:\models\qwen.gguf".to_string().into(),
            sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string()
                .into(),
            size_bytes: None,
            streaming_manifest_path: None,
        }],
    };
    fs::write(&manifest_path, serde_json::to_string(&manifest)?)?;

    let result = load_model_cache_manifest(&manifest_path);

    assert!(matches!(result, Err(MemoryError::ModelRuntime { .. })));
    Ok(())
}
