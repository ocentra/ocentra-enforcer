//! Mechanical contract for the product north star and tri-program architecture.

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
            "missing product contract marker: {value}"
        );
    }
}

#[test]
fn north_star_pins_mechanical_reuse_and_honesty() -> TestResult {
    let root = workspace_root()?;
    let source = std::fs::read_to_string(root.join("docs/PRODUCT_NORTH_STAR.md"))?;
    require_all(
        &source,
        &[
            "portable mechanical software assurance",
            "Reuse-First Doctrine",
            "Author-Time Contract",
            "AI reviewer",
            "not a clean pass",
            "exact-SHA proof",
            "mechanically expressible violations",
            "universal-language-enforcement-plan",
            "cyberskills-parity-plan",
            "rust-mjs-parity-retirement-plan",
        ],
    );
    Ok(())
}

#[test]
fn execution_architecture_pins_boss_manager_and_lock_boundaries() -> TestResult {
    let root = workspace_root()?;
    let source =
        std::fs::read_to_string(root.join("docs/plans/PROGRAM_EXECUTION_ARCHITECTURE.md"))?;
    require_all(
        &source,
        &[
            "Primary boss",
            "one ready bundle",
            "hard branch-write conflict",
            "merge risk",
            "one integrator",
            "doesNotProve",
            "Cross-program dependency order",
            "manager independently reproduced",
            "Boss heartbeat",
            "hourly heartbeat",
            "PROGRAM_STATUS.md",
        ],
    );
    Ok(())
}

#[test]
fn boss_dashboard_keeps_status_subordinate_to_exact_sha_proof() -> TestResult {
    let root = workspace_root()?;
    let source = std::fs::read_to_string(root.join("docs/plans/PROGRAM_STATUS.md"))?;
    require_all(
        &source,
        &[
            "CP08 batch-30",
            "UL06 P1A1",
            "RM02-RM07 bounded read-only behavioral oracles",
            "Reuse-first tool adapter",
            "HOLD -> READY",
            "ACTIVE -> ACCEPTED",
            "integrationSha=<sha>",
            "never self-promote",
        ],
    );
    Ok(())
}
