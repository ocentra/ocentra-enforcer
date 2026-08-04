//! MCP boundary stale-server write-gate + `ocentra_enforcer_run` CLI fallback.
//!
//! Ported from `mcp/rust-rules-mcp-fallback.mjs`
//! (`shouldBlockStaleMcpTool`/`mcpStaleError`/`buildStaleFallback`) and
//! `mcp/rust-rules-mcp-context.mjs` (`COORDINATION_WRITE_TOOLS`,
//! `WRITE_ACTIONS_BY_TOOL`). This is a load-bearing fail-closed safety
//! invariant hit in real live smoke tests (workpack row) — when the
//! running server's code fingerprint disagrees with disk, or the
//! coordination hash-compat check fails, every coordination WRITE tool
//! must be REFUSED with a structured fallback pointing at the on-disk
//! CLI, never silently allowed to write a possibly-incompatible event.
//!
//! BOUNDARY-INVARIANT: raw tool arguments are decoded into canonical MCP
//! values before the stale-write policy is evaluated.
//!
//! # a02 fingerprint-over-running-artifact seam
//! [`Freshness`] is the seam this pack lays for a02 (owned elsewhere,
//! per the workpack's "a02 fingerprint-over-running-artifact" reference):
//! the actual fingerprint COMPUTATION (hash the running binary/build
//! artifact, compare to what is on disk) is NOT implemented here — this
//! module only consumes an already-resolved [`Freshness`] value and applies
//! the write-gate PREDICATE against it. A caller (the eventual `serve`
//! entry point, or a02 itself) is responsible for constructing
//! [`Freshness`] from the real running-artifact fingerprint.

use enforcer_domain::mcp_types::{
    ArtifactPath, McpActionName, McpExecutionMode, McpFreshness, McpToolName, McpWriteIntent,
};
use std::collections::BTreeSet;

/// Freshness/hash-compatibility verdict this gate consumes. Constructed
/// upstream (see module docs — the a02 seam); this module never computes
/// it, only branches on it.
///
/// Coordination tools that are ALWAYS a write when called at all (no
/// action/arg inspection needed) — ported verbatim from
/// `COORDINATION_WRITE_TOOLS`.
pub fn coordination_write_tools() -> BTreeSet<&'static str> {
    [
        "ocentra_enforcer_coordination_init",
        "ocentra_enforcer_coordination_claim",
        "ocentra_enforcer_coordination_closeout",
        "ocentra_enforcer_coordination_release",
        "ocentra_enforcer_coordination_report",
        "ocentra_enforcer_coordination_message",
        "ocentra_enforcer_coordination_sync",
        "ocentra_enforcer_coordination_ensure",
        "ocentra_enforcer_coordination_compact",
    ]
    .into_iter()
    .collect()
}

/// Tools whose write-ness depends on an `action` arg — ported verbatim
/// from `WRITE_ACTIONS_BY_TOOL`.
fn write_actions_for(tool: &str) -> Option<&'static [&'static str]> {
    match tool {
        "ocentra_enforcer_coordination_mail" => Some(&["send", "ack"]),
        "ocentra_enforcer_coordination_peer" => Some(&["add", "remove", "sync"]),
        _ => None,
    }
}

/// The minimal shape this gate needs from a tool call's args: whether
/// `write`/`dryRun`/`action` were present, without depending on the full
/// arg schema of every tool.
#[derive(Debug, Clone)]
pub struct GateArgs {
    pub write: McpWriteIntent,
    pub dry_run: McpExecutionMode,
    pub action: Option<McpActionName>,
    pub peek: bool,
}

impl Default for GateArgs {
    fn default() -> Self {
        Self {
            write: McpWriteIntent::Unspecified,
            dry_run: McpExecutionMode::Unspecified,
            action: None,
            peek: false,
        }
    }
}

/// The write-gate predicate. Mirrors `shouldBlockStaleMcpTool` exactly,
/// including the `coordination_repair` special case (workpack row: a
/// dry-run/read repair is NEVER gated; `write:true` OR `dryRun:false`
/// makes it a write).
pub fn should_block_stale_tool(
    name: &McpToolName,
    args: &GateArgs,
    freshness: McpFreshness,
) -> bool {
    if matches!(freshness, McpFreshness::Fresh) {
        return false;
    }
    if coordination_write_tools().contains(name.as_str()) {
        return true;
    }
    if name.as_str() == "ocentra_enforcer_coordination_repair" {
        return matches!(args.write, McpWriteIntent::Write)
            || matches!(args.dry_run, McpExecutionMode::Apply);
    }
    if name.as_str() == "ocentra_enforcer_coordination_notify" {
        return !args.peek;
    }
    match write_actions_for(name.as_str()) {
        Some(actions) => args
            .action
            .as_ref()
            .map(McpActionName::as_str)
            .map(str::to_ascii_lowercase)
            .is_some_and(|action| actions.contains(&action.as_str())),
        None => false,
    }
}

/// The structured fallback payload a gated call returns, pointing at the
/// on-disk `ocentra_enforcer_run` CLI. Field names are camelCase to match
/// the legacy `.mjs` fallback shape byte-for-byte (agents/tooling already
/// parse this shape).
// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// stale_fallback_dto_round_trip_preserves_refusal_contract
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized stale-server refusal and CLI fallback contract.
pub struct StaleFallbackDto {
    ok: bool,
    error: String,
    operation: String,
    direct_writes_allowed: bool,
    write_capable: bool,
    fallback_available: bool,
    reload_required: bool,
    fallback: FallbackCommandDto,
    next_step: String,
}

// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// stale_fallback_dto_round_trip_preserves_refusal_contract
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized command recommendation nested in a stale fallback response.
pub struct FallbackCommandDto {
    recommended_tool: String,
    command: Vec<String>,
    command_line: String,
}

impl From<FallbackCommandDto> for serde_json::Value {
    // NEGATIVE-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
    // malformed_fallback_command_is_rejected_before_domain_conversion
    fn from(value: FallbackCommandDto) -> Self {
        serde_json::json!({
            "recommendedTool": value.recommended_tool,
            "command": value.command,
            "commandLine": value.command_line,
        })
    }
}

impl From<StaleFallbackDto> for serde_json::Value {
    fn from(value: StaleFallbackDto) -> Self {
        serde_json::json!({
            "ok": value.ok,
            "error": value.error,
            "operation": value.operation,
            "directWritesAllowed": value.direct_writes_allowed,
            "writeCapable": value.write_capable,
            "fallbackAvailable": value.fallback_available,
            "reloadRequired": value.reload_required,
            "fallback": serde_json::Value::from(value.fallback),
            "nextStep": value.next_step,
        })
    }
}

/// Build the structured refusal for a gated call. `cli_path` is the
/// absolute path to the on-disk `enforcer` binary/CLI entry (caller
/// resolves this; this module does not know the pack root).
pub fn stale_fallback(
    name: &McpToolName,
    freshness: McpFreshness,
    cli_path: &ArtifactPath,
) -> StaleFallbackDto {
    let reason = if matches!(freshness, McpFreshness::HashIncompatible) {
        "coordination hash compatibility failed"
    } else {
        "MCP server is stale"
    };
    let command = vec![
        cli_path.as_str().to_owned(),
        "coordination".to_owned(),
        "run".to_owned(),
    ];
    let command_line = command.join(" ");
    StaleFallbackDto {
        ok: false,
        error: format!(
            "{reason}; refusing {name} because it may write incompatible coordination events."
        ),
        operation: name.to_string(),
        direct_writes_allowed: false,
        write_capable: false,
        fallback_available: true,
        reload_required: true,
        fallback: FallbackCommandDto {
            recommended_tool: "ocentra_enforcer_run".to_owned(),
            command,
            command_line,
        },
        next_step:
            "Restart the MCP client, or call ocentra_enforcer_run with the fallback command."
                .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{coordination_write_tools, should_block_stale_tool, stale_fallback, GateArgs};
    use enforcer_domain::mcp_types::{ArtifactPath, McpFreshness, McpToolName};
    use enforcer_domain::mcp_types::{McpActionName, McpExecutionMode, McpWriteIntent};
    use std::path::Path;

    fn tool(
        value: &str,
    ) -> Result<McpToolName, enforcer_domain::boundary::decode_error::DecodeError> {
        McpToolName::try_new(value)
    }

    #[test]
    fn fail_fixture_stale_server_refuses_a_write_tool() -> Result<(), Box<dyn std::error::Error>> {
        let blocked = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_claim")?,
            &GateArgs::default(),
            McpFreshness::Stale,
        );
        assert!(
            blocked,
            "a stale server must refuse a coordination WRITE tool"
        );
        let fallback = stale_fallback(
            &tool("ocentra_enforcer_coordination_claim")?,
            McpFreshness::Stale,
            &ArtifactPath::from_path(Path::new("/abs/path/to/enforcer")),
        );
        assert_eq!(fallback.fallback.recommended_tool, "ocentra_enforcer_run");
        assert!(fallback.reload_required);
        Ok(())
    }

    #[test]
    fn pass_fixture_fresh_server_dispatches_the_same_write_tool(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let blocked = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_claim")?,
            &GateArgs::default(),
            McpFreshness::Fresh,
        );
        assert!(
            !blocked,
            "a fresh, hash-compatible server must dispatch the write tool"
        );
        Ok(())
    }

    #[test]
    fn read_only_tool_is_never_gated_even_while_stale() -> Result<(), Box<dyn std::error::Error>> {
        let blocked = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_status")?,
            &GateArgs::default(),
            McpFreshness::Stale,
        );
        assert!(!blocked, "read-only tools are never gated");
        let blocked_status = should_block_stale_tool(
            &tool("ocentra_enforcer_mcp_status")?,
            &GateArgs::default(),
            McpFreshness::Stale,
        );
        assert!(!blocked_status, "mcp_status is never gated");
        Ok(())
    }

    #[test]
    fn repair_dry_run_is_allowed_on_a_stale_server() -> Result<(), Box<dyn std::error::Error>> {
        let dry_run_default = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_repair")?,
            &GateArgs::default(),
            McpFreshness::Stale,
        );
        assert!(
            !dry_run_default,
            "dryRun unset defaults to allowed (not a write)"
        );

        let dry_run_true = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_repair")?,
            &GateArgs {
                dry_run: McpExecutionMode::DryRun,
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        );
        assert!(!dry_run_true, "explicit dryRun:true stays allowed");
        Ok(())
    }

    #[test]
    fn repair_real_write_is_refused_on_a_stale_server() -> Result<(), Box<dyn std::error::Error>> {
        let write_true = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_repair")?,
            &GateArgs {
                write: McpWriteIntent::Write,
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        );
        assert!(write_true, "write:true must be refused while stale");

        let dry_run_false = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_repair")?,
            &GateArgs {
                dry_run: McpExecutionMode::Apply,
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        );
        assert!(dry_run_false, "dryRun:false must be refused while stale");
        Ok(())
    }

    #[test]
    fn notify_only_bypasses_the_stale_write_gate_when_peeking(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tool = tool("ocentra_enforcer_coordination_notify")?;
        assert!(should_block_stale_tool(
            &tool,
            &GateArgs::default(),
            McpFreshness::Stale,
        ));
        assert!(!should_block_stale_tool(
            &tool,
            &GateArgs {
                peek: true,
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        ));
        Ok(())
    }

    #[test]
    fn mail_action_gate_honors_write_actions_by_tool() -> Result<(), Box<dyn std::error::Error>> {
        let send_blocked = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_mail")?,
            &GateArgs {
                action: Some(McpActionName::try_new("send")?),
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        );
        assert!(send_blocked, "mail:send is a write action");

        let inbox_allowed = should_block_stale_tool(
            &tool("ocentra_enforcer_coordination_mail")?,
            &GateArgs {
                action: Some(McpActionName::try_new("inbox")?),
                ..GateArgs::default()
            },
            McpFreshness::Stale,
        );
        assert!(!inbox_allowed, "mail:inbox is not a write action");
        Ok(())
    }

    #[test]
    fn hash_incompatible_reason_string_differs_from_plain_stale(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stale = stale_fallback(
            &tool("ocentra_enforcer_coordination_claim")?,
            McpFreshness::Stale,
            &ArtifactPath::from_path(Path::new("/x")),
        );
        let incompatible = stale_fallback(
            &tool("ocentra_enforcer_coordination_claim")?,
            McpFreshness::HashIncompatible,
            &ArtifactPath::from_path(Path::new("/x")),
        );
        assert_eq!(
            stale.error,
            "MCP server is stale; refusing ocentra_enforcer_coordination_claim because it may write incompatible coordination events."
        );
        assert_eq!(
            incompatible.error,
            "coordination hash compatibility failed; refusing ocentra_enforcer_coordination_claim because it may write incompatible coordination events."
        );
        Ok(())
    }

    #[test]
    fn coordination_write_tools_set_matches_legacy_membership() {
        let set = coordination_write_tools();
        let expected = [
            "ocentra_enforcer_coordination_init",
            "ocentra_enforcer_coordination_claim",
            "ocentra_enforcer_coordination_closeout",
            "ocentra_enforcer_coordination_release",
            "ocentra_enforcer_coordination_report",
            "ocentra_enforcer_coordination_message",
            "ocentra_enforcer_coordination_sync",
            "ocentra_enforcer_coordination_ensure",
            "ocentra_enforcer_coordination_compact",
        ]
        .into_iter()
        .collect();
        assert_eq!(set, expected);
    }
}
