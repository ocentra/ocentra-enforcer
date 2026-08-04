//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! The ONE stdio-protocol boundary sink module in this crate.
//!
//! Per the workpack: "Confine all stdout/stderr writes to ONE
//! stdio-protocol sink module... carrying a scoped, documented
//! `#![allow(clippy::print_stdout, clippy::print_stderr)]` at module
//! scope; every other module obeys the `[workspace.lints]` deny wall."
//! This module is that sink: it owns the actual `Read`/`Write` against
//! real stdio handles, driving [`crate::transport`]'s pure framing logic
//! and [`crate::router`]'s dispatch. No other module in this crate may
//! write to stdout/stderr.

use std::io::{Read, Write};

use crate::boundary::rpc_request::RpcMessageDto;
use crate::boundary::rpc_response::{RpcErrorDto, RpcResultDto};
use crate::router::{self, DispatchContext, DispatchOutcome};
use crate::transport::{encode_frame, Frame, FrameReader, Framing};
use enforcer_domain::mcp_types::ArtifactPath;
use enforcer_domain::mcp_types::McpFreshness;
use enforcer_domain::mcp_types::McpToolName;
use enforcer_domain::mcp_types::{RpcErrorBody, RpcErrorCode, RpcErrorMessage};

/// Run the MCP stdio server loop against real `stdin`/`stdout`, blocking
/// until stdin closes. `ctx` carries the freshness verdict (a02 seam) and
/// the on-disk CLI path used in stale-refusal fallbacks.
///
/// # Errors
/// Returns an I/O error only for a genuine stdin/stdout failure; malformed
/// client messages are reported back over the wire as JSON-RPC errors, not
/// propagated as a Rust `Err`.
pub fn run_stdio_server(ctx: &DispatchContext) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut reader = FrameReader::new();
    let mut buf = [0_u8; 4096];
    loop {
        let read = handle.read(&mut buf)?;
        if read == 0 {
            return Ok(());
        }
        let chunk = buf.get(..read).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stdio read length exceeded the receive buffer",
            )
        })?;
        for frame in reader.push(chunk) {
            handle_frame(&frame, ctx, &mut out)?;
        }
    }
}

/// Handle one already-framed message: parse, dispatch, write the reply in
/// the SAME framing it arrived in. Exposed at module visibility so
/// in-crate integration tests can drive it against an in-memory
/// `Vec<u8>` writer without a real stdio handle.
pub(crate) fn handle_frame(
    frame: &Frame,
    ctx: &DispatchContext,
    out: &mut impl Write,
) -> std::io::Result<()> {
    let message: RpcMessageDto = match serde_json::from_str(&frame.body) {
        Ok(message) => message,
        Err(err) => {
            let error = RpcErrorDto::new(
                serde_json::Value::Null,
                rpc_error(RpcErrorCode::ParseError, format!("Parse error: {err}")),
            );
            return write_reply(out, &error, frame.framing);
        }
    };
    if message.is_notification() {
        return Ok(());
    }
    let Some(id) = message.id.clone() else {
        // A request with no id and a non-`notifications/` method is
        // malformed; there is nothing to reply to.
        return Ok(());
    };
    match handle_method(&message, ctx) {
        Ok(result) => write_reply(out, &RpcResultDto::new(id, result), frame.framing),
        Err(error) => write_reply(out, &RpcErrorDto::new(id, error), frame.framing),
    }
}

fn handle_method(
    message: &RpcMessageDto,
    ctx: &DispatchContext,
) -> Result<serde_json::Value, RpcErrorBody> {
    let params = message.params.clone().unwrap_or(serde_json::Value::Null);
    match message.method.as_str() {
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(serde_json::json!({})),
        "tools/list" => Ok(serde_json::json!({
            "tools": crate::registry::build_tool_descriptors(),
        })),
        "tools/call" => Ok(handle_tools_call(&params, ctx)),
        "resources/list" => Ok(serde_json::json!({ "resources": [] })),
        "resources/templates/list" => Ok(serde_json::json!({ "resourceTemplates": [] })),
        "prompts/list" => Ok(serde_json::json!({ "prompts": [] })),
        "shutdown" => Ok(serde_json::Value::Null),
        other => Err(rpc_error(
            RpcErrorCode::MethodNotFound,
            format!("Unknown method: {other}"),
        )),
    }
}

fn rpc_error(code: RpcErrorCode, message: impl Into<String>) -> RpcErrorBody {
    let message = message.into();
    match RpcErrorMessage::try_new(&message) {
        Ok(message) => RpcErrorBody::new(code, message),
        Err(_) => RpcErrorBody::new(code, RpcErrorMessage::fallback()),
    }
}

fn initialize_result(params: &serde_json::Value) -> serde_json::Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("2024-11-05");
    serde_json::json!({
        "protocolVersion": protocol_version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": crate::name::SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

fn handle_tools_call(params: &serde_json::Value, ctx: &DispatchContext) -> serde_json::Value {
    let name = params
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let empty_args = serde_json::Value::Object(serde_json::Map::new());
    let args = params.get("arguments").unwrap_or(&empty_args);
    let Ok(name) = McpToolName::try_new(name) else {
        return serde_json::json!({ "ok": false, "error": "tools/call requires a non-empty name" });
    };
    match router::dispatch(&name, args, ctx) {
        DispatchOutcome::Result(value) => value,
        DispatchOutcome::UnknownTool => serde_json::json!({
            "ok": false,
            "error": format!("Unknown tool: {name}"),
        }),
        DispatchOutcome::StaleRefused(fallback) => serde_json::Value::from(*fallback),
    }
}

fn write_reply(
    out: &mut impl Write,
    reply: &impl serde::Serialize,
    framing: Framing,
) -> std::io::Result<()> {
    let body = serde_json::to_string(reply).unwrap_or_else(|_| {
        "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"encode failure\"}}"
            .to_owned()
    });
    out.write_all(&encode_frame(&body, framing))?;
    out.flush()
}

/// Capture the running server artifact at startup so every subsequent tool
/// call can fail closed if the executable is replaced on disk.
pub fn default_dispatch_context(cli_path: impl AsRef<std::path::Path>) -> DispatchContext {
    let startup_fingerprint = crate::fingerprint::build_running_mcp_fingerprint(None).ok();
    DispatchContext {
        freshness: if startup_fingerprint.is_some() {
            McpFreshness::Fresh
        } else {
            McpFreshness::HashIncompatible
        },
        startup_fingerprint,
        cli_path: ArtifactPath::from_path(cli_path.as_ref()),
        validation_history: std::sync::Arc::new(std::sync::Mutex::new(
            crate::validation_history::ValidationHistory::default(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{default_dispatch_context, handle_frame};
    use crate::transport::{Frame, Framing};

    #[test]
    fn pass_fixture_canned_request_over_the_transport_yields_expected_tool_result(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ctx = default_dispatch_context("/abs/enforcer");
        let frame = Frame {
            body: serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": "ocentra_enforcer_mcp_status", "arguments": {} },
            })
            .to_string(),
            framing: Framing::Ndjson,
        };
        let mut out = Vec::new();
        handle_frame(&frame, &ctx, &mut out)?;
        let reply: serde_json::Value =
            serde_json::from_slice(out.strip_suffix(b"\n").unwrap_or(&out))?;
        assert_eq!(reply["result"]["ok"], serde_json::json!(true));
        Ok(())
    }

    #[test]
    fn fail_fixture_malformed_request_is_rejected_with_a_proper_error_frame(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ctx = default_dispatch_context("/abs/enforcer");
        let frame = Frame {
            body: "{not valid json".to_owned(),
            framing: Framing::Ndjson,
        };
        let mut out = Vec::new();
        handle_frame(&frame, &ctx, &mut out)?;
        let reply: serde_json::Value =
            serde_json::from_slice(out.strip_suffix(b"\n").unwrap_or(&out))?;
        assert_eq!(reply["error"]["code"], serde_json::json!(-32700));
        Ok(())
    }

    #[test]
    fn unknown_method_yields_method_not_found_error_frame() -> Result<(), Box<dyn std::error::Error>>
    {
        let ctx = default_dispatch_context("/abs/enforcer");
        let frame = Frame {
            body: serde_json::json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "totally/unknown",
            })
            .to_string(),
            framing: Framing::ContentLength,
        };
        let mut out = Vec::new();
        handle_frame(&frame, &ctx, &mut out)?;
        let text = String::from_utf8(out)?;
        let body_start = text.find("\r\n\r\n").map(|at| at + 4).unwrap_or(0);
        let reply: serde_json::Value = serde_json::from_str(&text[body_start..])?;
        assert_eq!(reply["error"]["code"], serde_json::json!(-32601));
        Ok(())
    }

    #[test]
    fn notification_produces_no_reply() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = default_dispatch_context("/abs/enforcer");
        let frame = Frame {
            body: serde_json::json!({ "method": "notifications/initialized" }).to_string(),
            framing: Framing::Ndjson,
        };
        let mut out = Vec::new();
        handle_frame(&frame, &ctx, &mut out)?;
        assert!(
            out.is_empty(),
            "a notification must never produce a reply frame"
        );
        Ok(())
    }

    #[test]
    fn initialize_and_tools_list_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = default_dispatch_context("/abs/enforcer");
        let init_frame = Frame {
            body: serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })
                .to_string(),
            framing: Framing::Ndjson,
        };
        let mut out = Vec::new();
        handle_frame(&init_frame, &ctx, &mut out)?;
        let reply: serde_json::Value =
            serde_json::from_slice(out.strip_suffix(b"\n").unwrap_or(&out))?;
        assert_eq!(
            reply["result"]["serverInfo"]["name"],
            serde_json::json!(crate::name::SERVER_NAME)
        );

        let list_frame = Frame {
            body: serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })
                .to_string(),
            framing: Framing::Ndjson,
        };
        let mut out2 = Vec::new();
        handle_frame(&list_frame, &ctx, &mut out2)?;
        let reply2: serde_json::Value =
            serde_json::from_slice(out2.strip_suffix(b"\n").unwrap_or(&out2))?;
        let tools = reply2["result"]["tools"]
            .as_array()
            .ok_or("tools must be an array")?;
        assert_eq!(
            tools.first().and_then(|tool| tool.get("name")),
            Some(&serde_json::json!("ocentra_enforcer_scan"))
        );
        Ok(())
    }
}
