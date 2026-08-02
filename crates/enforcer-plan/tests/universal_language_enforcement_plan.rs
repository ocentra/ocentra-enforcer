//! Mechanical integrity contract for the universal-language plan.
//!
//! This intentionally validates documentation structure rather than a live
//! runtime. It prevents the dispatcher from presenting an index-only plan or
//! from erasing the mechanical-authority and reuse-first limits that make the
//! workpacks safe to delegate.

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "expected enforcer-plan below the workspace root".into())
}

fn require_all(source: &str, required: &[&str], context: &str) {
    for value in required {
        assert!(source.contains(value), "{context} missing marker: {value}");
    }
}

#[test]
fn universal_plan_has_fifteen_real_capsuled_workpacks_and_complete_index() -> TestResult {
    let root = workspace_root()?;
    let plan = root.join("docs/plans/universal-language-enforcement-plan");
    let index = std::fs::read_to_string(plan.join("WORKPACK_INDEX.md"))?;
    let expected = [
        ("UL00", "ul00-capability-truth-inventory.md"),
        ("UL01", "ul01-shape-driven-doctrine.md"),
        ("UL02", "ul02-grammar-ownership-transfer.md"),
        ("UL03", "ul03-shared-syntax-extraction.md"),
        ("UL04", "ul04-facts-and-parse-honesty.md"),
        ("UL05", "ul05-validator-analysis-bridge.md"),
        ("UL06", "ul06-canonical-language-routing.md"),
        ("UL07", "ul07-reuse-first-tool-adapter.md"),
        ("UL08", "ul08-fact-backed-rule-pilot.md"),
        ("UL09", "ul09-schema-framework-adapters.md"),
        ("UL10", "ul10-existing-language-routing.md"),
        ("UL11", "ul11-language-capability-waves.md"),
        ("UL12", "ul12-generic-fact-rule-families.md"),
        ("UL13", "ul13-graph-and-semantic-providers.md"),
        ("UL14", "ul14-closure-and-dogfood.md"),
    ];

    assert_eq!(
        expected.len(),
        15,
        "the plan is deliberately UL00 through UL14"
    );
    for (id, filename) in expected {
        require_all(&index, &[id, filename], "workpack index");
        let source = std::fs::read_to_string(plan.join("workpacks").join(filename))?;
        require_all(
            &source,
            &[
                "<!-- agent-capsule -->",
                "Plan: `universal-language-enforcement-plan`",
                "## Owns",
                "## Acceptance And Proof",
                "## Stop conditions",
                "## Parallel Ownership Notes",
            ],
            filename,
        );
    }
    Ok(())
}

#[test]
fn universal_plan_preserves_mechanical_reuse_and_honest_outcomes() -> TestResult {
    let root = workspace_root()?;
    let plan = root.join("docs/plans/universal-language-enforcement-plan");
    let architecture = std::fs::read_to_string(plan.join("ARCHITECTURE.md"))?;
    let adapter = std::fs::read_to_string(plan.join("workpacks/ul07-reuse-first-tool-adapter.md"))?;
    let routing =
        std::fs::read_to_string(plan.join("workpacks/ul10-existing-language-routing.md"))?;
    let north_star = std::fs::read_to_string(root.join("docs/PRODUCT_NORTH_STAR.md"))?;

    require_all(
        &north_star,
        &[
            "portable mechanical software assurance",
            "Reuse-First Doctrine",
            "deterministic tools, typed policy, executable fixtures",
            "not a clean pass",
        ],
        "product north star",
    );
    require_all(
        &architecture,
        &[
            "shared allowlisted `enforcer-harness` adapter contract",
            "CyberSkills consumes it",
            "`unknown` means no canonical identity was derived",
            "`recognized-but-unsupported`",
            "`unavailable` means a declared provider could not be used",
            "None is a clean mechanical pass",
        ],
        "universal architecture",
    );
    require_all(
        &adapter,
        &[
            "deepen the existing `enforcer-harness`",
            "do not create a second generic runner",
            "version-mismatch",
            "malformed-output",
            "required never silently skips",
            "allowlisted executable/argument templates",
        ],
        "UL07 adapter contract",
    );
    require_all(
        &routing,
        &[
            "Dart, CFML, or Go",
            "unsupported/unavailable",
            "shared scan/router/tool/capability registry files are integrator-only",
        ],
        "UL10 routing contract",
    );
    Ok(())
}

#[test]
fn universal_plan_keeps_integrator_and_closure_authority_single_writer() -> TestResult {
    let root = workspace_root()?;
    let plan = root.join("docs/plans/universal-language-enforcement-plan");
    let runbook = std::fs::read_to_string(plan.join("MANAGER_RUNBOOK.md"))?;
    let waves = std::fs::read_to_string(plan.join("workpacks/ul11-language-capability-waves.md"))?;
    let closure = std::fs::read_to_string(plan.join("workpacks/ul14-closure-and-dogfood.md"))?;

    require_all(
        &runbook,
        &[
            "One active workpack is permitted at a time",
            "at most three implementation children",
            "UL07, shared registries, contracts, and UL14 are never child-parallel",
        ],
        "manager runbook",
    );
    require_all(
        &waves,
        &[
            "canonical registry/matrix are integrator-only",
            "one language per child",
            "at most three languages per wave",
        ],
        "UL11 wave contract",
    );
    require_all(
        &closure,
        &[
            "`UL07` always required",
            "derived from active profile and capability matrix, not hand-picked",
            "independent gatekeeper",
            "closure does not patch it",
        ],
        "UL14 closure contract",
    );
    Ok(())
}
