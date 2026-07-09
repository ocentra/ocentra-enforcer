use std::collections::BTreeMap;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn string_array(
    value: &serde_json::Value,
    name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{name} must be an array"))?;
    array
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{name} must contain only strings").into())
        })
        .collect()
}

#[test]
fn checked_in_kg_proof_pins_one_store_backed_graph_system() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-kg.json"))?;

    assert_eq!(proof["status"], "green");
    assert_eq!(
        proof["canonicalDataSource"]["sourceOfTruth"],
        "Store append logs plus manifest"
    );
    assert_eq!(
        proof["canonicalDataSource"]["separateKgSystemCreated"], false,
        "X06 must not drift into a second graph system beside Store"
    );

    let projections = string_array(
        &proof["canonicalDataSource"]["derivedProjections"],
        "derivedProjections",
    )?;
    for projection in [
        "CodeGraph",
        "MemoryGraph",
        "TraceStore",
        "summaries",
        "search documents",
        "SQLite operational graph",
        "learning curves",
        "model observations",
    ] {
        assert!(
            projections.iter().any(|entry| entry == projection),
            "{projection} must stay a derived Store projection"
        );
    }

    let writers = string_array(&proof["implementedStoreWriters"], "implementedStoreWriters")?;
    for writer in [
        "append_observation_entry",
        "append_graph_event_entry",
        "append_model_observation",
        "append_trace_records",
        "record_procedural_in_store",
        "record_route_choice_in_store",
    ] {
        assert!(
            writers.iter().any(|entry| entry == writer),
            "{writer} must stay wired as a Store writer"
        );
    }

    let replay = string_array(&proof["replayProof"], "replayProof")?;
    for proof_name in [
        "replay_incident_observations_from_store",
        "replay_procedural_and_routes_from_store",
        "project_model_runtime_observations_from_store",
        "replay_trace_records_from_store",
        "store_backed_projection_rebuilds_from_a_real_code_graph_fixture",
        "query_graph_prefers_store_projection_when_stores_dir_is_available",
    ] {
        assert!(
            replay.iter().any(|entry| entry == proof_name),
            "{proof_name} must stay represented in replay proof"
        );
    }

    let parity = &proof["baselineParity"];
    assert_eq!(parity["artifactPath"], "proof/memory/x06-kg-parity.json");
    assert_eq!(parity["baselineExecuted"], true);
    assert_eq!(parity["toolsTotal"], 23);
    assert_eq!(parity["toolsWorse"], 0);
    assert_eq!(parity["toolsUnrunnable"], 0);
    Ok(())
}

#[test]
fn checked_in_kg_parity_counts_match_rows_without_fake_green() -> TestResult {
    let parity: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-kg-parity.json"))?;

    assert_eq!(parity["baseline_executed"], true);
    assert_eq!(parity["tools_total"], 23);
    assert_eq!(parity["tools_worse"], 0);
    assert_eq!(parity["tools_unrunnable"], 0);

    let rows = parity["rows"]
        .as_array()
        .ok_or("x06-kg-parity rows must be an array")?;
    assert_eq!(
        rows.len(),
        parity["tools_total"].as_u64().unwrap_or_default() as usize
    );

    let mut counts = BTreeMap::<String, u64>::new();
    for row in rows {
        let tool = row["tool"]
            .as_str()
            .ok_or("each parity row must name the compared tool")?;
        assert!(!tool.trim().is_empty());

        let verdict = row["comparison_verdict"]
            .as_str()
            .ok_or("each parity row must include a comparison verdict")?;
        *counts.entry(verdict.to_owned()).or_default() += 1;
        assert_ne!(verdict, "worse", "{tool} must not regress below baseline");
        assert_ne!(
            verdict, "unrunnable",
            "{tool} must not be counted as parity green"
        );
    }

    assert_eq!(counts.get("equal").copied().unwrap_or_default(), 18);
    assert_eq!(counts.get("better").copied().unwrap_or_default(), 3);
    assert_eq!(counts.get("incomparable").copied().unwrap_or_default(), 2);
    assert_eq!(counts.values().sum::<u64>(), 23);
    Ok(())
}
