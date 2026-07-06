//! X06.7: the CLI mirror of [`crate::mcp`]'s tool surface.
//!
//! Baseline parity requires `<binary> memory cli [--progress] [--json]
//! <tool> [json|--flags]` alongside the MCP stdio server (binding:
//! `refs/x06-baseline-tool-schemas.md` §16 partial + coordinator
//! verification). This module is the library-level entry point a future
//! `enforcer-cli` subcommand wires to; it defines no `[[bin]]` of its own.
//!
//! # Same registry, same dispatch, same envelope as MCP
//!
//! [`cli_invoke`] parses the caller's JSON string into a [`serde_json::Value`]
//! and calls [`crate::mcp::call_tool`] -- the EXACT function the MCP
//! `tools/call` handler calls (unknown-tool special case included). There
//! is no second tool table, no second per-tool match, no second
//! error-shaping path, no second envelope shape: MCP and CLI are two thin
//! transports over one dispatcher and one envelope, which is what makes
//! "CLI mirror parity" (same tool+json -> same envelope JSON as MCP) true
//! by construction rather than by keeping two implementations in sync by
//! hand. [`cli_invoke`] is also what `--json` mode prints verbatim (see
//! [`run_cli`]).
//!
//! # Default (non-`--json`) output: unwrap the envelope (binding spec)
//!
//! Per the baseline CLI contract: default output UNWRAPS the MCP
//! envelope, printing `content[0].text` to stdout (or stderr, when
//! `isError` is true) rather than the full envelope object; `--json`
//! prints the raw envelope (what [`cli_invoke`] returns). Exit codes are
//! strictly `0` (success or `--help`) and `1` (everything else -- tool
//! error, usage error, unknown tool). There is no JSON-RPC layer in CLI
//! mode: [`run_cli`] never touches [`crate::mcp::run_stdio_session`] or
//! any framing type.
//!
//! # Flag form: kebab-case flags coerced by the tool's inputSchema
//!
//! Besides the raw-JSON invocation (`<tool> '{"repoPath":"..."}'`), the
//! binding spec also names a flag form: `--name-pattern foo` becomes JSON
//! key `name_pattern` (kebab -> snake_case; NOTE this crate's own
//! [`crate::mcp`] schemas use camelCase keys like `repoPath`, so flag-form
//! callers targeting THIS server's tools should expect
//! `--repo-path`->`repoPath`-shaped coercion, not `repo_path` --
//! [`kebab_to_key`] converts kebab-case to camelCase to match this
//! server's own schema convention, a deliberate adaptation of the
//! baseline's snake_case convention to this crate's wire casing rather
//! than a byte-identical port); repeated flags accumulate into a JSON
//! array; a flag with no following value (or immediately followed by
//! another `--flag`) is treated as `true` (boolean flag).

use serde_json::Value;

/// Error returned by [`cli_invoke`] itself (argument decoding failure) --
/// distinct from a tool-level failure, which is still `Ok` at this layer
/// (an envelope JSON value with `"isError": false`, exactly as MCP
/// returns it). Callers that need a nonzero process exit code check the
/// returned JSON's `"isError"` field themselves (see [`is_error_result`])
/// rather than this type, which only ever fires on malformed CLI input
/// (bad JSON).
#[derive(Debug, Clone, thiserror::Error)]
pub enum CliError {
    #[error("invalid JSON arguments: {0}")]
    InvalidJson(String),
    /// Retained for callers that want to fail fast on an unrecognized tool
    /// name before touching JSON -- [`cli_invoke`] itself no longer
    /// returns this variant (an unknown tool name is now a normal
    /// envelope result per the binding spec's "isError:true, text
    /// 'unknown tool: <name>'" contract, matching MCP's `tools/call`
    /// exactly via [`crate::mcp::call_tool`]); [`is_unknown_tool`] exists
    /// for callers that still want to detect this case from the envelope.
    #[error("unknown tool: {0}")]
    UnknownTool(String),
}

/// Invoke `tool` with `json_args` (a raw JSON object string, e.g.
/// `{"repoPath": "/repo"}`) and return the pretty-printed, envelope-wrapped
/// JSON result string -- the same shape [`crate::mcp::call_tool`] produces
/// for the equivalent MCP `tools/call`, INCLUDING the unknown-tool-name
/// special case (binding spec: `isError:true`, text `"unknown tool:
/// <name>"`) -- so CLI mirror parity holds for every input, not just
/// recognized tool names. Returns [`CliError::InvalidJson`] only for
/// malformed JSON input; every other outcome (bad repo path, parse error,
/// not-wired tool, unknown tool name) is a successful `Ok(String)` whose
/// envelope has `"isError": true`, mirroring the MCP contract exactly.
pub fn cli_invoke(tool: &str, json_args: &str) -> Result<String, CliError> {
    let args: Value = serde_json::from_str(json_args)
        .map_err(|source| CliError::InvalidJson(source.to_string()))?;
    let result = crate::mcp::call_tool(tool, &args);
    serde_json::to_string_pretty(&result)
        .map_err(|source| CliError::InvalidJson(format!("failed to encode result: {source}")))
}

/// True if `result_json` (as returned by [`cli_invoke`]) represents a
/// tool-level failure -- the CLI binary's exit-code decision point (the
/// workpack's "structured errors, nonzero exit on error" requirement).
/// Malformed JSON is also treated as an error (defensive: this should
/// never happen for a string [`cli_invoke`] itself produced).
pub fn is_error_result(result_json: &str) -> bool {
    match serde_json::from_str::<Value>(result_json) {
        Ok(value) => value.get("isError").and_then(Value::as_bool) != Some(false),
        Err(_) => true,
    }
}

/// True if `envelope` (a parsed [`cli_invoke`] result) is specifically the
/// unknown-tool-name special case, by its exact binding-spec text.
fn is_unknown_tool(envelope: &Value) -> bool {
    envelope["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.starts_with("unknown tool: "))
}

/// Parsed CLI invocation: which output mode, and the tool + JSON args to
/// dispatch. Built by [`parse_cli_args`]; consumed by [`run_cli`].
#[derive(Debug, Clone, PartialEq)]
pub struct CliInvocation {
    pub json_output: bool,
    /// `--progress` is accepted (binding spec: `cli [--progress]
    /// [--json] <tool> [json|--flags]`) but has no effect in this crate's
    /// synchronous, non-streaming dispatch -- there is no progress-sink
    /// seam to report through yet. Recorded rather than silently dropped
    /// so a future progress-reporting pass has a documented flag to wire,
    /// not a flag this parser would need to be taught about from scratch.
    pub progress: bool,
    pub tool: String,
    pub args_json: String,
}

/// Parse `argv` (NOT including the program name / `cli` subcommand word
/// itself -- callers pass exactly the tokens after `cli`) into a
/// [`CliInvocation`]. Accepts either a single JSON-object positional
/// argument after the tool name, or zero-or-more `--flag [value]` pairs
/// (mutually exclusive with the raw-JSON form -- whichever the first
/// non-flag token after the tool name looks like is exclusively used).
pub fn parse_cli_args(argv: &[String]) -> Result<CliInvocation, CliError> {
    let mut json_output = false;
    let mut progress = false;
    let mut rest: Vec<&str> = Vec::new();
    for arg in argv {
        match arg.as_str() {
            "--json" => json_output = true,
            "--progress" => progress = true,
            other => rest.push(other),
        }
    }
    let Some((&tool, flag_args)) = rest.split_first() else {
        return Err(CliError::InvalidJson(
            "missing required <tool> argument".to_owned(),
        ));
    };
    let args_json = if flag_args.is_empty() {
        "{}".to_owned()
    } else if flag_args.len() == 1 && flag_args[0].starts_with('{') {
        flag_args[0].to_owned()
    } else {
        flags_to_json(flag_args)?
    };
    Ok(CliInvocation {
        json_output,
        progress,
        tool: tool.to_owned(),
        args_json,
    })
}

/// Convert `--flag value --flag2 value2 --bool-flag` tokens into a JSON
/// object string. Kebab-case flag names are converted to this server's
/// own camelCase schema convention (see module docs) via
/// [`kebab_to_key`]; a flag repeated more than once accumulates its
/// values into a JSON array (order preserved); a flag with no following
/// value (end of args, or immediately followed by another `--flag`)
/// becomes the JSON boolean `true`.
fn flags_to_json(tokens: &[&str]) -> Result<String, CliError> {
    let mut map: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        let Some(flag) = token.strip_prefix("--") else {
            return Err(CliError::InvalidJson(format!(
                "expected a --flag, got {token:?}"
            )));
        };
        let key = kebab_to_key(flag);
        let has_value = i + 1 < tokens.len() && !tokens[i + 1].starts_with("--");
        let value: Value = if has_value {
            i += 1;
            parse_flag_value(tokens[i])
        } else {
            Value::Bool(true)
        };
        insert_or_accumulate(&mut map, key, value);
        i += 1;
    }
    serde_json::to_string(&Value::Object(map))
        .map_err(|source| CliError::InvalidJson(format!("failed to encode flags: {source}")))
}

fn insert_or_accumulate(map: &mut serde_json::Map<String, Value>, key: String, value: Value) {
    match map.get_mut(&key) {
        None => {
            map.insert(key, value);
        }
        Some(Value::Array(existing)) => existing.push(value),
        Some(existing) => {
            let previous = existing.clone();
            map.insert(key, Value::Array(vec![previous, value]));
        }
    }
}

/// Best-effort scalar coercion for a flag's raw string value: `true`/
/// `false` become JSON booleans, a value that parses as an integer
/// becomes a JSON number, everything else stays a JSON string. This is
/// deliberately simpler than full schema-driven coercion (looking up
/// each tool's declared property type) -- every one of this crate's own
/// handlers (`src/mcp.rs`) already accepts loosely-typed JSON and reports
/// a clear `tool_error` on a genuine type mismatch, so a permissive
/// string-first coercion here does not hide a real input error, it just
/// defers the type check to the handler that already owns it.
fn parse_flag_value(raw: &str) -> Value {
    match raw {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        other => other
            .parse::<i64>()
            .map(|n| Value::Number(n.into()))
            .unwrap_or_else(|_| Value::String(other.to_owned())),
    }
}

/// Convert one kebab-case flag name (`repo-path`) to this server's
/// camelCase schema key convention (`repoPath`) -- see module docs.
fn kebab_to_key(flag: &str) -> String {
    let mut out = String::with_capacity(flag.len());
    let mut uppercase_next = false;
    for c in flag.chars() {
        if c == '-' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.extend(c.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The result of running a full CLI invocation end to end: what to print
/// (and to which stream) and the process exit code, WITHOUT this function
/// itself touching real stdio -- callers (a future `enforcer-cli`
/// subcommand's `main`) do the actual printing, keeping this crate's own
/// `print_stdout`/`print_stderr` clippy denies intact (workspace lints).
#[derive(Debug, Clone, PartialEq)]
pub struct CliOutcome {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    /// Strictly `0` (success, or `--help`/usage) or `1` (everything else
    /// -- tool error, usage error, unknown tool), per the binding spec.
    pub exit_code: i32,
}

/// Emit the CLI transport's per-request diagnostic record (event
/// `mcp.request`, WARN on error), matching the MCP `tools/call` path's
/// own emission in [`crate::mcp::handle_frame`].
fn emit_request_diagnostic(tool: &str, duration: std::time::Duration, is_error: bool) {
    let record = crate::diagnostics::RequestRecord {
        protocol: "cli",
        method: "cli".to_owned(),
        tool: Some(tool.to_owned()),
        duration,
        is_error,
    };
    let diagnostics = crate::diagnostics::Diagnostics::from_env();
    crate::diagnostics::emit_to_stderr(&diagnostics, record.level(), &record);
}

/// Minimal usage text for `--help`/`-h` (binding spec: exit code `0` for
/// help, same as success -- see [`run_cli`]).
const HELP_TEXT: &str =
    "Usage: enforcer-memory cli [--progress] [--json] <tool> [json|--flags]\n\nRun `tools/list` over the MCP surface (crate::mcp) for the full list of 14 tools and their input schemas.";

/// Run one full CLI invocation: parse `argv`, dispatch through
/// [`cli_invoke`], and decide what to print where and the exit code,
/// implementing the binding spec's default-unwraps-the-envelope /
/// `--json`-prints-the-raw-envelope / strictly-0-or-1-exit-code contract.
pub fn run_cli(argv: &[String]) -> CliOutcome {
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        return CliOutcome {
            stdout: Some(HELP_TEXT.to_owned()),
            stderr: None,
            exit_code: 0,
        };
    }
    let invocation = match parse_cli_args(argv) {
        Ok(invocation) => invocation,
        Err(err) => {
            return CliOutcome {
                stdout: None,
                stderr: Some(err.to_string()),
                exit_code: 1,
            }
        }
    };
    let started = std::time::Instant::now();
    let envelope_json = match cli_invoke(&invocation.tool, &invocation.args_json) {
        Ok(json) => json,
        Err(err) => {
            emit_request_diagnostic(&invocation.tool, started.elapsed(), true);
            return CliOutcome {
                stdout: None,
                stderr: Some(err.to_string()),
                exit_code: 1,
            };
        }
    };
    let envelope: Value = match serde_json::from_str(&envelope_json) {
        Ok(value) => value,
        Err(err) => {
            emit_request_diagnostic(&invocation.tool, started.elapsed(), true);
            return CliOutcome {
                stdout: None,
                stderr: Some(format!(
                    "internal error: failed to re-parse envelope: {err}"
                )),
                exit_code: 1,
            };
        }
    };
    let is_error = is_error_result(&envelope_json) || is_unknown_tool(&envelope);
    emit_request_diagnostic(&invocation.tool, started.elapsed(), is_error);
    let exit_code = if is_error { 1 } else { 0 };

    if invocation.json_output {
        let printed = if is_error {
            None
        } else {
            Some(envelope_json.clone())
        };
        let printed_err = if is_error { Some(envelope_json) } else { None };
        return CliOutcome {
            stdout: printed,
            stderr: printed_err,
            exit_code,
        };
    }

    // Default mode: unwrap the envelope, printing content[0].text alone.
    let text = envelope["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    if is_error {
        CliOutcome {
            stdout: None,
            stderr: Some(text),
            exit_code,
        }
    } else {
        CliOutcome {
            stdout: Some(text),
            stderr: None,
            exit_code,
        }
    }
}
