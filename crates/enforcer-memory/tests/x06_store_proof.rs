type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn checked_in_store_proof_pins_required_capabilities_without_fake_green() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-store.json"))?;

    assert_eq!(proof["namedTest"], "memory-store-core");
    assert_eq!(proof["result"]["testsFailed"], 0);
    assert_eq!(proof["fakeGreenCheck"]["status"], "PASS");
    assert_eq!(
        proof["fakeGreenCheck"]["realStreamIntegrationTest"],
        "tests/store_real_stream.rs real_stream_file_round_trips_through_the_store"
    );

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or("x06-store hardRequirements must be an object")?;
    for requirement in [
        "appendOnlyLogsWithSha256Chain",
        "sqliteOperationalReadModel",
        "contentAddressedArtifactManifest",
        "indexManifestsWithSourceHighWatermark",
        "corruptionDetectionAndQuarantine",
        "noGhostProjectDatabaseCreation",
        "windowsSafePathNormalization",
        "proceduralObservationNativeLog",
        "routeChoiceNativeLog",
        "modelRuntimeObservationNativeLog",
        "storeBackedLearningProjection",
        "legacyObservationPayloadFallbacks",
    ] {
        assert_eq!(
            hard_requirements
                .get(requirement)
                .and_then(|entry| entry["status"].as_str()),
            Some("DONE"),
            "{requirement} must stay proven DONE in x06-store"
        );
    }

    assert_eq!(
        proof["hardRequirements"]["duckdbAnalyticsReadModel"]["status"],
        "DEFERRED_WITH_TRAIT_FALLBACK",
        "DuckDB analytics must stay explicitly deferred unless a real gated backend is proven"
    );

    let hard_tests = proof["hardTests"]
        .as_object()
        .ok_or("x06-store hardTests must be an object")?;
    for (test_key, expected) in [
        (
            "proceduralRouteReplay",
            "crates/enforcer-memory/tests/unit_observations.rs procedural_and_route_records_replay_from_store",
        ),
        (
            "modelRuntimeObservationPersistence",
            "crates/enforcer-memory/tests/model_observations.rs model_runtime_observation_persists_to_store_and_graph",
        ),
        (
            "modelRuntimeObservationReplay",
            "crates/enforcer-memory/tests/model_observations.rs store_backed_model_runtime_observation_replays_without_duplicate_writes",
        ),
        (
            "learningProjection",
            "crates/enforcer-memory/tests/unit_learning.rs store_learning_projection_replays_observations_into_curves",
        ),
        (
            "continuousLearningProjection",
            "crates/enforcer-memory/tests/continuous_learning.rs store_backed_learning_projection_replays_t0_t1_t2",
        ),
    ] {
        assert_eq!(
            hard_tests.get(test_key).and_then(|entry| entry.as_str()),
            Some(expected),
            "{test_key} must pin the exact Store-backed proof test"
        );
    }

    let legacy_fallbacks = hard_tests
        .get("legacyObservationFallbacks")
        .and_then(|entry| entry.as_str())
        .ok_or("legacyObservationFallbacks proof must name fallback tests")?;
    assert!(
        legacy_fallbacks.contains("legacy_observation_payloads_replay_when_native_logs_are_empty")
            && legacy_fallbacks.contains(
                "projection_falls_back_to_legacy_observation_payloads_when_native_log_is_empty"
            ),
        "legacyObservationFallbacks must pin procedural/route and model fallback coverage"
    );
    Ok(())
}
