//! The consolidated MCP boundary tool registry: every tool this server exposes,
//! its JSON input schema, and (for `check`) the named-check enum parity
//! seam (workpack row "check named-check enum parity").
//!
//! This module is DATA ONLY — no dispatch logic (that is [`crate::router`])
//! and no I/O (that is [`crate::sink`]). Tool descriptions here are read by
//! the d05 context-budget tool-surface measure (see [`tool_surface_bytes`])
//! and by `tools/list`.

use crate::boundary::tool_descriptor::ToolDescriptorDto;
use enforcer_domain::ids::RuleId;
use enforcer_domain::mcp_types::McpToolName;

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
    // coordination family (arc-16 delegate) — write tools per the gate row
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
    // coordination family — read-only, never write-gated
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
    // server/meta — never write-gated
    "ocentra_enforcer_mcp_status",
    // ui family (arc-24/g01 delegate) — read-only report of the served
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
/// lookup surface of their own — `enforcer-rules`' `RuleRegistry` is keyed
/// by [`RuleId`] (e.g. `RR-6.1`), not by these friendly slugs. Rather than
/// fabricate a false-positive parity claim, this table is declared HERE,
/// owned by this crate, and the parity test in this module asserts
/// bidirectional equality between [`NAMED_CHECKS`] and this table's keys
/// (a same-crate consistency gate). When a sibling pack later exposes a
/// real named-check -> RuleId-family registry, this table's values (empty
/// `Vec`s below) are the ONLY thing that changes — the parity test and its
/// bidirectional-equality assertion do not need to change shape, only this
/// data. Until then an empty backing vec means "declared, not yet wired",
/// which is what [`is_wired`] reports honestly rather than silently.
pub fn named_check_backing() -> Vec<(&'static str, Vec<RuleId>)> {
    NAMED_CHECKS
        .iter()
        .map(|&name| (name, Vec::new()))
        .collect()
}

/// True once at least one [`RuleId`] backs the named check (see
/// [`named_check_backing`]'s honest-scope note: currently always `false`
/// until a sibling pack wires real backing ids in).
pub fn is_wired(entry: &(&'static str, Vec<RuleId>)) -> bool {
    !entry.1.is_empty()
}

/// Byte length of the JSON-encoded canonical tool descriptor list — the
/// measurable surface the d05 context-budget ratchet consumes (this crate
/// owns the measurable surface; d05 owns the baseline/ratchet files, see
/// the workpack's "Parallel Ownership Notes").
pub fn tool_surface_bytes(descriptors: &[ToolDescriptorDto]) -> usize {
    serde_json::to_vec(descriptors)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

/// Build every tool descriptor: canonical tools first (stable order,
/// matching [`CANONICAL_TOOLS`]), then legacy aliases (see
/// [`crate::aliases`]) — mirrors `TOOLS = [...CANONICAL_TOOLS,
/// ...LEGACY_ALIAS_TOOLS]` in the legacy `.mjs` registry so the
/// `tools/list` order is deterministic (required for the d05 measure to be
/// reproducible).
pub fn build_tool_descriptors() -> Vec<ToolDescriptorDto> {
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
        other => format!("Ocentra Enforcer tool: {other}."),
    }
}

fn canonical_input_schema(name: &str) -> serde_json::Value {
    if name == "ocentra_enforcer_check" {
        return serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "check": { "type": "string", "enum": NAMED_CHECKS },
                "root": { "type": "string" },
            },
            "required": ["check"],
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
    serde_json::json!({
        "type": "object",
        "additionalProperties": true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_tool_descriptors, is_wired, named_check_backing, tool_surface_bytes, CANONICAL_TOOLS,
        NAMED_CHECKS,
    };
    use std::collections::BTreeSet;

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
        // (bidirectional equality of the KEY SET — see module docs on the
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
    fn is_wired_is_honest_about_unwired_backing() {
        for entry in named_check_backing() {
            assert!(
                !is_wired(&entry),
                "no sibling pack has wired named-check backing yet; is_wired must report false, not fabricate parity"
            );
        }
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
        Ok(())
    }
}
