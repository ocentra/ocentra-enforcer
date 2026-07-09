use enforcer_memory::llama_cpp::{
    llama_cpp_command_plan, parse_generation_rate, LlamaCppBackendHint, LlamaCppProbeConfig,
    LlamaCppProbeKind,
};
use enforcer_memory::local_runtime::LocalRuntimeAcceleration;

fn llama_binary_name(base_name: &str) -> String {
    format!("{base_name}{}", std::env::consts::EXE_SUFFIX)
}

fn contains_arg_pair(args: &[String], key: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0].as_str() == key && pair[1].as_str() == value)
}

fn build_probe_config(
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
        max_tokens: 8,
        timeout_ms: 1_000,
    }
}

#[test]
fn generation_plan_is_single_turn_and_subprocess_safe() {
    let config = build_probe_config(LocalRuntimeAcceleration::Cpu, LlamaCppBackendHint::Native);

    let plan = llama_cpp_command_plan(&config);

    assert!(plan.args.iter().any(|arg| arg == "-st"));
    assert!(plan.args.iter().any(|arg| arg == "--simple-io"));
    assert!(plan.args.iter().any(|arg| arg == "--no-display-prompt"));
}

#[test]
fn auto_acceleration_defaults_to_cpu_first() {
    let config = build_probe_config(LocalRuntimeAcceleration::Auto, LlamaCppBackendHint::Native);

    let plan = llama_cpp_command_plan(&config);

    assert!(contains_arg_pair(&plan.args, "-ngl", "0"));
    assert!(plan.env.is_empty());
}

#[test]
fn openvino_auto_acceleration_keeps_cpu_device_selection() {
    let config = build_probe_config(
        LocalRuntimeAcceleration::Auto,
        LlamaCppBackendHint::OpenVino,
    );

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
