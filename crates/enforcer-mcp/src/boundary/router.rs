//! Boundary tool router: dispatches a `tools/call` request to the right engine
//! crate. Each handler is a thin adapter: decode typed args, call the sibling crate's
//! real function, encode the typed result as camelCase JSON.
//!
//! Three cross-cutting concerns apply to every dispatch, in order:
//! 1. [`crate::aliases::normalize_tool_name`] folds a `rust_rules_*` call
//!    to its canonical name before lookup (only while
//!    [`crate::aliases::deprecation_window_open`]).
//! 2. [`crate::gate::should_block_stale_tool`] refuses coordination WRITE
//!    tools on a stale/hash-incompatible server (see [`crate::gate`]).
//! 3. The matched handler runs and returns a `serde_json::Value` result.
//!
//! NEGATIVE-TEST: malformed and unknown tool calls are rejected by the
//! router tests before any backing engine is invoked.
//! BOUNDARY-INVARIANT: JSON-RPC request fields are decoded here and converted
//! before handlers call typed engine adapters.
//! boundaryOwnerNote: enforcer-mcp owns transport dispatch; durable behavior
//! remains in the typed engine crates.

use enforcer_coordination::api::{self, CallerContext, ClaimRequestArgs, Hub};
use enforcer_domain::{
    boundary::{
        mcp::{execution_mode, write_intent},
        validation::McpReportLabelText,
    },
    config_types::{ConfigProfileName, CrateName, HarnessArtifactByteLimit, HarnessRunLimit},
    coordination_types::{
        ClaimOutcomeStatus, ClaimPath, ClaimReason, ClaimWriter, CoordinationBranch,
        CoordinationLedgerRoot, CoordinationProjectId, CoordinationRepoRoot, CoordinationWorktree,
    },
    harness_types::{
        HarnessArtifactKind, HarnessDomainName, HarnessPackageName, HarnessRunId, HarnessRunStatus,
        HarnessTag, HarnessToolName,
    },
    ids::{HubName, LaneId},
    mcp_types::{ArtifactPath, McpActionName, McpFreshness, McpToolName},
    paths::{RelPath, RepoRoot},
    scan_types::{CommitRef, RouteScope},
    severity::Severity,
};

use crate::gate::{self, GateArgs};
use crate::validation_history::{
    CompactScope, FindingCount, ReportLabel, SeverityCount, ValidationCounts, ValidationHistory,
    ValidationKind, ValidationOutcome, ValidationSummary, ValidationTimestamp,
};
use std::sync::{Arc, Mutex};

/// The outcome of routing one `tools/call`.
#[derive(Debug, Clone)]
pub enum DispatchOutcome {
    /// The handler ran and produced a JSON result.
    Result(serde_json::Value),
    /// The tool name (post-alias-normalization) has no registered handler.
    UnknownTool,
    /// The stale-server write-gate refused this call.
    StaleRefused(Box<gate::StaleFallbackDto>),
}

/// Everything a dispatch needs beyond the tool name/args: the freshness
/// verdict (see [`crate::gate`]'s a02 seam note) and the on-disk CLI path
/// used to build a refusal's fallback command.
#[derive(Debug, Clone)]
pub struct DispatchContext {
    pub freshness: McpFreshness,
    pub cli_path: ArtifactPath,
    pub validation_history: Arc<Mutex<ValidationHistory>>,
}

/// Route one `tools/call`. `name` is taken as received on the wire (may be
/// a legacy alias); `args` is the raw JSON `arguments` object (or `null`).
pub fn dispatch(
    name: &McpToolName,
    args: &serde_json::Value,
    ctx: &DispatchContext,
) -> DispatchOutcome {
    let canonical = if crate::aliases::deprecation_window_open() {
        match crate::aliases::normalize_tool_name(name) {
            Ok(value) => value,
            Err(_) => return DispatchOutcome::UnknownTool,
        }
    } else if name
        .as_str()
        .starts_with(crate::aliases::LEGACY_ALIAS_PREFIX)
    {
        // Deprecation window closed: an alias call is Unknown, matching
        // the workpack's fail fixture intent exactly.
        return DispatchOutcome::UnknownTool;
    } else {
        // Routing owns the canonical name because the gate and handler lookup
        // share it; re-parse the already validated wire value rather than
        // cloning an arbitrary caller-owned domain object.
        match McpToolName::try_new(name.as_str()) {
            Ok(value) => value,
            Err(_) => return DispatchOutcome::UnknownTool,
        }
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
        "ocentra_enforcer_scan" => DispatchOutcome::Result(scan(args, ctx)),
        "ocentra_enforcer_check" => DispatchOutcome::Result(check(args, ctx)),
        "ocentra_enforcer_explain" => DispatchOutcome::Result(explain(args)),
        "ocentra_enforcer_run" => DispatchOutcome::Result(run_harness(args)),
        "ocentra_enforcer_run_status" => DispatchOutcome::Result(run_status(args, ctx)),
        "ocentra_enforcer_doctor" => DispatchOutcome::Result(doctor(args)),
        "ocentra_enforcer_diagnostics" => DispatchOutcome::Result(diagnostics(args)),
        "ocentra_enforcer_last_failure" => DispatchOutcome::Result(last_failure(args)),
        "ocentra_enforcer_artifact" => DispatchOutcome::Result(artifact(args)),
        "ocentra_enforcer_prune_runs" => DispatchOutcome::Result(prune_runs(args)),
        "ocentra_enforcer_reset_runs" => DispatchOutcome::Result(reset_runs(args)),
        "ocentra_enforcer_route" => DispatchOutcome::Result(route(args)),
        "ocentra_enforcer_test_doctrine_scan" => DispatchOutcome::Result(test_doctrine_scan(args)),
        "ocentra_enforcer_ui_logic_coupling_scan" => {
            DispatchOutcome::Result(ui_logic_coupling_scan(args))
        }
        "ocentra_enforcer_mcp_status" => DispatchOutcome::Result(mcp_status(ctx)),
        "ocentra_enforcer_coordination_status"
        | "ocentra_enforcer_coordination_health"
        | "ocentra_enforcer_coordination_presence"
        | "ocentra_enforcer_coordination_streams"
        | "ocentra_enforcer_coordination_inbox"
        | "ocentra_enforcer_coordination_workers"
        | "ocentra_enforcer_coordination_tasks"
        | "ocentra_enforcer_coordination_guard"
        | "ocentra_enforcer_coordination_init"
        | "ocentra_enforcer_coordination_claim"
        | "ocentra_enforcer_coordination_release"
        | "ocentra_enforcer_coordination_closeout"
        | "ocentra_enforcer_coordination_message"
        | "ocentra_enforcer_coordination_mail"
        | "ocentra_enforcer_coordination_index"
        | "ocentra_enforcer_coordination_sync"
        | "ocentra_enforcer_coordination_peer"
        | "ocentra_enforcer_coordination_ensure"
        | "ocentra_enforcer_coordination_compact"
        | "ocentra_enforcer_coordination_repair"
        | "ocentra_enforcer_coordination_notify"
        | "ocentra_enforcer_coordination_report" => {
            DispatchOutcome::Result(coordination(canonical.as_str(), args))
        }
        "ocentra_enforcer_ui" => DispatchOutcome::Result(ui_tool(args)),
        "ocentra_enforcer_proof_route" => DispatchOutcome::Result(proof_route(args)),
        "ocentra_enforcer_proof_status" => DispatchOutcome::Result(proof_status(args)),
        "ocentra_enforcer_proof_inventory" => DispatchOutcome::Result(proof_inventory(args)),
        "ocentra_enforcer_proof_run"
        | "ocentra_enforcer_proof_artifact"
        | "ocentra_enforcer_proof_claim"
        | "ocentra_enforcer_proof_export"
        | "ocentra_enforcer_proof_import_legacy"
        | "ocentra_enforcer_proof_last_failure"
        | "ocentra_enforcer_proof_parity"
        | "ocentra_enforcer_proof_prune"
        | "ocentra_enforcer_proof_reset"
        | "ocentra_enforcer_proof_diagnostics" => {
            DispatchOutcome::Result(proof_lifecycle(canonical.as_str(), args))
        }
        // Every other registered tool is a real delegate seam owned by a
        // sibling pack's future wiring pass; this skeleton reports it as
        // registered-but-not-yet-wired rather than silently no-op'ing or
        // fabricating a result, while staying observable.
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

/// Native `ocentra_enforcer_explain`: decode the public id at this boundary,
/// then resolve it from the Rust-compiled rule catalog.  A missing/malformed
/// id is an explicit rejection, never a best-effort MJS subprocess fallback.
fn explain(args: &serde_json::Value) -> serde_json::Value {
    let Some(raw_rule_id) = args.get("ruleId").and_then(serde_json::Value::as_str) else {
        return serde_json::json!({
            "ok": false,
            "error": "ruleId is required and must be a string",
            "kind": "invalid_input",
        });
    };
    let rule_id = match raw_rule_id.parse::<enforcer_domain::ids::RuleId>() {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({
                "ok": false,
                "ruleId": raw_rule_id,
                "error": error.to_string(),
                "kind": "invalid_rule_id",
            });
        }
    };
    match crate::registry::explain_rule(&rule_id) {
        Ok(Some(rule)) => serde_json::json!({
            "ok": true,
            "ruleId": rule.rule_id.to_string(),
            "language": rule.language,
            "family": rule.family,
            "severity": rule.severity,
            "title": rule.title,
            "fixHint": rule.fix_hint,
            "doc": rule.doc_anchor,
            "source": "native-rust-rule-catalog",
        }),
        Ok(None) => serde_json::json!({
            "ok": false,
            "ruleId": raw_rule_id,
            "error": "rule id is not present in the native Rust rule catalog",
            "kind": "rule_not_found",
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "ruleId": raw_rule_id,
            "error": error,
            "kind": "native_catalog_error",
        }),
    }
}

/// Thin MCP projection over `enforcer-proof`'s durable lifecycle.  Read-only
/// proof tools share the typed snapshot; mutation tools are explicit and do
/// not fabricate legacy results when their native input is absent.
fn proof_lifecycle(operation: &str, args: &serde_json::Value) -> serde_json::Value {
    let root = args
        .get("root")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(".");
    let lifecycle = match enforcer_proof::boundary::lifecycle::NativeProofLifecycle::open(
        std::path::Path::new(root),
    ) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()});
        }
    };
    match operation {
        "ocentra_enforcer_proof_run" => {
            let Some(proof) = args.get("proofId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"proofId is required"});
            };
            let Some(run) = args.get("runId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"runId is required"});
            };
            let command = args
                .get("command")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let proof_id = match proof.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid proofId"});
                }
            };
            let run_id = match run.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid runId"});
                }
            };
            let canonical_root = std::path::Path::new(root)
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(root));
            let request = enforcer_proof::harness::RunProofArgs {
                proof_id,
                root: canonical_root,
                run_id,
                command,
                capability: None,
                claims_proved: Vec::new(),
                claims_not_proved: Vec::new(),
                pin: false,
            };
            match lifecycle.run(&request, None) {
                Ok(outcome) => {
                    serde_json::json!({"ok":outcome.ok,"operation":operation,"run":outcome.proof_run,"diagnostics":outcome.diagnostics})
                }
                Err(error) => {
                    serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
                }
            }
        }
        "ocentra_enforcer_proof_reset" => match lifecycle.reset() {
            Ok(()) => serde_json::json!({"ok":true,"operation":operation}),
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
        "ocentra_enforcer_proof_prune" => {
            let Some(value) = args.get("runId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"runId is required"});
            };
            match value
                .parse()
                .map_err(enforcer_core::error::Error::Decode)
                .and_then(|run_id| lifecycle.prune_run(&run_id))
            {
                Ok(pruned) => serde_json::json!({"ok":true,"operation":operation,"pruned":pruned}),
                Err(error) => {
                    serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
                }
            }
        }
        "ocentra_enforcer_proof_artifact" => {
            let Some(run) = args.get("runId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"runId is required"});
            };
            let Some(artifact) = args.get("artifact").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"artifact is required"});
            };
            let run_id = match run.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid runId"});
                }
            };
            let path = match enforcer_domain::paths::RelPath::try_from(artifact.to_owned()) {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid artifact path"});
                }
            };
            match lifecycle.read_declared_artifact(&run_id, &path) {
                Ok(bytes) => {
                    serde_json::json!({"ok":true,"operation":operation,"artifact":String::from_utf8_lossy(&bytes)})
                }
                Err(error) => {
                    serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
                }
            }
        }
        "ocentra_enforcer_proof_export" => match lifecycle.export() {
            Ok(bytes) => {
                serde_json::json!({"ok":true,"operation":operation,"export":String::from_utf8_lossy(&bytes)})
            }
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
        "ocentra_enforcer_proof_claim" => match lifecycle.claim() {
            Ok(claim) => serde_json::json!({"ok":true,"operation":operation,"claim":claim}),
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
        "ocentra_enforcer_proof_diagnostics" => match lifecycle.diagnostics() {
            Ok(diagnostics) => {
                serde_json::json!({"ok":true,"operation":operation,"diagnostics":diagnostics})
            }
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
        "ocentra_enforcer_proof_last_failure" => match lifecycle.last_failure() {
            Ok(run) => serde_json::json!({"ok":true,"operation":operation,"run":run}),
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
        "ocentra_enforcer_proof_import_legacy" => {
            let Some(proof) = args.get("proofId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"proofId is required"});
            };
            let Some(run) = args.get("runId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"runId is required"});
            };
            let roots: Vec<&str> = args
                .get("legacyPaths")
                .and_then(serde_json::Value::as_array)
                .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();
            let proof_id = match proof.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid proofId"});
                }
            };
            let run_id = match run.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid runId"});
                }
            };
            match lifecycle.import_legacy(&proof_id, &run_id, &roots) {
                Ok(run) => serde_json::json!({"ok":true,"operation":operation,"run":run}),
                Err(error) => {
                    serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
                }
            }
        }
        "ocentra_enforcer_proof_parity" => {
            let Some(run) = args.get("runId").and_then(serde_json::Value::as_str) else {
                return serde_json::json!({"ok":false,"operation":operation,"error":"runId is required"});
            };
            let roots: Vec<&str> = args
                .get("legacyPaths")
                .and_then(serde_json::Value::as_array)
                .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();
            let run_id = match run.parse() {
                Ok(value) => value,
                Err(_) => {
                    return serde_json::json!({"ok":false,"operation":operation,"error":"invalid runId"});
                }
            };
            match lifecycle.parity(&run_id, &roots) {
                Ok((coverage, deletion_ready)) => {
                    serde_json::json!({"ok":true,"operation":operation,"coverage":coverage,"deletionReady":deletion_ready})
                }
                Err(error) => {
                    serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
                }
            }
        }
        _ => match lifecycle.snapshot() {
            Ok(snapshot) => {
                serde_json::json!({"ok":true,"operation":operation,"snapshot":snapshot})
            }
            Err(error) => {
                serde_json::json!({"ok":false,"operation":operation,"error":error.to_string()})
            }
        },
    }
}

/// Thin proof-route transport adapter: JSON is decoded here; catalog loading,
/// merging, and route selection remain in `enforcer-proof`.
fn proof_route(args: &serde_json::Value) -> serde_json::Value {
    let lifecycle = match proof_lifecycle_for_root(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_route", error),
    };
    let query = match decode_proof_route_query(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_route", error),
    };
    match lifecycle.route(&query) {
        Ok(value) => encode_proof_route(value),
        Err(error) => proof_query_error("ocentra_enforcer_proof_route", error.to_string()),
    }
}

/// Thin proof-status adapter. The lifecycle reads only persisted run envelopes
/// and returns a filtered bounded collection rather than a generic snapshot.
fn proof_status(args: &serde_json::Value) -> serde_json::Value {
    let lifecycle = match proof_lifecycle_for_root(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_status", error),
    };
    let query = match decode_proof_status_query(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_status", error),
    };
    match lifecycle.status(&query) {
        Ok(value) => encode_proof_status(value),
        Err(error) => proof_query_error("ocentra_enforcer_proof_status", error.to_string()),
    }
}

/// Thin proof-inventory adapter. The lifecycle owns safe script discovery and
/// compact aggregate formation.
fn proof_inventory(args: &serde_json::Value) -> serde_json::Value {
    let lifecycle = match proof_lifecycle_for_root(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_inventory", error),
    };
    let query = match decode_proof_inventory_query(args) {
        Ok(value) => value,
        Err(error) => return proof_query_error("ocentra_enforcer_proof_inventory", error),
    };
    match lifecycle.inventory(&query) {
        Ok(value) => encode_proof_inventory(value),
        Err(error) => proof_query_error("ocentra_enforcer_proof_inventory", error.to_string()),
    }
}

/// Render a typed proof-route application result at the MCP JSON boundary.
fn encode_proof_route(
    value: enforcer_proof::boundary::lifecycle::ProofRouteResult,
) -> serde_json::Value {
    let scope = proof_route_scope_json(&value.query);
    let proofs = value.proofs.into_iter().map(|proof| {
        serde_json::json!({
            "id": proof.id,
            "title": proof.title,
            "family": proof.family,
            "severity": proof.severity,
            "collector": proof.collector,
            "capabilities": proof.capabilities,
            "docs": proof.docs,
        })
    });
    serde_json::json!({
        "ok": true,
        "productName": value.product_name,
        "profileName": value.profile_name,
        "index": value.index,
        "scope": scope,
        "docs": value.docs,
        "proofs": proofs.collect::<Vec<_>>(),
    })
}

/// Render a typed persisted-run query result at the MCP JSON boundary.
fn encode_proof_status(
    value: enforcer_proof::boundary::lifecycle::ProofStatusResult,
) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "root": value.root.to_string_lossy(),
        "runs": value.runs,
    })
}

/// Render a typed inventory application result at the MCP JSON boundary.
fn encode_proof_inventory(
    value: enforcer_proof::boundary::lifecycle::ProofInventoryResult,
) -> serde_json::Value {
    let scripts = value.scripts.into_iter().map(|script| {
        serde_json::json!({
            "path": script.path,
            "name": script.name,
            "family": script.family,
            "planBucket": script.plan_bucket,
            "proofTypes": script.proof_types,
            "capabilities": script.capabilities,
            "signals": {
                "spawn": script.signals.spawn,
                "writesProof": script.signals.writes_proof,
                "readsProof": script.signals.reads_proof,
                "manualOrDevice": script.signals.manual_or_device,
                "importsBuiltOrSchemaParse": script.signals.imports_built_or_schema_parse,
            },
        })
    });
    serde_json::json!({
        "ok": true,
        "root": value.root,
        "scriptsRoot": value.scripts_root,
        "totals": {
            "scripts": value.totals.scripts,
            "proofNamed": value.totals.proof_named,
            "spawnCommands": value.totals.spawn_commands,
            "writesProof": value.totals.writes_proof,
            "readsProof": value.totals.reads_proof,
            "manualOrDevice": value.totals.manual_or_device,
            "importsBuiltOrSchemaParse": value.totals.imports_built_or_schema_parse,
        },
        "byFamily": value.by_family,
        "byProofType": value.by_proof_type,
        "byCapability": value.by_capability,
        "scriptRowsIncluded": value.script_rows_included,
        "scriptLimit": value.script_limit,
        "omittedScriptCount": value.omitted_script_count,
        "scripts": scripts.collect::<Vec<_>>(),
    })
}

/// Project the typed route query into the frozen MCP scope envelope.
fn proof_route_scope_json(
    query: &enforcer_proof::boundary::proof_query::ProofRouteQuery,
) -> serde_json::Value {
    if let Some(proof_id) = &query.proof_id {
        return serde_json::json!({"mode":"proof","proofId":proof_id.as_str()});
    }
    if !query.files.is_empty() {
        return serde_json::json!({
            "mode":"files",
            "files":query.files.iter().map(enforcer_domain::paths::RelPath::as_str).collect::<Vec<_>>()
        });
    }
    if let Some(plan) = &query.plan {
        return serde_json::json!({"mode":"plan","plan":plan});
    }
    if let Some(capability) = &query.capability {
        return serde_json::json!({"mode":"capability","capability":capability.as_str()});
    }
    serde_json::json!({"mode":query.scope.as_deref().unwrap_or("workspace")})
}

fn proof_lifecycle_for_root(
    args: &serde_json::Value,
) -> std::result::Result<enforcer_proof::boundary::lifecycle::NativeProofLifecycle, String> {
    let root = match args.get("root") {
        None => std::path::PathBuf::from("."),
        Some(serde_json::Value::String(value)) => std::path::PathBuf::from(value),
        Some(_) => return Err("root must be a string".to_owned()),
    };
    enforcer_proof::boundary::lifecycle::NativeProofLifecycle::open(&root)
        .map_err(|error| error.to_string())
}

fn decode_proof_route_query(
    args: &serde_json::Value,
) -> std::result::Result<enforcer_proof::boundary::proof_query::ProofRouteQuery, String> {
    let proof_id = optional_proof_id(args, "proofId")?;
    let files = optional_rel_paths(args, "files")?;
    let plan = optional_string(args, "plan")?;
    let capability = optional_string(args, "capability")?
        .map(|value| {
            value.parse().map_err(
                |error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string(),
            )
        })
        .transpose()?;
    let scope = optional_string(args, "scope")?;
    if scope
        .as_deref()
        .is_some_and(|value| !matches!(value, "workspace" | "files" | "crate" | "diff"))
    {
        return Err("scope must be one of workspace, files, crate, or diff".to_owned());
    }
    let profile = optional_string(args, "profile")?
        .map(|value| {
            enforcer_domain::config_types::ConfigProfileName::try_new(value)
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    Ok(enforcer_proof::boundary::proof_query::ProofRouteQuery {
        proof_id,
        files,
        plan,
        capability,
        scope,
        profile,
    })
}

fn decode_proof_status_query(
    args: &serde_json::Value,
) -> std::result::Result<enforcer_proof::boundary::proof_query::ProofStatusQuery, String> {
    let status = optional_string(args, "status")?
        .map(|value| parse_proof_status(&value))
        .transpose()?;
    Ok(enforcer_proof::boundary::proof_query::ProofStatusQuery {
        proof_id: optional_proof_id(args, "proofId")?,
        status,
        limit: optional_limit(args)?,
    })
}

fn decode_proof_inventory_query(
    args: &serde_json::Value,
) -> std::result::Result<enforcer_proof::boundary::proof_query::ProofInventoryQuery, String> {
    let include_scripts = match args.get("includeScripts") {
        None => false,
        Some(serde_json::Value::Bool(value)) => *value,
        Some(_) => return Err("includeScripts must be a boolean".to_owned()),
    };
    Ok(enforcer_proof::boundary::proof_query::ProofInventoryQuery {
        include_scripts,
        limit: optional_limit(args)?,
    })
}

fn optional_proof_id(
    args: &serde_json::Value,
    field: &str,
) -> std::result::Result<Option<enforcer_domain::proof_types::ProofId>, String> {
    optional_string(args, field)?
        .map(|value| {
            value.parse().map_err(
                |error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string(),
            )
        })
        .transpose()
}

fn optional_rel_paths(
    args: &serde_json::Value,
    field: &str,
) -> std::result::Result<Vec<RelPath>, String> {
    match args.get(field) {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .map(|value| {
                let Some(value) = value.as_str() else {
                    return Err(format!("{field} must contain only strings"));
                };
                RelPath::try_from(value.to_owned()).map_err(|error| error.to_string())
            })
            .collect(),
        Some(_) => Err(format!("{field} must be an array")),
    }
}

fn optional_string(
    args: &serde_json::Value,
    field: &str,
) -> std::result::Result<Option<String>, String> {
    match args.get(field) {
        None => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("{field} must be a string")),
    }
}

fn optional_limit(args: &serde_json::Value) -> std::result::Result<usize, String> {
    match args.get("limit") {
        None => Ok(enforcer_proof::boundary::lifecycle::DEFAULT_PROOF_QUERY_LIMIT),
        Some(value) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| "limit must be a non-negative integer".to_owned()),
    }
}

fn parse_proof_status(
    value: &str,
) -> std::result::Result<enforcer_domain::proof_types::ProofStatus, String> {
    match value {
        "passed" => Ok(enforcer_domain::proof_types::ProofStatus::Passed),
        "failed" => Ok(enforcer_domain::proof_types::ProofStatus::Failed),
        "manual-required" => Ok(enforcer_domain::proof_types::ProofStatus::ManualRequired),
        "unavailable" => Ok(enforcer_domain::proof_types::ProofStatus::Unavailable),
        "waived" => {
            Err("status `waived` has no native persisted ProofStatus representation".to_owned())
        }
        _ => {
            Err("status must be passed, failed, manual-required, unavailable, or waived".to_owned())
        }
    }
}

fn proof_query_error(operation: &str, error: String) -> serde_json::Value {
    let error = serde_json::Value::String(error);
    serde_json::json!({"ok":false,"operation":operation,"error":error})
}

fn gate_args_from(args: &serde_json::Value) -> GateArgs {
    GateArgs {
        write: write_intent(args.get("write").and_then(serde_json::Value::as_bool)),
        dry_run: execution_mode(args.get("dryRun").and_then(serde_json::Value::as_bool)),
        action: args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| McpActionName::try_new(value).ok()),
    }
}

/// Native Rust scan adapter. This deliberately accepts only the scope modes
/// the Rust engine executes correctly today. Unsupported legacy options are
/// rejected at the MCP boundary rather than silently ignored or delegated
/// back to the frozen MJS implementation.
fn scan_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    const SUPPORTED_FIELDS: &[&str] = &[
        "root",
        "scope",
        "files",
        "crateName",
        "base",
        "head",
        "languages",
    ];
    let Some(object) = args.as_object() else {
        return json_error("scan arguments must be an object");
    };
    if let Some(unsupported) = object
        .keys()
        .find(|field| !SUPPORTED_FIELDS.contains(&field.as_str()))
    {
        return json_error(&format!("scan does not support `{unsupported}`"));
    }
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let result = enforcer_scan::boundary::native_scan::execute(&request, &root)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()));
    match result {
        Ok(value) => value,
        Err(err) => json_error(&err),
    }
}

fn native_scan_request(
    args: &serde_json::Value,
) -> Result<
    (
        RepoRoot,
        enforcer_scan::boundary::native_scan::NativeScanRequest,
    ),
    String,
> {
    let Some(root_raw) = args.get("root").and_then(serde_json::Value::as_str) else {
        return Err("scan requires a `root` path".to_owned());
    };
    let root = root_raw
        .parse::<RepoRoot>()
        .map_err(|error| error.to_string())?;
    let files = match args.get("files") {
        None => None,
        Some(serde_json::Value::Array(values)) => match values
            .iter()
            .map(|value| value.as_str().map(std::path::PathBuf::from))
            .collect::<Option<Vec<_>>>()
        {
            Some(values) => Some(values),
            None => return Err("scan `files` must contain only paths".to_owned()),
        },
        Some(_) => return Err("scan `files` must be an array".to_owned()),
    };
    let languages = match parse_scan_languages(args.get("languages")) {
        Ok(value) => value,
        Err(message) => return Err(message),
    };
    let scope = match parse_scan_scope(args.get("scope"), files, args) {
        Ok(value) => value,
        Err(message) => return Err(message),
    };
    Ok((
        root,
        enforcer_scan::boundary::native_scan::NativeScanRequest { scope, languages },
    ))
}

fn scan(args: &serde_json::Value, ctx: &DispatchContext) -> serde_json::Value {
    let report = scan_unrecorded(args);
    if report.get("error").is_none() {
        if let Some(root) = args.get("root").and_then(serde_json::Value::as_str) {
            record_validation_at_root(ctx, root, ValidationKind::Scan, &report);
        }
    }
    report
}

fn parse_scan_languages(
    raw: Option<&serde_json::Value>,
) -> Result<Vec<enforcer_scan::boundary::native_scan::NativeScanLanguage>, String> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let Some(values) = raw.as_array() else {
        return Err("scan `languages` must be an array".to_owned());
    };
    values
        .iter()
        .map(|value| match value.as_str() {
            Some("rust") => Ok(enforcer_scan::boundary::native_scan::NativeScanLanguage::Rust),
            Some("typescript") => {
                Ok(enforcer_scan::boundary::native_scan::NativeScanLanguage::TypeScript)
            }
            Some("python") => Ok(enforcer_scan::boundary::native_scan::NativeScanLanguage::Python),
            Some("terraform") => {
                Ok(enforcer_scan::boundary::native_scan::NativeScanLanguage::Terraform)
            }
            Some("yaml-or-config") => {
                Ok(enforcer_scan::boundary::native_scan::NativeScanLanguage::YamlOrConfig)
            }
            Some(value) => Err(format!("scan does not support language `{value}`")),
            None => Err("scan `languages` must contain only strings".to_owned()),
        })
        .collect()
}

fn parse_scan_scope(
    scope: Option<&serde_json::Value>,
    files: Option<Vec<std::path::PathBuf>>,
    args: &serde_json::Value,
) -> Result<enforcer_scan::boundary::native_scan::NativeScanScope, String> {
    let requested = scope.and_then(serde_json::Value::as_str);
    if scope.is_some() && requested.is_none() {
        return Err("scan `scope` must be a string".to_owned());
    }
    let scope = requested.unwrap_or_else(|| {
        if files.is_some() {
            "files"
        } else if args.get("crateName").is_some() {
            "crate"
        } else if args.get("base").is_some() || args.get("head").is_some() {
            "diff"
        } else {
            "workspace"
        }
    });
    match scope {
        "files" => {
            if args.get("crateName").is_some()
                || args.get("base").is_some()
                || args.get("head").is_some()
            {
                return Err(
                    "scan `scope: files` cannot combine with crate or diff options".to_owned(),
                );
            }
            let Some(files) = files else {
                return Err("scan `scope: files` requires `files`".to_owned());
            };
            Ok(enforcer_scan::boundary::native_scan::NativeScanScope::Files(files))
        }
        "workspace" => {
            if files.is_some()
                || args.get("crateName").is_some()
                || args.get("base").is_some()
                || args.get("head").is_some()
            {
                return Err(
                    "scan `scope: workspace` cannot combine with narrowing options".to_owned(),
                );
            }
            Ok(enforcer_scan::boundary::native_scan::NativeScanScope::Workspace)
        }
        "crate" => {
            if files.is_some() || args.get("base").is_some() || args.get("head").is_some() {
                return Err(
                    "scan `scope: crate` cannot combine with files or diff options".to_owned(),
                );
            }
            let Some(name) = args.get("crateName").and_then(serde_json::Value::as_str) else {
                return Err("scan `scope: crate` requires `crateName`".to_owned());
            };
            let name = name
                .parse::<CrateName>()
                .map_err(|error| error.to_string())?;
            Ok(enforcer_scan::boundary::native_scan::NativeScanScope::Crate(name))
        }
        "diff" => {
            if files.is_some() || args.get("crateName").is_some() {
                return Err("scan `scope: diff` cannot combine with files or crateName".to_owned());
            }
            let Some(base) = args.get("base").and_then(serde_json::Value::as_str) else {
                return Err("scan `scope: diff` requires `base`".to_owned());
            };
            let Some(head) = args.get("head").and_then(serde_json::Value::as_str) else {
                return Err("scan `scope: diff` requires `head`".to_owned());
            };
            Ok(
                enforcer_scan::boundary::native_scan::NativeScanScope::Diff {
                    base: base
                        .parse::<CommitRef>()
                        .map_err(|error| error.to_string())?,
                    head: head
                        .parse::<CommitRef>()
                        .map_err(|error| error.to_string())?,
                },
            )
        }
        _ => Err("scan supports only `files`, `workspace`, `crate`, or `diff` scope".to_owned()),
    }
}

/// Native MCP check adapter. A named check is executed only when its exact
/// frozen-MJS rule mapping is backed by the native scan engine. Other frozen
/// names are deliberately rejected with a typed result: advertising a schema
/// entry is compatibility, not permission to fabricate a passing report.
fn check(args: &serde_json::Value, ctx: &DispatchContext) -> serde_json::Value {
    let Some(name) = args.get("check").and_then(serde_json::Value::as_str) else {
        return json_error("check requires a named `check`");
    };
    if !crate::registry::NAMED_CHECKS.contains(&name) {
        return named_check_rejection(name, "unknown_named_check");
    }
    if name == "generated-artifacts" {
        let report = named_generated_artifacts_unrecorded(args);
        if report.get("error").is_none() {
            if let Some(root) = args.get("root").and_then(serde_json::Value::as_str) {
                record_validation_at_root(ctx, root, ValidationKind::Check, &report);
            }
        }
        return report;
    }
    let backing = crate::registry::named_check_backing();
    let Some((_, rule_ids)) = backing.iter().find(|(candidate, _)| *candidate == name) else {
        return named_check_rejection(name, "missing_backing_declaration");
    };
    if rule_ids.is_empty() {
        return named_check_rejection(name, "native_engine_not_implemented");
    }
    const NATIVE_SCAN_FIELDS: &[&str] = &[
        "check",
        "root",
        "scope",
        "files",
        "crateName",
        "base",
        "head",
        "languages",
    ];
    let allowed_fields = if name == "literal-risk" {
        [
            NATIVE_SCAN_FIELDS,
            &[
                "minScore",
                "includeLow",
                "includeIgnored",
                "includeUnknownCode",
                "respectGitignore",
                "maxFileBytes",
                "failAbove",
                "hardCategories",
                "hardRuleIds",
                "literalRiskMinScore",
                "literalRiskIncludeLow",
                "literalRiskIncludeIgnored",
                "literalRiskIncludeUnknownCode",
                "literalRiskRespectGitignore",
                "literalRiskMaxFileBytes",
                "literalRiskFailAbove",
                "literalRiskHardCategories",
                "literalRiskHardRuleIds",
            ],
        ]
        .concat()
    } else if name == "sbom" {
        [NATIVE_SCAN_FIELDS, &["output"]].concat()
    } else if name == "required-tests" {
        [NATIVE_SCAN_FIELDS, &["strictEmptyTestTrees"]].concat()
    } else if name == "ai-rule-index" {
        [NATIVE_SCAN_FIELDS, &["maxLines"]].concat()
    } else if name == "source-shape"
        || name == "architecture-policy"
        || name == "single-source-contracts"
    {
        [NATIVE_SCAN_FIELDS, &["configPath"]].concat()
    } else {
        NATIVE_SCAN_FIELDS.to_vec()
    };
    if let Some(unsupported) = args.as_object().and_then(|object| {
        object
            .keys()
            .find(|field| !allowed_fields.contains(&field.as_str()))
    }) {
        return named_check_rejection(
            name,
            &format!("native_option_not_implemented:{unsupported}"),
        );
    }
    let Some(mut scan_args) = args.as_object().cloned() else {
        return json_error("check arguments must be an object");
    };
    scan_args.remove("check");
    let native_args = serde_json::Value::Object(scan_args);
    let mut report = match name {
        "secrets" => named_policy_unrecorded(&native_args, "secrets"),
        "dependency-policy" => named_policy_unrecorded(&native_args, "dependency-policy"),
        "import-boundaries" => named_import_boundaries_unrecorded(&native_args),
        "literal-risk" => named_literal_risk_unrecorded(&native_args),
        "reexports" => named_reexports_unrecorded(&native_args),
        "no-naked-domain-strings" | "rust-string-boundaries" => named_rust_string_boundaries_unrecorded(&native_args),
        "no-zod-source"
        | "no-test-doubles" | "weak-assertions" | "skipped-focused-tests"
        | "validation-bypass" | "placeholder-implementation"
        | "cross-platform-script-commands" => named_check_rejection(
            name,
            "narrow_native_engine_not_implemented: broad scan filtering is not a named-policy implementation",
        ),
        "required-tests" => named_required_tests_unrecorded(&native_args),
        "sbom" => named_sbom_unrecorded(&native_args),
        "source-shape" => named_source_shape_unrecorded(&native_args),
        "architecture-policy" => named_architecture_policy_unrecorded(&native_args),
        "single-source-contracts" => named_single_source_contracts_unrecorded(&native_args),
        "ai-rule-index" => named_ai_rule_index_unrecorded(&native_args),
        _ => scan_unrecorded(&native_args),
    };
    let Some(object) = report.as_object_mut() else {
        return json_error("native scan produced an invalid report shape");
    };
    if object.contains_key("error") {
        return report;
    }
    let declared_rule_ids: std::collections::BTreeSet<&str> =
        rule_ids.iter().map(|rule_id| rule_id.as_str()).collect();
    for field in ["violations", "warnings", "waived", "findings"] {
        if let Some(serde_json::Value::Array(findings)) = object.get_mut(field) {
            findings.retain(|finding| {
                finding
                    .get("ruleId")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|rule_id| declared_rule_ids.contains(rule_id))
            });
        }
    }
    let has_violation = object
        .get("violations")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|violations| !violations.is_empty());
    object.insert("ok".to_owned(), serde_json::Value::Bool(!has_violation));
    object.insert(
        "command".to_owned(),
        serde_json::Value::String("check".to_owned()),
    );
    object.insert(
        "check".to_owned(),
        serde_json::Value::String(name.to_owned()),
    );
    if let Some(root) = args.get("root").and_then(serde_json::Value::as_str) {
        record_validation_at_root(ctx, root, ValidationKind::Check, &report);
    }
    report
}
fn named_ai_rule_index_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let max_lines = match args.get("maxLines") {
        None => None,
        Some(serde_json::Value::Number(value)) => {
            value.as_u64().and_then(|value| usize::try_from(value).ok())
        }
        Some(_) => return json_error("ai-rule-index `maxLines` must be a non-negative integer"),
    };
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    enforcer_scan::boundary::native_scan::execute_ai_rule_index(&request, &root, max_lines)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| json_error(&error))
}

fn named_source_shape_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let config_path = args
        .get("configPath")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("ocentra-enforcer.config.json"));
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::path::Path::new(root.as_str()).join(config_path)
    };
    let config = match enforcer_config::load_project_config(&config_path) {
        Ok(config) => config,
        Err(error) => return json_error(&format!("cannot load source-shape config: {error}")),
    };
    enforcer_scan::boundary::native_scan::execute_source_shape_policy(&request, &root, &config)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| json_error(&error))
}

fn named_architecture_policy_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let config_path = args
        .get("configPath")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("ocentra-enforcer.config.json"));
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::path::Path::new(root.as_str()).join(config_path)
    };
    let config = match enforcer_config::load_project_config(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return json_error(&format!("cannot load architecture-policy config: {error}"))
        }
    };
    enforcer_scan::boundary::native_scan::execute_architecture_policy(&request, &root, &config)
        .map_err(|error| error.to_string())
        .and_then(|result| {
            let mut report =
                serde_json::to_value(result.report).map_err(|error| error.to_string())?;
            let object = report
                .as_object_mut()
                .ok_or_else(|| "native architecture-policy report is not an object".to_owned())?;
            object.insert(
                "checks".to_owned(),
                serde_json::Value::Array(
                    result
                        .checks
                        .into_iter()
                        .map(|check| {
                            serde_json::json!({
                                "check": check.check,
                                "ok": check.ok,
                                "unavailable": check.unavailable,
                                "violations": check.violations,
                            })
                        })
                        .collect(),
                ),
            );
            Ok(report)
        })
        .unwrap_or_else(|error| json_error(&error))
}

fn named_single_source_contracts_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    enforcer_scan::boundary::native_scan::execute_single_source_contracts(
        &request,
        &root,
        args.get("configPath").and_then(serde_json::Value::as_str),
    )
    .map_err(|error| error.to_string())
    .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
    .unwrap_or_else(|error| json_error(&error))
}

fn named_required_tests_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let strict = match args.get("strictEmptyTestTrees") {
        None => false,
        Some(serde_json::Value::Bool(value)) => *value,
        Some(_) => return json_error("required-tests `strictEmptyTestTrees` must be a boolean"),
    };
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let config_path = std::path::Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = match enforcer_config::load_project_config(&config_path) {
        Ok(config) => config,
        Err(error) => return json_error(&format!("cannot load required-tests config: {error}")),
    };
    enforcer_scan::boundary::native_scan::execute_required_test_policy(
        &request, &root, &config, strict,
    )
    .map_err(|error| error.to_string())
    .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
    .unwrap_or_else(|error| json_error(&error))
}

fn named_generated_artifacts_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let tracked = args
        .get("tracked")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let config_path = args
        .get("configPath")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("ocentra-enforcer.config.json"));
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::path::Path::new(root.as_str()).join(config_path)
    };
    let config = match enforcer_config::load_project_config(&config_path) {
        Ok(value) => value,
        Err(error) => {
            return json_error(&format!("cannot load generated-artifacts config: {error}"))
        }
    };
    match enforcer_scan::boundary::native_scan::execute_generated_artifacts(
        &request, &root, &config, tracked,
    )
    .and_then(|result| {
        serde_json::to_value(result.report).map_err(|error| {
            enforcer_scan::boundary::native_scan::NativeScanError::Io {
                operation: "generated-artifacts serialization",
                reason: error.to_string(),
            }
        })
    }) {
        Ok(report) => report,
        Err(error) => json_error(&error.to_string()),
    }
}

/// Generate a deterministic, lockfile-bound native Cargo SBOM. The output is
/// an explicit artifact path rather than a synthetic scanner finding.
fn named_sbom_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let Some(root_raw) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("sbom requires a `root` path");
    };
    let root = std::path::PathBuf::from(root_raw);
    let output = match args.get("output") {
        Some(serde_json::Value::String(value)) => root.join(value),
        Some(_) => return json_error("sbom `output` must be a string"),
        None => root.join("target").join("enforcer-sbom"),
    };
    match enforcer_scan::sbom_policy::generate_current_workspace(&root, &output) {
        Ok(artifact) => serde_json::json!({
            "ok": true,
            "command": "check",
            "check": "sbom",
            "artifact": artifact.to_string_lossy(),
            "violations": [],
            "warnings": [],
            "waived": [],
            "findings": [],
        }),
        Err(error) => json_error(&error),
    }
}

/// Execute named checks whose native implementation is intentionally narrower
/// than the full language scanner.  The scan request boundary remains shared,
/// so scope and language input cannot diverge between MCP tools.
fn named_policy_unrecorded(args: &serde_json::Value, policy: &str) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let result = match policy {
        "secrets" => enforcer_scan::boundary::native_scan::execute_secret_policy(&request, &root),
        "dependency-policy" => {
            enforcer_scan::boundary::native_scan::execute_dependency_policy(&request, &root)
        }
        _ => return json_error("named policy is not implemented"),
    };
    result
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| json_error(&error))
}

fn named_import_boundaries_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    enforcer_scan::boundary::native_scan::execute_import_boundaries_policy(&request, &root)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| json_error(&error))
}
fn named_reexports_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    enforcer_scan::boundary::native_scan::execute_reexports_policy(&request, &root)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| json_error(&error))
}

fn named_rust_string_boundaries_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    let (root, request) = match native_scan_request(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let config_path = args
        .get("configPath")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("ocentra-enforcer.config.json"));
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::path::Path::new(root.as_str()).join(config_path)
    };
    let config = match enforcer_config::load_project_config(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return json_error(&format!(
                "cannot load no-naked-domain-strings config: {error}"
            ))
        }
    };
    enforcer_scan::boundary::native_scan::execute_rust_string_boundaries_policy(
        &request, &root, &config,
    )
    .map_err(|error| error.to_string())
    .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()))
    .unwrap_or_else(|error| json_error(&error))
}

fn named_literal_risk_unrecorded(args: &serde_json::Value) -> serde_json::Value {
    use enforcer_domain::scan_types::{
        LiteralFileByteLimit, LiteralRiskScore, LiteralScanPaths, LiteralScanRoot,
        LiteralScanToggle,
    };
    let mut normalized = args.clone();
    if let Some(object) = normalized.as_object_mut() {
        for (alias, canonical) in [
            ("literalRiskMinScore", "minScore"),
            ("literalRiskIncludeLow", "includeLow"),
            ("literalRiskIncludeIgnored", "includeIgnored"),
            ("literalRiskIncludeUnknownCode", "includeUnknownCode"),
            ("literalRiskRespectGitignore", "respectGitignore"),
            ("literalRiskMaxFileBytes", "maxFileBytes"),
            ("literalRiskFailAbove", "failAbove"),
            ("literalRiskHardCategories", "hardCategories"),
            ("literalRiskHardRuleIds", "hardRuleIds"),
        ] {
            if !object.contains_key(canonical) {
                if let Some(value) = object.get(alias).cloned() {
                    object.insert(canonical.to_owned(), value);
                }
            }
        }
    }
    let args = &normalized;
    let Some(root) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("literal-risk requires a `root` path");
    };
    let score = |name: &str, default: u8| -> Result<LiteralRiskScore, String> {
        let value = args.get(name).map_or(Ok(default), |value| {
            value
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| format!("literal-risk `{name}` must be an integer from 0 to 100"))
        })?;
        std::num::NonZeroU8::new(value)
            .map(LiteralRiskScore::try_from)
            .transpose()
            .map(|value| value.unwrap_or(LiteralRiskScore::ZERO))
            .map_err(|error| error.to_string())
    };
    let boolean = |name: &str, default: bool| -> Result<bool, String> {
        args.get(name).map_or(Ok(default), |value| {
            value
                .as_bool()
                .ok_or_else(|| format!("literal-risk `{name}` must be a boolean"))
        })
    };
    let files: Result<Vec<std::path::PathBuf>, String> = match args.get("files") {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(std::path::PathBuf::from)
                    .ok_or_else(|| "literal-risk `files` must be strings".to_owned())
            })
            .collect::<Result<Vec<_>, _>>(),
        Some(_) => Err("literal-risk `files` must be an array".to_owned()),
    };
    let files = match files {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let max_bytes: Result<Option<LiteralFileByteLimit>, String> = match args.get("maxFileBytes") {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(std::num::NonZeroU64::new)
            .map(LiteralFileByteLimit::try_from_nonzero)
            .ok_or_else(|| "literal-risk `maxFileBytes` must be a positive integer".to_owned())
            .map(Some),
    };
    let max_bytes = match max_bytes {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let mut options = enforcer_literal_scan::CliOptions {
        root: LiteralScanRoot::from(std::path::PathBuf::from(root)),
        files: LiteralScanPaths::from(files),
        ..enforcer_literal_scan::CliOptions::default()
    };
    let min_score = match score("minScore", 40) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    options.min_score = min_score;
    let fail_above = match args.get("failAbove") {
        None | Some(serde_json::Value::Null) => None,
        Some(_) => match score("failAbove", 0) {
            Ok(value) => Some(value),
            Err(error) => return json_error(&error),
        },
    };
    options.fail_above = fail_above;
    for (name, target) in [
        ("includeLow", &mut options.include_low),
        ("includeIgnored", &mut options.include_ignored),
        ("includeUnknownCode", &mut options.include_unknown_code),
        ("respectGitignore", &mut options.respect_gitignore),
    ] {
        let default = name == "includeLow" || name == "respectGitignore";
        match boolean(name, default) {
            Ok(value) => *target = LiteralScanToggle::from(value),
            Err(error) => return json_error(&error),
        }
    }
    if let Some(value) = max_bytes {
        options.max_file_bytes = value;
    }
    let hard_categories = string_set(args, "hardCategories");
    let hard_rules = string_set(args, "hardRuleIds");
    if hard_categories.is_err() || hard_rules.is_err() {
        return json_error("literal-risk hardCategories and hardRuleIds must be string arrays");
    }
    // Frozen standalone literal-risk semantics always treat secret-shaped
    // findings as hard, even when callers do not repeat the default category.
    let mut hard_categories = hard_categories.unwrap_or_default();
    hard_categories.insert("secret-like");
    let hard_rules = hard_rules.unwrap_or_default();
    match enforcer_literal_scan::run_scan(&options) {
        Ok(report) => {
            let map = |finding: &enforcer_literal_scan::Finding, fallback: &str| {
                let category = finding.category.wire_name();
                let rule = finding.rule_id.as_str();
                let blocking = finding.blocking.is_blocking()
                    || hard_categories.contains(category)
                    || hard_rules.contains(rule);
                serde_json::json!({"ruleId":rule,"severity":if blocking {"error"} else {fallback},"file":finding.file.to_string(),"line":finding.line.get(),"detail":finding.reason.to_string(),"snippet":finding.literal_preview.to_string(),"category":category,"score":finding.score.get(),"confidence":finding.confidence.wire_name(),"fileRole":finding.file_role.wire_name(),"literalKind":finding.literal_kind.wire_name(),"literalPreview":finding.literal_preview.to_string(),"literalHash":finding.literal_hash.to_string(),"blocking":blocking,"source":finding.context.to_string()})
            };
            let findings = report
                .hard_findings
                .iter()
                .map(|f| map(f, "error"))
                .chain(report.literal_risks.iter().map(|f| map(f, "warning")))
                .collect::<Vec<_>>();
            let ok = !findings
                .iter()
                .any(|row| row.get("blocking") == Some(&serde_json::Value::Bool(true)));
            serde_json::json!({"ok":ok,"check":"literal-risk","findings":findings,"violations":[],"warnings":[],"waived":[],"literalRiskReport":{"summary":{"filesDiscovered":report.summary.files_discovered.get(),"filesScanned":report.summary.files_scanned.get(),"literalRisks":report.summary.literal_risks.get(),"hardFindings":report.summary.hard_findings.get()},"options":{"minScore":options.min_score.get(),"includeLow":options.include_low.is_enabled(),"includeIgnored":options.include_ignored.is_enabled(),"includeUnknownCode":options.include_unknown_code.is_enabled(),"respectGitignore":options.respect_gitignore.is_enabled(),"failAbove":options.fail_above.map(|value|value.get())}}})
        }
        Err(error) => json_error(&format!("literal-risk native scan failed: {error}")),
    }
}
fn string_set<'a>(
    args: &'a serde_json::Value,
    name: &str,
) -> Result<std::collections::BTreeSet<&'a str>, ()> {
    args.get(name)
        .map_or(Ok(std::collections::BTreeSet::new()), |value| {
            value.as_array().ok_or(()).and_then(|values| {
                values
                    .iter()
                    .map(|value| value.as_str().ok_or(()))
                    .collect()
            })
        })
}

/// Return a structured refusal for a frozen check whose dedicated native
/// engine has not landed. This must stay distinct from a clean report.
fn named_check_rejection(name: &str, code: &str) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "check": name,
        "error": {
            "code": code,
            "message": format!("native MCP check `{name}` cannot execute: {code}"),
        },
    })
}

fn record_validation(
    ctx: &DispatchContext,
    root: RepoRoot,
    kind: ValidationKind,
    report: &serde_json::Value,
) {
    if let Ok(mut history) = ctx.validation_history.lock() {
        history.record(validation_summary_from_report(root, kind, report));
    }
}

fn record_validation_at_root(
    ctx: &DispatchContext,
    root: &str,
    kind: ValidationKind,
    report: &serde_json::Value,
) {
    if let Ok(root) = root.parse::<RepoRoot>() {
        record_validation(ctx, root, kind, report);
    }
}

fn validation_summary_from_report(
    root: RepoRoot,
    kind: ValidationKind,
    report: &serde_json::Value,
) -> ValidationSummary {
    let findings = ["violations", "warnings"].into_iter().flat_map(|field| {
        report
            .get(field)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
    });
    let mut by_severity = std::collections::BTreeMap::new();
    let mut rule_ids = std::collections::BTreeSet::new();
    let mut docs = std::collections::BTreeSet::new();
    let mut finding_count = 0;
    for finding in findings {
        finding_count += 1;
        if let Some(severity) = finding.get("severity").and_then(serde_json::Value::as_str) {
            if let Ok(label) =
                McpReportLabelText::try_new(severity.to_owned()).map(ReportLabel::try_new)
            {
                by_severity.entry(label).or_insert(SeverityCount(0)).0 += 1;
            }
        }
        if let Some(rule_id) = finding.get("ruleId").and_then(serde_json::Value::as_str) {
            if let Ok(label) =
                McpReportLabelText::try_new(rule_id.to_owned()).map(ReportLabel::try_new)
            {
                rule_ids.insert(label);
            }
        }
        if let Some(doc) = finding.get("doc").and_then(serde_json::Value::as_str) {
            if let Ok(label) = McpReportLabelText::try_new(doc.to_owned()).map(ReportLabel::try_new)
            {
                docs.insert(label);
            }
        }
    }
    let timestamp = enforcer_core::platform::epoch_millis()
        .map(enforcer_core::platform::iso8601_utc)
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_owned());
    ValidationSummary {
        kind,
        command: boundary_label(report.get("command")),
        check: boundary_label(report.get("check")),
        outcome: match report.get("ok").and_then(serde_json::Value::as_bool) {
            Some(true) => ValidationOutcome::Passed,
            Some(false) => ValidationOutcome::Failed,
            None => ValidationOutcome::Unknown,
        },
        root,
        profile_name: boundary_label(report.get("profileName")),
        at: ValidationTimestamp::parse(
            McpReportLabelText::try_new(timestamp)
                .map(ReportLabel::try_new)
                .unwrap_or_else(|_| ReportLabel::try_new(McpReportLabelText::epoch_fallback())),
        ),
        by_severity,
        counts: ValidationCounts {
            findings: FindingCount(finding_count),
            violations: FindingCount(
                report
                    .get("violations")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, Vec::len),
            ),
            warnings: FindingCount(
                report
                    .get("warnings")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, Vec::len),
            ),
        },
        rule_ids: rule_ids.into_iter().collect(),
        docs: docs.into_iter().collect(),
        scope: compact_validation_scope(report.get("scope")),
    }
}

fn boundary_label(value: Option<&serde_json::Value>) -> Option<ReportLabel> {
    value.and_then(serde_json::Value::as_str).and_then(|text| {
        McpReportLabelText::try_new(text.to_owned())
            .ok()
            .map(ReportLabel::try_new)
    })
}

fn compact_validation_scope(scope: Option<&serde_json::Value>) -> Option<CompactScope> {
    let object = scope?.as_object()?;
    let sample_files = object
        .get("files")
        .and_then(serde_json::Value::as_array)
        .map(|files| {
            files
                .iter()
                .take(20)
                .filter_map(serde_json::Value::as_str)
                .filter_map(|file| {
                    McpReportLabelText::try_new(file.to_owned())
                        .ok()
                        .map(ReportLabel::try_new)
                })
                .collect()
        })
        .unwrap_or_default();
    Some(CompactScope {
        mode: boundary_label(object.get("mode")),
        crate_name: boundary_label(object.get("crateName")),
        base: boundary_label(object.get("base")),
        head: boundary_label(object.get("head")),
        file_count: object
            .get("files")
            .and_then(serde_json::Value::as_array)
            .map(|files| FindingCount(files.len())),
        sample_files,
    })
}

fn validation_summary_json(summary: &ValidationSummary) -> serde_json::Value {
    let kind = match summary.kind {
        ValidationKind::Scan => "scan",
        ValidationKind::Check => "check",
    };
    let mut object = serde_json::json!({
        "kind": kind,
        "ok": match summary.outcome { ValidationOutcome::Passed => Some(true), ValidationOutcome::Failed => Some(false), ValidationOutcome::Unknown => None },
        "root": summary.root.as_str(),
        "at": String::from(&summary.at),
        "bySeverity": summary.by_severity.iter().map(|(name, count)| (String::from(name), serde_json::json!(count.0))).collect::<serde_json::Map<_, _>>(),
        "counts": {"findings": summary.counts.findings.0, "violations": summary.counts.violations.0, "warnings": summary.counts.warnings.0},
        "ruleIds": summary.rule_ids.iter().map(String::from).collect::<Vec<_>>(),
        "docs": summary.docs.iter().map(String::from).collect::<Vec<_>>(),
    });
    let Some(fields) = object.as_object_mut() else {
        return serde_json::Value::Null;
    };
    if let Some(value) = &summary.command {
        fields.insert(
            "command".to_owned(),
            serde_json::Value::String(String::from(value)),
        );
    }
    if let Some(value) = &summary.check {
        fields.insert(
            "check".to_owned(),
            serde_json::Value::String(String::from(value)),
        );
    }
    if let Some(value) = &summary.profile_name {
        fields.insert(
            "profileName".to_owned(),
            serde_json::Value::String(String::from(value)),
        );
    }
    if let Some(scope) = &summary.scope {
        let mut compact = serde_json::Map::new();
        if let Some(value) = &scope.mode {
            compact.insert(
                "mode".to_owned(),
                serde_json::Value::String(String::from(value)),
            );
        }
        if let Some(value) = &scope.crate_name {
            compact.insert(
                "crateName".to_owned(),
                serde_json::Value::String(String::from(value)),
            );
        }
        if let Some(value) = &scope.base {
            compact.insert(
                "base".to_owned(),
                serde_json::Value::String(String::from(value)),
            );
        }
        if let Some(value) = &scope.head {
            compact.insert(
                "head".to_owned(),
                serde_json::Value::String(String::from(value)),
            );
        }
        if let Some(file_count) = scope.file_count {
            compact.insert("fileCount".to_owned(), serde_json::json!(file_count.0));
            compact.insert(
                "sampleFiles".to_owned(),
                serde_json::json!(scope
                    .sample_files
                    .iter()
                    .map(String::from)
                    .collect::<Vec<_>>()),
            );
        }
        fields.insert("scope".to_owned(), serde_json::Value::Object(compact));
    }
    object
}

fn run_harness(args: &serde_json::Value) -> serde_json::Value {
    let request = match crate::boundary::harness_run::decode_run(args) {
        Ok(request) => request,
        Err(error) => return json_error(&error),
    };
    match crate::application::harness_run::execute(request) {
        Ok(outcome) => {
            serde_json::json!({"ok": outcome.status == HarnessRunStatus::Passed, "summary": outcome.summary, "diagnostics": outcome.diagnostics.iter().map(|diagnostic| serde_json::json!({"runId":diagnostic.run_id.as_str(),"tool":diagnostic.tool.as_str(),"language":diagnostic.language.as_str(),"severity":format!("{:?}", diagnostic.severity).to_ascii_lowercase(),"ruleId":diagnostic.rule_id.as_str(),"file":diagnostic.file.as_str(),"line":diagnostic.line.get(),"message":diagnostic.message.as_str(),"source":diagnostic.source.as_ref().map(|value| value.as_str()),"fingerprint":diagnostic.fingerprint.as_ref().map(|value| value.as_str())})).collect::<Vec<_>>() })
        }
        Err(error) => json_error(&error.to_string()),
    }
}

fn prune_runs(args: &serde_json::Value) -> serde_json::Value {
    let request = match crate::boundary::harness_run::decode_prune(args) {
        Ok(request) => request,
        Err(error) => return json_error(&error),
    };
    match crate::application::harness_run::prune_frozen_compat(&request) {
        Ok(outcome) => {
            serde_json::json!({"ok": true, "root": request.repo_root.as_str(), "removed": outcome.removed})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

/// Frozen-MJS-compatible status envelope: durable harness summary wins over
/// transient validation history, while validation history remains observable.
fn run_status(args: &serde_json::Value, ctx: &DispatchContext) -> serde_json::Value {
    let request = match decode_harness_query(args, "run_status") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let summary = match enforcer_harness::query::run_summary(
        std::path::Path::new(request.root.as_str()),
        &request.config,
        &request.query,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    let validation_summary = ctx.validation_history.lock().ok().and_then(|history| {
        let filter = match args.get("tool").and_then(serde_json::Value::as_str) {
            Some("scan") => Some(ValidationKind::Scan),
            Some("check") => Some(ValidationKind::Check),
            _ => None,
        };
        history
            .latest(&request.root, filter)
            .map(validation_summary_json)
    });
    let artifact = if summary.is_some() && args.get("artifact").is_some() {
        match enforcer_harness::query::read_artifact(
            std::path::Path::new(request.root.as_str()),
            &request.config,
            &request.query,
            request.artifact,
            request.limit_bytes,
        ) {
            Ok((true, Some(run_id), text, _)) => {
                let path = summary
                    .as_ref()
                    .and_then(|run| run.get("artifacts"))
                    .and_then(|artifacts| artifacts.get(request.artifact.as_str()))
                    .and_then(serde_json::Value::as_str);
                match path {
                    Some(path) => Some(
                        serde_json::json!({"ok":true,"runId":run_id,"artifact":request.artifact.as_str(),"path":path,"text":text}),
                    ),
                    None => {
                        Some(serde_json::json!({"ok":false,"text":"","message":"Unknown artifact"}))
                    }
                }
            }
            Ok((false, _, text, Some(message))) => {
                Some(serde_json::json!({"ok":false,"text":text,"message":message}))
            }
            Ok((false, _, text, None)) => {
                Some(serde_json::json!({"ok":false,"text":text,"message":"No harness run found."}))
            }
            Ok((true, None, _, _)) => {
                Some(serde_json::json!({"ok":false,"text":"","message":"No harness run found."}))
            }
            Err(error) => {
                Some(serde_json::json!({"ok":false,"text":"","message":error.to_string()}))
            }
        }
    } else {
        None
    };
    let summary_type = if summary.is_some() {
        "harness"
    } else if validation_summary.is_some() {
        "validation"
    } else {
        "none"
    };
    let selected_summary = summary
        .as_ref()
        .or(validation_summary.as_ref())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let mut result = serde_json::json!({
        "ok": true,
        "summary": selected_summary,
        "summaryType": summary_type,
        "validationSummary": validation_summary,
    });
    if let Some(artifact) = artifact {
        if let Some(object) = result.as_object_mut() {
            object.insert("artifact".to_owned(), artifact);
        }
    }
    result
}

/// Native repository-readiness doctor. This intentionally invokes
/// `enforcer-scan::doctor`, not the separate user-harness registration doctor.
fn doctor(args: &serde_json::Value) -> serde_json::Value {
    const SUPPORTED_FIELDS: &[&str] = &[
        "root",
        "configPath",
        "profile",
        "scope",
        "files",
        "crateName",
        "base",
        "head",
    ];
    let Some(object) = args.as_object() else {
        return json_error("doctor arguments must be an object");
    };
    if let Some(field) = object
        .keys()
        .find(|field| !SUPPORTED_FIELDS.contains(&field.as_str()))
    {
        return json_error(&format!("doctor does not support `{field}`"));
    }
    let root = match args.get("root").and_then(serde_json::Value::as_str) {
        Some(value) => value.parse::<RepoRoot>().map_err(|error| error.to_string()),
        None => std::env::current_dir()
            .map_err(|error| error.to_string())
            .and_then(|path| {
                path.to_string_lossy()
                    .parse::<RepoRoot>()
                    .map_err(|error| error.to_string())
            }),
    };
    let root = match root {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let files = match args.get("files") {
        None => None,
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .map(|value| value.as_str().map(std::path::PathBuf::from))
            .collect(),
        Some(_) => return json_error("doctor `files` must be an array"),
    };
    let scope = match parse_scan_scope(args.get("scope"), files, args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let config_path = args
        .get("configPath")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(root.as_str()).join("ocentra-enforcer.config.json")
        });
    let config = match args.get("profile").and_then(serde_json::Value::as_str) {
        Some(profile) => ConfigProfileName::try_new(profile.to_owned())
            .map_err(|error| error.to_string())
            .and_then(|profile| {
                enforcer_config::resolve::resolve_profile_only(&profile)
                    .map_err(|error| error.to_string())
            }),
        None => {
            enforcer_config::load_project_config(&config_path).map_err(|error| error.to_string())
        }
    };
    let config = match config {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let root_text = root.as_str().to_owned();
    let request = enforcer_scan::doctor::DoctorRequest::new(
        root,
        enforcer_scan::boundary::native_scan::NativeScanRequest {
            scope,
            languages: Vec::new(),
        },
        config,
    );
    match enforcer_scan::doctor::run(&request) {
        Ok(report) => serde_json::json!({
            "ok": report.ok(), "command": report.command(), "root": root_text,
            "profileName": report.profile_name(),
            "checks": report.checks().iter().map(|check| serde_json::json!({"name": check.name(), "ok": check.ok(), "detail": check.detail()})).collect::<Vec<_>>(),
            "violations": [],
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

/// Typed result from the frozen harness-query wire adapter.
struct HarnessQueryRequest {
    root: RepoRoot,
    config: enforcer_domain::config_types::HarnessConfig,
    query: enforcer_harness::query::RunQuery,
    diagnostics: enforcer_harness::query::DiagnosticsFilter,
    diagnostic_limit: Option<HarnessRunLimit>,
    artifact: HarnessArtifactKind,
    limit_bytes: Option<HarnessArtifactByteLimit>,
}

/// Shared typed adapter for the frozen harness-query tools. The raw JSON
/// shape stops here; all storage selection and filtering remains owned by
/// `enforcer-harness`.
fn decode_harness_query(
    args: &serde_json::Value,
    operation: &str,
) -> Result<HarnessQueryRequest, String> {
    const SUPPORTED_FIELDS: &[&str] = &[
        "root",
        "runId",
        "limit",
        "diagnosticLimit",
        "severity",
        "status",
        "file",
        "tool",
        "crateName",
        "packageName",
        "domain",
        "tag",
        "artifact",
        "limitBytes",
    ];
    let object = args
        .as_object()
        .ok_or_else(|| format!("{operation} arguments must be an object"))?;
    if let Some(field) = object
        .keys()
        .find(|field| !SUPPORTED_FIELDS.contains(&field.as_str()))
    {
        return Err(format!("{operation} does not support `{field}`"));
    }
    let root = match args.get("root") {
        Some(serde_json::Value::String(value)) => {
            value.parse::<RepoRoot>().map_err(|error| error.to_string())
        }
        Some(_) => return Err(format!("{operation} `root` must be a string")),
        None => std::env::current_dir()
            .map_err(|error| error.to_string())
            .and_then(|path| {
                path.to_string_lossy()
                    .parse::<RepoRoot>()
                    .map_err(|error| error.to_string())
            }),
    }?;
    let config_path = std::path::Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = enforcer_config::load_project_config(&config_path)
        .map_err(|error| error.to_string())?
        .harness;
    let optional_text = |name: &str| -> Result<Option<&str>, String> {
        match args.get(name) {
            Some(serde_json::Value::String(value)) => Ok(Some(value)),
            Some(_) => Err(format!("{operation} `{name}` must be a string")),
            None => Ok(None),
        }
    };
    let optional_limit = |name: &str| -> Result<Option<HarnessRunLimit>, String> {
        match args.get(name) {
            Some(value) => value
                .as_u64()
                .map(HarnessRunLimit::from_value)
                .ok_or_else(|| format!("{operation} `{name}` must be a non-negative integer"))
                .map(Some),
            None => Ok(None),
        }
    };
    let query = enforcer_harness::query::RunQuery {
        run_id: optional_text("runId")?
            .map(str::parse::<HarnessRunId>)
            .transpose()
            .map_err(|error| error.to_string())?,
        status: match optional_text("status")? {
            Some("passed") => Some(HarnessRunStatus::Passed),
            Some("failed") => Some(HarnessRunStatus::Failed),
            Some(_) => return Err(format!("{operation} `status` must be `passed` or `failed`")),
            None => None,
        },
        tool: optional_text("tool")?
            .map(str::parse::<HarnessToolName>)
            .transpose()
            .map_err(|error| error.to_string())?,
        crate_name: optional_text("crateName")?
            .map(str::parse::<CrateName>)
            .transpose()
            .map_err(|error| error.to_string())?,
        package_name: optional_text("packageName")?
            .map(str::parse::<HarnessPackageName>)
            .transpose()
            .map_err(|error| error.to_string())?,
        domain: optional_text("domain")?
            .map(str::parse::<HarnessDomainName>)
            .transpose()
            .map_err(|error| error.to_string())?,
        tag: optional_text("tag")?
            .map(str::parse::<HarnessTag>)
            .transpose()
            .map_err(|error| error.to_string())?,
        limit: optional_limit("limit")?,
    };
    let diagnostics = enforcer_harness::query::DiagnosticsFilter {
        severity: match optional_text("severity")? {
            Some("error") => Some(Severity::Error),
            Some("warning") => Some(Severity::Warning),
            Some("info") => Some(Severity::Info),
            Some(_) => {
                return Err(format!(
                    "{operation} `severity` must be `error`, `warning`, or `info`"
                ));
            }
            None => None,
        },
        file: optional_text("file")?
            .map(str::parse::<RelPath>)
            .transpose()
            .map_err(|error| error.to_string())?,
        limit: optional_limit("limit")?,
    };
    let artifact = match optional_text("artifact")? {
        Some("stdout") => HarnessArtifactKind::Stdout,
        Some("stderr") | None => HarnessArtifactKind::Stderr,
        Some("diagnostics") => HarnessArtifactKind::Diagnostics,
        Some("events") => HarnessArtifactKind::Events,
        Some(_) => {
            return Err(format!(
                "{operation} `artifact` must be stdout, stderr, diagnostics, or events"
            ));
        }
    };
    let limit_bytes = match args.get("limitBytes") {
        Some(value) => value
            .as_u64()
            .map(HarnessArtifactByteLimit::from_value)
            .ok_or_else(|| format!("{operation} `limitBytes` must be a non-negative integer"))
            .map(Some)?,
        None => None,
    };
    Ok(HarnessQueryRequest {
        root,
        config,
        query,
        diagnostics,
        diagnostic_limit: optional_limit("diagnosticLimit")?,
        artifact,
        limit_bytes,
    })
}

fn diagnostics(args: &serde_json::Value) -> serde_json::Value {
    let request = match decode_harness_query(args, "diagnostics") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::query::run_diagnostics(
        std::path::Path::new(request.root.as_str()),
        &request.config,
        &request.query,
        &request.diagnostics,
    ) {
        Ok((true, Some(run_id), diagnostics)) => serde_json::json!({
            "ok": true, "runId": run_id, "diagnostics": diagnostics,
        }),
        Ok((false, _, _)) => serde_json::json!({
            "ok": false, "diagnostics": [], "message": "No harness run found.",
        }),
        Ok((true, None, diagnostics)) => serde_json::json!({
            "ok": true, "diagnostics": diagnostics,
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

fn last_failure(args: &serde_json::Value) -> serde_json::Value {
    let request = match decode_harness_query(args, "last_failure") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::query::last_failure(
        std::path::Path::new(request.root.as_str()),
        &request.config,
        &request.query,
        request.diagnostic_limit,
    ) {
        Ok((true, Some(run), diagnostics)) => serde_json::json!({
            "ok": true, "found": true, "run": run, "diagnostics": diagnostics,
        }),
        Ok((false, _, _)) => serde_json::json!({
            "ok": true, "found": false, "message": "No failed harness run found.",
        }),
        Ok((true, None, diagnostics)) => serde_json::json!({
            "ok": true, "found": false, "message": "No failed harness run found.", "diagnostics": diagnostics,
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

fn artifact(args: &serde_json::Value) -> serde_json::Value {
    let request = match decode_harness_query(args, "artifact") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::query::read_artifact(
        std::path::Path::new(request.root.as_str()),
        &request.config,
        &request.query,
        request.artifact,
        request.limit_bytes,
    ) {
        Ok((true, Some(run_id), text, _)) => {
            let path = enforcer_harness::query::run_summary(
                std::path::Path::new(request.root.as_str()),
                &request.config,
                &request.query,
            )
            .ok()
            .flatten()
            .and_then(|run| {
                run.get("artifacts")?
                    .get(request.artifact.as_str())?
                    .as_str()
                    .map(str::to_owned)
            });
            match path {
                Some(path) => serde_json::json!({
                    "ok": true, "runId": run_id, "artifact": request.artifact.as_str(), "path": path, "text": text,
                }),
                None => {
                    json_error("native artifact query did not resolve the selected artifact path")
                }
            }
        }
        Ok((false, _, text, Some(message))) => serde_json::json!({
            "ok": false, "text": text, "message": message,
        }),
        Ok((false, _, text, None)) => serde_json::json!({
            "ok": false, "text": text, "message": "No harness run found.",
        }),
        Ok((true, None, _, _)) => {
            json_error("native artifact query did not return its selected run id")
        }
        Err(error) => json_error(&error.to_string()),
    }
}

/// Clear the complete typed harness store. Query filters are still decoded by
/// the shared frozen-schema adapter, but reset intentionally has no filter:
/// it removes every candidate storage root, exactly as the legacy tool does.
fn reset_runs(args: &serde_json::Value) -> serde_json::Value {
    let request = match decode_harness_query(args, "reset_runs") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::storage::reset_runs(
        std::path::Path::new(request.root.as_str()),
        &request.config,
    ) {
        Ok(removed) => serde_json::json!({
            "ok": true,
            "root": request.root.as_str(),
            "removed": removed.iter().map(|path| path.as_str()).collect::<Vec<_>>(),
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

/// Native MCP route adapter. The route plan is built by `enforcer-scan` from
/// the same walked paths and resolved project tie that native scan consumers
/// use; this boundary only validates the wire scope and projects the result.
fn route(args: &serde_json::Value) -> serde_json::Value {
    let Some(root_raw) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("route requires a `root` path");
    };
    let root = match root_raw.parse::<RepoRoot>() {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let requested_files = match args.get("files") {
        None => Vec::new(),
        Some(serde_json::Value::Array(values)) => match values
            .iter()
            .map(|value| value.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
        {
            Some(values) => values,
            None => return json_error("route `files` must contain only paths"),
        },
        Some(_) => return json_error("route `files` must be an array"),
    };
    let scope_name = match args.get("scope") {
        None if requested_files.is_empty() => "workspace",
        None => "files",
        Some(serde_json::Value::String(value)) if value == "workspace" || value == "files" => {
            value.as_str()
        }
        Some(_) => {
            return json_error("native MCP route supports only `files` or `workspace` scope");
        }
    };
    if scope_name == "files" && requested_files.is_empty() {
        return json_error("route `scope: files` requires at least one file");
    }
    if scope_name == "workspace" && !requested_files.is_empty() {
        return json_error("route `scope: workspace` cannot combine with `files`");
    }

    let root_path = std::path::Path::new(root.as_str());
    let mut paths =
        match enforcer_scan::walk::walk(root_path, &enforcer_scan::walk::IgnoreRules::default()) {
            Ok(value) => value,
            Err(err) => return json_error(&format!("route walk failed: {err}")),
        };
    if scope_name == "files" {
        paths.retain(|path| requested_files.iter().any(|file| path.as_str() == file));
        if paths.is_empty() {
            return json_error("route `files` did not resolve to any walked source files");
        }
    }
    let tie =
        match enforcer_config::project_tie::load_project_tie(&root_path.join(".enforce/config")) {
            Ok(value) => value,
            Err(err) => return json_error(&format!("route config failed: {err}")),
        };
    let scope = if scope_name == "workspace" {
        RouteScope::Workspace
    } else {
        RouteScope::Repo
    };
    let plan = enforcer_scan::router::plan::build_route_plan(&paths, &scope, &tie);
    match serde_json::to_value(plan) {
        Ok(serde_json::Value::Object(mut value)) => {
            value.insert("ok".to_owned(), serde_json::Value::Bool(true));
            serde_json::Value::Object(value)
        }
        Ok(_) => json_error("native route produced an invalid report shape"),
        Err(err) => json_error(&format!("failed to encode route plan: {err}")),
    }
}

/// Native MCP adapter for the test-doctrine posture analysis. The analyzer
/// owns all filesystem and evidence interpretation; this boundary only
/// decodes the branded root and encodes its typed report.
fn test_doctrine_scan(args: &serde_json::Value) -> serde_json::Value {
    let root_raw = match args.get("root") {
        None => match std::env::current_dir() {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(err) => {
                return json_error(&format!("cannot resolve default test-doctrine root: {err}"));
            }
        },
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(_) => return json_error("test_doctrine_scan `root` must be a string"),
    };
    let root = match root_raw.parse::<RepoRoot>() {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    match enforcer_scan::test_doctrine::analyze(&root) {
        Ok(report) => match serde_json::to_value(report) {
            Ok(serde_json::Value::Object(mut value)) => {
                value.insert("ok".to_owned(), serde_json::Value::Bool(true));
                serde_json::Value::Object(value)
            }
            Ok(_) => json_error("native test-doctrine analysis produced an invalid report shape"),
            Err(err) => json_error(&format!(
                "failed to encode native test-doctrine report: {err}"
            )),
        },
        Err(err) => json_error(&format!("native test-doctrine analysis failed: {err}")),
    }
}

/// Native, advisory ARCH-1.16 adapter. It returns evidence only; no scan or
/// CI policy is changed by this dedicated report tool.
fn ui_logic_coupling_scan(args: &serde_json::Value) -> serde_json::Value {
    let root_raw = match args.get("root") {
        None => match std::env::current_dir() {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(err) => {
                return json_error(&format!("cannot resolve default UI coupling root: {err}"));
            }
        },
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(_) => return json_error("ui_logic_coupling_scan `root` must be a string"),
    };
    let root = match root_raw.parse::<RepoRoot>() {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    match enforcer_scan::ui_logic_coupling::analyze(&root) {
        Ok(report) => match serde_json::to_value(report) {
            Ok(serde_json::Value::Object(mut value)) => {
                value.insert("ok".to_owned(), serde_json::Value::Bool(true));
                serde_json::Value::Object(value)
            }
            Ok(_) => json_error("native UI coupling analysis produced an invalid report shape"),
            Err(err) => json_error(&format!(
                "failed to encode native UI coupling report: {err}"
            )),
        },
        Err(err) => json_error(&format!("native UI coupling analysis failed: {err}")),
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
        "directWritesAllowed": matches!(ctx.freshness, McpFreshness::Fresh),
        "hashCompatible": !matches!(ctx.freshness, McpFreshness::HashIncompatible),
        "aliasWindowOpen": crate::aliases::deprecation_window_open(),
        "toolCount": descriptors.len(),
        "toolSurfaceBytes": crate::registry::tool_surface_bytes(&descriptors),
    })
}

/// Coordination MCP family.  This boundary only projects operations that
/// have a durable `enforcer-coordination` backing.  The remaining frozen MJS
/// names are deliberately refused with an explicit machine-readable reason;
/// a successful response must always correspond to a real native ledger
/// read or append-only event.
fn coordination(operation: &str, args: &serde_json::Value) -> serde_json::Value {
    let Some(root_raw) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error(&format!("{operation} requires a `root` ledger path"));
    };
    let root_path = std::path::Path::new(root_raw);
    match operation {
        "ocentra_enforcer_coordination_status" | "ocentra_enforcer_coordination_health" => {
            coordination_status(args)
        }
        "ocentra_enforcer_coordination_streams" => {
            match enforcer_coordination::sync::stream::read_all_streams(root_path) {
                Ok(all) => serde_json::json!({
                    "ok": true,
                    "eventCount": all.events.len(),
                    "duplicateCount": all.duplicate_count.as_nonzero().map_or(0, std::num::NonZeroUsize::get),
                    "warningCount": all.warnings.len(),
                    "streams": enforcer_coordination::sync::stream::list_stream_files(root_path)
                        .map(|names| names.into_iter().map(|name| name.as_str().to_owned()).collect::<Vec<_>>())
                        .unwrap_or_default(),
                }),
                Err(error) => json_error(&error.to_string()),
            }
        }
        "ocentra_enforcer_coordination_presence" => {
            match enforcer_coordination::sync::stream::read_all_streams(root_path) {
                Ok(all) => {
                    let mut lanes = std::collections::BTreeSet::new();
                    let mut writers = std::collections::BTreeSet::new();
                    for event in &all.events {
                        lanes.insert(event.lane.clone());
                        writers.insert(event.writer.clone());
                    }
                    serde_json::json!({"ok":true,"laneCount":lanes.len(),"writerCount":writers.len(),"lanes":lanes,"writers":writers})
                }
                Err(error) => json_error(&error.to_string()),
            }
        }
        "ocentra_enforcer_coordination_inbox" => {
            match enforcer_coordination::sync::stream::read_all_streams(root_path) {
                Ok(all) => {
                    let lane = args.get("lane").and_then(serde_json::Value::as_str);
                    let messages: Vec<_> = all
                        .events
                        .into_iter()
                        .filter(|event| {
                            event.kind == "message"
                                && lane.is_none_or(|value| event.to.as_deref() == Some(value))
                        })
                        .collect();
                    serde_json::json!({"ok":true,"messageCount":messages.len(),"messages":messages})
                }
                Err(error) => json_error(&error.to_string()),
            }
        }
        "ocentra_enforcer_coordination_workers" => {
            coordination_event_rows(root_path, "worker", "workers")
        }
        "ocentra_enforcer_coordination_tasks" => {
            coordination_event_rows(root_path, "task", "tasks")
        }
        "ocentra_enforcer_coordination_init" => coordination_init(args),
        "ocentra_enforcer_coordination_claim" => coordination_claim(args),
        "ocentra_enforcer_coordination_release" => coordination_release(args),
        "ocentra_enforcer_coordination_closeout" => coordination_closeout(args),
        "ocentra_enforcer_coordination_message" => coordination_message(args),
        "ocentra_enforcer_coordination_mail" => coordination_mail(args),
        "ocentra_enforcer_coordination_guard" => coordination_guard(args),
        "ocentra_enforcer_coordination_compact" => coordination_compact(args),
        "ocentra_enforcer_coordination_report" => coordination_report(args),
        "ocentra_enforcer_coordination_index" => coordination_index(args),
        "ocentra_enforcer_coordination_notify" => coordination_notify(args),
        "ocentra_enforcer_coordination_peer" => coordination_peer(args),
        "ocentra_enforcer_coordination_sync" => coordination_sync(args),
        "ocentra_enforcer_coordination_ensure" => coordination_ensure(args),
        "ocentra_enforcer_coordination_repair" => coordination_repair(args),
        unsupported => serde_json::json!({
            "ok": false,
            "operation": unsupported,
            "refusal": "native coordination engine has no durable backing for this frozen operation yet",
            "code": "native_coordination_operation_unavailable",
        }),
    }
}

fn coordination_ensure(args: &serde_json::Value) -> serde_json::Value {
    let host = args
        .get("host")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("127.0.0.1");
    let port = match args.get("port") {
        None => 8787_u16,
        Some(value) => value
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(0),
    };
    if port == 0 {
        return json_error("coordination_ensure `port` must be a u16");
    }
    let token = match args.get("token") {
        None => None,
        Some(serde_json::Value::String(value)) if !value.is_empty() => Some(value.as_str()),
        Some(_) => return json_error("coordination_ensure `token` must be a non-empty string"),
    };
    match enforcer_coordination::daemon::boundary::ensure(host, port, token) {
        Ok(status) => serde_json::json!({"ok":true,"service":status}),
        Err(error) => json_error(&error),
    }
}

fn coordination_repair(args: &serde_json::Value) -> serde_json::Value {
    let action = args
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("stale-claims");
    if !matches!(action, "stale-claims" | "claim-conflicts" | "conflicts") {
        return serde_json::json!({"ok":false,"code":"native_repair_substrate_not_implemented","error":"legacy-hash and sequence repair require the durable stream-integrity substrate"});
    }
    let (hub, lane, caller) = match coordination_context(args, "coordination_repair") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let Some(raw_paths) = args.get("paths").and_then(serde_json::Value::as_array) else {
        return json_error("coordination_repair stale-claims requires exact `paths`");
    };
    let paths = match raw_paths
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or("repair paths must be strings")
                .and_then(|path| {
                    ClaimPath::parse(path).map_err(|_| "repair path failed validation")
                })
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(paths) if !paths.is_empty() => paths,
        Ok(_) => return json_error("coordination_repair paths must not be empty"),
        Err(error) => return json_error(&error),
    };
    let owners = match args.get("owners").or_else(|| args.get("owner")) {
        None => None,
        Some(serde_json::Value::String(owner)) => match ClaimWriter::parse(owner.to_owned()) {
            Ok(owner) => Some(vec![owner]),
            Err(error) => return json_error(&error.to_string()),
        },
        Some(serde_json::Value::Array(values)) => match values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("repair owners must be strings")
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(owners) => match owners
                .into_iter()
                .map(ClaimWriter::parse)
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(owners) => Some(owners),
                Err(error) => return json_error(&error.to_string()),
            },
            Err(error) => return json_error(error),
        },
        Some(_) => return json_error("repair owner(s) must be strings"),
    };
    let mode = if args.get("write").and_then(serde_json::Value::as_bool) == Some(true) {
        enforcer_domain::coordination_types::RepairMode::Write
    } else {
        enforcer_domain::coordination_types::RepairMode::DryRun
    };
    let dry_run = matches!(
        mode,
        enforcer_domain::coordination_types::RepairMode::DryRun
    );
    match api::repair_stale_claims(&hub, &lane, &paths, owners.as_deref(), &caller, mode) {
        Ok((matched, event)) => {
            serde_json::json!({"ok":true,"action":"stale-claims","dryRun":dry_run,"matchedClaimCount":matched.get(),"event":event,"nextStep":if dry_run {"review exact matched claims then rerun with write:true"} else {"rerun coordination health"}})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

/// Persist and resolve native peer descriptors. Tokens are never persisted:
/// the registry may name an environment variable, which is resolved only at
/// the outbound transport boundary.
fn coordination_peer(args: &serde_json::Value) -> serde_json::Value {
    use enforcer_coordination::sync::peer::{self, PeerRecord};
    use enforcer_domain::coordination_types::{
        CoordinationLedgerRoot, CoordinationPeerName, CoordinationPeerTokenEnv, CoordinationPeerUrl,
    };
    let root = match args
        .get("root")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| CoordinationLedgerRoot::parse(std::path::Path::new(raw)).ok())
    {
        Some(value) => value,
        None => return json_error("coordination_peer requires a valid `root` ledger path"),
    };
    let registry_json = |registry: peer::PeerRegistry| serde_json::json!({"ok":true,"registry":{"peers":registry.peers.into_iter().map(|entry| serde_json::json!({"name":entry.name.as_str(),"url":entry.url.as_str(),"mode":"pull","tokenEnv":entry.token_env.as_ref().map(|value| value.as_str())})).collect::<Vec<_>>()}});
    match args
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("list")
    {
        "list" => match peer::load_registry(&root) {
            Ok(registry) => registry_json(registry),
            Err(error) => json_error(&error.to_string()),
        },
        "add" => {
            let parsed = (|| {
                let name = CoordinationPeerName::parse(
                    args.get("name")
                        .or_else(|| args.get("peer"))
                        .and_then(serde_json::Value::as_str)
                        .ok_or("coordination_peer add requires `name`")?
                        .to_owned(),
                )
                .map_err(|error| error.to_string())?;
                let url = CoordinationPeerUrl::parse(
                    args.get("url")
                        .or_else(|| args.get("peerUrl"))
                        .and_then(serde_json::Value::as_str)
                        .ok_or("coordination_peer add requires `url`")?
                        .to_owned(),
                )
                .map_err(|error| error.to_string())?;
                let token_env = args
                    .get("tokenEnv")
                    .and_then(serde_json::Value::as_str)
                    .map(|value| CoordinationPeerTokenEnv::parse(value.to_owned()))
                    .transpose()
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(PeerRecord {
                    name,
                    url,
                    token_env,
                })
            })();
            match parsed
                .and_then(|record| peer::add_peer(&root, record).map_err(|error| error.to_string()))
            {
                Ok(registry) => registry_json(registry),
                Err(error) => json_error(&error),
            }
        }
        "remove" => {
            let outcome: Result<peer::PeerRegistry, String> = (|| {
                let raw = args
                    .get("name")
                    .or_else(|| args.get("peer"))
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "coordination_peer remove requires `name`".to_owned())?;
                let name = CoordinationPeerName::parse(raw.to_owned())
                    .map_err(|error| error.to_string())?;
                peer::remove_peer(&root, &name).map_err(|error| error.to_string())
            })();
            match outcome {
                Ok(registry) => registry_json(registry),
                Err(error) => json_error(&error),
            }
        }
        unsupported => json_error(&format!(
            "unsupported native coordination peer action: {unsupported}"
        )),
    }
}

/// Import only an authenticated peer suffix. Divergence is preserved as a
/// conflict artifact; this operation never rewrites local append-only data.
fn coordination_sync(args: &serde_json::Value) -> serde_json::Value {
    use enforcer_coordination::sync::peer;
    use enforcer_domain::coordination_types::{
        CoordinationLedgerRoot, CoordinationPeerName, CoordinationPeerUrl,
    };
    let root = match args
        .get("root")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| CoordinationLedgerRoot::parse(std::path::Path::new(raw)).ok())
    {
        Some(value) => value,
        None => return json_error("coordination_sync requires a valid `root` ledger path"),
    };
    let peer_raw = match args
        .get("peer")
        .or_else(|| args.get("url"))
        .or_else(|| args.get("peerUrl"))
        .and_then(serde_json::Value::as_str)
    {
        Some(value) => value,
        None => return json_error("coordination_sync requires `peer`"),
    };
    let result = if std::path::Path::new(peer_raw).is_dir() {
        peer::sync_local(&root, std::path::Path::new(peer_raw))
    } else {
        let resolved = CoordinationPeerUrl::parse(peer_raw.to_owned())
            .map(|url| {
                (
                    url,
                    args.get("token")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                )
            })
            .or_else(|_| {
                let name = CoordinationPeerName::parse(peer_raw.to_owned())?;
                let record = peer::resolve_peer(&root, &name)?;
                Ok((record.url, peer::token_from_env(record.token_env.as_ref())?))
            });
        resolved.and_then(|(url, token)| peer::sync_http(&root, &url, token.as_deref()))
    };
    match result {
        Ok(sync) => {
            serde_json::json!({"ok":sync.conflicts.is_empty(),"peer":peer_raw,"result":{"imported":sync.imported,"transferredLines":sync.transferred_lines,"conflicts":sync.conflicts}})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_report(args: &serde_json::Value) -> serde_json::Value {
    let (hub, lane, caller) = match coordination_context(args, "coordination_report") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let Some(summary_raw) = args.get("summary").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_report requires `summary`");
    };
    let summary =
        match enforcer_domain::coordination_types::CoordinationReportSummary::parse(summary_raw) {
            Ok(value) => value,
            Err(error) => return json_error(&error.to_string()),
        };
    let title = match enforcer_domain::coordination_types::CoordinationReportTitle::parse(
        args.get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(summary_raw),
    ) {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    match api::report(&hub, &lane, title, summary, &caller) {
        Ok(event) => serde_json::json!({"ok":true,"event":event}),
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_index(args: &serde_json::Value) -> serde_json::Value {
    let Some(root) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_index requires a `root` ledger path");
    };
    match enforcer_coordination::ledger::materialize(std::path::Path::new(root)) {
        Ok(snapshot) => serde_json::json!({
            "ok":true,
            "indexKind":"derived-stream-replay",
            "digest":snapshot.digest,
            "eventCount":snapshot.events.len(),
            "activeClaims":snapshot.active_claims.len(),
            "reports":snapshot.reports,
            "workers":snapshot.workers,
            "tasks":snapshot.tasks,
            "inbox":snapshot.inbox,
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_notify(args: &serde_json::Value) -> serde_json::Value {
    let (hub, lane, _caller) = match coordination_context(args, "coordination_notify") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let state_file = args
        .get("stateFile")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from);
    let peek = args
        .get("peek")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    match api::notify(
        &hub,
        &lane,
        enforcer_coordination::api::boundary::NotifyRequest { peek, state_file },
    ) {
        Ok(result) => {
            serde_json::json!({"ok":true,"targetLane":result.target_lane,"wakeRequests":result.wake_requests,"peek":result.peek})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

/// Compact a previously initialized ledger.  Unlike initialization and claim
/// paths, compaction must never create authority as a side effect: its root,
/// hub identity, and retention count are all checked before any stream write.
fn coordination_compact(args: &serde_json::Value) -> serde_json::Value {
    let root_raw = match args.get("root").and_then(serde_json::Value::as_str) {
        Some(value) => value,
        None => return json_error("coordination_compact requires a `root` ledger path"),
    };
    let requested_hub = match args.get("hub").and_then(serde_json::Value::as_str) {
        Some(value) => match value.parse::<HubName>() {
            Ok(hub) => hub,
            Err(error) => return json_error(&error.to_string()),
        },
        None => return json_error("coordination_compact requires a `hub` name"),
    };
    let lane_raw = match args.get("lane").and_then(serde_json::Value::as_str) {
        Some(value) => value,
        None => return json_error("coordination_compact requires a `lane` id"),
    };
    if let Err(error) = lane_raw.parse::<LaneId>() {
        return json_error(&error.to_string());
    }
    let root = match CoordinationLedgerRoot::parse(std::path::Path::new(root_raw)) {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    let keep_latest = match compact_keep_latest(args) {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let hub = match api::open(root) {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    if hub.config.hub != requested_hub {
        return serde_json::json!({
            "ok": false,
            "code": "coordination_hub_mismatch",
            "error": "coordination_compact hub does not match the initialized ledger authority",
        });
    }
    match api::compact(&hub, keep_latest) {
        Ok(result) => serde_json::json!({
            "ok": true,
            "hub": hub.config.hub.as_str(),
            "keepLatest": keep_latest.value().get(),
            "compactedStreams": result.compacted_streams.into_iter().map(|stream| serde_json::json!({
                "stream": stream.stream.as_str(),
                "archivedEvents": stream.archived_events.value(),
                "retainedEvents": stream.retained_events.value(),
                "archivePath": stream.archive_path.as_path(),
            })).collect::<Vec<_>>(),
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

fn compact_keep_latest(
    args: &serde_json::Value,
) -> Result<enforcer_domain::coordination_types::CompactionKeepCount, String> {
    let raw = args.get("keepLatest").map_or(Ok(250_usize), |value| {
        value
            .as_u64()
            .ok_or_else(|| "coordination_compact keepLatest must be a positive integer".to_owned())
            .and_then(|value| {
                usize::try_from(value)
                    .map_err(|_error| "coordination_compact keepLatest is too large".to_owned())
            })
    })?;
    let positive = std::num::NonZeroUsize::new(raw)
        .ok_or_else(|| "coordination_compact keepLatest must be a positive integer".to_owned())?;
    Ok(enforcer_domain::coordination_types::CompactionKeepCount::new(positive))
}

fn coordination_event_rows(root: &std::path::Path, kind: &str, field: &str) -> serde_json::Value {
    match enforcer_coordination::sync::stream::read_all_streams(root) {
        Ok(all) => {
            let rows: Vec<_> = all
                .events
                .into_iter()
                .filter(|event| event.kind == kind)
                .collect();
            serde_json::json!({"ok":true, field:rows})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_init(args: &serde_json::Value) -> serde_json::Value {
    let Some(root) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_init requires a `root` ledger path");
    };
    let Some(hub_raw) = args.get("hub").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_init requires a `hub` name");
    };
    let Some(lane_raw) = args.get("lane").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_init requires a `lane` id");
    };
    let (Ok(hub), Ok(lane)) = (hub_raw.parse::<HubName>(), lane_raw.parse::<LaneId>()) else {
        return json_error("hub/lane failed enforcer-domain brand validation");
    };
    match api::init(std::path::Path::new(root), &hub, &lane) {
        Ok(config) => {
            serde_json::json!({"ok":true,"hub":config.hub.as_str(),"defaultLane":config.default_lane.as_str(),"nodeId":config.node_id.as_str()})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_context(
    args: &serde_json::Value,
    operation: &str,
) -> Result<(Hub, LaneId, CallerContext), String> {
    let root = args
        .get("root")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{operation} requires a `root` ledger path"))?;
    let hub_raw = args
        .get("hub")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{operation} requires a `hub` name"))?;
    let lane_raw = args
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{operation} requires a `lane` id"))?;
    let hub_name = hub_raw
        .parse::<HubName>()
        .map_err(|error| error.to_string())?;
    let lane = lane_raw
        .parse::<LaneId>()
        .map_err(|error| error.to_string())?;
    let root_path = std::path::Path::new(root);
    let ledger_root =
        CoordinationLedgerRoot::parse(root_path).map_err(|error| error.to_string())?;
    let config = api::init(root_path, &hub_name, &lane).map_err(|error| error.to_string())?;
    let worktree_raw = args
        .get("worktreeRoot")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(root);
    let worktree_root =
        CoordinationWorktree::parse(worktree_raw).map_err(|error| error.to_string())?;
    let branch = CoordinationBranch::parse(
        args.get("branch")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
    )
    .map_err(|error| error.to_string())?;
    let project_id = CoordinationProjectId::parse(
        args.get("projectId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
    )
    .map_err(|error| error.to_string())?;
    let commit = args
        .get("commit")
        .and_then(serde_json::Value::as_str)
        .map(str::parse::<CommitRef>)
        .transpose()
        .map_err(|error| error.to_string())?;
    Ok((
        Hub {
            root: ledger_root,
            config,
        },
        lane,
        CallerContext {
            project_id,
            worktree_root,
            branch,
            commit,
            codex_thread_id: None,
            codex_session_id: None,
        },
    ))
}

fn coordination_release(args: &serde_json::Value) -> serde_json::Value {
    let (hub, lane, caller) = match coordination_context(args, "coordination_release") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let Some(raw_paths) = args.get("paths").and_then(serde_json::Value::as_array) else {
        return json_error("coordination_release requires a `paths` array");
    };
    let paths = match raw_paths
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or("release paths must contain strings")
                .and_then(|value| {
                    ClaimPath::parse(value).map_err(|_error| "release path failed validation")
                })
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(value) => value,
        Err(error) => return json_error(error),
    };
    let reason = match args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(ClaimReason::parse)
        .transpose()
    {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    match api::release(&hub, &lane, &paths, &caller, reason.as_ref()) {
        Ok(event) => serde_json::json!({"ok":true,"event":event}),
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_closeout(args: &serde_json::Value) -> serde_json::Value {
    let (hub, lane, caller) = match coordination_context(args, "coordination_closeout") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let mut filters = api::CloseoutFilters {
        lane: Some(lane.clone()),
        ..Default::default()
    };
    if args.get("allLanes").and_then(serde_json::Value::as_bool) == Some(true) {
        filters.lane_scope = enforcer_domain::coordination_types::CloseoutLaneScope::AllLanes;
    }
    match api::closeout(&hub, &lane, &filters, &caller, None) {
        Ok(events) => {
            serde_json::json!({"ok":true,"releasedEventCount":events.len(),"events":events})
        }
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_message(args: &serde_json::Value) -> serde_json::Value {
    let (hub, lane, caller) = match coordination_context(args, "coordination_message") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    let Some(to) = args.get("to").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_message requires a recipient `to`");
    };
    let Some(body) = args
        .get("body")
        .or_else(|| args.get("message"))
        .and_then(serde_json::Value::as_str)
    else {
        return json_error("coordination_message requires `body`");
    };
    let recipient = match to.parse::<LaneId>() {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    let body = match enforcer_domain::coordination_types::CoordinationMessageBody::parse(body) {
        Ok(value) => value,
        Err(error) => return json_error(&error.to_string()),
    };
    match api::send_message(&hub, &lane, recipient, body, &caller) {
        Ok(event) => serde_json::json!({"ok":true,"event":event}),
        Err(error) => json_error(&error.to_string()),
    }
}

fn coordination_mail(args: &serde_json::Value) -> serde_json::Value {
    match args
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("inbox")
    {
        "inbox" => coordination("ocentra_enforcer_coordination_inbox", args),
        "send" => coordination_message(args),
        "ack" => {
            let (hub, lane, caller) = match coordination_context(args, "coordination_mail") {
                Ok(value) => value,
                Err(error) => return json_error(&error),
            };
            let Some(id) = args.get("messageId").and_then(serde_json::Value::as_str) else {
                return json_error("coordination_mail ack requires `messageId`");
            };
            let id = match enforcer_domain::coordination_types::ClaimEventId::parse(id.to_owned()) {
                Ok(value) => value,
                Err(error) => return json_error(&error.to_string()),
            };
            match api::acknowledge_message(&hub, &lane, id, &caller) {
                Ok(event) => serde_json::json!({"ok":true,"event":event}),
                Err(error) => json_error(&error.to_string()),
            }
        }
        other => {
            serde_json::json!({"ok":false,"operation":"ocentra_enforcer_coordination_mail","code":"unsupported_mail_action","error":format!("unsupported native mail action: {other}")})
        }
    }
}

fn coordination_guard(args: &serde_json::Value) -> serde_json::Value {
    let Some(root) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("coordination_guard requires a `root` ledger path");
    };
    let Some(paths) = args
        .get("paths")
        .or_else(|| args.get("changedPaths"))
        .and_then(serde_json::Value::as_array)
    else {
        return json_error("coordination_guard requires `paths`");
    };
    let requested: Vec<_> = paths.iter().filter_map(serde_json::Value::as_str).collect();
    if requested.len() != paths.len() || requested.is_empty() {
        return json_error("coordination_guard paths must be non-empty strings");
    }
    match enforcer_coordination::sync::stream::read_all_streams(std::path::Path::new(root)) {
        Ok(all) => {
            let active = enforcer_coordination::ledger::active_claims(&all.events);
            let lane = args.get("lane").and_then(serde_json::Value::as_str);
            let blockers: Vec<_> = active
                .into_iter()
                .filter(|claim| {
                    lane != Some(claim.lane.as_str())
                        && claim
                            .paths
                            .iter()
                            .any(|owned| requested.iter().any(|path| owned.as_str() == *path))
                })
                .collect();
            serde_json::json!({"ok":blockers.is_empty(),"allowed":blockers.is_empty(),"blockerCount":blockers.len(),"blockers":blockers.iter().map(|claim| serde_json::json!({"lane":claim.lane.as_str(),"paths":claim.paths.iter().map(ClaimPath::as_str).collect::<Vec<_>>() })).collect::<Vec<_>>()})
        }
        Err(error) => json_error(&error.to_string()),
    }
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
    let owns = match paths
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or("coordination_claim paths must contain only strings")
                .and_then(|raw| {
                    ClaimPath::parse(raw).map_err(|_error| "claim path failed validation")
                })
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(values) => values,
        Err(message) => return json_error(message),
    };

    let (Ok(hub_name), Ok(lane_id)) = (hub_raw.parse::<HubName>(), lane_raw.parse::<LaneId>())
    else {
        return json_error("hub/lane failed enforcer-domain brand validation");
    };

    // L2: caller identity is a REQUIRED param the wire caller must supply
    // — this MCP layer never resolves it server-side (see
    // `enforcer_coordination::api::CallerContext` doc). Minimal fields
    // wired here; richer caller context is a sibling wiring pass.
    let worktree_root = match CoordinationWorktree::parse(
        args.get("worktreeRoot")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(root),
    ) {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let branch = match CoordinationBranch::parse(
        args.get("branch")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
    ) {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let project_id = match CoordinationProjectId::parse(
        args.get("projectId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
    ) {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let commit = match args.get("commit").and_then(serde_json::Value::as_str) {
        Some(raw) => match raw.parse::<CommitRef>() {
            Ok(value) => Some(value),
            Err(err) => return json_error(&err.to_string()),
        },
        None => None,
    };
    let caller = CallerContext {
        project_id,
        worktree_root,
        branch,
        commit,
        codex_thread_id: None,
        codex_session_id: None,
    };
    let reason = match args.get("reason").and_then(serde_json::Value::as_str) {
        Some(raw) => match ClaimReason::parse(raw) {
            Ok(value) => Some(value),
            Err(err) => return json_error(&err.to_string()),
        },
        None => None,
    };

    let root_path = std::path::Path::new(root);
    let ledger_root = match CoordinationLedgerRoot::parse(root_path) {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let repo_root = match CoordinationRepoRoot::parse(root_path) {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let hub_config = match api::init(root_path, &hub_name, &lane_id) {
        Ok(config) => config,
        Err(err) => return json_error(&err.to_string()),
    };
    let hub = Hub {
        root: ledger_root,
        config: hub_config,
    };
    let outcome = api::claim_all(
        &hub,
        ClaimRequestArgs {
            repo_root: &repo_root,
            lane: &lane_id,
            owns: &owns,
            caller: &caller,
            reason: reason.as_ref(),
        },
    );
    match outcome {
        Ok(result) => serde_json::json!({
            "ok": matches!(result.status, ClaimOutcomeStatus::Accepted),
            "eventCount": result.events.len(),
            "blockerCount": result.blockers.len(),
        }),
        Err(err) => json_error(&err.to_string()),
    }
}

/// `ocentra_enforcer_ui` — never write-gated (read-only report). Delegates
/// entirely to `enforcer_ui::serve::ui_tool_response`, which performs no
/// I/O and never binds a socket -- silent-agent-safe by construction (see
/// that function's doc comment on the f04 gate not having landed yet).
/// This handler's only job is JSON<->typed decoding, matching every other
/// handler's transport-adapter charter.
fn ui_tool(args: &serde_json::Value) -> serde_json::Value {
    let host = args
        .get("host")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("127.0.0.1")
        .to_owned();
    let port = args
        .get("port")
        .and_then(serde_json::Value::as_u64)
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(0);
    let token = args
        .get("token")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    match enforcer_ui::serve::BindOptions::try_new(host, port, token) {
        Ok(request) => enforcer_ui::serve::ui_tool_response(&request),
        Err(err) => json_error(&err.to_string()),
    }
}

fn json_error(message: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": message })
}

#[cfg(test)]
mod tests {
    use super::{dispatch, DispatchContext, DispatchOutcome};
    use enforcer_domain::mcp_types::McpToolName;
    use enforcer_domain::mcp_types::{ArtifactPath, McpFreshness};

    fn tool(
        value: &str,
    ) -> Result<McpToolName, enforcer_domain::boundary::decode_error::DecodeError> {
        McpToolName::try_new(value)
    }

    fn ctx(freshness: McpFreshness) -> DispatchContext {
        DispatchContext {
            freshness,
            cli_path: ArtifactPath::from_path(std::path::Path::new("/abs/enforcer")),
            validation_history: std::sync::Arc::new(std::sync::Mutex::new(
                crate::validation_history::ValidationHistory::default(),
            )),
        }
    }

    #[test]
    fn proof_status_returns_a_bounded_filtered_run_collection_not_a_snapshot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_string_lossy();
        for (run_id, proof_id) in [("status-a", "status.alpha"), ("status-b", "status.beta")] {
            let outcome = dispatch(
                &tool("ocentra_enforcer_proof_run")?,
                &serde_json::json!({
                    "root":root,"proofId":proof_id,"runId":run_id,
                    "command": if cfg!(windows) { serde_json::json!(["cmd","/C","exit 0"]) } else { serde_json::json!(["true"]) }
                }),
                &ctx(McpFreshness::Fresh),
            );
            let DispatchOutcome::Result(value) = outcome else {
                return Err("proof run did not produce a native result".into());
            };
            assert_eq!(value["ok"], serde_json::json!(true));
        }
        let outcome = dispatch(
            &tool("ocentra_enforcer_proof_status")?,
            &serde_json::json!({"root":root,"proofId":"status.alpha","status":"passed","limit":1}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("proof tool did not produce a native result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert!(value.get("snapshot").is_none());
        assert_eq!(value["runs"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            value["runs"][0]["proofId"],
            serde_json::json!("status.alpha")
        );
        let invalid = dispatch(
            &tool("ocentra_enforcer_proof_status")?,
            &serde_json::json!({"root":root,"limit":1.5}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = invalid else {
            return Err("invalid proof status did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        Ok(())
    }

    #[test]
    fn proof_route_uses_the_native_pack_not_the_target_repository_root(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_proof_route")?,
            &serde_json::json!({
                "root":temp.path().to_string_lossy(),"profile":"strict","scope":"workspace"
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("proof route did not produce a native result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["profileName"], serde_json::json!("strict"));
        assert!(value["proofs"].is_array());
        assert!(value.get("snapshot").is_none());

        let invalid = dispatch(
            &tool("ocentra_enforcer_proof_route")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"profile":"../escape"}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = invalid else {
            return Err("invalid proof route did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        Ok(())
    }

    #[test]
    fn proof_inventory_is_repo_contained_and_only_returns_opted_in_bounded_rows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let scripts = temp.path().join("scripts/test");
        std::fs::create_dir_all(&scripts)?;
        std::fs::write(
            scripts.join("alpha-proof.mjs"),
            "spawn('x'); writeFile('proof.md', 'x');",
        )?;
        std::fs::write(scripts.join("beta.mjs"), "await import('dist/x');")?;
        let absent_rows = dispatch(
            &tool("ocentra_enforcer_proof_inventory")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"limit":1}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = absent_rows else {
            return Err("proof inventory did not produce a result".into());
        };
        assert_eq!(value["totals"]["scripts"], serde_json::json!(2));
        assert_eq!(value["scripts"].as_array().map(Vec::len), Some(0));
        assert_eq!(value["omittedScriptCount"], serde_json::json!(2));

        let rows = dispatch(
            &tool("ocentra_enforcer_proof_inventory")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"includeScripts":true,"limit":1}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = rows else {
            return Err("bounded proof inventory did not produce a result".into());
        };
        assert_eq!(value["scripts"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            value["scripts"][0]["path"],
            serde_json::json!("scripts/test/alpha-proof.mjs")
        );
        assert_eq!(
            value["scripts"][0]["signals"]["spawn"],
            serde_json::json!(true)
        );
        Ok(())
    }

    #[test]
    fn pass_fixture_canned_request_yields_expected_tool_result(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_mcp_status")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("mcp_status did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert!(value["toolCount"].as_u64().is_some_and(|count| count > 0));
        Ok(())
    }

    #[test]
    fn doctor_dispatches_to_the_native_repository_readiness_engine(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(
            temp.path().join("src/lib.rs"),
            "pub fn doctor_fixture() {}\n",
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_doctor")?,
            &serde_json::json!({ "root": temp.path().to_string_lossy(), "files": ["src/lib.rs"] }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("doctor did not produce a result".into());
        };
        assert_eq!(value["command"], serde_json::json!("doctor"));
        assert!(value["checks"]
            .as_array()
            .is_some_and(|checks| checks.len() == 6));
        Ok(())
    }

    #[test]
    fn scan_dispatches_to_the_native_rust_engine() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(
            src.join("lib.rs"),
            "mod inner { pub struct Thing; }\npub use inner::Thing;\n",
        )?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_scan")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "files": ["src/lib.rs"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native scan did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["findings"].as_array().is_some_and(|findings| {
            findings
                .iter()
                .any(|finding| finding["ruleId"] == "T1-NOREEXPORT.1")
        }));
        Ok(())
    }

    #[test]
    fn scan_rejects_unsupported_schema_fields_instead_of_ignoring_them(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_scan")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "profile": "strict",
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native scan did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["error"]
            .as_str()
            .is_some_and(|message| message.contains("does not support `profile`")));
        Ok(())
    }

    #[test]
    fn scan_language_filter_reaches_the_native_typed_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(
            src.join("lib.rs"),
            "mod inner { pub struct Thing; }\npub use inner::Thing;\n",
        )?;
        std::fs::write(src.join("app.ts"), "export const value = true;\n")?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_scan")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "scope": "files",
                "files": ["src/lib.rs", "src/app.ts"],
                "languages": ["typescript"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native scan did not produce a result".into());
        };
        assert!(value["findings"].as_array().is_some_and(|findings| {
            findings
                .iter()
                .all(|finding| finding["ruleId"] != "T1-NOREEXPORT.1")
        }));
        Ok(())
    }

    #[test]
    fn check_no_zod_source_filters_to_its_native_rule() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(
            src.join("schema.ts"),
            "import { z } from \"zod\";\nexport const value = z.string();\n",
        )?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "no-zod-source",
                "scope": "files",
                "files": ["src/schema.ts"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native check did not produce a result".into());
        };
        assert_eq!(value["command"], serde_json::json!("check"));
        assert_eq!(value["check"], serde_json::json!("no-zod-source"));
        assert_eq!(value["ok"], serde_json::json!(false));
        let findings = value["findings"].as_array().ok_or("missing findings")?;
        assert!(!findings.is_empty());
        assert!(findings.iter().all(|finding| finding["ruleId"] == "TS-1.2"));
        Ok(())
    }

    #[test]
    fn check_reexports_filters_a_real_native_scan_to_its_declared_family(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(
            src.join("barrel.ts"),
            "export { value } from \"./value\";\n",
        )?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "reexports",
                "scope": "files",
                "files": ["src/barrel.ts"],
                "languages": ["typescript"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native check did not produce a result".into());
        };
        assert_eq!(value["check"], serde_json::json!("reexports"));
        assert_eq!(value["ok"], serde_json::json!(false));
        let findings = value["findings"].as_array().ok_or("missing findings")?;
        assert!(!findings.is_empty());
        assert!(findings.iter().all(|finding| finding["ruleId"] == "TS-1.1"));
        Ok(())
    }

    #[test]
    fn naked_domain_string_alias_uses_native_rust_policy_and_skips_generated_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src/generated"))?;
        std::fs::write(
            temp.path().join("src/generated/value.rs"),
            "pub fn raw(value: String) { }",
        )?;
        std::fs::write(
            temp.path().join("ocentra-enforcer.config.json"),
            r#"{"schemaVersion":2,"profileName":"default"}"#,
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"rust-string-boundaries","scope":"files","files":["src/generated/value.rs"],"languages":["rust"]}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native alias did not dispatch".into());
        };
        assert_eq!(value["check"], serde_json::json!("rust-string-boundaries"));
        assert_eq!(value["ok"], serde_json::json!(true));
        Ok(())
    }

    #[test]
    fn naked_domain_string_policy_refuses_unsupported_language_requests(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(
            temp.path().join("src/value.ts"),
            "export const value = 'raw';",
        )?;
        std::fs::write(
            temp.path().join("ocentra-enforcer.config.json"),
            r#"{"schemaVersion":2,"profileName":"default"}"#,
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"no-naked-domain-strings","scope":"files","files":["src/value.ts"],"languages":["typescript"]}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native policy did not dispatch".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["error"]
            .as_str()
            .is_some_and(|error| error.contains("only rust")));
        Ok(())
    }

    #[test]
    fn check_secrets_uses_the_dedicated_sec_policy_engine() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(
            src.join("config.rs"),
            "const SECRET = \"0123456789abcdefghijklmnop\";\n",
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "secrets",
                "scope": "files",
                "files": ["src/config.rs"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native secrets check did not produce a result".into());
        };
        assert_eq!(value["check"], serde_json::json!("secrets"));
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["findings"].as_array().is_some_and(|findings| {
            findings.iter().all(|finding| {
                finding["ruleId"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("SEC-"))
            })
        }));
        Ok(())
    }

    #[test]
    fn check_dependency_policy_uses_the_dedicated_workspace_engine(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("crates/app"))?;
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )?;
        std::fs::write(
            temp.path().join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\noutside = { path = \"../../outside\" }\n",
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "dependency-policy",
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native dependency policy did not produce a result".into());
        };
        assert_eq!(value["check"], serde_json::json!("dependency-policy"));
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().all(|finding| finding["ruleId"] == "RR-9.3")));
        Ok(())
    }

    #[test]
    fn check_sbom_generates_a_lockfile_bound_native_artifact(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let output = tempfile::tempdir()?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": workspace.to_string_lossy(),
                "check": "sbom",
                "output": output.path().to_string_lossy(),
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native sbom check did not produce a result".into());
        };
        assert_eq!(value["check"], serde_json::json!("sbom"));
        assert_eq!(value["ok"], serde_json::json!(true));
        let artifact = value["artifact"].as_str().ok_or("missing sbom artifact")?;
        assert!(std::path::Path::new(artifact).is_file());
        Ok(())
    }

    #[test]
    fn run_status_returns_process_local_validation_after_final_check_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("schema.ts"), "import { z } from \"zod\";\n")?;
        let context = ctx(McpFreshness::Fresh);
        let check = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({ "root": temp.path().to_string_lossy(), "check": "no-zod-source", "files": ["src/schema.ts"] }),
            &context,
        );
        assert!(matches!(check, DispatchOutcome::Result(_)));
        let status = dispatch(
            &tool("ocentra_enforcer_run_status")?,
            &serde_json::json!({ "root": temp.path().to_string_lossy(), "tool": "check" }),
            &context,
        );
        let DispatchOutcome::Result(value) = status else {
            return Err("run status did not produce a result".into());
        };
        assert_eq!(value["summaryType"], serde_json::json!("validation"));
        assert_eq!(value["summary"]["kind"], serde_json::json!("check"));
        assert_eq!(
            value["summary"]["check"],
            serde_json::json!("no-zod-source")
        );
        assert_eq!(value["summary"]["ruleIds"], serde_json::json!(["TS-1.2"]));
        assert!(value.get("artifact").is_none());
        Ok(())
    }

    #[test]
    fn route_dispatches_to_the_native_rust_route_engine() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("lib.rs"), "pub struct RouteFixture;\n")?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_route")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "scope": "files",
                "files": ["src/lib.rs"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native route did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["scope"]["kind"], serde_json::json!("repo"));
        assert_eq!(value["languages"], serde_json::json!(["rust"]));
        assert!(value["rulePacks"]
            .as_array()
            .is_some_and(|packs| packs.iter().any(|pack| pack == "rust")));
        Ok(())
    }

    #[test]
    fn test_doctrine_scan_dispatches_to_the_native_rust_analyzer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("tests"))?;
        std::fs::write(
            temp.path().join("package.json"),
            r#"{ "dependencies": { "express": "1" } }"#,
        )?;
        std::fs::write(
            temp.path().join("tests/unit.test.ts"),
            "it('works', () => {});",
        )?;

        let outcome = dispatch(
            &tool("ocentra_enforcer_test_doctrine_scan")?,
            &serde_json::json!({ "root": temp.path().to_string_lossy() }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native test-doctrine scan did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["nature"]["isWebApi"], serde_json::json!(true));
        assert_eq!(
            value["detected"]["unit"]["present"],
            serde_json::json!(true)
        );
        assert!(value["missing"]
            .as_array()
            .is_some_and(|missing| missing.iter().any(|item| item["category"] == "security")));
        Ok(())
    }

    #[test]
    fn check_source_shape_executes_the_config_driven_native_engine(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(temp.path().join("src/lib.rs"), "fn one() {}\nfn two() {}\n")?;
        std::fs::write(
            temp.path().join("source.json"),
            r#"{"schemaVersion":2,"profileName":"default","sourceShapePolicies":[{"roots":["src"],"extensions":[".rs"],"kind":"rust","maxFunctions":1}]}"#,
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "source-shape",
                "configPath": "source.json",
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native check did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert_eq!(value["check"], serde_json::json!("source-shape"));
        assert!(value["error"].is_null() || value.get("error").is_none());
        assert!(value["violations"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        Ok(())
    }

    #[test]
    fn check_source_shape_masks_typescript_template_data_through_the_named_route(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(
            temp.path().join("src/view.ts"),
            "const sample = `\n ${value ? `{ data: true }` : `{ data: false }`}\n`;\nexport const run = () => { return 1; };\n",
        )?;
        std::fs::write(
            temp.path().join("source.json"),
            r#"{"schemaVersion":2,"profileName":"default","sourceShapePolicies":[{"roots":["src"],"extensions":[".ts"],"kind":"typescript","maxNestingDepth":1}]}"#,
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"source-shape","configPath":"source.json","scope":"files","files":["src/view.ts"],"languages":["typescript"]}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native source-shape route did not dispatch".into());
        };
        assert_eq!(value["check"], serde_json::json!("source-shape"));
        assert_eq!(value["ok"], serde_json::json!(true));
        Ok(())
    }

    #[test]
    fn check_architecture_policy_aggregates_configured_native_members(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("architecture.json"),
            r#"{"schemaVersion":2,"profileName":"default","architecturePolicyChecks":["generated-artifacts","missing-native-check"]}"#,
        )?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "architecture-policy",
                "configPath": "architecture.json",
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native check did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert_eq!(value["check"], serde_json::json!("architecture-policy"));
        assert_eq!(value["checks"].as_array().map(Vec::len), Some(2));
        assert!(value["findings"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["ruleId"] == "ARCH-1.10") }));
        Ok(())
    }

    #[test]
    fn harness_query_rejects_non_integral_wire_limits() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_diagnostics")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "limit": 1.5,
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native diagnostics query did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["error"]
            .as_str()
            .is_some_and(|message| message.contains("non-negative integer")));
        Ok(())
    }

    #[test]
    fn run_dispatches_through_typed_boundary_and_native_harness_engine(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_string_lossy();
        let outcome = dispatch(
            &tool("ocentra_enforcer_run")?,
            &serde_json::json!({
                "root": root,
                "runId": "mcp-native-run",
                "tool": "rustc",
                "language": "rust",
                "command": ["rustc", "--version"],
                "tags": ["mcp-e2e"],
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native run did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(
            value["summary"]["runId"],
            serde_json::json!("mcp-native-run")
        );

        let status = dispatch(
            &tool("ocentra_enforcer_run_status")?,
            &serde_json::json!({"root": root, "runId": "mcp-native-run"}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(status) = status else {
            return Err("native run status did not produce a result".into());
        };
        assert_eq!(status["ok"], serde_json::json!(true));
        assert_eq!(
            status["summary"]["runId"],
            serde_json::json!("mcp-native-run")
        );
        Ok(())
    }

    #[test]
    fn fail_fixture_malformed_request_unknown_tool_is_rejected(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("not_a_real_tool")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Fresh),
        );
        assert!(matches!(outcome, DispatchOutcome::UnknownTool));
        Ok(())
    }

    #[test]
    fn legacy_alias_resolves_to_the_same_handler_as_canonical(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = dispatch(
            &tool("ocentra_enforcer_mcp_status")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Fresh),
        );
        let aliased = dispatch(
            &tool("rust_rules_mcp_status")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Fresh),
        );
        let (DispatchOutcome::Result(a), DispatchOutcome::Result(b)) = (canonical, aliased) else {
            return Err("canonical and alias calls did not both produce results".into());
        };
        assert_eq!(a, b);
        Ok(())
    }

    #[test]
    fn stale_server_refuses_a_write_tool_via_the_router() -> Result<(), Box<dyn std::error::Error>>
    {
        let outcome = dispatch(
            &tool("ocentra_enforcer_coordination_claim")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Stale),
        );
        assert!(matches!(outcome, DispatchOutcome::StaleRefused(_)));
        Ok(())
    }

    #[test]
    fn read_only_tool_still_dispatches_while_stale() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_mcp_status")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Stale),
        );
        assert!(matches!(outcome, DispatchOutcome::Result(_)));
        Ok(())
    }

    #[test]
    fn explain_reads_the_native_rule_catalog_and_never_needs_mjs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_explain")?,
            &serde_json::json!({"ruleId":"RR-6.1"}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("explain did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["ruleId"], serde_json::json!("RR-6.1"));
        assert_eq!(
            value["source"],
            serde_json::json!("native-rust-rule-catalog")
        );
        assert!(value["fixHint"]
            .as_str()
            .is_some_and(|hint| !hint.is_empty()));
        Ok(())
    }

    #[test]
    fn explain_rejects_unknown_or_malformed_rule_ids() -> Result<(), Box<dyn std::error::Error>> {
        let unknown = dispatch(
            &tool("ocentra_enforcer_explain")?,
            &serde_json::json!({"ruleId":"RR-999.999"}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(unknown) = unknown else {
            return Err("unknown explain did not produce a result".into());
        };
        assert_eq!(unknown["kind"], serde_json::json!("rule_not_found"));
        let malformed = dispatch(
            &tool("ocentra_enforcer_explain")?,
            &serde_json::json!({"ruleId":"not a rule"}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(malformed) = malformed else {
            return Err("malformed explain did not produce a result".into());
        };
        assert_eq!(malformed["kind"], serde_json::json!("invalid_rule_id"));
        Ok(())
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
            &tool("ocentra_enforcer_coordination_claim")?,
            &args,
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("coordination claim did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["eventCount"], serde_json::json!(1));
        Ok(())
    }

    /// `ocentra_enforcer_ui` on the loopback default: reports the served
    /// URL, never launches -- proving the g01 workpack's "MCP `ui` tool
    /// returns the served URL, never auto-launches during silent agent
    /// runs" requirement at the router boundary (not just in
    /// `enforcer-ui`'s own unit tests).
    #[test]
    fn ui_tool_loopback_default_reports_url_without_launching(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_ui")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("ui tool did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["launched"], serde_json::json!(false));
        Ok(())
    }

    /// `ocentra_enforcer_ui` with a non-loopback host and no token:
    /// reports the fail-closed refusal as DATA (never panics, never
    /// binds), still `launched: false`.
    #[test]
    fn ui_tool_remote_without_token_reports_refusal_without_launching(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_ui")?,
            &serde_json::json!({ "host": "0.0.0.0" }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("ui tool did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert_eq!(value["launched"], serde_json::json!(false));
        Ok(())
    }

    /// `ocentra_enforcer_ui` is read-only/never write-gated: it still
    /// dispatches while the server is stale.
    #[test]
    fn ui_tool_still_dispatches_while_stale() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = dispatch(
            &tool("ocentra_enforcer_ui")?,
            &serde_json::json!({}),
            &ctx(McpFreshness::Stale),
        );
        assert!(matches!(outcome, DispatchOutcome::Result(_)));
        Ok(())
    }

    /// Full native coordination lifecycle against an isolated ledger: no
    /// process-global default root may leak into MCP coordination calls.
    #[test]
    fn coordination_tools_use_the_requested_temp_ledger_and_preserve_write_safety(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ledger = tempfile::tempdir()?;
        let root = ledger.path().to_string_lossy().to_string();
        let common = serde_json::json!({
            "root": root, "hub": "mcp-e2e", "lane": "lane-a",
            "worktreeRoot": "E:/mcp-e2e", "branch": "rust-build", "projectId": "mcp-e2e"
        });
        let mut claim_args = common.clone();
        claim_args["paths"] = serde_json::json!(["crates/example/src/lib.rs"]);
        let DispatchOutcome::Result(claim) = dispatch(
            &tool("ocentra_enforcer_coordination_claim")?,
            &claim_args,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("claim was not dispatched".into());
        };
        assert_eq!(claim["ok"], serde_json::json!(true));

        let mut guard_args = common.clone();
        guard_args["paths"] = serde_json::json!(["crates/example/src/lib.rs"]);
        let DispatchOutcome::Result(guard) = dispatch(
            &tool("ocentra_enforcer_coordination_guard")?,
            &guard_args,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("guard was not dispatched".into());
        };
        assert_eq!(guard["allowed"], serde_json::json!(true));

        let mut conflicting_guard = guard_args.clone();
        conflicting_guard["lane"] = serde_json::json!("lane-b");
        let DispatchOutcome::Result(blocked) = dispatch(
            &tool("ocentra_enforcer_coordination_guard")?,
            &conflicting_guard,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("conflicting guard was not dispatched".into());
        };
        assert_eq!(blocked["allowed"], serde_json::json!(false));

        let mut release_args = common.clone();
        release_args["paths"] = serde_json::json!(["crates/example/src/lib.rs"]);
        let DispatchOutcome::Result(release) = dispatch(
            &tool("ocentra_enforcer_coordination_release")?,
            &release_args,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("release was not dispatched".into());
        };
        assert_eq!(release["ok"], serde_json::json!(true));

        let DispatchOutcome::Result(second_claim) = dispatch(
            &tool("ocentra_enforcer_coordination_claim")?,
            &claim_args,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("second claim was not dispatched".into());
        };
        assert_eq!(second_claim["ok"], serde_json::json!(true));
        let DispatchOutcome::Result(closeout) = dispatch(
            &tool("ocentra_enforcer_coordination_closeout")?,
            &common,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("closeout was not dispatched".into());
        };
        assert_eq!(closeout["ok"], serde_json::json!(true));

        assert!(matches!(
            dispatch(
                &tool("ocentra_enforcer_coordination_claim")?,
                &claim_args,
                &ctx(McpFreshness::Stale),
            ),
            DispatchOutcome::StaleRefused(_)
        ));
        let peer_root = tempfile::tempdir()?;
        std::fs::create_dir_all(peer_root.path().join("streams"))?;
        std::fs::write(
            peer_root
                .path()
                .join("streams")
                .join("node_peer.lane-a.ndjson"),
            "{\"id\":\"remote-evidence\"}\n",
        )?;
        let mut sync_args = common.clone();
        sync_args["peer"] = serde_json::json!(peer_root.path());
        let DispatchOutcome::Result(synced) = dispatch(
            &tool("ocentra_enforcer_coordination_sync")?,
            &sync_args,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("native sync was not dispatched".into());
        };
        assert_eq!(synced["ok"], serde_json::json!(true));
        assert_eq!(synced["result"]["imported"], serde_json::json!(1));
        Ok(())
    }

    /// Frozen `coordination_compact` has a real native retention backing: it
    /// archives only eligible historical events, leaves current live evidence
    /// available, verifies initialized hub authority, and remains subject to
    /// the router's stale-write refusal before any filesystem mutation.
    #[test]
    fn coordination_compact_archives_eligible_events_without_losing_current_evidence(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ledger = tempfile::tempdir()?;
        let root = ledger.path().to_string_lossy().to_string();
        let common = serde_json::json!({
            "root": root,
            "hub": "compact-e2e",
            "lane": "lane-a",
            "worktreeRoot": "E:/compact-e2e",
            "branch": "rust-build",
            "projectId": "compact-e2e"
        });

        let DispatchOutcome::Result(initialized) = dispatch(
            &tool("ocentra_enforcer_coordination_init")?,
            &common,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("compact fixture init was not dispatched".into());
        };
        assert_eq!(initialized["ok"], serde_json::json!(true));

        for body in ["one", "two", "three", "four"] {
            let mut message = common.clone();
            message["to"] = serde_json::json!("lane-b");
            message["body"] = serde_json::json!(body);
            let DispatchOutcome::Result(sent) = dispatch(
                &tool("ocentra_enforcer_coordination_message")?,
                &message,
                &ctx(McpFreshness::Fresh),
            ) else {
                return Err("compact fixture message was not dispatched".into());
            };
            assert_eq!(sent["ok"], serde_json::json!(true));
        }

        let mut compact = common;
        compact["keepLatest"] = serde_json::json!(2);
        let DispatchOutcome::Result(result) = dispatch(
            &tool("ocentra_enforcer_coordination_compact")?,
            &compact,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("compact was not dispatched".into());
        };
        assert_eq!(result["ok"], serde_json::json!(true));
        assert_eq!(result["keepLatest"], serde_json::json!(2));
        assert_eq!(
            result["compactedStreams"][0]["archivedEvents"],
            serde_json::json!(2)
        );
        assert_eq!(
            result["compactedStreams"][0]["retainedEvents"],
            serde_json::json!(2)
        );
        let live_event_count =
            enforcer_coordination::sync::stream::list_stream_files(ledger.path())?
                .into_iter()
                .map(|stream| {
                    std::fs::read_to_string(ledger.path().join("streams").join(stream.as_str()))
                        .map(|contents| {
                            contents
                                .lines()
                                .filter(|line| !line.trim().is_empty())
                                .count()
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .sum::<usize>();
        assert_eq!(live_event_count, 2, "current evidence must stay live");

        let mut wrong_hub = compact.clone();
        wrong_hub["hub"] = serde_json::json!("other-hub");
        let DispatchOutcome::Result(mismatch) = dispatch(
            &tool("ocentra_enforcer_coordination_compact")?,
            &wrong_hub,
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("hub mismatch did not produce a result".into());
        };
        assert_eq!(
            mismatch["code"],
            serde_json::json!("coordination_hub_mismatch")
        );

        let absent = ledger.path().join("absent-ledger");
        let DispatchOutcome::Result(bad_root) = dispatch(
            &tool("ocentra_enforcer_coordination_compact")?,
            &serde_json::json!({"root":absent.to_string_lossy(),"hub":"compact-e2e","lane":"lane-a","keepLatest":2}),
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("bad root did not produce a result".into());
        };
        assert_eq!(bad_root["ok"], serde_json::json!(false));
        assert!(
            !absent.exists(),
            "compact must not initialize a missing root"
        );

        assert!(matches!(
            dispatch(
                &tool("ocentra_enforcer_coordination_compact")?,
                &compact,
                &ctx(McpFreshness::Stale),
            ),
            DispatchOutcome::StaleRefused(_)
        ));
        Ok(())
    }

    #[test]
    fn check_import_boundaries_uses_only_the_native_ts_validator(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("src/domain/model.ts");
        std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
        std::fs::write(&file, "import value from '../infrastructure/value';\n")?;
        let DispatchOutcome::Result(value) = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"import-boundaries","scope":"files","files":["src/domain/model.ts"]}),
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("native import-boundaries did not produce a result".into());
        };
        assert_eq!(value["check"], serde_json::json!("import-boundaries"));
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["findings"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| row["ruleId"] == "TS-4.1")));
        Ok(())
    }

    #[test]
    fn check_literal_risk_adapts_native_report_and_aliases(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let DispatchOutcome::Result(value) = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"literal-risk","minScore":40,"literalRiskIncludeLow":true,"hardCategories":[]}),
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("literal-risk adapter did not produce a result".into());
        };
        assert!(value.get("error").is_none());
        assert!(value["literalRiskReport"]["options"].is_object());
        assert_eq!(
            value["literalRiskReport"]["options"]["includeLow"],
            serde_json::json!(true)
        );
        Ok(())
    }

    #[test]
    fn check_literal_risk_rejects_malformed_alias_types() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        for args in [
            serde_json::json!({"literalRiskMinScore":"high"}),
            serde_json::json!({"literalRiskIncludeLow":"yes"}),
            serde_json::json!({"literalRiskHardCategories":"secret-like"}),
        ] {
            let mut args = args;
            args["root"] = serde_json::json!(temp.path().to_string_lossy());
            args["check"] = serde_json::json!("literal-risk");
            let DispatchOutcome::Result(value) = dispatch(
                &tool("ocentra_enforcer_check")?,
                &args,
                &ctx(McpFreshness::Fresh),
            ) else {
                return Err("literal-risk malformed input did not produce a result".into());
            };
            assert_eq!(value["ok"], serde_json::json!(false));
            assert_ne!(value.get("error"), Some(&serde_json::Value::Null));
        }
        Ok(())
    }

    #[test]
    fn reexports_direct_route_is_not_a_broad_scan() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("src/lib.rs");
        std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
        std::fs::write(&file, "pub use crate::internal::Thing;\n")?;
        let DispatchOutcome::Result(value) = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({"root":temp.path().to_string_lossy(),"check":"reexports","scope":"files","files":["src/lib.rs"]}),
            &ctx(McpFreshness::Fresh),
        ) else {
            return Err("reexports did not dispatch".into());
        };
        assert_eq!(value["check"], serde_json::json!("reexports"));
        assert!(value["findings"].as_array().is_some_and(|rows| rows
            .iter()
            .all(|row| row["ruleId"] == "RR-7.2" || row["ruleId"] == "RR-7.3")));
        Ok(())
    }
    #[test]
    fn unavailable_default_checks_refuse_explicitly() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        for check in [
            "no-zod-source",
            "no-naked-domain-strings",
            "rust-string-boundaries",
            "no-test-doubles",
            "weak-assertions",
            "skipped-focused-tests",
            "validation-bypass",
            "placeholder-implementation",
            "cross-platform-script-commands",
        ] {
            let DispatchOutcome::Result(value) = dispatch(
                &tool("ocentra_enforcer_check")?,
                &serde_json::json!({"root":temp.path().to_string_lossy(),"check":check}),
                &ctx(McpFreshness::Fresh),
            ) else {
                return Err("refusal did not dispatch".into());
            };
            assert_eq!(value["ok"], serde_json::json!(false));
            assert_eq!(value["error"]["code"],serde_json::json!("narrow_native_engine_not_implemented: broad scan filtering is not a named-policy implementation"));
        }
        Ok(())
    }
}
