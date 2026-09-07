//! Mechanical contract for the Rust/MJS parity-retirement plan.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WORKPACK_IDS: [&str; 15] = [
    "RM00", "RM01", "RM02", "RM03", "RM04", "RM05", "RM06", "RM07", "RM08", "RM09", "RM10", "RM11",
    "RM12", "RM13", "RM14",
];

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "expected enforcer-plan below the workspace root".into())
}

fn require_all(source: &str, required: &[&str]) {
    for value in required {
        assert!(
            source.contains(value),
            "missing parity-plan marker: {value}"
        );
    }
}

#[test]
fn parity_retirement_plan_has_all_workpacks_and_correct_authority() -> TestResult {
    let root = workspace_root()?;
    let plan = root.join("docs/plans/rust-mjs-parity-retirement-plan");
    let readme = std::fs::read_to_string(plan.join("README.md"))?;
    for required_doc in ["AGENTS.md", "ARCHITECTURE.md", "WORKER_CHECKLIST.md"] {
        assert!(plan.join(required_doc).is_file(), "missing {required_doc}");
    }
    require_all(
        &readme,
        &[
            "planId: rust-mjs-parity-retirement-plan",
            "267af94b701bd592e01a47649e3c18c26ee04239",
            "immutable public oracle",
            "d7162b6173e2c664547fcb9715ba135c435d0b1e",
            "Common fork base only; it is not the current public oracle.",
            "9d21780f9a4f5a498fb16a6b1ae1c05ac2d83e36",
            "allowlist-only",
            "never a public oracle, public source, public verdict input, or merge source",
            "rust-build",
            "267af94b701bd592e01a47649e3c18c26ee04239",
            "split runtime authority",
            "union/equal-or-stricter proof",
            "overlay's two exact allowlisted behaviors",
            "exact-SHA aggregate",
            "native rollback rehearsal",
            "delete-not-merge retirement",
            "never a production fallback",
        ],
    );
    let architecture = std::fs::read_to_string(plan.join("ARCHITECTURE.md"))?;
    require_all(
        &architecture,
        &[
            "equal-or-stricter union",
            "Schema equality is not behavior equality",
            "must not delegate to Node",
            "previous native release",
            "never merged into `main` or `rust-build`",
        ],
    );
    let worker = std::fs::read_to_string(plan.join("WORKER_CHECKLIST.md"))?;
    require_all(
        &worker,
        &[
            "same fixture/input/config",
            "never use it to make a public row pass",
            "do not self-promote",
            "Immediate stop",
        ],
    );
    let index = std::fs::read_to_string(plan.join("WORKPACK_INDEX.md"))?;
    let indexed_rows = index
        .lines()
        .filter(|line| line.starts_with("| RM"))
        .count();
    assert_eq!(
        indexed_rows, 15,
        "workpack index must have exactly 15 RM rows"
    );
    let rm01_row = index
        .lines()
        .find(|line| line.starts_with("| RM01 |"))
        .ok_or("missing RM01 index row")?;
    assert!(
        rm01_row.ends_with("| ACCEPTED |"),
        "RM01 is accepted only after public-surface inventory coverage is complete"
    );
    for active_id in ["RM02", "RM03", "RM04", "RM05", "RM06", "RM07"] {
        let prefix = format!("| {active_id} |");
        let row = index
            .lines()
            .find(|line| line.starts_with(&prefix))
            .ok_or("missing dependent oracle index row")?;
        assert!(
            row.ends_with("| ACTIVE-ORACLE |"),
            "{active_id} must remain an active read-only oracle after RM01 acceptance"
        );
    }
    let workpacks: Vec<_> = std::fs::read_dir(plan.join("workpacks"))?
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("rm"))
        .collect();
    assert_eq!(
        workpacks.len(),
        15,
        "must have exactly 15 RM workpack files"
    );
    for id in WORKPACK_IDS {
        let workpack = format!("{}-", id.to_ascii_lowercase());
        let matched: Vec<_> = workpacks
            .iter()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&workpack))
            .collect();
        assert_eq!(matched.len(), 1, "missing or duplicate workpack {id}");
        let entry = matched[0];
        let source = std::fs::read_to_string(entry.path())?;
        require_all(
            &source,
            &[
                "> Plan: rust-mjs-parity-retirement-plan",
                &format!("id: {id}"),
            ],
        );
        let index_marker = format!("| {id} |");
        assert_eq!(
            index
                .lines()
                .filter(|line| line.starts_with(&index_marker))
                .count(),
            1,
            "{id} must be indexed exactly once"
        );
    }
    Ok(())
}

#[test]
fn parity_retirement_plan_forbids_mjs_fallback_at_closure() -> TestResult {
    let root = workspace_root()?;
    let source = std::fs::read_to_string(root.join(
        "docs/plans/rust-mjs-parity-retirement-plan/workpacks/rm14-delete-not-merge-retirement.md",
    ))?;
    require_all(
        &source,
        &[
            "Delete-Not-Merge",
            "No executable MJS enforcement path remains",
            "without Node",
            "Do not merge the private overlay",
            "runtime MJS fallback",
        ],
    );
    Ok(())
}

#[test]
fn rm00_manifest_pins_public_plus_exact_overlay_authority() -> TestResult {
    let root = workspace_root()?;
    let raw = std::fs::read_to_string(
        root.join("docs/plans/rust-mjs-parity-retirement-plan/authority/RM00_AUTHORITY.json"),
    )?;
    let manifest: serde_json::Value = serde_json::from_str(&raw)?;

    assert_eq!(
        manifest
            .pointer("/schemaVersion")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        manifest
            .pointer("/authorities/publicFrozenOracle/sha")
            .and_then(serde_json::Value::as_str),
        Some("267af94b701bd592e01a47649e3c18c26ee04239")
    );
    assert_eq!(
        manifest
            .pointer("/authorities/provenanceBase/role")
            .and_then(serde_json::Value::as_str),
        Some("common-fork-provenance-only")
    );
    assert_eq!(
        manifest
            .pointer("/authorities/privateOverlay/publicVerdictAuthority")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        manifest
            .pointer("/overlayBehaviors/0/id")
            .and_then(serde_json::Value::as_str),
        Some("private-rust-test-module-exact-match")
    );
    assert_eq!(
        manifest
            .pointer("/overlayBehaviors/1/id")
            .and_then(serde_json::Value::as_str),
        Some("private-rust-test-module-policy-preservation")
    );
    assert_eq!(
        manifest
            .pointer("/aggregateParityContract/candidateVerdict")
            .and_then(serde_json::Value::as_str),
        Some("equal-or-stricter")
    );
    assert_eq!(
        manifest
            .pointer("/aggregateParityContract/privateOverlayMayProducePublicPass")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    Ok(())
}

#[test]
fn rm01_inventory_is_machine_readable_complete_and_unproved() -> TestResult {
    let root = workspace_root()?;
    let inventory_root = root.join("docs/plans/rust-mjs-parity-retirement-plan/inventory");
    let raw = std::fs::read_to_string(inventory_root.join("RM01_CAPABILITIES.json"))?;
    let matrix: serde_json::Value = serde_json::from_str(&raw)?;
    let schema_raw = std::fs::read_to_string(inventory_root.join("RM01_CAPABILITIES.schema.json"))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_raw)?;

    assert_eq!(
        matrix
            .pointer("/schemaVersion")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        matrix
            .pointer("/workpackId")
            .and_then(serde_json::Value::as_str),
        Some("RM01")
    );
    assert_eq!(
        matrix
            .pointer("/inventoryState")
            .and_then(serde_json::Value::as_str),
        Some("complete-unproved")
    );
    assert_eq!(
        matrix
            .pointer("/sourceReport/managerThreadId")
            .and_then(serde_json::Value::as_str),
        Some("019fc4c6-b2fb-78b3-985d-d5c235130a6e")
    );
    assert_eq!(
        matrix
            .pointer("/sourceReport/observedAtCandidateSha")
            .and_then(serde_json::Value::as_str),
        Some("e19076353d8cfc945b138311de9d4738021ec05d")
    );
    assert_eq!(
        matrix
            .pointer("/sourceReport/authorityDecisionInputCandidateSha")
            .and_then(serde_json::Value::as_str),
        Some("221179f0226665d66d2151897f757c4936bc1092")
    );
    assert_eq!(
        matrix
            .pointer("/coverage/complete")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        matrix
            .pointer("/coverage/proposalRowCount")
            .and_then(serde_json::Value::as_u64),
        Some(837)
    );
    assert_eq!(
        matrix
            .pointer("/coverage/knownPublicSurfaceCounts/canonicalMcpTools")
            .and_then(serde_json::Value::as_u64),
        Some(50)
    );
    assert_eq!(
        matrix
            .pointer("/coverage/knownPublicSurfaceCounts/legacyMcpAliases")
            .and_then(serde_json::Value::as_u64),
        Some(50)
    );
    assert_eq!(
        matrix
            .pointer("/coverage/knownPublicSurfaceCounts/registeredRuleIds")
            .and_then(serde_json::Value::as_u64),
        Some(570)
    );

    let rows = matrix
        .pointer("/rows")
        .and_then(serde_json::Value::as_array)
        .ok_or("RM01 rows must be an array")?;
    assert_eq!(rows.len(), 837);
    let ids: BTreeSet<_> = rows
        .iter()
        .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(ids.len(), rows.len(), "RM01 row IDs must be unique");

    for row in rows {
        assert_eq!(
            row.get("evidenceStatus")
                .and_then(serde_json::Value::as_str),
            Some("source-inventory-only")
        );
        assert_eq!(
            row.get("observedResult")
                .and_then(serde_json::Value::as_str),
            Some("unmeasured"),
            "source inventory must not promote a behavioral verdict"
        );
        assert!(
            row.get("doesNotProve")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|claims| claims
                    .iter()
                    .any(|claim| claim.as_str() == Some("behavioral parity"))),
            "every RM01 row must disclaim behavioral parity"
        );
    }

    let missing_ids: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.get("initialDisposition")
                .and_then(serde_json::Value::as_str)
                == Some("missing")
        })
        .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(missing_ids, ["C3-005"]);
    for corrected_id in ["C3-014", "C3-015"] {
        let row = rows
            .iter()
            .find(|row| row.get("id").and_then(serde_json::Value::as_str) == Some(corrected_id))
            .ok_or("missing corrected coordination row")?;
        assert_eq!(
            row.get("initialDisposition")
                .and_then(serde_json::Value::as_str),
            Some("unknown")
        );
        assert!(
            row.get("rustSources")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|sources| sources
                    .iter()
                    .any(|source| source.to_string().contains("crates/enforcer-mcp"))),
            "corrected coordination row must retain the native MCP surface"
        );
    }

    assert_eq!(
        schema
            .pointer("/properties/inventoryState/enum")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(4)
    );
    assert!(
        schema
            .pointer("/$defs/capabilityRow/properties/observedResult")
            .is_some(),
        "RM01 schema must distinguish source inventory from observed behavior"
    );
    Ok(())
}
