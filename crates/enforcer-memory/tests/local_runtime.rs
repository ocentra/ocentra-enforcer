//! Local runtime adapter contract tests.
//!
//! These are fixture-backed contract tests only. They do not run any
//! model inference and they never claim parity from the deterministic
//! fallback.

use enforcer_memory::error::MemoryError;
use enforcer_memory::local_runtime::{
    arbitrate_runtime_workload, onnx_ort_feature_compiled, ort_worker_command,
    ort_worker_execution_plan, ort_worker_execution_plan_with_provider_resolution,
    provider_from_env_value, provider_order, resolve_ort_provider, runtime_backend_contract,
    validate_control_plane, validate_fixture, validate_ort_worker_execution_plan, BackendReadiness,
    LocalRuntimeControlPlane, LocalRuntimeFixture, LocalRuntimeKind, OrtWorkerLifecycleAction,
    OrtWorkerLifecycleState, OrtWorkerTask, RuntimeActivityState, RuntimeAdmission,
    RuntimeExecutionIsolation, RuntimeManagedCapability, RuntimeOwnershipMode,
    RuntimeRequestProtocol, RuntimeWorkload, REQUIRED_MANAGED_CAPABILITIES,
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
    let llama_lifecycle = &proof["runtimePolicy"]["llamaCpp"]["lifecycle"];
    assert_eq!(
        llama_lifecycle["stateMachine"],
        "enforcer-owned-llama-cpp-subprocess"
    );
    assert_eq!(llama_lifecycle["requestProtocol"], "enforcer-stdio");
    assert_eq!(llama_lifecycle["timeoutKillRequired"], true);
    assert_eq!(
        llama_lifecycle["externalServerRejectedBeforeTransition"],
        true
    );
    assert_eq!(llama_lifecycle["portBindingAllowedForParity"], false);
    assert_eq!(llama_lifecycle["invalidTransitionRejected"], true);
    for state in [
        "toolchain-ready",
        "model-loading",
        "chat-active",
        "embedding-active",
        "timed-out",
        "unloaded",
    ] {
        assert!(
            llama_lifecycle["states"]
                .as_array()
                .is_some_and(|states| states
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(state))),
            "missing llama.cpp lifecycle state {state}"
        );
    }
    for action in [
        "resolve-toolchain",
        "load-model",
        "start-chat",
        "start-embedding",
        "timeout-kill",
        "unload",
    ] {
        assert!(
            llama_lifecycle["actions"]
                .as_array()
                .is_some_and(|actions| actions
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(action))),
            "missing llama.cpp lifecycle action {action}"
        );
    }
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["externalServerAllowedForParity"],
        false
    );
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["requestProtocol"],
        "enforcer-worker-env"
    );
    assert_eq!(
        proof["runtimePolicy"]["onnxOrt"]["portBindingAllowedForParity"],
        false
    );
    let ort_env = proof["runtimePolicy"]["onnxOrt"]["childEnvironment"]
        .as_array()
        .ok_or("onnxOrt childEnvironment must be an array")?;
    for key in [
        "ENFORCER_X06_ORT_CHILD_TASK",
        "ENFORCER_X06_CHILD_PROVIDER",
        "ENFORCER_X06_CHILD_REQUESTED_PROVIDER",
        "ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS",
        "ENFORCER_X06_CHILD_PROVIDER_PROBE_PASSED",
        "ENFORCER_X06_CHILD_ARTIFACT_PATH",
        "ENFORCER_X06_CHILD_TOKENIZER_PATH",
        "ENFORCER_X06_ORT_TIMEOUT_MS",
    ] {
        assert!(
            ort_env.iter().any(|value| value.as_str() == Some(key)),
            "missing ORT child env key {key}"
        );
    }
    let lifecycle = &proof["runtimePolicy"]["onnxOrt"]["lifecycle"];
    assert_eq!(lifecycle["stateMachine"], "enforcer-owned-ort-worker");
    assert_eq!(lifecycle["timeoutKillRequired"], true);
    assert_eq!(lifecycle["externalPlanRejectedBeforeTransition"], true);
    assert_eq!(lifecycle["invalidTransitionRejected"], true);
    for state in [
        "loading",
        "ready",
        "paused-embedding",
        "timed-out",
        "unloaded",
    ] {
        assert!(
            lifecycle["states"].as_array().is_some_and(|states| states
                .iter()
                .any(|candidate| candidate.as_str() == Some(state))),
            "missing ORT lifecycle state {state}"
        );
    }
    for action in ["load", "pause", "resume", "timeout-kill", "unload"] {
        assert!(
            lifecycle["actions"]
                .as_array()
                .is_some_and(|actions| actions
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(action))),
            "missing ORT lifecycle action {action}"
        );
    }
    assert!(
        proof["learningSignals"].as_array().is_some_and(|signals| {
            signals
                .iter()
                .any(|signal| signal.as_str() == Some("llama-cpp-server-embedding-route-rejected"))
        }),
        "proof must record llama.cpp server-route rejection as a learning signal"
    );
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
    assert_eq!(
        plan.request_protocol,
        RuntimeRequestProtocol::EnforcerWorkerEnv
    );
    assert!(!plan.external_server_allowed);
    assert!(!plan.port_binding_allowed);
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
        plan.env_value("ENFORCER_X06_CHILD_REQUESTED_PROVIDER"),
        Some("open-vino")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS"),
        Some("open-vino,cpu")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_PROVIDER_PROBE_PASSED"),
        Some("true")
    );
    assert_eq!(
        plan.provider_resolution.resolved_provider,
        ProviderKind::OpenVino
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
fn ort_worker_command_uses_owned_worker_args_and_env_without_server_surface() -> TestResult {
    let spec = ModelSpec::qwen3_reranker(
        "model/hf/qwen-reranker/model.onnx",
        "abc123",
        "model/hf/qwen-reranker/tokenizer.json",
        "def456",
    );
    let plan = ort_worker_execution_plan(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Reranker,
        &spec,
        ProviderKind::Npu,
        45_000,
    )?;

    let command = ort_worker_command(&plan)?;
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        args,
        vec![
            "--x06-ort-worker",
            "--task",
            "reranker",
            "--provider",
            "npu"
        ]
    );
    assert_eq!(
        command.get_program().to_string_lossy(),
        "target/debug/x06_model_runtime_probe"
    );
    let env = command
        .get_envs()
        .filter_map(|(key, value)| {
            value.map(|value| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        env.get("ENFORCER_X06_ORT_CHILD_TASK").map(String::as_str),
        Some("reranker")
    );
    assert_eq!(
        env.get("ENFORCER_X06_CHILD_PROVIDER").map(String::as_str),
        Some("npu")
    );
    assert_eq!(
        env.get("ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS")
            .map(String::as_str),
        Some("npu,cpu")
    );
    assert_eq!(
        env.get("ENFORCER_X06_ORT_TIMEOUT_MS").map(String::as_str),
        Some("45000")
    );
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains("server") || arg.contains("port")),
        "ORT worker command must not expose an external server or port surface"
    );
    Ok(())
}

#[test]
fn ort_worker_plan_downgrades_unprobed_acceleration_before_child_spawn() -> TestResult {
    let spec = ModelSpec::qwen3_embedding(
        "model/hf/qwen/model.onnx",
        "abc123",
        "model/hf/qwen/tokenizer.json",
        "def456",
    );
    let resolution = resolve_ort_provider(ProviderKind::Npu, &[]);
    let plan = ort_worker_execution_plan_with_provider_resolution(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Embedding,
        &spec,
        resolution,
        30_000,
    )?;

    validate_ort_worker_execution_plan(&plan)?;
    assert_eq!(plan.provider, ProviderKind::Cpu);
    assert_eq!(plan.env_value("ENFORCER_X06_CHILD_PROVIDER"), Some("cpu"));
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_REQUESTED_PROVIDER"),
        Some("npu")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_AVAILABLE_PROVIDERS"),
        Some("cpu")
    );
    assert_eq!(
        plan.env_value("ENFORCER_X06_CHILD_PROVIDER_PROBE_PASSED"),
        Some("false")
    );
    assert_eq!(
        plan.provider_resolution.downgrade_reason.as_deref(),
        Some("requested ORT provider npu but provider probe did not report it available; downgraded to cpu")
    );
    Ok(())
}

#[test]
fn runtime_backend_contract_is_derived_from_typed_runtime_ownership() {
    let contract = runtime_backend_contract();
    assert_eq!(
        contract.llama_cpp.ownership,
        RuntimeOwnershipMode::EnforcerSubprocess
    );
    assert_eq!(
        contract.llama_cpp.execution_isolation,
        RuntimeExecutionIsolation::EnforcerManagedChildProcess
    );
    assert_eq!(
        contract.llama_cpp.request_protocol,
        RuntimeRequestProtocol::EnforcerStdio
    );
    assert!(!contract.llama_cpp.external_http_allowed);
    assert!(!contract.llama_cpp.port_binding_allowed);
    assert!(!contract.llama_cpp.server_surface_accepted_for_parity);
    assert_eq!(
        contract.llama_cpp.route,
        "enforcer-managed-llama-cpp-subprocess"
    );

    assert_eq!(
        contract.ort.ownership,
        RuntimeOwnershipMode::EnforcerIsolatedWorker
    );
    assert_eq!(
        contract.ort.execution_isolation,
        RuntimeExecutionIsolation::EnforcerIsolatedWorkerProcess
    );
    assert_eq!(
        contract.ort.request_protocol,
        RuntimeRequestProtocol::EnforcerWorkerEnv
    );
    assert!(!contract.ort.external_http_allowed);
    assert!(!contract.ort.port_binding_allowed);
    assert!(!contract.ort.server_surface_accepted_for_parity);
    assert_eq!(contract.ort.route, "enforcer-isolated-ort-worker");
    assert_eq!(
        contract.ort.managed_by_service,
        vec!["embeddings", "rerank"]
    );
}

#[test]
fn ort_worker_lifecycle_load_pause_resume_cancel_and_unload_are_owned() -> TestResult {
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
        ProviderKind::Cpu,
        30_000,
    )?;

    let load = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        OrtWorkerLifecycleState::Idle,
        OrtWorkerLifecycleAction::Load,
    )?;
    assert_eq!(load.after, OrtWorkerLifecycleState::Loading);
    assert_eq!(load.activity, RuntimeActivityState::Loading);
    assert_eq!(load.ownership, RuntimeOwnershipMode::EnforcerIsolatedWorker);
    assert_eq!(
        load.request_protocol,
        RuntimeRequestProtocol::EnforcerWorkerEnv
    );

    let ready = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        load.after,
        OrtWorkerLifecycleAction::MarkReady,
    )?;
    assert_eq!(ready.after, OrtWorkerLifecycleState::Ready);
    assert_eq!(ready.activity, RuntimeActivityState::Idle);

    let active = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        ready.after,
        OrtWorkerLifecycleAction::StartEmbedding,
    )?;
    assert_eq!(active.after, OrtWorkerLifecycleState::EmbeddingActive);
    assert_eq!(active.activity, RuntimeActivityState::EmbeddingActive);

    let paused = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        active.after,
        OrtWorkerLifecycleAction::Pause,
    )?;
    assert_eq!(paused.after, OrtWorkerLifecycleState::PausedEmbedding);
    assert_eq!(paused.activity, RuntimeActivityState::Paused);

    let resumed = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        paused.after,
        OrtWorkerLifecycleAction::Resume,
    )?;
    assert_eq!(resumed.after, OrtWorkerLifecycleState::EmbeddingActive);

    let cancelled = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        resumed.after,
        OrtWorkerLifecycleAction::Cancel,
    )?;
    assert_eq!(cancelled.after, OrtWorkerLifecycleState::Cancelled);
    assert_eq!(cancelled.activity, RuntimeActivityState::Idle);

    let unloaded = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        cancelled.after,
        OrtWorkerLifecycleAction::Unload,
    )?;
    assert_eq!(unloaded.after, OrtWorkerLifecycleState::Unloaded);
    assert_eq!(unloaded.activity, RuntimeActivityState::Idle);
    Ok(())
}

#[test]
fn ort_worker_lifecycle_timeout_kill_requires_owned_plan() -> TestResult {
    let spec = ModelSpec::qwen3_reranker(
        "model/hf/qwen-reranker/model.onnx",
        "abc123",
        "model/hf/qwen-reranker/tokenizer.json",
        "def456",
    );
    let mut plan = ort_worker_execution_plan(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Reranker,
        &spec,
        ProviderKind::Cpu,
        30_000,
    )?;

    let timeout = enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        OrtWorkerLifecycleState::RerankingActive,
        OrtWorkerLifecycleAction::TimeoutKill,
    )?;
    assert_eq!(timeout.after, OrtWorkerLifecycleState::TimedOut);
    assert!(timeout.kill_on_timeout);
    assert_eq!(
        timeout.reason,
        "Enforcer killed the owned ORT worker after timeout"
    );

    plan.request_protocol = RuntimeRequestProtocol::ExternalHttp;
    let err = match enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        OrtWorkerLifecycleState::RerankingActive,
        OrtWorkerLifecycleAction::TimeoutKill,
    ) {
        Ok(transition) => {
            return Err(format!("external HTTP lifecycle should fail, got {transition:?}").into());
        }
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-ort-worker-execution-plan",
            ..
        }
    ));
    Ok(())
}

#[test]
fn ort_worker_lifecycle_rejects_invalid_transition() -> TestResult {
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
        ProviderKind::Cpu,
        30_000,
    )?;

    let err = match enforcer_memory::local_runtime::transition_ort_worker_lifecycle(
        &plan,
        OrtWorkerLifecycleState::Idle,
        OrtWorkerLifecycleAction::StartEmbedding,
    ) {
        Ok(transition) => {
            return Err(format!("idle start should fail, got {transition:?}").into());
        }
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "transition-ort-worker-lifecycle",
            ..
        }
    ));
    Ok(())
}

#[test]
fn ort_worker_plan_rejects_external_http_or_port_binding() -> TestResult {
    let spec = ModelSpec::qwen3_embedding(
        "model/hf/qwen/model.onnx",
        "abc123",
        "model/hf/qwen/tokenizer.json",
        "def456",
    );
    let mut plan = ort_worker_execution_plan(
        "target/debug/x06_model_runtime_probe",
        OrtWorkerTask::Embedding,
        &spec,
        ProviderKind::Cpu,
        30_000,
    )?;

    plan.request_protocol = RuntimeRequestProtocol::ExternalHttp;
    let err = match validate_ort_worker_execution_plan(&plan) {
        Ok(()) => return Err("external HTTP ORT worker protocol should fail".into()),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-ort-worker-execution-plan",
            ..
        }
    ));

    plan.request_protocol = RuntimeRequestProtocol::EnforcerWorkerEnv;
    plan.port_binding_allowed = true;
    let err = match validate_ort_worker_execution_plan(&plan) {
        Ok(()) => return Err("ORT worker port binding should fail".into()),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        MemoryError::ModelRuntime {
            operation: "validate-ort-worker-execution-plan",
            ..
        }
    ));
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

#[test]
fn ort_provider_resolution_downgrades_unprobed_acceleration_to_cpu() {
    let resolution = resolve_ort_provider(ProviderKind::OpenVino, &[]);

    assert_eq!(resolution.requested_provider, ProviderKind::OpenVino);
    assert_eq!(resolution.resolved_provider, ProviderKind::Cpu);
    assert_eq!(resolution.available_providers, vec![ProviderKind::Cpu]);
    assert!(!resolution.provider_probe_passed);
    assert_eq!(
        resolution.downgrade_reason.as_deref(),
        Some(
            "requested ORT provider open-vino but provider probe did not report it available; downgraded to cpu"
        )
    );
}

#[test]
fn ort_provider_resolution_accepts_probed_npu_without_duplicate_cpu() {
    let resolution = resolve_ort_provider(
        ProviderKind::Npu,
        &[ProviderKind::Npu, ProviderKind::Npu, ProviderKind::Cpu],
    );

    assert_eq!(resolution.requested_provider, ProviderKind::Npu);
    assert_eq!(resolution.resolved_provider, ProviderKind::Npu);
    assert_eq!(
        resolution.available_providers,
        vec![ProviderKind::Npu, ProviderKind::Cpu]
    );
    assert!(resolution.provider_probe_passed);
    assert_eq!(resolution.downgrade_reason, None);
}

#[test]
fn checked_in_ort_provider_policy_proof_matches_resolver_contract() -> TestResult {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-models-ort-provider-policy.json"
    ))?;

    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], "x06-models-ort-provider-policy");
    assert_eq!(proof["runtimeMode"], "plan");
    assert_eq!(proof["allowNetwork"], false);
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);
    assert_eq!(proof["backend"], "onnx");
    assert_eq!(proof["executionRoute"], "enforcer-isolated-ort-worker");
    assert_eq!(proof["ownership"], "enforcer-isolated-worker");
    assert_eq!(proof["requestProtocol"], "enforcer-worker-env");
    assert_eq!(proof["externalServerAllowed"], false);
    assert_eq!(proof["portBindingAllowed"], false);
    assert_eq!(proof["inProcessAllowedForParity"], false);

    let cpu = resolve_ort_provider(ProviderKind::Cpu, &[]);
    let gpu_downgrade = resolve_ort_provider(ProviderKind::OpenVino, &[]);
    let npu = resolve_ort_provider(
        ProviderKind::Npu,
        &[ProviderKind::Npu, ProviderKind::Npu, ProviderKind::Cpu],
    );
    let examples = proof["providerResolutionExamples"]
        .as_array()
        .ok_or("providerResolutionExamples must be an array")?;

    assert_provider_example(&examples[0], "cpu-default", &cpu);
    assert_provider_example(&examples[1], "gpu-unprobed-downgrade", &gpu_downgrade);
    assert_provider_example(&examples[2], "npu-probed", &npu);

    assert_eq!(proof["providerPolicy"]["cpu"]["probeRequired"], false);
    assert_eq!(
        proof["providerPolicy"]["cpu"]["usableWithoutHardwareProbe"],
        true
    );
    assert_eq!(proof["providerPolicy"]["gpu"]["probeRequired"], true);
    assert_eq!(
        proof["providerPolicy"]["gpu"]["fallbackProvider"],
        serde_json::json!("cpu")
    );
    assert_eq!(proof["providerPolicy"]["npu"]["probeRequired"], true);
    assert_eq!(
        proof["providerPolicy"]["npu"]["fallbackProvider"],
        serde_json::json!("cpu")
    );
    assert!(proof["providerPolicy"]["npu"]["openVinoCompatibility"]
        .as_str()
        .unwrap_or_default()
        .contains("provider probe reports npu/open-vino availability"));

    let signals = proof["learningSignals"]
        .as_array()
        .ok_or("learningSignals must be an array")?;
    for signal in [
        "ort-accelerated-provider-requires-positive-probe",
        "ort-unprobed-acceleration-downgraded-before-child-spawn",
        "ort-npu-admitted-only-with-provider-evidence",
        "ort-worker-owned-env-contract-required",
    ] {
        assert!(
            signals
                .iter()
                .any(|candidate| candidate.as_str() == Some(signal)),
            "missing ORT provider learning signal {signal}"
        );
    }
    Ok(())
}

fn assert_provider_example(
    example: &Value,
    case: &str,
    resolution: &enforcer_memory::local_runtime::OrtProviderResolution,
) {
    assert_eq!(example["case"], case);
    assert_eq!(
        example["requestedProvider"],
        serde_json::json!(provider_env_name(resolution.requested_provider))
    );
    assert_eq!(
        example["resolvedProvider"],
        serde_json::json!(provider_env_name(resolution.resolved_provider))
    );
    assert_eq!(
        example["providerProbePassed"],
        serde_json::json!(resolution.provider_probe_passed)
    );
    assert_eq!(
        example["downgradeReason"],
        resolution
            .downgrade_reason
            .as_deref()
            .map_or(serde_json::Value::Null, serde_json::Value::from)
    );
    let expected_available: Vec<serde_json::Value> = resolution
        .available_providers
        .iter()
        .map(|provider| serde_json::json!(provider_env_name(*provider)))
        .collect();
    assert_eq!(
        example["availableProviders"],
        serde_json::Value::Array(expected_available)
    );
}

fn provider_env_name(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Cpu => "cpu",
        ProviderKind::DirectMl => "direct-ml",
        ProviderKind::OpenVino => "open-vino",
        ProviderKind::Cuda => "cuda",
        ProviderKind::Vulkan => "vulkan",
        ProviderKind::CoreMl => "core-ml",
        ProviderKind::Npu => "npu",
    }
}
