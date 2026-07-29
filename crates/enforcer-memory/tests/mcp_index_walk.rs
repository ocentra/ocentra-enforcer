//! Regression coverage for the MCP repository walk's generated-tree filter.

use std::error::Error;
use std::io::Cursor;

use enforcer_memory::mcp;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

fn call_tool(name: &str, arguments: &Value) -> Result<Value, Box<dyn Error>> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    });
    let mut input = Cursor::new(format!("{request}\n").into_bytes());
    let mut output = Vec::new();
    mcp::run_stdio_session(&mut input, &mut output)?;
    let reply = String::from_utf8(output)?;
    Ok(serde_json::from_str(
        reply
            .lines()
            .next()
            .ok_or("MCP response must contain one line")?,
    )?)
}

#[test]
fn index_repository_and_package_discovery_skip_nested_target_trees() -> TestResult {
    let repo = tempfile::tempdir()?;
    std::fs::create_dir_all(repo.path().join("target/generated"))?;
    std::fs::create_dir_all(repo.path().join("nested/target-generated/deeper"))?;
    std::fs::create_dir_all(repo.path().join(".tmp-index"))?;
    std::fs::create_dir_all(repo.path().join("targeting"))?;
    std::fs::create_dir_all(repo.path().join("crates/real/src"))?;
    std::fs::write(
        repo.path().join("target/generated/Cargo.toml"),
        "[package]\nname = \"generated\"\nversion = \"0.1.0\"\n",
    )?;
    std::fs::write(
        repo.path().join("target/generated/generated.rs"),
        "fn generated() {}\n",
    )?;
    std::fs::write(
        repo.path()
            .join("nested/target-generated/deeper/Cargo.toml"),
        "[package]\nname = \"generated-nested\"\nversion = \"0.1.0\"\n",
    )?;
    std::fs::write(
        repo.path()
            .join("nested/target-generated/deeper/generated.rs"),
        "fn generated_nested() {}\n",
    )?;
    std::fs::write(
        repo.path().join(".tmp-index/transient.rs"),
        "fn transient() {}\n",
    )?;
    std::fs::write(repo.path().join("targeting/real.rs"), "fn real() {}\n")?;
    std::fs::write(
        repo.path().join("crates/real/Cargo.toml"),
        "[package]\nname = \"real\"\nversion = \"0.1.0\"\n",
    )?;
    std::fs::write(
        repo.path().join("crates/real/src/lib.rs"),
        "fn library() {}\n",
    )?;

    let reply = call_tool(
        "index_repository",
        &json!({ "repoPath": repo.path().to_string_lossy() }),
    )?;
    let structured = &reply["result"]["structuredContent"];
    assert_eq!(structured["ok"], json!(true));
    assert_eq!(structured["filesIndexed"], json!(3));

    let architecture = call_tool(
        "get_architecture",
        &json!({
            "repoPath": repo.path().to_string_lossy(),
            "aspects": ["packages"]
        }),
    )?;
    let packages = architecture["result"]["structuredContent"]["packages"]
        .as_array()
        .ok_or("package discovery must return an array")?;
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["manifestRelPath"], "crates/real/Cargo.toml");
    Ok(())
}
