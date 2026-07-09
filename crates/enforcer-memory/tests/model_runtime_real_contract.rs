use enforcer_memory::error::MemoryError;
use enforcer_memory::hf_cache::{
    expand_onnx_spec_from_metadata, model_cache_dir, resolve_cached_hf_model,
    resolve_cached_hf_model_from_manifest, resolve_onnx_external_data_files,
    select_x06_chat_model_for_hardware, validate_hf_file_path, validate_hf_repo_id, HfModelSpec,
    HfRepoFile, HfRepoMetadata, HfSingleFileSpecInput, X06ModelLineup,
};
use enforcer_memory::llama_cpp::{
    llama_cpp_command_plan, parse_llama_cpp_devices, resolve_llama_cpp_execution,
    validate_executable, validate_model, LlamaCppBackendHint, LlamaCppDevice, LlamaCppProbeConfig,
    LlamaCppProbeKind,
};
use enforcer_memory::local_runtime::LocalRuntimeAcceleration;
use enforcer_memory::local_runtime::RuntimeOwnershipMode;
use enforcer_memory::model_runtime::{
    dev_model_cache_root, evaluate_chat_usability, loaded_non_chat_usability,
    resolve_model_cache_root, ChatThroughputPolicy, ModelCacheRootMode, ModelRuntimeServiceConfig,
    DEFAULT_EMBEDDING_GGUF_FILE, DEFAULT_EMBEDDING_GGUF_REPO, DEFAULT_EMBEDDING_ONNX_FILE,
    DEFAULT_EMBEDDING_ONNX_REPO, DEFAULT_MIN_CHAT_TOKENS_PER_SECOND, DEFAULT_MODEL_CACHE_DIR_NAME,
    DEFAULT_ORNITH_GGUF_FILE, DEFAULT_ORNITH_GGUF_REPO, DEFAULT_RERANKER_ONNX_FILE,
    DEFAULT_RERANKER_ONNX_REPO, TARGET_CHAT_TOKENS_PER_SECOND_HIGH,
    TARGET_CHAT_TOKENS_PER_SECOND_LOW,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn llama_binary_name(base_name: &str) -> String {
    format!("{base_name}{}", std::env::consts::EXE_SUFFIX)
}

fn assert_contract_terms(haystack: &str, expected_terms: &[&str]) {
    let missing_terms: Vec<&str> = expected_terms
        .iter()
        .copied()
        .filter(|term| !haystack.contains(term))
        .collect();
    assert_eq!(missing_terms, Vec::<&str>::new());
}

fn assert_model_runtime_error(
    result: Result<(), MemoryError>,
    expected_operation: &str,
    expected_reason: &str,
) -> TestResult {
    match result {
        Err(error) => match error {
            MemoryError::ModelRuntime { operation, reason } => {
                assert_eq!(operation, expected_operation);
                assert_eq!(reason, expected_reason);
                Ok(())
            }
            other => Err(format!("expected model runtime error, got {other:?}").into()),
        },
        Ok(()) => Err("expected model runtime error, got Ok(())".into()),
    }
}

#[test]
fn hf_specs_pin_enforcer_model_lineup() -> TestResult {
    let lineup = X06ModelLineup::defaults();
    lineup.validate()?;

    let ornith = lineup.chat_generation;
    assert_eq!(ornith.repo_id, DEFAULT_ORNITH_GGUF_REPO);
    assert_eq!(ornith.files[0].path, DEFAULT_ORNITH_GGUF_FILE);

    let embedding_gguf = lineup.embedding_gguf;
    assert_eq!(embedding_gguf.repo_id, DEFAULT_EMBEDDING_GGUF_REPO);
    assert_eq!(embedding_gguf.files[0].path, DEFAULT_EMBEDDING_GGUF_FILE);

    let embedding_onnx = lineup.embedding_onnx;
    assert_eq!(embedding_onnx.repo_id, DEFAULT_EMBEDDING_ONNX_REPO);
    assert!(embedding_onnx
        .files
        .iter()
        .any(|file| file.path == DEFAULT_EMBEDDING_ONNX_FILE));
    assert!(embedding_onnx
        .files
        .iter()
        .any(|file| file.path == "tokenizer.json"));

    let reranker_onnx = lineup.reranker_onnx;
    assert_eq!(reranker_onnx.repo_id, DEFAULT_RERANKER_ONNX_REPO);
    assert!(reranker_onnx
        .files
        .iter()
        .any(|file| file.path == DEFAULT_RERANKER_ONNX_FILE));
    Ok(())
}

#[test]
fn user_model_override_specs_validate_before_download() -> TestResult {
    let spec = HfModelSpec::with_single_model_file(HfSingleFileSpecInput {
        repo_id: "custom-org/custom-chat-gguf".to_owned(),
        revision: "main".to_owned(),
        backend: enforcer_memory::local_runtime::LocalRuntimeBackend::LlamaCpp,
        task: enforcer_memory::model_runtime::ModelTask::Summarization,
        model_id: "custom-org/custom-chat-gguf".to_owned(),
        acceleration: enforcer_memory::local_runtime::LocalRuntimeAcceleration::Auto,
        file_path: "custom-chat-Q4_K_M.gguf".to_owned(),
    });
    spec.validate()?;

    let bad = HfModelSpec::with_single_model_file(HfSingleFileSpecInput {
        repo_id: "../bad/repo".to_owned(),
        revision: "main".to_owned(),
        backend: enforcer_memory::local_runtime::LocalRuntimeBackend::LlamaCpp,
        task: enforcer_memory::model_runtime::ModelTask::Summarization,
        model_id: "../bad/repo".to_owned(),
        acceleration: enforcer_memory::local_runtime::LocalRuntimeAcceleration::Auto,
        file_path: "../secret.gguf".to_owned(),
    });
    assert_model_runtime_error(
        bad.validate(),
        "validate-hf-repo-id",
        "invalid Hugging Face repo id: \"../bad/repo\"",
    )?;
    Ok(())
}

#[test]
fn llama_cpp_device_parser_reads_vram_from_backend_output() {
    let devices = parse_llama_cpp_devices(
        "Available devices:\n  Vulkan0: GeForce RTX 2070 SUPER (8257 MiB, 7484 MiB free)\n  Vulkan1: GeForce GTX 760 (2007 MiB, 1706 MiB free)\n",
    );

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].id, "Vulkan0");
    assert_eq!(devices[0].free_memory_mib, 7484);
    assert_eq!(devices[1].name, "GeForce GTX 760");
}

#[test]
fn chat_model_selector_prefers_q4_model_that_fits_detected_hardware() {
    let selection = select_x06_chat_model_for_hardware(Some(7_484));

    assert_eq!(selection.selected.model_id, "Qwen/Qwen3-4B-GGUF:Q4_K_M");
    assert_eq!(selection.selected_quantization, "Q4_K_M");
    assert_eq!(
        selection.reason,
        "selected Qwen/Qwen3-4B-GGUF:Q4_K_M because detected free VRAM is 7484 MiB and required free VRAM is 4096 MiB"
    );
}

#[test]
fn chat_model_selector_prefers_moe_when_vram_can_fit_it() {
    let selection = select_x06_chat_model_for_hardware(Some(24_000));

    assert_eq!(selection.selected.repo_id, "Qwen/Qwen3-30B-A3B-GGUF");
    assert_eq!(
        selection.selected.files[0].path,
        "Qwen3-30B-A3B-Q4_K_M.gguf"
    );
    assert!(selection
        .candidates
        .iter()
        .any(|candidate| candidate.active_parameter_count_millions == Some(3_000)));
}

#[test]
fn chat_model_selector_retains_ornith_as_dense_fallback_candidate() {
    let selection = select_x06_chat_model_for_hardware(Some(24_000));

    assert!(selection
        .candidates
        .iter()
        .any(|candidate| candidate.spec.repo_id == DEFAULT_ORNITH_GGUF_REPO));
}

#[test]
fn auto_llama_execution_prefers_best_gpu_and_backend_from_device_probe() {
    let devices = vec![
        LlamaCppDevice {
            id: "Vulkan0".to_owned(),
            name: "GeForce RTX 2070 SUPER".to_owned(),
            total_memory_mib: 8_257,
            free_memory_mib: 7_484,
        },
        LlamaCppDevice {
            id: "Vulkan1".to_owned(),
            name: "GeForce GTX 760".to_owned(),
            total_memory_mib: 2_007,
            free_memory_mib: 1_706,
        },
    ];

    let resolution = resolve_llama_cpp_execution(
        LlamaCppBackendHint::Auto,
        LocalRuntimeAcceleration::Auto,
        &devices,
    );

    assert_eq!(resolution.backend_hint, LlamaCppBackendHint::Vulkan);
    assert_eq!(
        resolution.resolved_acceleration,
        LocalRuntimeAcceleration::Gpu
    );
    assert_eq!(resolution.selected_device_id.as_deref(), Some("Vulkan0"));
    assert_eq!(resolution.selected_main_gpu, Some(0));
    assert_eq!(resolution.detected_free_vram_mib, Some(7_484));
    assert_eq!(resolution.downgrade_reason, None);
}

#[test]
fn requested_gpu_without_provider_probe_downgrades_to_cpu() {
    let resolution = resolve_llama_cpp_execution(
        LlamaCppBackendHint::Auto,
        LocalRuntimeAcceleration::Gpu,
        &[],
    );

    assert_eq!(resolution.backend_hint, LlamaCppBackendHint::Native);
    assert_eq!(
        resolution.resolved_acceleration,
        LocalRuntimeAcceleration::Cpu
    );
    assert_eq!(resolution.selected_device_id, None);
    assert!(resolution
        .downgrade_reason
        .as_deref()
        .unwrap_or_default()
        .contains("requested GPU acceleration"));
}

#[test]
fn requested_npu_prefers_openvino_when_npu_device_is_reported() {
    let devices = vec![LlamaCppDevice {
        id: "NPU0".to_owned(),
        name: "Intel NPU".to_owned(),
        total_memory_mib: 1_024,
        free_memory_mib: 768,
    }];

    let resolution = resolve_llama_cpp_execution(
        LlamaCppBackendHint::Auto,
        LocalRuntimeAcceleration::Npu,
        &devices,
    );

    assert_eq!(resolution.backend_hint, LlamaCppBackendHint::OpenVino);
    assert_eq!(
        resolution.resolved_acceleration,
        LocalRuntimeAcceleration::Npu
    );
    assert_eq!(resolution.selected_device_id.as_deref(), Some("NPU0"));
    assert_eq!(resolution.selected_main_gpu, None);
    assert_eq!(resolution.detected_free_vram_mib, Some(768));
}

#[test]
fn chat_usability_requires_ten_tokens_per_second_and_records_target_band() {
    let policy = ChatThroughputPolicy::default();

    assert_eq!(
        policy.min_tokens_per_second,
        DEFAULT_MIN_CHAT_TOKENS_PER_SECOND
    );
    assert_eq!(
        policy.target_tokens_per_second_low,
        TARGET_CHAT_TOKENS_PER_SECOND_LOW
    );
    assert_eq!(
        policy.target_tokens_per_second_high,
        TARGET_CHAT_TOKENS_PER_SECOND_HIGH
    );

    let crawl = evaluate_chat_usability(true, Some(2.0), "loaded", policy);
    assert!(!crawl.ok);
    assert_eq!(crawl.min_chat_tokens_per_second, Some(10.0));
    assert_eq!(crawl.target_chat_tokens_per_second_low, Some(40.0));
    assert_eq!(crawl.target_chat_tokens_per_second_high, Some(60.0));
    assert_eq!(
        crawl.reason,
        "chat not usable: measured 2.00 tokens/sec < required 10.00; target 40.00-60.00 tokens/sec"
    );

    let usable = evaluate_chat_usability(true, Some(10.0), "loaded", policy);
    assert!(usable.ok);
}

#[test]
fn non_chat_model_loads_do_not_use_chat_throughput_floor() {
    let loaded = loaded_non_chat_usability(true, None, "loaded");

    assert!(loaded.ok);
    assert_eq!(loaded.min_chat_tokens_per_second, None);
}

#[test]
fn dev_model_cache_is_repo_local_and_service_does_not_expose_llama_server() {
    let repo = std::path::Path::new("repo-root");
    let gitignore = include_str!("../../../.gitignore");

    let cache_root = dev_model_cache_root(repo);
    let policy = resolve_model_cache_root(repo, ModelCacheRootMode::DevRepoLocal, None);
    let service = ModelRuntimeServiceConfig::dev(repo);

    assert_eq!(cache_root, repo.join(DEFAULT_MODEL_CACHE_DIR_NAME));
    assert_eq!(policy.root, repo.join(DEFAULT_MODEL_CACHE_DIR_NAME));
    assert!(!service.expose_llama_server);
    assert_eq!(
        service.llama_cpp_ownership,
        RuntimeOwnershipMode::EnforcerSubprocess
    );
    assert_eq!(
        service.ort_ownership,
        RuntimeOwnershipMode::EnforcerInProcess
    );
    assert_eq!(service.cache_root, repo.join(DEFAULT_MODEL_CACHE_DIR_NAME));
    assert_eq!(service.bind_addr(), "127.0.0.1:8766");
    assert!(
        gitignore.lines().any(|line| line.trim() == "model/"),
        "repo-local model cache must never be staged"
    );
}

#[test]
fn runtime_proof_surface_does_not_hardcode_machine_absolute_paths() {
    let files = [
        ("runtime_probe.rs", include_str!("../src/runtime_probe.rs")),
        (
            "x06-real-model-proof.ps1",
            include_str!("../scripts/x06-real-model-proof.ps1"),
        ),
    ];
    let banned = [
        concat!("E", ":\\"),
        concat!("C", ":\\", "Users"),
        concat!("Desktop", "\\", "TabAgent"),
        concat!("ocentra", "-", "model", "-", "cache"),
        concat!("ocentra", "-", "enforcer", "-", "rust", "-", "build"),
    ];

    for (name, body) in files {
        for pattern in banned {
            assert!(
                !body.contains(pattern),
                "{name} contains hardcoded machine path pattern {pattern}"
            );
        }
    }
}

#[test]
fn checked_in_real_model_proofs_do_not_hardcode_machine_absolute_paths() {
    let proof_files = [
        (
            "x06-models.json",
            include_str!("../../../proof/memory/x06-models.json"),
        ),
        (
            "x06-models-chat-plan.json",
            include_str!("../../../proof/memory/x06-models-chat-plan.json"),
        ),
        (
            "x06-models-multi-probe-plan.json",
            include_str!("../../../proof/memory/x06-models-multi-probe-plan.json"),
        ),
        (
            "x06-models-qwen3-4b-download-local.json",
            include_str!("../../../proof/memory/x06-models-qwen3-4b-download-local.json"),
        ),
        (
            "x06-models-qwen3-4b-cpu-windows-local.json",
            include_str!("../../../proof/memory/x06-models-qwen3-4b-cpu-windows-local.json"),
        ),
        (
            "x06-models-qwen3-4b-vulkan-windows-local.json",
            include_str!("../../../proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json"),
        ),
        (
            "x06-models-chat-auto-gpu.json",
            include_str!("../../../proof/memory/x06-models-chat-auto-gpu.json"),
        ),
        (
            "x06-models-gemma3-4b-vulkan-live.json",
            include_str!("../../../proof/memory/x06-models-gemma3-4b-vulkan-live.json"),
        ),
        (
            "x06-models-qwen3-embedding-download.json",
            include_str!("../../../proof/memory/x06-models-qwen3-embedding-download.json"),
        ),
        (
            "x06-models-gemma3-4b-download-live.json",
            include_str!("../../../proof/memory/x06-models-gemma3-4b-download-live.json"),
        ),
        (
            "x06-models-qwen3-embedding-gguf-vulkan-live.json",
            include_str!("../../../proof/memory/x06-models-qwen3-embedding-gguf-vulkan-live.json"),
        ),
        (
            "x06-models-qwen3-embedding-ort-cpu.json",
            include_str!("../../../proof/memory/x06-models-qwen3-embedding-ort-cpu.json"),
        ),
        (
            "x06-models-qwen3-reranker-download.json",
            include_str!("../../../proof/memory/x06-models-qwen3-reranker-download.json"),
        ),
        (
            "x06-models-qwen3-reranker-ort-cpu.json",
            include_str!("../../../proof/memory/x06-models-qwen3-reranker-ort-cpu.json"),
        ),
        (
            "x06-models-cache-only-missing.json",
            include_str!("../../../proof/memory/x06-models-cache-only-missing.json"),
        ),
        (
            "x06-models-cache-only-preseeded.json",
            include_str!("../../../proof/memory/x06-models-cache-only-preseeded.json"),
        ),
        (
            "x06-models-hash-mismatch.json",
            include_str!("../../../proof/memory/x06-models-hash-mismatch.json"),
        ),
        (
            "x06-models-tokenizer-mismatch.json",
            include_str!("../../../proof/memory/x06-models-tokenizer-mismatch.json"),
        ),
    ];
    let banned = [
        concat!("E", ":\\"),
        concat!("C", ":\\", "Users"),
        concat!("Desktop", "\\", "TabAgent"),
        concat!("ocentra", "-", "enforcer", "-", "rust", "-", "build"),
    ];

    for (name, body) in proof_files {
        for pattern in banned {
            assert!(
                !body.contains(pattern),
                "{name} contains hardcoded machine path pattern {pattern}"
            );
        }
    }
}

#[test]
fn checked_in_real_model_proofs_are_not_claimed_as_ci_parity() -> TestResult {
    let proof_files = [
        include_str!("../../../proof/memory/x06-models.json"),
        include_str!("../../../proof/memory/x06-models-chat-plan.json"),
        include_str!("../../../proof/memory/x06-models-multi-probe-plan.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-4b-download-local.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-4b-cpu-windows-local.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json"),
        include_str!("../../../proof/memory/x06-models-chat-auto-gpu.json"),
        include_str!("../../../proof/memory/x06-models-gemma3-4b-vulkan-live.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-embedding-download.json"),
        include_str!("../../../proof/memory/x06-models-gemma3-4b-download-live.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-embedding-gguf-vulkan-live.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-embedding-ort-cpu.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-reranker-download.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-reranker-ort-cpu.json"),
        include_str!("../../../proof/memory/x06-models-cache-only-missing.json"),
        include_str!("../../../proof/memory/x06-models-cache-only-preseeded.json"),
        include_str!("../../../proof/memory/x06-models-hash-mismatch.json"),
        include_str!("../../../proof/memory/x06-models-tokenizer-mismatch.json"),
    ];

    for body in proof_files {
        let proof: serde_json::Value = serde_json::from_str(body)?;
        assert_eq!(proof["proofScope"]["ciParity"], false);
        assert_ne!(proof["proofScope"]["portability"], "portable-ci-proof");
    }
    Ok(())
}

#[test]
fn checked_in_cache_only_proofs_are_zero_network_and_path_redacted() -> TestResult {
    let missing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-cache-only-missing.json"
    ))?;
    let preseeded: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-cache-only-preseeded.json"
    ))?;

    for proof in [&missing, &preseeded] {
        assert_eq!(proof["runtimeMode"], "cache-only");
        assert_eq!(proof["allowNetwork"], false);
        assert_eq!(
            proof["proofScope"]["portability"],
            "portable-cache-contract"
        );
        assert_eq!(proof["proofScope"]["localHardwareRequired"], false);

        let chat = &proof["chatGenerationGguf"];
        assert_eq!(chat["cacheOnly"], true);
        assert_eq!(chat["downloadEnabled"], false);
        assert_eq!(chat["networkMayBeAttempted"], false);
        assert_eq!(chat["strictCacheHash"], true);
    }

    assert_eq!(missing["chatGenerationGguf"]["ok"], false);
    assert!(missing["chatGenerationGguf"]["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("<repo>/target/x06-cache-proof/missing"));
    assert_eq!(preseeded["chatGenerationGguf"]["ok"], true);
    assert_eq!(
        preseeded["chatGenerationGguf"]["downloadedFiles"][0]["localPath"],
        "<repo>/target/x06-cache-proof/preseeded/hf/local-fixtures--x06-chat-cache/main/chat-fixture-Q4_K_M.gguf"
    );

    let banned_paths: [(&str, &str); 3] = [
        ("windows-drive-backslash", concat!("E", ":\\")),
        ("windows-drive-url-slash", concat!("E", "://")),
        ("user-profile-path", concat!("C", ":\\", "Users")),
    ];
    for (banned_label, banned_pattern) in banned_paths {
        for proof_text in [
            include_str!("../../../proof/memory/x06-models-cache-only-missing.json"),
            include_str!("../../../proof/memory/x06-models-cache-only-preseeded.json"),
        ] {
            assert_eq!(
                proof_text.find(banned_pattern),
                None,
                "{banned_label} leaked into cache-only proof"
            );
        }
    }

    Ok(())
}

#[test]
fn checked_in_qwen3_vulkan_chat_probe_is_real_usable_local_gguf() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json"
    ))?;
    let chat = &proof["chatGenerationGguf"];
    let report = &chat["report"];
    let usability = &chat["usability"];

    assert_eq!(proof["runtimeMode"], "probe");
    assert_eq!(proof["proofScope"]["portability"], "local-runtime-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], true);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(
        proof["chatModelSelection"]["selected"]["repoId"],
        "Qwen/Qwen3-4B-GGUF"
    );

    assert_eq!(chat["operation"], "chat-generation-gguf");
    assert_eq!(chat["loaded"], true);
    assert_eq!(chat["ok"], true);
    assert_eq!(report["kind"], "generate");
    assert_eq!(report["backendHint"], "vulkan");
    assert_eq!(report["requestedAcceleration"], "gpu");
    assert_eq!(
        report["modelPath"],
        "<repo>/model/hf/Qwen--Qwen3-4B-GGUF/main/Qwen3-4B-Q4_K_M.gguf"
    );
    assert!(report["stdoutExcerpt"]
        .as_str()
        .unwrap_or_default()
        .contains("Say hello from the local chat model in one short sentence."));

    let measured = usability["measuredTokensPerSecond"]
        .as_f64()
        .ok_or("missing chat throughput measurement")?;
    assert!(measured >= DEFAULT_MIN_CHAT_TOKENS_PER_SECOND);
    assert!(measured >= TARGET_CHAT_TOKENS_PER_SECOND_HIGH);
    assert_eq!(usability["ok"], true);

    let observation = &proof["observations"][0]["candidate"];
    assert_eq!(observation["observationKind"], "successful-local-load");
    assert_eq!(observation["modelId"], "Qwen/Qwen3-4B-GGUF");
    assert_eq!(observation["provider"], "vulkan");
    assert_eq!(observation["loadedFromLocalCache"], true);
    Ok(())
}

#[test]
fn checked_in_qwen3_cpu_chat_probe_records_below_floor_failure() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-4b-cpu-windows-local.json"
    ))?;
    let chat = &proof["chatGenerationGguf"];
    let usability = &chat["usability"];

    assert_eq!(chat["loaded"], true);
    assert_eq!(chat["ok"], false);
    assert_eq!(chat["report"]["requestedAcceleration"], "cpu");
    assert_eq!(usability["ok"], false);
    assert_eq!(usability["minChatTokensPerSecond"], 10.0);
    assert!(usability["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("measured 8.50 tokens/sec < required 10.00"));
    assert_eq!(
        proof["observations"][0]["candidate"]["observationKind"],
        "model-load-failure"
    );
    Ok(())
}

#[test]
fn checked_in_auto_gpu_chat_probe_selects_qwen_and_is_usable() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-chat-auto-gpu.json"
    ))?;
    let plan = &proof["probeExecutionPolicy"];
    let selection = &proof["chatModelSelection"];
    let chat = &proof["chatGenerationGguf"];
    let report = &chat["report"];
    let usability = &chat["usability"];

    assert_eq!(proof["runtimeMode"], "probe");
    assert_eq!(proof["proofScope"]["portability"], "local-runtime-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], true);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["cacheRoot"], "<repo>/model");
    assert_eq!(proof["serviceConfig"]["exposeLlamaServer"], false);

    assert_eq!(plan["allowMultiProbe"], false);
    assert_eq!(plan["cpuFirst"], true);
    assert_eq!(plan["oneModelAtATime"], true);
    assert_eq!(plan["gpuAndNpuRequireProviderProbe"], true);
    assert_eq!(plan["selectedProbes"][0], "chat-generation-gguf");

    assert_eq!(selection["requestedBackendHint"], "auto");
    assert_eq!(selection["requestedAcceleration"], "gpu");
    assert_eq!(selection["resolvedAcceleration"], "gpu");
    assert_eq!(selection["providerProbePassed"], true);
    assert_eq!(selection["selectedDeviceId"], "Vulkan0");
    assert_eq!(
        selection["selection"]["selected"]["repoId"],
        "Qwen/Qwen3-4B-GGUF"
    );
    assert_eq!(selection["selection"]["selectedQuantization"], "Q4_K_M");

    assert_eq!(chat["operation"], "chat-generation-gguf");
    assert_eq!(chat["loaded"], true);
    assert_eq!(chat["ok"], true);
    assert_eq!(report["kind"], "generate");
    assert_eq!(report["backendHint"], "vulkan");
    assert_eq!(report["requestedAcceleration"], "gpu");
    assert_eq!(
        report["modelPath"],
        "<repo>/model/hf/Qwen--Qwen3-4B-GGUF/main/Qwen3-4B-Q4_K_M.gguf"
    );

    let measured = usability["measuredTokensPerSecond"]
        .as_f64()
        .ok_or("missing auto-gpu chat throughput measurement")?;
    assert!(measured >= DEFAULT_MIN_CHAT_TOKENS_PER_SECOND);
    assert!(measured >= TARGET_CHAT_TOKENS_PER_SECOND_HIGH);
    assert_eq!(usability["ok"], true);

    let observation = &proof["observations"][0]["candidate"];
    assert_eq!(observation["observationKind"], "successful-local-load");
    assert_eq!(observation["modelId"], "Qwen/Qwen3-4B-GGUF");
    assert_eq!(observation["provider"], "vulkan");
    assert_eq!(observation["loadedFromLocalCache"], true);
    Ok(())
}

#[test]
fn checked_in_gemma_vulkan_chat_probe_is_real_usable_local_gguf() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-gemma3-4b-vulkan-live.json"
    ))?;
    let chat = &proof["chatGenerationGguf"];
    let report = &chat["report"];
    let usability = &chat["usability"];

    assert_eq!(proof["runtimeMode"], "probe");
    assert_eq!(proof["allowNetwork"], false);
    assert_eq!(proof["proofScope"]["portability"], "local-runtime-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], true);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(
        proof["chatModelSelection"]["selected"]["repoId"],
        "bartowski/google_gemma-3-4b-it-GGUF"
    );

    assert_eq!(chat["operation"], "chat-generation-gguf");
    assert_eq!(chat["loaded"], true);
    assert_eq!(chat["ok"], true);
    assert_eq!(report["kind"], "generate");
    assert_eq!(report["backendHint"], "vulkan");
    assert_eq!(report["requestedAcceleration"], "gpu");
    assert_eq!(
        report["modelPath"],
        "<repo>/model/hf/bartowski--google_gemma-3-4b-it-GGUF/main/google_gemma-3-4b-it-Q4_K_M.gguf"
    );
    assert!(report["stdoutExcerpt"]
        .as_str()
        .unwrap_or_default()
        .contains("Say hello from the local chat model in one short sentence."));

    let measured = usability["measuredTokensPerSecond"]
        .as_f64()
        .ok_or("missing gemma chat throughput measurement")?;
    assert!(measured >= DEFAULT_MIN_CHAT_TOKENS_PER_SECOND);
    assert!(measured >= TARGET_CHAT_TOKENS_PER_SECOND_LOW);
    assert_eq!(usability["ok"], true);

    let observation = &proof["observations"][0]["candidate"];
    assert_eq!(observation["observationKind"], "successful-local-load");
    assert_eq!(
        observation["modelId"],
        "bartowski/google_gemma-3-4b-it-GGUF"
    );
    assert_eq!(observation["provider"], "vulkan");
    assert_eq!(observation["loadedFromLocalCache"], true);
    Ok(())
}

#[test]
fn checked_in_gemma_download_proof_records_repo_local_cache_acquisition() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-gemma3-4b-download-live.json"
    ))?;
    let chat = &proof["chatGenerationGguf"];

    assert_eq!(proof["runtimeMode"], "download");
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["proofScope"]["portability"], "cache-artifact-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(
        proof["chatModelSelection"]["selection"]["selected"]["repoId"],
        "bartowski/google_gemma-3-4b-it-GGUF"
    );

    assert_eq!(chat["operation"], "chat-generation-cache");
    assert_eq!(chat["ok"], true);
    assert_eq!(
        chat["cacheDir"],
        "<repo>/model/hf/bartowski--google_gemma-3-4b-it-GGUF/main"
    );
    assert_eq!(
        chat["manifestPath"],
        "<repo>/model/hf/bartowski--google_gemma-3-4b-it-GGUF/main/manifest.json"
    );
    assert_eq!(
        chat["downloadedFiles"][0]["localPath"],
        "<repo>/model/hf/bartowski--google_gemma-3-4b-it-GGUF/main/google_gemma-3-4b-it-Q4_K_M.gguf"
    );
    assert_eq!(
        chat["downloadedFiles"][0]["streamingManifestPath"],
        serde_json::Value::Null
    );
    Ok(())
}

#[test]
fn checked_in_qwen3_chat_download_proof_records_repo_local_cache_acquisition() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-4b-download-local.json"
    ))?;
    let chat = &proof["chatGenerationGguf"];

    assert_eq!(proof["runtimeMode"], "download");
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["proofScope"]["portability"], "cache-artifact-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["chatModelSelection"]["enabled"], false);
    assert_eq!(
        proof["chatModelSelection"]["selected"]["repoId"],
        "Qwen/Qwen3-4B-GGUF"
    );

    assert_eq!(chat["operation"], "chat-generation-cache");
    assert_eq!(chat["ok"], true);
    assert_eq!(chat["cacheDir"], "<repo>/model/hf/Qwen--Qwen3-4B-GGUF/main");
    assert_eq!(
        chat["manifestPath"],
        "<repo>/model/hf/Qwen--Qwen3-4B-GGUF/main/manifest.json"
    );
    assert_eq!(
        chat["downloadedFiles"][0]["localPath"],
        "<repo>/model/hf/Qwen--Qwen3-4B-GGUF/main/Qwen3-4B-Q4_K_M.gguf"
    );
    assert_eq!(
        chat["downloadedFiles"][0]["streamingManifestPath"],
        serde_json::Value::Null
    );
    Ok(())
}

#[test]
fn checked_in_qwen3_embedding_download_proof_records_repo_local_cache_acquisition() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-embedding-download.json"
    ))?;
    let embedding = &proof["qwenEmbeddingOnnx"];

    assert_eq!(proof["runtimeMode"], "download");
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["proofScope"]["portability"], "cache-artifact-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["chatModelSelection"]["enabled"], false);
    assert_eq!(
        proof["probeExecutionPolicy"]["selectedProbes"][0],
        "qwen-embedding-onnx"
    );

    assert_eq!(embedding["operation"], "qwen-embedding-onnx-cache");
    assert_eq!(embedding["ok"], true);
    assert_eq!(
        embedding["cacheDir"],
        "<repo>/model/hf/onnx-community--Qwen3-Embedding-0.6B-ONNX/main"
    );
    assert_eq!(
        embedding["manifestPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Embedding-0.6B-ONNX/main/manifest.json"
    );
    assert_eq!(
        embedding["downloadedFiles"][0]["localPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Embedding-0.6B-ONNX/main/onnx/model_q4.onnx"
    );
    assert_eq!(
        embedding["downloadedFiles"][1]["localPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Embedding-0.6B-ONNX/main/tokenizer.json"
    );
    Ok(())
}

#[test]
fn checked_in_qwen3_reranker_download_proof_records_repo_local_cache_acquisition() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-reranker-download.json"
    ))?;
    let reranker = &proof["qwenRerankerOnnx"];

    assert_eq!(proof["runtimeMode"], "download");
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["proofScope"]["portability"], "cache-artifact-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["chatModelSelection"]["enabled"], false);
    assert_eq!(
        proof["probeExecutionPolicy"]["selectedProbes"][0],
        "qwen-reranker-onnx"
    );

    assert_eq!(reranker["operation"], "qwen-reranker-onnx-cache");
    assert_eq!(reranker["ok"], true);
    assert_eq!(
        reranker["cacheDir"],
        "<repo>/model/hf/onnx-community--Qwen3-Reranker-0.6B-ONNX/main"
    );
    assert_eq!(
        reranker["manifestPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Reranker-0.6B-ONNX/main/manifest.json"
    );
    assert_eq!(
        reranker["downloadedFiles"][0]["localPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Reranker-0.6B-ONNX/main/onnx/model_q4.onnx"
    );
    assert_eq!(
        reranker["downloadedFiles"][1]["localPath"],
        "<repo>/model/hf/onnx-community--Qwen3-Reranker-0.6B-ONNX/main/tokenizer.json"
    );
    Ok(())
}

#[test]
fn checked_in_qwen3_embedding_gguf_probe_is_real_usable_local_runtime() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-embedding-gguf-vulkan-live.json"
    ))?;
    let embedding = &proof["qwenEmbeddingGguf"];
    let report = &embedding["report"];
    let usability = &embedding["usability"];

    assert_eq!(proof["runtimeMode"], "probe");
    assert_eq!(proof["allowNetwork"], true);
    assert_eq!(proof["proofScope"]["portability"], "local-runtime-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], true);
    assert_eq!(proof["proofScope"]["ciParity"], false);

    assert_eq!(embedding["operation"], "qwen-embedding-gguf");
    assert_eq!(embedding["loaded"], true);
    assert_eq!(embedding["ok"], true);
    assert_eq!(report["kind"], "embedding");
    assert_eq!(report["backendHint"], "vulkan");
    assert_eq!(
        report["binaryPath"],
        format!(
            "<repo>/model/bin/llama-b9904-bin-win-vulkan-x64/{}",
            llama_binary_name("llama-server")
        )
    );
    assert_eq!(report["executionRoute"], "llama-server-v1-embeddings");
    assert_eq!(
        report["fallbackReason"],
        "llama-embedding executable not configured/found; using llama-server /v1/embeddings"
    );
    assert_eq!(report["fallbackFromBinaryPath"], serde_json::Value::Null);
    assert_eq!(report["requestedAcceleration"], "gpu");
    assert_eq!(
        report["modelPath"],
        "<repo>/model/hf/Qwen--Qwen3-Embedding-0.6B-GGUF/main/Qwen3-Embedding-0.6B-Q8_0.gguf"
    );
    assert_eq!(report["outputDimensions"], 1024);
    assert!(report["stdoutExcerpt"]
        .as_str()
        .unwrap_or_default()
        .contains("embedding dimensions: 1024"));
    assert_eq!(
        usability["reason"],
        "embedding usable: dimensions 1024 matched expected 1024"
    );
    assert_eq!(usability["ok"], true);

    let observation = &proof["observations"][0]["candidate"];
    assert_eq!(observation["observationKind"], "successful-local-load");
    assert_eq!(observation["modelId"], "Qwen/Qwen3-Embedding-0.6B-GGUF");
    assert_eq!(observation["provider"], "vulkan");
    assert_eq!(observation["loadedFromLocalCache"], true);
    Ok(())
}

#[test]
fn checked_in_negative_mismatch_proofs_emit_learning_observations() -> TestResult {
    let proof_files = [
        (
            "artifact-hash-mismatch",
            "x06-models-hash-mismatch",
            include_str!("../../../proof/memory/x06-models-hash-mismatch.json"),
        ),
        (
            "tokenizer-hash-mismatch",
            "x06-models-tokenizer-mismatch",
            include_str!("../../../proof/memory/x06-models-tokenizer-mismatch.json"),
        ),
    ];

    for (kind, run_id, body) in proof_files {
        let proof: serde_json::Value = serde_json::from_str(body)?;
        assert_eq!(proof["runtimeMode"], "negative-fixture");
        assert_eq!(proof["allowNetwork"], false);
        assert_eq!(proof["ok"], false);
        assert_eq!(
            proof["proofScope"]["portability"],
            "portable-negative-contract"
        );
        assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
        assert_eq!(proof["proofScope"]["ciParity"], false);
        assert_eq!(proof["case"], kind);
        assert_eq!(proof["error"]["kind"], kind);

        let observation = &proof["observations"][0];
        assert_eq!(observation["source"], "x06-model-runtime-negative-proof");
        assert_eq!(observation["runId"], run_id);
        let candidate = &observation["candidate"];
        assert_eq!(candidate["observationKind"], kind);
        assert!(candidate["path"]
            .as_str()
            .unwrap_or_default()
            .starts_with("<repo>/model/"));
        let expected = candidate["expectedSha256"].as_str().unwrap_or_default();
        let observed = candidate["observedSha256"].as_str().unwrap_or_default();
        assert_eq!(expected.len(), 64);
        assert_eq!(observed.len(), 64);
        assert_ne!(expected, observed);
    }
    Ok(())
}

#[test]
fn portable_plan_proof_does_not_probe_local_hardware() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-models.json"))?;
    let chat_plan: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-chat-plan.json"
    ))?;
    let multi_probe_plan: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-multi-probe-plan.json"
    ))?;

    assert_eq!(proof["runtimeMode"], "plan");
    assert_eq!(proof["proofScope"]["portability"], "portable-contract");
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert!(proof["chatModelSelection"]["deviceReport"].is_null());
    assert_eq!(proof["cacheRoot"], "<repo>/model");
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][0]["artifactPath"],
        "proof/memory/x06-models-chat-plan.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][0]["status"],
        "chat-plan-variant"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][0]["runtimeMode"],
        "plan"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][1]["artifactPath"],
        "proof/memory/x06-models-multi-probe-plan.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][1]["status"],
        "multi-probe-plan-variant"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["planningProofs"][1]["runtimeMode"],
        "plan"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][0]["artifactPath"],
        "proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][0]["status"],
        "usable-local-chat"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][0]["ciParity"],
        false
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][0]["measuredTokensPerSecond"],
        101.9
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][1]["artifactPath"],
        "proof/memory/x06-models-chat-auto-gpu.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][1]["status"],
        "usable-local-chat-auto-select"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][1]["measuredTokensPerSecond"],
        107.8
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][2]["status"],
        "below-chat-floor"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][3]["artifactPath"],
        "proof/memory/x06-models-gemma3-4b-vulkan-live.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][3]["status"],
        "usable-local-chat"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["localRuntimeProofs"][3]["measuredTokensPerSecond"],
        78.3
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][0]["artifactPath"],
        "proof/memory/x06-models-qwen3-4b-download-local.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][0]["status"],
        "repo-local-cache-acquired"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][0]["runtimeMode"],
        "download"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][1]["artifactPath"],
        "proof/memory/x06-models-gemma3-4b-download-live.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][1]["status"],
        "repo-local-cache-acquired"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][1]["runtimeMode"],
        "download"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][2]["artifactPath"],
        "proof/memory/x06-models-qwen3-embedding-download.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][2]["status"],
        "repo-local-cache-acquired"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][2]["runtimeMode"],
        "download"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][3]["artifactPath"],
        "proof/memory/x06-models-qwen3-reranker-download.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][3]["status"],
        "repo-local-cache-acquired"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["cacheAcquisitionProofs"][3]["runtimeMode"],
        "download"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ggufRuntimeProofs"][0]["artifactPath"],
        "proof/memory/x06-models-qwen3-embedding-gguf-vulkan-live.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ggufRuntimeProofs"][0]["status"],
        "usable-local-embedding"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ggufRuntimeProofs"][0]["runtimeMode"],
        "probe"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ortRuntimeProofs"][0]["artifactPath"],
        "proof/memory/x06-models-qwen3-embedding-ort-cpu.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ortRuntimeProofs"][0]["status"],
        "usable-local-embedding"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ortRuntimeProofs"][1]["artifactPath"],
        "proof/memory/x06-models-qwen3-reranker-ort-cpu.json"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["ortRuntimeProofs"][1]["status"],
        "usable-local-reranker"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["negativeLearningProofs"][0]["observationKind"],
        "artifact-hash-mismatch"
    );
    assert_eq!(
        proof["linkedProofArtifacts"]["negativeLearningProofs"][1]["observationKind"],
        "tokenizer-hash-mismatch"
    );
    assert_eq!(chat_plan["runtimeMode"], "plan");
    assert_eq!(chat_plan["allowNetwork"], false);
    assert_eq!(chat_plan["probeExecutionPolicy"]["allowMultiProbe"], false);
    assert_eq!(
        chat_plan["probeExecutionPolicy"]["selectedProbes"][0],
        "chat-generation-gguf"
    );
    assert_eq!(
        chat_plan["chatModelSelection"]["requestedAcceleration"],
        "gpu"
    );
    assert_eq!(
        chat_plan["chatModelSelection"]["selection"]["selected"]["repoId"],
        "bartowski/google_gemma-3-4b-it-GGUF"
    );
    assert_eq!(multi_probe_plan["runtimeMode"], "plan");
    assert_eq!(multi_probe_plan["allowNetwork"], false);
    assert_eq!(
        multi_probe_plan["probeExecutionPolicy"]["allowMultiProbe"],
        true
    );
    assert_eq!(
        multi_probe_plan["probeExecutionPolicy"]["requestedProbeFilter"],
        "all"
    );
    assert_eq!(
        multi_probe_plan["probeExecutionPolicy"]["selectedProbes"],
        serde_json::json!([
            "chat-generation-gguf",
            "qwen-embedding-onnx",
            "qwen-reranker-onnx",
            "qwen-embedding-gguf"
        ])
    );
    assert_eq!(
        multi_probe_plan["probeExecutionPolicy"]["reason"],
        "multi-probe explicitly enabled"
    );
    Ok(())
}

#[test]
fn ort_runtime_uses_qwen_causal_lm_reranker_scoring_contract() {
    let runtime = include_str!("../src/ort_runtime.rs");

    assert_contract_terms(
        runtime,
        &[
            "Tensor::<f32>::new(",
            "Shape::new([1, QWEN3_KV_HEAD_COUNT as i64, 0, QWEN3_HEAD_DIM as i64])",
            "answer can only be \\\"yes\\\" or \\\"no\\\"",
            "token_to_id(\"yes\")",
            "token_to_id(\"no\")",
            "let last_token_offset = (active_seq_len - 1) * vocab_size;",
            "let yes_exp = (yes_logit - max_logit).exp();",
        ],
    );
}

#[test]
fn checked_in_reranker_runtime_proof_shows_relevance_lift() -> TestResult {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-qwen3-reranker-ort-cpu.json"
    ))?;
    let ranked = proof["qwenRerankerOnnx"]["ranked"]
        .as_array()
        .ok_or("reranker proof missing ranked rows")?;

    assert_eq!(
        ranked.first().and_then(|row| row["docId"].as_str()),
        Some("relevant")
    );
    assert_eq!(
        ranked.get(1).and_then(|row| row["docId"].as_str()),
        Some("irrelevant")
    );
    assert!(
        ranked[0]["score"].as_f64().unwrap_or_default()
            > ranked[1]["score"].as_f64().unwrap_or_default(),
        "reranker proof must show relevant score above irrelevant score"
    );
    Ok(())
}

#[test]
fn real_model_probe_defaults_to_one_probe_and_requires_multi_probe_opt_in() {
    let probe = include_str!("../src/runtime_probe.rs");
    let script = include_str!("../scripts/x06-real-model-proof.ps1");

    assert_contract_terms(
        probe,
        &[
            "const DEFAULT_PROBE_FILTER: &str = \"chat\";",
            "\"oneModelAtATime\": plan.one_model_at_a_time",
            "\"cpuFirst\": plan.cpu_first",
            "\"gpuAndNpuRequireProviderProbe\": plan.gpu_and_npu_require_provider_probe",
            "\"killOnTimeout\": plan.kill_on_timeout",
            "\"providerProbeTimeoutMs\": plan.provider_probe_timeout_ms",
            "\"modelProbeTimeoutMs\": plan.model_probe_timeout_ms",
            "\"minimumChatTokensPerSecond\": plan.minimum_chat_tokens_per_second",
            "ENFORCER_X06_ALLOW_MULTI_PROBE",
            "\"cache-only\" | \"cache_only\" | \"cacheonly\"",
            "runtime_mode == \"probe\"",
            "\"reranker\" | \"ranker\" | \"reranker-onnx\"",
            "one model at a time; CPU first; GPU/NPU only after provider probes pass; timeout kills the child process",
        ],
    );
    assert_contract_terms(
        script,
        &[
            "[string]$Acceleration = 'cpu'",
            "[switch]$AllowMultiProbe",
            "[string]$LlamaServer = ''",
            "[string]$LlamaEmbedding = ''",
            "[string]$DownloadLlamaUrl",
            "[string]$DownloadLlamaArchiveName",
            "[string]$ImportLlamaToolchainPath = ''",
            "Invoke-WebRequest -Uri $DownloadLlamaUrl -OutFile $archivePath",
            "Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force",
            "Import-LlamaToolchain -SourcePath $extractDir -RepoRoot $RepoRoot",
            "$env:ENFORCER_X06_ALLOW_MULTI_PROBE",
        ],
    );
}

#[test]
fn real_model_probe_can_import_external_chat_assets_into_repo_model_cache() {
    let probe = include_str!("../src/runtime_probe.rs");
    let script = include_str!("../scripts/x06-real-model-proof.ps1");
    let llama_server_binary = llama_binary_name("llama-server");
    let llama_embedding_binary = llama_binary_name("llama-embedding");
    let llama_server_term = format!(
        "Find-RepoLlamaBinary -Root (Join-Path $RepoRoot 'model\\bin') -BinaryName '{}'",
        llama_server_binary
    );
    let llama_embedding_term = format!(
        "Find-RepoLlamaBinary -Root (Join-Path $RepoRoot 'model\\bin') -BinaryName '{}'",
        llama_embedding_binary
    );

    assert_contract_terms(
        probe,
        &[
            "ENFORCER_X06_CHAT_MODEL_PATH",
            "maybe_direct_chat_model_report",
            "\"providerProbePassed\":",
            "\"resolvedAcceleration\":",
            "llama_report_proof(input.repo_root, &report)",
            "llama_device_report_proof(context.repo_root, report)",
            "repo_relative_display(repo_root, &report.binary_path)",
            "repo_path_redacted_text(repo_root, &report.stdout_excerpt)",
            "hf_downloaded_files_proof(repo_root, &report.downloaded_files)",
            "ENFORCER_X06_LLAMA_SERVER",
            "/v1/embeddings",
            "--embeddings",
            "--pooling",
            "mean",
            "invalid argument: --embedding",
        ],
    );
    assert_contract_terms(
        script,
        &[
            "[string]$ImportChatModelPath",
            "[string]$ImportLlamaCliPath",
            "[string]$ImportLlamaToolchainPath",
            "$env:ENFORCER_X06_ORT_TIMEOUT_MS",
            "$env:ENFORCER_X06_LLAMA_SERVER",
            "$env:ENFORCER_X06_LLAMA_EMBEDDING",
            llama_server_term.as_str(),
            llama_embedding_term.as_str(),
            "Filter '*.dll'",
            "model\\local\\chat",
            "model\\bin",
        ],
    );
}

#[test]
fn onnx_metadata_expansion_includes_external_data_files() {
    let files = vec![
        HfRepoFile {
            path: "onnx/model.onnx".to_owned(),
            size: Some(10),
        },
        HfRepoFile {
            path: "onnx/model.onnx_data".to_owned(),
            size: Some(20),
        },
        HfRepoFile {
            path: "onnx/other.onnx.data".to_owned(),
            size: Some(30),
        },
    ];

    let resolved = resolve_onnx_external_data_files("onnx/model.onnx", &files);

    assert!(resolved.iter().any(|file| file.path == "onnx/model.onnx"));
    assert!(resolved
        .iter()
        .any(|file| file.path == "onnx/model.onnx_data"));
    assert!(!resolved
        .iter()
        .any(|file| file.path == "onnx/other.onnx.data"));
    assert!(!resolved.iter().any(|file| file.path == "tokenizer.json"));
}

#[test]
fn onnx_metadata_expansion_includes_dot_external_data_files() -> TestResult {
    let metadata = HfRepoMetadata {
        model_id: Some("test/split-onnx".to_owned()),
        siblings: vec![
            HfRepoFile {
                path: "onnx/model_fp16.onnx".to_owned(),
                size: Some(10),
            },
            HfRepoFile {
                path: "onnx/model_fp16.onnx.data".to_owned(),
                size: Some(20),
            },
            HfRepoFile {
                path: "tokenizer.json".to_owned(),
                size: Some(1),
            },
        ],
    };
    let spec = HfModelSpec::with_onnx_model_file(
        "test/split-onnx",
        "main",
        enforcer_memory::model_runtime::ModelTask::Embedding,
        "test/split-onnx",
        "onnx/model_fp16.onnx",
    );

    let expanded = expand_onnx_spec_from_metadata(spec, &metadata)?;

    assert!(expanded
        .files
        .iter()
        .any(|file| file.path == "onnx/model_fp16.onnx.data"));
    Ok(())
}

#[test]
fn hf_cache_paths_are_stable_and_safe() -> TestResult {
    validate_hf_repo_id("Qwen/Qwen3-Embedding-0.6B-GGUF")?;
    validate_hf_file_path("onnx/model_q4.onnx")?;
    assert_model_runtime_error(
        validate_hf_repo_id("../bad/repo"),
        "validate-hf-repo-id",
        "invalid Hugging Face repo id: \"../bad/repo\"",
    )?;
    assert_model_runtime_error(
        validate_hf_file_path("../secret"),
        "validate-hf-file-path",
        "unsafe Hugging Face file path: \"../secret\"",
    )?;
    assert_model_runtime_error(
        validate_hf_file_path("/absolute/model.onnx"),
        "validate-hf-file-path",
        "unsafe Hugging Face file path: \"/absolute/model.onnx\"",
    )?;

    let cache = model_cache_dir(
        std::path::Path::new("target/cache"),
        "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "main",
    );
    assert!(cache.ends_with(std::path::Path::new(
        "hf/Qwen--Qwen3-Embedding-0.6B-GGUF/main"
    )));
    Ok(())
}

#[test]
fn llama_cpp_validation_fails_closed_for_missing_assets() -> TestResult {
    let missing_llama_cli = llama_binary_name("missing-llama-cli");
    assert_model_runtime_error(
        validate_executable(std::path::Path::new(&missing_llama_cli)),
        "validate-llama-executable",
        &format!("llama.cpp executable not found: {missing_llama_cli}"),
    )?;
    assert_model_runtime_error(
        validate_model(std::path::Path::new("missing-model.gguf")),
        "validate-llama-model",
        "GGUF model not found: missing-model.gguf",
    )?;
    Ok(())
}

#[test]
fn llama_cpp_native_cpu_plan_forces_zero_gpu_layers() {
    let config = llama_plan_fixture(LocalRuntimeAcceleration::Cpu, LlamaCppBackendHint::Native);

    let plan = llama_cpp_command_plan(&config);

    assert!(contains_arg_pair(&plan.args, "-ngl", "0"));
    assert!(plan.env.is_empty());
}

#[test]
fn llama_cpp_native_gpu_plan_uses_gpu_layers() {
    let mut config = llama_plan_fixture(LocalRuntimeAcceleration::Gpu, LlamaCppBackendHint::Native);
    config.gpu_layers = Some(42);

    let plan = llama_cpp_command_plan(&config);

    assert!(contains_arg_pair(&plan.args, "-ngl", "42"));
    assert!(plan.env.is_empty());
}

#[test]
fn llama_cpp_gpu_plan_can_split_across_cpu_and_gpus_with_fit() {
    let mut config = llama_plan_fixture(LocalRuntimeAcceleration::Gpu, LlamaCppBackendHint::Native);
    config.device = Some("Vulkan0,Vulkan1".to_owned());
    config.main_gpu = Some(0);
    config.split_mode = Some("layer".to_owned());
    config.tensor_split = Some("4,1".to_owned());
    config.fit = Some(true);

    let plan = llama_cpp_command_plan(&config);

    assert!(contains_arg_pair(&plan.args, "--device", "Vulkan0,Vulkan1"));
    assert!(contains_arg_pair(&plan.args, "-ngl", "auto"));
    assert!(contains_arg_pair(&plan.args, "--main-gpu", "0"));
    assert!(contains_arg_pair(&plan.args, "--split-mode", "layer"));
    assert!(contains_arg_pair(&plan.args, "--tensor-split", "4,1"));
    assert!(contains_arg_pair(&plan.args, "--fit", "on"));
}

#[test]
fn llama_cpp_openvino_gpu_plan_uses_device_env_not_native_offload() {
    let config = llama_plan_fixture(LocalRuntimeAcceleration::Gpu, LlamaCppBackendHint::OpenVino);

    let plan = llama_cpp_command_plan(&config);

    assert!(plan
        .env
        .iter()
        .any(|(key, value)| key == "GGML_OPENVINO_DEVICE" && value == "GPU"));
    assert!(!plan.args.iter().any(|arg| arg == "-ngl"));
}

#[test]
fn llama_cpp_npu_plan_requires_openvino_device_env_and_small_context() {
    let config = llama_plan_fixture(LocalRuntimeAcceleration::Npu, LlamaCppBackendHint::Auto);

    let plan = llama_cpp_command_plan(&config);

    assert!(plan
        .env
        .iter()
        .any(|(key, value)| key == "GGML_OPENVINO_DEVICE" && value == "NPU"));
    assert!(contains_arg_pair(&plan.args, "-c", "512"));
}

#[test]
fn preseeded_hf_cache_resolves_without_network() -> TestResult {
    let temp = tempfile::TempDir::new()?;
    let spec = HfModelSpec::with_single_model_file(HfSingleFileSpecInput {
        repo_id: "custom-org/custom-chat-gguf".to_owned(),
        revision: "main".to_owned(),
        backend: enforcer_memory::local_runtime::LocalRuntimeBackend::LlamaCpp,
        task: enforcer_memory::model_runtime::ModelTask::Summarization,
        model_id: "custom-org/custom-chat-gguf".to_owned(),
        acceleration: enforcer_memory::local_runtime::LocalRuntimeAcceleration::Auto,
        file_path: "custom-chat-Q4_K_M.gguf".to_owned(),
    });
    let cache = model_cache_dir(temp.path(), &spec.repo_id, &spec.revision);
    std::fs::create_dir_all(&cache)?;
    std::fs::write(cache.join("custom-chat-Q4_K_M.gguf"), b"gguf bytes")?;

    let report = resolve_cached_hf_model(&spec, temp.path())?;
    let trusted_report = resolve_cached_hf_model_from_manifest(&spec, temp.path())?;

    assert_eq!(report.repo_id, spec.repo_id);
    assert_eq!(report.downloaded_files.len(), 1);
    assert!(report.manifest_path.is_file());
    assert!(report.downloaded_files[0].streaming_manifest_path.is_none());
    assert_eq!(
        trusted_report.downloaded_files[0].sha256,
        report.downloaded_files[0].sha256
    );

    let manifest_body = std::fs::read_to_string(&report.manifest_path)?;
    std::fs::write(
        &report.manifest_path,
        manifest_body.replace(&report.downloaded_files[0].sha256, "unchecked"),
    )?;
    assert!(
        resolve_cached_hf_model_from_manifest(&spec, temp.path()).is_err(),
        "manifest resolver must not pass unchecked hashes into runtime validation"
    );
    Ok(())
}

fn llama_plan_fixture(
    acceleration: LocalRuntimeAcceleration,
    backend_hint: LlamaCppBackendHint,
) -> LlamaCppProbeConfig {
    LlamaCppProbeConfig {
        binary_path: llama_binary_name("llama-cli").into(),
        model_path: "model.gguf".into(),
        model_sha256: None,
        prompt: "hello".to_owned(),
        kind: LlamaCppProbeKind::Generate,
        backend_hint,
        acceleration,
        gpu_layers: None,
        device: None,
        main_gpu: None,
        split_mode: None,
        tensor_split: None,
        fit: None,
        context_size: None,
        max_tokens: 1,
        timeout_ms: 1_000,
    }
}

fn contains_arg_pair(args: &[String], key: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0].as_str() == key && pair[1].as_str() == value)
}
