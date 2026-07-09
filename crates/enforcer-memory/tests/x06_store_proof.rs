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
    Ok(())
}
