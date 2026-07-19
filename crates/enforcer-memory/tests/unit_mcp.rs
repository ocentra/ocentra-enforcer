//! X06.7 unit-shaped tests for [`enforcer_memory::mcp`], moved out of
//! `src/mcp.rs` per this crate's "no inline `#[cfg(test)]` modules" style
//! (workspace clippy denies `unwrap`/`expect`/`panic` even in test code,
//! so every assertion here goes through `Result` + `?` rather than the
//! original inline module's `.unwrap()`/`.expect(...)` calls).

use enforcer_domain::mcp_types::McpToolName;
use enforcer_domain::memory_types::GraphEventKind;
use enforcer_memory::boundary::log_schema::{GraphEventLogEntryDto, SCHEMA_VERSION};
use enforcer_memory::mcp::{dispatch_tool, tool_descriptors, wrap_envelope, ToolDescriptorDto};
use enforcer_memory::store::sqlite::OperationalGraph;
use enforcer_memory::store::Store;
use serde_json::json;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn tool_descriptor_maps_to_canonical_name() -> TestResult {
    let descriptor = ToolDescriptorDto {
        name: "search_graph".to_owned(),
        title: "Search graph".to_owned(),
        description: "Search the indexed graph".to_owned(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
    };
    let wire = serde_json::to_vec(&descriptor)?;
    let restored: ToolDescriptorDto = serde_json::from_slice(&wire)?;
    assert_eq!(restored, descriptor);
    let canonical = McpToolName::try_from(restored)?;
    assert_eq!(canonical.as_str(), "search_graph");
    Ok(())
}

#[test]
fn tool_descriptor_rejects_invalid_malformed_tool_name() -> TestResult {
    let invalid_payload = json!({
        "name": "invalid tool name",
        "title": "Invalid",
        "description": "must not enter the MCP catalog",
        "inputSchema": {"type": "object"},
        "outputSchema": {"type": "object"}
    });
    let invalid_result = McpToolName::try_from(serde_json::from_value::<ToolDescriptorDto>(
        invalid_payload,
    )?);
    assert!(invalid_result.is_err());
    Ok(())
}

#[test]
fn tool_descriptors_cover_baseline_plus_x06_extension_tools_with_schemas() {
    let descriptors = tool_descriptors();
    assert_eq!(descriptors.len(), 15);
    for descriptor in &descriptors {
        assert!(
            descriptor.input_schema.is_object(),
            "{} must have an object input schema",
            descriptor.name
        );
        assert!(
            !descriptor.description.is_empty(),
            "{} must have a description",
            descriptor.name
        );
    }
}

#[test]
fn dispatch_unknown_tool_is_a_tool_error_not_a_panic() {
    let result = dispatch_tool("totally_unknown_tool", &json!({}));
    assert_eq!(result["ok"], json!(false));
}

#[test]
fn manage_adr_create_then_get_round_trips_through_caller_owned_state() {
    let created = dispatch_tool(
        "manage_adr",
        &json!({ "operation": "create", "id": "adr-1", "title": "Use SQLite", "adrs": [] }),
    );
    assert_eq!(created["ok"], json!(true));
    let adrs = created["adrs"].clone();

    let fetched = dispatch_tool(
        "manage_adr",
        &json!({ "operation": "get", "id": "adr-1", "adrs": adrs }),
    );
    assert_eq!(fetched["ok"], json!(true));
    assert_eq!(fetched["adr"]["title"], json!("Use SQLite"));
}

#[test]
fn wrap_envelope_success_carries_content_text_and_structured_content() -> TestResult {
    let inner = json!({ "ok": true, "value": 42 });
    let envelope = wrap_envelope(&inner);
    assert_eq!(envelope["isError"], json!(false));
    assert_eq!(envelope["structuredContent"], inner);
    let text = envelope["content"][0]["text"]
        .as_str()
        .ok_or("content[0].text must be a string")?;
    let reparsed: serde_json::Value = serde_json::from_str(text)?;
    assert_eq!(reparsed, inner);
    Ok(())
}

#[test]
fn wrap_envelope_error_omits_structured_content() -> TestResult {
    // Build an error-shaped inner result the same way dispatch_tool does
    // for an unrecognized tool name (`{"ok": false, "error": {...}}`),
    // rather than reaching into mcp's now-private not_wired() helper --
    // every tool is wired in this pass, so there is no live not_wired
    // call site left to exercise for this envelope-shape assertion.
    let inner = dispatch_tool("totally_unknown_tool", &json!({}));
    let envelope = wrap_envelope(&inner);
    assert_eq!(envelope["isError"], json!(true));
    assert!(
        envelope.get("structuredContent").is_none(),
        "structuredContent must be omitted entirely on error, not null: {envelope}"
    );
    let text = envelope["content"][0]["text"]
        .as_str()
        .ok_or("content[0].text must be a string")?;
    let payload: serde_json::Value = serde_json::from_str(text)?;
    assert_eq!(
        payload["error"]["message"],
        json!("unknown tool: totally_unknown_tool")
    );
    Ok(())
}

#[test]
fn list_projects_on_empty_stores_dir_returns_empty_ok() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = dispatch_tool(
        "list_projects",
        &json!({ "storesDir": dir.path().to_string_lossy() }),
    );
    assert_eq!(result["ok"], json!(true));
    assert_eq!(result["projects"], json!([]));
    Ok(())
}

#[test]
fn get_graph_schema_on_empty_repo_returns_zero_totals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = dispatch_tool(
        "get_graph_schema",
        &json!({ "repoPath": dir.path().to_string_lossy() }),
    );
    assert_eq!(result["ok"], json!(true));
    assert_eq!(result["totalNodes"], json!(0));
    Ok(())
}

#[test]
fn query_graph_prefers_store_projection_when_stores_dir_is_available() -> TestResult {
    let repo_dir = tempfile::tempdir()?;
    let stores_dir = tempfile::tempdir()?;
    let repo_root =
        enforcer_memory::ids::repo_root(&repo_dir.path().to_string_lossy().as_ref().into())?;
    let mut store = Store::init(stores_dir.path(), &repo_root, "2026-07-07T00:00:00Z")?;

    let file_id = "file:store_only.rs".to_owned();
    let symbol_id = "sym:store_only.rs:7:store_only_symbol".to_owned();
    store.append_graph_event_entry(|seq| GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: seq.into(),
        id: format!("evt-node-file-{seq}").into(),
        event: GraphEventKind::NodeAdded {
            node_id: file_id.clone().into(),
            node_kind: "File".into(),
        },
        ts: "2026-07-07T00:00:00Z".into(),
        supersedes_seq: None,
    })?;
    store.append_graph_event_entry(|seq| GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: seq.into(),
        id: format!("evt-node-symbol-{seq}").into(),
        event: GraphEventKind::NodeAdded {
            node_id: symbol_id.clone().into(),
            node_kind: "Function".into(),
        },
        ts: "2026-07-07T00:00:00Z".into(),
        supersedes_seq: None,
    })?;
    store.append_graph_event_entry(|seq| GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: seq.into(),
        id: format!("evt-edge-contains-{seq}").into(),
        event: GraphEventKind::EdgeAdded {
            from: file_id.clone().into(),
            to: symbol_id.clone().into(),
            label: "contains".into(),
        },
        ts: "2026-07-07T00:00:00Z".into(),
        supersedes_seq: None,
    })?;

    let sqlite_path = store.sqlite_path();
    let entries = store.read_graph_event_entries()?;
    drop(store);
    let mut operational = OperationalGraph::open(&sqlite_path)?;
    operational.rebuild(&entries.entries)?;

    let result = dispatch_tool(
        "query_graph",
        &json!({
            "repoPath": repo_dir.path().to_string_lossy(),
            "storesDir": stores_dir.path().to_string_lossy(),
            "query": "MATCH (f:Function) RETURN f.name ORDER BY f.name"
        }),
    );

    assert_eq!(result["ok"], json!(true));
    assert_eq!(result["graphSource"], json!("storeProjection"));
    assert_eq!(result["rowCount"], json!(1));
    assert_eq!(result["rows"][0]["f"], json!(symbol_id));

    Ok(())
}

#[test]
fn index_repository_with_stores_dir_primes_a_fresh_store_projection() -> TestResult {
    let repo_dir = tempfile::tempdir()?;
    let stores_dir = tempfile::tempdir()?;
    std::fs::write(
        repo_dir.path().join("widget.rs"),
        "fn helper() { let _ = 1; }\nfn caller() { helper(); }\n",
    )?;

    let indexed = dispatch_tool(
        "index_repository",
        &json!({
            "repoPath": repo_dir.path().to_string_lossy(),
            "storesDir": stores_dir.path().to_string_lossy()
        }),
    );
    assert_eq!(indexed["ok"], json!(true));
    assert_eq!(indexed["graphPersistence"]["enabled"], json!(true));
    assert_eq!(
        indexed["graphPersistence"]["refreshMode"],
        json!("fresh-store-only")
    );

    let queried = dispatch_tool(
        "query_graph",
        &json!({
            "repoPath": repo_dir.path().to_string_lossy(),
            "storesDir": stores_dir.path().to_string_lossy(),
            "query": "MATCH (f:Function) RETURN f.name ORDER BY f.name"
        }),
    );
    assert_eq!(queried["ok"], json!(true));
    assert_eq!(queried["graphSource"], json!("storeProjection"));
    assert_eq!(queried["rowCount"], json!(2));

    Ok(())
}

#[test]
fn index_repository_rejects_appending_into_an_existing_store_projection() -> TestResult {
    let repo_dir = tempfile::tempdir()?;
    let stores_dir = tempfile::tempdir()?;
    std::fs::write(
        repo_dir.path().join("widget.rs"),
        "fn helper() { let _ = 1; }\n",
    )?;

    let first = dispatch_tool(
        "index_repository",
        &json!({
            "repoPath": repo_dir.path().to_string_lossy(),
            "storesDir": stores_dir.path().to_string_lossy()
        }),
    );
    assert_eq!(first["ok"], json!(true));

    let second = dispatch_tool(
        "index_repository",
        &json!({
            "repoPath": repo_dir.path().to_string_lossy(),
            "storesDir": stores_dir.path().to_string_lossy()
        }),
    );
    assert_eq!(second["ok"], json!(false));
    let message = second["error"]["message"]
        .as_str()
        .ok_or("index_repository error message must be a string")?;
    assert!(
        message.contains("refresh/reindex over an existing Store projection is refused"),
        "{message}"
    );

    Ok(())
}

#[test]
fn get_architecture_rejects_unknown_aspect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = dispatch_tool(
        "get_architecture",
        &json!({ "repoPath": dir.path().to_string_lossy(), "aspects": ["bogus"] }),
    );
    assert_eq!(result["ok"], json!(false));
    Ok(())
}
