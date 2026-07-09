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

fn assert_green_artifact(
    proof: &serde_json::Value,
    artifact: &str,
    named_test: &str,
    requirements: &[&str],
) -> TestResult {
    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], artifact);
    assert_eq!(proof["status"], "green");
    assert_eq!(proof["namedTest"], named_test);
    assert_eq!(proof["result"]["testsFailed"], 0);

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or_else(|| format!("{artifact} hardRequirements must be an object"))?;
    for requirement in requirements {
        assert_eq!(
            hard_requirements
                .get(*requirement)
                .and_then(serde_json::Value::as_str),
            Some("covered"),
            "{artifact} requirement {requirement} must remain covered"
        );
    }
    assert_eq!(
        hard_requirements.len(),
        requirements.len(),
        "{artifact} should not gain unreviewed hard requirements without this proof test changing"
    );
    Ok(())
}

fn assert_evidence(
    proof: &serde_json::Value,
    artifact: &str,
    expected_tests: &[&str],
) -> TestResult {
    let evidence = string_set(&proof["result"]["evidenceTests"], "evidenceTests")?;
    for test_name in expected_tests {
        assert!(
            evidence.contains(*test_name),
            "{artifact} missing required evidence test {test_name}"
        );
    }
    assert_eq!(
        evidence.len(),
        expected_tests.len(),
        "{artifact} should not gain unreviewed evidence tests without this proof test changing"
    );
    Ok(())
}

#[test]
fn checked_in_cli_proof_pins_cli_mirror_and_exit_code_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-cli.json"))?;

    assert_green_artifact(
        &proof,
        "x06-cli",
        "x06-cli",
        &[
            "mcpEnvelopeParity",
            "strictExitCodes",
            "jsonMode",
            "compactDiagnostics",
        ],
    )?;
    assert_evidence(
        &proof,
        "x06-cli",
        &[
            "mcp_cli_live::cli_mirror_produces_the_same_envelope_json_as_the_mcp_tools_call_path",
            "mcp_cli_live::cli_run_cli_exit_codes_are_strictly_0_or_1",
            "mcp_cli_live::cli_json_mode_prints_the_same_raw_envelope_cli_invoke_returns",
            "mcp_cli_live::unknown_method_yields_method_not_found_with_id_echoed",
            "mcp_cli_live::malformed_json_yields_a_parse_error_with_id_zero",
        ],
    )
}

#[test]
fn checked_in_mcp_proof_pins_live_tool_and_ort_fallback_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-mcp.json"))?;

    assert_green_artifact(
        &proof,
        "x06-mcp",
        "x06-mcp",
        &[
            "jsonRpcHandshake",
            "toolListPagination",
            "fourteenBaselineTools",
            "x06ModelRuntimeStatusTool",
            "liveToolCalls",
            "typedToolErrors",
            "searchGraphSemanticRuntimeTelemetry",
            "cacheOnlyOrtEmbeddingFallback",
            "ortProviderSelectionTelemetry",
            "ortFallbackKindTelemetry",
        ],
    )?;
    assert_eq!(proof["result"]["toolsAdvertised"], 15);
    assert_eq!(proof["result"]["baselineToolsAdvertised"], 14);
    assert_evidence(
        &proof,
        "x06-mcp",
        &[
            "mcp_cli_live::initialize_handshake_returns_protocol_version_capabilities_and_server_info",
            "mcp_cli_live::tools_list_across_both_pages_covers_baseline_plus_x06_extension_tools",
            "mcp_cli_live::tools_call_model_runtime_status_reports_managed_zero_network_contract",
            "mcp_cli_live::tools_call_get_graph_schema_on_a_real_fixture_repo_returns_structured_content",
            "mcp_cli_live::tools_call_index_repository_on_a_real_fixture_repo_reports_files_indexed",
            "mcp_cli_live::tools_call_search_graph_regex_mode_returns_real_matches",
            "mcp_cli_live::tools_call_search_graph_semantic_mode_returns_a_separate_semantic_results_list",
            "mcp_cli_live::tools_call_search_graph_ort_embedding_missing_cache_falls_back_without_network",
            "mcp_cli_live::tools_call_trace_path_data_flow_mode_reports_call_graph_only_approximation",
            "mcp_cli_live::tools_call_ingest_traces_merges_a_real_runtime_trace_into_the_call_graph",
        ],
    )
}

#[test]
fn checked_in_weaver_proof_pins_async_indexing_and_dead_letter_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-weaver.json"))?;

    assert_green_artifact(
        &proof,
        "x06-weaver",
        "x06-weaver",
        &[
            "eventDrivenQueue",
            "semanticIndexerWorker",
            "summaryInvalidation",
            "deadLetterQueue",
            "boundedRetry",
            "foregroundQueryNotBlocked",
        ],
    )?;
    assert_evidence(
        &proof,
        "x06-weaver",
        &[
            "unit_weaver::node_created_event_triggers_embedding_task",
            "unit_weaver::file_changed_event_invalidates_cached_summary",
            "unit_weaver::index_report_translates_into_file_and_node_events",
            "weaver_enrichment::failed_task_enters_dead_letter_after_retries_exhausted",
            "weaver_enrichment::retry_succeeds_on_transient_failure",
            "weaver_enrichment::queue_does_not_block_foreground_query",
        ],
    )
}
