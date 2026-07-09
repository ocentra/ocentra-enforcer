use std::collections::BTreeSet;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn proof_json(path: &str, raw: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let value =
        serde_json::from_str(raw).map_err(|error| format!("failed to parse {path}: {error}"))?;
    Ok(value)
}

#[test]
fn checked_in_sharded_test_proofs_replace_monolithic_memory_gate() -> TestResult {
    let discovery = proof_json(
        "proof/memory/x06-test-shards.json",
        include_str!("../../../proof/memory/x06-test-shards.json"),
    )?;
    let local_exec = proof_json(
        "proof/memory/x06-test-shards-local-exec.json",
        include_str!("../../../proof/memory/x06-test-shards-local-exec.json"),
    )?;
    let rollup = proof_json(
        "proof/memory/x06-test-shards-rollup.json",
        include_str!("../../../proof/memory/x06-test-shards-rollup.json"),
    )?;

    assert_eq!(discovery["schemaVersion"], 1);
    assert_eq!(discovery["artifact"], "x06-test-shards");
    assert_eq!(discovery["package"], "enforcer-memory");
    assert_eq!(discovery["testRoot"], "crates/enforcer-memory/tests");
    assert_eq!(discovery["totalTargets"], 232);
    assert_eq!(discovery["selectedTargets"], 232);
    assert_eq!(discovery["result"]["mode"], "discovery-only");
    assert_eq!(discovery["result"]["ok"], true);
    assert_eq!(discovery["result"]["executedTargets"], 0);

    for (category, expected_count) in [
        ("integration", 10),
        ("x06-proof", 4),
        ("model-runtime", 5),
        ("parity-live", 4),
        ("unit-core", 56),
        ("unit-languages", 153),
    ] {
        assert_eq!(
            discovery["byCategory"][category], expected_count,
            "discovery category {category} count drifted"
        );
    }
    for policy in [
        "deterministicDiscovery",
        "crossPlatform",
        "oneCargoTestTargetPerProcess",
        "avoidsMonolithicPackageTimeout",
        "zeroNetwork",
    ] {
        assert_eq!(
            discovery["executionPolicy"][policy], true,
            "sharded test proof must keep execution policy {policy}"
        );
    }

    assert_eq!(local_exec["only"], "local");
    assert_eq!(local_exec["selectedTargets"], 1);
    assert_eq!(local_exec["result"]["mode"], "executed");
    assert_eq!(local_exec["result"]["ok"], true);
    assert_eq!(local_exec["result"]["executedTargets"], 1);
    assert_eq!(local_exec["targets"][0]["target"], "local_runtime");
    assert_eq!(local_exec["targets"][0]["category"], "model-runtime");

    assert_eq!(rollup["artifact"], "x06-test-shards-rollup");
    assert_eq!(rollup["package"], "enforcer-memory");
    assert_eq!(rollup["shardCount"], 8);
    assert_eq!(rollup["totalTargets"], 232);
    assert_eq!(rollup["uniqueExecutedTargets"], 232);
    assert_eq!(rollup["executedTargets"], 232);
    assert_eq!(rollup["allShardsOk"], true);
    assert_eq!(rollup["failedTargets"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        rollup["replacesMonolithicGate"],
        "cargo test -p enforcer-memory --quiet -j 1"
    );

    let shards = rollup["shards"]
        .as_array()
        .ok_or("x06-test-shards-rollup shards must be an array")?;
    assert_eq!(shards.len(), 8);
    let mut proof_paths = BTreeSet::new();
    let mut shard_ids = BTreeSet::new();
    for shard in shards {
        let shard_id = shard["shard"]
            .as_str()
            .ok_or("rollup shard id must be a string")?;
        shard_ids.insert(shard_id.to_owned());
        assert_eq!(shard["ok"], true, "shard {shard_id} must remain green");
        assert_eq!(
            shard["selectedTargets"], 29,
            "shard {shard_id} selected count drifted"
        );
        assert_eq!(
            shard["executedTargets"], 29,
            "shard {shard_id} executed count drifted"
        );
        assert_eq!(
            shard["failedTargets"], 0,
            "shard {shard_id} must have no failures"
        );
        let proof_path = shard["proof"]
            .as_str()
            .ok_or("rollup shard proof path must be a string")?;
        assert!(
            proof_path.starts_with("proof/memory/x06-test-shards-")
                && proof_path.ends_with(".json")
                && Path::new(proof_path).is_relative(),
            "shard {shard_id} proof path must be portable repo-relative JSON, got {proof_path}"
        );
        proof_paths.insert(proof_path.to_owned());
    }

    assert_eq!(
        shard_ids,
        BTreeSet::from([
            "1/8".to_owned(),
            "2/8".to_owned(),
            "3/8".to_owned(),
            "4/8".to_owned(),
            "5/8".to_owned(),
            "6/8".to_owned(),
            "7/8".to_owned(),
            "8/8".to_owned(),
        ])
    );
    assert_eq!(proof_paths.len(), 8);
    Ok(())
}
