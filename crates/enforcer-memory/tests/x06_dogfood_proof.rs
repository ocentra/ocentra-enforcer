use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn string_array<'a>(
    value: &'a serde_json::Value,
    name: &str,
) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{name} must be an array"))?;
    array
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| format!("{name} must contain only strings").into())
        })
        .collect()
}

#[test]
fn checked_in_dogfood_proof_pins_current_enforcer_and_learning_evidence() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-dogfood.json"))?;

    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], "x06-dogfood");
    assert_eq!(proof["hub"], "ocentra-enforcer");
    assert_eq!(proof["stateRoot"], "<ledger>/ocentra-enforcer");
    assert_eq!(proof["lane"], "codex-x06-harvest-sync");

    let green_gates = string_array(&proof["greenGates"], "greenGates")?;
    assert!(
        green_gates.len() >= 200,
        "dogfood proof should keep broad focused-gate evidence, got {} gates",
        green_gates.len()
    );
    for required_marker in [
        "ocentra_enforcer_mcp_status ok/hash-compatible/write-compatible",
        "ocentra_enforcer_coordination_health operation=commit ok",
        "ocentra_enforcer_doctor scope=crate crateName=enforcer-memory ok",
        "cargo test -p enforcer-memory ok",
        "cargo test -p enforcer-memory --features ort-models",
        "cargo clippy -p enforcer-memory --all-targets -- -D warnings",
        "cargo fmt --package enforcer-memory --check",
        "node scripts/x06-enforcer-memory-sharded-test.mjs --shard N/8",
        "x06-rag-qa.json records 250/250 QA rows green",
        "ORT worker",
        "llama",
    ] {
        assert!(
            green_gates
                .iter()
                .any(|gate| gate.contains(required_marker)),
            "x06-dogfood greenGates missing required marker {required_marker}"
        );
    }

    let lessons = proof["lessons"]
        .as_array()
        .ok_or("x06-dogfood lessons must be an array")?;
    assert!(
        lessons.len() >= 200,
        "dogfood proof should preserve durable learning evidence, got {} lessons",
        lessons.len()
    );

    let lesson_shapes = lessons
        .iter()
        .filter_map(|lesson| lesson["shape"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        lesson_shapes,
        BTreeSet::from(["t0", "t1", "t2"]),
        "dogfood lessons must retain t0/t1/t2 evidence shape"
    );

    for required_learning_signal in [
        "ort-provider-resolution-must-travel-with-owned-worker-plan",
        "provider-downgrade-tests-need-exact-reasons",
        "checked-in-runtime-proof-artifacts-must-track-harness-contract",
        "default-qa-must-separate-host-local-proof-from-live-loaded",
        "scanner-backed-checks-must-preserve-explicit-file-scope",
    ] {
        assert!(
            lessons.iter().any(|lesson| {
                lesson["learningSignal"].as_str() == Some(required_learning_signal)
            }),
            "x06-dogfood lessons missing required learning signal {required_learning_signal}"
        );
    }

    assert!(
        lessons.iter().any(|lesson| {
            lesson["incident"].as_str().is_some_and(|incident| {
                incident.contains("monolithic cargo test -p enforcer-memory")
            }) && lesson["incident"]
                .as_str()
                .is_some_and(|incident| incident.contains("232 integration test targets"))
        }),
        "dogfood lessons must preserve the monolithic-suite sharding incident"
    );

    assert!(
        lessons.iter().any(|lesson| {
            lesson["evidence"].as_str().is_some_and(|evidence| {
                evidence.contains("x06-test-shards-rollup.json")
                    && evidence.contains("232 unique executed targets")
            })
        }),
        "dogfood lessons must preserve sharded-test proof evidence"
    );
    Ok(())
}
