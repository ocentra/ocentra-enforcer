//! P3 live-MCP-tool smoke test: spawn the REAL compiled binary as a child
//! process, speak real bytes over its stdin/stdout pipes, and assert
//! observable behavior — the workpack's stated "key row" ("the MCP stdio
//! smoke (spawn the built binary, initialize, list tools, call one tool
//! end-to-end) is the key row. It must always run as a real process proof.").
//!
//! This spawns `enforcer-mcp-smoke` (see `src/bin/enforcer-mcp-smoke.rs`),
//! a throwaway harness binary that calls the exact same
//! `enforcer_mcp::sink::run_stdio_server` entry point the eventual
//! `enforcer-cli` (arc-22) `serve` subcommand will call — so this is a
//! real end-to-end proof of the stdio surface, rather than an in-process
//! substitute.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn smoke_binary_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let mut dir = exe
        .parent()
        .ok_or("test binary has no parent directory")?
        .to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join(if cfg!(windows) {
        "enforcer-mcp-smoke.exe"
    } else {
        "enforcer-mcp-smoke"
    });
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(format!("smoke binary not found at {}", candidate.display()).into())
}

/// Send one NDJSON request and read exactly one NDJSON reply line.
fn round_trip(
    stdin: &mut impl Write,
    stdout: &mut impl BufRead,
    request: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let line = format!("{request}\n");
    stdin.write_all(line.as_bytes())?;
    stdin.flush()?;
    let mut reply_line = String::new();
    stdout.read_line(&mut reply_line)?;
    Ok(serde_json::from_str(reply_line.trim_end())?)
}

#[test]
fn stdio_smoke_initialize_list_tools_and_call_one_tool_end_to_end(
) -> Result<(), Box<dyn std::error::Error>> {
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);

    // 1. initialize
    let init_reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05" },
        }),
    )?;
    assert_eq!(
        init_reply["result"]["serverInfo"]["name"],
        serde_json::json!(enforcer_mcp::name::SERVER_NAME)
    );

    // 2. tools/list
    let list_reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )?;
    let tools = list_reply["result"]["tools"]
        .as_array()
        .ok_or("tools/list result.tools must be an array")?;
    assert!(
        !tools.is_empty(),
        "tools/list must return a non-empty tool surface"
    );
    assert!(
        tools
            .iter()
            .any(|t| t["name"] == serde_json::json!("ocentra_enforcer_mcp_status")),
        "ocentra_enforcer_mcp_status must be in the advertised tool surface"
    );

    // 3. tools/call — end-to-end call of one real tool
    let call_reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "ocentra_enforcer_mcp_status", "arguments": {} },
        }),
    )?;
    assert_eq!(call_reply["result"]["ok"], serde_json::json!(true));
    assert!(call_reply["result"]["toolCount"].as_u64().unwrap_or(0) > 0);

    // Close stdin so the child's read loop observes EOF and exits cleanly.
    drop(stdin);
    let status = child.wait()?;
    assert!(status.success(), "smoke binary must exit 0 on stdin EOF");
    Ok(())
}

#[test]
fn stdio_smoke_legacy_alias_call_resolves_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);

    let reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "rust_rules_mcp_status", "arguments": {} },
        }),
    )?;
    assert_eq!(reply["result"]["ok"], serde_json::json!(true));

    drop(stdin);
    let status = child.wait()?;
    assert!(status.success());
    Ok(())
}
