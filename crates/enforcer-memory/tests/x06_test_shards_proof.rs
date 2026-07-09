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
    let shard_files = [
        (
            "1/8",
            "proof/memory/x06-test-shards-1-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-1-of-8.json"),
        ),
        (
            "2/8",
            "proof/memory/x06-test-shards-2-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-2-of-8.json"),
        ),
        (
            "3/8",
            "proof/memory/x06-test-shards-3-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-3-of-8.json"),
        ),
        (
            "4/8",
            "proof/memory/x06-test-shards-4-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-4-of-8.json"),
        ),
        (
            "5/8",
            "proof/memory/x06-test-shards-5-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-5-of-8.json"),
        ),
        (
            "6/8",
            "proof/memory/x06-test-shards-6-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-6-of-8.json"),
        ),
        (
            "7/8",
            "proof/memory/x06-test-shards-7-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-7-of-8.json"),
        ),
        (
            "8/8",
            "proof/memory/x06-test-shards-8-of-8.json",
            include_str!("../../../proof/memory/x06-test-shards-8-of-8.json"),
        ),
    ];
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
    assert_eq!(discovery["totalTargets"], 245);
    assert_eq!(discovery["selectedTargets"], 245);
    assert_eq!(discovery["result"]["mode"], "discovery-only");
    assert_eq!(discovery["result"]["ok"], true);
    assert_eq!(discovery["result"]["executedTargets"], 0);

    for (category, expected_count) in [
        ("integration", 10),
        ("x06-proof", 17),
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
    assert_eq!(rollup["totalTargets"], 245);
    assert_eq!(rollup["uniqueExecutedTargets"], 245);
    assert_eq!(rollup["executedTargets"], 245);
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
    let shard_proofs = shard_files
        .iter()
        .map(|(shard_id, path, raw)| {
            let proof = proof_json(path, raw)?;
            Ok(((*shard_id).to_owned(), (*path).to_owned(), proof))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let mut proof_paths = BTreeSet::new();
    let mut shard_ids = BTreeSet::new();
    let mut executed_target_ids = BTreeSet::new();
    for shard in shards {
        let shard_id = shard["shard"]
            .as_str()
            .ok_or("rollup shard id must be a string")?;
        shard_ids.insert(shard_id.to_owned());
        assert_eq!(shard["ok"], true, "shard {shard_id} must remain green");
        let selected_targets = shard["selectedTargets"]
            .as_u64()
            .ok_or_else(|| format!("shard {shard_id} selectedTargets must be a number"))?;
        assert!(
            (30..=31).contains(&selected_targets),
            "shard {shard_id} selected count drifted outside balanced 245-target shard range"
        );
        assert_eq!(
            shard["executedTargets"], selected_targets,
            "shard {shard_id} executed count must match selected count"
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

        let (_, _, shard_proof) = shard_proofs
            .iter()
            .find(|(id, path, _)| id == shard_id && path == proof_path)
            .ok_or_else(|| format!("missing checked-in proof body for shard {shard_id}"))?;
        assert_eq!(shard_proof["schemaVersion"], 1);
        assert_eq!(shard_proof["artifact"], "x06-test-shards");
        assert_eq!(shard_proof["package"], "enforcer-memory");
        assert_eq!(shard_proof["testRoot"], "crates/enforcer-memory/tests");
        assert_eq!(shard_proof["totalTargets"], 245);
        assert_eq!(shard_proof["selectedTargets"], shard["selectedTargets"]);
        assert_eq!(shard_proof["shard"], shard_id);
        assert_eq!(shard_proof["only"], serde_json::Value::Null);
        assert_eq!(shard_proof["result"]["mode"], "executed");
        assert_eq!(shard_proof["result"]["ok"], true);
        assert_eq!(
            shard_proof["result"]["executedTargets"],
            shard["executedTargets"]
        );
        assert_eq!(
            shard_proof["result"]["failedTargets"]
                .as_array()
                .map(Vec::len),
            Some(0),
            "shard {shard_id} proof must carry no failed targets"
        );
        for policy in [
            "deterministicDiscovery",
            "crossPlatform",
            "oneCargoTestTargetPerProcess",
            "avoidsMonolithicPackageTimeout",
            "zeroNetwork",
        ] {
            assert_eq!(
                shard_proof["executionPolicy"][policy], true,
                "shard {shard_id} must keep execution policy {policy}"
            );
        }
        let targets = shard_proof["targets"]
            .as_array()
            .ok_or_else(|| format!("shard {shard_id} targets must be an array"))?;
        assert_eq!(
            targets.len(),
            shard["selectedTargets"].as_u64().unwrap_or_default() as usize
        );
        for target in targets {
            let target_name = target["target"]
                .as_str()
                .ok_or_else(|| format!("shard {shard_id} target name must be a string"))?;
            assert!(
                executed_target_ids.insert(target_name.to_owned()),
                "target {target_name} appeared in more than one shard"
            );
            assert_eq!(target["cargoArgs"][0], "test");
            assert_eq!(target["cargoArgs"][1], "-p");
            assert_eq!(target["cargoArgs"][2], "enforcer-memory");
            assert_eq!(target["cargoArgs"][3], "--test");
            assert_eq!(target["cargoArgs"][4], target_name);
            assert!(
                target["category"].as_str().is_some_and(|category| {
                    matches!(
                        category,
                        "integration"
                            | "x06-proof"
                            | "model-runtime"
                            | "parity-live"
                            | "unit-core"
                            | "unit-languages"
                    )
                }),
                "shard {shard_id} target {target_name} has unexpected category"
            );
        }
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
    assert_eq!(
        executed_target_ids.len(),
        245,
        "per-shard proof bodies must cover every discovered target exactly once"
    );
    Ok(())
}
