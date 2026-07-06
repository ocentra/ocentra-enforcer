//! X06.7 unit-shaped tests for [`enforcer_memory::mcp`], moved out of
//! `src/mcp.rs` per this crate's "no inline `#[cfg(test)]` modules" style
//! (workspace clippy denies `unwrap`/`expect`/`panic` even in test code,
//! so every assertion here goes through `Result` + `?` rather than the
//! original inline module's `.unwrap()`/`.expect(...)` calls).

use enforcer_memory::mcp::{dispatch_tool, tool_descriptors, wrap_envelope};
use serde_json::json;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn tool_descriptors_cover_all_14_baseline_tools_with_schemas() {
    let descriptors = tool_descriptors();
    assert_eq!(descriptors.len(), 14);
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
    assert!(text.contains("unknown tool"));
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
fn get_architecture_rejects_unknown_aspect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = dispatch_tool(
        "get_architecture",
        &json!({ "repoPath": dir.path().to_string_lossy(), "aspects": ["bogus"] }),
    );
    assert_eq!(result["ok"], json!(false));
    Ok(())
}
