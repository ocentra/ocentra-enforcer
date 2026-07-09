use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn json_f64(value: &serde_json::Value, field: &str) -> Result<f64, Box<dyn std::error::Error>> {
    value[field]
        .as_f64()
        .ok_or_else(|| format!("{field} must be a JSON number").into())
}

#[test]
fn checked_in_longitudinal_proof_recomputes_local_benchmark_gates() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-longitudinal.json"))?;

    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["proofScope"]["artifact"], "x06-longitudinal");
    assert_eq!(proof["proofScope"]["capability"], "local-benchmark-proof");
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["indexing"]["changeSetPercent"], 0.0);
    assert_eq!(proof["indexing"]["incrementalWinsAllTiers"], true);
    assert_eq!(proof["retrievalLatency"]["passesRegressionGate"], true);

    let samples = proof["indexing"]["samples"]
        .as_array()
        .ok_or("indexing samples must be an array")?;
    assert!(
        samples.len() >= 3,
        "longitudinal indexing proof must cover at least three synthetic tiers"
    );

    let mut file_counts = BTreeSet::new();
    let mut minimum_speedup = f64::INFINITY;
    for sample in samples {
        let file_count = sample["fileCount"]
            .as_u64()
            .ok_or("indexing sample must include fileCount")?;
        assert!(
            file_counts.insert(file_count),
            "duplicate longitudinal fileCount tier {file_count}"
        );

        let full = json_f64(sample, "fullIndexMs")?;
        let incremental = json_f64(sample, "incrementalNoopIndexMs")?;
        assert!(full > 0.0, "fullIndexMs must be positive");
        assert!(incremental > 0.0, "incrementalNoopIndexMs must be positive");
        assert!(
            incremental < full,
            "incremental indexing must beat full rebuild for {file_count} files"
        );

        let recomputed = (full / incremental * 10.0).round() / 10.0;
        assert_eq!(
            json_f64(sample, "speedup")?,
            recomputed,
            "speedup must be recomputed from sample timings"
        );
        minimum_speedup = minimum_speedup.min(recomputed);
    }
    assert_eq!(
        json_f64(&proof["indexing"], "minimumSpeedup")?,
        minimum_speedup
    );

    let tiers = proof["retrievalLatency"]["tiers"]
        .as_array()
        .ok_or("retrievalLatency tiers must be an array")?;
    let baseline = tiers
        .iter()
        .find(|tier| tier["name"] == "baseline")
        .ok_or("retrievalLatency must include a baseline tier")?;
    let large = tiers
        .iter()
        .find(|tier| tier["name"] == "large-synthetic")
        .ok_or("retrievalLatency must include a large-synthetic tier")?;
    let p50_ratio =
        (json_f64(large, "p50Ms")? / json_f64(baseline, "p50Ms")? * 100.0).round() / 100.0;
    let p95_ratio =
        (json_f64(large, "p95Ms")? / json_f64(baseline, "p95Ms")? * 100.0).round() / 100.0;

    assert_eq!(
        json_f64(&proof["retrievalLatency"], "p50RegressionRatio")?,
        p50_ratio
    );
    assert_eq!(
        json_f64(&proof["retrievalLatency"], "p95RegressionRatio")?,
        p95_ratio
    );
    assert!(p50_ratio <= json_f64(&proof["retrievalLatency"], "maxAllowedP50RegressionRatio")?);
    assert!(p95_ratio <= json_f64(&proof["retrievalLatency"], "maxAllowedP95RegressionRatio")?);

    let observation_kinds: BTreeSet<String> = proof["observations"]
        .as_array()
        .ok_or("longitudinal observations must be an array")?
        .iter()
        .filter_map(|entry| entry["candidate"]["observationKind"].as_str())
        .map(str::to_owned)
        .collect();
    for required in [
        "longitudinal-indexing-proof",
        "longitudinal-retrieval-latency-proof",
    ] {
        assert!(
            observation_kinds.contains(required),
            "missing longitudinal learning observation {required}"
        );
    }
    Ok(())
}
