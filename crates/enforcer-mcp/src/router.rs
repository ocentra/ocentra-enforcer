//! The tool router: dispatches a `tools/call` request to the right engine
//! crate. No business logic lives here (per the crate charter) — every
//! handler is a thin adapter: decode typed args, call the sibling crate's
//! real function, encode the typed result as camelCase JSON.
//!
//! Three cross-cutting concerns apply to every dispatch, in order:
//! 1. [`crate::aliases::normalize_tool_name`] folds a `rust_rules_*` call
//!    to its canonical name before lookup (only while
//!    [`crate::aliases::deprecation_window_open`]).
//! 2. [`crate::gate::should_block_stale_tool`] refuses coordination WRITE
//!    tools on a stale/hash-incompatible server (see [`crate::gate`]).
//! 3. The matched handler runs and returns a `serde_json::Value` result.

use enforcer_coordination::api::{self, CallerContext, ClaimRequestArgs, Hub};
use enforcer_domain::ids::{HubName, LaneId};

use crate::gate::{self, Freshness, GateArgs};

/// The outcome of routing one `tools/call`.
#[derive(Debug, Clone)]
pub enum DispatchOutcome {
    /// The handler ran and produced a JSON result.
    Result(serde_json::Value),
    /// The tool name (post-alias-normalization) has no registered handler.
    UnknownTool,
    /// The stale-server write-gate refused this call.
    StaleRefused(Box<gate::StaleFallback>),
}

/// Everything a dispatch needs beyond the tool name/args: the freshness
/// verdict (see [`crate::gate`]'s a02 seam note) and the on-disk CLI path
/// used to build a refusal's fallback command.
#[derive(Debug, Clone)]
pub struct DispatchContext {
    pub freshness: Freshness,
    pub cli_path: String,
}

/// Route one `tools/call`. `name` is taken as received on the wire (may be
/// a legacy alias); `args` is the raw JSON `arguments` object (or `null`).
pub fn dispatch(name: &str, args: &serde_json::Value, ctx: &DispatchContext) -> DispatchOutcome {
    let canonical = if crate::aliases::deprecation_window_open() {
        crate::aliases::normalize_tool_name(name)
    } else if name.starts_with(crate::name::LEGACY_ALIAS_PREFIX) {
        // Deprecation window closed: an alias call is Unknown, matching
        // the workpack's fail fixture intent exactly.
        return DispatchOutcome::UnknownTool;
    } else {
        name.to_owned()
    };

    if !crate::registry::CANONICAL_TOOLS.contains(&canonical.as_str()) {
        return DispatchOutcome::UnknownTool;
    }

    let gate_args = gate_args_from(args);
    if gate::should_block_stale_tool(&canonical, &gate_args, ctx.freshness) {
        return DispatchOutcome::StaleRefused(Box::new(gate::stale_fallback(
            &canonical,
            ctx.freshness,
            &ctx.cli_path,
        )));
    }

    match canonical.as_str() {
        "ocentra_enforcer_mcp_status" => DispatchOutcome::Result(mcp_status(ctx)),
        "ocentra_enforcer_coordination_status" => {
            DispatchOutcome::Result(coordination_status(args))
        }
        "ocentra_enforcer_coordination_claim" => DispatchOutcome::Result(coordination_claim(args)),
        // Every other registered tool is a real delegate seam owned by a
        // sibling pack's future wiring pass; this skeleton reports it as
        // registered-but-not-yet-wired rather than silently no-op'ing or
        // fabricating a result, honoring the "no business logic here"
        // charter while staying observable.
        other if crate::registry::CANONICAL_TOOLS.contains(&other) => {
            DispatchOutcome::Result(serde_json::json!({
                "ok": false,
                "error": format!("{other} is registered but not yet wired to its engine delegate"),
                "operation": other,
            }))
        }
        _ => DispatchOutcome::UnknownTool,
    }
}

fn gate_args_from(args: &serde_json::Value) -> GateArgs {
    GateArgs {
        write: args.get("write").and_then(serde_json::Value::as_bool),
        dry_run: args.get("dryRun").and_then(serde_json::Value::as_bool),
        action: args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

/// `ocentra_enforcer_mcp_status` — never write-gated (read-only server
/// self-report). Reports the tool-surface size (the d05 measure seam) plus
/// the alias-window state.
fn mcp_status(ctx: &DispatchContext) -> serde_json::Value {
    let descriptors = crate::registry::build_tool_descriptors();
    serde_json::json!({
        "ok": true,
        "serverName": crate::name::SERVER_NAME,
        "directWritesAllowed": ctx.freshness.direct_writes_allowed,
        "hashCompatible": ctx.freshness.hash_compatible,
        "aliasWindowOpen": crate::aliases::deprecation_window_open(),
        "toolCount": descriptors.len(),
        "toolSurfaceBytes": crate::registry::tool_surface_bytes(&descriptors),
    })
}

/// `ocentra_enforcer_coordination_status` — read-only; delegates to
/// `enforcer-coordination`'s ledger projection over whatever hub root the
/// caller names. Deliberately minimal args (`root`, `hub`, `lane`) — the
/// full argument surface is a sibling wiring pass.
fn coordination_status(args: &serde_json::Value) -> serde_json::Value {
    let root = match args.get("root").and_then(serde_json::Value::as_str) {
        Some(root) => std::path::PathBuf::from(root),
        None => return json_error("coordination_status requires a `root` path"),
    };
    match enforcer_coordination::sync::stream::read_all_streams(&root) {
        Ok(all) => {
            let active = enforcer_coordination::ledger::active_claims(&all.events);
            serde_json::json!({
                "ok": true,
                "activeClaimCount": active.len(),
                "eventCount": all.events.len(),
            })
        }
        Err(err) => json_error(&err.to_string()),
    }
}

/// `ocentra_enforcer_coordination_claim` — the write tool this workpack's
/// L1/L2/L13 requirements exist for. Delegates entirely to
/// `enforcer_coordination::api::{init, claim_all}`; this handler's only job
/// is JSON<->typed decoding, never re-implementing claim semantics.
fn coordination_claim(args: &serde_json::Value) -> serde_json::Value {
    let Some(root) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_claim requires a `root` path");
    };
    let Some(hub_raw) = args.get("hub").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_claim requires a `hub` name");
    };
    let Some(lane_raw) = args.get("lane").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_claim requires a `lane` id");
    };
    let Some(paths) = args.get("paths").and_then(serde_json::Value::as_array) else {
        return json_error("coordination_claim requires a `paths` array");
    };
    let owns: Vec<String> = paths
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    let (Ok(hub_name), Ok(lane_id)) = (hub_raw.parse::<HubName>(), lane_raw.parse::<LaneId>())
    else {
        return json_error("hub/lane failed enforcer-domain brand validation");
    };

    // L2: caller identity is a REQUIRED param the wire caller must supply
    // — this MCP layer never resolves it server-side (see
    // `enforcer_coordination::api::CallerContext` doc). Minimal fields
    // wired here; richer caller context is a sibling wiring pass.
    let worktree_root = args
        .get("worktreeRoot")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(root)
        .to_owned();
    let branch = args
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let caller = CallerContext {
        project_id: args
            .get("projectId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        worktree_root,
        branch,
        commit: args
            .get("commit")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        codex_thread_id: None,
        codex_session_id: None,
    };
    let reason = args.get("reason").and_then(serde_json::Value::as_str);

    let root_path = std::path::Path::new(root);
    let hub_config = match api::init(root_path, &hub_name, &lane_id) {
        Ok(config) => config,
        Err(err) => return json_error(&err.to_string()),
    };
    let hub = Hub {
        root: root_path.to_owned(),
        config: hub_config,
    };
    let outcome = api::claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: root_path,
            lane: &lane_id,
            owns: &owns,
            caller: &caller,
            reason,
        },
    );
    match outcome {
        Ok(result) => serde_json::json!({
            "ok": result.ok,
            "eventCount": result.events.len(),
            "blockerCount": result.blockers.len(),
        }),
        Err(err) => json_error(&err.to_string()),
    }
}

fn json_error(message: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": message })
}

#[cfg(test)]
mod tests {
    use super::{dispatch, DispatchContext, DispatchOutcome};
    use crate::gate::Freshness;

    fn ctx(freshness: Freshness) -> DispatchContext {
        DispatchContext {
            freshness,
            cli_path: "/abs/enforcer".to_owned(),
        }
    }

    #[test]
    fn pass_fixture_canned_request_yields_expected_tool_result() {
        let outcome = dispatch(
            "ocentra_enforcer_mcp_status",
            &serde_json::json!({}),
            &ctx(Freshness::fresh()),
        );
        let DispatchOutcome::Result(value) = outcome else {
            unreachable!("mcp_status must always produce a Result outcome");
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert!(value["toolCount"].as_u64().unwrap_or(0) > 0);
    }

    #[test]
    fn fail_fixture_malformed_request_unknown_tool_is_rejected() {
        let outcome = dispatch(
            "not_a_real_tool",
            &serde_json::json!({}),
            &ctx(Freshness::fresh()),
        );
        assert!(matches!(outcome, DispatchOutcome::UnknownTool));
    }

    #[test]
    fn legacy_alias_resolves_to_the_same_handler_as_canonical() {
        let canonical = dispatch(
            "ocentra_enforcer_mcp_status",
            &serde_json::json!({}),
            &ctx(Freshness::fresh()),
        );
        let aliased = dispatch(
            "rust_rules_mcp_status",
            &serde_json::json!({}),
            &ctx(Freshness::fresh()),
        );
        let (DispatchOutcome::Result(a), DispatchOutcome::Result(b)) = (canonical, aliased) else {
            unreachable!("both calls must produce a Result outcome");
        };
        assert_eq!(a, b);
    }

    #[test]
    fn stale_server_refuses_a_write_tool_via_the_router() {
        let outcome = dispatch(
            "ocentra_enforcer_coordination_claim",
            &serde_json::json!({}),
            &ctx(Freshness::stale()),
        );
        assert!(matches!(outcome, DispatchOutcome::StaleRefused(_)));
    }

    #[test]
    fn read_only_tool_still_dispatches_while_stale() {
        let outcome = dispatch(
            "ocentra_enforcer_mcp_status",
            &serde_json::json!({}),
            &ctx(Freshness::stale()),
        );
        assert!(matches!(outcome, DispatchOutcome::Result(_)));
    }

    #[test]
    fn coordination_claim_end_to_end_against_a_real_temp_hub(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let file_a = root.join("crate_a.rs");
        std::fs::write(&file_a, "fn main() {}\n")?;

        let args = serde_json::json!({
            "root": root.to_string_lossy(),
            "hub": "enforcer-rust-build",
            "lane": "arc-99",
            "paths": ["crate_a.rs"],
            "worktreeRoot": root.to_string_lossy(),
            "branch": "lane/arc-99",
            "projectId": "test-project",
            "reason": "router end-to-end fixture",
        });
        let outcome = dispatch(
            "ocentra_enforcer_coordination_claim",
            &args,
            &ctx(Freshness::fresh()),
        );
        let DispatchOutcome::Result(value) = outcome else {
            unreachable!("expected a successful claim Result outcome");
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["eventCount"], serde_json::json!(1));
        Ok(())
    }
}
