use std::error::Error;
use std::io::Cursor;

use enforcer_memory::mcp;
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn send_ndjson(body: &Value) -> TestResult<Value> {
    let mut input = Cursor::new(format!("{body}\n").into_bytes());
    let mut output = Vec::new();
    mcp::run_stdio_session(&mut input, &mut output)?;
    let line = String::from_utf8(output)?;
    serde_json::from_str(line.trim_end()).map_err(Into::into)
}

#[test]
fn architecture_edges_keep_the_mcp_dependency_and_boundary_contract() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("api"))?;
    std::fs::create_dir_all(dir.path().join("core"))?;
    std::fs::write(
        dir.path().join("api/main.py"),
        "from core.lib import load\nload()\n",
    )?;
    std::fs::write(
        dir.path().join("core/lib.py"),
        "def load():\n    return None\n",
    )?;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "get_architecture",
            "arguments": {
                "repoPath": dir.path().to_string_lossy(),
                "aspects": ["dependencies", "boundaries"]
            }
        }
    });
    let reply = send_ndjson(&request)?;
    let structured = &reply["result"]["structuredContent"];
    assert_eq!(structured["ok"], json!(true));

    let dependencies = structured["dependencies"]
        .as_array()
        .ok_or("dependencies must be an array")?;
    let dependency = dependencies
        .iter()
        .find(|edge| edge["from"] == "api" && edge["to"] == "core")
        .ok_or("expected an api-to-core dependency")?;
    assert!(dependency["count"].as_u64().is_some_and(|count| count > 0));

    let boundaries = structured["boundaries"]
        .as_array()
        .ok_or("boundaries must be an array")?;
    let boundary = boundaries
        .iter()
        .find(|edge| edge["from"] == "api" && edge["to"] == "core")
        .ok_or("expected an api-to-core boundary")?;
    assert!(boundary["callCount"]
        .as_u64()
        .is_some_and(|count| count > 0));
    Ok(())
}
