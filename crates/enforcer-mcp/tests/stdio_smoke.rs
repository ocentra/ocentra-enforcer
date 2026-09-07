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
        .stderr(Stdio::inherit())
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
fn stdio_coordination_report_index_and_notify_replay_the_real_temp_ledger(
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger = tempfile::tempdir()?;
    let mut child = Command::new(smoke_binary_path()?)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let mut reader = BufReader::new(child.stdout.take().ok_or("child has no stdout")?);
    let root = ledger.path().to_string_lossy();
    let init = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0","id":0,"method":"tools/call",
            "params":{"name":"ocentra_enforcer_coordination_init","arguments":{"root":root,"hub":"test-hub","lane":"primary"}}
        }),
    )?;
    assert_eq!(init["result"]["ok"], true);
    let report = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"ocentra_enforcer_coordination_report","arguments": {"root":root,"hub":"test-hub","lane":"worker-a","worktreeRoot":"C:/wt","branch":"rust-build","projectId":"test-project","summary":"BLOCKED needs primary review"}}
        }),
    )?;
    assert_eq!(report["result"]["ok"], true);
    assert_eq!(report["result"]["event"]["type"], "report");
    let index = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"ocentra_enforcer_coordination_index","arguments":{"root":root}}
        }),
    )?;
    assert_eq!(index["result"]["ok"], true);
    assert_eq!(index["result"]["indexKind"], "derived-stream-replay");
    assert_eq!(index["result"]["reports"].as_array().map(Vec::len), Some(1));
    let notify = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"ocentra_enforcer_coordination_notify","arguments":{"root":root,"hub":"test-hub","lane":"primary","worktreeRoot":"C:/primary","branch":"rust-build","projectId":"test-project","peek":true}}
        }),
    )?;
    assert_eq!(notify["result"]["ok"], true);
    assert_eq!(
        notify["result"]["wakeRequests"].as_array().map(Vec::len),
        Some(1)
    );
    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_server_keeps_validation_history_for_its_process_lifetime(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("src"))?;
    std::fs::write(
        root.path().join("src/lib.rs"),
        "mod inner { pub struct Thing; }\npub use inner::Thing;\n",
    )?;
    let mut child = Command::new(smoke_binary_path()?)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let mut reader = BufReader::new(child.stdout.take().ok_or("child has no stdout")?);
    let scan = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_scan","arguments":{"root":root.path().to_string_lossy(),"files":["src/lib.rs"]}}
        }),
    )?;
    assert_eq!(scan["result"]["ok"], serde_json::json!(false));
    let status = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_run_status","arguments":{"root":root.path().to_string_lossy(),"tool":"scan"}}
        }),
    )?;
    assert_eq!(
        status["result"]["summaryType"],
        serde_json::json!("validation")
    );
    assert_eq!(
        status["result"]["summary"]["kind"],
        serde_json::json!("scan")
    );
    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_doctor_reaches_native_repository_engine() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("src"))?;
    std::fs::write(
        fixture.path().join("src/lib.rs"),
        "pub fn stdio_doctor() {}\n",
    )?;
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);
    let reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_doctor","arguments":{"root":fixture.path().to_string_lossy(),"files":["src/lib.rs"]}}
        }),
    )?;
    assert_eq!(reply["result"]["command"], serde_json::json!("doctor"));
    assert_eq!(reply["result"]["checks"].as_array().map(Vec::len), Some(6));
    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_harness_query_tools_reach_the_native_rust_engine(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let run_dir = fixture.path().join(".enforce/runs/run-native-failed");
    std::fs::create_dir_all(&run_dir)?;
    std::fs::write(
        run_dir.join("summary.json"),
        r#"{"runId":"run-native-failed","status":"failed","tool":"cargo","startedAt":"2026-07-30T00:00:00Z","artifacts":{"diagnostics":".enforce/runs/run-native-failed/diagnostics.ndjson","stderr":".enforce/runs/run-native-failed/stderr.log"}}"#,
    )?;
    std::fs::write(
        run_dir.join("diagnostics.ndjson"),
        r#"{"severity":"error","file":"src/lib.rs","message":"native fixture"}
"#,
    )?;
    std::fs::write(run_dir.join("stderr.log"), "native stderr fixture")?;

    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);
    let root = fixture.path().to_string_lossy();

    let diagnostics = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_diagnostics","arguments":{"root":root,"severity":"error","file":"src/lib.rs"}}
        }),
    )?;
    assert_eq!(diagnostics["result"]["ok"], true);
    assert_eq!(diagnostics["result"]["runId"], "run-native-failed");
    assert_eq!(
        diagnostics["result"]["diagnostics"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let failure = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_last_failure","arguments":{"root":root,"tool":"cargo","diagnosticLimit":1}}
        }),
    )?;
    assert_eq!(failure["result"]["ok"], true);
    assert_eq!(failure["result"]["found"], true);
    assert_eq!(failure["result"]["run"]["runId"], "run-native-failed");

    let artifact = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":3, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_artifact","arguments":{"root":root,"runId":"run-native-failed","artifact":"stderr","limitBytes":80}}
        }),
    )?;
    assert_eq!(artifact["result"]["ok"], true);
    assert_eq!(artifact["result"]["artifact"], "stderr");
    assert_eq!(
        artifact["result"]["path"],
        ".enforce/runs/run-native-failed/stderr.log"
    );
    assert_eq!(artifact["result"]["text"], "native stderr fixture");

    let reset = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc":"2.0", "id":4, "method":"tools/call",
            "params":{"name":"ocentra_enforcer_reset_runs","arguments":{"root":root,"tag":"ignored-by-reset"}}
        }),
    )?;
    assert_eq!(reset["result"]["ok"], true);
    assert_eq!(reset["result"]["removed"], serde_json::json!([".enforce"]));
    assert!(!fixture.path().join(".enforce").exists());

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_legacy_alias_call_resolves_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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

#[test]
fn stdio_smoke_scan_reaches_the_native_rust_engine() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let src = fixture.path().join("src");
    std::fs::create_dir_all(&src)?;
    std::fs::write(
        src.join("lib.rs"),
        "mod inner { pub struct Thing; }\npub use inner::Thing;\n",
    )?;

    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
            "params": {
                "name": "ocentra_enforcer_scan",
                "arguments": {
                    "root": fixture.path().to_string_lossy(),
                    "files": ["src/lib.rs"],
                    "languages": ["rust"],
                },
            },
        }),
    )?;
    assert_eq!(reply["result"]["ok"], serde_json::json!(false));
    assert!(reply["result"]["findings"]
        .as_array()
        .is_some_and(|findings| findings
            .iter()
            .any(|finding| finding["ruleId"] == "T1-NOREEXPORT.1")));

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_check_no_zod_source_returns_the_native_rule_report(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let src = fixture.path().join("src");
    std::fs::create_dir_all(&src)?;
    std::fs::write(
        src.join("schema.ts"),
        "import { z } from \"zod\";\nexport const value = z.string();\n",
    )?;

    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
            "params": {
                "name": "ocentra_enforcer_check",
                "arguments": {
                    "root": fixture.path().to_string_lossy(),
                    "check": "no-zod-source",
                    "scope": "files",
                    "files": ["src/schema.ts"],
                },
            },
        }),
    )?;
    assert_eq!(reply["result"]["check"], serde_json::json!("no-zod-source"));
    assert_eq!(reply["result"]["ok"], serde_json::json!(false));
    assert!(reply["result"].get("error").is_none());
    assert!(reply["result"]["findings"]
        .as_array()
        .is_some_and(|findings| {
            findings.iter().any(|finding| {
                finding["ruleId"] == serde_json::json!("TS-1.2")
                    && finding["file"] == serde_json::json!("src/schema.ts")
            })
        }));

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_check_source_shape_reaches_the_native_policy_engine(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
            "params": {
                "name": "ocentra_enforcer_check",
                "arguments": {
                    "root": fixture.path().to_string_lossy(),
                    "check": "source-shape",
                },
            },
        }),
    )?;
    assert_eq!(reply["result"]["command"], serde_json::json!("check"));
    assert_eq!(reply["result"]["check"], serde_json::json!("source-shape"));
    assert_eq!(reply["result"]["ok"], serde_json::json!(true));
    assert!(reply["result"]["findings"]
        .as_array()
        .is_some_and(Vec::is_empty));

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_route_reaches_the_native_rust_route_engine() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = tempfile::tempdir()?;
    let src = fixture.path().join("src");
    std::fs::create_dir_all(&src)?;
    std::fs::write(src.join("lib.rs"), "pub struct RouteFixture;\n")?;

    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
            "params": {
                "name": "ocentra_enforcer_route",
                "arguments": {
                    "root": fixture.path().to_string_lossy(),
                    "scope": "files",
                    "files": ["src/lib.rs"],
                },
            },
        }),
    )?;
    assert_eq!(reply["result"]["ok"], serde_json::json!(true));
    assert_eq!(reply["result"]["languages"], serde_json::json!(["rust"]));
    assert!(reply["result"]["rulePacks"]
        .as_array()
        .is_some_and(|packs| packs.iter().any(|pack| pack == "rust")));

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_test_doctrine_reaches_the_native_rust_analyzer(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("tests"))?;
    std::fs::write(
        fixture.path().join("package.json"),
        r#"{ "dependencies": { "express": "1" } }"#,
    )?;
    std::fs::write(
        fixture.path().join("tests/unit.test.ts"),
        "it('works', () => {});",
    )?;

    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
            "params": {
                "name": "ocentra_enforcer_test_doctrine_scan",
                "arguments": { "root": fixture.path().to_string_lossy() },
            },
        }),
    )?;
    assert_eq!(reply["result"]["ok"], serde_json::json!(true));
    assert_eq!(
        reply["result"]["nature"]["isWebApi"],
        serde_json::json!(true)
    );
    assert_eq!(
        reply["result"]["detected"]["unit"]["present"],
        serde_json::json!(true)
    );
    assert!(
        reply["result"]["summary"]["categoriesMissing"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );

    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn stdio_smoke_ui_logic_coupling_reaches_the_native_advisory_analyzer(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("src/components"))?;
    std::fs::write(
        fixture.path().join("src/components/Orders.tsx"),
        "import { api } from '/lib/api';\napi.load();",
    )?;
    let binary = smoke_binary_path()?;
    let mut child = Command::new(binary)
        .arg("/abs/path/to/enforcer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);
    let reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "ocentra_enforcer_ui_logic_coupling_scan", "arguments": { "root": fixture.path().to_string_lossy() } } }),
    )?;
    assert_eq!(reply["result"]["ok"], true);
    assert_eq!(reply["result"]["rule"]["id"], "ARCH-1.16");
    assert_eq!(reply["result"]["summary"]["hardFindings"], 1);
    drop(stdin);
    assert!(child.wait()?.success());
    Ok(())
}
