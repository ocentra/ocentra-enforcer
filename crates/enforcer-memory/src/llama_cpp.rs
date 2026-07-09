//! `llama.cpp` process runner for GGUF proof.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::{Deserialize, Serialize};

use crate::embed::{DegradedState, LoadState};
use crate::error::{MemoryError, Result};
use crate::local_runtime::{
    LocalRuntimeAcceleration, RuntimeActivityState, RuntimeOwnershipMode, RuntimeRequestProtocol,
};
use crate::model_runtime::validate_file_hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlamaCppProbeKind {
    Generate,
    Embedding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlamaCppBackendHint {
    Auto,
    Native,
    Vulkan,
    OpenVino,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppProbeConfig {
    pub binary_path: PathBuf,
    pub model_path: PathBuf,
    pub model_sha256: Option<String>,
    pub prompt: String,
    pub kind: LlamaCppProbeKind,
    pub backend_hint: LlamaCppBackendHint,
    pub acceleration: LocalRuntimeAcceleration,
    pub gpu_layers: Option<usize>,
    pub device: Option<String>,
    pub main_gpu: Option<usize>,
    pub split_mode: Option<String>,
    pub tensor_split: Option<String>,
    pub fit: Option<bool>,
    pub context_size: Option<usize>,
    pub max_tokens: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppProbeReport {
    pub kind: LlamaCppProbeKind,
    pub backend_hint: LlamaCppBackendHint,
    pub requested_acceleration: LocalRuntimeAcceleration,
    pub binary_path: PathBuf,
    pub execution_route: String,
    pub model_path: PathBuf,
    pub exit_code: Option<i32>,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
    pub duration_ms: u128,
    pub measured_tokens_per_second: Option<f64>,
    pub load_state: String,
    pub timed_out: bool,
    pub fallback_reason: Option<String>,
    pub fallback_from_binary_path: Option<PathBuf>,
    pub output_dimensions: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppCommandPlan {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlamaCppLifecycleState {
    Idle,
    ToolchainReady,
    ModelLoading,
    Ready,
    ChatActive,
    EmbeddingActive,
    PausedChat,
    PausedEmbedding,
    Cancelled,
    TimedOut,
    Unloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlamaCppLifecycleAction {
    ResolveToolchain,
    LoadModel,
    MarkReady,
    StartChat,
    StartEmbedding,
    Pause,
    Resume,
    Cancel,
    TimeoutKill,
    Unload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppLifecycleTransition {
    pub before: LlamaCppLifecycleState,
    pub action: LlamaCppLifecycleAction,
    pub after: LlamaCppLifecycleState,
    pub activity: RuntimeActivityState,
    pub ownership: RuntimeOwnershipMode,
    pub request_protocol: RuntimeRequestProtocol,
    pub execution_route: String,
    pub external_server_allowed: bool,
    pub port_binding_allowed: bool,
    pub kill_on_timeout: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppDevice {
    pub id: String,
    pub name: String,
    pub total_memory_mib: u64,
    pub free_memory_mib: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppDeviceReport {
    pub binary_path: PathBuf,
    pub devices: Vec<LlamaCppDevice>,
    pub stderr_excerpt: String,
    pub timed_out: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppExecutionResolution {
    pub requested_backend_hint: LlamaCppBackendHint,
    pub requested_acceleration: LocalRuntimeAcceleration,
    pub backend_hint: LlamaCppBackendHint,
    pub resolved_acceleration: LocalRuntimeAcceleration,
    pub provider_probe_passed: bool,
    pub selected_device_id: Option<String>,
    pub selected_main_gpu: Option<usize>,
    pub detected_free_vram_mib: Option<u64>,
    pub downgrade_reason: Option<String>,
}

impl LlamaCppProbeReport {
    pub fn loaded(&self) -> bool {
        self.exit_code == Some(0) && self.load_state == "loaded"
    }

    pub fn state(&self) -> LoadState {
        if self.loaded() {
            LoadState::Loaded
        } else {
            LoadState::Degraded(DegradedState::ModelLoadFailed)
        }
    }
}

pub fn list_llama_cpp_devices(binary_path: &Path, timeout_ms: u64) -> Result<LlamaCppDeviceReport> {
    validate_executable(binary_path)?;
    let mut command = Command::new(binary_path);
    command
        .arg("--list-devices")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_llama_child_process(&mut command, binary_path);

    let started = Instant::now();
    let mut child = command.spawn().map_err(|source| MemoryError::Io {
        path: binary_path.to_path_buf(),
        source,
    })?;
    let timeout = Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    loop {
        if child
            .try_wait()
            .map_err(|source| MemoryError::Io {
                path: binary_path.to_path_buf(),
                source,
            })?
            .is_some()
        {
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child.kill().map_err(|source| MemoryError::Io {
                path: binary_path.to_path_buf(),
                source,
            })?;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let output = child.wait_with_output().map_err(|source| MemoryError::Io {
        path: binary_path.to_path_buf(),
        source,
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(LlamaCppDeviceReport {
        binary_path: binary_path.to_path_buf(),
        devices: parse_llama_cpp_devices(&stdout),
        stderr_excerpt: excerpt_tail(&String::from_utf8_lossy(&output.stderr), 4096),
        timed_out,
    })
}

pub fn parse_llama_cpp_devices(output: &str) -> Vec<LlamaCppDevice> {
    output
        .lines()
        .filter_map(parse_llama_cpp_device_line)
        .collect()
}

pub fn resolve_llama_cpp_execution(
    requested_backend_hint: LlamaCppBackendHint,
    requested_acceleration: LocalRuntimeAcceleration,
    devices: &[LlamaCppDevice],
) -> LlamaCppExecutionResolution {
    let provider_probe_passed = !devices.is_empty();
    let selected_gpu = devices
        .iter()
        .filter(|device| !device_looks_like_npu(device))
        .max_by_key(|device| device.free_memory_mib);
    let selected_npu = devices.iter().find(|device| device_looks_like_npu(device));

    match requested_acceleration {
        LocalRuntimeAcceleration::Cpu => LlamaCppExecutionResolution {
            requested_backend_hint,
            requested_acceleration,
            backend_hint: resolve_backend_hint(
                requested_backend_hint,
                None,
                requested_acceleration,
            ),
            resolved_acceleration: LocalRuntimeAcceleration::Cpu,
            provider_probe_passed,
            selected_device_id: None,
            selected_main_gpu: None,
            detected_free_vram_mib: None,
            downgrade_reason: None,
        },
        LocalRuntimeAcceleration::Gpu => {
            if let Some(device) = selected_gpu {
                return LlamaCppExecutionResolution {
                    requested_backend_hint,
                    requested_acceleration,
                    backend_hint: resolve_backend_hint(
                        requested_backend_hint,
                        Some(device),
                        requested_acceleration,
                    ),
                    resolved_acceleration: LocalRuntimeAcceleration::Gpu,
                    provider_probe_passed,
                    selected_device_id: Some(device.id.clone()),
                    selected_main_gpu: parse_device_index(&device.id),
                    detected_free_vram_mib: Some(device.free_memory_mib),
                    downgrade_reason: None,
                };
            }
            LlamaCppExecutionResolution {
                requested_backend_hint,
                requested_acceleration,
                backend_hint: LlamaCppBackendHint::Native,
                resolved_acceleration: LocalRuntimeAcceleration::Cpu,
                provider_probe_passed,
                selected_device_id: None,
                selected_main_gpu: None,
                detected_free_vram_mib: None,
                downgrade_reason: Some(
                    "requested GPU acceleration but llama.cpp provider probe did not report a usable GPU device"
                        .to_owned(),
                ),
            }
        }
        LocalRuntimeAcceleration::Npu => {
            if let Some(device) = selected_npu {
                return LlamaCppExecutionResolution {
                    requested_backend_hint,
                    requested_acceleration,
                    backend_hint: LlamaCppBackendHint::OpenVino,
                    resolved_acceleration: LocalRuntimeAcceleration::Npu,
                    provider_probe_passed,
                    selected_device_id: Some(device.id.clone()),
                    selected_main_gpu: None,
                    detected_free_vram_mib: Some(device.free_memory_mib),
                    downgrade_reason: None,
                };
            }
            LlamaCppExecutionResolution {
                requested_backend_hint,
                requested_acceleration,
                backend_hint: LlamaCppBackendHint::Native,
                resolved_acceleration: LocalRuntimeAcceleration::Cpu,
                provider_probe_passed,
                selected_device_id: None,
                selected_main_gpu: None,
                detected_free_vram_mib: None,
                downgrade_reason: Some(
                    "requested NPU acceleration but llama.cpp provider probe did not report an NPU/OpenVINO device"
                        .to_owned(),
                ),
            }
        }
        LocalRuntimeAcceleration::Auto => {
            if let Some(device) = selected_gpu {
                return LlamaCppExecutionResolution {
                    requested_backend_hint,
                    requested_acceleration,
                    backend_hint: resolve_backend_hint(
                        requested_backend_hint,
                        Some(device),
                        LocalRuntimeAcceleration::Gpu,
                    ),
                    resolved_acceleration: LocalRuntimeAcceleration::Gpu,
                    provider_probe_passed,
                    selected_device_id: Some(device.id.clone()),
                    selected_main_gpu: parse_device_index(&device.id),
                    detected_free_vram_mib: Some(device.free_memory_mib),
                    downgrade_reason: None,
                };
            }
            if let Some(device) = selected_npu {
                return LlamaCppExecutionResolution {
                    requested_backend_hint,
                    requested_acceleration,
                    backend_hint: LlamaCppBackendHint::OpenVino,
                    resolved_acceleration: LocalRuntimeAcceleration::Npu,
                    provider_probe_passed,
                    selected_device_id: Some(device.id.clone()),
                    selected_main_gpu: None,
                    detected_free_vram_mib: Some(device.free_memory_mib),
                    downgrade_reason: None,
                };
            }
            LlamaCppExecutionResolution {
                requested_backend_hint,
                requested_acceleration,
                backend_hint: LlamaCppBackendHint::Native,
                resolved_acceleration: LocalRuntimeAcceleration::Cpu,
                provider_probe_passed,
                selected_device_id: None,
                selected_main_gpu: None,
                detected_free_vram_mib: None,
                downgrade_reason: None,
            }
        }
    }
}

pub fn run_llama_cpp_probe(config: &LlamaCppProbeConfig) -> Result<LlamaCppProbeReport> {
    validate_executable(&config.binary_path)?;
    validate_model(&config.model_path)?;
    if let Some(expected_hash) = &config.model_sha256 {
        validate_file_hash(
            &config.model_path,
            expected_hash,
            "validate-llama-model-hash",
        )?;
    }

    let plan = llama_cpp_command_plan(config);
    let mut command = Command::new(&config.binary_path);
    command
        .args(&plan.args)
        .envs(plan.env.iter().map(|(key, value)| (key, value)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_llama_child_process(&mut command, &config.binary_path);

    let started = Instant::now();
    let mut child = command.spawn().map_err(|source| MemoryError::Io {
        path: config.binary_path.clone(),
        source,
    })?;
    let timeout = Duration::from_millis(config.timeout_ms);
    let mut timed_out = false;
    loop {
        if child
            .try_wait()
            .map_err(|source| MemoryError::Io {
                path: config.binary_path.clone(),
                source,
            })?
            .is_some()
        {
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child.kill().map_err(|source| MemoryError::Io {
                path: config.binary_path.clone(),
                source,
            })?;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = child.wait_with_output().map_err(|source| MemoryError::Io {
        path: config.binary_path.clone(),
        source,
    })?;
    let duration_ms = started.elapsed().as_millis();
    let stdout_excerpt = excerpt(&String::from_utf8_lossy(&output.stdout), 4096);
    let stderr_excerpt = excerpt_tail(&String::from_utf8_lossy(&output.stderr), 4096);
    let exit_code = output.status.code();
    let measured_tokens_per_second = if output.status.success() {
        measured_tokens_per_second(config, &stdout_excerpt, &stderr_excerpt, duration_ms)
    } else {
        None
    };
    let load_state = if output.status.success() {
        "loaded"
    } else if timed_out {
        "degraded-model-load-timeout"
    } else {
        "degraded-model-load-failed"
    }
    .to_owned();

    Ok(LlamaCppProbeReport {
        kind: config.kind,
        backend_hint: config.backend_hint,
        requested_acceleration: config.acceleration,
        binary_path: config.binary_path.clone(),
        execution_route: probe_execution_route(&config.binary_path, config.kind),
        model_path: config.model_path.clone(),
        exit_code,
        stdout_excerpt,
        stderr_excerpt,
        duration_ms,
        measured_tokens_per_second,
        load_state,
        timed_out,
        fallback_reason: None,
        fallback_from_binary_path: None,
        output_dimensions: parse_embedding_dimensions_from_output(
            config.kind,
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
        ),
    })
}

pub fn llama_cpp_command_plan(config: &LlamaCppProbeConfig) -> LlamaCppCommandPlan {
    let mut args = vec![
        "-m".to_owned(),
        config.model_path.display().to_string(),
        "-p".to_owned(),
        config.prompt.clone(),
    ];
    let mut env = Vec::new();

    append_acceleration_plan(config, &mut args, &mut env);

    match config.kind {
        LlamaCppProbeKind::Generate => {
            args.push("-n".to_owned());
            args.push(config.max_tokens.to_string());
            args.push("--no-display-prompt".to_owned());
            args.push("-st".to_owned());
            args.push("--simple-io".to_owned());
        }
        LlamaCppProbeKind::Embedding => {
            args.push("--embedding".to_owned());
            args.push("--embd-output-format".to_owned());
            args.push("json".to_owned());
        }
    }

    LlamaCppCommandPlan { args, env }
}

pub fn transition_llama_cpp_lifecycle(
    config: &LlamaCppProbeConfig,
    before: LlamaCppLifecycleState,
    action: LlamaCppLifecycleAction,
) -> Result<LlamaCppLifecycleTransition> {
    validate_llama_cpp_lifecycle_config(config)?;
    let after = match (before, action) {
        (
            LlamaCppLifecycleState::Idle | LlamaCppLifecycleState::Unloaded,
            LlamaCppLifecycleAction::ResolveToolchain,
        ) => LlamaCppLifecycleState::ToolchainReady,
        (LlamaCppLifecycleState::ToolchainReady, LlamaCppLifecycleAction::LoadModel) => {
            LlamaCppLifecycleState::ModelLoading
        }
        (LlamaCppLifecycleState::ModelLoading, LlamaCppLifecycleAction::MarkReady) => {
            LlamaCppLifecycleState::Ready
        }
        (LlamaCppLifecycleState::Ready, LlamaCppLifecycleAction::StartChat) => {
            LlamaCppLifecycleState::ChatActive
        }
        (LlamaCppLifecycleState::Ready, LlamaCppLifecycleAction::StartEmbedding) => {
            LlamaCppLifecycleState::EmbeddingActive
        }
        (LlamaCppLifecycleState::ChatActive, LlamaCppLifecycleAction::Pause) => {
            LlamaCppLifecycleState::PausedChat
        }
        (LlamaCppLifecycleState::EmbeddingActive, LlamaCppLifecycleAction::Pause) => {
            LlamaCppLifecycleState::PausedEmbedding
        }
        (LlamaCppLifecycleState::PausedChat, LlamaCppLifecycleAction::Resume) => {
            LlamaCppLifecycleState::ChatActive
        }
        (LlamaCppLifecycleState::PausedEmbedding, LlamaCppLifecycleAction::Resume) => {
            LlamaCppLifecycleState::EmbeddingActive
        }
        (
            LlamaCppLifecycleState::ModelLoading
            | LlamaCppLifecycleState::ChatActive
            | LlamaCppLifecycleState::EmbeddingActive
            | LlamaCppLifecycleState::PausedChat
            | LlamaCppLifecycleState::PausedEmbedding,
            LlamaCppLifecycleAction::Cancel,
        ) => LlamaCppLifecycleState::Cancelled,
        (
            LlamaCppLifecycleState::ModelLoading
            | LlamaCppLifecycleState::ChatActive
            | LlamaCppLifecycleState::EmbeddingActive
            | LlamaCppLifecycleState::PausedChat
            | LlamaCppLifecycleState::PausedEmbedding,
            LlamaCppLifecycleAction::TimeoutKill,
        ) => LlamaCppLifecycleState::TimedOut,
        (
            LlamaCppLifecycleState::Ready
            | LlamaCppLifecycleState::ChatActive
            | LlamaCppLifecycleState::EmbeddingActive
            | LlamaCppLifecycleState::PausedChat
            | LlamaCppLifecycleState::PausedEmbedding
            | LlamaCppLifecycleState::Cancelled
            | LlamaCppLifecycleState::TimedOut,
            LlamaCppLifecycleAction::Unload,
        ) => LlamaCppLifecycleState::Unloaded,
        _ => {
            return Err(model_error(
                "transition-llama-cpp-lifecycle",
                format!("invalid llama.cpp lifecycle transition: {before:?} + {action:?}"),
            ));
        }
    };

    Ok(LlamaCppLifecycleTransition {
        before,
        action,
        after,
        activity: llama_lifecycle_activity(after),
        ownership: RuntimeOwnershipMode::EnforcerSubprocess,
        request_protocol: RuntimeRequestProtocol::EnforcerStdio,
        execution_route: probe_execution_route(&config.binary_path, config.kind),
        external_server_allowed: false,
        port_binding_allowed: false,
        kill_on_timeout: true,
        reason: llama_lifecycle_reason(action),
    })
}

fn validate_llama_cpp_lifecycle_config(config: &LlamaCppProbeConfig) -> Result<()> {
    if config.timeout_ms == 0 {
        return Err(model_error(
            "validate-llama-cpp-lifecycle-config",
            "llama.cpp lifecycle requires a non-zero timeout",
        ));
    }
    let route = probe_execution_route(&config.binary_path, config.kind);
    if route == "llama-server-v1-embeddings" {
        return Err(model_error(
            "validate-llama-cpp-lifecycle-config",
            "llama.cpp server embedding route is not accepted for Enforcer-owned GGUF parity",
        ));
    }
    Ok(())
}

fn llama_lifecycle_activity(state: LlamaCppLifecycleState) -> RuntimeActivityState {
    match state {
        LlamaCppLifecycleState::ModelLoading => RuntimeActivityState::Loading,
        LlamaCppLifecycleState::ChatActive => RuntimeActivityState::ChatActive,
        LlamaCppLifecycleState::EmbeddingActive => RuntimeActivityState::EmbeddingActive,
        LlamaCppLifecycleState::PausedChat | LlamaCppLifecycleState::PausedEmbedding => {
            RuntimeActivityState::Paused
        }
        _ => RuntimeActivityState::Idle,
    }
}

fn llama_lifecycle_reason(action: LlamaCppLifecycleAction) -> String {
    match action {
        LlamaCppLifecycleAction::ResolveToolchain => {
            "Enforcer resolved the llama.cpp toolchain before model load"
        }
        LlamaCppLifecycleAction::LoadModel => {
            "Enforcer started an owned llama.cpp subprocess for GGUF model load"
        }
        LlamaCppLifecycleAction::MarkReady => "llama.cpp subprocess reported ready for work",
        LlamaCppLifecycleAction::StartChat => {
            "Enforcer admitted foreground chat on the owned llama.cpp subprocess"
        }
        LlamaCppLifecycleAction::StartEmbedding => {
            "Enforcer admitted GGUF embedding work on the owned llama.cpp subprocess"
        }
        LlamaCppLifecycleAction::Pause => "Enforcer paused llama.cpp work for workload arbitration",
        LlamaCppLifecycleAction::Resume => "Enforcer resumed paused llama.cpp work",
        LlamaCppLifecycleAction::Cancel => "Enforcer cancelled the owned llama.cpp subprocess work",
        LlamaCppLifecycleAction::TimeoutKill => {
            "Enforcer killed the owned llama.cpp subprocess after timeout"
        }
        LlamaCppLifecycleAction::Unload => "Enforcer unloaded the owned llama.cpp subprocess",
    }
    .to_owned()
}

fn append_acceleration_plan(
    config: &LlamaCppProbeConfig,
    args: &mut Vec<String>,
    env: &mut Vec<(String, String)>,
) {
    match (config.backend_hint, config.acceleration) {
        (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Auto) => {
            env.push(("GGML_OPENVINO_DEVICE".to_owned(), "CPU".to_owned()));
        }
        (_, LocalRuntimeAcceleration::Auto) => {
            args.push("-ngl".to_owned());
            args.push("0".to_owned());
        }
        (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Cpu) => {
            env.push(("GGML_OPENVINO_DEVICE".to_owned(), "CPU".to_owned()));
        }
        (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Gpu) => {
            env.push(("GGML_OPENVINO_DEVICE".to_owned(), "GPU".to_owned()));
        }
        (_, LocalRuntimeAcceleration::Cpu) => {
            args.push("-ngl".to_owned());
            args.push("0".to_owned());
        }
        (_, LocalRuntimeAcceleration::Gpu) => {
            if let Some(device) = &config.device {
                args.push("--device".to_owned());
                args.push(device.clone());
            }
            args.push("-ngl".to_owned());
            args.push(
                config
                    .gpu_layers
                    .map(|layers| layers.to_string())
                    .unwrap_or_else(|| "auto".to_owned()),
            );
            if let Some(main_gpu) = config.main_gpu {
                args.push("--main-gpu".to_owned());
                args.push(main_gpu.to_string());
            }
            if let Some(split_mode) = &config.split_mode {
                args.push("--split-mode".to_owned());
                args.push(split_mode.clone());
            }
            if let Some(tensor_split) = &config.tensor_split {
                args.push("--tensor-split".to_owned());
                args.push(tensor_split.clone());
            }
            if let Some(fit) = config.fit {
                args.push("--fit".to_owned());
                args.push(if fit { "on" } else { "off" }.to_owned());
            }
        }
        (_, LocalRuntimeAcceleration::Npu) => {
            env.push(("GGML_OPENVINO_DEVICE".to_owned(), "NPU".to_owned()));
            args.push("-c".to_owned());
            args.push(config.context_size.unwrap_or(512).to_string());
        }
    }
}

pub fn validate_executable(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(model_error(
            "validate-llama-executable",
            format!("llama.cpp executable not found: {}", path.display()),
        ))
    }
}

pub fn validate_model(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(model_error(
            "validate-llama-model",
            format!("GGUF model not found: {}", path.display()),
        ))
    }
}

fn excerpt(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn excerpt_tail(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn probe_execution_route(binary_path: &Path, kind: LlamaCppProbeKind) -> String {
    let file_name = binary_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if file_name.contains("llama-server") && kind == LlamaCppProbeKind::Embedding {
        "llama-server-v1-embeddings".to_owned()
    } else if file_name.contains("llama-embedding") {
        "llama-embedding-cli".to_owned()
    } else {
        "llama-cli".to_owned()
    }
}

fn parse_embedding_dimensions_from_output(
    kind: LlamaCppProbeKind,
    stdout: &str,
    stderr: &str,
) -> Option<usize> {
    if kind != LlamaCppProbeKind::Embedding {
        return None;
    }
    parse_embedding_dimensions(stdout).or_else(|| parse_embedding_dimensions(stderr))
}

fn parse_embedding_dimensions(text: &str) -> Option<usize> {
    text.lines().find_map(|line| {
        let marker = "embedding dimensions:";
        let index = line.find(marker)?;
        let suffix = &line[index + marker.len()..];
        let digits: String = suffix
            .chars()
            .skip_while(|ch| ch.is_whitespace())
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        digits.parse().ok()
    })
}

fn parse_llama_cpp_device_line(line: &str) -> Option<LlamaCppDevice> {
    let trimmed = line.trim();
    let (id, rest) = trimmed.split_once(':')?;
    let (name, memory) = rest.rsplit_once('(')?;
    let memory = memory.strip_suffix(')')?;
    let (total, free) = memory.split_once(',')?;
    let total_memory_mib = parse_mib(total.trim())?;
    let free_memory_mib = parse_mib(free.trim().strip_suffix(" free")?.trim())?;
    Some(LlamaCppDevice {
        id: id.trim().to_owned(),
        name: name.trim().to_owned(),
        total_memory_mib,
        free_memory_mib,
    })
}

fn device_looks_like_npu(device: &LlamaCppDevice) -> bool {
    let id = device.id.to_ascii_lowercase();
    let name = device.name.to_ascii_lowercase();
    id.contains("npu") || name.contains("npu") || name.contains("neural")
}

fn resolve_backend_hint(
    requested_backend_hint: LlamaCppBackendHint,
    device: Option<&LlamaCppDevice>,
    acceleration: LocalRuntimeAcceleration,
) -> LlamaCppBackendHint {
    if requested_backend_hint != LlamaCppBackendHint::Auto {
        return requested_backend_hint;
    }
    match acceleration {
        LocalRuntimeAcceleration::Npu => LlamaCppBackendHint::OpenVino,
        LocalRuntimeAcceleration::Gpu => match device {
            Some(device) => {
                let id = device.id.to_ascii_lowercase();
                let name = device.name.to_ascii_lowercase();
                if id.contains("vulkan") || name.contains("vulkan") {
                    LlamaCppBackendHint::Vulkan
                } else if id.contains("openvino") || name.contains("openvino") {
                    LlamaCppBackendHint::OpenVino
                } else {
                    LlamaCppBackendHint::Native
                }
            }
            None => LlamaCppBackendHint::Native,
        },
        LocalRuntimeAcceleration::Auto | LocalRuntimeAcceleration::Cpu => {
            LlamaCppBackendHint::Native
        }
    }
}

fn parse_device_index(device_id: &str) -> Option<usize> {
    let digits_rev: String = device_id
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits_rev.is_empty() {
        return None;
    }
    digits_rev.chars().rev().collect::<String>().parse().ok()
}

fn parse_mib(value: &str) -> Option<u64> {
    value.strip_suffix(" MiB")?.trim().parse().ok()
}

fn configure_llama_child_process(command: &mut Command, binary_path: &Path) {
    if let Some(parent) = binary_path.parent() {
        command.current_dir(parent);
    }
    configure_platform_child_process(command);
}

#[cfg(windows)]
fn configure_platform_child_process(command: &mut Command) {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_platform_child_process(_command: &mut Command) {}

fn measured_tokens_per_second(
    config: &LlamaCppProbeConfig,
    stdout: &str,
    stderr: &str,
    duration_ms: u128,
) -> Option<f64> {
    parse_token_rate(stdout)
        .or_else(|| parse_token_rate(stderr))
        .or_else(|| conservative_token_rate(config, duration_ms))
}

fn conservative_token_rate(config: &LlamaCppProbeConfig, duration_ms: u128) -> Option<f64> {
    if config.kind != LlamaCppProbeKind::Generate || config.max_tokens == 0 || duration_ms == 0 {
        return None;
    }
    Some(config.max_tokens as f64 / (duration_ms as f64 / 1_000.0))
}

fn parse_token_rate(text: &str) -> Option<f64> {
    for line in text.lines().rev() {
        if !line.contains("tokens per second") && !line.contains("tok/s") && !line.contains("t/s") {
            continue;
        }
        if let Some(value) = parse_generation_rate(line) {
            return Some(value);
        }
        let tokens = line
            .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .filter(|part| !part.is_empty());
        for token in tokens {
            if let Ok(value) = token.parse::<f64>() {
                if value.is_finite() && value > 0.0 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_generation_rate(line: &str) -> Option<f64> {
    let (_, after_generation) = line.split_once("Generation:")?;
    let token = after_generation
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .find(|part| !part.is_empty())?;
    token.parse::<f64>().ok().filter(|value| {
        value.is_finite()
            && *value > 0.0
            && (after_generation.contains("t/s") || after_generation.contains("tok/s"))
    })
}

fn model_error(operation: &'static str, reason: impl Into<String>) -> MemoryError {
    MemoryError::ModelRuntime {
        operation,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn llama_binary_name(base_name: &str) -> String {
        format!("{base_name}{}", std::env::consts::EXE_SUFFIX)
    }

    fn contains_arg_pair(args: &[String], key: &str, value: &str) -> bool {
        args.windows(2)
            .any(|pair| pair[0].as_str() == key && pair[1].as_str() == value)
    }

    #[test]
    fn generation_plan_is_single_turn_and_subprocess_safe() {
        let config = LlamaCppProbeConfig {
            binary_path: llama_binary_name("llama-cli").into(),
            model_path: "model.gguf".into(),
            model_sha256: None,
            prompt: "hello".to_owned(),
            kind: LlamaCppProbeKind::Generate,
            backend_hint: LlamaCppBackendHint::Native,
            acceleration: LocalRuntimeAcceleration::Cpu,
            gpu_layers: None,
            device: None,
            main_gpu: None,
            split_mode: None,
            tensor_split: None,
            fit: None,
            context_size: None,
            max_tokens: 8,
            timeout_ms: 1_000,
        };

        let plan = llama_cpp_command_plan(&config);

        assert!(plan.args.iter().any(|arg| arg == "-st"));
        assert!(plan.args.iter().any(|arg| arg == "--simple-io"));
        assert!(plan.args.iter().any(|arg| arg == "--no-display-prompt"));
    }

    #[test]
    fn auto_acceleration_defaults_to_cpu_first() {
        let config = LlamaCppProbeConfig {
            binary_path: llama_binary_name("llama-cli").into(),
            model_path: "model.gguf".into(),
            model_sha256: None,
            prompt: "hello".to_owned(),
            kind: LlamaCppProbeKind::Generate,
            backend_hint: LlamaCppBackendHint::Native,
            acceleration: LocalRuntimeAcceleration::Auto,
            gpu_layers: None,
            device: None,
            main_gpu: None,
            split_mode: None,
            tensor_split: None,
            fit: None,
            context_size: None,
            max_tokens: 8,
            timeout_ms: 1_000,
        };

        let plan = llama_cpp_command_plan(&config);

        assert!(contains_arg_pair(&plan.args, "-ngl", "0"));
        assert!(plan.env.is_empty());
    }

    #[test]
    fn openvino_auto_acceleration_keeps_cpu_device_selection() {
        let config = LlamaCppProbeConfig {
            binary_path: llama_binary_name("llama-cli").into(),
            model_path: "model.gguf".into(),
            model_sha256: None,
            prompt: "hello".to_owned(),
            kind: LlamaCppProbeKind::Generate,
            backend_hint: LlamaCppBackendHint::OpenVino,
            acceleration: LocalRuntimeAcceleration::Auto,
            gpu_layers: None,
            device: None,
            main_gpu: None,
            split_mode: None,
            tensor_split: None,
            fit: None,
            context_size: None,
            max_tokens: 8,
            timeout_ms: 1_000,
        };

        let plan = llama_cpp_command_plan(&config);

        assert!(plan
            .env
            .iter()
            .any(|(key, value)| key == "GGML_OPENVINO_DEVICE" && value == "CPU"));
        assert!(!plan.args.iter().any(|arg| arg == "-ngl"));
    }

    #[test]
    fn current_llama_generation_rate_line_is_parsed() {
        let line = "[ Prompt: 18.9 t/s | Generation: 6.4 t/s ]";

        assert_eq!(parse_generation_rate(line), Some(6.4));
    }

    #[test]
    fn llama_binary_name_matches_platform_suffix() {
        assert_eq!(
            llama_binary_name("llama-cli"),
            format!("llama-cli{}", std::env::consts::EXE_SUFFIX)
        );
    }
}
