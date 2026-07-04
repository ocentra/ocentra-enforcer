//! Stale-server write-gate + `ocentra_enforcer_run` CLI fallback.
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
//! # a02 fingerprint-over-running-artifact seam
//! [`Freshness`] is the seam this pack lays for a02 (owned elsewhere,
//! per the workpack's "a02 fingerprint-over-running-artifact" reference):
//! the actual fingerprint COMPUTATION (hash the running binary/build
//! artifact, compare to what is on disk) is NOT implemented here — this
//! module only consumes an already-resolved [`Freshness`] value and applies
//! the write-gate PREDICATE against it. A caller (the eventual `serve`
//! entry point, or a02 itself) is responsible for constructing
//! [`Freshness`] from the real running-artifact fingerprint.

use std::collections::BTreeSet;

/// Freshness/hash-compatibility verdict this gate consumes. Constructed
/// upstream (see module docs — the a02 seam); this module never computes
/// it, only branches on it.
#[derive(Debug, Clone, Copy)]
pub struct Freshness {
    /// `true` once the running artifact's fingerprint matches disk AND the
    /// coordination hash-compat check (arc-16 source of truth) passes.
    /// Mirrors `directWritesAllowed === (!stale && hashCompatible)`.
    pub direct_writes_allowed: bool,
    /// `true` when the coordination hash-compat check specifically failed
    /// (used only to pick the right refusal reason string).
    pub hash_compatible: bool,
}

impl Freshness {
    /// A fresh, fully hash-compatible server — the common case, never
    /// gated.
    pub fn fresh() -> Self {
        Self {
            direct_writes_allowed: true,
            hash_compatible: true,
        }
    }

    /// A stale server (fingerprint mismatch), still hash-compatible.
    pub fn stale() -> Self {
        Self {
            direct_writes_allowed: false,
            hash_compatible: true,
        }
    }

    /// A server whose coordination hash-compat check itself failed.
    pub fn hash_incompatible() -> Self {
        Self {
            direct_writes_allowed: false,
            hash_compatible: false,
        }
    }
}

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
#[derive(Debug, Clone, Default)]
pub struct GateArgs {
    pub write: Option<bool>,
    pub dry_run: Option<bool>,
    pub action: Option<String>,
}

/// The write-gate predicate. Mirrors `shouldBlockStaleMcpTool` exactly,
/// including the `coordination_repair` special case (workpack row: a
/// dry-run/read repair is NEVER gated; `write:true` OR `dryRun:false`
/// makes it a write).
pub fn should_block_stale_tool(name: &str, args: &GateArgs, freshness: Freshness) -> bool {
    if freshness.direct_writes_allowed {
        return false;
    }
    if coordination_write_tools().contains(name) {
        return true;
    }
    if name == "ocentra_enforcer_coordination_repair" {
        return args.write == Some(true) || args.dry_run == Some(false);
    }
    match write_actions_for(name) {
        Some(actions) => args
            .action
            .as_deref()
            .map(str::to_ascii_lowercase)
            .is_some_and(|action| actions.contains(&action.as_str())),
        None => false,
    }
}

/// The structured fallback payload a gated call returns, pointing at the
/// on-disk `ocentra_enforcer_run` CLI. Field names are camelCase to match
/// the legacy `.mjs` fallback shape byte-for-byte (agents/tooling already
/// parse this shape).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleFallback {
    pub ok: bool,
    pub error: String,
    pub operation: String,
    pub direct_writes_allowed: bool,
    pub write_capable: bool,
    pub fallback_available: bool,
    pub reload_required: bool,
    pub fallback: FallbackCommand,
    pub next_step: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackCommand {
    pub recommended_tool: String,
    pub command: Vec<String>,
    pub command_line: String,
}

/// Build the structured refusal for a gated call. `cli_path` is the
/// absolute path to the on-disk `enforcer` binary/CLI entry (caller
/// resolves this; this module does not know the pack root).
pub fn stale_fallback(name: &str, freshness: Freshness, cli_path: &str) -> StaleFallback {
    let reason = if !freshness.hash_compatible {
        "coordination hash compatibility failed"
    } else {
        "MCP server is stale"
    };
    let command = vec![cli_path.to_owned(), "coordination".to_owned(), "run".to_owned()];
    let command_line = command.join(" ");
    StaleFallback {
        ok: false,
        error: format!("{reason}; refusing {name} because it may write incompatible coordination events."),
        operation: name.to_owned(),
        direct_writes_allowed: false,
        write_capable: false,
        fallback_available: true,
        reload_required: true,
        fallback: FallbackCommand {
            recommended_tool: "ocentra_enforcer_run".to_owned(),
            command,
            command_line,
        },
        next_step: "Restart the MCP client, or call ocentra_enforcer_run with the fallback command."
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        coordination_write_tools, should_block_stale_tool, stale_fallback, Freshness, GateArgs,
    };

    #[test]
    fn fail_fixture_stale_server_refuses_a_write_tool() {
        let blocked = should_block_stale_tool(
            "ocentra_enforcer_coordination_claim",
            &GateArgs::default(),
            Freshness::stale(),
        );
        assert!(blocked, "a stale server must refuse a coordination WRITE tool");
        let fallback = stale_fallback(
            "ocentra_enforcer_coordination_claim",
            Freshness::stale(),
            "/abs/path/to/enforcer",
        );
        assert_eq!(fallback.fallback.recommended_tool, "ocentra_enforcer_run");
        assert!(fallback.reload_required);
    }

    #[test]
    fn pass_fixture_fresh_server_dispatches_the_same_write_tool() {
        let blocked = should_block_stale_tool(
            "ocentra_enforcer_coordination_claim",
            &GateArgs::default(),
            Freshness::fresh(),
        );
        assert!(!blocked, "a fresh, hash-compatible server must dispatch the write tool");
    }

    #[test]
    fn read_only_tool_is_never_gated_even_while_stale() {
        let blocked = should_block_stale_tool(
            "ocentra_enforcer_coordination_status",
            &GateArgs::default(),
            Freshness::stale(),
        );
        assert!(!blocked, "read-only tools are never gated");
        let blocked_status = should_block_stale_tool(
            "ocentra_enforcer_mcp_status",
            &GateArgs::default(),
            Freshness::stale(),
        );
        assert!(!blocked_status, "mcp_status is never gated");
    }

    #[test]
    fn repair_dry_run_is_allowed_on_a_stale_server() {
        let dry_run_default = should_block_stale_tool(
            "ocentra_enforcer_coordination_repair",
            &GateArgs::default(),
            Freshness::stale(),
        );
        assert!(!dry_run_default, "dryRun unset defaults to allowed (not a write)");

        let dry_run_true = should_block_stale_tool(
            "ocentra_enforcer_coordination_repair",
            &GateArgs {
                dry_run: Some(true),
                ..GateArgs::default()
            },
            Freshness::stale(),
        );
        assert!(!dry_run_true, "explicit dryRun:true stays allowed");
    }

    #[test]
    fn repair_real_write_is_refused_on_a_stale_server() {
        let write_true = should_block_stale_tool(
            "ocentra_enforcer_coordination_repair",
            &GateArgs {
                write: Some(true),
                ..GateArgs::default()
            },
            Freshness::stale(),
        );
        assert!(write_true, "write:true must be refused while stale");

        let dry_run_false = should_block_stale_tool(
            "ocentra_enforcer_coordination_repair",
            &GateArgs {
                dry_run: Some(false),
                ..GateArgs::default()
            },
            Freshness::stale(),
        );
        assert!(dry_run_false, "dryRun:false must be refused while stale");
    }

    #[test]
    fn mail_action_gate_honors_write_actions_by_tool() {
        let send_blocked = should_block_stale_tool(
            "ocentra_enforcer_coordination_mail",
            &GateArgs {
                action: Some("send".to_owned()),
                ..GateArgs::default()
            },
            Freshness::stale(),
        );
        assert!(send_blocked, "mail:send is a write action");

        let inbox_allowed = should_block_stale_tool(
            "ocentra_enforcer_coordination_mail",
            &GateArgs {
                action: Some("inbox".to_owned()),
                ..GateArgs::default()
            },
            Freshness::stale(),
        );
        assert!(!inbox_allowed, "mail:inbox is not a write action");
    }

    #[test]
    fn hash_incompatible_reason_string_differs_from_plain_stale() {
        let stale = stale_fallback("ocentra_enforcer_coordination_claim", Freshness::stale(), "/x");
        let incompatible = stale_fallback(
            "ocentra_enforcer_coordination_claim",
            Freshness::hash_incompatible(),
            "/x",
        );
        assert!(stale.error.contains("MCP server is stale"));
        assert!(incompatible.error.contains("hash compatibility failed"));
    }

    #[test]
    fn coordination_write_tools_set_matches_legacy_membership() {
        let set = coordination_write_tools();
        assert!(set.contains("ocentra_enforcer_coordination_init"));
        assert!(set.contains("ocentra_enforcer_coordination_compact"));
        assert!(!set.contains("ocentra_enforcer_coordination_status"));
        assert!(!set.contains("ocentra_enforcer_coordination_repair"));
    }
}
