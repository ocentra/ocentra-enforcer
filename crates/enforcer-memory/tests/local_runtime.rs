//! Local runtime adapter contract tests.
//!
//! These are fixture-backed contract tests only. They do not run any
//! model inference and they never claim parity from the deterministic
//! fallback.

use enforcer_memory::error::MemoryError;
use enforcer_memory::local_runtime::{
    arbitrate_runtime_workload, onnx_ort_feature_compiled, ort_worker_execution_plan,
    provider_from_env_value, provider_order, validate_control_plane, validate_fixture,
    validate_ort_worker_execution_plan, BackendReadiness, LocalRuntimeControlPlane,
    LocalRuntimeFixture, LocalRuntimeKind, OrtWorkerTask, RuntimeActivityState, RuntimeAdmission,
    RuntimeManagedCapability, RuntimeOwnershipMode, RuntimeWorkload, REQUIRED_MANAGED_CAPABILITIES,
};
use enforcer_memory::model_runtime::{ModelSpec, ProviderKind};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn checked_in_runtime_control_plane_proof_matches_contract() -> TestResult {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-runtime-control-plane.json"
    ))?;
    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["status"], "degraded-pass-evidence");
    assert_eq!(proof["proofScope"]["networkRequired"], false);
    assert_eq!(
        proof["runtimePolicy"]["llamaCpp"]["requiredOwnership"],
        "enforcer-subprocess"
    );
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["requiredOwnership"],
        "enforcer-isolated-worker"
    );
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["inProcessAllowedForParity"],
        false
    );
    assert_eq!(
        proof["runtimePolicy"]["llamaCpp"]["externalServerAllowedForParity"],
        false
    );
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["externalServerAllowedForParity"],
        false
    );
    let ort_env = proof["runtimePolicy"]["onnxOrt"]["childEnvironment"]
        .as_array()
        .ok_or("onnxOrt childEnvironment must be an array")?;
    for key in [
        "ENFORCER_X06_ORT_CHILD_TASK",
        "ENFORCER_X06_CHILD_PROVIDER",
        "ENFORCER_X06_CHILD_ARTIFACT_PATH",
        "ENFORCER_X06_CHILD_TOKENIZER_PATH",
        "ENFORCER_X06_ORT_TIMEOUT_MS",
    ] {
        assert!(
            ort_env.iter().any(|value| value.as_str() == Some(key)),
            "missing ORT child env key {key}"
        );
    }
    assert!(
        proof["learningSignals"]
            .as_array()
            .is_some_and(|signals| signals
                .iter()
                .any(|signal| signal.as_str() == Some("ort-in-process-parity-rejected"))),
        "proof must record ORT in-process rejection as a learning signal"
    );
    Ok(())
}

#[test]
fn provider_order_prefers_explicit_llama_then_optional_ort_then_fallback() -> TestResult {
    let fixture = LocalRuntimeFixture {
        preferred_backend: Some(LocalRuntimeKind::LlamaCpp),
        llama_cpp: BackendReadiness::new(true, true),
        onnx_ort: BackendReadiness::new(true, true),
        output: Some(vec![0.75, 0.25]),
        parity_claimed: false,
    };

    let report = validate_fixture(&fixture, 2)?;

    let mut expected = vec![LocalRuntimeKind::LlamaCpp];
    if onnx_ort_feature_compiled() {
        expected.push(LocalRuntimeKind::OnnxOrt);
    }
    expected.push(LocalRuntimeKind::DeterministicFallback);

    assert_eq!(provider_order(&fixture), expected);
    assert_eq!(report.ordered_backends, expected);
    assert_eq!(report.selected_backend, LocalRuntimeKind::LlamaCpp);
    assert!(report.real_backend_selected);
    Ok(())
}

#[test]
fn feature_gate_truth_matches_compile_configuration() {
    assert_eq!(onnx_ort_feature_compiled(), cfg!(feature = "ort-models"));
}

#[test]
fn invalid_output_is_rejected_by_the_fixture_validator() -> TestResult {
    let fixture = LocalRuntimeFixture {
        preferred_backend: Some(LocalRuntimeKind::LlamaCpp),
        llama_cpp: BackendReadiness::new(true, true),
        onnx_ort: BackendReadiness::new(false, false),
        output: Some(vec![0.5]),
        parity_claimed: false,
    };

    let err = match validate_fixture(&fixture, 2) {
        Ok(report) => {
            return Err(format!("output length mismatch should fail, got {report:?}").into());
        }
        Err(err) => err,
    };

    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-output",
            ..
        }
    ));
    Ok(())
}

#[test]
fn deterministic_fallback_cannot_be_claimed_as_parity() -> TestResult {
    let fixture = LocalRuntimeFixture {
        preferred_backend: None,
        llama_cpp: BackendReadiness::new(false, false),
        onnx_ort: BackendReadiness::new(false, false),
        output: None,
        parity_claimed: true,
    };

    let err = match validate_fixture(&fixture, 1) {
        Ok(report) => {
            return Err(format!("fallback parity claim should fail, got {report:?}").into());
        }
        Err(err) => err,
    };

    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-fixture",
            ..
        }
    ));
    Ok(())
}

#[test]
fn x06_runtime_control_plane_accepts_managed_llama_and_ort() -> TestResult {
    let llama = LocalRuntimeControlPlane::llama_cpp_managed();
    let ort = LocalRuntimeControlPlane::onnx_ort_managed();

    validate_control_plane(&llama)?;
    validate_control_plane(&ort)?;
    assert_eq!(
        llama.managed_capabilities,
        REQUIRED_MANAGED_CAPABILITIES.to_vec()
    );
    assert_eq!(
        ort.managed_capabilities,
        REQUIRED_MANAGED_CAPABILITIES.to_vec()
    );
    assert_eq!(ort.ownership, RuntimeOwnershipMode::EnforcerIsolatedWorker);
    assert!(ort.spawn_controlled);
    assert!(ort.timeout_kill_supported);
    assert!(ort
        .managed_capabilities
        .contains(&RuntimeManagedCapability::ProviderSelection));
    Ok(())
}

#[test]
fn x06_runtime_control_plane_rejects_external_server_ownership() -> TestResult {
    let err = match validate_control_plane(&LocalRuntimeControlPlane::externally_owned_server(
        LocalRuntimeKind::LlamaCpp,
    )) {
        Ok(()) => return Err("external server ownership should not pass parity control".into()),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-control-plane",
            ..
        }
    ));
    Ok(())
}

#[test]
fn x06_runtime_control_plane_rejects_backend_wrong_ownership_mode() -> TestResult {
    let mut ort = LocalRuntimeControlPlane::onnx_ort_managed();
    ort.ownership = RuntimeOwnershipMode::EnforcerInProcess;
    let err = match validate_control_plane(&ort) {
        Ok(()) => return Err("ORT in-process ownership should not pass parity control".into()),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-control-plane",
            ..
        }
    ));

    let mut llama = LocalRuntimeControlPlane::llama_cpp_managed();
    llama.ownership = RuntimeOwnershipMode::EnforcerIsolatedWorker;
    let err = match validate_control_plane(&llama) {
        Ok(()) => {
            return Err("llama.cpp non-subprocess ownership should not pass parity control".into())
        }
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-control-plane",
            ..
        }
    ));
    Ok(())
}

#[test]
fn x06_runtime_control_plane_rejects_missing_managed_capability() -> TestResult {
    let mut control = LocalRuntimeControlPlane::llama_cpp_managed();
    control
        .managed_capabilities
        .retain(|capability| *capability != RuntimeManagedCapability::ChatHistoryPolicy);

    let err = match validate_control_plane(&control) {
        Ok(()) => return Err("missing chat history policy capability should fail".into()),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-local-runtime-control-plane",
            ..
        }
    ));
    Ok(())
}

#[test]
fn runtime_arbitration_admits_idle_workloads() {
    let decision =
        arbitrate_runtime_workload(RuntimeActivityState::Idle, RuntimeWorkload::Embedding);

    assert_eq!(decision.admission, RuntimeAdmission::Admit);
    assert_eq!(decision.requested, RuntimeWorkload::Embedding);
}

#[test]
fn runtime_arbitration_keeps_model_load_exclusive() {
    let decision = arbitrate_runtime_workload(RuntimeActivityState::Loading, RuntimeWorkload::Chat);

    assert_eq!(decision.admission, RuntimeAdmission::Queue);
    assert_eq!(
        decision.reason,
        "model load is exclusive; queue requested workload"
    );
}

#[test]
fn runtime_arbitration_chat_preempts_background_retrieval() {
    let embedding =
        arbitrate_runtime_workload(RuntimeActivityState::EmbeddingActive, RuntimeWorkload::Chat);
    let reranking =
        arbitrate_runtime_workload(RuntimeActivityState::RerankingActive, RuntimeWorkload::Chat);

    assert_eq!(
        embedding.admission,
        RuntimeAdmission::PauseBackgroundThenAdmit
    );
    assert_eq!(
        reranking.admission,
        RuntimeAdmission::PauseBackgroundThenAdmit
    );
}

#[test]
fn runtime_arbitration_chat_queues_background_work() {
    let decision =
        arbitrate_runtime_workload(RuntimeActivityState::ChatActive, RuntimeWorkload::Reranking);

    assert_eq!(decision.admission, RuntimeAdmission::Queue);
    assert_eq!(
        decision.reason,
        "chat has priority; queue background model work"
    );
}

#[test]
fn ort_worker_plan_materializes_enforcer_owned_child_env_contract() -> TestResult {
    let spec = ModelSpec::qwen3_embedding(
        "model/hf/qwen/model.onnx",
        "abc123",
        "model/hf/qwen/tokenizer.json",
        "def456",
    );
    let plan = ort_worker_execution_plan(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Embedding,
        &spec,
        ProviderKind::OpenVino,
        30_000,
    )?;

    validate_ort_worker_execution_plan(&plan)?;
    assert_eq!(plan.ownership, RuntimeOwnershipMode::EnforcerIsolatedWorker);
    assert!(plan.kill_on_timeout);
    assert_eq!(
        plan.env_value("ENFORCER_X06_ORT_CHILD_TASK"),
        Some("embedding")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_PROVIDER"),
        Some("open-vino")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_ARTIFACT_PATH"),
        Some("model/hf/qwen/model.onnx")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_TOKENIZER_PATH"),
        Some("model/hf/qwen/tokenizer.json")
    );
    assert_eq!(plan.env_value("ENFORCER_X06_ORT_TIMEOUT_MS"), Some("30000"));
    Ok(())
}

#[test]
fn ort_worker_plan_rejects_zero_timeout() -> TestResult {
    let spec = ModelSpec::qwen3_reranker(
        "model/hf/qwen-reranker/model.onnx",
        "abc123",
        "model/hf/qwen-reranker/tokenizer.json",
        "def456",
    );

    let err = match ort_worker_execution_plan(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Reranker,
        &spec,
        ProviderKind::Cpu,
        0,
    ) {
        Ok(plan) => return Err(format!("zero-timeout ORT plan should fail, got {plan:?}").into()),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "build-ort-worker-execution-plan",
            ..
        }
    ));
    Ok(())
}

#[test]
fn ort_provider_env_roundtrip_covers_cpu_gpu_and_npu_names() {
    assert_eq!(provider_from_env_value("cpu"), Some(ProviderKind::Cpu));
    assert_eq!(provider_from_env_value("cuda"), Some(ProviderKind::Cuda));
    assert_eq!(
        provider_from_env_value("direct-ml"),
        Some(ProviderKind::DirectMl)
    );
    assert_eq!(
        provider_from_env_value("open-vino"),
        Some(ProviderKind::OpenVino)
    );
    assert_eq!(provider_from_env_value("npu"), Some(ProviderKind::Npu));
    assert_eq!(provider_from_env_value("llama-server"), None);
}
