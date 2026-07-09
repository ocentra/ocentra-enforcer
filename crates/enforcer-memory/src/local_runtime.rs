//! X06 backend-neutral local runtime adapter contract.
//!
//! This module keeps the local runtime surface explicit without making
//! any inference claims. `llama.cpp` / GGUF is the first-class local
//! backend, `onnx-ort` is an optional backend behind the existing
//! `ort-models` feature gate, and `DeterministicFallback` is the
//! zero-network stand-in that keeps default builds honest.
//!
//! The contract here is selection and validation only:
//! - provider ordering prefers an explicitly configured local backend,
//!   then configured/cache-present `llama.cpp`, then `onnx-ort` when
//!   the feature and cache are both available, then deterministic
//!   fallback;
//! - output validation rejects invalid fixture data instead of faking a
//!   runtime response;
//! - parity claims are only allowed when a real local backend was
//!   selected.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};
use crate::model_runtime::{ModelSpec, ModelTask, ProviderKind, SourcePolicy};

/// Runtime/provider kind for the local model seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalRuntimeKind {
    LlamaCpp,
    OnnxOrt,
    DeterministicFallback,
}

impl LocalRuntimeKind {
    pub fn is_real_backend(self) -> bool {
        !matches!(self, Self::DeterministicFallback)
    }
}

/// Backend alias used by the model-cache contract.
pub type LocalRuntimeBackend = LocalRuntimeKind;

/// Artifact kind inferred from a cache entry path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalRuntimeArtifactKind {
    Manifest,
    Model,
    Tokenizer,
    Config,
    Adapter,
    ExternalData,
    Unknown,
}

/// Runtime acceleration hint for a cached local backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalRuntimeAcceleration {
    Auto,
    Cpu,
    Gpu,
    Npu,
}

/// Runtime workload class for resource arbitration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeWorkload {
    Chat,
    Embedding,
    Reranking,
}

/// Current runtime activity used to decide whether new work may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeActivityState {
    Idle,
    Loading,
    ChatActive,
    EmbeddingActive,
    RerankingActive,
    Paused,
}

/// Admission result for a requested runtime workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeAdmission {
    Admit,
    Queue,
    PauseBackgroundThenAdmit,
}

/// Deterministic resource-arbitration result for the runtime manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeArbitrationDecision {
    pub active: RuntimeActivityState,
    pub requested: RuntimeWorkload,
    pub admission: RuntimeAdmission,
    pub reason: String,
}

/// Who owns the runtime lifecycle for a backend.
///
/// X06 requires Enforcer-owned lifecycle control. External servers can
/// be useful for exploratory proof/debug work, but they cannot be the
/// product contract because Enforcer must own load, unload, cancel,
/// timeout/kill, history policy, provider selection, and cache policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeOwnershipMode {
    EnforcerSubprocess,
    EnforcerIsolatedWorker,
    EnforcerInProcess,
    ExternalServer,
    Unmanaged,
}

impl RuntimeOwnershipMode {
    pub fn is_enforcer_owned(self) -> bool {
        matches!(
            self,
            Self::EnforcerSubprocess | Self::EnforcerIsolatedWorker | Self::EnforcerInProcess
        )
    }
}

/// Request protocol owned by Enforcer for a local runtime worker.
///
/// ORT runs behind an isolated worker protocol, not a model-provider
/// server. This keeps request shaping, history/context policy, and
/// cancellation semantics in Enforcer-owned code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeRequestProtocol {
    EnforcerWorkerEnv,
    EnforcerStdio,
    ExternalHttp,
    None,
}

/// Product responsibilities that must stay in Enforcer even when the
/// low-level backend is llama.cpp or ORT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeManagedCapability {
    LoadUnload,
    PauseResumeCancel,
    TimeoutKill,
    ProviderSelection,
    CachePolicy,
    ChatHistoryPolicy,
    WorkloadAdmission,
}

/// ORT workload executed by Enforcer's isolated worker subprocess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrtWorkerTask {
    Embedding,
    Reranker,
}

impl OrtWorkerTask {
    pub const fn env_value(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Reranker => "reranker",
        }
    }
}

/// Fully materialized ORT child-process contract.
///
/// This is intentionally backend-neutral data rather than a `Command`.
/// Runtime code owns process spawning, but tests and proof artifacts can
/// inspect this plan without loading ORT or touching local hardware.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrtWorkerExecutionPlan {
    pub executable_path: PathBuf,
    pub task: OrtWorkerTask,
    pub provider: ProviderKind,
    pub provider_resolution: OrtProviderResolution,
    pub timeout_ms: u64,
    pub ownership: RuntimeOwnershipMode,
    pub request_protocol: RuntimeRequestProtocol,
    pub external_server_allowed: bool,
    pub port_binding_allowed: bool,
    pub kill_on_timeout: bool,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrtProviderResolution {
    pub requested_provider: ProviderKind,
    pub resolved_provider: ProviderKind,
    pub available_providers: Vec<ProviderKind>,
    pub provider_probe_passed: bool,
    pub downgrade_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackendContract {
    pub llama_cpp: RuntimeBackendContractEntry,
    pub ort: RuntimeBackendContractEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackendContractEntry {
    pub backend: &'static str,
    pub ownership: RuntimeOwnershipMode,
    pub request_protocol: RuntimeRequestProtocol,
    pub external_http_allowed: bool,
    pub port_binding_allowed: bool,
    pub server_surface_accepted_for_parity: bool,
    pub route: &'static str,
    pub managed_by_service: Vec<&'static str>,
}

/// Owned ORT worker lifecycle state.
///
/// This is a control-plane state machine. It does not claim inference
/// parity; it proves Enforcer owns the lifecycle transitions that will
/// later wrap the real ORT worker process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrtWorkerLifecycleState {
    Idle,
    Loading,
    Ready,
    EmbeddingActive,
    RerankingActive,
    PausedEmbedding,
    PausedReranking,
    Cancelled,
    TimedOut,
    Unloaded,
}

/// Lifecycle actions Enforcer may apply to an owned ORT worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrtWorkerLifecycleAction {
    Load,
    MarkReady,
    StartEmbedding,
    StartReranker,
    Pause,
    Resume,
    Cancel,
    TimeoutKill,
    Unload,
}

/// Auditable result of one ORT lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrtWorkerLifecycleTransition {
    pub before: OrtWorkerLifecycleState,
    pub action: OrtWorkerLifecycleAction,
    pub after: OrtWorkerLifecycleState,
    pub activity: RuntimeActivityState,
    pub ownership: RuntimeOwnershipMode,
    pub request_protocol: RuntimeRequestProtocol,
    pub kill_on_timeout: bool,
    pub reason: String,
}

pub const REQUIRED_MANAGED_CAPABILITIES: &[RuntimeManagedCapability] = &[
    RuntimeManagedCapability::LoadUnload,
    RuntimeManagedCapability::PauseResumeCancel,
    RuntimeManagedCapability::TimeoutKill,
    RuntimeManagedCapability::ProviderSelection,
    RuntimeManagedCapability::CachePolicy,
    RuntimeManagedCapability::ChatHistoryPolicy,
    RuntimeManagedCapability::WorkloadAdmission,
];

/// One cached artifact participating in a local runtime candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRuntimeArtifact {
    pub kind: LocalRuntimeArtifactKind,
    pub path: PathBuf,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Candidate local runtime assembled from a cache manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRuntimeCandidate {
    pub backend: LocalRuntimeBackend,
    pub task: ModelTask,
    pub model_id: String,
    pub acceleration: LocalRuntimeAcceleration,
    pub source_policy: SourcePolicy,
    pub artifacts: Vec<LocalRuntimeArtifact>,
}

/// Minimal readiness signal for a backend that depends on a local
/// artifact cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendReadiness {
    pub configured: bool,
    pub cache_present: bool,
}

impl BackendReadiness {
    pub const fn new(configured: bool, cache_present: bool) -> Self {
        Self {
            configured,
            cache_present,
        }
    }

    pub const fn ready(self) -> bool {
        self.configured && self.cache_present
    }
}

/// Fixture used to validate provider ordering and contract behavior
/// without running any actual inference.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalRuntimeFixture {
    pub preferred_backend: Option<LocalRuntimeKind>,
    pub llama_cpp: BackendReadiness,
    pub onnx_ort: BackendReadiness,
    pub output: Option<Vec<f32>>,
    pub parity_claimed: bool,
}

/// Summary of a fixture-backed local runtime validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRuntimeSelectionReport {
    pub ordered_backends: Vec<LocalRuntimeKind>,
    pub selected_backend: LocalRuntimeKind,
    pub real_backend_selected: bool,
}

/// Lifecycle/control contract for a selected runtime backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRuntimeControlPlane {
    pub backend: LocalRuntimeKind,
    pub ownership: RuntimeOwnershipMode,
    pub spawn_controlled: bool,
    pub stop_supported: bool,
    pub timeout_kill_supported: bool,
    pub cache_policy_enforced: bool,
    pub provider_selection_controlled: bool,
    pub managed_capabilities: Vec<RuntimeManagedCapability>,
}

impl OrtWorkerExecutionPlan {
    pub fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    }
}

impl OrtWorkerLifecycleState {
    pub const fn activity(self) -> RuntimeActivityState {
        match self {
            Self::Idle | Self::Ready | Self::Cancelled | Self::TimedOut | Self::Unloaded => {
                RuntimeActivityState::Idle
            }
            Self::Loading => RuntimeActivityState::Loading,
            Self::EmbeddingActive => RuntimeActivityState::EmbeddingActive,
            Self::RerankingActive => RuntimeActivityState::RerankingActive,
            Self::PausedEmbedding | Self::PausedReranking => RuntimeActivityState::Paused,
        }
    }
}

impl LocalRuntimeControlPlane {
    pub fn llama_cpp_managed() -> Self {
        Self {
            backend: LocalRuntimeKind::LlamaCpp,
            ownership: RuntimeOwnershipMode::EnforcerSubprocess,
            spawn_controlled: true,
            stop_supported: true,
            timeout_kill_supported: true,
            cache_policy_enforced: true,
            provider_selection_controlled: true,
            managed_capabilities: REQUIRED_MANAGED_CAPABILITIES.to_vec(),
        }
    }

    pub fn onnx_ort_managed() -> Self {
        Self {
            backend: LocalRuntimeKind::OnnxOrt,
            ownership: RuntimeOwnershipMode::EnforcerIsolatedWorker,
            spawn_controlled: true,
            stop_supported: true,
            timeout_kill_supported: true,
            cache_policy_enforced: true,
            provider_selection_controlled: true,
            managed_capabilities: REQUIRED_MANAGED_CAPABILITIES.to_vec(),
        }
    }

    pub fn externally_owned_server(backend: LocalRuntimeKind) -> Self {
        Self {
            backend,
            ownership: RuntimeOwnershipMode::ExternalServer,
            spawn_controlled: false,
            stop_supported: false,
            timeout_kill_supported: false,
            cache_policy_enforced: false,
            provider_selection_controlled: false,
            managed_capabilities: Vec::new(),
        }
    }
}

impl LocalRuntimeSelectionReport {
    fn new(ordered_backends: Vec<LocalRuntimeKind>) -> Self {
        let selected_backend = ordered_backends
            .first()
            .copied()
            .unwrap_or(LocalRuntimeKind::DeterministicFallback);
        Self {
            ordered_backends,
            selected_backend,
            real_backend_selected: selected_backend.is_real_backend(),
        }
    }
}

fn model_runtime_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
    MemoryError::ModelRuntime {
        operation,
        reason: reason.into(),
    }
}

fn push_unique(order: &mut Vec<LocalRuntimeKind>, kind: LocalRuntimeKind) {
    if !order.contains(&kind) {
        order.push(kind);
    }
}

fn backend_ready(kind: LocalRuntimeKind, fixture: &LocalRuntimeFixture) -> bool {
    match kind {
        LocalRuntimeKind::LlamaCpp => fixture.llama_cpp.ready(),
        LocalRuntimeKind::OnnxOrt => fixture.onnx_ort.ready() && onnx_ort_feature_compiled(),
        LocalRuntimeKind::DeterministicFallback => true,
    }
}

/// Return the provider order for a local-runtime fixture.
///
/// The returned order is deterministic and never contains duplicates.
pub fn provider_order(fixture: &LocalRuntimeFixture) -> Vec<LocalRuntimeKind> {
    let mut order = Vec::new();
    if let Some(preferred) = fixture
        .preferred_backend
        .filter(|kind| kind.is_real_backend())
        .filter(|kind| backend_ready(*kind, fixture))
    {
        push_unique(&mut order, preferred);
    }
    if backend_ready(LocalRuntimeKind::LlamaCpp, fixture) {
        push_unique(&mut order, LocalRuntimeKind::LlamaCpp);
    }
    if backend_ready(LocalRuntimeKind::OnnxOrt, fixture) {
        push_unique(&mut order, LocalRuntimeKind::OnnxOrt);
    }
    push_unique(&mut order, LocalRuntimeKind::DeterministicFallback);
    order
}

/// Infer a cache artifact kind from its path.
pub fn infer_artifact_kind(path: &str) -> Option<LocalRuntimeArtifactKind> {
    let filename = file_name(path).to_lowercase();
    if filename == "manifest.json" {
        return Some(LocalRuntimeArtifactKind::Manifest);
    }
    if filename == "config.json" || filename == "generation_config.json" {
        return Some(LocalRuntimeArtifactKind::Config);
    }
    if filename == "tokenizer.json"
        || filename == "tokenizer.model"
        || filename == "vocab.json"
        || filename == "added_tokens.json"
        || filename == "special_tokens_map.json"
    {
        return Some(LocalRuntimeArtifactKind::Tokenizer);
    }
    if filename == "adapter.bin"
        || filename == "adapter_model.bin"
        || filename.ends_with(".adapter")
        || filename.contains("lora")
    {
        return Some(LocalRuntimeArtifactKind::Adapter);
    }
    if filename.ends_with(".onnx_data") || filename.ends_with(".onnx.data") {
        return Some(LocalRuntimeArtifactKind::ExternalData);
    }
    if filename.ends_with(".onnx")
        || filename.ends_with(".gguf")
        || filename.ends_with(".ggml")
        || filename.ends_with(".bin")
        || filename.ends_with(".safetensors")
    {
        return Some(LocalRuntimeArtifactKind::Model);
    }
    if filename.contains("manifest") {
        return Some(LocalRuntimeArtifactKind::Manifest);
    }
    None
}

/// Validate a runtime output shape for a fixture-backed contract.
pub fn validate_output(output: &[f32], expected_len: usize) -> Result<()> {
    if output.len() != expected_len {
        return Err(model_runtime_error(
            "validate-local-runtime-output",
            format!(
                "output length mismatch: expected {expected_len}, actual {}",
                output.len()
            ),
        ));
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err(model_runtime_error(
            "validate-local-runtime-output",
            "output contains NaN or infinite value",
        ));
    }
    Ok(())
}

/// Validate the local runtime fixture and return the computed selection
/// report.
pub fn validate_fixture(
    fixture: &LocalRuntimeFixture,
    expected_output_len: usize,
) -> Result<LocalRuntimeSelectionReport> {
    if let Some(output) = fixture.output.as_deref() {
        validate_output(output, expected_output_len)?;
    }

    let report = LocalRuntimeSelectionReport::new(provider_order(fixture));
    if fixture.parity_claimed && !report.selected_backend.is_real_backend() {
        return Err(model_runtime_error(
            "validate-local-runtime-fixture",
            "deterministic fallback cannot be claimed as parity",
        ));
    }

    Ok(report)
}

pub fn ort_worker_execution_plan(
    executable_path: impl Into<PathBuf>,
    task: OrtWorkerTask,
    spec: &ModelSpec,
    provider: ProviderKind,
    timeout_ms: u64,
) -> Result<OrtWorkerExecutionPlan> {
    let provider_resolution = resolve_ort_provider(provider, &[provider]);
    ort_worker_execution_plan_with_provider_resolution(
        executable_path,
        task,
        spec,
        provider_resolution,
        timeout_ms,
    )
}

pub fn ort_worker_execution_plan_with_provider_resolution(
    executable_path: impl Into<PathBuf>,
    task: OrtWorkerTask,
    spec: &ModelSpec,
    provider_resolution: OrtProviderResolution,
    timeout_ms: u64,
) -> Result<OrtWorkerExecutionPlan> {
    let control = LocalRuntimeControlPlane::onnx_ort_managed();
    validate_control_plane(&control)?;
    if timeout_ms == 0 {
        return Err(model_runtime_error(
            "build-ort-worker-execution-plan",
            "ORT worker timeout must be greater than zero",
        ));
    }
    let env = vec![
        (
            "ENFORCER_X06_ORT_CHILD_TASK".to_owned(),
            task.env_value().to_owned(),
        ),
        (
            "ENFORCER_X06_CHILD_PROVIDER".to_owned(),
            provider_env_value(provider_resolution.resolved_provider).to_owned(),
        ),
        (
            "ENFORCER_X06_CHILD_REQUESTED_PROVIDER".to_owned(),
            provider_env_value(provider_resolution.requested_provider).to_owned(),
        ),
        (
            "ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS".to_owned(),
            provider_resolution
                .available_providers
                .iter()
                .map(|provider| provider_env_value(*provider))
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "ENFORCER_X06_CHILD_PROVIDER_PROBE_PASSED".to_owned(),
            provider_resolution.provider_probe_passed.to_string(),
        ),
        (
            "ENFORCER_X06_CHILD_MODEL_ID".to_owned(),
            spec.model_id.clone(),
        ),
        (
            "ENFORCER_X06_CHILD_REVISION".to_owned(),
            spec.revision.clone(),
        ),
        (
            "ENFORCER_X06_CHILD_ARTIFACT_PATH".to_owned(),
            spec.artifact_path.display().to_string(),
        ),
        (
            "ENFORCER_X06_CHILD_ARTIFACT_SHA256".to_owned(),
            spec.artifact_sha256.clone(),
        ),
        (
            "ENFORCER_X06_CHILD_TOKENIZER_PATH".to_owned(),
            spec.tokenizer_path.display().to_string(),
        ),
        (
            "ENFORCER_X06_CHILD_TOKENIZER_SHA256".to_owned(),
            spec.tokenizer_sha256.clone(),
        ),
        ("ENFORCER_X06_CHILD_DTYPE".to_owned(), spec.dtype.clone()),
        (
            "ENFORCER_X06_CHILD_DIMENSION".to_owned(),
            spec.dimension.to_string(),
        ),
        (
            "ENFORCER_X06_CHILD_TASK".to_owned(),
            format!("{:?}", spec.task),
        ),
        (
            "ENFORCER_X06_ORT_TIMEOUT_MS".to_owned(),
            timeout_ms.to_string(),
        ),
    ];
    Ok(OrtWorkerExecutionPlan {
        executable_path: executable_path.into(),
        task,
        provider: provider_resolution.resolved_provider,
        provider_resolution,
        timeout_ms,
        ownership: control.ownership,
        request_protocol: RuntimeRequestProtocol::EnforcerWorkerEnv,
        external_server_allowed: false,
        port_binding_allowed: false,
        kill_on_timeout: control.timeout_kill_supported,
        env,
    })
}

pub fn validate_ort_worker_execution_plan(plan: &OrtWorkerExecutionPlan) -> Result<()> {
    if plan.ownership != RuntimeOwnershipMode::EnforcerIsolatedWorker || !plan.kill_on_timeout {
        return Err(model_runtime_error(
            "validate-ort-worker-execution-plan",
            "ORT worker must be an Enforcer-owned isolated worker with timeout kill support",
        ));
    }
    if plan.request_protocol != RuntimeRequestProtocol::EnforcerWorkerEnv
        || plan.external_server_allowed
        || plan.port_binding_allowed
    {
        return Err(model_runtime_error(
            "validate-ort-worker-execution-plan",
            "ORT worker must use Enforcer-owned worker protocol without external server or port binding",
        ));
    }
    if plan.timeout_ms == 0 {
        return Err(model_runtime_error(
            "validate-ort-worker-execution-plan",
            "ORT worker timeout must be greater than zero",
        ));
    }
    for required in [
        "ENFORCER_X06_ORT_CHILD_TASK",
        "ENFORCER_X06_CHILD_PROVIDER",
        "ENFORCER_X06_CHILD_REQUESTED_PROVIDER",
        "ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS",
        "ENFORCER_X06_CHILD_PROVIDER_PROBE_PASSED",
        "ENFORCER_X06_CHILD_MODEL_ID",
        "ENFORCER_X06_CHILD_ARTIFACT_PATH",
        "ENFORCER_X06_CHILD_ARTIFACT_SHA256",
        "ENFORCER_X06_CHILD_TOKENIZER_PATH",
        "ENFORCER_X06_CHILD_TOKENIZER_SHA256",
        "ENFORCER_X06_ORT_TIMEOUT_MS",
    ] {
        if plan.env_value(required).is_none() {
            return Err(model_runtime_error(
                "validate-ort-worker-execution-plan",
                format!("ORT worker plan missing required env {required}"),
            ));
        }
    }
    if plan.provider != plan.provider_resolution.resolved_provider {
        return Err(model_runtime_error(
            "validate-ort-worker-execution-plan",
            "ORT worker provider must match the probed provider resolution",
        ));
    }
    if plan.provider != ProviderKind::Cpu && !plan.provider_resolution.provider_probe_passed {
        return Err(model_runtime_error(
            "validate-ort-worker-execution-plan",
            "accelerated ORT provider requires positive provider probe evidence",
        ));
    }
    Ok(())
}

pub fn resolve_ort_provider(
    requested_provider: ProviderKind,
    available_providers: &[ProviderKind],
) -> OrtProviderResolution {
    let mut available = Vec::new();
    for provider in available_providers {
        if !available.contains(provider) {
            available.push(*provider);
        }
    }
    if !available.contains(&ProviderKind::Cpu) {
        available.push(ProviderKind::Cpu);
    }
    let provider_probe_passed = !available_providers.is_empty();
    if requested_provider == ProviderKind::Cpu || available.contains(&requested_provider) {
        return OrtProviderResolution {
            requested_provider,
            resolved_provider: requested_provider,
            available_providers: available,
            provider_probe_passed,
            downgrade_reason: None,
        };
    }
    OrtProviderResolution {
        requested_provider,
        resolved_provider: ProviderKind::Cpu,
        available_providers: available,
        provider_probe_passed,
        downgrade_reason: Some(format!(
            "requested ORT provider {} but provider probe did not report it available; downgraded to cpu",
            provider_env_value(requested_provider)
        )),
    }
}

pub fn ort_worker_command(plan: &OrtWorkerExecutionPlan) -> Result<Command> {
    validate_ort_worker_execution_plan(plan)?;
    let mut command = Command::new(&plan.executable_path);
    command
        .arg("--x06-ort-worker")
        .arg("--task")
        .arg(plan.task.env_value())
        .arg("--provider")
        .arg(provider_env_value(plan.provider))
        .envs(plan.env.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(command)
}

pub fn runtime_backend_contract() -> RuntimeBackendContract {
    RuntimeBackendContract {
        llama_cpp: RuntimeBackendContractEntry {
            backend: "gguf",
            ownership: RuntimeOwnershipMode::EnforcerSubprocess,
            request_protocol: RuntimeRequestProtocol::EnforcerStdio,
            external_http_allowed: false,
            port_binding_allowed: false,
            server_surface_accepted_for_parity: false,
            route: "enforcer-managed-llama-cpp-subprocess",
            managed_by_service: vec!["chat", "embeddings"],
        },
        ort: RuntimeBackendContractEntry {
            backend: "onnx",
            ownership: RuntimeOwnershipMode::EnforcerIsolatedWorker,
            request_protocol: RuntimeRequestProtocol::EnforcerWorkerEnv,
            external_http_allowed: false,
            port_binding_allowed: false,
            server_surface_accepted_for_parity: false,
            route: "enforcer-isolated-ort-worker",
            managed_by_service: vec!["embeddings", "rerank"],
        },
    }
}

pub fn transition_ort_worker_lifecycle(
    plan: &OrtWorkerExecutionPlan,
    before: OrtWorkerLifecycleState,
    action: OrtWorkerLifecycleAction,
) -> Result<OrtWorkerLifecycleTransition> {
    validate_ort_worker_execution_plan(plan)?;
    let (after, reason) = match (before, action) {
        (
            OrtWorkerLifecycleState::Idle | OrtWorkerLifecycleState::Unloaded,
            OrtWorkerLifecycleAction::Load,
        ) => (
            OrtWorkerLifecycleState::Loading,
            "Enforcer starts the isolated ORT worker load path",
        ),
        (OrtWorkerLifecycleState::Loading, OrtWorkerLifecycleAction::MarkReady) => (
            OrtWorkerLifecycleState::Ready,
            "ORT worker reported model and tokenizer ready",
        ),
        (OrtWorkerLifecycleState::Ready, OrtWorkerLifecycleAction::StartEmbedding) => (
            OrtWorkerLifecycleState::EmbeddingActive,
            "Enforcer admitted an embedding request to the ready ORT worker",
        ),
        (OrtWorkerLifecycleState::Ready, OrtWorkerLifecycleAction::StartReranker) => (
            OrtWorkerLifecycleState::RerankingActive,
            "Enforcer admitted a reranker request to the ready ORT worker",
        ),
        (OrtWorkerLifecycleState::EmbeddingActive, OrtWorkerLifecycleAction::Pause) => (
            OrtWorkerLifecycleState::PausedEmbedding,
            "Enforcer paused background embedding work",
        ),
        (OrtWorkerLifecycleState::RerankingActive, OrtWorkerLifecycleAction::Pause) => (
            OrtWorkerLifecycleState::PausedReranking,
            "Enforcer paused background reranker work",
        ),
        (OrtWorkerLifecycleState::PausedEmbedding, OrtWorkerLifecycleAction::Resume) => (
            OrtWorkerLifecycleState::EmbeddingActive,
            "Enforcer resumed paused embedding work",
        ),
        (OrtWorkerLifecycleState::PausedReranking, OrtWorkerLifecycleAction::Resume) => (
            OrtWorkerLifecycleState::RerankingActive,
            "Enforcer resumed paused reranker work",
        ),
        (
            OrtWorkerLifecycleState::Loading
            | OrtWorkerLifecycleState::EmbeddingActive
            | OrtWorkerLifecycleState::RerankingActive
            | OrtWorkerLifecycleState::PausedEmbedding
            | OrtWorkerLifecycleState::PausedReranking,
            OrtWorkerLifecycleAction::Cancel,
        ) => (
            OrtWorkerLifecycleState::Cancelled,
            "Enforcer cancelled the owned ORT worker operation",
        ),
        (
            OrtWorkerLifecycleState::Loading
            | OrtWorkerLifecycleState::EmbeddingActive
            | OrtWorkerLifecycleState::RerankingActive
            | OrtWorkerLifecycleState::PausedEmbedding
            | OrtWorkerLifecycleState::PausedReranking,
            OrtWorkerLifecycleAction::TimeoutKill,
        ) => (
            OrtWorkerLifecycleState::TimedOut,
            "Enforcer killed the owned ORT worker after timeout",
        ),
        (
            OrtWorkerLifecycleState::Ready
            | OrtWorkerLifecycleState::EmbeddingActive
            | OrtWorkerLifecycleState::RerankingActive
            | OrtWorkerLifecycleState::PausedEmbedding
            | OrtWorkerLifecycleState::PausedReranking
            | OrtWorkerLifecycleState::Cancelled
            | OrtWorkerLifecycleState::TimedOut,
            OrtWorkerLifecycleAction::Unload,
        ) => (
            OrtWorkerLifecycleState::Unloaded,
            "Enforcer unloaded the ORT worker and released runtime ownership",
        ),
        _ => {
            return Err(model_runtime_error(
                "transition-ort-worker-lifecycle",
                format!("invalid ORT lifecycle transition: {before:?} + {action:?}"),
            ));
        }
    };

    Ok(OrtWorkerLifecycleTransition {
        before,
        action,
        after,
        activity: after.activity(),
        ownership: plan.ownership,
        request_protocol: plan.request_protocol,
        kill_on_timeout: plan.kill_on_timeout,
        reason: reason.to_owned(),
    })
}

pub fn provider_env_value(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Cpu => "cpu",
        ProviderKind::DirectMl => "direct-ml",
        ProviderKind::OpenVino => "open-vino",
        ProviderKind::Vulkan => "vulkan",
        ProviderKind::Cuda => "cuda",
        ProviderKind::CoreMl => "core-ml",
        ProviderKind::Npu => "npu",
    }
}

pub fn provider_from_env_value(value: &str) -> Option<ProviderKind> {
    match value {
        "cpu" | "Cpu" | "CPU" => Some(ProviderKind::Cpu),
        "direct-ml" | "directml" | "DirectMl" => Some(ProviderKind::DirectMl),
        "open-vino" | "openvino" | "OpenVino" => Some(ProviderKind::OpenVino),
        "vulkan" | "Vulkan" => Some(ProviderKind::Vulkan),
        "cuda" | "Cuda" | "CUDA" => Some(ProviderKind::Cuda),
        "core-ml" | "coreml" | "CoreMl" => Some(ProviderKind::CoreMl),
        "npu" | "Npu" | "NPU" => Some(ProviderKind::Npu),
        _ => None,
    }
}

pub fn arbitrate_runtime_workload(
    active: RuntimeActivityState,
    requested: RuntimeWorkload,
) -> RuntimeArbitrationDecision {
    let (admission, reason) = match (active, requested) {
        (RuntimeActivityState::Idle | RuntimeActivityState::Paused, _) => (
            RuntimeAdmission::Admit,
            "runtime is available for requested workload",
        ),
        (RuntimeActivityState::Loading, _) => (
            RuntimeAdmission::Queue,
            "model load is exclusive; queue requested workload",
        ),
        (RuntimeActivityState::ChatActive, RuntimeWorkload::Chat) => (
            RuntimeAdmission::Queue,
            "chat is already active; queue the next chat turn",
        ),
        (RuntimeActivityState::ChatActive, _) => (
            RuntimeAdmission::Queue,
            "chat has priority; queue background model work",
        ),
        (
            RuntimeActivityState::EmbeddingActive | RuntimeActivityState::RerankingActive,
            RuntimeWorkload::Chat,
        ) => (
            RuntimeAdmission::PauseBackgroundThenAdmit,
            "chat request preempts background retrieval work",
        ),
        (
            RuntimeActivityState::EmbeddingActive | RuntimeActivityState::RerankingActive,
            RuntimeWorkload::Embedding | RuntimeWorkload::Reranking,
        ) => (
            RuntimeAdmission::Queue,
            "background retrieval work is already active; queue requested workload",
        ),
    };
    RuntimeArbitrationDecision {
        active,
        requested,
        admission,
        reason: reason.to_owned(),
    }
}

/// Validate that a runtime backend is acceptable for X06 product parity.
pub fn validate_control_plane(control: &LocalRuntimeControlPlane) -> Result<()> {
    if !control.ownership.is_enforcer_owned() {
        return Err(model_runtime_error(
            "validate-local-runtime-control-plane",
            format!(
                "{:?} is not Enforcer-owned; external server/unmanaged runtimes cannot claim X06 parity",
                control.backend
            ),
        ));
    }
    let expected_ownership = match control.backend {
        LocalRuntimeKind::LlamaCpp => RuntimeOwnershipMode::EnforcerSubprocess,
        LocalRuntimeKind::OnnxOrt => RuntimeOwnershipMode::EnforcerIsolatedWorker,
        LocalRuntimeKind::DeterministicFallback => RuntimeOwnershipMode::Unmanaged,
    };
    if control.backend.is_real_backend() && control.ownership != expected_ownership {
        return Err(model_runtime_error(
            "validate-local-runtime-control-plane",
            format!(
                "{:?} must use {:?} ownership for X06 parity, got {:?}",
                control.backend, expected_ownership, control.ownership
            ),
        ));
    }
    if !control.stop_supported || !control.timeout_kill_supported {
        return Err(model_runtime_error(
            "validate-local-runtime-control-plane",
            "runtime must support stop plus timeout/kill control",
        ));
    }
    if !control.cache_policy_enforced || !control.provider_selection_controlled {
        return Err(model_runtime_error(
            "validate-local-runtime-control-plane",
            "runtime must enforce Enforcer cache policy and provider selection",
        ));
    }
    let missing_capabilities: Vec<RuntimeManagedCapability> = REQUIRED_MANAGED_CAPABILITIES
        .iter()
        .copied()
        .filter(|capability| !control.managed_capabilities.contains(capability))
        .collect();
    if !missing_capabilities.is_empty() {
        return Err(model_runtime_error(
            "validate-local-runtime-control-plane",
            format!("runtime is missing managed capabilities: {missing_capabilities:?}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "ort-models")]
pub fn onnx_ort_feature_compiled() -> bool {
    true
}

#[cfg(not(feature = "ort-models"))]
pub fn onnx_ort_feature_compiled() -> bool {
    false
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
