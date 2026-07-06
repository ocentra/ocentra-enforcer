use enforcer_memory::hf_cache::{
    expand_onnx_spec_from_metadata, model_cache_dir, resolve_cached_hf_model,
    resolve_cached_hf_model_from_manifest, resolve_onnx_external_data_files,
    select_x06_chat_model_for_hardware, validate_hf_file_path, validate_hf_repo_id, HfModelSpec,
    HfRepoFile, HfRepoMetadata, HfSingleFileSpecInput, X06ModelLineup,
};
use enforcer_memory::llama_cpp::{
    llama_cpp_command_plan, parse_llama_cpp_devices, validate_executable, validate_model,
    LlamaCppBackendHint, LlamaCppProbeConfig, LlamaCppProbeKind,
};
use enforcer_memory::local_runtime::LocalRuntimeAcceleration;
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
    assert!(bad.validate().is_err());
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
    assert!(selection.reason.contains("7484"));
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
    assert!(crawl.reason.contains("chat not usable"));

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
        (
            "x06_model_runtime_probe.rs",
            include_str!("../examples/x06_model_runtime_probe.rs"),
        ),
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
            "x06-models-qwen3-embedding-download.json",
            include_str!("../../../proof/memory/x06-models-qwen3-embedding-download.json"),
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
        include_str!("../../../proof/memory/x06-models-qwen3-4b-download-local.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-4b-cpu-windows-local.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-embedding-download.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-embedding-ort-cpu.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-reranker-download.json"),
        include_str!("../../../proof/memory/x06-models-qwen3-reranker-ort-cpu.json"),
    ];

    for body in proof_files {
        let proof: serde_json::Value = serde_json::from_str(body)?;
        assert_eq!(proof["proofScope"]["ciParity"], false);
        assert_ne!(proof["proofScope"]["portability"], "portable-ci-proof");
    }
    Ok(())
}

#[test]
fn portable_plan_proof_does_not_probe_local_hardware() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-models.json"))?;

    assert_eq!(proof["runtimeMode"], "plan");
    assert_eq!(proof["proofScope"]["portability"], "portable-contract");
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert!(proof["chatModelSelection"]["deviceReport"].is_null());
    assert_eq!(proof["cacheRoot"], "<repo>/model");
    Ok(())
}

#[test]
fn ort_runtime_uses_qwen_causal_lm_reranker_scoring_contract() {
    let runtime = include_str!("../src/ort_runtime.rs");

    assert!(runtime.contains("Tensor::<f32>::new("));
    assert!(
        runtime.contains("Shape::new([1, QWEN3_KV_HEAD_COUNT as i64, 0, QWEN3_HEAD_DIM as i64])")
    );
    assert!(runtime.contains("answer can only be \\\"yes\\\" or \\\"no\\\""));
    assert!(runtime.contains("token_to_id(\"yes\")"));
    assert!(runtime.contains("token_to_id(\"no\")"));
    assert!(runtime.contains("let last_token_offset = (active_seq_len - 1) * vocab_size;"));
    assert!(runtime.contains("let yes_exp = (yes_logit - max_logit).exp();"));
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
    let probe = include_str!("../examples/x06_model_runtime_probe.rs");
    let script = include_str!("../scripts/x06-real-model-proof.ps1");

    assert!(probe.contains("const DEFAULT_PROBE_FILTER: &str = \"chat\";"));
    assert!(probe.contains("ENFORCER_X06_ALLOW_MULTI_PROBE"));
    assert!(probe.contains("\"reranker\" | \"ranker\" | \"reranker-onnx\""));
    assert!(
        probe.contains("one real model probe at a time by default"),
        "probe proof should explain why broad model launches are disabled by default"
    );
    assert!(script.contains("[switch]$AllowMultiProbe"));
    assert!(script.contains("$env:ENFORCER_X06_ALLOW_MULTI_PROBE"));
}

#[test]
fn real_model_probe_can_import_external_chat_assets_into_repo_model_cache() {
    let probe = include_str!("../examples/x06_model_runtime_probe.rs");
    let script = include_str!("../scripts/x06-real-model-proof.ps1");

    assert!(probe.contains("ENFORCER_X06_CHAT_MODEL_PATH"));
    assert!(probe.contains("maybe_direct_chat_model_report"));
    assert!(script.contains("[string]$ImportChatModelPath"));
    assert!(script.contains("[string]$ImportLlamaCliPath"));
    assert!(script.contains("$env:ENFORCER_X06_ORT_TIMEOUT_MS"));
    assert!(script.contains("Filter '*.dll'"));
    assert!(script.contains("model\\local\\chat"));
    assert!(script.contains("model\\bin"));
    assert!(probe.contains("llama_report_proof(repo_root, &report)"));
    assert!(probe.contains("llama_device_report_proof(repo_root, report)"));
    assert!(probe.contains("repo_relative_display(repo_root, &report.binary_path)"));
    assert!(probe.contains("repo_path_redacted_text(repo_root, &report.stdout_excerpt)"));
    assert!(probe.contains("hf_downloaded_files_proof(repo_root, &report.downloaded_files)"));
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
    assert!(validate_hf_repo_id("../bad/repo").is_err());
    assert!(validate_hf_file_path("../secret").is_err());
    assert!(validate_hf_file_path("/absolute/model.onnx").is_err());

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
fn llama_cpp_validation_fails_closed_for_missing_assets() {
    assert!(validate_executable(std::path::Path::new("missing-llama-cli.exe")).is_err());
    assert!(validate_model(std::path::Path::new("missing-model.gguf")).is_err());
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
        binary_path: "llama-cli.exe".into(),
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
