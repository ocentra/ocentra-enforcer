//! X06 model runtime/cache contracts.
//!
//! This module is the Enforcer-native landing zone for the TabAgentServer
//! model-cache / execution-provider ideas and the OcentraParent lifecycle
//! contract shapes. It intentionally does not load real models in the
//! default build. Instead it defines the manifest, integrity, provider,
//! capability, and proof shapes that any real local backend must satisfy
//! before x06 can claim local-model parity.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::embed::{DegradedState, LoadState, ResourceClass};
use crate::error::{MemoryError, Result};
use crate::local_runtime::{RuntimeManagedCapability, RuntimeOwnershipMode};

pub const DEFAULT_EMBEDDING_MODEL_ID: &str = "Qwen/Qwen3-Embedding-0.6B";
pub const DEFAULT_RERANKER_MODEL_ID: &str = "Qwen/Qwen3-Reranker-0.6B";
pub const DEFAULT_ORNITH_GGUF_REPO: &str = "deepreinforce-ai/Ornith-1.0-9B-GGUF";
pub const DEFAULT_ORNITH_GGUF_FILE: &str = "ornith-1.0-9b-Q4_K_M.gguf";
pub const DEFAULT_EMBEDDING_GGUF_REPO: &str = "Qwen/Qwen3-Embedding-0.6B-GGUF";
pub const DEFAULT_EMBEDDING_GGUF_FILE: &str = "Qwen3-Embedding-0.6B-Q8_0.gguf";
pub const DEFAULT_EMBEDDING_ONNX_REPO: &str = "onnx-community/Qwen3-Embedding-0.6B-ONNX";
pub const DEFAULT_EMBEDDING_ONNX_FILE: &str = "onnx/model_q4.onnx";
pub const DEFAULT_RERANKER_ONNX_REPO: &str = "onnx-community/Qwen3-Reranker-0.6B-ONNX";
pub const DEFAULT_RERANKER_ONNX_FILE: &str = "onnx/model_q4.onnx";
pub const DEFAULT_MODEL_REVISION: &str = "main";
pub const MODEL_RUNTIME_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MODEL_CACHE_DIR_NAME: &str = "model";
pub const DEFAULT_MODEL_SERVICE_HOST: &str = "127.0.0.1";
pub const DEFAULT_MODEL_SERVICE_PORT: u16 = 8766;
pub const DEFAULT_MIN_CHAT_TOKENS_PER_SECOND: f64 = 10.0;
pub const TARGET_CHAT_TOKENS_PER_SECOND_LOW: f64 = 40.0;
pub const TARGET_CHAT_TOKENS_PER_SECOND_HIGH: f64 = 60.0;
pub const DEFAULT_DEVICE_PROBE_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_MODEL_PROBE_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelTask {
    Embedding,
    Reranking,
    Summarization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourcePolicy {
    Bundled,
    ParentInstalled,
    LocalCache,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheState {
    Unavailable,
    NotCached,
    CacheReady,
    CacheDegraded,
    CacheCorrupted,
    StorageError,
    ArtifactPresent,
    ArtifactMissing,
    HashMismatch,
    TokenizerMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManifestIntegrity {
    Unavailable,
    Verified,
    Unchecked,
    ManifestMissing,
    ChecksumMismatch,
    SignatureInvalid,
    Corrupted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheHealth {
    Healthy,
    Degraded,
    Unavailable,
    DownloadDisabled,
    Corrupted,
    StorageError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadStatus {
    DownloadDisabled,
    DownloadNotRequested,
    DownloadInProgress,
    DownloadComplete,
    DownloadFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheUnavailableReason {
    ModelSourceUnconfigured,
    ArtifactNotInstalled,
    ManifestUnavailable,
    DownloadDisabled,
    CacheStorageUnavailable,
    IntegrityUnverified,
    CorruptionDetected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheStorageErrorCode {
    CacheRootUnavailable,
    ManifestReadFailed,
    ArtifactReadFailed,
    MetadataWriteDisabled,
    StoragePermissionDenied,
    QuotaUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheCorruptionReasonCode {
    ManifestMissing,
    ChecksumMismatch,
    SignatureInvalid,
    ArtifactMissing,
    ManifestArtifactMismatch,
    UnknownIntegrity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Cpu,
    DirectMl,
    OpenVino,
    Vulkan,
    Cuda,
    CoreMl,
    Npu,
}

impl ProviderKind {
    pub fn resource_class(self) -> ResourceClass {
        match self {
            ProviderKind::Cpu => ResourceClass::Cpu,
            ProviderKind::DirectMl
            | ProviderKind::OpenVino
            | ProviderKind::Vulkan
            | ProviderKind::Cuda
            | ProviderKind::CoreMl => ResourceClass::Gpu,
            ProviderKind::Npu => ResourceClass::Npu,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub model_id: String,
    pub revision: String,
    pub artifact_path: PathBuf,
    pub artifact_sha256: String,
    pub tokenizer_path: PathBuf,
    pub tokenizer_sha256: String,
    pub dtype: String,
    pub dimension: usize,
    pub task: ModelTask,
}

impl ModelSpec {
    pub fn qwen3_embedding(
        artifact_path: impl Into<PathBuf>,
        artifact_sha256: impl Into<String>,
        tokenizer_path: impl Into<PathBuf>,
        tokenizer_sha256: impl Into<String>,
    ) -> Self {
        Self {
            model_id: DEFAULT_EMBEDDING_MODEL_ID.to_owned(),
            revision: DEFAULT_MODEL_REVISION.to_owned(),
            artifact_path: artifact_path.into(),
            artifact_sha256: artifact_sha256.into(),
            tokenizer_path: tokenizer_path.into(),
            tokenizer_sha256: tokenizer_sha256.into(),
            dtype: "f32".to_owned(),
            dimension: 1024,
            task: ModelTask::Embedding,
        }
    }

    pub fn qwen3_reranker(
        artifact_path: impl Into<PathBuf>,
        artifact_sha256: impl Into<String>,
        tokenizer_path: impl Into<PathBuf>,
        tokenizer_sha256: impl Into<String>,
    ) -> Self {
        Self {
            model_id: DEFAULT_RERANKER_MODEL_ID.to_owned(),
            revision: DEFAULT_MODEL_REVISION.to_owned(),
            artifact_path: artifact_path.into(),
            artifact_sha256: artifact_sha256.into(),
            tokenizer_path: tokenizer_path.into(),
            tokenizer_sha256: tokenizer_sha256.into(),
            dtype: "f32".to_owned(),
            dimension: 1,
            task: ModelTask::Reranking,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeConfig {
    pub cache_root: PathBuf,
    pub allow_network: bool,
    pub preferred_providers: Vec<ProviderKind>,
    pub embedding: ModelSpec,
    pub reranker: ModelSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelCacheRootMode {
    DevRepoLocal,
    AppData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheRootPolicy {
    pub mode: ModelCacheRootMode,
    pub root: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeServiceConfig {
    pub bind_host: String,
    pub port: u16,
    pub cache_root: PathBuf,
    pub expose_llama_server: bool,
    pub external_runtime_servers_allowed: bool,
    pub llama_cpp_execution_route: String,
    pub llama_cpp_ownership: RuntimeOwnershipMode,
    pub ort_execution_route: String,
    pub ort_ownership: RuntimeOwnershipMode,
    pub managed_capabilities: Vec<RuntimeManagedCapability>,
    pub routes: Vec<ModelRuntimeServiceRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatThroughputPolicy {
    pub min_tokens_per_second: f64,
    pub target_tokens_per_second_low: f64,
    pub target_tokens_per_second_high: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsabilityReport {
    pub ok: bool,
    pub reason: String,
    pub min_chat_tokens_per_second: Option<f64>,
    pub target_chat_tokens_per_second_low: Option<f64>,
    pub target_chat_tokens_per_second_high: Option<f64>,
    pub measured_tokens_per_second: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeProbePlan {
    pub default_probe_filter: String,
    pub one_model_at_a_time: bool,
    pub cpu_first: bool,
    pub gpu_and_npu_require_provider_probe: bool,
    pub provider_probe_timeout_ms: u64,
    pub model_probe_timeout_ms: u64,
    pub kill_on_timeout: bool,
    pub minimum_chat_tokens_per_second: u32,
    pub target_chat_tokens_per_second_low: u32,
    pub target_chat_tokens_per_second_high: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRuntimeServiceRoute {
    Health,
    Models,
    LoadModel,
    UnloadModel,
    Chat,
    Embeddings,
    Rerank,
}

impl ModelRuntimeConfig {
    pub fn source_policy(&self) -> SourcePolicy {
        let _ = self.allow_network;
        SourcePolicy::LocalCache
    }
}

impl ModelRuntimeServiceConfig {
    pub fn dev(repo_root: impl AsRef<Path>) -> Self {
        Self {
            bind_host: DEFAULT_MODEL_SERVICE_HOST.to_owned(),
            port: DEFAULT_MODEL_SERVICE_PORT,
            cache_root: dev_model_cache_root(repo_root),
            expose_llama_server: false,
            external_runtime_servers_allowed: false,
            llama_cpp_execution_route: "enforcer-managed-llama-cpp-subprocess".to_owned(),
            llama_cpp_ownership: RuntimeOwnershipMode::EnforcerSubprocess,
            ort_execution_route: "enforcer-isolated-ort-worker".to_owned(),
            ort_ownership: RuntimeOwnershipMode::EnforcerIsolatedWorker,
            managed_capabilities: default_model_service_managed_capabilities(),
            routes: default_model_service_routes(),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_host, self.port)
    }
}

impl Default for ChatThroughputPolicy {
    fn default() -> Self {
        Self {
            min_tokens_per_second: DEFAULT_MIN_CHAT_TOKENS_PER_SECOND,
            target_tokens_per_second_low: TARGET_CHAT_TOKENS_PER_SECOND_LOW,
            target_tokens_per_second_high: TARGET_CHAT_TOKENS_PER_SECOND_HIGH,
        }
    }
}

pub fn evaluate_chat_usability(
    is_loaded: bool,
    measured_tokens_per_second: Option<f64>,
    load_failure_reason: impl Into<String>,
    policy: ChatThroughputPolicy,
) -> ModelUsabilityReport {
    if !is_loaded {
        return ModelUsabilityReport {
            ok: false,
            reason: load_failure_reason.into(),
            min_chat_tokens_per_second: Some(policy.min_tokens_per_second),
            target_chat_tokens_per_second_low: Some(policy.target_tokens_per_second_low),
            target_chat_tokens_per_second_high: Some(policy.target_tokens_per_second_high),
            measured_tokens_per_second,
        };
    }

    let measured = measured_tokens_per_second.unwrap_or(0.0);
    let target = format!(
        "target {:.2}-{:.2} tokens/sec",
        policy.target_tokens_per_second_low, policy.target_tokens_per_second_high
    );
    if measured >= policy.min_tokens_per_second {
        ModelUsabilityReport {
            ok: true,
            reason: format!(
                "chat usable: measured {measured:.2} tokens/sec >= required {:.2}; {target}",
                policy.min_tokens_per_second
            ),
            min_chat_tokens_per_second: Some(policy.min_tokens_per_second),
            target_chat_tokens_per_second_low: Some(policy.target_tokens_per_second_low),
            target_chat_tokens_per_second_high: Some(policy.target_tokens_per_second_high),
            measured_tokens_per_second,
        }
    } else {
        ModelUsabilityReport {
            ok: false,
            reason: format!(
                "chat not usable: measured {measured:.2} tokens/sec < required {:.2}; {target}",
                policy.min_tokens_per_second
            ),
            min_chat_tokens_per_second: Some(policy.min_tokens_per_second),
            target_chat_tokens_per_second_low: Some(policy.target_tokens_per_second_low),
            target_chat_tokens_per_second_high: Some(policy.target_tokens_per_second_high),
            measured_tokens_per_second,
        }
    }
}

pub fn loaded_non_chat_usability(
    is_loaded: bool,
    measured_tokens_per_second: Option<f64>,
    load_failure_reason: impl Into<String>,
) -> ModelUsabilityReport {
    if is_loaded {
        ModelUsabilityReport {
            ok: true,
            reason: "loaded; no chat throughput floor applies".to_owned(),
            min_chat_tokens_per_second: None,
            target_chat_tokens_per_second_low: None,
            target_chat_tokens_per_second_high: None,
            measured_tokens_per_second,
        }
    } else {
        ModelUsabilityReport {
            ok: false,
            reason: load_failure_reason.into(),
            min_chat_tokens_per_second: None,
            target_chat_tokens_per_second_low: None,
            target_chat_tokens_per_second_high: None,
            measured_tokens_per_second,
        }
    }
}

pub fn default_model_service_routes() -> Vec<ModelRuntimeServiceRoute> {
    vec![
        ModelRuntimeServiceRoute::Health,
        ModelRuntimeServiceRoute::Models,
        ModelRuntimeServiceRoute::LoadModel,
        ModelRuntimeServiceRoute::UnloadModel,
        ModelRuntimeServiceRoute::Chat,
        ModelRuntimeServiceRoute::Embeddings,
        ModelRuntimeServiceRoute::Rerank,
    ]
}

pub fn default_model_service_managed_capabilities() -> Vec<RuntimeManagedCapability> {
    crate::local_runtime::REQUIRED_MANAGED_CAPABILITIES.to_vec()
}

pub fn dev_model_cache_root(repo_root: impl AsRef<Path>) -> PathBuf {
    repo_root.as_ref().join(DEFAULT_MODEL_CACHE_DIR_NAME)
}

pub fn app_data_model_cache_root(app_name: &str) -> PathBuf {
    app_data_base_dir(app_name).join(DEFAULT_MODEL_CACHE_DIR_NAME)
}

pub fn resolve_model_cache_root(
    repo_root: impl AsRef<Path>,
    mode: ModelCacheRootMode,
    explicit: Option<PathBuf>,
) -> ModelCacheRootPolicy {
    if let Some(root) = explicit {
        return ModelCacheRootPolicy {
            mode,
            root,
            reason: "explicit model cache root override".to_owned(),
        };
    }
    match mode {
        ModelCacheRootMode::DevRepoLocal => ModelCacheRootPolicy {
            mode,
            root: dev_model_cache_root(repo_root),
            reason: "dev mode keeps downloaded models in the repository-local model directory"
                .to_owned(),
        },
        ModelCacheRootMode::AppData => ModelCacheRootPolicy {
            mode,
            root: app_data_model_cache_root("OcentraEnforcer"),
            reason: "application mode keeps downloaded models in the platform app-data directory"
                .to_owned(),
        },
    }
}

fn app_data_base_dir(app_name: &str) -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join("AppData").join("Local"))
            .join(app_name)
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join(app_name)
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local").join("share"))
            .join(app_name)
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilityReport {
    pub task: ModelTask,
    pub load_state: LoadStateReport,
    pub resource_class: ResourceClassReport,
    pub provider: Option<ProviderKind>,
    pub manifest_integrity: ManifestIntegrity,
    pub cache_state: CacheState,
    pub source_policy: SourcePolicy,
    pub cache_health: CacheHealth,
    pub download_enabled: bool,
    pub download_status: DownloadStatus,
    pub cache_byte_size: u64,
    pub checked_at: String,
    pub unavailable_reason: Option<CacheUnavailableReason>,
    pub storage_error: Option<CacheStorageErrorCode>,
    pub corruption_reason: Option<CacheCorruptionReasonCode>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadStateReport {
    Unavailable,
    Loading,
    Loaded,
    DegradedProviderUnavailable,
    DegradedOverloaded,
    DegradedModelLoadFailed,
    DegradedInvalidOutput,
    DegradedLowConfidence,
    Failed,
}

impl From<LoadState> for LoadStateReport {
    fn from(value: LoadState) -> Self {
        match value {
            LoadState::Unavailable => LoadStateReport::Unavailable,
            LoadState::Loading => LoadStateReport::Loading,
            LoadState::Loaded => LoadStateReport::Loaded,
            LoadState::Degraded(DegradedState::ProviderUnavailable) => {
                LoadStateReport::DegradedProviderUnavailable
            }
            LoadState::Degraded(DegradedState::Overloaded) => LoadStateReport::DegradedOverloaded,
            LoadState::Degraded(DegradedState::ModelLoadFailed) => {
                LoadStateReport::DegradedModelLoadFailed
            }
            LoadState::Degraded(DegradedState::InvalidOutput) => {
                LoadStateReport::DegradedInvalidOutput
            }
            LoadState::Degraded(DegradedState::LowConfidence) => {
                LoadStateReport::DegradedLowConfidence
            }
            LoadState::Failed => LoadStateReport::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceClassReport {
    Cpu,
    Gpu,
    Npu,
}

impl From<ResourceClass> for ResourceClassReport {
    fn from(value: ResourceClass) -> Self {
        match value {
            ResourceClass::Cpu => ResourceClassReport::Cpu,
            ResourceClass::Gpu => ResourceClassReport::Gpu,
            ResourceClass::Npu => ResourceClassReport::Npu,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeManifest {
    pub schema_version: u32,
    pub backend: String,
    pub model_id: String,
    pub revision: String,
    pub artifact_sha256: String,
    pub tokenizer_sha256: String,
    pub provider: Option<ProviderKind>,
    pub task: ModelTask,
    pub dtype: String,
    pub dimension: usize,
    pub formatter_version: String,
    pub chunker_version: String,
    pub parser_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeFile {
    pub path: String,
    pub size_bytes: Option<u64>,
}

impl ModelRuntimeFile {
    pub fn new(path: impl Into<String>, size_bytes: Option<u64>) -> Self {
        Self {
            path: path.into(),
            size_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModelArtifact {
    pub onnx_path: String,
    pub dtype: String,
    pub files: Vec<String>,
    pub has_external_data: bool,
}

impl ModelRuntimeManifest {
    pub fn from_spec(spec: &ModelSpec, backend: &str, provider: Option<ProviderKind>) -> Self {
        Self {
            schema_version: MODEL_RUNTIME_SCHEMA_VERSION,
            backend: backend.to_owned(),
            model_id: spec.model_id.clone(),
            revision: spec.revision.clone(),
            artifact_sha256: spec.artifact_sha256.clone(),
            tokenizer_sha256: spec.tokenizer_sha256.clone(),
            provider,
            task: spec.task,
            dtype: spec.dtype.clone(),
            dimension: spec.dimension,
            formatter_version: "1".to_owned(),
            chunker_version: "1".to_owned(),
            parser_version: "1".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeProof {
    pub schema_version: u32,
    pub backend: String,
    pub zero_network_default: bool,
    pub probe_plan: ModelRuntimeProbePlan,
    pub embedding: ModelCapabilityReport,
    pub reranker: ModelCapabilityReport,
    pub learning_observation_kinds: Vec<ModelRuntimeObservationKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRuntimeObservationKind {
    ModelLoadFailed,
    ProviderDowngraded,
    ArtifactHashMismatch,
    TokenizerHashMismatch,
    DegradedFallback,
    LocalLoadSucceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheStatus {
    pub artifact_ref: String,
    pub manifest_ref: Option<String>,
    pub source_policy: SourcePolicy,
    pub cache_state: CacheState,
    pub cache_health: CacheHealth,
    pub manifest_integrity: ManifestIntegrity,
    pub download_enabled: bool,
    pub download_status: DownloadStatus,
    pub cache_byte_size: u64,
    pub checked_at: String,
    pub unavailable_reason: Option<CacheUnavailableReason>,
    pub storage_error: Option<CacheStorageErrorCode>,
    pub corruption_reason: Option<CacheCorruptionReasonCode>,
}

impl ModelCacheStatus {
    pub fn unavailable(
        artifact_ref: impl Into<String>,
        checked_at: impl Into<String>,
        reason: CacheUnavailableReason,
    ) -> Self {
        Self {
            artifact_ref: artifact_ref.into(),
            manifest_ref: None,
            source_policy: SourcePolicy::Unavailable,
            cache_state: CacheState::Unavailable,
            cache_health: CacheHealth::Unavailable,
            manifest_integrity: ManifestIntegrity::Unavailable,
            download_enabled: false,
            download_status: DownloadStatus::DownloadDisabled,
            cache_byte_size: 0,
            checked_at: checked_at.into(),
            unavailable_reason: Some(reason),
            storage_error: None,
            corruption_reason: None,
        }
    }

    pub fn parent_installed_degraded(
        artifact_ref: impl Into<String>,
        manifest_ref: Option<String>,
        checked_at: impl Into<String>,
    ) -> Self {
        Self {
            artifact_ref: artifact_ref.into(),
            manifest_ref,
            source_policy: SourcePolicy::ParentInstalled,
            cache_state: CacheState::CacheDegraded,
            cache_health: CacheHealth::Degraded,
            manifest_integrity: ManifestIntegrity::Unchecked,
            download_enabled: false,
            download_status: DownloadStatus::DownloadDisabled,
            cache_byte_size: 0,
            checked_at: checked_at.into(),
            unavailable_reason: Some(CacheUnavailableReason::IntegrityUnverified),
            storage_error: None,
            corruption_reason: None,
        }
    }
}

pub fn default_provider_order(preferred: &[ProviderKind]) -> Vec<ProviderKind> {
    let mut providers = Vec::new();
    for provider in preferred {
        push_unique(&mut providers, *provider);
    }
    push_unique(&mut providers, ProviderKind::OpenVino);
    push_unique(&mut providers, ProviderKind::DirectMl);
    push_unique(&mut providers, ProviderKind::Cpu);
    providers
}

pub fn is_onnx_model_file(path: &str) -> bool {
    path.ends_with(".onnx") && !is_onnx_external_data_file(path)
}

pub fn is_onnx_external_data_file(path: &str) -> bool {
    path.ends_with(".onnx_data") || path.ends_with(".onnx.data")
}

pub fn is_model_supporting_file(path: &str) -> bool {
    let filename = file_name(path).to_lowercase();
    const SUPPORTING_FILES: &[&str] = &[
        "config.json",
        "generation_config.json",
        "tokenizer_config.json",
        "tokenizer.json",
        "vocab.json",
        "added_tokens.json",
        "tokenizer.model",
        "special_tokens_map.json",
    ];
    SUPPORTING_FILES.contains(&filename.as_str())
}

pub fn extract_model_dtype(path: &str) -> String {
    let filename = file_name(path);
    let name_without_ext = filename
        .strip_suffix(".onnx")
        .or_else(|| filename.strip_suffix(".onnx_data"))
        .or_else(|| filename.strip_suffix(".onnx.data"))
        .unwrap_or(filename);
    let lowered = name_without_ext.to_lowercase();
    let quant_patterns = [
        "q4", "q4f16", "q8", "int4", "uint4", "fp16", "float16", "fp32", "float32",
    ];

    for pattern in quant_patterns {
        if let Some(index) = lowered.find(pattern) {
            return name_without_ext[index..]
                .split('_')
                .next()
                .unwrap_or(&name_without_ext[index..])
                .to_lowercase();
        }
    }

    "fp32".to_owned()
}

pub fn discover_onnx_artifacts(files: &[ModelRuntimeFile]) -> Vec<DiscoveredModelArtifact> {
    let mut artifacts = Vec::new();
    for onnx_file in files.iter().filter(|file| is_onnx_model_file(&file.path)) {
        let onnx_path = &onnx_file.path;
        let base_name = onnx_path.strip_suffix(".onnx").unwrap_or(onnx_path);
        let data_path_underscore = format!("{base_name}.onnx_data");
        let data_path_dot = format!("{base_name}.onnx.data");
        let mut artifact_files = vec![onnx_path.clone()];
        let mut has_external_data = false;

        for file in files {
            if file.path == data_path_underscore || file.path == data_path_dot {
                artifact_files.push(file.path.clone());
                has_external_data = true;
            }
        }

        let onnx_dir = file_directory(onnx_path);
        let mut supporting_files = Vec::new();
        for file in files {
            if is_model_supporting_file(&file.path) {
                let file_dir = file_directory(&file.path);
                if (file_dir == onnx_dir || file_dir.is_empty())
                    && !artifact_files.contains(&file.path)
                {
                    supporting_files.push(file.path.clone());
                }
            }
        }

        supporting_files.sort_by_key(|path| (supporting_file_rank(path), path.clone()));
        artifact_files.extend(supporting_files);

        artifacts.push(DiscoveredModelArtifact {
            onnx_path: onnx_path.clone(),
            dtype: extract_model_dtype(onnx_path),
            files: artifact_files,
            has_external_data,
        });
    }
    artifacts
}

fn file_directory(path: &str) -> &str {
    path.rsplit_once(['/', '\\'])
        .map(|(directory, _)| directory)
        .unwrap_or("")
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn supporting_file_rank(path: &str) -> usize {
    let filename = file_name(path).to_lowercase();
    match filename.as_str() {
        "config.json" => 0,
        "generation_config.json" => 1,
        "tokenizer_config.json" => 2,
        "tokenizer.json" => 3,
        "vocab.json" => 4,
        "added_tokens.json" => 5,
        "tokenizer.model" => 6,
        "special_tokens_map.json" => 7,
        _ => 8,
    }
}

fn push_unique(providers: &mut Vec<ProviderKind>, provider: ProviderKind) {
    if !providers.contains(&provider) {
        providers.push(provider);
    }
}

fn degraded_cache_contract(
    source_policy: SourcePolicy,
) -> (
    CacheState,
    CacheHealth,
    ManifestIntegrity,
    Option<CacheUnavailableReason>,
) {
    match source_policy {
        SourcePolicy::Unavailable => (
            CacheState::Unavailable,
            CacheHealth::Unavailable,
            ManifestIntegrity::Unavailable,
            Some(CacheUnavailableReason::ModelSourceUnconfigured),
        ),
        SourcePolicy::LocalCache => (
            CacheState::Unavailable,
            CacheHealth::Unavailable,
            ManifestIntegrity::Unavailable,
            Some(CacheUnavailableReason::ArtifactNotInstalled),
        ),
        SourcePolicy::Bundled | SourcePolicy::ParentInstalled => (
            CacheState::CacheDegraded,
            CacheHealth::Degraded,
            ManifestIntegrity::Unchecked,
            Some(CacheUnavailableReason::IntegrityUnverified),
        ),
    }
}

pub fn degraded_capability_report(
    task: ModelTask,
    source_policy: SourcePolicy,
    reason: impl Into<String>,
) -> ModelCapabilityReport {
    let (cache_state, cache_health, manifest_integrity, unavailable_reason) =
        degraded_cache_contract(source_policy);
    ModelCapabilityReport {
        task,
        load_state: LoadState::Degraded(DegradedState::ProviderUnavailable).into(),
        resource_class: ResourceClass::Cpu.into(),
        provider: None,
        manifest_integrity,
        cache_state,
        source_policy,
        cache_health,
        download_enabled: false,
        download_status: DownloadStatus::DownloadDisabled,
        cache_byte_size: 0,
        checked_at: "unavailable".to_owned(),
        unavailable_reason,
        storage_error: None,
        corruption_reason: None,
        reason: Some(reason.into()),
    }
}

pub fn default_zero_network_proof() -> ModelRuntimeProof {
    ModelRuntimeProof {
        schema_version: MODEL_RUNTIME_SCHEMA_VERSION,
        backend: "deterministic-fallback".to_owned(),
        zero_network_default: true,
        probe_plan: default_model_runtime_probe_plan(),
        embedding: degraded_capability_report(
            ModelTask::Embedding,
            SourcePolicy::LocalCache,
            "default build has no compiled real model provider; provider probes remain unavailable",
        ),
        reranker: degraded_capability_report(
            ModelTask::Reranking,
            SourcePolicy::LocalCache,
            "default build has no compiled real model provider; provider probes remain unavailable",
        ),
        learning_observation_kinds: vec![
            ModelRuntimeObservationKind::ModelLoadFailed,
            ModelRuntimeObservationKind::ProviderDowngraded,
            ModelRuntimeObservationKind::ArtifactHashMismatch,
            ModelRuntimeObservationKind::TokenizerHashMismatch,
            ModelRuntimeObservationKind::DegradedFallback,
            ModelRuntimeObservationKind::LocalLoadSucceeded,
        ],
    }
}

pub fn default_model_runtime_probe_plan() -> ModelRuntimeProbePlan {
    ModelRuntimeProbePlan {
        default_probe_filter: "chat".to_owned(),
        one_model_at_a_time: true,
        cpu_first: true,
        gpu_and_npu_require_provider_probe: true,
        provider_probe_timeout_ms: DEFAULT_DEVICE_PROBE_TIMEOUT_MS,
        model_probe_timeout_ms: DEFAULT_MODEL_PROBE_TIMEOUT_MS,
        kill_on_timeout: true,
        minimum_chat_tokens_per_second: DEFAULT_MIN_CHAT_TOKENS_PER_SECOND as u32,
        target_chat_tokens_per_second_low: TARGET_CHAT_TOKENS_PER_SECOND_LOW as u32,
        target_chat_tokens_per_second_high: TARGET_CHAT_TOKENS_PER_SECOND_HIGH as u32,
    }
}

pub fn validate_sha256_hex(value: &str) -> Result<()> {
    let valid = value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(MemoryError::ModelRuntime {
            operation: "validate-sha256",
            reason: format!("expected 64 lowercase/uppercase hex chars, got {value:?}"),
        })
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{digest:x}"))
}

pub fn validate_file_hash(
    path: &Path,
    expected_sha256: &str,
    operation: &'static str,
) -> Result<()> {
    validate_sha256_hex(expected_sha256)?;
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(MemoryError::ModelRuntime {
            operation,
            reason: format!(
                "hash mismatch for {}: expected {expected_sha256}, actual {actual}",
                path.display()
            ),
        })
    }
}

pub fn validate_model_artifacts(spec: &ModelSpec) -> Result<()> {
    validate_file_hash(
        &spec.artifact_path,
        &spec.artifact_sha256,
        "validate-model-artifact",
    )?;
    validate_file_hash(
        &spec.tokenizer_path,
        &spec.tokenizer_sha256,
        "validate-tokenizer-artifact",
    )
}

pub fn validate_embedding_output(vector: &[f32], expected_dimension: usize) -> Result<()> {
    if vector.len() != expected_dimension {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-embedding-output",
            reason: format!(
                "dimension mismatch: expected {expected_dimension}, actual {}",
                vector.len()
            ),
        });
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-embedding-output",
            reason: "embedding contains NaN or infinite value".to_owned(),
        });
    }
    Ok(())
}

pub fn validate_reranker_scores(scores: &[f32], candidate_count: usize) -> Result<()> {
    if scores.len() != candidate_count {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-reranker-output",
            reason: format!(
                "score count mismatch: expected {candidate_count}, actual {}",
                scores.len()
            ),
        });
    }
    if scores.iter().any(|value| !value.is_finite()) {
        return Err(MemoryError::ModelRuntime {
            operation: "validate-reranker-output",
            reason: "reranker score contains NaN or infinite value".to_owned(),
        });
    }
    Ok(())
}

#[cfg(feature = "ort-models")]
pub fn ort_feature_compiled() -> bool {
    true
}

#[cfg(not(feature = "ort-models"))]
pub fn ort_feature_compiled() -> bool {
    false
}
