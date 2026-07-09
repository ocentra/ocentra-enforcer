//! Local runtime adapter contract tests.
//!
//! These are fixture-backed contract tests only. They do not run any
//! model inference and they never claim parity from the deterministic
//! fallback.

use enforcer_memory::error::MemoryError;
use enforcer_memory::local_runtime::{
    arbitrate_runtime_workload, onnx_ort_feature_compiled, provider_order, validate_control_plane,
    validate_fixture, BackendReadiness, LocalRuntimeControlPlane, LocalRuntimeFixture,
    LocalRuntimeKind, RuntimeActivityState, RuntimeAdmission, RuntimeManagedCapability,
    RuntimeWorkload, REQUIRED_MANAGED_CAPABILITIES,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
    assert!(!ort.spawn_controlled);
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
