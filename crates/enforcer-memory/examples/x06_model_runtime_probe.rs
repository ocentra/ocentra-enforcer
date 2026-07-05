#[cfg(not(feature = "real-models"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;

    let proof = enforcer_memory::model_runtime::default_zero_network_proof();
    let proof_text = format!("{}\n", serde_json::to_string_pretty(&proof)?);
    std::io::stdout().write_all(proof_text.as_bytes())?;
    Ok(())
}

#[cfg(feature = "real-models")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::Duration;
    use std::time::Instant;

    use enforcer_memory::hf_cache::{
        download_hf_model, resolve_cached_hf_model, resolve_cached_hf_model_from_manifest,
        select_x06_chat_model_for_hardware, HfDownloadReport, HfModelSpec, X06ModelLineup,
    };
    use enforcer_memory::llama_cpp::{
        list_llama_cpp_devices, run_llama_cpp_probe, LlamaCppBackendHint, LlamaCppProbeConfig,
        LlamaCppProbeKind,
    };
    use enforcer_memory::local_runtime::LocalRuntimeAcceleration;
    use enforcer_memory::model_observations::{
        LocalLoadSucceeded, ModelLoadFailure, ModelRuntimeObservationCandidate,
        ModelRuntimeObservationRecord,
    };
    use enforcer_memory::model_runtime::{
        evaluate_chat_usability, loaded_non_chat_usability, resolve_model_cache_root,
        ChatThroughputPolicy, ModelCacheRootMode, ModelRuntimeServiceConfig, ModelSpec, ModelTask,
        ModelUsabilityReport, ProviderKind, DEFAULT_EMBEDDING_MODEL_ID,
        DEFAULT_MIN_CHAT_TOKENS_PER_SECOND, DEFAULT_RERANKER_MODEL_ID,
        TARGET_CHAT_TOKENS_PER_SECOND_HIGH, TARGET_CHAT_TOKENS_PER_SECOND_LOW,
    };
    use enforcer_memory::ort_runtime::{OrtEmbedder, OrtReranker};
    use enforcer_memory::ranking::RankedHit;
    use enforcer_memory::search::document::DocumentKind;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeProbeProof {
        schema_version: u32,
        runtime_mode: String,
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
    let llama_acceleration = llama_acceleration_from_env();
    let llama_backend_hint = llama_backend_hint_from_env();
    let probe_filter = probe_filter_from_env();
    let probe_execution_policy = probe_execution_policy(&probe_filter);
    let chat_throughput_policy = chat_throughput_policy_from_env();

    let mut lineup = X06ModelLineup::from_env()?;
    let chat_model_selection =
        maybe_select_chat_model_for_hardware(&mut lineup, llama_backend_hint, llama_acceleration);
    let mut observations = Vec::new();

    if runtime_mode == "plan" {
        let proof = RuntimeProbeProof {
            schema_version: 1,
            runtime_mode,
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

    if runtime_mode == "download" {
        let proof = RuntimeProbeProof {
            schema_version: 1,
            runtime_mode,
            allow_network,
            probe_execution_policy,
            cache_root: repo_relative_display(&repo_root, &cache_root),
            cache_root_policy: cache_root_policy_proof(&repo_root, &cache_root_policy)?,
            service_config: service_config_proof(&repo_root, &service_config)?,
            chat_throughput_policy,
            chat_model_selection,
            chat_generation_gguf: if should_run_probe(&probe_filter, "chat-generation-gguf") {
                cache_only_probe(
                    &lineup.chat_generation,
                    &cache_root,
                    allow_network,
                    "chat-generation-cache",
                )
            } else {
                proof_skipped("chat-generation-cache")
            },
            qwen_embedding_gguf: if should_run_probe(&probe_filter, "qwen-embedding-gguf") {
                cache_only_probe(
                    &lineup.embedding_gguf,
                    &cache_root,
                    allow_network,
                    "qwen-embedding-gguf-cache",
                )
            } else {
                proof_skipped("qwen-embedding-gguf-cache")
            },
            qwen_embedding_onnx: if should_run_probe(&probe_filter, "qwen-embedding-onnx") {
                cache_only_probe(
                    &lineup.embedding_onnx,
                    &cache_root,
                    allow_network,
                    "qwen-embedding-onnx-cache",
                )
            } else {
                proof_skipped("qwen-embedding-onnx-cache")
            },
            qwen_reranker_onnx: if should_run_probe(&probe_filter, "qwen-reranker-onnx") {
                cache_only_probe(
                    &lineup.reranker_onnx,
                    &cache_root,
                    allow_network,
                    "qwen-reranker-onnx-cache",
                )
            } else {
                proof_skipped("qwen-reranker-onnx-cache")
            },
            observations,
        };
        write_proof(&proof_out, &proof)?;
        return Ok(());
    }

    let chat_generation_gguf = if should_run_probe(&probe_filter, "chat-generation-gguf") {
        match maybe_download(&lineup.chat_generation, &cache_root, allow_network) {
            Ok(report) => run_llama_generation(
                &report,
                llama_backend_hint,
                llama_acceleration,
                &mut observations,
                &observed_at,
                &run_id,
            ),
            Err(error) => {
                observations.push(load_failure_observation(
                    &observed_at,
                    &run_id,
                    lineup.chat_generation.model_id.clone(),
                    lineup.chat_generation.task,
                    None,
                    error.clone(),
                ));
                proof_error("chat-generation-cache", error)
            }
        }
    } else {
        proof_skipped("chat-generation-gguf")
    };

    let qwen_embedding_gguf_proof = if should_run_probe(&probe_filter, "qwen-embedding-gguf") {
        match maybe_download(&lineup.embedding_gguf, &cache_root, allow_network) {
            Ok(report) => run_llama_embedding(
                &report,
                llama_backend_hint,
                llama_acceleration,
                &mut observations,
                &observed_at,
                &run_id,
            ),
            Err(error) => {
                observations.push(load_failure_observation(
                    &observed_at,
                    &run_id,
                    lineup.embedding_gguf.model_id.clone(),
                    lineup.embedding_gguf.task,
                    None,
                    error.clone(),
                ));
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
                observations.push(load_failure_observation(
                    &observed_at,
                    &run_id,
                    lineup.embedding_onnx.model_id.clone(),
                    lineup.embedding_onnx.task,
                    Some(ProviderKind::Cpu),
                    error.clone(),
                ));
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
                observations.push(load_failure_observation(
                    &observed_at,
                    &run_id,
                    lineup.reranker_onnx.model_id.clone(),
                    lineup.reranker_onnx.task,
                    Some(ProviderKind::Cpu),
                    error.clone(),
                ));
                proof_error("qwen-reranker-onnx-cache", error)
            }
        }
    } else {
        proof_skipped("qwen-reranker-onnx")
    };

    let proof = RuntimeProbeProof {
        schema_version: 1,
        runtime_mode,
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
        std::fs::write(&proof_out, serde_json::to_string_pretty(&proof)?)?;
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
        cache_root_policy: &enforcer_memory::model_runtime::ModelCacheRootPolicy,
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
            .map(|relative| format!("<repo>/{}", relative.display()))
            .unwrap_or_else(|_| path.display().to_string())
    }

    fn cache_only_probe(
        spec: &HfModelSpec,
        cache_root: &Path,
        allow_network: bool,
        operation: &str,
    ) -> serde_json::Value {
        match maybe_download(spec, cache_root, allow_network) {
            Ok(report) => serde_json::json!({
                "operation": operation,
                "ok": true,
                "repoId": report.repo_id,
                "revision": report.revision,
                "manifestPath": report.manifest_path,
                "downloadedFiles": report.downloaded_files
            }),
            Err(error) => proof_error(operation, error),
        }
    }

    fn maybe_download(
        spec: &HfModelSpec,
        cache_root: &Path,
        allow_network: bool,
    ) -> Result<HfDownloadReport, String> {
        let cached = if env_truthy("ENFORCER_X06_STRICT_CACHE_HASH") {
            resolve_cached_hf_model(spec, cache_root)
        } else {
            resolve_cached_hf_model_from_manifest(spec, cache_root)
        };
        if let Ok(report) = cached {
            return Ok(report);
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

    fn maybe_select_chat_model_for_hardware(
        lineup: &mut X06ModelLineup,
        backend_hint: LlamaCppBackendHint,
        acceleration: LocalRuntimeAcceleration,
    ) -> serde_json::Value {
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

        let binary = env_path("ENFORCER_X06_LLAMA_CLI").or_else(default_llama_cli);
        let device_report = binary.as_ref().and_then(|binary| {
            list_llama_cpp_devices(
                binary,
                env_u64("ENFORCER_X06_DEVICE_TIMEOUT_MS").unwrap_or(5_000),
            )
            .ok()
        });
        let detected_free_vram_mib = match (backend_hint, acceleration) {
            (LlamaCppBackendHint::OpenVino, LocalRuntimeAcceleration::Npu)
            | (_, LocalRuntimeAcceleration::Npu)
            | (_, LocalRuntimeAcceleration::Cpu) => None,
            _ => device_report.as_ref().and_then(|report| {
                report
                    .devices
                    .iter()
                    .map(|device| device.free_memory_mib)
                    .max()
            }),
        };
        let selection = select_x06_chat_model_for_hardware(detected_free_vram_mib);
        lineup.chat_generation = selection.selected.clone();
        serde_json::json!({
            "enabled": true,
            "backendHint": backend_hint,
            "requestedAcceleration": acceleration,
            "deviceReport": device_report,
            "selection": selection
        })
    }

    fn run_llama_generation(
        report: &HfDownloadReport,
        backend_hint: LlamaCppBackendHint,
        acceleration: LocalRuntimeAcceleration,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
        observed_at: &str,
        run_id: &str,
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
            backend_hint,
            acceleration,
            gpu_layers: env_usize("ENFORCER_X06_LLAMA_GPU_LAYERS"),
            device: std::env::var("ENFORCER_X06_LLAMA_DEVICE").ok(),
            main_gpu: env_usize("ENFORCER_X06_LLAMA_MAIN_GPU"),
            split_mode: std::env::var("ENFORCER_X06_LLAMA_SPLIT_MODE").ok(),
            tensor_split: std::env::var("ENFORCER_X06_LLAMA_TENSOR_SPLIT").ok(),
            fit: env_optional_bool("ENFORCER_X06_LLAMA_FIT").or(Some(true)),
            context_size: env_usize("ENFORCER_X06_LLAMA_CONTEXT"),
            max_tokens: env_usize("ENFORCER_X06_LLAMA_MAX_TOKENS").unwrap_or(32),
            timeout_ms: env_u64("ENFORCER_X06_LLAMA_TIMEOUT_MS").unwrap_or(120_000),
        });
        llama_result_json(
            "chat-generation-gguf",
            result,
            report.repo_id.clone(),
            ModelTask::Summarization,
            provider_kind_for_llama(backend_hint, acceleration),
            observations,
            observed_at,
            run_id,
        )
    }

    fn run_llama_embedding(
        report: &HfDownloadReport,
        backend_hint: LlamaCppBackendHint,
        acceleration: LocalRuntimeAcceleration,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
        observed_at: &str,
        run_id: &str,
    ) -> serde_json::Value {
        let Some(model) = first_model_file(report) else {
            return proof_error("qwen-embedding-gguf-model", "no GGUF model file in report");
        };
        let binary = env_path("ENFORCER_X06_LLAMA_EMBEDDING").or_else(default_llama_embedding);
        let Some(binary) = binary else {
            return proof_error(
                "qwen-embedding-gguf-llama-embedding",
                "llama-embedding executable not configured/found",
            );
        };
        let result = run_llama_cpp_probe(&LlamaCppProbeConfig {
            binary_path: binary,
            model_path: model.local_path.clone(),
            model_sha256: strict_model_hash(&model.sha256),
            prompt: "embedding hello world for x06 memory retrieval".to_owned(),
            kind: LlamaCppProbeKind::Embedding,
            backend_hint,
            acceleration,
            gpu_layers: env_usize("ENFORCER_X06_LLAMA_GPU_LAYERS"),
            device: std::env::var("ENFORCER_X06_LLAMA_DEVICE").ok(),
            main_gpu: env_usize("ENFORCER_X06_LLAMA_MAIN_GPU"),
            split_mode: std::env::var("ENFORCER_X06_LLAMA_SPLIT_MODE").ok(),
            tensor_split: std::env::var("ENFORCER_X06_LLAMA_TENSOR_SPLIT").ok(),
            fit: env_optional_bool("ENFORCER_X06_LLAMA_FIT").or(Some(true)),
            context_size: env_usize("ENFORCER_X06_LLAMA_CONTEXT"),
            max_tokens: 0,
            timeout_ms: env_u64("ENFORCER_X06_LLAMA_TIMEOUT_MS").unwrap_or(120_000),
        });
        llama_result_json(
            "qwen-embedding-gguf",
            result,
            report.repo_id.clone(),
            ModelTask::Embedding,
            provider_kind_for_llama(backend_hint, acceleration),
            observations,
            observed_at,
            run_id,
        )
    }

    fn llama_result_json(
        operation: &str,
        result: enforcer_memory::error::Result<enforcer_memory::llama_cpp::LlamaCppProbeReport>,
        model_id: String,
        task: ModelTask,
        provider: ProviderKind,
        observations: &mut Vec<ModelRuntimeObservationRecord>,
        observed_at: &str,
        run_id: &str,
    ) -> serde_json::Value {
        match result {
            Ok(report) => {
                let usability = model_usability(task, &report);
                if usability.ok {
                    observations.push(success_observation(
                        observed_at,
                        run_id,
                        model_id,
                        task,
                        provider,
                    ));
                } else {
                    observations.push(load_failure_observation(
                        observed_at,
                        run_id,
                        model_id,
                        task,
                        Some(provider),
                        usability.reason.clone(),
                    ));
                }
                serde_json::json!({
                    "operation": operation,
                    "ok": usability.ok,
                    "loaded": report.loaded(),
                    "usability": usability,
                    "report": report
                })
            }
            Err(error) => {
                observations.push(load_failure_observation(
                    observed_at,
                    run_id,
                    model_id,
                    task,
                    Some(provider),
                    error.to_string(),
                ));
                proof_error(operation, error)
            }
        }
    }

    fn model_usability(
        task: ModelTask,
        report: &enforcer_memory::llama_cpp::LlamaCppProbeReport,
    ) -> ModelUsabilityReport {
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
                observations.push(load_failure_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Embedding,
                    Some(ProviderKind::Cpu),
                    proof.to_string(),
                ));
                proof
            }
            Err(error) => {
                observations.push(load_failure_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Embedding,
                    Some(ProviderKind::Cpu),
                    error.to_string(),
                ));
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
                observations.push(load_failure_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Reranking,
                    Some(ProviderKind::Cpu),
                    proof.to_string(),
                ));
                proof
            }
            Err(error) => {
                observations.push(load_failure_observation(
                    observed_at,
                    run_id,
                    spec.model_id,
                    ModelTask::Reranking,
                    Some(ProviderKind::Cpu),
                    error.to_string(),
                ));
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
        std::env::var(name).map_err(|_| format!("missing required env {name}"))
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

    fn first_model_file(
        report: &HfDownloadReport,
    ) -> Option<&enforcer_memory::hf_cache::HfDownloadedFile> {
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
            .unwrap_or_else(|_| "auto".to_owned())
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
            (_, LocalRuntimeAcceleration::Npu) => ProviderKind::Npu,
            (_, LocalRuntimeAcceleration::Gpu) => ProviderKind::Cuda,
            (_, LocalRuntimeAcceleration::Auto | LocalRuntimeAcceleration::Cpu) => {
                ProviderKind::Cpu
            }
        }
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
                "reranker" | "ranker" | "qwen-reranker-onnx" => {
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
        serde_json::json!({
            "defaultProbeFilter": DEFAULT_PROBE_FILTER,
            "requestedProbeFilter": requested_filter,
            "allowMultiProbe": env_truthy("ENFORCER_X06_ALLOW_MULTI_PROBE"),
            "selectedProbes": selected_probes,
            "probeOrder": PROBE_ORDER,
            "reason": if env_truthy("ENFORCER_X06_ALLOW_MULTI_PROBE") {
                "multi-probe explicitly enabled"
            } else {
                "one real model probe at a time by default to avoid resource contention or host instability"
            }
        })
    }

    fn default_llama_cli() -> Option<PathBuf> {
        first_existing_model_bin("llama-cli.exe")
    }

    fn default_llama_embedding() -> Option<PathBuf> {
        first_existing_model_bin("llama-embedding.exe").or_else(default_llama_cli)
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

    fn load_failure_observation(
        observed_at: &str,
        run_id: &str,
        model_id: String,
        task: ModelTask,
        requested_provider: Option<ProviderKind>,
        failure_reason: String,
    ) -> ModelRuntimeObservationRecord {
        ModelRuntimeObservationRecord::new(
            observed_at,
            "x06-model-runtime-probe",
            run_id,
            ModelRuntimeObservationCandidate::ModelLoadFailure(ModelLoadFailure {
                model_id,
                task,
                requested_provider,
                failure_reason,
            }),
        )
    }
}
