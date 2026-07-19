use enforcer_domain::memory_types::{
    LlamaCppBackendHint, LlamaCppLifecycleAction, LlamaCppLifecycleState, LlamaCppProbeKind,
    LocalRuntimeAcceleration, RuntimeActivityState, RuntimeOwnershipMode, RuntimeRequestProtocol,
};
use enforcer_memory::llama_cpp::{
    llama_cpp_command_plan, parse_generation_rate, LlamaCppDeviceDto, LlamaCppDeviceReportDto,
    LlamaCppExecutionResolutionDto, LlamaCppLifecycleTransitionDto, LlamaCppProbeConfigDto,
    LlamaCppProbeReportDto,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

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
) -> LlamaCppProbeConfigDto {
    LlamaCppProbeConfigDto {
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
fn generation_plan_is_single_turn_and_subprocess_safe() -> TestResult {
    let config = build_probe_config(LocalRuntimeAcceleration::Cpu, LlamaCppBackendHint::Native);

    let plan = llama_cpp_command_plan(&config)?;

    assert!(plan.args.iter().any(|arg| arg == "-st"));
    assert!(plan.args.iter().any(|arg| arg == "--simple-io"));
    assert!(plan.args.iter().any(|arg| arg == "--no-display-prompt"));
    Ok(())
}

#[test]
fn auto_acceleration_defaults_to_cpu_first() -> TestResult {
    let config = build_probe_config(LocalRuntimeAcceleration::Auto, LlamaCppBackendHint::Native);

    let plan = llama_cpp_command_plan(&config)?;

    assert!(contains_arg_pair(&plan.args, "-ngl", "0"));
    assert!(plan.env.is_empty());
    Ok(())
}

#[test]
fn openvino_auto_acceleration_keeps_cpu_device_selection() -> TestResult {
    let config = build_probe_config(
        LocalRuntimeAcceleration::Auto,
        LlamaCppBackendHint::OpenVino,
    );

    let plan = llama_cpp_command_plan(&config)?;

    assert!(plan
        .env
        .iter()
        .any(|(key, value)| key == "GGML_OPENVINO_DEVICE" && value == "CPU"));
    assert!(!plan.args.iter().any(|arg| arg == "-ngl"));
    Ok(())
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

#[test]
fn llama_cpp_dto_shapes_round_trip_through_json() -> TestResult {
    let config: LlamaCppProbeConfigDto =
        build_probe_config(LocalRuntimeAcceleration::Cpu, LlamaCppBackendHint::Native);
    assert_json_round_trip(&config)?;
    assert_json_round_trip(&LlamaCppProbeReportDto {
        kind: LlamaCppProbeKind::Generate,
        backend_hint: LlamaCppBackendHint::Native,
        requested_acceleration: LocalRuntimeAcceleration::Cpu,
        binary_path: "llama-cli".into(),
        execution_route: "managed-subprocess".to_owned(),
        model_path: "model.gguf".into(),
        exit_code: Some(0),
        stdout_excerpt: "ok".to_owned(),
        stderr_excerpt: String::new(),
        duration_ms: 10,
        measured_tokens_per_second: Some(20.0),
        load_state: "loaded".to_owned(),
        timed_out: false,
        fallback_reason: None,
        fallback_from_binary_path: None,
        output_dimensions: None,
    })?;
    assert_json_round_trip(&LlamaCppLifecycleTransitionDto {
        before: LlamaCppLifecycleState::Idle,
        action: LlamaCppLifecycleAction::ResolveToolchain,
        after: LlamaCppLifecycleState::ToolchainReady,
        activity: RuntimeActivityState::Idle,
        ownership: RuntimeOwnershipMode::EnforcerSubprocess,
        request_protocol: RuntimeRequestProtocol::EnforcerStdio,
        execution_route: "managed-subprocess".to_owned(),
        external_server_allowed: false,
        port_binding_allowed: false,
        kill_on_timeout: true,
        reason: "toolchain resolved".to_owned(),
    })?;
    let device = LlamaCppDeviceDto {
        id: "0".to_owned(),
        name: "CPU".to_owned(),
        total_memory_mib: 1_024,
        free_memory_mib: 512,
    };
    assert_json_round_trip(&device)?;
    assert_json_round_trip(&LlamaCppDeviceReportDto {
        binary_path: "llama-cli".into(),
        devices: vec![device],
        stderr_excerpt: String::new(),
        timed_out: false,
    })?;
    assert_json_round_trip(&LlamaCppExecutionResolutionDto {
        requested_backend_hint: LlamaCppBackendHint::Native,
        requested_acceleration: LocalRuntimeAcceleration::Cpu,
        backend_hint: LlamaCppBackendHint::Native,
        resolved_acceleration: LocalRuntimeAcceleration::Cpu,
        provider_probe_passed: true,
        selected_device_id: Some("0".to_owned()),
        selected_main_gpu: None,
        detected_free_vram_mib: None,
        downgrade_reason: None,
    })?;
    Ok(())
}
