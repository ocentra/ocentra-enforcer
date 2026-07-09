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

use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};
use crate::model_runtime::{ModelTask, SourcePolicy};

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
    EnforcerInProcess,
    ExternalServer,
    Unmanaged,
}

impl RuntimeOwnershipMode {
    pub fn is_enforcer_owned(self) -> bool {
        matches!(self, Self::EnforcerSubprocess | Self::EnforcerInProcess)
    }
}

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
        }
    }

    pub fn onnx_ort_managed() -> Self {
        Self {
            backend: LocalRuntimeKind::OnnxOrt,
            ownership: RuntimeOwnershipMode::EnforcerInProcess,
            spawn_controlled: false,
            stop_supported: true,
            timeout_kill_supported: true,
            cache_policy_enforced: true,
            provider_selection_controlled: true,
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
