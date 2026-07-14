//! X06.7 unit-shaped tests for [`enforcer_memory::cli`], moved out of
//! `src/cli.rs` per this crate's "no inline `#[cfg(test)]` modules" style
//! (workspace clippy denies `unwrap`/`expect`/`panic` even in test code,
//! so every assertion here goes through `Result` + `?` rather than the
//! original inline module's `.unwrap()`/`.expect(...)` calls).
//!
//! `ingest_traces` moved from "not wired" to a landed tool in this pass
//! (see `src/mcp.rs`'s `WIRED_TOOLS`), so the tests that used to exercise
//! its `not_wired` error payload as a stand-in "some tool call fails"
//! fixture now use a genuine tool-level error instead: `ingest_traces`
//! called with a `repoPath` that is not a directory.

use enforcer_memory::cli::{cli_invoke, is_error_result, parse_cli_args, run_cli, CliError};
use serde_json::Value;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

/// A minimal args payload that reaches `ingest_traces`' dispatch but
/// fails at `build_graph` (repoPath is not a directory) -- a genuine
/// tool-level error, distinct from the CLI-argument-decoding errors
/// [`CliError`] itself covers.
const FAILING_INGEST_TRACES_ARGS: &str =
    r#"{"repoPath":"/definitely/not/a/real/directory","traces":[]}"#;

#[test]
fn cli_invoke_unknown_tool_is_ok_with_the_exact_binding_text() -> TestResult {
    let output = cli_invoke("not_a_real_tool", "{}")?;
    assert!(is_error_result(&output));
    assert!(output.contains("unknown tool: not_a_real_tool"));
    Ok(())
}

#[test]
fn cli_invoke_rejects_malformed_json_arguments() {
    let err = cli_invoke("list_projects", "{not json");
    assert!(matches!(err, Err(CliError::InvalidJson(_))));
}

#[test]
fn cli_invoke_tool_level_error_is_ok_with_error_payload() -> TestResult {
    let output = cli_invoke("ingest_traces", FAILING_INGEST_TRACES_ARGS)?;
    assert!(is_error_result(&output));
    assert!(output.contains("is not a directory"));
    Ok(())
}

#[test]
fn is_error_result_false_for_a_genuine_ok_envelope() {
    assert!(!is_error_result(r#"{"isError": false}"#));
}

#[test]
fn is_error_result_true_for_malformed_json() {
    assert!(is_error_result("not json"));
}

#[test]
fn parse_cli_args_extracts_json_output_and_progress_flags_in_any_position() -> TestResult {
    let argv: Vec<String> = ["--json", "list_projects", "--progress", "{}"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let parsed = parse_cli_args(&argv)?;
    assert!(parsed.json_output);
    assert!(parsed.progress);
    assert_eq!(parsed.tool, "list_projects");
    assert_eq!(parsed.args_json, "{}");
    Ok(())
}

#[test]
fn parse_cli_args_missing_tool_is_an_error() {
    let argv: Vec<String> = ["--json"].iter().map(|s| s.to_string()).collect();
    assert!(parse_cli_args(&argv).is_err());
}

#[test]
fn kebab_flags_coerce_to_camel_case_keys_matching_this_servers_schema() -> TestResult {
    let argv: Vec<String> = [
        "index_repository",
        "--repo-path",
        "/tmp/repo",
        "--hotspot-limit",
        "5",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let parsed = parse_cli_args(&argv)?;
    let args: Value = serde_json::from_str(&parsed.args_json)?;
    assert_eq!(args["repoPath"], Value::String("/tmp/repo".to_owned()));
    assert_eq!(args["hotspotLimit"], Value::Number(5.into()));
    Ok(())
}

#[test]
fn boolean_flag_with_no_value_becomes_true() -> TestResult {
    let argv: Vec<String> = ["get_code_snippet", "--include-neighbors"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let parsed = parse_cli_args(&argv)?;
    let args: Value = serde_json::from_str(&parsed.args_json)?;
    assert_eq!(args["includeNeighbors"], Value::Bool(true));
    Ok(())
}

#[test]
fn adjacent_cli_flags_each_remain_boolean() -> TestResult {
    let argv: Vec<String> = ["get_code_snippet", "--include-neighbors", "--verbose"]
        .iter()
        .map(|value| value.to_string())
        .collect();
    let parsed = parse_cli_args(&argv)?;
    let args: Value = serde_json::from_str(&parsed.args_json)?;

    assert_eq!(
        args,
        serde_json::json!({"includeNeighbors": true, "verbose": true})
    );
    Ok(())
}

#[test]
fn repeated_flag_accumulates_into_an_array() -> TestResult {
    let argv: Vec<String> = ["ingest_traces", "--caller", "a", "--caller", "b"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let parsed = parse_cli_args(&argv)?;
    let args: Value = serde_json::from_str(&parsed.args_json)?;
    assert_eq!(
        args["caller"],
        Value::Array(vec![
            Value::String("a".to_owned()),
            Value::String("b".to_owned())
        ])
    );
    Ok(())
}

#[test]
fn run_cli_help_flag_exits_zero_without_dispatching_any_tool() {
    for flag in ["--help", "-h"] {
        let argv: Vec<String> = [flag].iter().map(|s| s.to_string()).collect();
        let outcome = run_cli(&argv);
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stdout.is_some());
        assert!(outcome.stderr.is_none());
    }
}

#[test]
fn run_cli_default_mode_prints_unwrapped_text_and_exits_zero_on_success() -> TestResult {
    let argv: Vec<String> = [
        "manage_adr",
        r#"{"operation":"create","id":"a","title":"t","adrs":[]}"#,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let outcome = run_cli(&argv);
    assert_eq!(outcome.exit_code, 0);
    let stdout = outcome.stdout.ok_or("expected stdout on success")?;
    assert!(outcome.stderr.is_none());
    // Unwrapped text is the tool's inner JSON string, not the envelope
    // object -- it must not itself contain the envelope's own keys.
    assert!(!stdout.contains("\"content\""));
    Ok(())
}

#[test]
fn run_cli_default_mode_prints_unwrapped_text_to_stderr_and_exits_one_on_error() -> TestResult {
    let argv: Vec<String> = ["ingest_traces", FAILING_INGEST_TRACES_ARGS]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let outcome = run_cli(&argv);
    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.stdout.is_none());
    let stderr = outcome.stderr.ok_or("expected stderr on error")?;
    assert!(stderr.contains("is not a directory"));
    Ok(())
}

#[test]
fn run_cli_json_mode_prints_the_raw_envelope() -> TestResult {
    let argv: Vec<String> = ["--json", "ingest_traces", FAILING_INGEST_TRACES_ARGS]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let outcome = run_cli(&argv);
    assert_eq!(outcome.exit_code, 1);
    let printed = outcome.stderr.ok_or("error envelope goes to stderr")?;
    let value: Value = serde_json::from_str(&printed)?;
    assert_eq!(value["isError"], Value::Bool(true));
    Ok(())
}

#[test]
fn run_cli_unknown_tool_exits_one_with_the_exact_binding_text() -> TestResult {
    let argv: Vec<String> = ["no_such_tool", "{}"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let outcome = run_cli(&argv);
    assert_eq!(outcome.exit_code, 1);
    let stderr = outcome.stderr.ok_or("expected stderr")?;
    assert!(stderr.contains("unknown tool: no_such_tool"));
    Ok(())
}

#[test]
fn run_cli_missing_tool_argument_is_a_usage_error_exit_one() {
    let outcome = run_cli(&[]);
    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.stderr.is_some());
}
