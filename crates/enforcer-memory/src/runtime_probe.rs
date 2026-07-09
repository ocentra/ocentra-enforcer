//! Real-model X06 runtime probe harness.
//!
//! This is the reusable library home for the runtime-proof path that
//! used to live only in `examples/x06_model_runtime_probe.rs`. The
//! example remains as a thin wrapper so other crates/projects can call
//! the same env-driven harness logic directly.

#[cfg(not(feature = "real-models"))]
pub fn write_runtime_probe_stdout() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;

    let proof = crate::model_runtime::default_zero_network_proof();
    let proof_text = format!("{}\n", serde_json::to_string_pretty(&proof)?);
    std::io::stdout().write_all(proof_text.as_bytes())?;
    Ok(())
}

#[cfg(feature = "real-models")]
pub fn write_runtime_probe_stdout() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::Duration;
    use std::time::Instant;

    use crate::hf_cache::{
        download_hf_model, resolve_cached_hf_model, resolve_cached_hf_model_from_manifest,
        select_x06_chat_model_for_hardware, HfDownloadReport, HfDownloadedFile, HfModelSpec,
        X06ModelLineup,
    };
    use crate::llama_cpp::{
        list_llama_cpp_devices, llama_binary_name, resolve_llama_cpp_execution,
        run_llama_cpp_probe, LlamaCppBackendHint, LlamaCppExecutionResolution, LlamaCppProbeConfig,
        LlamaCppProbeKind,
    };
    use crate::local_runtime::LocalRuntimeAcceleration;
    use crate::model_observations::{
        LocalLoadSucceeded, ModelLoadFailure, ModelRuntimeObservationCandidate,
        ModelRuntimeObservationRecord, ProviderDowngrade,
    };
    use crate::model_runtime::{
        default_model_runtime_probe_plan, evaluate_chat_usability, loaded_non_chat_usability,
        resolve_model_cache_root, sha256_file, ChatThroughputPolicy, ModelCacheRootMode,
        ModelRuntimeServiceConfig, ModelSpec, ModelTask, ModelUsabilityReport, ProviderKind,
        DEFAULT_EMBEDDING_MODEL_ID, DEFAULT_MIN_CHAT_TOKENS_PER_SECOND, DEFAULT_RERANKER_MODEL_ID,
        TARGET_CHAT_TOKENS_PER_SECOND_HIGH, TARGET_CHAT_TOKENS_PER_SECOND_LOW,
    };
    use crate::ort_runtime::{OrtEmbedder, OrtReranker};
    use crate::ranking::RankedHit;
    use crate::search::document::DocumentKind;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeProbeProof {
        schema_version: u32,
        runtime_mode: String,
        proof_scope: serde_json::Value,
        allow_network: bool,
        probe_execution_policy: serde_json::Value,
        cache_root: String,
        cache_root_policy: serde_json::Value,
        service_config: serde_json::Value,
        chat_throughput_policy: ChatThroughputPolicy,
        chat_model_selection: serde_json::Value,
        chat_generation_gguf: serde_json::Value,
        qwen_embedding_gguf: serde_json::Value,
        qwen_embedding_onnx: serde_json::Value,
        qwen_reranker_onnx: serde_json::Value,
        observations: Vec<ModelRuntimeObservationRecord>,
    }

    const EXPECTED_QWEN_EMBEDDING_DIMENSIONS: usize = 1024;

    #[derive(Debug, Clone, Copy)]
    struct CacheProofMode {
        cache_only: bool,
        download_enabled: bool,
        network_may_be_attempted: bool,
    }

    #[derive(Debug, Clone, Copy)]
    struct ChatSelectionContext<'a> {
        repo_root: &'a Path,
        requested_backend_hint: LlamaCppBackendHint,
        execution: &'a LlamaCppExecutionResolution,
        chat_probe_selected: bool,
        device_report: Option<&'a crate::llama_cpp::LlamaCppDeviceReport>,
    }

    #[derive(Debug, Clone)]
    struct LlamaRunContext<'a> {
        repo_root: &'a Path,
        backend_hint: LlamaCppBackendHint,
        acceleration: LocalRuntimeAcceleration,
        selected_device_id: Option<String>,
        selected_main_gpu: Option<usize>,
        default_split_mode: Option<String>,
        observed_at: &'a str,
        run_id: &'a str,
    }

    struct LlamaResultInput<'a> {
        operation: &'a str,
        result: crate::error::Result<crate::llama_cpp::LlamaCppProbeReport>,
        model_id: String,
        task: ModelTask,
        provider: ProviderKind,
        observed_at: &'a str,
        run_id: &'a str,
        repo_root: &'a Path,
    }

    struct LoadFailureInput<'a> {
        observed_at: &'a str,
        run_id: &'a str,
        model_id: String,
        task: ModelTask,
        requested_provider: Option<ProviderKind>,
        failure_reason: String,
    }

    let allow_network = env_truthy("ENFORCER_X06_ALLOW_NETWORK");
    let repo_root = std::env::current_dir()?;
    let cache_root_mode = cache_root_mode_from_env();
    let cache_root_policy = resolve_model_cache_root(
        &repo_root,
        cache_root_mode,
        std::env::var("ENFORCER_X06_MODEL_CACHE")
            .ok()
            .map(PathBuf::from),
    );
    let cache_root = cache_root_policy.root.clone();
    let service_config = ModelRuntimeServiceConfig::dev(&repo_root);
    let proof_out = std::env::var("ENFORCER_X06_PROOF_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from("proof")
                .join("memory")
                .join("x06-models.json")
        });
    let run_id = std::env::var("ENFORCER_X06_RUN_ID")
        .unwrap_or_else(|_| "x06-model-runtime-probe".to_owned());
    let observed_at = std::env::var("ENFORCER_X06_OBSERVED_AT")
        .unwrap_or_else(|_| "manual-runtime-probe".to_owned());
    let runtime_mode = runtime_mode_from_env();
    if let Ok(child_task) = std::env::var("ENFORCER_X06_ORT_CHILD_TASK") {
        let proof_text = format!("{}\n", serde_json::to_string(&run_ort_child(&child_task))?);
        std::io::stdout().write_all(proof_text.as_bytes())?;
        return Ok(());
    }
    let requested_llama_acceleration = llama_acceleration_from_env();
    let llama_backend_hint = llama_backend_hint_from_env();
    let probe_filter = probe_filter_from_env();
    let probe_execution_policy = probe_execution_policy(&probe_filter);
    let chat_throughput_policy = chat_throughput_policy_from_env();

    let mut lineup = X06ModelLineup::from_env()?;
    let chat_probe_selected = should_run_probe(&probe_filter, "chat-generation-gguf");
    let device_report = maybe_llama_device_report(runtime_mode == "probe");
    let llama_execution = resolve_llama_cpp_execution(
        llama_backend_hint,
        requested_llama_acceleration,
        device_report
            .as_ref()
            .map(|report| report.devices.as_slice())
            .unwrap_or(&[]),
    );
    let chat_selection_context = ChatSelectionContext {
        repo_root: &repo_root,
        requested_backend_hint: llama_backend_hint,
        execution: &llama_execution,
        chat_probe_selected,
        device_report: device_report.as_ref(),
    };
    let chat_model_selection =
        maybe_select_chat_model_for_hardware(&mut lineup, chat_selection_context);
    let llama_run_context = LlamaRunContext {
        repo_root: &repo_root,
        backend_hint: llama_execution.backend_hint,
        acceleration: llama_execution.resolved_acceleration,
        selected_device_id: llama_execution.selected_device_id.clone(),
        selected_main_gpu: llama_execution.selected_main_gpu,
        default_split_mode: default_split_mode_for_execution(&llama_execution),
        observed_at: &observed_at,
        run_id: &run_id,
    };
    let mut observations = Vec::new();
    if let Some(observation) = provider_downgrade_observation(
        &llama_execution,
        &lineup.chat_generation.model_id,
        ModelTask::Summarization,
        &observed_at,
        &run_id,
    ) {
        observations.push(observation);
    }

    if runtime_mode == "plan" {
        let proof = RuntimeProbeProof {
            schema_version: 1,
            runtime_mode,
            proof_scope: proof_scope("plan"),
            allow_network,
            probe_execution_policy,
            cache_root: repo_relative_display(&repo_root, &cache_root),
            cache_root_policy: cache_root_policy_proof(&repo_root, &cache_root_policy)?,
            service_config: service_config_proof(&repo_root, &service_config)?,
            chat_throughput_policy,
            chat_model_selection,
            chat_generation_gguf: proof_skipped_reason(
                "chat-generation-gguf",
                "runtime mode plan does not download or load models",
            ),
            qwen_embedding_gguf: proof_skipped_reason(
                "qwen-embedding-gguf",
                "runtime mode plan does not download or load models",
            ),
            qwen_embedding_onnx: proof_skipped_reason(
                "qwen-embedding-onnx",
                "runtime mode plan does not download or load models",
            ),
            qwen_reranker_onnx: proof_skipped_reason(
                "qwen-reranker-onnx",
                "runtime mode plan does not download or load models",
            ),
            observations,
        };
        write_proof(&proof_out, &proof)?;
        return Ok(());
    }

    if runtime_mode == "download" || runtime_mode == "cache-only" {
        let cache_only = runtime_mode == "cache-only";
        let cache_probe = |spec: &HfModelSpec, operation: &str| {
            if cache_only {
                cache_only_probe(&repo_root, spec, &cache_root, operation)
            } else {
                cache_or_download_probe(&repo_root, spec, &cache_root, allow_network, operation)
            }
        };
        let proof = RuntimeProbeProof {
            schema_version: 1,
            runtime_mode: runtime_mode.clone(),
            proof_scope: proof_scope(&runtime_mode),
            allow_network: allow_network && !cache_only,
            probe_execution_policy,
            cache_root: repo_relative_display(&repo_root, &cache_root),
            cache_root_policy: cache_root_policy_proof(&repo_root, &cache_root_policy)?,
            service_config: service_config_proof(&repo_root, &service_config)?,
            chat_throughput_policy,
            chat_model_selection,
            chat_generation_gguf: if should_run_probe(&probe_filter, "chat-generation-gguf") {
                cache_probe(&lineup.chat_generation, "chat-generation-cache")
            } else {
                proof_skipped("chat-generation-cache")
            },
            qwen_embedding_gguf: if should_run_probe(&probe_filter, "qwen-embedding-gguf") {
                cache_probe(&lineup.embedding_gguf, "qwen-embedding-gguf-cache")
            } else {
                proof_skipped("qwen-embedding-gguf-cache")
            },
            qwen_embedding_onnx: if should_run_probe(&probe_filter, "qwen-embedding-onnx") {
                cache_probe(&lineup.embedding_onnx, "qwen-embedding-onnx-cache")
            } else {
                proof_skipped("qwen-embedding-onnx-cache")
            },
            qwen_reranker_onnx: if should_run_probe(&probe_filter, "qwen-reranker-onnx") {
                cache_probe(&lineup.reranker_onnx, "qwen-reranker-onnx-cache")
            } else {
                proof_skipped("qwen-reranker-onnx-cache")
            },
            observations,
        };
        write_proof(&proof_out, &proof)?;
        return Ok(());
    }

    let chat_generation_gguf = if should_run_probe(&probe_filter, "chat-generation-gguf") {
        match maybe_direct_chat_model_report(&cache_root)
            .or_else(|| maybe_download(&lineup.chat_generation, &cache_root, allow_network).ok())
        {
            Some(report) => run_llama_generation(&report, &llama_run_context, &mut observations),
            None => match maybe_download(&lineup.chat_generation, &cache_root, allow_network) {
                Ok(report) => run_llama_generation(&report, &llama_run_context, &mut observations),
                Err(error) => {
                    observations.push(load_failure_observation(LoadFailureInput {
                        observed_at: &observed_at,
                        run_id: &run_id,
                        model_id: lineup.chat_generation.model_id.clone(),
                        task: lineup.chat_generation.task,
                        requested_provider: None,
                        failure_reason: error.clone(),
                    }));
                    proof_error("chat-generation-cache", error)
                }
            },
        }
    } else {
        proof_skipped("chat-generation-gguf")
    };

    let qwen_embedding_gguf_proof = if should_run_probe(&probe_filter, "qwen-embedding-gguf") {
        match maybe_download(&lineup.embedding_gguf, &cache_root, allow_network) {
            Ok(report) => run_llama_embedding(&report, &llama_run_context, &mut observations),
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at: &observed_at,
                    run_id: &run_id,
                    model_id: lineup.embedding_gguf.model_id.clone(),
                    task: lineup.embedding_gguf.task,
                    requested_provider: None,
                    failure_reason: error.clone(),
                }));
                proof_error("qwen-embedding-gguf-cache", error)
            }
        }
    } else {
        proof_skipped("qwen-embedding-gguf")
    };

    let qwen_embedding_onnx_proof = if should_run_probe(&probe_filter, "qwen-embedding-onnx") {
        match maybe_download(&lineup.embedding_onnx, &cache_root, allow_network) {
            Ok(report) => run_ort_embedding(&report, &mut observations, &observed_at, &run_id),
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at: &observed_at,
                    run_id: &run_id,
                    model_id: lineup.embedding_onnx.model_id.clone(),
                    task: lineup.embedding_onnx.task,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: error.clone(),
                }));
                proof_error("qwen-embedding-onnx-cache", error)
            }
        }
    } else {
        proof_skipped("qwen-embedding-onnx")
    };

    let qwen_reranker_onnx_proof = if should_run_probe(&probe_filter, "qwen-reranker-onnx") {
        match maybe_download(&lineup.reranker_onnx, &cache_root, allow_network) {
            Ok(report) => run_ort_reranker(&report, &mut observations, &observed_at, &run_id),
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at: &observed_at,
                    run_id: &run_id,
                    model_id: lineup.reranker_onnx.model_id.clone(),
                    task: lineup.reranker_onnx.task,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: error.clone(),
                }));
                proof_error("qwen-reranker-onnx-cache", error)
            }
        }
    } else {
        proof_skipped("qwen-reranker-onnx")
    };

    let proof = RuntimeProbeProof {
        schema_version: 1,
        runtime_mode,
        proof_scope: proof_scope("probe"),
        allow_network,
        probe_execution_policy,
        cache_root: repo_relative_display(&repo_root, &cache_root),
        cache_root_policy: cache_root_policy_proof(&repo_root, &cache_root_policy)?,
        service_config: service_config_proof(&repo_root, &service_config)?,
        chat_throughput_policy,
        chat_model_selection,
        chat_generation_gguf,
        qwen_embedding_gguf: qwen_embedding_gguf_proof,
        qwen_embedding_onnx: qwen_embedding_onnx_proof,
        qwen_reranker_onnx: qwen_reranker_onnx_proof,
        observations,
    };

    write_proof(&proof_out, &proof)?;
    return Ok(());

    fn write_proof(
        proof_out: &Path,
        proof: &RuntimeProbeProof,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = proof_out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(proof_out, serde_json::to_string_pretty(&proof)?)?;
        let proof_path = format!("{}\n", proof_out.display());
        std::io::stdout().write_all(proof_path.as_bytes())?;
        Ok(())
    }

    fn service_config_proof(
        repo_root: &Path,
        service_config: &ModelRuntimeServiceConfig,
    ) -> Result<serde_json::Value, serde_json::Error> {
        let mut value = serde_json::to_value(service_config)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "cacheRoot".to_owned(),
                serde_json::json!(repo_relative_display(repo_root, &service_config.cache_root)),
            );
        }
        Ok(value)
    }

    fn cache_root_policy_proof(
        repo_root: &Path,
        cache_root_policy: &crate::model_runtime::ModelCacheRootPolicy,
    ) -> Result<serde_json::Value, serde_json::Error> {
        let mut value = serde_json::to_value(cache_root_policy)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "root".to_owned(),
                serde_json::json!(repo_relative_display(repo_root, &cache_root_policy.root)),
            );
        }
        Ok(value)
    }

    fn repo_relative_display(repo_root: &Path, path: &Path) -> String {
        path.strip_prefix(repo_root)
            .map(|relative| format!("<repo>/{}", normalize_display_path(relative)))
            .unwrap_or_else(|_| {
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "path-redacted".to_owned());
                format!("<external>/{file_name}")
            })
    }

    fn normalize_display_path(path: &Path) -> String {
        path.display().to_string().replace('\\', "/")
    }

    fn cache_only_probe(
        repo_root: &Path,
        spec: &HfModelSpec,
        cache_root: &Path,
        operation: &str,
    ) -> serde_json::Value {
        let mode = CacheProofMode {
            cache_only: true,
            download_enabled: false,
            network_may_be_attempted: false,
        };
        match resolve_cache_without_network(spec, cache_root) {
            Ok(report) => cache_report_json(repo_root, &report, operation, mode),
            Err(error) => cache_error_json(repo_root, operation, error, mode),
        }
    }

    fn cache_or_download_probe(
        repo_root: &Path,
        spec: &HfModelSpec,
        cache_root: &Path,
        allow_network: bool,
        operation: &str,
    ) -> serde_json::Value {
        let mode = CacheProofMode {
            cache_only: false,
            download_enabled: allow_network,
            network_may_be_attempted: allow_network,
        };
        match maybe_download(spec, cache_root, allow_network) {
            Ok(report) => cache_report_json(repo_root, &report, operation, mode),
            Err(error) => cache_error_json(repo_root, operation, error, mode),
        }
    }

    fn cache_report_json(
        repo_root: &Path,
        report: &HfDownloadReport,
        operation: &str,
        mode: CacheProofMode,
    ) -> serde_json::Value {
        serde_json::json!({
            "operation": operation,
            "ok": true,
            "cacheOnly": mode.cache_only,
            "downloadEnabled": mode.download_enabled,
            "networkMayBeAttempted": mode.network_may_be_attempted,
            "strictCacheHash": env_truthy("ENFORCER_X06_STRICT_CACHE_HASH"),
            "repoId": report.repo_id,
            "revision": report.revision,
            "manifestPath": repo_relative_display(repo_root, &report.manifest_path),
            "cacheDir": repo_relative_display(repo_root, &report.cache_dir),
            "downloadedFiles": hf_downloaded_files_proof(repo_root, &report.downloaded_files)
        })
    }

    fn cache_error_json(
        repo_root: &Path,
        operation: &str,
        error: impl std::fmt::Display,
        mode: CacheProofMode,
    ) -> serde_json::Value {
        serde_json::json!({
            "operation": operation,
            "ok": false,
            "cacheOnly": mode.cache_only,
            "downloadEnabled": mode.download_enabled,
            "networkMayBeAttempted": mode.network_may_be_attempted,
            "strictCacheHash": env_truthy("ENFORCER_X06_STRICT_CACHE_HASH"),
            "reason": repo_path_redacted_text(repo_root, &error.to_string())
        })
    }

    fn hf_downloaded_files_proof(
        repo_root: &Path,
        files: &[HfDownloadedFile],
    ) -> Vec<serde_json::Value> {
        files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "sourcePath": file.source_path,
                    "localPath": repo_relative_display(repo_root, &file.local_path),
                    "sha256": file.sha256,
                    "sizeBytes": file.size_bytes,
                    "streamingManifestPath": file.streaming_manifest_path.as_ref().map(|path| {
                        repo_relative_display(repo_root, path)
                    })
                })
            })
            .collect()
    }

    fn maybe_download(
        spec: &HfModelSpec,
        cache_root: &Path,
        allow_network: bool,
    ) -> Result<HfDownloadReport, String> {
        match resolve_cache_without_network(spec, cache_root) {
            Ok(report) => return Ok(report),
            Err(cache_error) if !allow_network => {
                return Err(format!(
                    "{cache_error}; ENFORCER_X06_ALLOW_NETWORK is not enabled"
                ))
            }
            Err(_) => {}
        }
        if allow_network {
            download_hf_model(spec, cache_root, None).map_err(|error| error.to_string())
        } else {
            Err(
                "ENFORCER_X06_ALLOW_NETWORK is not enabled; explicit proof download/cache disabled"
                    .to_owned(),
            )
        }
    }

    fn resolve_cache_without_network(
        spec: &HfModelSpec,
        cache_root: &Path,
    ) -> Result<HfDownloadReport, String> {
        let cached = if env_truthy("ENFORCER_X06_STRICT_CACHE_HASH") {
            resolve_cached_hf_model(spec, cache_root)
        } else {
            resolve_cached_hf_model_from_manifest(spec, cache_root)
        };
        cached.map_err(|error| error.to_string())
    }

    fn maybe_direct_chat_model_report(cache_root: &Path) -> Option<HfDownloadReport> {
        let model_path = std::env::var("ENFORCER_X06_CHAT_MODEL_PATH")
            .ok()
            .map(PathBuf::from)
            .filter(|path| path.is_file())?;
        let file_name = model_path.file_name()?.to_string_lossy().to_string();
        let metadata = std::fs::metadata(&model_path).ok()?;
        let sha256 = sha256_file(&model_path).ok()?;
        let model_id = std::env::var("ENFORCER_X06_CHAT_MODEL_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "local/direct-chat-gguf".to_owned());
        let revision = std::env::var("ENFORCER_X06_CHAT_MODEL_REVISION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "local".to_owned());
        let cache_dir = cache_root.join("local").join("chat");
        Some(HfDownloadReport {
            repo_id: model_id,
            revision,
            cache_dir: cache_dir.clone(),
            manifest_path: cache_dir.join("direct-chat-model.manifest.json"),
            downloaded_files: vec![HfDownloadedFile {
                source_path: file_name,
                local_path: model_path,
                sha256,
                size_bytes: metadata.len(),
                streaming_manifest_path: None,
            }],
        })
    }

    fn maybe_llama_device_report(
        allow_device_probe: bool,
    ) -> Option<crate::llama_cpp::LlamaCppDeviceReport> {
        if !allow_device_probe {
            return None;
        }
        let binary = env_path("ENFORCER_X06_LLAMA_CLI").or_else(default_llama_cli)?;
        list_llama_cpp_devices(
            &binary,
            env_u64("ENFORCER_X06_DEVICE_TIMEOUT_MS")
                .unwrap_or_else(|| default_model_runtime_probe_plan().provider_probe_timeout_ms),
        )
        .ok()
    }

    fn maybe_select_chat_model_for_hardware(
        lineup: &mut X06ModelLineup,
        context: ChatSelectionContext<'_>,
    ) -> serde_json::Value {
        if !context.chat_probe_selected {
            return serde_json::json!({
                "enabled": false,
                "reason": "chat-generation-gguf not selected by ENFORCER_X06_PROBE_FILTER",
                "selected": lineup.chat_generation
            });
        }
        if !env_default_truthy("ENFORCER_X06_AUTO_CHAT_MODEL", true) {
            return serde_json::json!({
                "enabled": false,
                "reason": "ENFORCER_X06_AUTO_CHAT_MODEL disabled",
                "selected": lineup.chat_generation
            });
        }
        if chat_model_override_present() {
            return serde_json::json!({
                "enabled": false,
                "reason": "explicit ENFORCER_X06_CHAT_MODEL_* override present",
                "selected": lineup.chat_generation
            });
        }

        let provider_probe_passed = context.execution.provider_probe_passed;
        let detected_free_vram_mib = context.execution.detected_free_vram_mib;
        let selection = select_x06_chat_model_for_hardware(detected_free_vram_mib);
        lineup.chat_generation = selection.selected.clone();
        serde_json::json!({
            "enabled": true,
            "requestedBackendHint": context.requested_backend_hint,
            "backendHint": context.execution.backend_hint,
            "requestedAcceleration": context.execution.requested_acceleration,
            "resolvedAcceleration": context.execution.resolved_acceleration,
            "providerProbePassed": provider_probe_passed,
            "selectedDeviceId": context.execution.selected_device_id.clone(),
            "selectedMainGpu": context.execution.selected_main_gpu,
            "downgradeReason": context.execution.downgrade_reason.clone(),
            "providerProbeTimeoutMs": env_u64("ENFORCER_X06_DEVICE_TIMEOUT_MS")
                .unwrap_or_else(|| default_model_runtime_probe_plan().provider_probe_timeout_ms),
            "deviceReport": context.device_report.map(|report| {
                llama_device_report_proof(context.repo_root, report)
            }),
            "selection": selection
        })
    }

    fn llama_device_report_proof(
        repo_root: &Path,
        report: &crate::llama_cpp::LlamaCppDeviceReport,
    ) -> serde_json::Value {
        let mut value = serde_json::to_value(report).unwrap_or_else(|error| {
            serde_json::json!({
                "serializationError": error.to_string()
            })
        });
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "binaryPath".to_owned(),
                serde_json::json!(repo_relative_display(repo_root, &report.binary_path)),
            );
            object.insert(
                "stderrExcerpt".to_owned(),
                serde_json::json!(repo_path_redacted_text(repo_root, &report.stderr_excerpt)),
            );
        }
        value
    }

    fn run_llama_generation(
        report: &HfDownloadReport,
        context: &LlamaRunContext<'_>,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
    ) -> serde_json::Value {
        let Some(model) = first_model_file(report) else {
            return proof_error("chat-generation-model", "no GGUF model file in report");
        };
        let binary = env_path("ENFORCER_X06_LLAMA_CLI").or_else(default_llama_cli);
        let Some(binary) = binary else {
            return proof_error(
                "chat-generation-llama-cli",
                "llama-cli executable not configured/found",
            );
        };
        let result = run_llama_cpp_probe(&LlamaCppProbeConfig {
            binary_path: binary,
            model_path: model.local_path.clone(),
            model_sha256: strict_model_hash(&model.sha256),
            prompt: "Say hello from the local chat model in one short sentence.".to_owned(),
            kind: LlamaCppProbeKind::Generate,
            backend_hint: context.backend_hint,
            acceleration: context.acceleration,
            gpu_layers: env_usize("ENFORCER_X06_LLAMA_GPU_LAYERS"),
            device: std::env::var("ENFORCER_X06_LLAMA_DEVICE")
                .ok()
                .or_else(|| context.selected_device_id.clone()),
            main_gpu: env_usize("ENFORCER_X06_LLAMA_MAIN_GPU").or(context.selected_main_gpu),
            split_mode: std::env::var("ENFORCER_X06_LLAMA_SPLIT_MODE")
                .ok()
                .or_else(|| context.default_split_mode.clone()),
            tensor_split: std::env::var("ENFORCER_X06_LLAMA_TENSOR_SPLIT").ok(),
            fit: env_optional_bool("ENFORCER_X06_LLAMA_FIT").or(Some(true)),
            context_size: env_usize("ENFORCER_X06_LLAMA_CONTEXT"),
            max_tokens: env_usize("ENFORCER_X06_LLAMA_MAX_TOKENS").unwrap_or(32),
            timeout_ms: env_u64("ENFORCER_X06_LLAMA_TIMEOUT_MS")
                .unwrap_or_else(|| default_model_runtime_probe_plan().model_probe_timeout_ms),
        });
        llama_result_json(
            LlamaResultInput {
                operation: "chat-generation-gguf",
                result,
                model_id: report.repo_id.clone(),
                task: ModelTask::Summarization,
                provider: provider_kind_for_llama(context.backend_hint, context.acceleration),
                observed_at: context.observed_at,
                run_id: context.run_id,
                repo_root: context.repo_root,
            },
            observations,
        )
    }

    fn run_llama_embedding(
        report: &HfDownloadReport,
        context: &LlamaRunContext<'_>,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
    ) -> serde_json::Value {
        let Some(model) = first_model_file(report) else {
            return proof_error("qwen-embedding-gguf-model", "no GGUF model file in report");
        };
        let config = LlamaCppProbeConfig {
            binary_path: env_path("ENFORCER_X06_LLAMA_EMBEDDING")
                .or_else(default_llama_embedding)
                .unwrap_or_else(|| PathBuf::from(llama_binary_name("llama-embedding"))),
            model_path: model.local_path.clone(),
            model_sha256: strict_model_hash(&model.sha256),
            prompt: "embedding hello world for x06 memory retrieval".to_owned(),
            kind: LlamaCppProbeKind::Embedding,
            backend_hint: context.backend_hint,
            acceleration: context.acceleration,
            gpu_layers: env_usize("ENFORCER_X06_LLAMA_GPU_LAYERS"),
            device: std::env::var("ENFORCER_X06_LLAMA_DEVICE")
                .ok()
                .or_else(|| context.selected_device_id.clone()),
            main_gpu: env_usize("ENFORCER_X06_LLAMA_MAIN_GPU").or(context.selected_main_gpu),
            split_mode: std::env::var("ENFORCER_X06_LLAMA_SPLIT_MODE")
                .ok()
                .or_else(|| context.default_split_mode.clone()),
            tensor_split: std::env::var("ENFORCER_X06_LLAMA_TENSOR_SPLIT").ok(),
            fit: env_optional_bool("ENFORCER_X06_LLAMA_FIT").or(Some(true)),
            context_size: env_usize("ENFORCER_X06_LLAMA_CONTEXT"),
            max_tokens: 0,
            timeout_ms: env_u64("ENFORCER_X06_LLAMA_TIMEOUT_MS")
                .unwrap_or_else(|| default_model_runtime_probe_plan().model_probe_timeout_ms),
        };
        let result = if config.binary_path.is_file() {
            run_llama_cpp_probe(&config)
        } else {
            Err(crate::error::MemoryError::ModelRuntime {
                operation: "qwen-embedding-gguf-llama-embedding",
                reason: format!(
                    "llama-embedding executable not configured/found at {}; Enforcer does not fall back to llama-server for X06 GGUF embedding proof",
                    config.binary_path.display()
                ),
            })
        };
        llama_result_json(
            LlamaResultInput {
                operation: "qwen-embedding-gguf",
                result,
                model_id: report.repo_id.clone(),
                task: ModelTask::Embedding,
                provider: provider_kind_for_llama(context.backend_hint, context.acceleration),
                observed_at: context.observed_at,
                run_id: context.run_id,
                repo_root: context.repo_root,
            },
            observations,
        )
    }

    fn llama_result_json(
        input: LlamaResultInput<'_>,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
    ) -> serde_json::Value {
        match input.result {
            Ok(report) => {
                let usability = model_usability(input.task, &report);
                if usability.ok {
                    observations.push(success_observation(
                        input.observed_at,
                        input.run_id,
                        input.model_id,
                        input.task,
                        input.provider,
                    ));
                } else {
                    observations.push(load_failure_observation(LoadFailureInput {
                        observed_at: input.observed_at,
                        run_id: input.run_id,
                        model_id: input.model_id,
                        task: input.task,
                        requested_provider: Some(input.provider),
                        failure_reason: usability.reason.clone(),
                    }));
                }
                serde_json::json!({
                    "operation": input.operation,
                    "ok": usability.ok,
                    "loaded": report.loaded(),
                    "usability": usability,
                    "report": llama_report_proof(input.repo_root, &report)
                })
            }
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at: input.observed_at,
                    run_id: input.run_id,
                    model_id: input.model_id,
                    task: input.task,
                    requested_provider: Some(input.provider),
                    failure_reason: error.to_string(),
                }));
                proof_error(input.operation, error)
            }
        }
    }

    fn proof_scope(runtime_mode: &str) -> serde_json::Value {
        let platform = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match runtime_mode {
            "plan" => serde_json::json!({
                "portability": "portable-contract",
                "capability": "ci",
                "ciParity": false,
                "localHardwareRequired": false,
                "platform": "any",
                "arch": "any",
                "reason": "plan mode proves zero-network contract shape only, not real model parity"
            }),
            "download" => serde_json::json!({
                "portability": "cache-artifact-proof",
                "capability": "network-local",
                "ciParity": false,
                "localHardwareRequired": false,
                "platform": platform,
                "arch": arch,
                "reason": "download mode proves explicit cache acquisition and integrity metadata; default CI must not download models"
            }),
            "cache-only" => serde_json::json!({
                "portability": "portable-cache-contract",
                "capability": "ci-cache",
                "ciParity": false,
                "localHardwareRequired": false,
                "platform": "any",
                "arch": "any",
                "reason": "cache-only mode proves deterministic local cache resolution without network, model loading, or hardware probes"
            }),
            _ => serde_json::json!({
                "portability": "local-runtime-proof",
                "capability": format!("{platform}-{arch}"),
                "ciParity": false,
                "localHardwareRequired": true,
                "platform": platform,
                "arch": arch,
                "reason": "probe mode proves this host runtime only; CI parity requires its own platform proof or zero-network degraded contract"
            }),
        }
    }

    fn llama_report_proof(
        repo_root: &Path,
        report: &crate::llama_cpp::LlamaCppProbeReport,
    ) -> serde_json::Value {
        let mut value = serde_json::to_value(report).unwrap_or_else(|error| {
            serde_json::json!({
                "serializationError": error.to_string()
            })
        });
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "binaryPath".to_owned(),
                serde_json::json!(repo_relative_display(repo_root, &report.binary_path)),
            );
            object.insert(
                "modelPath".to_owned(),
                serde_json::json!(repo_relative_display(repo_root, &report.model_path)),
            );
            object.insert(
                "fallbackFromBinaryPath".to_owned(),
                match report.fallback_from_binary_path.as_ref() {
                    Some(path) => serde_json::json!(repo_relative_display(repo_root, path)),
                    None => serde_json::Value::Null,
                },
            );
            object.insert(
                "stdoutExcerpt".to_owned(),
                serde_json::json!(repo_path_redacted_text(repo_root, &report.stdout_excerpt)),
            );
            object.insert(
                "stderrExcerpt".to_owned(),
                serde_json::json!(repo_path_redacted_text(repo_root, &report.stderr_excerpt)),
            );
        }
        value
    }

    fn repo_path_redacted_text(repo_root: &Path, text: &str) -> String {
        let native = repo_root.display().to_string();
        let slash = native.replace('\\', "/");
        let doubled_slash = slash.replace('/', "//");
        text.replace(&doubled_slash, "<repo>")
            .replace(&native, "<repo>")
            .replace(&slash, "<repo>")
            .replace('\\', "/")
            .replace("<repo>//", "<repo>/")
    }

    fn model_usability(
        task: ModelTask,
        report: &crate::llama_cpp::LlamaCppProbeReport,
    ) -> ModelUsabilityReport {
        if task == ModelTask::Embedding {
            return embedding_usability(report, EXPECTED_QWEN_EMBEDDING_DIMENSIONS);
        }
        if task != ModelTask::Summarization {
            return loaded_non_chat_usability(
                report.loaded(),
                report.measured_tokens_per_second,
                report.stderr_excerpt.clone(),
            );
        }
        let policy = ChatThroughputPolicy {
            ..chat_throughput_policy_from_env()
        };
        evaluate_chat_usability(
            report.loaded(),
            report.measured_tokens_per_second,
            report.stderr_excerpt.clone(),
            policy,
        )
    }

    fn embedding_usability(
        report: &crate::llama_cpp::LlamaCppProbeReport,
        expected_dimensions: usize,
    ) -> ModelUsabilityReport {
        if !report.loaded() {
            return loaded_non_chat_usability(
                false,
                report.measured_tokens_per_second,
                report.stderr_excerpt.clone(),
            );
        }
        match report.output_dimensions {
            Some(actual) if actual == expected_dimensions => ModelUsabilityReport {
                ok: true,
                reason: format!(
                    "embedding usable: dimensions {actual} matched expected {expected_dimensions}"
                ),
                min_chat_tokens_per_second: None,
                target_chat_tokens_per_second_low: None,
                target_chat_tokens_per_second_high: None,
                measured_tokens_per_second: report.measured_tokens_per_second,
            },
            Some(actual) => ModelUsabilityReport {
                ok: false,
                reason: format!(
                    "embedding dimensions mismatch: expected {expected_dimensions}, got {actual}"
                ),
                min_chat_tokens_per_second: None,
                target_chat_tokens_per_second_low: None,
                target_chat_tokens_per_second_high: None,
                measured_tokens_per_second: report.measured_tokens_per_second,
            },
            None => ModelUsabilityReport {
                ok: false,
                reason: format!("embedding dimensions missing; expected {expected_dimensions}"),
                min_chat_tokens_per_second: None,
                target_chat_tokens_per_second_low: None,
                target_chat_tokens_per_second_high: None,
                measured_tokens_per_second: report.measured_tokens_per_second,
            },
        }
    }

    fn run_ort_embedding(
        report: &HfDownloadReport,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
        observed_at: &str,
        run_id: &str,
    ) -> serde_json::Value {
        let spec = match onnx_spec(
            report,
            DEFAULT_EMBEDDING_MODEL_ID,
            ModelTask::Embedding,
            1024,
        ) {
            Ok(spec) => spec,
            Err(error) => return proof_error("qwen-embedding-onnx-spec", error),
        };
        let timeout_ms = env_u64("ENFORCER_X06_ORT_TIMEOUT_MS").unwrap_or(30_000);
        match run_ort_child_probe("embedding", &spec, timeout_ms) {
            Ok(proof) if proof.get("ok").and_then(|value| value.as_bool()) == Some(true) => {
                observations.push(success_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Embedding,
                    ProviderKind::Cpu,
                ));
                proof
            }
            Ok(proof) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at,
                    run_id,
                    model_id: spec.model_id,
                    task: ModelTask::Embedding,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: proof.to_string(),
                }));
                proof
            }
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at,
                    run_id,
                    model_id: spec.model_id,
                    task: ModelTask::Embedding,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: error.to_string(),
                }));
                proof_error("qwen-embedding-onnx", error)
            }
        }
    }

    fn run_ort_reranker(
        report: &HfDownloadReport,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
        observed_at: &str,
        run_id: &str,
    ) -> serde_json::Value {
        let spec = match onnx_spec(report, DEFAULT_RERANKER_MODEL_ID, ModelTask::Reranking, 1) {
            Ok(spec) => spec,
            Err(error) => return proof_error("qwen-reranker-onnx-spec", error),
        };
        let timeout_ms = env_u64("ENFORCER_X06_ORT_TIMEOUT_MS").unwrap_or(30_000);
        match run_ort_child_probe("reranker", &spec, timeout_ms) {
            Ok(proof) if proof.get("ok").and_then(|value| value.as_bool()) == Some(true) => {
                observations.push(success_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Reranking,
                    ProviderKind::Cpu,
                ));
                proof
            }
            Ok(proof) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at,
                    run_id,
                    model_id: spec.model_id,
                    task: ModelTask::Reranking,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: proof.to_string(),
                }));
                proof
            }
            Err(error) => {
                observations.push(load_failure_observation(LoadFailureInput {
                    observed_at,
                    run_id,
                    model_id: spec.model_id,
                    task: ModelTask::Reranking,
                    requested_provider: Some(ProviderKind::Cpu),
                    failure_reason: error.to_string(),
                }));
                proof_error("qwen-reranker-onnx", error)
            }
        }
    }

    fn run_ort_child_probe(
        child_task: &str,
        spec: &ModelSpec,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, String> {
        let started = Instant::now();
        let mut child = Command::new(std::env::current_exe().map_err(|error| error.to_string())?)
            .env("ENFORCER_X06_ORT_CHILD_TASK", child_task)
            .env("ENFORCER_X06_CHILD_MODEL_ID", &spec.model_id)
            .env("ENFORCER_X06_CHILD_REVISION", &spec.revision)
            .env("ENFORCER_X06_CHILD_ARTIFACT_PATH", &spec.artifact_path)
            .env("ENFORCER_X06_CHILD_ARTIFACT_SHA256", &spec.artifact_sha256)
            .env("ENFORCER_X06_CHILD_TOKENIZER_PATH", &spec.tokenizer_path)
            .env(
                "ENFORCER_X06_CHILD_TOKENIZER_SHA256",
                &spec.tokenizer_sha256,
            )
            .env("ENFORCER_X06_CHILD_DTYPE", &spec.dtype)
            .env("ENFORCER_X06_CHILD_DIMENSION", spec.dimension.to_string())
            .env("ENFORCER_X06_CHILD_TASK", format!("{:?}", spec.task))
            .env("ENFORCER_X06_ORT_TIMEOUT_MS", timeout_ms.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;
        let timeout = Duration::from_millis(timeout_ms);
        let mut timed_out = false;
        loop {
            if child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                break;
            }
            if started.elapsed() >= timeout {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let output = child
            .wait_with_output()
            .map_err(|error| error.to_string())?;
        if timed_out {
            return Ok(serde_json::json!({
                "operation": format!("qwen-{child_task}-onnx"),
                "ok": false,
                "timedOut": true,
                "timeoutMs": timeout_ms,
                "error": "ORT child probe timed out during load or inference"
            }));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            return Ok(serde_json::json!({
                "operation": format!("qwen-{child_task}-onnx"),
                "ok": false,
                "exitCode": output.status.code(),
                "stderr": excerpt_tail(&String::from_utf8_lossy(&output.stderr), 4096),
                "stdout": excerpt_tail(&stdout, 4096)
            }));
        }
        serde_json::from_str(stdout.trim()).map_err(|error| error.to_string())
    }

    fn run_ort_child(child_task: &str) -> serde_json::Value {
        let timeout =
            Duration::from_millis(env_u64("ENFORCER_X06_ORT_TIMEOUT_MS").unwrap_or(30_000));
        let spec = match child_model_spec() {
            Ok(spec) => spec,
            Err(error) => {
                return serde_json::json!({
                    "operation": format!("qwen-{child_task}-onnx"),
                    "ok": false,
                    "error": error
                });
            }
        };
        match child_task {
            "embedding" => match OrtEmbedder::load(&spec, ProviderKind::Cpu).and_then(|embedder| {
                embedder.embed_with_timeout("hello world from qwen3 embedding 0.6b", timeout)
            }) {
                Ok(vector) => serde_json::json!({
                    "operation": "qwen-embedding-onnx",
                    "ok": true,
                    "dimension": vector.len(),
                    "head": vector.iter().take(8).copied().collect::<Vec<_>>()
                }),
                Err(error) => serde_json::json!({
                    "operation": "qwen-embedding-onnx",
                    "ok": false,
                    "error": error.to_string()
                }),
            },
            "reranker" => {
                let candidates = vec![
                    RankedHit {
                        doc_id: "irrelevant".to_owned(),
                        kind: DocumentKind::Function,
                        snippet: "unrelated socket timeout retry".to_owned(),
                        source_path: None,
                        score: 0.0,
                    },
                    RankedHit {
                        doc_id: "relevant".to_owned(),
                        kind: DocumentKind::Function,
                        snippet: "model runtime cache loads qwen embeddings".to_owned(),
                        source_path: None,
                        score: 0.0,
                    },
                ];
                match OrtReranker::load(&spec, ProviderKind::Cpu).and_then(|reranker| {
                    reranker.rerank_with_timeout("qwen model runtime cache", &candidates, timeout)
                }) {
                    Ok(ranked) => serde_json::json!({
                        "operation": "qwen-reranker-onnx",
                        "ok": true,
                        "ranked": ranked.iter().map(|hit| serde_json::json!({
                            "docId": hit.doc_id,
                            "score": hit.score
                        })).collect::<Vec<_>>()
                    }),
                    Err(error) => serde_json::json!({
                        "operation": "qwen-reranker-onnx",
                        "ok": false,
                        "error": error.to_string()
                    }),
                }
            }
            _ => serde_json::json!({
                "operation": format!("qwen-{child_task}-onnx"),
                "ok": false,
                "error": "unknown ORT child task"
            }),
        }
    }

    fn child_model_spec() -> Result<ModelSpec, String> {
        Ok(ModelSpec {
            model_id: required_env("ENFORCER_X06_CHILD_MODEL_ID")?,
            revision: required_env("ENFORCER_X06_CHILD_REVISION")?,
            artifact_path: PathBuf::from(required_env("ENFORCER_X06_CHILD_ARTIFACT_PATH")?),
            artifact_sha256: required_env("ENFORCER_X06_CHILD_ARTIFACT_SHA256")?,
            tokenizer_path: PathBuf::from(required_env("ENFORCER_X06_CHILD_TOKENIZER_PATH")?),
            tokenizer_sha256: required_env("ENFORCER_X06_CHILD_TOKENIZER_SHA256")?,
            dtype: required_env("ENFORCER_X06_CHILD_DTYPE")?,
            dimension: required_env("ENFORCER_X06_CHILD_DIMENSION")?
                .parse::<usize>()
                .map_err(|error| error.to_string())?,
            task: match required_env("ENFORCER_X06_CHILD_TASK")?.as_str() {
                "Embedding" => ModelTask::Embedding,
                "Reranking" => ModelTask::Reranking,
                "Summarization" => ModelTask::Summarization,
                task => return Err(format!("unknown model task: {task}")),
            },
        })
    }

    fn required_env(name: &str) -> Result<String, String> {
        std::env::var(name).map_err(|error| format!("missing required env {name}: {error}"))
    }

    fn onnx_spec(
        report: &HfDownloadReport,
        model_id: &str,
        task: ModelTask,
        dimension: usize,
    ) -> Result<ModelSpec, String> {
        let model = report
            .downloaded_files
            .iter()
            .find(|file| file.source_path.ends_with(".onnx"))
            .ok_or_else(|| "missing ONNX model file".to_owned())?;
        let tokenizer = report
            .downloaded_files
            .iter()
            .find(|file| file.source_path == "tokenizer.json")
            .ok_or_else(|| "missing tokenizer.json".to_owned())?;
        let mut spec = match task {
            ModelTask::Embedding => ModelSpec::qwen3_embedding(
                &model.local_path,
                &model.sha256,
                &tokenizer.local_path,
                &tokenizer.sha256,
            ),
            ModelTask::Reranking => ModelSpec::qwen3_reranker(
                &model.local_path,
                &model.sha256,
                &tokenizer.local_path,
                &tokenizer.sha256,
            ),
            ModelTask::Summarization => {
                return Err("ONNX spec does not support summarization".to_owned());
            }
        };
        spec.model_id = model_id.to_owned();
        spec.dimension = dimension;
        Ok(spec)
    }

    fn first_model_file(report: &HfDownloadReport) -> Option<&crate::hf_cache::HfDownloadedFile> {
        report
            .downloaded_files
            .iter()
            .find(|file| file.source_path.ends_with(".gguf"))
    }

    fn env_truthy(name: &str) -> bool {
        std::env::var(name)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false)
    }

    fn runtime_mode_from_env() -> String {
        match std::env::var("ENFORCER_X06_RUNTIME_MODE")
            .unwrap_or_else(|_| "probe".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "plan" => "plan".to_owned(),
            "download" => "download".to_owned(),
            "cache-only" | "cache_only" | "cacheonly" => "cache-only".to_owned(),
            _ => "probe".to_owned(),
        }
    }

    fn cache_root_mode_from_env() -> ModelCacheRootMode {
        match std::env::var("ENFORCER_X06_MODEL_CACHE_MODE")
            .unwrap_or_else(|_| "dev".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "app-data" | "appdata" | "prod" | "production" => ModelCacheRootMode::AppData,
            _ => ModelCacheRootMode::DevRepoLocal,
        }
    }

    fn env_default_truthy(name: &str, default: bool) -> bool {
        std::env::var(name)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(default)
    }

    fn env_optional_bool(name: &str) -> Option<bool> {
        std::env::var(name).ok().map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    }

    fn chat_model_override_present() -> bool {
        [
            "ENFORCER_X06_CHAT_MODEL_REPO",
            "ENFORCER_X06_CHAT_MODEL_FILE",
            "ENFORCER_X06_CHAT_MODEL_ID",
            "ENFORCER_X06_CHAT_MODEL_REVISION",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    }

    fn llama_acceleration_from_env() -> LocalRuntimeAcceleration {
        match std::env::var("ENFORCER_X06_LLAMA_ACCELERATION")
            .unwrap_or_else(|_| "cpu".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "cpu" => LocalRuntimeAcceleration::Cpu,
            "gpu" => LocalRuntimeAcceleration::Gpu,
            "npu" => LocalRuntimeAcceleration::Npu,
            _ => LocalRuntimeAcceleration::Auto,
        }
    }

    fn llama_backend_hint_from_env() -> LlamaCppBackendHint {
        match std::env::var("ENFORCER_X06_LLAMA_BACKEND")
            .unwrap_or_else(|_| "auto".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "native" => LlamaCppBackendHint::Native,
            "vulkan" => LlamaCppBackendHint::Vulkan,
            "openvino" => LlamaCppBackendHint::OpenVino,
            _ => LlamaCppBackendHint::Auto,
        }
    }

    fn chat_throughput_policy_from_env() -> ChatThroughputPolicy {
        ChatThroughputPolicy {
            min_tokens_per_second: env_f64("ENFORCER_X06_MIN_CHAT_TOKENS_PER_SECOND")
                .unwrap_or(DEFAULT_MIN_CHAT_TOKENS_PER_SECOND),
            target_tokens_per_second_low: env_f64("ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_LOW")
                .unwrap_or(TARGET_CHAT_TOKENS_PER_SECOND_LOW),
            target_tokens_per_second_high: env_f64(
                "ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_HIGH",
            )
            .unwrap_or(TARGET_CHAT_TOKENS_PER_SECOND_HIGH),
        }
    }

    fn env_usize(name: &str) -> Option<usize> {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
    }

    fn env_u64(name: &str) -> Option<u64> {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
    }

    fn env_f64(name: &str) -> Option<f64> {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
    }

    fn strict_model_hash(sha256: &str) -> Option<String> {
        env_truthy("ENFORCER_X06_STRICT_CACHE_HASH").then(|| sha256.to_owned())
    }

    fn provider_kind_for_llama(
        backend_hint: LlamaCppBackendHint,
        acceleration: LocalRuntimeAcceleration,
    ) -> ProviderKind {
        match (backend_hint, acceleration) {
            (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Cpu)
            | (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Gpu) => {
                ProviderKind::OpenVino
            }
            (LlamaCppBackendHint::Vulkan, LocalRuntimeAcceleration::Gpu) => ProviderKind::Vulkan,
            (_, LocalRuntimeAcceleration::Npu) => ProviderKind::Npu,
            (_, LocalRuntimeAcceleration::Gpu) => ProviderKind::Cuda,
            (_, LocalRuntimeAcceleration::Auto | LocalRuntimeAcceleration::Cpu) => {
                ProviderKind::Cpu
            }
        }
    }

    fn default_split_mode_for_execution(execution: &LlamaCppExecutionResolution) -> Option<String> {
        (execution.resolved_acceleration == LocalRuntimeAcceleration::Gpu)
            .then(|| "layer".to_owned())
    }

    fn provider_downgrade_observation(
        execution: &LlamaCppExecutionResolution,
        model_id: &str,
        task: ModelTask,
        observed_at: &str,
        run_id: &str,
    ) -> Option<ModelRuntimeObservationRecord> {
        let reason = execution.downgrade_reason.clone()?;
        let requested_provider = provider_kind_for_llama(
            execution.requested_backend_hint,
            execution.requested_acceleration,
        );
        let fallback_provider =
            provider_kind_for_llama(execution.backend_hint, execution.resolved_acceleration);
        Some(ModelRuntimeObservationRecord::new(
            observed_at,
            "x06-model-runtime-probe",
            run_id,
            ModelRuntimeObservationCandidate::ProviderDowngrade(ProviderDowngrade {
                model_id: model_id.to_owned(),
                task,
                requested_provider,
                fallback_provider,
                reason,
            }),
        ))
    }

    fn env_path(name: &str) -> Option<PathBuf> {
        std::env::var(name)
            .ok()
            .map(PathBuf::from)
            .filter(|path| path.is_file())
    }

    const DEFAULT_PROBE_FILTER: &str = "chat";
    const PROBE_ORDER: &[&str] = &[
        "chat-generation-gguf",
        "qwen-embedding-onnx",
        "qwen-reranker-onnx",
        "qwen-embedding-gguf",
    ];

    fn probe_filter_from_env() -> Vec<String> {
        let raw_filter = std::env::var("ENFORCER_X06_PROBE_FILTER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PROBE_FILTER.to_owned());
        let requested = raw_filter
            .split(',')
            .map(|entry| entry.trim().to_ascii_lowercase())
            .filter(|entry| !entry.is_empty())
            .collect::<Vec<_>>();
        let mut expanded = expand_probe_filter(&requested);
        if expanded.is_empty() {
            expanded = expand_probe_filter(&[DEFAULT_PROBE_FILTER.to_owned()]);
        }
        if !env_truthy("ENFORCER_X06_ALLOW_MULTI_PROBE") && expanded.len() > 1 {
            expanded.truncate(1);
        }
        expanded
    }

    fn expand_probe_filter(requested: &[String]) -> Vec<String> {
        let mut probes = Vec::new();
        for entry in requested {
            match entry.as_str() {
                "all" => {
                    for probe in PROBE_ORDER {
                        push_unique_probe(&mut probes, probe);
                    }
                }
                "chat" | "chat-generation" | "chat-generation-gguf" => {
                    push_unique_probe(&mut probes, "chat-generation-gguf");
                }
                "embedding" | "embedding-onnx" | "qwen-embedding-onnx" => {
                    push_unique_probe(&mut probes, "qwen-embedding-onnx");
                }
                "embedding-gguf" | "qwen-embedding-gguf" => {
                    push_unique_probe(&mut probes, "qwen-embedding-gguf");
                }
                "reranker" | "ranker" | "reranker-onnx" | "qwen-reranker-onnx" => {
                    push_unique_probe(&mut probes, "qwen-reranker-onnx");
                }
                probe if PROBE_ORDER.contains(&probe) => push_unique_probe(&mut probes, probe),
                _ => {}
            }
        }
        probes.sort_by_key(|probe| {
            PROBE_ORDER
                .iter()
                .position(|known| known == probe)
                .unwrap_or(PROBE_ORDER.len())
        });
        probes
    }

    fn push_unique_probe(probes: &mut Vec<String>, probe: &str) {
        if !probes.iter().any(|existing| existing == probe) {
            probes.push(probe.to_owned());
        }
    }

    fn should_run_probe(filter: &[String], probe: &str) -> bool {
        filter.iter().any(|entry| entry == probe)
    }

    fn probe_execution_policy(selected_probes: &[String]) -> serde_json::Value {
        let requested_filter = std::env::var("ENFORCER_X06_PROBE_FILTER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PROBE_FILTER.to_owned());
        let plan = default_model_runtime_probe_plan();
        serde_json::json!({
            "defaultProbeFilter": plan.default_probe_filter,
            "requestedProbeFilter": requested_filter,
            "oneModelAtATime": plan.one_model_at_a_time,
            "cpuFirst": plan.cpu_first,
            "gpuAndNpuRequireProviderProbe": plan.gpu_and_npu_require_provider_probe,
            "providerProbeTimeoutMs": plan.provider_probe_timeout_ms,
            "modelProbeTimeoutMs": plan.model_probe_timeout_ms,
            "killOnTimeout": plan.kill_on_timeout,
            "minimumChatTokensPerSecond": plan.minimum_chat_tokens_per_second,
            "targetChatTokensPerSecondLow": plan.target_chat_tokens_per_second_low,
            "targetChatTokensPerSecondHigh": plan.target_chat_tokens_per_second_high,
            "allowMultiProbe": env_truthy("ENFORCER_X06_ALLOW_MULTI_PROBE"),
            "selectedProbes": selected_probes,
            "probeOrder": PROBE_ORDER,
            "reason": if env_truthy("ENFORCER_X06_ALLOW_MULTI_PROBE") {
                "multi-probe explicitly enabled"
            } else {
                "one model at a time; CPU first; GPU/NPU only after provider probes pass; timeout kills the child process"
            }
        })
    }

    fn default_llama_cli() -> Option<PathBuf> {
        first_existing_model_bin(&llama_binary_name("llama-cli"))
    }

    fn default_llama_embedding() -> Option<PathBuf> {
        first_existing_model_bin(&llama_binary_name("llama-embedding"))
    }

    fn first_existing_model_bin(file_name: &str) -> Option<PathBuf> {
        let root = std::env::current_dir().ok()?.join("model").join("bin");
        find_file_under(&root, file_name)
    }

    fn find_file_under(root: &Path, file_name: &str) -> Option<PathBuf> {
        let entries = std::fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().and_then(|name| name.to_str()) == Some(file_name)
            {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = find_file_under(&path, file_name) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn proof_error(operation: &str, error: impl std::fmt::Display) -> serde_json::Value {
        serde_json::json!({
            "operation": operation,
            "ok": false,
            "error": error.to_string()
        })
    }

    fn proof_skipped(operation: &str) -> serde_json::Value {
        proof_skipped_reason(operation, "not selected by ENFORCER_X06_PROBE_FILTER")
    }

    fn proof_skipped_reason(operation: &str, reason: &str) -> serde_json::Value {
        serde_json::json!({
            "operation": operation,
            "ok": false,
            "skipped": true,
            "reason": reason
        })
    }

    fn excerpt_tail(text: &str, max_chars: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        let start = chars.len().saturating_sub(max_chars);
        chars[start..].iter().collect()
    }

    fn success_observation(
        observed_at: &str,
        run_id: &str,
        model_id: String,
        task: ModelTask,
        provider: ProviderKind,
    ) -> ModelRuntimeObservationRecord {
        ModelRuntimeObservationRecord::new(
            observed_at,
            "x06-model-runtime-probe",
            run_id,
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(LocalLoadSucceeded {
                model_id,
                task,
                provider,
                loaded_from_local_cache: true,
            }),
        )
    }

    fn load_failure_observation(input: LoadFailureInput<'_>) -> ModelRuntimeObservationRecord {
        ModelRuntimeObservationRecord::new(
            input.observed_at,
            "x06-model-runtime-probe",
            input.run_id,
            ModelRuntimeObservationCandidate::ModelLoadFailure(ModelLoadFailure {
                model_id: input.model_id,
                task: input.task,
                requested_provider: input.requested_provider,
                failure_reason: input.failure_reason,
            }),
        )
    }
}
