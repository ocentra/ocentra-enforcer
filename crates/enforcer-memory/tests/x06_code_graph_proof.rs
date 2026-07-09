use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn string_set(
    value: &serde_json::Value,
    name: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
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
fn checked_in_code_graph_proof_pins_multilanguage_graph_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-code-graph.json"))?;

    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], "x06-code-graph");
    assert_eq!(proof["status"], "green");
    assert_eq!(proof["namedTest"], "x06-code-graph");
    assert_eq!(proof["result"]["testsFailed"], 0);

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or("x06-code-graph hardRequirements must be an object")?;
    for (requirement, expected_status) in [
        ("fileModulePackageNodes", "covered by code graph fixtures"),
        ("functionsTypesTestsRoutes", "covered"),
        ("importsAndCalls", "covered"),
        ("textOnlyFallback", "covered"),
        (
            "multiLanguageIndexing",
            "covered by fixture repo plus live parity rows",
        ),
    ] {
        assert_eq!(
            hard_requirements
                .get(requirement)
                .and_then(serde_json::Value::as_str),
            Some(expected_status),
            "x06-code-graph requirement {requirement} must keep its exact coverage proof"
        );
    }
    assert_eq!(
        hard_requirements.len(),
        5,
        "x06-code-graph should not gain unreviewed hard requirements without this proof test changing"
    );

    let evidence = string_set(
        &proof["result"]["evidenceTests"],
        "code graph evidenceTests",
    )?;
    for test_name in [
        "code_graph_indexer::full_fixture_repo_indexes_every_supported_language_plus_text_only",
        "unit_code_graph::symbol_extraction_produces_function_type_test_nodes",
        "unit_code_graph::route_extraction_produces_route_edges",
        "unit_code_graph::import_and_call_edges_are_recorded",
        "unit_code_graph::unsupported_extension_becomes_text_only_node_not_skipped",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-code-graph missing required evidence test {test_name}"
        );
    }
    assert_eq!(
        evidence.len(),
        5,
        "x06-code-graph should not gain unreviewed evidence tests without this proof test changing"
    );
    Ok(())
}
