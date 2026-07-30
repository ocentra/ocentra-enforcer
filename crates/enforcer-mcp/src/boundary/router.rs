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
//! before handlers cross into domain or coordination APIs.
//! boundaryOwnerNote: enforcer-mcp owns transport dispatch and no business
//! or persistence decisions are introduced by this router.

use enforcer_coordination::api::{self, CallerContext, ClaimRequestArgs, Hub};
use enforcer_domain::{
    boundary::{
        mcp::{execution_mode, write_intent},
        validation::McpReportLabelText,
    },
    config_types::{ConfigProfileName, CrateName, HarnessArtifactByteLimit, HarnessRunLimit},
    coordination_types::{
        ClaimOutcomeStatus, ClaimPath, ClaimReason, CoordinationBranch, CoordinationLedgerRoot,
        CoordinationProjectId, CoordinationRepoRoot, CoordinationWorktree,
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
        "ocentra_enforcer_coordination_status" => {
            DispatchOutcome::Result(coordination_status(args))
        }
        "ocentra_enforcer_coordination_claim" => DispatchOutcome::Result(coordination_claim(args)),
        "ocentra_enforcer_ui" => DispatchOutcome::Result(ui_tool(args)),
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
    let Some(root_raw) = args.get("root").and_then(serde_json::Value::as_str) else {
        return json_error("scan requires a `root` path");
    };
    let root = match root_raw.parse::<RepoRoot>() {
        Ok(value) => value,
        Err(err) => return json_error(&err.to_string()),
    };
    let files = match args.get("files") {
        None => None,
        Some(serde_json::Value::Array(values)) => match values
            .iter()
            .map(|value| value.as_str().map(std::path::PathBuf::from))
            .collect::<Option<Vec<_>>>()
        {
            Some(values) => Some(values),
            None => return json_error("scan `files` must contain only paths"),
        },
        Some(_) => return json_error("scan `files` must be an array"),
    };
    let languages = match parse_scan_languages(args.get("languages")) {
        Ok(value) => value,
        Err(message) => return json_error(&message),
    };
    let scope = match parse_scan_scope(args.get("scope"), files, args) {
        Ok(value) => value,
        Err(message) => return json_error(&message),
    };
    let request = enforcer_scan::boundary::native_scan::NativeScanRequest { scope, languages };
    let result = enforcer_scan::boundary::native_scan::execute(&request, &root)
        .map_err(|error| error.to_string())
        .and_then(|result| serde_json::to_value(result.report).map_err(|error| error.to_string()));
    match result {
        Ok(value) => value,
        Err(err) => json_error(&err),
    }
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

/// Native MCP check adapter. A named check is accepted only once its exact
/// frozen-MJS rule mapping is backed by a runtime-wired Rust validator.
fn check(args: &serde_json::Value, ctx: &DispatchContext) -> serde_json::Value {
    let Some(name) = args.get("check").and_then(serde_json::Value::as_str) else {
        return json_error("check requires a named `check`");
    };
    if name != "no-zod-source" {
        return json_error(&format!("native MCP check `{name}` is not wired yet"));
    }
    let Some(mut scan_args) = args.as_object().cloned() else {
        return json_error("check arguments must be an object");
    };
    scan_args.remove("check");
    let mut report = scan_unrecorded(&serde_json::Value::Object(scan_args));
    let Some(object) = report.as_object_mut() else {
        return json_error("native scan produced an invalid report shape");
    };
    if object.contains_key("error") {
        return report;
    }
    for field in ["violations", "warnings", "waived", "findings"] {
        if let Some(serde_json::Value::Array(findings)) = object.get_mut(field) {
            findings.retain(|finding| finding.get("ruleId") == Some(&serde_json::json!("TS-1.2")));
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
    let (root, config, query, _, _, artifact_kind, limit_bytes) =
        match decode_harness_query(args, "run_status") {
            Ok(value) => value,
            Err(error) => return json_error(&error),
        };
    let summary = match enforcer_harness::query::run_summary(
        std::path::Path::new(root.as_str()),
        &config,
        &query,
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
        history.latest(&root, filter).map(validation_summary_json)
    });
    let artifact = if summary.is_some() && args.get("artifact").is_some() {
        match enforcer_harness::query::read_artifact(
            std::path::Path::new(root.as_str()),
            &config,
            &query,
            artifact_kind,
            limit_bytes,
        ) {
            Ok((true, Some(run_id), text, _)) => {
                let path = summary
                    .as_ref()
                    .and_then(|run| run.get("artifacts"))
                    .and_then(|artifacts| artifacts.get(artifact_kind.as_str()))
                    .and_then(serde_json::Value::as_str);
                match path {
                    Some(path) => Some(
                        serde_json::json!({"ok":true,"runId":run_id,"artifact":artifact_kind.as_str(),"path":path,"text":text}),
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
    let mut result = serde_json::json!({
        "ok": true,
        "summary": summary.clone().or_else(|| validation_summary.clone()).unwrap_or(serde_json::Value::Null),
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
        Err(error) => return json_error(&error.to_string()),
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
    let request = enforcer_scan::doctor::DoctorRequest::new(
        root.clone(),
        enforcer_scan::boundary::native_scan::NativeScanRequest {
            scope,
            languages: Vec::new(),
        },
        config,
    );
    match enforcer_scan::doctor::run(&request) {
        Ok(report) => serde_json::json!({
            "ok": report.ok(), "command": report.command(), "root": root.as_str(),
            "profileName": report.profile_name(),
            "checks": report.checks().iter().map(|check| serde_json::json!({"name": check.name(), "ok": check.ok(), "detail": check.detail()})).collect::<Vec<_>>(),
            "violations": [],
        }),
        Err(error) => json_error(&error.to_string()),
    }
}

/// Shared typed adapter for the frozen harness-query tools. The raw JSON
/// shape stops here; all storage selection and filtering remains owned by
/// `enforcer-harness`.
fn decode_harness_query(
    args: &serde_json::Value,
    operation: &str,
) -> Result<
    (
        RepoRoot,
        enforcer_domain::config_types::HarnessConfig,
        enforcer_harness::query::RunQuery,
        enforcer_harness::query::DiagnosticsFilter,
        Option<HarnessRunLimit>,
        HarnessArtifactKind,
        Option<HarnessArtifactByteLimit>,
    ),
    String,
> {
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
                ))
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
            ))
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
    Ok((
        root,
        config,
        query,
        diagnostics,
        optional_limit("diagnosticLimit")?,
        artifact,
        limit_bytes,
    ))
}

fn diagnostics(args: &serde_json::Value) -> serde_json::Value {
    let (root, config, query, filter, _, _, _) = match decode_harness_query(args, "diagnostics") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::query::run_diagnostics(
        std::path::Path::new(root.as_str()),
        &config,
        &query,
        &filter,
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
    let (root, config, query, _, diagnostic_limit, _, _) =
        match decode_harness_query(args, "last_failure") {
            Ok(value) => value,
            Err(error) => return json_error(&error),
        };
    match enforcer_harness::query::last_failure(
        std::path::Path::new(root.as_str()),
        &config,
        &query,
        diagnostic_limit,
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
    let (root, config, query, _, _, artifact, limit_bytes) =
        match decode_harness_query(args, "artifact") {
            Ok(value) => value,
            Err(error) => return json_error(&error),
        };
    match enforcer_harness::query::read_artifact(
        std::path::Path::new(root.as_str()),
        &config,
        &query,
        artifact,
        limit_bytes,
    ) {
        Ok((true, Some(run_id), text, _)) => {
            let path = enforcer_harness::query::run_summary(
                std::path::Path::new(root.as_str()),
                &config,
                &query,
            )
            .ok()
            .flatten()
            .and_then(|run| {
                run.get("artifacts")?
                    .get(artifact.as_str())?
                    .as_str()
                    .map(str::to_owned)
            });
            match path {
                Some(path) => serde_json::json!({
                    "ok": true, "runId": run_id, "artifact": artifact.as_str(), "path": path, "text": text,
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
    let (root, config, _, _, _, _, _) = match decode_harness_query(args, "reset_runs") {
        Ok(value) => value,
        Err(error) => return json_error(&error),
    };
    match enforcer_harness::storage::reset_runs(std::path::Path::new(root.as_str()), &config) {
        Ok(removed) => serde_json::json!({
            "ok": true,
            "root": root.as_str(),
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
            return json_error("native MCP route supports only `files` or `workspace` scope")
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
                return json_error(&format!("cannot resolve default test-doctrine root: {err}"))
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
                return json_error(&format!("cannot resolve default UI coupling root: {err}"))
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
        assert!(value["findings"].as_array().is_some_and(|findings| findings
            .iter()
            .any(|finding| finding["ruleId"] == "T1-NOREEXPORT.1")));
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
        assert!(value["findings"].as_array().is_some_and(|findings| findings
            .iter()
            .all(|finding| finding["ruleId"] != "T1-NOREEXPORT.1")));
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
    fn check_rejects_a_registered_name_without_native_wiring(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let outcome = dispatch(
            &tool("ocentra_enforcer_check")?,
            &serde_json::json!({
                "root": temp.path().to_string_lossy(),
                "check": "weak-assertions",
            }),
            &ctx(McpFreshness::Fresh),
        );
        let DispatchOutcome::Result(value) = outcome else {
            return Err("native check did not produce a result".into());
        };
        assert_eq!(value["ok"], serde_json::json!(false));
        assert!(value["error"]
            .as_str()
            .is_some_and(|message| message.contains("not wired yet")));
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
}
