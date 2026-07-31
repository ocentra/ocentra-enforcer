//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! The consolidated MCP boundary tool registry: every tool this server exposes,
//! its JSON input schema, and (for `check`) the named-check enum parity
//! seam (workpack row "check named-check enum parity").
//!
//! This module is DATA ONLY Ã¢â‚¬â€ no dispatch logic (that is [`crate::router`])
//! and no I/O (that is [`crate::sink`]). Tool descriptions here are read by
//! the d05 context-budget tool-surface measure (see [`tool_surface_bytes`])
//! and by `tools/list`.
//! Negative invalid-input coverage: malformed or corrupt payloads are rejected by this boundary.

use crate::boundary::tool_descriptor::ToolDescriptorDto;
use enforcer_domain::ids::RuleId;
use enforcer_domain::mcp_types::McpToolName;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A native explanation record projected from the checked-in Rust pack rule
/// catalog.  This is deliberately a small, typed read model: MCP consumes no
/// MJS process and does not invent hints when a rule is absent.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuleExplanationDto {
    #[serde(rename = "id")]
    pub rule_id: RuleId,
    pub language: String,
    pub family: String,
    pub severity: String,
    pub title: String,
    #[serde(rename = "snippet")]
    pub fix_hint: String,
    #[serde(rename = "doc")]
    pub doc_anchor: String,
}

#[derive(serde::Deserialize)]
struct RuleCatalog {
    rules: Vec<RuleExplanationDto>,
}

/// Resolve the native checked-in rule catalog once, fail-closed on a malformed
/// catalog, and look up a branded [`RuleId`].  The source is compiled into the
/// Rust binary so `ocentra_enforcer_explain` has no MJS runtime dependency.
pub fn explain_rule(rule_id: &RuleId) -> Result<Option<RuleExplanationDto>, String> {
    static CATALOG: OnceLock<Result<BTreeMap<RuleId, RuleExplanationDto>, String>> =
        OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        let parsed: RuleCatalog =
            serde_json::from_str(include_str!("../../../../rules/rules.json"))
                .map_err(|error| format!("native rule catalog is malformed: {error}"))?;
        let mut indexed = BTreeMap::new();
        for entry in parsed.rules {
            if indexed.insert(entry.rule_id.clone(), entry).is_some() {
                return Err("native rule catalog contains a duplicate rule id".to_owned());
            }
        }
        Ok(indexed)
    });
    catalog
        .as_ref()
        .map(|entries| entries.get(rule_id).cloned())
        .map_err(Clone::clone)
}

/// Every CANONICAL (`ocentra_enforcer_*`) tool name this server registers,
/// grouped by family for readability. This is the source of truth the
/// router's dispatch table and the legacy-alias table
/// ([`crate::aliases::alias_name`]) both key off; add a tool here ONCE and
/// it appears canonically + (until the deprecation window closes) under its
/// `rust_rules_*` alias.
pub const CANONICAL_TOOLS: &[&str] = &[
    // scan/check family (arc-15 delegate)
    "ocentra_enforcer_scan",
    "ocentra_enforcer_check",
    "ocentra_enforcer_explain",
    // run/diagnostics family (arc-18 delegate)
    "ocentra_enforcer_run",
    "ocentra_enforcer_run_status",
    "ocentra_enforcer_diagnostics",
    "ocentra_enforcer_last_failure",
    "ocentra_enforcer_artifact",
    "ocentra_enforcer_prune_runs",
    "ocentra_enforcer_reset_runs",
    "ocentra_enforcer_route",
    "ocentra_enforcer_doctor",
    // project-posture analysis (native `enforcer-scan` delegate)
    "ocentra_enforcer_test_doctrine_scan",
    "ocentra_enforcer_ui_logic_coupling_scan",
    // proof family (arc-17 delegate)
    "ocentra_enforcer_proof_run",
    "ocentra_enforcer_proof_status",
    "ocentra_enforcer_proof_artifact",
    "ocentra_enforcer_proof_claim",
    "ocentra_enforcer_proof_route",
    "ocentra_enforcer_proof_export",
    "ocentra_enforcer_proof_import_legacy",
    "ocentra_enforcer_proof_inventory",
    "ocentra_enforcer_proof_last_failure",
    "ocentra_enforcer_proof_parity",
    "ocentra_enforcer_proof_prune",
    "ocentra_enforcer_proof_reset",
    "ocentra_enforcer_proof_diagnostics",
    // coordination family (arc-16 delegate) Ã¢â‚¬â€ write tools per the gate row
    "ocentra_enforcer_coordination_init",
    "ocentra_enforcer_coordination_claim",
    "ocentra_enforcer_coordination_closeout",
    "ocentra_enforcer_coordination_release",
    "ocentra_enforcer_coordination_report",
    "ocentra_enforcer_coordination_message",
    "ocentra_enforcer_coordination_mail",
    "ocentra_enforcer_coordination_sync",
    "ocentra_enforcer_coordination_ensure",
    "ocentra_enforcer_coordination_compact",
    "ocentra_enforcer_coordination_repair",
    // coordination family Ã¢â‚¬â€ read-only, never write-gated
    "ocentra_enforcer_coordination_status",
    "ocentra_enforcer_coordination_health",
    "ocentra_enforcer_coordination_index",
    "ocentra_enforcer_coordination_inbox",
    "ocentra_enforcer_coordination_streams",
    "ocentra_enforcer_coordination_tasks",
    "ocentra_enforcer_coordination_workers",
    "ocentra_enforcer_coordination_notify",
    "ocentra_enforcer_coordination_presence",
    "ocentra_enforcer_coordination_peer",
    "ocentra_enforcer_coordination_guard",
    // server/meta Ã¢â‚¬â€ never write-gated
    "ocentra_enforcer_mcp_status",
    // ui family (arc-24/g01 delegate) Ã¢â‚¬â€ read-only report of the served
    // URL, never write-gated, never auto-launches (see
    // `enforcer_ui::serve::ui_tool_response`'s silent-agent-safe-by-
    // construction contract)
    "ocentra_enforcer_ui",
];

/// The fixed enum of named checks `ocentra_enforcer_check` advertises.
/// Ported verbatim from `mcp/rust-rules-mcp-tool-registry-rules.mjs`'s
/// `check` input schema enum. See module docs on the parity seam this
/// backs.
pub const NAMED_CHECKS: &[&str] = &[
    "no-zod-source",
    "no-naked-domain-strings",
    "no-test-doubles",
    "weak-assertions",
    "skipped-focused-tests",
    "validation-bypass",
    "placeholder-implementation",
    "reexports",
    "cross-platform-script-commands",
    "generated-artifacts",
    "secrets",
    "rust-string-boundaries",
    "source-shape",
    "required-tests",
    "single-source-contracts",
    "dependency-policy",
    "sbom",
    "literal-risk",
    "ai-rule-index",
    "import-boundaries",
    "architecture-policy",
];

/// Named-check -> backing [`RuleId`] family declaration. This is the
/// SEAM the workpack's "check named-check enum parity" row requires: a
/// place that declares what backs each named check, so an entry can never
/// silently go unbacked without a test noticing.
///
/// # Honest scope note
/// As of this pass, the language/mechanization packs that OWN these
/// validators (arc-06..12, d01) have not yet registered a "named check"
/// lookup surface of their own Ã¢â‚¬â€ `enforcer-rules`' `RuleRegistry` is keyed
/// by [`RuleId`] (e.g. `RR-6.1`), not by these friendly slugs. Rather than
/// fabricate a false-positive parity claim, this table is declared HERE,
/// owned by this crate, and the parity test in this module asserts
/// bidirectional equality between [`NAMED_CHECKS`] and this table's keys
/// (a same-crate consistency gate). When a sibling pack later exposes a
/// real named-check -> RuleId-family registry, this table's values (empty
/// `Vec`s below) are the ONLY thing that changes Ã¢â‚¬â€ the parity test and its
/// bidirectional-equality assertion do not need to change shape, only this
/// data. Until then an empty backing vec means "declared, not yet wired",
/// which is what [`is_wired`] reports honestly rather than silently.
pub fn named_check_backing() -> Vec<(&'static str, Vec<RuleId>)> {
    NAMED_CHECKS
        .iter()
        .map(|&name| {
            let rule_ids = match name {
                // A nonempty row is an executable native named-check
                // contract, not merely a frozen-MJS rule-family reference.
                // Checks without a narrow native implementation deliberately
                // retain their empty default below; the router returns an
                // explicit refusal instead of claiming a filtered broad scan.
                "no-naked-domain-strings" => ["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "validation-bypass" => ["RR-2.1", "RR-2.2", "TS-2.1", "PY-1.1", "PY-1.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "placeholder-implementation" => ["RR-4.2", "RR-4.3", "SRC-1.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "skipped-focused-tests" => ["TS-3.1", "PY-2.1", "TEST-1.3"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "weak-assertions" => ["TEST-1.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "no-zod-source" => ["TS-1.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "no-test-doubles" => ["TEST-1.1", "TS-8.8"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "cross-platform-script-commands" => ["PORT-1.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "reexports" => ["T1-NOREEXPORT.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "generated-artifacts" => ["GEN-1.1", "GEN-1.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "mutation-risk" => ["ENF-2.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "docs-completeness" => [
                    "DOCENF-1.1",
                    "DOCENF-1.2",
                    "DOCENF-1.3",
                    "DOCENF-1.4",
                    "DOCENF-1.5",
                    "DOCENF-1.6",
                    "DOCENF-1.7",
                    "DOCENF-1.8",
                    "DOCENF-1.9",
                    "DOCENF-1.10",
                ]
                .into_iter()
                .filter_map(|raw| raw.parse::<RuleId>().ok())
                .collect(),
                "source-shape" => [
                    "SRC-1.1", "SRC-2.1", "SRC-2.2", "SRC-2.4", "SRC-2.5", "SRC-2.6", "SRC-2.7",
                ]
                .into_iter()
                .filter_map(|raw| raw.parse::<RuleId>().ok())
                .collect(),
                // Dedicated native engines: these names are not filtered
                // full scans. The router decodes the shared typed scan scope
                // then invokes the corresponding narrow engine.
                "secrets" => [
                    "SEC-1.1", "SEC-1.2", "SEC-2.1", "SEC-2.2", "SEC-2.3", "SEC-2.4", "SEC-2.5",
                    "SEC-2.6", "SEC-2.7", "SEC-2.8", "SEC-2.9", "SEC-2.10", "SEC-2.11", "SEC-2.12",
                    "SEC-2.13", "SEC-2.14", "SEC-2.15", "SEC-2.16", "SEC-2.17", "SEC-2.18",
                    "SEC-2.19", "SEC-2.20",
                ]
                .into_iter()
                .filter_map(|raw| raw.parse::<RuleId>().ok())
                .collect(),
                "dependency-policy" => ["RR-9.3"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "sbom" => ["SBOM-1.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "literal-risk" => ["LIT-2.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "import-boundaries" => ["TS-4.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "rust-string-boundaries" => ["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "required-tests" => ["TEST-2.1", "TEST-2.2"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "single-source-contracts" => ["CONTRACT-1.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "ai-rule-index" => ["AI-1.1"]
                    .into_iter()
                    .filter_map(|raw| raw.parse::<RuleId>().ok())
                    .collect(),
                "architecture-policy" => [
                    "RR-7.2",
                    "RR-7.3",
                    "TS-1.1",
                    "RR-2.1",
                    "RR-2.2",
                    "TS-2.1",
                    "PY-1.1",
                    "PY-1.2",
                    "RR-4.2",
                    "RR-4.3",
                    "SRC-1.2",
                    "TS-3.1",
                    "PY-2.1",
                    "TEST-1.3",
                    "TEST-1.2",
                    "RR-6.1",
                    "RR-6.5",
                    "RR-18.16",
                    "TS-1.3",
                    "PY-1.3",
                    "TS-1.2",
                    "TEST-1.1",
                    "TS-8.8",
                    "PORT-1.1",
                    "GEN-1.2",
                    "ARCH-1.10",
                ]
                .into_iter()
                .filter_map(|raw| raw.parse::<RuleId>().ok())
                .collect(),
                _ => Vec::new(),
            };
            (name, rule_ids)
        })
        .collect()
}

/// True once at least one [`RuleId`] backs the named check.
pub fn is_wired(entry: &(&'static str, Vec<RuleId>)) -> bool {
    !entry.1.is_empty()
}

/// Byte length of the JSON-encoded canonical tool descriptor list Ã¢â‚¬â€ the
/// measurable surface the d05 context-budget ratchet consumes (this crate
/// owns the measurable surface; d05 owns the baseline/ratchet files, see
/// the workpack's "Parallel Ownership Notes").
pub(crate) fn tool_surface_bytes(descriptors: &[ToolDescriptorDto]) -> usize {
    serde_json::to_vec(descriptors)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

/// Build every tool descriptor: canonical tools first (stable order,
/// matching [`CANONICAL_TOOLS`]), then legacy aliases (see
/// [`crate::aliases`]) Ã¢â‚¬â€ mirrors `TOOLS = [...CANONICAL_TOOLS,
/// ...LEGACY_ALIAS_TOOLS]` in the legacy `.mjs` registry so the
/// `tools/list` order is deterministic (required for the d05 measure to be
/// reproducible).
pub(crate) fn build_tool_descriptors() -> Vec<ToolDescriptorDto> {
    let mut out: Vec<ToolDescriptorDto> = CANONICAL_TOOLS
        .iter()
        .map(|&name| ToolDescriptorDto {
            name: name.to_owned(),
            description: canonical_description(name),
            input_schema: canonical_input_schema(name),
        })
        .collect();
    if crate::aliases::deprecation_window_open() {
        for &canonical in CANONICAL_TOOLS {
            let Ok(canonical_name) = McpToolName::try_new(canonical) else {
                continue;
            };
            let Ok(alias) = crate::aliases::alias_name(&canonical_name) else {
                continue;
            };
            out.push(ToolDescriptorDto {
                name: alias.to_string(),
                description: format!(
                    "Legacy alias for {canonical}; kept for one Rust-pack compatibility release."
                ),
                input_schema: canonical_input_schema(canonical),
            });
        }
    }
    out
}

fn canonical_description(name: &str) -> String {
    match name {
        "ocentra_enforcer_check" => {
            "Run a named Ocentra Enforcer reusable check (see the check enum: \
             no-zod-source, source-shape, dependency-policy, sbom, ...)."
                .to_owned()
        }
        "ocentra_enforcer_scan" => "Run the parallel scan engine over a resolved scope.".to_owned(),
        "ocentra_enforcer_mcp_status" => {
            "Report this MCP server's freshness/fingerprint status; never write-gated.".to_owned()
        }
        "ocentra_enforcer_ui" => {
            "Report the g01 UI serve surface's resolved URL and view-mount registry; never \
             binds a socket or launches the surface itself (silent-agent-safe)."
                .to_owned()
        }
        "ocentra_enforcer_test_doctrine_scan" => {
            "Analyze project test posture from native filesystem, manifest, and CI evidence; never runs tests or shells out.".to_owned()
        }
        "ocentra_enforcer_ui_logic_coupling_scan" => {
            "Advisory native ARCH-1.16 presentation/UI-to-business-logic coupling evidence scan; never changes CI gating.".to_owned()
        }
        other => format!("Ocentra Enforcer tool: {other}."),
    }
}

fn canonical_input_schema(name: &str) -> serde_json::Value {
    if name == "ocentra_enforcer_mcp_status" {
        return serde_json::json!({"type":"object","additionalProperties":false,"properties":{}});
    }
    if name == "ocentra_enforcer_explain" {
        return serde_json::json!({"type":"object","additionalProperties":false,"required":["ruleId"],"properties":{"ruleId":{"type":"string"}}});
    }
    if name == "ocentra_enforcer_route" {
        return common_input_schema_with(serde_json::Map::from_iter([(
            "ruleId".to_owned(),
            serde_json::json!({"type":"string"}),
        )]));
    }
    if name.starts_with("ocentra_enforcer_coordination_") {
        return coordination_input_schema(name);
    }
    if matches!(
        name,
        "ocentra_enforcer_proof_route"
            | "ocentra_enforcer_proof_run"
            | "ocentra_enforcer_proof_status"
            | "ocentra_enforcer_proof_inventory"
            | "ocentra_enforcer_proof_import_legacy"
            | "ocentra_enforcer_proof_parity"
            | "ocentra_enforcer_proof_claim"
            | "ocentra_enforcer_proof_last_failure"
            | "ocentra_enforcer_proof_diagnostics"
            | "ocentra_enforcer_proof_artifact"
            | "ocentra_enforcer_proof_reset"
            | "ocentra_enforcer_proof_prune"
            | "ocentra_enforcer_proof_export"
    ) {
        return serde_json::json!({"type":"object","additionalProperties":false,"properties":{
            "root":{"type":"string"},"profile":{"type":"string"},
            "scope":{"type":"string","enum":["workspace","files","crate","diff"]},
            "files":{"type":"array","items":{"type":"string"}},"plan":{"type":"string"},
            "capability":{"type":"string","enum":["ci","local","windows","linux","macos","wsl","android-emulator","android-device","ios-simulator","ios-device","browser","network","cloud","manual-required"]},
            "proofId":{"type":"string"},"proofIds":{"type":"array","items":{"type":"string"}},"runId":{"type":"string"},
            "command":{"type":"array","items":{"type":"string"}},"tags":{"type":"array","items":{"type":"string"}},
            "artifact":{"type":"string"},"legacyPaths":{"type":"array","items":{"type":"string"}},
            "limit":{"type":"number"},"diagnosticLimit":{"type":"number"},"limitBytes":{"type":"number"},
            "includeScripts":{"type":"boolean"},"status":{"type":"string","enum":["passed","failed","manual-required","unavailable","waived"]},
            "pin":{"type":"boolean"},"claimId":{"type":"string"},"prReady":{"type":"boolean"},"allowDirty":{"type":"boolean"},"dryRun":{"type":"boolean"}
        }});
    }
    if name == "ocentra_enforcer_run" {
        return serde_json::json!({"type":"object","additionalProperties":false,"required":["command"],"properties":{"root":{"type":"string"},"profile":{"type":"string"},"tool":{"type":"string"},"language":{"type":"string","enum":["rust","typescript","python","common"]},"cwd":{"type":"string"},"runId":{"type":"string"},"crateName":{"type":"string"},"packageName":{"type":"string"},"domain":{"type":"string"},"command":{"type":"array","items":{"type":"string"}},"tags":{"type":"array","items":{"type":"string"}}}});
    }
    if matches!(
        name,
        "ocentra_enforcer_run_status"
            | "ocentra_enforcer_diagnostics"
            | "ocentra_enforcer_last_failure"
            | "ocentra_enforcer_artifact"
            | "ocentra_enforcer_prune_runs"
            | "ocentra_enforcer_reset_runs"
    ) {
        return serde_json::json!({"type":"object","additionalProperties":false,"properties":{"root":{"type":"string"},"runId":{"type":"string"},"limit":{"type":"integer","minimum":0},"diagnosticLimit":{"type":"integer","minimum":0},"severity":{"type":"string","enum":["error","warning","info"]},"status":{"type":"string","enum":["passed","failed"]},"file":{"type":"string"},"tool":{"type":"string"},"crateName":{"type":"string"},"packageName":{"type":"string"},"domain":{"type":"string"},"tag":{"type":"string"},"artifact":{"type":"string","enum":["stdout","stderr","diagnostics","events"]},"limitBytes":{"type":"integer","minimum":0}}});
    }
    if name == "ocentra_enforcer_check" {
        return serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "check": { "type": "string", "enum": NAMED_CHECKS },
                "root": { "type": "string" },
                "configPath": { "type": "string" },
                "profile": { "type": "string" },
                "scope": { "type": "string", "enum": ["workspace", "files", "crate", "diff"] },
                "files": { "type": "array", "items": { "type": "string" } },
                "crateName": { "type": "string" },
                "languages": { "type": "array", "items": { "type": "string", "enum": ["rust", "typescript", "python", "common"] } },
                "base": { "type": "string" },
                "head": { "type": "string" },
                "checkConfigPath": { "type": "string" },
                "output": { "type": "string" },
                "dryRun": { "type": "boolean" },
                "staged": { "type": "boolean" },
                "tracked": { "type": "boolean" },
                "strictEmptyTestTrees": { "type": "boolean" },
                "diagnosticLimit": { "type": "number" },
                "summaryOnly": { "type": "boolean" },
                "groupBy": { "type": "string", "enum": ["file", "slice"] },
                "includeScope": { "type": "boolean" },
            },
            "required": ["check"],
        });
    }
    if name == "ocentra_enforcer_doctor" {
        return common_input_schema_with(serde_json::Map::new());
    }
    if matches!(
        name,
        "ocentra_enforcer_diagnostics"
            | "ocentra_enforcer_last_failure"
            | "ocentra_enforcer_artifact"
    ) {
        return serde_json::json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "root": { "type": "string" }, "runId": { "type": "string" },
                "limit": { "type": "integer", "minimum": 0 },
                "diagnosticLimit": { "type": "integer", "minimum": 0 },
                "severity": { "type": "string", "enum": ["error", "warning", "info"] },
                "status": { "type": "string", "enum": ["passed", "failed"] },
                "file": { "type": "string" }, "tool": { "type": "string" },
                "crateName": { "type": "string" }, "packageName": { "type": "string" },
                "domain": { "type": "string" }, "tag": { "type": "string" },
                "artifact": { "type": "string", "enum": ["stdout", "stderr", "diagnostics", "events"] },
                "limitBytes": { "type": "integer", "minimum": 0 }
            }
        });
    }
    if name == "ocentra_enforcer_ui" {
        return serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "host": { "type": "string" },
                "port": { "type": "integer" },
                "token": { "type": "string" },
            },
        });
    }
    if name == "ocentra_enforcer_test_doctrine_scan" {
        return serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "root": { "type": "string", "description": "Target repository root; defaults to the server working directory." },
            },
        });
    }
    if name == "ocentra_enforcer_ui_logic_coupling_scan" {
        return serde_json::json!({
            "type": "object", "additionalProperties": false,
            "properties": { "root": { "type": "string", "description": "Target repository root; defaults to the server working directory." } },
        });
    }
    if name == "ocentra_enforcer_scan" {
        return common_input_schema_with(serde_json::Map::from_iter([
            ("cargo".to_owned(), serde_json::json!({"type":"boolean"})),
            (
                "diagnosticLimit".to_owned(),
                serde_json::json!({"type":"number"}),
            ),
            (
                "summaryOnly".to_owned(),
                serde_json::json!({"type":"boolean"}),
            ),
            (
                "groupBy".to_owned(),
                serde_json::json!({"type":"string","enum":["file","slice"]}),
            ),
            (
                "includeScope".to_owned(),
                serde_json::json!({"type":"boolean"}),
            ),
        ]));
    }
    // A newly registered tool must supply an explicit schema; accepting an
    // unbounded object would silently violate MCP-1.3.
    serde_json::json!({"type":"object","additionalProperties":false,"properties":{}})
}

fn common_input_schema_with(
    extra: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut properties = serde_json::Map::from_iter([
        ("root".to_owned(), serde_json::json!({"type":"string"})),
        (
            "configPath".to_owned(),
            serde_json::json!({"type":"string"}),
        ),
        ("profile".to_owned(), serde_json::json!({"type":"string"})),
        (
            "scope".to_owned(),
            serde_json::json!({"type":"string","enum":["workspace","files","crate","diff"]}),
        ),
        (
            "files".to_owned(),
            serde_json::json!({"type":"array","items":{"type":"string"}}),
        ),
        ("crateName".to_owned(), serde_json::json!({"type":"string"})),
        (
            "languages".to_owned(),
            serde_json::json!({"type":"array","items":{"type":"string","enum":["rust","typescript","python","common"]}}),
        ),
        ("base".to_owned(), serde_json::json!({"type":"string"})),
        ("head".to_owned(), serde_json::json!({"type":"string"})),
    ]);
    properties.extend(extra);
    serde_json::json!({"type":"object","additionalProperties":false,"properties":properties})
}

fn coordination_input_schema(name: &str) -> serde_json::Value {
    let mut properties = serde_json::Map::from_iter([
        ("root".to_owned(), serde_json::json!({"type":"string"})),
        ("stateRoot".to_owned(), serde_json::json!({"type":"string"})),
        ("hub".to_owned(), serde_json::json!({"type":"string"})),
        ("lane".to_owned(), serde_json::json!({"type":"string"})),
        (
            "paths".to_owned(),
            serde_json::json!({"type":"array","items":{"type":"string"}}),
        ),
        (
            "changedPaths".to_owned(),
            serde_json::json!({"type":"array","items":{"type":"string"}}),
        ),
        ("reason".to_owned(), serde_json::json!({"type":"string"})),
        ("summary".to_owned(), serde_json::json!({"type":"string"})),
        ("owner".to_owned(), serde_json::json!({"type":"string"})),
        (
            "operation".to_owned(),
            serde_json::json!({"type":"string","enum":["inspect","edit","commit","push","rebase","merge","pr_ready"]}),
        ),
        (
            "lockKind".to_owned(),
            serde_json::json!({"type":"string","enum":["writeLock","globalWriteLock","branchLease","workReservation"]}),
        ),
        (
            "onConflict".to_owned(),
            serde_json::json!({"type":"string","enum":["fail","intent"]}),
        ),
        (
            "claimGroup".to_owned(),
            serde_json::json!({"type":"string"}),
        ),
        ("waitMs".to_owned(), serde_json::json!({"type":"number"})),
        ("from".to_owned(), serde_json::json!({"type":"string"})),
        ("to".to_owned(), serde_json::json!({"type":"string"})),
        ("subject".to_owned(), serde_json::json!({"type":"string"})),
        ("body".to_owned(), serde_json::json!({"type":"string"})),
        ("message".to_owned(), serde_json::json!({"type":"string"})),
        ("messageId".to_owned(), serde_json::json!({"type":"string"})),
        ("taskId".to_owned(), serde_json::json!({"type":"string"})),
        ("state".to_owned(), serde_json::json!({"type":"string"})),
        ("sessionId".to_owned(), serde_json::json!({"type":"string"})),
        ("action".to_owned(), serde_json::json!({"type":"string"})),
        ("peer".to_owned(), serde_json::json!({"type":"string"})),
        ("peerUrl".to_owned(), serde_json::json!({"type":"string"})),
        ("url".to_owned(), serde_json::json!({"type":"string"})),
        ("name".to_owned(), serde_json::json!({"type":"string"})),
        ("token".to_owned(), serde_json::json!({"type":"string"})),
        ("tokenEnv".to_owned(), serde_json::json!({"type":"string"})),
        (
            "mode".to_owned(),
            serde_json::json!({"type":"string","enum":["pull","push","both"]}),
        ),
        ("host".to_owned(), serde_json::json!({"type":"string"})),
        ("port".to_owned(), serde_json::json!({"type":"number"})),
        (
            "keepLatest".to_owned(),
            serde_json::json!({"type":"number"}),
        ),
        ("projectId".to_owned(), serde_json::json!({"type":"string"})),
        ("repoRoot".to_owned(), serde_json::json!({"type":"string"})),
        (
            "worktreeRoot".to_owned(),
            serde_json::json!({"type":"string"}),
        ),
        ("cwd".to_owned(), serde_json::json!({"type":"string"})),
        ("gitRemote".to_owned(), serde_json::json!({"type":"string"})),
        ("branch".to_owned(), serde_json::json!({"type":"string"})),
        ("commit".to_owned(), serde_json::json!({"type":"string"})),
        (
            "codexThreadId".to_owned(),
            serde_json::json!({"type":"string"}),
        ),
        (
            "codexSessionId".to_owned(),
            serde_json::json!({"type":"string"}),
        ),
        ("stateFile".to_owned(), serde_json::json!({"type":"string"})),
        ("peek".to_owned(), serde_json::json!({"type":"boolean"})),
        ("dryRun".to_owned(), serde_json::json!({"type":"boolean"})),
        ("write".to_owned(), serde_json::json!({"type":"boolean"})),
        ("focused".to_owned(), serde_json::json!({"type":"boolean"})),
        (
            "allowPrimaryWithoutClaims".to_owned(),
            serde_json::json!({"type":"boolean"}),
        ),
        (
            "allowMergeRisks".to_owned(),
            serde_json::json!({"type":"boolean"}),
        ),
        ("all".to_owned(), serde_json::json!({"type":"boolean"})),
        ("allOwned".to_owned(), serde_json::json!({"type":"boolean"})),
        ("allLanes".to_owned(), serde_json::json!({"type":"boolean"})),
        (
            "allowOtherNode".to_owned(),
            serde_json::json!({"type":"boolean"}),
        ),
        (
            "releaseOwned".to_owned(),
            serde_json::json!({"type":"boolean"}),
        ),
        (
            "repairStale".to_owned(),
            serde_json::json!({"type":"boolean"}),
        ),
        ("limit".to_owned(), serde_json::json!({"type":"number"})),
    ]);
    let action = match name {
        "ocentra_enforcer_coordination_claim" => Some(vec!["claim"]),
        "ocentra_enforcer_coordination_release" => Some(vec!["release"]),
        "ocentra_enforcer_coordination_closeout" => Some(vec!["closeout"]),
        "ocentra_enforcer_coordination_report" => Some(vec!["report"]),
        "ocentra_enforcer_coordination_message" => Some(vec!["message", "send"]),
        _ => None,
    };
    if let Some(actions) = action {
        properties.insert(
            "action".to_owned(),
            serde_json::json!({"type":"string","enum":actions}),
        );
    }
    serde_json::json!({"type":"object","additionalProperties":false,"properties":properties})
}

#[cfg(test)]
mod tests {
    use super::{
        build_tool_descriptors, is_wired, named_check_backing, tool_surface_bytes,
        RuleExplanationDto, CANONICAL_TOOLS, NAMED_CHECKS,
    };
    use std::collections::BTreeSet;

    fn normalized_schema(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => {
                let mut normalized = serde_json::Map::new();
                for (key, value) in object {
                    if key != "description" && key != "default" && key != "minimum" {
                        let normalized_value = normalized_schema(value);
                        // Frozen MJS uses JSON's single `number` wire type;
                        // native schemas may advertise integral limits for a
                        // stricter decoder. Compare the portable MCP shape.
                        if key == "type" && normalized_value == serde_json::json!("integer") {
                            normalized.insert(key.clone(), serde_json::json!("number"));
                        } else {
                            normalized.insert(key.clone(), normalized_value);
                        }
                    }
                }
                serde_json::Value::Object(normalized)
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.iter().map(normalized_schema).collect())
            }
            other => other.clone(),
        }
    }

    #[test]
    fn frozen_mjs_canonical_contract_matches_rust_except_documented_ui_tool(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This is a test-only parity oracle. The production Rust MCP never
        // executes MJS; it compares checked-in frozen tool declarations to
        // the Rust registry at build/test time so contract drift is visible.
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let script = "import { TOOLS } from './mcp/rust-rules-mcp-tool-registry.mjs'; console.log(JSON.stringify(TOOLS.filter((tool) => tool.name.startsWith('ocentra_enforcer_'))));";
        let output = std::process::Command::new("node")
            .arg("--input-type=module")
            .arg("--eval")
            .arg(script)
            .current_dir(workspace)
            .output()?;
        assert!(
            output.status.success(),
            "frozen MJS registry must load: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let frozen: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
        let frozen_by_name: std::collections::BTreeMap<String, serde_json::Value> = frozen
            .into_iter()
            .map(|tool| {
                tool["name"]
                    .as_str()
                    .map(|name| (name.to_owned(), tool["inputSchema"].clone()))
                    .ok_or("frozen tool must have a string name")
            })
            .collect::<Result<_, _>>()?;
        let rust_by_name: std::collections::BTreeMap<String, serde_json::Value> =
            build_tool_descriptors()
                .into_iter()
                .filter(|tool| tool.name.starts_with("ocentra_enforcer_"))
                .filter(|tool| tool.name != "ocentra_enforcer_ui")
                .map(|tool| (tool.name, tool.input_schema))
                .collect();
        assert_eq!(
            rust_by_name.keys().collect::<Vec<_>>(),
            frozen_by_name.keys().collect::<Vec<_>>(),
            "Rust may add only the documented native UI tool; frozen canonical names must otherwise match"
        );
        for (name, frozen_schema) in frozen_by_name {
            assert_eq!(
                normalized_schema(&rust_by_name[&name]),
                normalized_schema(&frozen_schema),
                "normalized schema mismatch for {name}"
            );
        }
        Ok(())
    }

    #[test]
    fn rule_explanation_dto_round_trips_through_the_external_catalog_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = r#"{
            "id":"RR-6.1","language":"rust","family":"domain","severity":"error",
            "title":"No raw string types","snippet":"Use a branded type.",
            "doc":"rules/rust/domain.md#covered-rules"
        }"#;
        let decoded: RuleExplanationDto = serde_json::from_str(raw)?;
        let encoded = serde_json::to_string(&decoded)?;
        let round_tripped: RuleExplanationDto = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, round_tripped);
        Ok(())
    }

    #[test]
    fn canonical_tools_list_has_no_duplicates() {
        let set: BTreeSet<&&str> = CANONICAL_TOOLS.iter().collect();
        assert_eq!(set.len(), CANONICAL_TOOLS.len());
    }

    #[test]
    fn named_checks_enum_matches_legacy_registry_verbatim() {
        // Fail fixture intent: if a legacy check id is dropped from
        // NAMED_CHECKS, this length/content check trips.
        let expected: BTreeSet<&str> = [
            "no-zod-source",
            "no-naked-domain-strings",
            "no-test-doubles",
            "weak-assertions",
            "skipped-focused-tests",
            "validation-bypass",
            "placeholder-implementation",
            "reexports",
            "cross-platform-script-commands",
            "generated-artifacts",
            "secrets",
            "rust-string-boundaries",
            "source-shape",
            "required-tests",
            "single-source-contracts",
            "dependency-policy",
            "sbom",
            "literal-risk",
            "ai-rule-index",
            "import-boundaries",
            "architecture-policy",
        ]
        .into_iter()
        .collect();
        let actual: BTreeSet<&str> = NAMED_CHECKS.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "named-check enum must not silently drop or gain an entry"
        );
    }

    #[test]
    fn named_check_backing_table_is_bidirectionally_equal_to_the_enum() {
        // Pass fixture: every enum entry has exactly one backing-table row
        // (bidirectional equality of the KEY SET Ã¢â‚¬â€ see module docs on the
        // honest-scope limitation of the VALUE side until a sibling pack
        // wires real RuleId backing in).
        let backing = named_check_backing();
        let backing_keys: BTreeSet<&str> = backing.iter().map(|(name, _)| *name).collect();
        let enum_keys: BTreeSet<&str> = NAMED_CHECKS.iter().copied().collect();
        assert_eq!(backing_keys, enum_keys);
    }

    #[test]
    fn fail_fixture_an_enum_entry_missing_from_backing_trips_the_gate() {
        // Simulates the "silently disappear" failure mode this row guards
        // against: a backing table missing one enum entry must NOT compare
        // equal to the full enum.
        let backing = named_check_backing();
        let mut backing_keys: BTreeSet<&str> = backing.iter().map(|(name, _)| *name).collect();
        backing_keys.remove("sbom");
        let enum_keys: BTreeSet<&str> = NAMED_CHECKS.iter().copied().collect();
        assert_ne!(
            backing_keys, enum_keys,
            "removing one entry must break bidirectional equality"
        );
    }

    #[test]
    fn is_wired_reports_only_the_landed_native_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let backing = named_check_backing();
        let Some(zod) = backing.iter().find(|(name, _)| *name == "no-zod-source") else {
            return Err("no-zod-source must be declared".into());
        };
        assert!(
            is_wired(zod),
            "no-zod-source now executes through the shared architecture rule-family executor"
        );
        let wired: BTreeSet<&str> = backing
            .iter()
            .filter(|entry| is_wired(entry))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            wired,
            BTreeSet::from([
                "cross-platform-script-commands",
                "no-naked-domain-strings",
                "no-test-doubles",
                "no-zod-source",
                "placeholder-implementation",
                "reexports",
                "secrets",
                "skipped-focused-tests",
                "dependency-policy",
                "sbom",
                "literal-risk",
                "import-boundaries",
                "rust-string-boundaries",
                "generated-artifacts",
                "required-tests",
                "source-shape",
                "architecture-policy",
                "single-source-contracts",
                "ai-rule-index",
                "validation-bypass",
                "weak-assertions",
            ])
        );
        Ok(())
    }

    #[test]
    fn tool_surface_enumeration_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let first = build_tool_descriptors();
        let second = build_tool_descriptors();
        let first_json = serde_json::to_string(&first)?;
        let second_json = serde_json::to_string(&second)?;
        assert_eq!(
            first_json, second_json,
            "tool-surface enumeration must be byte-deterministic for the d05 measure"
        );
        Ok(())
    }

    #[test]
    fn tool_surface_bytes_is_positive_and_stable_across_calls() {
        let descriptors = build_tool_descriptors();
        let first = tool_surface_bytes(&descriptors);
        let second = tool_surface_bytes(&descriptors);
        assert!(first > 0);
        assert_eq!(first, second);
    }

    #[test]
    fn check_tool_schema_carries_the_named_check_enum() -> Result<(), Box<dyn std::error::Error>> {
        let descriptors = build_tool_descriptors();
        let check_tool = descriptors
            .iter()
            .find(|d| d.name == "ocentra_enforcer_check")
            .ok_or("ocentra_enforcer_check must be registered")?;
        let schema_enum = check_tool.input_schema["properties"]["check"]["enum"]
            .as_array()
            .ok_or("check enum must be an array")?;
        assert_eq!(schema_enum.len(), NAMED_CHECKS.len());
        let properties = check_tool.input_schema["properties"]
            .as_object()
            .ok_or("check properties must be an object")?;
        let actual: BTreeSet<&str> = properties.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = [
            "check",
            "root",
            "configPath",
            "profile",
            "scope",
            "files",
            "crateName",
            "languages",
            "base",
            "head",
            "checkConfigPath",
            "output",
            "dryRun",
            "staged",
            "tracked",
            "strictEmptyTestTrees",
            "diagnosticLimit",
            "summaryOnly",
            "groupBy",
            "includeScope",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            actual, expected,
            "check schema must match frozen MJS fields"
        );
        Ok(())
    }
}
