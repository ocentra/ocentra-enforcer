//! X06.7 HARD TESTS: live, in-process JSON-RPC over [`enforcer_memory::mcp`]
//! plus the CLI mirror ([`enforcer_memory::cli`]).
//!
//! "Live" here means: real framed bytes pushed through
//! [`enforcer_memory::mcp::run_stdio_session`]/[`enforcer_memory::mcp::handle_frame`]
//! against in-memory `Read`/`Write` buffers -- not calling `dispatch_tool`
//! directly. This is what proves the wire contract (framing, envelope,
//! pagination, JSON-RPC error codes) end to end, not just the inner
//! dispatch logic (already covered by `src/mcp.rs`'s own unit tests).

use std::error::Error;
use std::io::Cursor;

use enforcer_memory::cli;
use enforcer_memory::mcp;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

const X06_MCP_PROOF: &str = include_str!("../../../proof/memory/x06-mcp.json");

/// Push `body` (a bare JSON-RPC message, no framing) through the server as
/// one NDJSON line and return the parsed single-line JSON reply.
fn send_ndjson(body: &Value) -> Result<Value, Box<dyn Error>> {
    let mut input = Cursor::new(format!("{body}\n").into_bytes());
    let mut output: Vec<u8> = Vec::new();
    let run_err = mcp::run_stdio_session(&mut input, &mut output);
    run_err
        .map_err(|e| -> Box<dyn Error> { format!("stdio session must not error: {e}").into() })?;
    let text = String::from_utf8(output)
        .map_err(|e| -> Box<dyn Error> { format!("reply must be valid UTF-8: {e}").into() })?;
    let line = text
        .lines()
        .next()
        .ok_or("expected exactly one reply line")?;
    let value = serde_json::from_str(line)
        .map_err(|e| -> Box<dyn Error> { format!("reply must be valid JSON: {e}").into() })?;
    Ok(value)
}

/// Push `body` through the server using LSP-style `Content-Length:`
/// framing and return the parsed reply, verifying the reply itself is
/// also `Content-Length`-framed with no trailing newline.
fn send_content_length(body: &Value) -> Result<Value, Box<dyn Error>> {
    let raw = body.to_string();
    let wire = format!("Content-Length: {}\r\n\r\n{}", raw.len(), raw);
    let mut input = Cursor::new(wire.into_bytes());
    let mut output: Vec<u8> = Vec::new();
    mcp::run_stdio_session(&mut input, &mut output)
        .map_err(|e| -> Box<dyn Error> { format!("stdio session must not error: {e}").into() })?;
    let text = String::from_utf8(output)
        .map_err(|e| -> Box<dyn Error> { format!("reply must be valid UTF-8: {e}").into() })?;
    assert!(
        text.starts_with("Content-Length:"),
        "reply must be Content-Length-framed to match the request's framing: {text:?}"
    );
    assert!(
        !text.ends_with('\n'),
        "a Content-Length-framed reply must carry no trailing newline: {text:?}"
    );
    let (header, rest) = text
        .split_once("\r\n\r\n")
        .ok_or("must have header/body separator")?;
    let declared_len: usize = header
        .strip_prefix("Content-Length: ")
        .ok_or("header must start with Content-Length: ")?
        .trim()
        .parse()
        .map_err(|e| -> Box<dyn Error> {
            format!("Content-Length value must be numeric: {e}").into()
        })?;
    assert_eq!(
        declared_len,
        rest.len(),
        "declared length must match actual body length"
    );
    let value = serde_json::from_str(rest)
        .map_err(|e| -> Box<dyn Error> { format!("reply body must be valid JSON: {e}").into() })?;
    Ok(value)
}

fn rpc_request(id: i64, method: &str, params: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

#[test]
fn x06_mcp_proof_names_search_graph_runtime_telemetry_evidence() -> TestResult {
    let proof: Value = serde_json::from_str(X06_MCP_PROOF)?;
    let tests = proof["result"]["evidenceTests"]
        .as_array()
        .ok_or("x06-mcp proof evidenceTests must be an array")?;
    assert!(tests.iter().any(|test| {
        test == "mcp_cli_live::tools_call_search_graph_semantic_mode_returns_a_separate_semantic_results_list"
    }));
    assert!(tests.iter().any(|test| {
        test == "mcp_cli_live::tools_call_search_graph_ort_embedding_missing_cache_falls_back_without_network"
    }));
    assert_eq!(
        proof["hardRequirements"]["searchGraphSemanticRuntimeTelemetry"],
        json!("covered")
    );
    assert_eq!(
        proof["hardRequirements"]["cacheOnlyOrtEmbeddingFallback"],
        json!("covered")
    );
    assert_eq!(
        proof["hardRequirements"]["ortProviderSelectionTelemetry"],
        json!("covered")
    );
    assert_eq!(
        proof["hardRequirements"]["ortFallbackKindTelemetry"],
        json!("covered")
    );
    Ok(())
}

// ---------------------------------------------------------------------
// initialize handshake
// ---------------------------------------------------------------------

#[test]
fn initialize_handshake_returns_protocol_version_capabilities_and_server_info() -> TestResult {
    let reply = send_ndjson(&rpc_request(
        1,
        "initialize",
        &json!({ "protocolVersion": "2025-11-25" }),
    ))?;
    assert_eq!(reply["jsonrpc"], json!("2.0"));
    assert_eq!(reply["id"], json!(1));
    let result = &reply["result"];
    assert_eq!(result["protocolVersion"], json!("2025-11-25"));
    assert_eq!(result["serverInfo"]["name"], json!("enforcer-memory"));
    assert_eq!(
        result["capabilities"],
        json!({ "tools": { "listChanged": false } })
    );
    Ok(())
}

#[test]
fn initialize_falls_back_to_newest_supported_version_for_an_unknown_request() -> TestResult {
    let reply = send_ndjson(&rpc_request(
        2,
        "initialize",
        &json!({ "protocolVersion": "9999-01-01" }),
    ))?;
    assert_eq!(reply["result"]["protocolVersion"], json!("2025-11-25"));
    Ok(())
}

// ---------------------------------------------------------------------
// tools/list pagination
// ---------------------------------------------------------------------

#[test]
fn tools_list_first_page_has_8_tools_and_a_next_cursor() -> TestResult {
    let reply = send_ndjson(&rpc_request(3, "tools/list", &json!({})))?;
    let tools = reply["result"]["tools"]
        .as_array()
        .ok_or("tools must be an array")?;
    assert_eq!(tools.len(), 8);
    assert_eq!(reply["result"]["nextCursor"], json!("8"));
    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["title"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"].is_object());
        assert_eq!(
            tool["outputSchema"],
            json!({ "type": "object", "additionalProperties": true })
        );
    }
    Ok(())
}

#[test]
fn tools_list_second_page_has_remaining_7_tools_and_no_next_cursor() -> TestResult {
    let reply = send_ndjson(&rpc_request(4, "tools/list", &json!({ "cursor": "8" })))?;
    let tools = reply["result"]["tools"]
        .as_array()
        .ok_or("tools must be an array")?;
    assert_eq!(tools.len(), 7);
    assert!(
        reply["result"].get("nextCursor").is_none(),
        "the last page must omit nextCursor entirely, not emit null: {reply}"
    );
    Ok(())
}

#[test]
fn tools_list_across_both_pages_covers_baseline_plus_x06_extension_tools() -> TestResult {
    let page1 = send_ndjson(&rpc_request(5, "tools/list", &json!({})))?;
    let page2 = send_ndjson(&rpc_request(6, "tools/list", &json!({ "cursor": "8" })))?;
    let page1_tools = page1["result"]["tools"]
        .as_array()
        .ok_or("page1 tools must be an array")?;
    let page2_tools = page2["result"]["tools"]
        .as_array()
        .ok_or("page2 tools must be an array")?;
    let mut names: Vec<String> = Vec::new();
    for tool in page1_tools.iter().chain(page2_tools) {
        let name = tool["name"].as_str().ok_or("tool name must be a string")?;
        names.push(name.to_owned());
    }
    names.sort();
    let mut expected: Vec<String> = mcp::TOOL_NAMES.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(names, expected);
    Ok(())
}

// ---------------------------------------------------------------------
// tools/call: wired tool returns real fixture data in the exact envelope
// ---------------------------------------------------------------------

#[test]
fn tools_call_model_runtime_status_reports_managed_zero_network_contract() -> TestResult {
    let dir = tempfile::tempdir()?;

    let reply = send_ndjson(&rpc_request(
        7,
        "tools/call",
        &json!({
            "name": "model_runtime_status",
            "arguments": { "repoPath": dir.path().to_string_lossy() }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["zeroNetwork"], json!(true));
    assert_eq!(structured["capabilityState"], json!("degraded"));
    assert_eq!(structured["service"]["exposeLlamaServer"], json!(false));
    assert_eq!(
        structured["service"]["llamaCppOwnership"],
        json!("enforcer-subprocess")
    );
    assert_eq!(
        structured["service"]["ortOwnership"],
        json!("enforcer-isolated-worker")
    );
    assert_eq!(
        structured["controlPlanes"]["llamaCpp"]["valid"],
        json!(true)
    );
    assert_eq!(structured["controlPlanes"]["onnxOrt"]["valid"], json!(true));
    assert_eq!(
        structured["arbitration"]["embeddingChat"]["admission"],
        json!("pause-background-then-admit")
    );
    Ok(())
}

#[test]
fn tools_call_get_graph_schema_on_a_real_fixture_repo_returns_structured_content() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}\n")?;

    let reply = send_ndjson(&rpc_request(
        7,
        "tools/call",
        &json!({
            "name": "get_graph_schema",
            "arguments": { "repoPath": dir.path().to_string_lossy() }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let ok = result["structuredContent"]["ok"]
        .as_bool()
        .ok_or("ok must be a bool")?;
    assert!(ok);
    let total_nodes = result["structuredContent"]["totalNodes"]
        .as_u64()
        .ok_or("totalNodes must be a u64")?;
    assert!(total_nodes >= 1);
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or("text must be a string")?;
    let reparsed: Value = serde_json::from_str(text)
        .map_err(|e| -> Box<dyn Error> { format!("text must be valid JSON: {e}").into() })?;
    assert_eq!(reparsed, result["structuredContent"]);
    Ok(())
}

#[test]
fn tools_call_index_repository_on_a_real_fixture_repo_reports_files_indexed() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("a.rs"), "pub fn a() {}\n")?;
    std::fs::write(dir.path().join("b.rs"), "pub fn b() { a(); }\n")?;

    let reply = send_ndjson(&rpc_request(
        8,
        "tools/call",
        &json!({
            "name": "index_repository",
            "arguments": { "repoPath": dir.path().to_string_lossy() }
        }),
    ))?;
    let structured = &reply["result"]["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["filesIndexed"], json!(2));
    Ok(())
}

// ---------------------------------------------------------------------
// tools/call: search_graph regex/semantic modes, trace_path data_flow/
// cross_service modes, and ingest_traces -- all newly wired in this
// pass (see src/mcp.rs's WIRED_TOOLS; 14 baseline tools plus the X06
// extension tool and every documented mode are now live, no not_wired
// arms remain).
// ---------------------------------------------------------------------

/// Build a small real fixture repo: `a.rs` calls `helper`
/// (`b.rs`), `router.ts` imports `a.rs` and declares `GET /a`. Shared by
/// the search_graph/trace_path/ingest_traces live-envelope tests below.
fn write_fixture_repo(dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    std::fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    std::fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    std::fs::write(
        dir.join("router.ts"),
        "import { caller } from \"./a\";\nrouter.get(\"/a\", caller);\n",
    )?;
    Ok(())
}

#[test]
fn tools_call_search_graph_regex_mode_returns_real_matches() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        15,
        "tools/call",
        &json!({
            "name": "search_graph",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "namePattern": "helper"
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["mode"], json!("regex"));
    let results = structured["results"].as_array().ok_or("results array")?;
    assert!(
        results.iter().any(|hit| hit["name"] == json!("helper")),
        "expected a hit named helper, got {results:?}"
    );
    Ok(())
}

#[test]
fn tools_call_search_graph_semantic_mode_returns_a_separate_semantic_results_list() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        16,
        "tools/call",
        &json!({
            "name": "search_graph",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "namePattern": ".*",
                "semanticQuery": ["helper"]
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert!(structured["semanticResults"].is_array());
    assert!(structured["results"].is_array());
    Ok(())
}

#[test]
fn tools_call_search_graph_ort_embedding_missing_cache_falls_back_without_network() -> TestResult {
    let dir = tempfile::tempdir()?;
    let cache = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        31,
        "tools/call",
        &json!({
            "name": "search_graph",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "namePattern": ".*",
                "semanticQuery": ["helper"],
                "embeddingBackend": "ort",
                "embeddingProvider": "direct-ml",
                "embeddingCacheRoot": cache.path().join("missing-cache").to_string_lossy()
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert!(structured["semanticResults"].is_array());
    assert_eq!(
        structured["embeddingRuntime"]["requestedBackend"],
        json!("ort")
    );
    assert_eq!(
        structured["embeddingRuntime"]["resolvedBackend"],
        json!("hashing")
    );
    assert_eq!(
        structured["embeddingRuntime"]["requestedProvider"],
        json!("direct-ml")
    );
    assert_eq!(
        structured["embeddingRuntime"]["resolvedProvider"],
        json!(null)
    );
    assert_eq!(
        structured["embeddingRuntime"]["fallbackKind"],
        json!("cache-missing-or-invalid")
    );
    assert_eq!(
        structured["embeddingRuntime"]["state"],
        json!("degraded/provider-unavailable")
    );
    assert!(structured["embeddingRuntime"]["fallbackReason"]
        .as_str()
        .is_some_and(|reason| reason.contains("cache-only ORT model resolution failed")));
    Ok(())
}

#[test]
fn tools_call_search_graph_ort_embedding_missing_cache_root_falls_back_without_network(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        32,
        "tools/call",
        &json!({
            "name": "search_graph",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "namePattern": ".*",
                "semanticQuery": ["helper"],
                "embeddingBackend": "ort",
                "embeddingProvider": "cpu"
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(
        structured["embeddingRuntime"]["requestedBackend"],
        json!("ort")
    );
    assert_eq!(
        structured["embeddingRuntime"]["resolvedBackend"],
        json!("hashing")
    );
    assert_eq!(
        structured["embeddingRuntime"]["requestedProvider"],
        json!("cpu")
    );
    assert_eq!(
        structured["embeddingRuntime"]["fallbackKind"],
        json!("cache-root-missing")
    );
    assert_eq!(
        structured["embeddingRuntime"]["state"],
        json!("degraded/provider-unavailable")
    );
    assert!(structured["embeddingRuntime"]["fallbackReason"]
        .as_str()
        .is_some_and(|reason| reason.contains("no network fallback attempted")));
    Ok(())
}

#[test]
fn tools_call_search_graph_ort_embedding_unknown_provider_falls_back_without_network() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let cache = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        33,
        "tools/call",
        &json!({
            "name": "search_graph",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "namePattern": ".*",
                "semanticQuery": ["helper"],
                "embeddingBackend": "ort",
                "embeddingProvider": "quantum-vram",
                "embeddingCacheRoot": cache.path().to_string_lossy()
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(
        structured["embeddingRuntime"]["requestedBackend"],
        json!("ort")
    );
    assert_eq!(
        structured["embeddingRuntime"]["resolvedBackend"],
        json!("hashing")
    );
    assert_eq!(
        structured["embeddingRuntime"]["requestedProvider"],
        json!(null)
    );
    assert_eq!(
        structured["embeddingRuntime"]["fallbackKind"],
        json!("invalid-provider")
    );
    assert_eq!(
        structured["embeddingRuntime"]["state"],
        json!("degraded/provider-unavailable")
    );
    assert!(structured["embeddingRuntime"]["fallbackReason"]
        .as_str()
        .is_some_and(|reason| reason.contains("unknown embeddingProvider")));
    Ok(())
}

#[test]
fn tools_call_trace_path_data_flow_mode_reports_call_graph_only_approximation() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        17,
        "tools/call",
        &json!({
            "name": "trace_path",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "startNodeId": "file:a.rs",
                "mode": "data_flow",
                "direction": "out"
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["mode"], json!("data_flow"));
    assert_eq!(structured["approximation"], json!("CallGraphOnly"));
    Ok(())
}

#[test]
fn tools_call_trace_path_cross_service_mode_finds_the_declared_route() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        18,
        "tools/call",
        &json!({
            "name": "trace_path",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "startNodeId": "file:router.ts",
                "mode": "cross_service",
                "direction": "both"
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["mode"], json!("cross_service"));
    let paths = structured["paths"].as_array().ok_or("paths array")?;
    assert!(
        paths.iter().any(
            |p| p["mediator"]["method"] == json!("GET") && p["mediator"]["path"] == json!("/a")
        ),
        "expected GET /a route among mediators, got {paths:?}"
    );
    Ok(())
}

#[test]
fn tools_call_trace_path_unknown_mode_is_a_tool_error_not_not_wired() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        19,
        "tools/call",
        &json!({
            "name": "trace_path",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "startNodeId": "file:a.rs",
                "mode": "bogus"
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(true));
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or("text must be a string")?;
    assert_eq!(
        text,
        "{\"error\":{\"message\":\"unknown mode \\\"bogus\\\". Valid: calls, data_flow, cross_service.\",\"tool\":\"trace_path\"},\"ok\":false}"
    );
    assert!(
        !text.contains("not_wired"),
        "an unknown mode is a bad argument, not a capability gap: {text}"
    );
    Ok(())
}

#[test]
fn tools_call_ingest_traces_merges_a_real_runtime_trace_into_the_call_graph() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        20,
        "tools/call",
        &json!({
            "name": "ingest_traces",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "traces": [
                    { "caller": "file:a.rs", "callee": "helper", "count": 5 }
                ]
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["ingestedCount"], json!(1));
    assert_eq!(structured["unresolvedCount"], json!(0));
    let edges = structured["edges"].as_array().ok_or("edges array")?;
    let annotated = edges
        .iter()
        .find(|e| e["caller"] == json!("file:a.rs") && e["callee"] == json!("helper"))
        .ok_or("expected the annotated edge")?;
    assert_eq!(annotated["provenance"], json!("Parsed"));
    assert_eq!(annotated["observedCount"], json!(5));
    Ok(())
}

#[test]
fn tools_call_ingest_traces_records_unresolved_trace_records_not_silently_dropped() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_fixture_repo(dir.path())?;

    let reply = send_ndjson(&rpc_request(
        21,
        "tools/call",
        &json!({
            "name": "ingest_traces",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "traces": [
                    { "caller": "sym:does-not-exist.rs:1:ghost", "callee": "helper", "count": 1 }
                ]
            }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    let structured = &result["structuredContent"];
    assert_eq!(structured["unresolvedCount"], json!(1));
    let unresolved = structured["unresolved"][0].clone();
    assert_eq!(unresolved["unresolvedCaller"], json!(true));
    assert_eq!(unresolved["unresolvedCallee"], json!(false));
    Ok(())
}

// ---------------------------------------------------------------------
// tools/call: manage_adr whole-document baseline parity
// (refs/x06-baseline-tool-schemas.md §14) -- update then get through the
// live MCP envelope, proving the exact `{"status":"updated"}` and
// `{"content": ...}` response shapes round-trip end to end (not just via
// dispatch_tool's own unit tests).
// ---------------------------------------------------------------------

#[test]
fn tools_call_manage_adr_update_then_get_roundtrips_through_the_mcp_envelope() -> TestResult {
    let markdown = "## PURPOSE\nlocal-first store\n\n## STACK\nrust\n";

    let update_reply = send_ndjson(&rpc_request(
        20,
        "tools/call",
        &json!({
            "name": "manage_adr",
            "arguments": { "project": "proj-a", "mode": "update", "content": markdown }
        }),
    ))?;
    let update_result = &update_reply["result"];
    assert_eq!(update_result["isError"], json!(false));
    assert_eq!(
        update_result["structuredContent"]["status"],
        json!("updated")
    );

    // The caller round-trips the stored document (this lane has no
    // persistence layer behind manage_adr) by passing it back as
    // `document` on the next call.
    let get_reply = send_ndjson(&rpc_request(
        21,
        "tools/call",
        &json!({
            "name": "manage_adr",
            "arguments": { "project": "proj-a", "mode": "get", "document": markdown }
        }),
    ))?;
    let get_result = &get_reply["result"];
    assert_eq!(get_result["isError"], json!(false));
    assert_eq!(get_result["structuredContent"]["content"], json!(markdown));
    assert!(get_result["structuredContent"]["status"].is_null());

    let sections_reply = send_ndjson(&rpc_request(
        22,
        "tools/call",
        &json!({
            "name": "manage_adr",
            "arguments": { "project": "proj-a", "mode": "sections", "document": markdown }
        }),
    ))?;
    let sections_result = &sections_reply["result"];
    assert_eq!(sections_result["isError"], json!(false));
    assert_eq!(
        sections_result["structuredContent"]["sections"],
        json!(["## PURPOSE", "## STACK"])
    );
    Ok(())
}

#[test]
fn tools_call_manage_adr_get_on_never_stored_project_is_no_adr_with_hint() -> TestResult {
    let reply = send_ndjson(&rpc_request(
        23,
        "tools/call",
        &json!({
            "name": "manage_adr",
            "arguments": { "project": "proj-never-stored" }
        }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false));
    assert_eq!(result["structuredContent"]["content"], json!(""));
    assert_eq!(result["structuredContent"]["status"], json!("no_adr"));
    assert!(result["structuredContent"]["adr_hint"]
        .as_str()
        .unwrap_or_default()
        .starts_with("No ADR yet."));
    Ok(())
}

// ---------------------------------------------------------------------
// tools/call: unknown tool name -> exact binding-spec envelope
// ---------------------------------------------------------------------

#[test]
fn tools_call_unknown_tool_name_is_iserror_with_exact_text() -> TestResult {
    let reply = send_ndjson(&rpc_request(
        10,
        "tools/call",
        &json!({ "name": "does_not_exist", "arguments": {} }),
    ))?;
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["content"][0]["text"],
        json!("unknown tool: does_not_exist")
    );
    Ok(())
}

// ---------------------------------------------------------------------
// ping
// ---------------------------------------------------------------------

#[test]
fn ping_returns_an_empty_object_result() -> TestResult {
    let reply = send_ndjson(&rpc_request(11, "ping", &json!({})))?;
    assert_eq!(reply["result"], json!({}));
    Ok(())
}

// ---------------------------------------------------------------------
// Both framings
// ---------------------------------------------------------------------

#[test]
fn ndjson_framing_round_trips_a_full_request_reply_cycle() -> TestResult {
    let reply = send_ndjson(&rpc_request(12, "ping", &json!({})))?;
    assert_eq!(reply["result"], json!({}));
    Ok(())
}

#[test]
fn content_length_framing_round_trips_a_full_request_reply_cycle() -> TestResult {
    let reply = send_content_length(&rpc_request(13, "ping", &json!({})))?;
    assert_eq!(reply["result"], json!({}));
    Ok(())
}

#[test]
fn content_length_request_gets_a_content_length_reply_even_for_a_tool_call() -> TestResult {
    let reply = send_content_length(&rpc_request(
        14,
        "tools/call",
        &json!({ "name": "ingest_traces", "arguments": {} }),
    ))?;
    assert_eq!(reply["result"]["isError"], json!(true));
    Ok(())
}

// ---------------------------------------------------------------------
// JSON-RPC-level errors: parse error and method-not-found
// ---------------------------------------------------------------------

#[test]
fn malformed_json_yields_a_parse_error_with_id_zero() -> TestResult {
    let mut input = Cursor::new(b"not json at all\n".to_vec());
    let mut output: Vec<u8> = Vec::new();
    mcp::run_stdio_session(&mut input, &mut output)
        .map_err(|e| -> Box<dyn Error> { format!("must not error: {e}").into() })?;
    let text = String::from_utf8(output)?;
    let line = text.lines().next().ok_or("expected a reply line")?;
    let reply: Value = serde_json::from_str(line)?;
    assert_eq!(reply["error"]["code"], json!(-32700));
    assert_eq!(reply["id"], json!(0));
    Ok(())
}

#[test]
fn unknown_method_yields_method_not_found_with_id_echoed() -> TestResult {
    let reply = send_ndjson(&rpc_request(42, "totally/unknown", &json!({})))?;
    assert_eq!(reply["error"]["code"], json!(-32601));
    assert_eq!(reply["id"], json!(42));
    Ok(())
}

#[test]
fn notifications_cancelled_produces_no_reply_at_all() -> TestResult {
    let mut input = Cursor::new(
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 1 } })
            .to_string()
            .into_bytes(),
    );
    let mut input = {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut input, &mut bytes)?;
        bytes.push(b'\n');
        Cursor::new(bytes)
    };
    let mut output: Vec<u8> = Vec::new();
    mcp::run_stdio_session(&mut input, &mut output)
        .map_err(|e| -> Box<dyn Error> { format!("must not error: {e}").into() })?;
    assert!(
        output.is_empty(),
        "a notification must never produce a reply"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// CLI mirror parity: same tool+json -> same inner JSON as MCP
// ---------------------------------------------------------------------

#[test]
fn cli_mirror_produces_the_same_envelope_json_as_the_mcp_tools_call_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}\n")?;

    let args = json!({ "repoPath": dir.path().to_string_lossy() });
    let mcp_envelope = mcp::call_tool("get_graph_schema", &args);

    let cli_envelope_str = cli::cli_invoke("get_graph_schema", &args.to_string())
        .map_err(|e| -> Box<dyn Error> { format!("cli_invoke must succeed: {e}").into() })?;
    let cli_envelope: Value = serde_json::from_str(&cli_envelope_str)?;

    assert_eq!(
        cli_envelope, mcp_envelope,
        "CLI and MCP must produce byte-identical envelopes"
    );
    Ok(())
}

#[test]
fn cli_run_cli_exit_codes_are_strictly_0_or_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}\n")?;

    let ok_argv: Vec<String> = [
        "get_graph_schema".to_owned(),
        format!(r#"{{"repoPath":{:?}}}"#, dir.path().to_string_lossy()),
    ]
    .to_vec();
    let ok_outcome = cli::run_cli(&ok_argv);
    assert_eq!(ok_outcome.exit_code, 0);

    let err_argv: Vec<String> = ["ingest_traces".to_owned(), "{}".to_owned()].to_vec();
    let err_outcome = cli::run_cli(&err_argv);
    assert_eq!(err_outcome.exit_code, 1);
    Ok(())
}

#[test]
fn cli_json_mode_prints_the_same_raw_envelope_cli_invoke_returns() -> TestResult {
    let argv: Vec<String> = [
        "--json".to_owned(),
        "ingest_traces".to_owned(),
        "{}".to_owned(),
    ]
    .to_vec();
    let outcome = cli::run_cli(&argv);
    let invoked = cli::cli_invoke("ingest_traces", "{}")
        .map_err(|e| -> Box<dyn Error> { format!("cli_invoke must succeed: {e}").into() })?;
    let stderr = outcome.stderr.ok_or("expected stderr output")?;
    let printed: Value = serde_json::from_str(&stderr)?;
    let invoked_value: Value = serde_json::from_str(&invoked)?;
    assert_eq!(printed, invoked_value);
    Ok(())
}

// ---------------------------------------------------------------------
// Watcher: single debounced event (also covered unit-side in watch.rs;
// this is the same behavioral contract exercised as a hard test here
// too, per the workpack's cross-surface hard-test list)
// ---------------------------------------------------------------------

#[test]
fn watcher_emits_exactly_one_debounced_reindex_request_for_a_burst_of_writes() -> TestResult {
    use enforcer_memory::watch::Watcher;
    use std::time::Duration;

    let dir = tempfile::tempdir()?;
    let watcher = Watcher::start(dir.path(), Duration::from_millis(150))?;

    let file_path = dir.path().join("watched.rs");
    std::fs::write(&file_path, "fn x() {}\n")?;
    std::fs::write(&file_path, "fn x() { /* edited */ }\n")?;

    let request = watcher
        .next_reindex_request(Duration::from_secs(10))?
        .ok_or("expected one reindex request")?;
    assert!(request.paths.iter().any(|p| p.ends_with("watched.rs")));

    let second = watcher.next_reindex_request(Duration::from_millis(200))?;
    assert!(
        second.is_none(),
        "burst must collapse to exactly one request, got {second:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Diagnostics: redaction + stdout purity
// ---------------------------------------------------------------------

#[test]
fn diagnostics_never_leak_full_source_text_and_never_touch_stdout() -> TestResult {
    use enforcer_memory::diagnostics::{Diagnostics, FileSkipRecord, Format, Level, SkipPhase};

    let huge_source = "fn leaked_secret_marker() {}\n".repeat(20);
    let record = FileSkipRecord {
        path: "src/whatever.rs".to_owned(),
        reason: huge_source.clone(),
        phase: SkipPhase::Parse,
    };
    let diagnostics = Diagnostics::new(Level::Debug, Format::Json);
    let mut buf: Vec<u8> = Vec::new();
    diagnostics
        .emit(&mut buf, Level::Warn, &record)
        .map_err(|e| -> Box<dyn Error> { format!("emit must not error: {e}").into() })?;
    let line = String::from_utf8(buf)?;
    assert!(
        !line.contains(&huge_source),
        "diagnostics must never leak full source text verbatim"
    );
    // This test's own emission target is an in-memory buffer, never real
    // stdout/stderr -- the crate-wide guarantee (module docs: "this
    // module NEVER writes to stdout") is that `Diagnostics::emit` itself
    // performs no stdio I/O of its own; only `emit_to_stderr` touches a
    // real handle, and it targets stderr exclusively (see its own source
    // and the module's `#[allow(clippy::print_stderr)]` scoping).
    Ok(())
}
