//! X06 model runtime/cache contract tests.
//!
//! These are contract fixtures, not real model parity claims. The
//! default path must stay zero-network and degraded until an explicit
//! `ort-models` backend can prove cache, tokenizer, provider, and output
//! integrity.

use enforcer_domain::memory_types::ModelRuntimeObservationKind;
use enforcer_domain::memory_types::{
    CacheCorruptionReasonCode, CacheHealth, CacheState, CacheStorageErrorCode,
    CacheUnavailableReason, DownloadStatus, LoadStateReport, ManifestIntegrity, ModelCacheRootMode,
    ModelTask, ProviderKind, SourcePolicy,
};
use enforcer_memory::error::MemoryError;
use enforcer_memory::model_runtime::evaluate_chat_usability;
use enforcer_memory::model_runtime::{
    default_model_runtime_probe_plan, default_provider_order, default_zero_network_proof,
    degraded_capability_report, discover_onnx_artifacts, ort_feature_compiled,
    resolve_model_cache_root, validate_embedding_output, validate_model_artifacts,
    validate_reranker_scores, validate_sha256_hex, ChatThroughputPolicyDto,
    DiscoveredModelArtifactDto, ModelCacheRootPolicyDto, ModelCacheStatusDto,
    ModelCapabilityReportDto, ModelRuntimeConfigDto, ModelRuntimeFileDto, ModelRuntimeManifestDto,
    ModelRuntimeProbePlanDto, ModelRuntimeProofDto, ModelRuntimeServiceConfigDto, ModelSpecDto,
    ModelUsabilityReportDto,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_json_round_trip<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let encoded = serde_json::to_vec(value)?;
    let decoded = serde_json::from_slice::<T>(&encoded)?;
    assert_eq!(&decoded, value);
    Ok(())
}

#[test]
fn model_runtime_dto_shapes_round_trip_through_json() -> TestResult {
    let hash = "00".repeat(32);
    let embedding = ModelSpecDto::qwen3_embedding(
        "embedding.gguf",
        hash.clone(),
        "tokenizer.json",
        hash.clone(),
    );
    let reranker =
        ModelSpecDto::qwen3_reranker("reranker.onnx", hash.clone(), "tokenizer.json", hash);
    assert_json_round_trip(&embedding)?;
    assert_json_round_trip(&ModelRuntimeConfigDto {
        cache_root: "model-cache".into(),
        allow_network: false,
        preferred_providers: vec![ProviderKind::Cpu],
        embedding: embedding.clone(),
        reranker,
    })?;
    let cache_root_policy: ModelCacheRootPolicyDto = resolve_model_cache_root(
        ".",
        ModelCacheRootMode::DevRepoLocal,
        Some("model-cache".into()),
    );
    assert_json_round_trip(&cache_root_policy)?;
    assert_json_round_trip(&ModelRuntimeServiceConfigDto::dev("."))?;

    let policy = ChatThroughputPolicyDto::default();
    assert_json_round_trip(&policy)?;
    let usability: ModelUsabilityReportDto =
        evaluate_chat_usability(true, Some(20.0), "unused failure", policy);
    assert_json_round_trip(&usability)?;
    let probe_plan: ModelRuntimeProbePlanDto = default_model_runtime_probe_plan();
    assert_json_round_trip(&probe_plan)?;

    let capability: ModelCapabilityReportDto = degraded_capability_report(
        ModelTask::Embedding,
        SourcePolicy::LocalCache,
        "provider unavailable",
    );
    assert_json_round_trip(&capability)?;
    assert_json_round_trip(&ModelRuntimeManifestDto::from_spec(
        &embedding,
        "llama.cpp",
        Some(ProviderKind::Cpu),
    ))?;
    assert_json_round_trip(&ModelRuntimeFileDto::new("model.gguf", Some(128)))?;
    assert_json_round_trip(&DiscoveredModelArtifactDto {
        onnx_path: "model.onnx".to_owned(),
        dtype: "f32".to_owned(),
        files: vec!["model.onnx".to_owned()],
        has_external_data: false,
    })?;
    let proof: ModelRuntimeProofDto = default_zero_network_proof();
    assert_json_round_trip(&proof)?;
    assert_json_round_trip(&ModelCacheStatusDto::unavailable(
        "artifact",
        "2026-07-17T00:00:00Z",
        CacheUnavailableReason::ArtifactNotInstalled,
    ))?;
    Ok(())
}

fn write_temp(contents: &[u8]) -> Result<(NamedTempFile, String), Box<dyn std::error::Error>> {
    let file = NamedTempFile::new().map_err(|source| MemoryError::Io {
        path: std::path::PathBuf::from("<tempfile>").into(),
        source,
    })?;
    std::fs::write(file.path(), contents).map_err(|source| MemoryError::Io {
        path: file.path().to_path_buf().into(),
        source,
    })?;
    let digest = enforcer_memory::model_runtime::sha256_file(file.path())?;
    Ok((file, digest))
}

#[test]
fn default_model_runtime_probe_plan_pins_safe_chat_probe_policy() {
    let plan = default_model_runtime_probe_plan();

    assert_eq!(plan.default_probe_filter, "chat");
    assert!(plan.one_model_at_a_time);
    assert!(plan.cpu_first);
    assert!(plan.gpu_and_npu_require_provider_probe);
    assert!(plan.kill_on_timeout);
    assert_eq!(plan.minimum_chat_tokens_per_second, 10);
    assert_eq!(plan.target_chat_tokens_per_second_low, 40);
    assert_eq!(plan.target_chat_tokens_per_second_high, 60);
    assert!(
        plan.provider_probe_timeout_ms < plan.model_probe_timeout_ms,
        "provider probes must fail fast before any model load attempt"
    );
}

#[test]
fn zero_network_default_proof_records_learning_observation_kinds() {
    let proof = default_zero_network_proof();

    assert!(proof.zero_network_default);
    assert_eq!(
        proof.embedding.load_state,
        LoadStateReport::DegradedProviderUnavailable
    );
    assert_eq!(
        proof.reranker.load_state,
        LoadStateReport::DegradedProviderUnavailable
    );
    assert!(proof.probe_plan.one_model_at_a_time);
    assert!(proof.probe_plan.cpu_first);
    assert!(proof.probe_plan.gpu_and_npu_require_provider_probe);
    assert_eq!(proof.probe_plan.default_probe_filter, "chat");
    assert_eq!(proof.probe_plan.minimum_chat_tokens_per_second, 10);
    assert_eq!(proof.probe_plan.target_chat_tokens_per_second_low, 40);
    assert_eq!(proof.probe_plan.target_chat_tokens_per_second_high, 60);
    assert!(proof.probe_plan.kill_on_timeout);
    assert_eq!(proof.embedding.source_policy, SourcePolicy::LocalCache);
    assert_eq!(proof.embedding.cache_state, CacheState::Unavailable);
    assert_eq!(proof.embedding.cache_health, CacheHealth::Unavailable);
    assert_eq!(
        proof.embedding.manifest_integrity,
        ManifestIntegrity::Unavailable
    );
    assert_eq!(
        proof.embedding.unavailable_reason,
        Some(CacheUnavailableReason::ArtifactNotInstalled)
    );
    assert!(proof
        .embedding
        .reason
        .as_deref()
        .unwrap_or_default()
        .contains("provider probes remain unavailable"));
    assert_eq!(proof.reranker.source_policy, SourcePolicy::LocalCache);
    assert_eq!(proof.reranker.cache_state, CacheState::Unavailable);
    assert_eq!(
        proof.reranker.unavailable_reason,
        Some(CacheUnavailableReason::ArtifactNotInstalled)
    );
    assert!(proof
        .learning_observation_kinds
        .contains(&ModelRuntimeObservationKind::ArtifactHashMismatch));
    assert!(proof
        .learning_observation_kinds
        .contains(&ModelRuntimeObservationKind::TokenizerHashMismatch));
    assert!(proof
        .learning_observation_kinds
        .contains(&ModelRuntimeObservationKind::SuccessfulLocalLoad));
}

#[test]
fn degraded_capability_report_tracks_source_policy_cache_semantics() {
    let unavailable = degraded_capability_report(
        ModelTask::Embedding,
        SourcePolicy::Unavailable,
        "model source missing",
    );
    assert_eq!(unavailable.cache_state, CacheState::Unavailable);
    assert_eq!(unavailable.cache_health, CacheHealth::Unavailable);
    assert_eq!(
        unavailable.manifest_integrity,
        ManifestIntegrity::Unavailable
    );
    assert_eq!(
        unavailable.unavailable_reason,
        Some(CacheUnavailableReason::ModelSourceUnconfigured)
    );

    let local_cache = degraded_capability_report(
        ModelTask::Embedding,
        SourcePolicy::LocalCache,
        "provider unavailable",
    );
    assert_eq!(local_cache.cache_state, CacheState::Unavailable);
    assert_eq!(local_cache.cache_health, CacheHealth::Unavailable);
    assert_eq!(
        local_cache.manifest_integrity,
        ManifestIntegrity::Unavailable
    );
    assert_eq!(
        local_cache.unavailable_reason,
        Some(CacheUnavailableReason::ArtifactNotInstalled)
    );

    let bundled = degraded_capability_report(
        ModelTask::Embedding,
        SourcePolicy::Bundled,
        "provider unavailable",
    );
    assert_eq!(bundled.cache_state, CacheState::CacheDegraded);
    assert_eq!(bundled.cache_health, CacheHealth::Degraded);
    assert_eq!(bundled.manifest_integrity, ManifestIntegrity::Unchecked);
    assert_eq!(
        bundled.unavailable_reason,
        Some(CacheUnavailableReason::IntegrityUnverified)
    );

    let parent_installed = degraded_capability_report(
        ModelTask::Embedding,
        SourcePolicy::ParentInstalled,
        "provider unavailable",
    );
    assert_eq!(parent_installed.cache_state, CacheState::CacheDegraded);
    assert_eq!(parent_installed.cache_health, CacheHealth::Degraded);
    assert_eq!(
        parent_installed.manifest_integrity,
        ManifestIntegrity::Unchecked
    );
    assert_eq!(
        parent_installed.unavailable_reason,
        Some(CacheUnavailableReason::IntegrityUnverified)
    );
}

#[test]
fn provider_order_preserves_preferences_without_duplicates() {
    let order = default_provider_order(&[ProviderKind::DirectMl, ProviderKind::Cpu]);

    assert_eq!(
        order,
        vec![
            ProviderKind::DirectMl,
            ProviderKind::Cuda,
            ProviderKind::Vulkan,
            ProviderKind::OpenVino,
            ProviderKind::CoreMl,
            ProviderKind::Npu,
            ProviderKind::Cpu
        ]
    );
}

#[test]
fn parent_policy_cache_and_integrity_states_are_represented() -> TestResult {
    let parent_policy_states = [
        SourcePolicy::Bundled,
        SourcePolicy::ParentInstalled,
        SourcePolicy::LocalCache,
        SourcePolicy::Unavailable,
    ];
    let parent_cache_states = [
        CacheState::Unavailable,
        CacheState::NotCached,
        CacheState::CacheReady,
        CacheState::CacheDegraded,
        CacheState::CacheCorrupted,
        CacheState::StorageError,
    ];
    let parent_cache_health_states = [
        CacheHealth::Healthy,
        CacheHealth::Degraded,
        CacheHealth::Unavailable,
        CacheHealth::DownloadDisabled,
        CacheHealth::Corrupted,
        CacheHealth::StorageError,
    ];
    let parent_integrity_states = [
        ManifestIntegrity::Unavailable,
        ManifestIntegrity::Unchecked,
        ManifestIntegrity::Verified,
        ManifestIntegrity::ManifestMissing,
        ManifestIntegrity::ChecksumMismatch,
        ManifestIntegrity::SignatureInvalid,
        ManifestIntegrity::Corrupted,
    ];
    let parent_unavailable_reasons = [
        CacheUnavailableReason::ModelSourceUnconfigured,
        CacheUnavailableReason::ArtifactNotInstalled,
        CacheUnavailableReason::ManifestUnavailable,
        CacheUnavailableReason::DownloadDisabled,
        CacheUnavailableReason::CacheStorageUnavailable,
        CacheUnavailableReason::IntegrityUnverified,
        CacheUnavailableReason::CorruptionDetected,
    ];
    let parent_storage_errors = [
        CacheStorageErrorCode::CacheRootUnavailable,
        CacheStorageErrorCode::ManifestReadFailed,
        CacheStorageErrorCode::ArtifactReadFailed,
        CacheStorageErrorCode::MetadataWriteDisabled,
        CacheStorageErrorCode::StoragePermissionDenied,
        CacheStorageErrorCode::QuotaUnavailable,
    ];
    let parent_corruption_reasons = [
        CacheCorruptionReasonCode::ManifestMissing,
        CacheCorruptionReasonCode::ChecksumMismatch,
        CacheCorruptionReasonCode::SignatureInvalid,
        CacheCorruptionReasonCode::ArtifactMissing,
        CacheCorruptionReasonCode::ManifestArtifactMismatch,
        CacheCorruptionReasonCode::UnknownIntegrity,
    ];

    assert_eq!(parent_policy_states.len(), 4);
    assert_eq!(parent_cache_states.len(), 6);
    assert_eq!(parent_cache_health_states.len(), 6);
    assert_eq!(parent_integrity_states.len(), 7);
    assert_eq!(parent_unavailable_reasons.len(), 7);
    assert_eq!(parent_storage_errors.len(), 6);
    assert_eq!(parent_corruption_reasons.len(), 6);

    let artifacts = discover_onnx_artifacts(&[
        ModelRuntimeFileDto::new("model_fp16.onnx", Some(10)),
        ModelRuntimeFileDto::new("model_fp16.onnx.data", Some(20)),
        ModelRuntimeFileDto::new("tokenizer_config.json", Some(4)),
        ModelRuntimeFileDto::new("tokenizer.json", Some(5)),
    ]);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].dtype, "fp16");
    assert!(artifacts[0].has_external_data);
    let tokenizer_config_index = artifacts[0]
        .files
        .iter()
        .position(|path| path.ends_with("tokenizer_config.json"))
        .ok_or("tokenizer_config.json should be included")?;
    let tokenizer_json_index = artifacts[0]
        .files
        .iter()
        .position(|path| path.ends_with("tokenizer.json"))
        .ok_or("tokenizer.json should be included")?;
    assert!(tokenizer_config_index < tokenizer_json_index);
    Ok(())
}

#[test]
fn parent_style_cache_status_serializes_with_kebab_case_states() -> TestResult {
    let status = ModelCacheStatusDto::parent_installed_degraded(
        "model-ref-local",
        Some("manifest-ref-local".to_string()),
        "2026-05-21T09:18:00Z",
    );

    let serialized = serde_json::to_value(status)?;

    assert_eq!(serialized["artifactRef"], "model-ref-local");
    assert_eq!(serialized["manifestRef"], "manifest-ref-local");
    assert_eq!(serialized["sourcePolicy"], "parent-installed");
    assert_eq!(serialized["cacheState"], "cache-degraded");
    assert_eq!(serialized["cacheHealth"], "degraded");
    assert_eq!(serialized["manifestIntegrity"], "unchecked");
    assert_eq!(serialized["downloadStatus"], "download-disabled");
    assert_eq!(serialized["unavailableReason"], "integrity-unverified");
    Ok(())
}

#[test]
fn enum_serialization_matches_parent_style_kebab_case() -> TestResult {
    let cases = [
        (
            serde_json::to_value(SourcePolicy::LocalCache)?,
            "local-cache",
        ),
        (
            serde_json::to_value(CacheHealth::Unavailable)?,
            "unavailable",
        ),
        (
            serde_json::to_value(CacheUnavailableReason::ArtifactNotInstalled)?,
            "artifact-not-installed",
        ),
        (
            serde_json::to_value(CacheStorageErrorCode::CacheRootUnavailable)?,
            "cache-root-unavailable",
        ),
        (
            serde_json::to_value(CacheCorruptionReasonCode::ManifestArtifactMismatch)?,
            "manifest-artifact-mismatch",
        ),
        (
            serde_json::to_value(DownloadStatus::DownloadInProgress)?,
            "download-in-progress",
        ),
        (
            serde_json::to_value(LoadStateReport::DegradedProviderUnavailable)?,
            "degraded-provider-unavailable",
        ),
        (serde_json::to_value(ProviderKind::Vulkan)?, "vulkan"),
    ];

    for (actual, expected) in cases {
        assert_eq!(actual, expected);
    }
    assert_eq!(
        ProviderKind::Vulkan.resource_class(),
        enforcer_domain::memory_types::ResourceClass::Gpu
    );
    Ok(())
}

#[test]
fn invalid_hash_shape_is_a_typed_model_runtime_error() {
    let result = validate_sha256_hex("not-a-hash");

    assert!(matches!(result, Err(MemoryError::ModelRuntime { .. })));
}

#[test]
fn output_validation_rejects_invalid_runtime_shapes() {
    assert!(matches!(validate_embedding_output(&[0.1, 0.2], 2), Ok(())));
    assert!(matches!(
        validate_embedding_output(&[0.1], 2),
        Err(MemoryError::ModelRuntime { .. })
    ));
    assert!(matches!(
        validate_embedding_output(&[f32::NAN], 1),
        Err(MemoryError::ModelRuntime { .. })
    ));

    assert!(matches!(validate_reranker_scores(&[0.9, 0.1], 2), Ok(())));
    assert!(matches!(
        validate_reranker_scores(&[0.9], 2),
        Err(MemoryError::ModelRuntime { .. })
    ));
    assert!(matches!(
        validate_reranker_scores(&[f32::INFINITY], 1),
        Err(MemoryError::ModelRuntime { .. })
    ));
}

#[test]
fn provider_order_keeps_preference_then_local_fallbacks() {
    let order = default_provider_order(&[ProviderKind::DirectMl]);
    assert_eq!(
        order,
        vec![
            ProviderKind::DirectMl,
            ProviderKind::Cuda,
            ProviderKind::Vulkan,
            ProviderKind::OpenVino,
            ProviderKind::CoreMl,
            ProviderKind::Npu,
            ProviderKind::Cpu,
        ]
    );
}

#[test]
fn discovers_onnx_artifact_with_external_data_and_support_files() {
    let files = vec![
        ModelRuntimeFileDto::new("onnx/model_q4f16.onnx", Some(10)),
        ModelRuntimeFileDto::new("onnx/model_q4f16.onnx_data", Some(20)),
        ModelRuntimeFileDto::new("onnx/tokenizer.json", Some(5)),
        ModelRuntimeFileDto::new("config.json", Some(1)),
        ModelRuntimeFileDto::new("other/readme.md", None),
    ];

    let artifacts = discover_onnx_artifacts(&files);

    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(artifact.onnx_path, "onnx/model_q4f16.onnx");
    assert_eq!(artifact.dtype, "q4f16");
    assert!(artifact.has_external_data);
    assert_eq!(
        artifact.files,
        vec![
            "onnx/model_q4f16.onnx".to_owned(),
            "onnx/model_q4f16.onnx_data".to_owned(),
            "config.json".to_owned(),
            "onnx/tokenizer.json".to_owned(),
        ]
    );
}

#[test]
fn artifact_discovery_handles_windows_separators() {
    let files = vec![
        ModelRuntimeFileDto::new(r"onnx\model_fp16.onnx", Some(10)),
        ModelRuntimeFileDto::new(r"onnx\model_fp16.onnx.data", Some(20)),
        ModelRuntimeFileDto::new(r"onnx\tokenizer.json", Some(5)),
        ModelRuntimeFileDto::new(r"other\tokenizer.json", Some(5)),
    ];

    let artifacts = discover_onnx_artifacts(&files);

    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert!(artifact.has_external_data);
    assert_eq!(
        artifact.files,
        vec![
            r"onnx\model_fp16.onnx".to_owned(),
            r"onnx\model_fp16.onnx.data".to_owned(),
            r"onnx\tokenizer.json".to_owned(),
        ]
    );
}

#[test]
fn default_proof_is_honest_zero_network_degraded() {
    let proof = default_zero_network_proof();
    assert!(proof.zero_network_default);
    assert_eq!(
        proof.embedding.load_state,
        LoadStateReport::DegradedProviderUnavailable
    );
    assert!(proof
        .learning_observation_kinds
        .contains(&ModelRuntimeObservationKind::DegradedFallback));
}

#[test]
fn artifact_hash_validation_accepts_exact_hash() -> TestResult {
    let (artifact, artifact_hash) = write_temp(b"model")?;
    let (tokenizer, tokenizer_hash) = write_temp(b"tokenizer")?;
    let spec = ModelSpecDto::qwen3_embedding(
        artifact.path(),
        artifact_hash,
        tokenizer.path(),
        tokenizer_hash,
    );
    validate_model_artifacts(&spec)?;
    Ok(())
}

#[test]
fn artifact_hash_validation_accepts_uppercase_manifest_hash() -> TestResult {
    let (artifact, artifact_hash) = write_temp(b"model")?;
    let (tokenizer, tokenizer_hash) = write_temp(b"tokenizer")?;
    let spec = ModelSpecDto::qwen3_embedding(
        artifact.path(),
        artifact_hash.to_uppercase(),
        tokenizer.path(),
        tokenizer_hash.to_uppercase(),
    );
    validate_model_artifacts(&spec)?;
    Ok(())
}

#[test]
fn artifact_hash_validation_rejects_mismatch() -> TestResult {
    let (artifact, _artifact_hash) = write_temp(b"model")?;
    let (tokenizer, tokenizer_hash) = write_temp(b"tokenizer")?;
    let spec = ModelSpecDto::qwen3_embedding(
        artifact.path(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        tokenizer.path(),
        tokenizer_hash,
    );
    let result = validate_model_artifacts(&spec);
    assert!(matches!(result, Err(MemoryError::ModelRuntime { .. })));
    Ok(())
}

#[cfg(feature = "ort-models")]
#[test]
fn ort_models_feature_compiles_fixture_validation_path() -> TestResult {
    assert!(ort_feature_compiled());
    let (artifact, artifact_hash) = write_temp(b"onnx-bytes")?;
    let (tokenizer, tokenizer_hash) = write_temp(b"tokenizer-json")?;
    let spec = ModelSpecDto::qwen3_reranker(
        artifact.path(),
        artifact_hash,
        tokenizer.path(),
        tokenizer_hash,
    );
    validate_model_artifacts(&spec)?;
    let manifest = enforcer_memory::model_runtime::ModelRuntimeManifestDto::from_spec(
        &spec,
        "ort",
        Some(ProviderKind::Cpu),
    );
    assert_eq!(manifest.backend, "ort");
    assert_eq!(manifest.provider, Some(ProviderKind::Cpu));
    Ok(())
}

#[cfg(not(feature = "ort-models"))]
#[test]
fn default_build_does_not_compile_real_ort_runtime() {
    assert!(!ort_feature_compiled());
}

#[cfg(feature = "ort-models")]
#[test]
fn ort_models_feature_is_only_a_contract_gate_in_this_slice() {
    assert!(ort_feature_compiled());
}
